/*
 This file is part of KFST.

 (c) 2023-2025 Iikka Hauhio <iikka.hauhio@helsinki.fi> and Théo Salmenkivi-Friberg <theo.friberg@helsinki.fi>

 KFST is free software: you can redistribute it and/or modify it under the
 terms of the GNU Lesser General Public License as published by the Free
 Software Foundation, either version 3 of the License, or (at your option) any
 later version.

 KFST is distributed in the hope that it will be useful, but WITHOUT ANY
 WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
 FOR A PARTICULAR PURPOSE. See the GNU Lesser General Public License for more
 details.

 You should have received a copy of the GNU Lesser General Public License
 along with KFST. If not, see <https://www.gnu.org/licenses/>.
*/

//! Fast and portable HFST-compatible finite-state transducers.
//!
//! An implementation of finite-state transducers mostly compatible with [HFST](https://hfst.github.io/).
//! Provides the optional accelerated back-end for [kfst](https://github.com/fergusq/fst-python).
//! Able to load and execute [Voikko](https://voikko.puimula.org/) and [Omorfi](https://github.com/flammie/omorfi):
//! see [kfst](https://github.com/fergusq/fst-python) for transducers converted to a compatible format as well as Python bindings.
//! Supports the ATT format and its own KFST format.
//!
//! To convert HFST (optimized lookup or otherwise) to ATT using HFST's tools, do:
//!
//! ```bash
//! hfst-fst2txt transducer.hfst -o transducer.att
//! ```
//!
//! Given the Voikko transducer in KFST or ATT format, one could create a simple analyzer like this:
//!
//! ```rust
//! use kfst_rs::{FSTState, FST};
//! use std::io::{self, Write};
//!
//! // Read in transducer
//!
//! # let pathtovoikko = "../pyvoikko/pyvoikko/voikko.kfst".to_string();
//! let fst = FST::from_kfst_file(pathtovoikko, true).unwrap();
//! // Alternatively, for ATT use FST::from_att_file
//!
//! // Read in word to analyze
//!
//! let mut buffer = String::new();
//! let stdin = io::stdin();
//! stdin.read_line(&mut buffer).unwrap();
//! buffer = buffer.trim().to_string();
//!
//! // Do analysis proper
//!
//! match fst.lookup(&buffer, FSTState::default(), true) {
//!     Ok(result) => {
//!         for (i, analysis) in result.into_iter().enumerate() {
//!             println!("Analysis {}: {} ({})", i+1, analysis.0, analysis.1)
//!         }
//!     },
//!     Err(err) => println!("No analysis: {:?}", err),
//! }
//! ```
//! Given the input "lentokoneessa", this gives the following analysis:
//!
//! ```text
//! Analysis 1: [Lt][Xp]lentää[X]len[Ln][Xj]to[X]to[Sn][Ny][Bh][Bc][Ln][Xp]kone[X]konee[Sine][Ny]ssa (0)
//! ```

#[cfg(feature = "python")]
use std::cmp::Ordering;
use std::collections::HashSet;
use std::fmt::Debug;
#[cfg(feature = "python")]
use std::fmt::Error;
use std::fs::{self, File};
use std::hash::Hash;
use std::io::Read;
use std::path::Path;

use im::HashMap;
use indexmap::{indexmap, IndexMap, IndexSet};
use lzma_rs::lzma_compress;
use nom::branch::alt;
use nom::bytes::complete::{tag, take_until1};
use nom::multi::many_m_n;
use nom::Parser;
use std::sync::{LazyLock, Mutex};

#[cfg(feature = "python")]
use pyo3::create_exception;
#[cfg(feature = "python")]
use pyo3::exceptions::{PyIOError, PyValueError};
#[cfg(feature = "python")]
use pyo3::types::PyDict;
#[cfg(feature = "python")]
use pyo3::{prelude::*, py_run, IntoPyObjectExt};

// We have result types that kinda depend on the target
// If we target pyo3, we want python results and errors
// Otherwise, we want stdlib errors

#[cfg(feature = "python")]
type KFSTResult<T> = PyResult<T>;
#[cfg(not(feature = "python"))]
type KFSTResult<T> = std::result::Result<T, String>;

#[cfg(feature = "python")]
fn value_error<T>(msg: String) -> KFSTResult<T> {
    KFSTResult::Err(PyErr::new::<PyValueError, _>(msg))
}
#[cfg(not(feature = "python"))]
fn value_error<T>(msg: String) -> KFSTResult<T> {
    KFSTResult::Err(msg)
}

#[cfg(feature = "python")]
fn io_error<T>(msg: String) -> KFSTResult<T> {
    use pyo3::exceptions::PyIOError;

    KFSTResult::Err(PyErr::new::<PyIOError, _>(msg))
}
#[cfg(not(feature = "python"))]
fn io_error<T>(msg: String) -> KFSTResult<T> {
    KFSTResult::Err(msg)
}

#[cfg(feature = "python")]
fn tokenization_exception<T>(msg: String) -> KFSTResult<T> {
    KFSTResult::Err(PyErr::new::<TokenizationException, _>(msg))
}
#[cfg(not(feature = "python"))]
fn tokenization_exception<T>(msg: String) -> KFSTResult<T> {
    KFSTResult::Err(msg)
}

#[cfg(feature = "python")]
create_exception!(
    kfst_rs,
    TokenizationException,
    pyo3::exceptions::PyException
);

// Symbol interning

static STRING_INTERNER: LazyLock<Mutex<IndexSet<String>>> =
    LazyLock::new(|| Mutex::new(IndexSet::new()));

fn intern(s: String) -> u32 {
    u32::try_from(STRING_INTERNER.lock().unwrap().insert_full(s).0).unwrap()
}

fn deintern(idx: u32) -> String {
    STRING_INTERNER
        .lock()
        .unwrap()
        .get_index(idx.try_into().unwrap())
        .unwrap()
        .clone()
}

#[cfg_attr(
    feature = "python",
    pyclass(str = "RawSymbol({value:?})", eq, ord, frozen, hash, get_all)
)]
#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug)]
#[readonly::make]
/// A Symbol type that has a signaling byte (the first one) and 14 other bytes to dispose of as the caller wishes.
/// This odd size is such that [Symbol] can be 16 bytes long: a 1-byte discriminant + 15 bytes.
/// (The [Symbol::Flag] variant forces [Symbol] to be at least 16 bytes.)
pub struct RawSymbol {
    /// The first bit of the first byte should be 1 if the symbol is to be seen as epsilon (see [is_epsilon](RawSymbol::is_epsilon)).
    ///
    /// The second bit of the first byte should be 1 if the symbol is to be seen as unknown (see [is_unknown](RawSymbol::is_unknown)).
    ///
    /// The remainder of the first byte is reserved.
    ///
    /// The following bytes are caller-defined.
    pub value: [u8; 15],
}

#[cfg_attr(feature = "python", pymethods)]
impl RawSymbol {
    /// Whether this symbol should be seen as ε. (See [Symbol::is_epsilon] for more general information on this)
    /// Returns true is the least-significant bit of the first byte of [RawSymbol::value] is set. Returns false otherwise.
    pub fn is_epsilon(&self) -> bool {
        (self.value[0] & 1) != 0
    }

    /// Whether this symbol should be seen as unknown. (See [Symbol::is_unknown] for more general information on this)
    /// Returns true is the second least-significant bit of the first byte of [RawSymbol::value] is set. Returns false otherwise.
    pub fn is_unknown(&self) -> bool {
        (self.value[0] & 2) != 0
    }

    /// A textual representation of this symbol. (See [Symbol::get_symbol] for more general information on this)
    pub fn get_symbol(&self) -> String {
        format!("RawSymbol({:?})", self.value)
    }

    #[cfg(feature = "python")]
    #[new]
    fn new(value: [u8; 15]) -> Self {
        RawSymbol { value }
    }

    /// Construct an instance of RawSymbol; simply sets [RawSymbol::value] to the provided value.
    #[cfg(not(feature = "python"))]
    pub fn new(value: [u8; 15]) -> Self {
        RawSymbol { value }
    }

    #[deprecated]
    /// Python-style string representation.
    pub fn __repr__(&self) -> String {
        format!("RawSymbol({:?})", self.value)
    }
}

#[cfg(feature = "python")]
struct PyObjectSymbol {
    value: PyObject,
}

#[cfg(feature = "python")]
impl Debug for PyObjectSymbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Python::with_gil(|py| {
            // Seemingly compiling for CPython3.13t (=free-threaded) doesn't for some mysterious reason allow to extract to a &str
            // So an owned string it must be
            let s: String = self
                .value
                .getattr(py, "__repr__")
                .unwrap()
                .call0(py)
                .unwrap()
                .extract(py)
                .unwrap();
            f.write_str(&s)
        })
    }
}

#[cfg(feature = "python")]
impl PartialEq for PyObjectSymbol {
    fn eq(&self, other: &Self) -> bool {
        Python::with_gil(|py| {
            self.value
                .getattr(py, "__eq__")
                .unwrap_or_else(|_| {
                    panic!(
                        "Symbol {} doesn't have an __eq__ implementation.",
                        self.value
                    )
                })
                .call1(py, (other.value.clone_ref(py),))
                .unwrap_or_else(|_| {
                    panic!("__eq__ on symbol {} failed to return a value.", self.value)
                })
                .extract(py)
                .unwrap_or_else(|_| panic!("__eq__ on symbol {} didn't return a bool.", self.value))
        })
    }
}

#[cfg(feature = "python")]
impl Hash for PyObjectSymbol {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_i128(Python::with_gil(|py| {
            self.value
                .getattr(py, "__hash__")
                .unwrap_or_else(|_| {
                    panic!(
                        "Symbol {} doesn't have a __hash__ implementation.",
                        self.value
                    )
                })
                .call0(py)
                .unwrap_or_else(|_| {
                    panic!(
                        "__hash__ on symbol {} failed to return a value.",
                        self.value
                    )
                })
                .extract(py)
                .unwrap_or_else(|_| {
                    panic!("__hash__ on symbol {} didn't return an int.", self.value)
                })
        }))
    }
}

#[cfg(feature = "python")]
impl Eq for PyObjectSymbol {}

#[cfg(feature = "python")]
impl PartialOrd for PyObjectSymbol {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(feature = "python")]
impl Ord for PyObjectSymbol {
    fn cmp(&self, other: &Self) -> Ordering {
        Python::with_gil(|py| {
            match self
                .value
                .getattr(py, "__gt__")
                .unwrap_or_else(|_| {
                    panic!(
                        "Symbol {} doesn't have a __gt__ implementation.",
                        self.value
                    )
                })
                .call1(py, (other.value.clone_ref(py),))
                .unwrap_or_else(|_| {
                    panic!("__gt__ on symbol {} failed to return a value.", self.value)
                })
                .extract::<bool>(py)
                .unwrap_or_else(|_| panic!("__gt__ on symbol {} didn't return a bool.", self.value))
            {
                true => Ordering::Greater,
                false => {
                    match self
                        .value
                        .getattr(py, "__eq__")
                        .unwrap_or_else(|_| {
                            panic!(
                                "Symbol {} doesn't have an __eq__ implementation.",
                                self.value
                            )
                        })
                        .call1(py, (other.value.clone_ref(py),))
                        .unwrap_or_else(|_| {
                            panic!("__eq__ on symbol {} failed to return a value.", self.value)
                        })
                        .extract::<bool>(py)
                        .unwrap_or_else(|_| {
                            panic!("__eq__ on symbol {} didn't return a bool.", self.value)
                        }) {
                        true => Ordering::Equal,
                        false => Ordering::Less,
                    }
                }
            }
        })
    }
}

#[cfg(feature = "python")]
impl Clone for PyObjectSymbol {
    fn clone(&self) -> Self {
        Python::with_gil(|py| Self {
            value: self.value.clone_ref(py),
        })
    }
}

#[cfg(feature = "python")]
impl FromPyObject<'_> for PyObjectSymbol {
    fn extract_bound(ob: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(PyObjectSymbol {
            value: ob.clone().unbind(),
        }) // The clone here is technical; no actual cloning of a value
    }
}

#[cfg(feature = "python")]
impl<'py> IntoPyObject<'py> for PyObjectSymbol {
    type Target = PyAny;

    type Output = Bound<'py, Self::Target>;

    type Error = pyo3::PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        self.value.into_bound_py_any(py)
    }
}

#[cfg(feature = "python")]
impl PyObjectSymbol {
    fn is_epsilon(&self) -> bool {
        Python::with_gil(|py| {
            self.value
                .getattr(py, "is_epsilon")
                .unwrap_or_else(|_| {
                    panic!(
                        "Symbol {} doesn't have an is_epsilon implementation.",
                        self.value
                    )
                })
                .call(py, (), None)
                .unwrap_or_else(|_| {
                    panic!(
                        "is_epsilon on symbol {} failed to return a value.",
                        self.value
                    )
                })
                .extract(py)
                .unwrap_or_else(|_| {
                    panic!("is_epsilon on symbol {} didn't return a bool.", self.value)
                })
        })
    }

    fn is_unknown(&self) -> bool {
        Python::with_gil(|py| {
            self.value
                .getattr(py, "is_unknown")
                .unwrap_or_else(|_| {
                    panic!(
                        "Symbol {} doesn't have an is_unknown implementation.",
                        self.value
                    )
                })
                .call(py, (), None)
                .unwrap_or_else(|_| {
                    panic!(
                        "is_unknown on symbol {} failed to return a value.",
                        self.value
                    )
                })
                .extract(py)
                .unwrap_or_else(|_| {
                    panic!("is_unknown on symbol {} didn't return a bool.", self.value)
                })
        })
    }

    fn get_symbol(&self) -> String {
        Python::with_gil(|py| {
            self.value
                .getattr(py, "get_symbol")
                .unwrap_or_else(|_| {
                    panic!(
                        "Symbol {} doesn't have a get_symbol implementation.",
                        self.value
                    )
                })
                .call(py, (), None)
                .unwrap_or_else(|_| {
                    panic!(
                        "get_symbol on symbol {} failed to return a value.",
                        self.value
                    )
                })
                .extract(py)
                .unwrap_or_else(|_| {
                    panic!("get_symbol on symbol {} didn't return a bool.", self.value)
                })
        })
    }
}

#[cfg_attr(
    feature = "python",
    pyclass(
        str = "StringSymbol({string:?}, {unknown})",
        eq,
        ord,
        frozen,
        hash,
        get_all
    )
)]
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
#[readonly::make]
/// A symbol that holds an interned string and the information of whether it should be seen as unknown (see [is_unknown](StringSymbol::is_unknown)).
/// The a copy of the interned string is **held until the end of the program**.
pub struct StringSymbol {
    string: u32,
    /// Whether this symbol is considered unknown.
    pub unknown: bool,
}

impl StringSymbol {
    /// Parse a [&str] into a StringSymbol carrying the same text. Returns a known symbol. Fails if given an empty string.
    ///
    /// ```
    /// use kfst_rs::StringSymbol;
    ///
    /// StringSymbol::parse("kissa").unwrap(); // Parses into a symbol
    /// assert!(StringSymbol::parse("").is_err()); // Fails because of empty string
    /// ```
    ///
    /// This is a [nom]-style parser that returns the unparsed part of the string alongside the parsed [StringSymbol].
    /// However, it gobbles up the whole input string and is guaranteed to return something of the form (assuming that it returns Ok at all)
    ///
    /// ```no_test
    /// Ok(("", StringSymbol { ... }))
    /// ```
    pub fn parse(symbol: &str) -> nom::IResult<&str, StringSymbol> {
        if symbol.is_empty() {
            return nom::IResult::Err(nom::Err::Error(nom::error::Error::new(
                symbol,
                nom::error::ErrorKind::Fail,
            )));
        }
        Ok((
            "",
            StringSymbol {
                string: intern(symbol.to_string()),
                unknown: false,
            },
        ))
    }
}

impl PartialOrd for StringSymbol {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for StringSymbol {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match other.string.cmp(&self.string) {
            std::cmp::Ordering::Less => std::cmp::Ordering::Less,
            std::cmp::Ordering::Equal => self.unknown.cmp(&other.unknown),
            std::cmp::Ordering::Greater => std::cmp::Ordering::Greater,
        }
    }
}
#[cfg_attr(feature = "python", pymethods)]
impl StringSymbol {
    /// Is this an ε symbol? (See [Symbol::is_epsilon] for more details on the general case)
    /// Always returns false.
    pub fn is_epsilon(&self) -> bool {
        false
    }

    /// Is this an unknown symbol? (See [Symbol::is_unknown] for more details on the general case)
    /// Returns the value of [StringSymbol::unknown].
    pub fn is_unknown(&self) -> bool {
        self.unknown
    }

    /// String representation of this symbol (returns the string from which the symbol was constructed)
    pub fn get_symbol(&self) -> String {
        deintern(self.string)
    }

    #[cfg(feature = "python")]
    #[new]
    fn new(string: String, unknown: bool) -> Self {
        StringSymbol {
            string: intern(string),
            unknown,
        }
    }

    #[cfg(not(feature = "python"))]
    /// Creates a new string symbol. Notably, this **interns the string for the program's runtime**.
    pub fn new(string: String, unknown: bool) -> Self {
        StringSymbol {
            string: intern(string),
            unknown,
        }
    }

    #[deprecated]
    /// Python-style string representation.
    pub fn __repr__(&self) -> String {
        format!("StringSymbol({:?}, {})", self.string, self.unknown)
    }
}

#[cfg_attr(feature = "python", pyclass(eq, ord, frozen))]
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy, Hash)]
/// The different types of flag diacritic supported by kfst_rs.
pub enum FlagDiacriticType {
    /// Unification diacritic.
    U,
    /// Requirement diacritic
    R,
    /// Denial diacritic
    D,
    /// Clearing diacritic. The transition can always be taken, and the associated flag is cleared.
    C,
    /// Positive setting diacritic.
    P,
    /// Negative setting diacritic. The transition can always be taken, and the associated flag is negatively set.
    /// Eg. `@N.X.Y@` means that `X` is set to a value that is guaranteed to not unify with `Y`.
    N,
}

impl FlagDiacriticType {
    /// Converts a string from the set {U, R, D, C, P, N} to the matching diacritic.
    /// This potentially confusing (see [std::str::FromStr::from_str]) name is as is for Python compatibility.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "U" => Some(FlagDiacriticType::U),
            "R" => Some(FlagDiacriticType::R),
            "D" => Some(FlagDiacriticType::D),
            "C" => Some(FlagDiacriticType::C),
            "P" => Some(FlagDiacriticType::P),
            "N" => Some(FlagDiacriticType::N),
            _ => None,
        }
    }
}

#[cfg_attr(feature = "python", pymethods)]
impl FlagDiacriticType {
    #[deprecated]
    /// Python-style string representation.
    pub fn __repr__(&self) -> String {
        format!("{:?}", &self)
    }
}

#[cfg_attr(
    feature = "python",
    pyclass(
        str = "FlagDiacriticSymbol({flag_type:?}, {key:?}, {value:?})",
        eq,
        ord,
        frozen,
        hash
    )
)]
#[derive(PartialEq, Eq, Clone, Copy, Hash)]
#[readonly::make]
/// A [Symbol] representing a flag diacritic.
/// Flag diacritics allow making state transitions depend on externally kept state, thus often making transducers smaller.
/// The symbol consist of three parts:
/// 1. The FlagType; see [FlagDiacriticType] for possible options
/// 2. The name of the flag itself (accessible via [FlagDiacriticSymbol::key])
/// 2. The value of the flag (accessible via [FlagDiacriticSymbol::value])
pub struct FlagDiacriticSymbol {
    /// The type of the flag; see [FlagDiacriticType] for all possible values.
    pub flag_type: FlagDiacriticType,
    key: u32,
    value: u32,
}

impl PartialOrd for FlagDiacriticSymbol {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FlagDiacriticSymbol {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // This should be clean; there is a bijection between all flag diacritics and a subset of strings
        other.get_symbol().cmp(&self.get_symbol())
    }
}

impl FlagDiacriticSymbol {
    /// Parse a flag diacritic from a string representation of the form @SYMBOL_TYPE.KEY.VALUE@ or @SYMBOL_TYPE.KEY@.
    pub fn parse(symbol: &str) -> nom::IResult<&str, FlagDiacriticSymbol> {
        let mut parser = (
            tag("@"),
            alt((tag("U"), tag("R"), tag("D"), tag("C"), tag("P"), tag("N"))),
            tag("."),
            many_m_n(0, 1, (take_until1("."), tag("."))),
            take_until1("@"),
            tag("@"),
        );
        let (input, (_, diacritic_type, _, named_piece_1, named_piece_2, _)) =
            parser.parse(symbol)?;
        let diacritic_type = match FlagDiacriticType::from_str(diacritic_type) {
            Some(x) => x,
            None => {
                return Err(nom::Err::Error(nom::error::Error::new(
                    diacritic_type,
                    nom::error::ErrorKind::Fail,
                )))
            }
        };

        let (name, value) = if !named_piece_1.is_empty() {
            (named_piece_1[0].0, intern(named_piece_2.to_string()))
        } else {
            (named_piece_2, u32::MAX)
        };

        Ok((
            input,
            FlagDiacriticSymbol {
                flag_type: diacritic_type,
                key: intern(name.to_string()),
                value,
            },
        ))
    }
}

// These functions have some non-trivial pyo3-attributes that cannot be cfg_attr'ed in and non-trivial content
// Need to be specified in separate impl block

impl FlagDiacriticSymbol {
    fn _from_symbol_string(symbol: &str) -> KFSTResult<Self> {
        match FlagDiacriticSymbol::parse(symbol) {
            Ok(("", symbol)) => KFSTResult::Ok(symbol),
            Ok((rest, _)) => value_error(format!("String {:?} contains a valid FlagDiacriticSymbol, but it has unparseable text at the end: {:?}", symbol, rest)),
            _ => value_error(format!("Not a valid FlagDiacriticSymbol: {:?}", symbol))
        }
    }

    #[cfg(not(feature = "python"))]
    #[deprecated]
    /// Parse from symbol string; exists for Python compatibility, prefer [FlagDiacriticSymbol::parse].
    pub fn from_symbol_string(symbol: &str) -> KFSTResult<Self> {
        FlagDiacriticSymbol::_from_symbol_string(symbol)
    }

    fn _new(flag_type: String, key: String, value: Option<String>) -> KFSTResult<Self> {
        let flag_type = match FlagDiacriticType::from_str(&flag_type) {
            Some(x) => x,
            None => value_error(format!(
                "String {:?} is not a valid FlagDiacriticType specifier",
                flag_type
            ))?,
        };
        Ok(FlagDiacriticSymbol {
            flag_type,
            key: intern(key),
            value: value.map(intern).unwrap_or(u32::MAX),
        })
    }

    #[cfg(not(feature = "python"))]
    /// Construct flag diacritic from a [String] representation of flag type, key and value.
    pub fn new(flag_type: String, key: String, value: Option<String>) -> KFSTResult<Self> {
        FlagDiacriticSymbol::_new(flag_type, key, value)
    }

    #[cfg(not(feature = "python"))]
    /// Deintern the key
    pub fn key(self) -> String {
        deintern(self.key)
    }

    #[cfg(not(feature = "python"))]
    /// Deintern the value
    pub fn value(self) -> String {
        deintern(self.value)
    }
}

#[cfg_attr(feature = "python", pymethods)]
impl FlagDiacriticSymbol {
    /// Is this symbol to be treated as an ε symbol? Flag diacritics are always ε; this method is guaranteed to return true.
    /// See [Symbol::is_epsilon] for a more in-depth explanation of what it means to be ε.
    pub fn is_epsilon(&self) -> bool {
        true
    }

    /// Is this symbol to be treated as an unknown symbol?
    /// See [Symbol::is_epsilon] for a more in-depth explanation of what it means to be unknown.
    pub fn is_unknown(&self) -> bool {
        false
    }

    pub fn get_symbol(&self) -> String {
        match self.value {
            u32::MAX => format!("@{:?}.{}@", self.flag_type, deintern(self.key)),
            value => format!(
                "@{:?}.{}.{}@",
                self.flag_type,
                deintern(self.key),
                deintern(value)
            ),
        }
    }

    #[cfg(feature = "python")]
    #[getter]
    fn flag_type(&self) -> String {
        format!("{:?}", self.flag_type)
    }

    #[cfg(not(feature = "python"))]
    /// Get the flag_type as a string.
    pub fn flag_type(&self) -> String {
        format!("{:?}", self.flag_type)
    }

    #[cfg(feature = "python")]
    #[getter]
    fn key(&self) -> String {
        deintern(self.key)
    }

    #[cfg(feature = "python")]
    #[getter]
    fn value(&self) -> String {
        deintern(self.value)
    }

    #[cfg(feature = "python")]
    #[new]
    #[pyo3(signature = (flag_type, key, value = None))]
    fn new(flag_type: String, key: String, value: Option<String>) -> KFSTResult<Self> {
        FlagDiacriticSymbol::_new(flag_type, key, value)
    }

    #[deprecated]
    /// Python-style string representation.
    pub fn __repr__(&self) -> String {
        match self.value {
            u32::MAX => format!(
                "FlagDiacriticSymbol({:?}, {:?})",
                self.flag_type,
                deintern(self.key)
            ),
            value => format!(
                "FlagDiacriticSymbol({:?}, {:?}, {:?})",
                self.flag_type,
                deintern(self.key),
                deintern(value)
            ),
        }
    }

    #[cfg(feature = "python")]
    #[staticmethod]
    fn is_flag_diacritic(symbol: &str) -> bool {
        matches!(FlagDiacriticSymbol::parse(symbol), Ok(("", _)))
    }

    #[cfg(not(feature = "python"))]
    /// Check if a string is a [FlagDiacriticSymbol], ie. of the form `@X.Y@` or `@X.Y.Z@` for arbitrary `Y` and `Z` and an `X` that is a [FlagDiacriticType];
    /// see [FlagDiacriticType::from_str]
    pub fn is_flag_diacritic(symbol: &str) -> bool {
        matches!(FlagDiacriticSymbol::parse(symbol), Ok(("", _)))
    }

    #[cfg(feature = "python")]
    #[staticmethod]
    fn from_symbol_string(symbol: &str) -> KFSTResult<Self> {
        FlagDiacriticSymbol::_from_symbol_string(symbol)
    }
}

impl std::fmt::Debug for FlagDiacriticSymbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get_symbol())
    }
}

#[cfg_attr(feature = "python", pyclass(eq, ord, frozen, hash))]
#[derive(PartialEq, Eq, Clone, Hash, Copy)]
/// The three possible HFST special symbols.
pub enum SpecialSymbol {
    /// The simplest possible ε-symbol.
    /// In transition position, it can always be followed and it doesn't modify flag state.
    /// If placed in ouput position, it is removed from the output string.
    EPSILON,
    /// The identity special symbol.
    /// It should only appear in transition position. It accepts any unknown symbol, ie. it accepts a symbol if [Symbol::is_unknown] returns `true` for it.
    /// It transduces an input symbol into the same symbol on the output side. (Hence the name "identity")
    IDENTITY,
    /// The unknown special symbol.
    /// It should only appear in transition position. It matches any unknown symbol, ie. it accepts a symbol if [Symbol::is_unknown] returns `true` for it.
    UNKNOWN,
}

impl SpecialSymbol {
    /// Parses this symbol from (the beginning of) a string representation.
    /// Accepts:
    ///
    /// * `@_EPSILON_SYMBOL_@` and `@0@` for ε ([SpecialSymbol::EPSILON])
    /// * `@_IDENTITY_SYMBOL_@` for identity ([SpecialSymbol::IDENTITY])
    /// * `@_UNKNOWN_SYMBOL_@` for unknown ([SpecialSymbol::UNKNOWN])
    ///
    /// Returns a result value (Err if the given &str didn't start with any of the given symbols) containing the remainder of the string and the parsed symbol.
    pub fn parse(symbol: &str) -> nom::IResult<&str, SpecialSymbol> {
        let (rest, value) = alt((
            tag("@_EPSILON_SYMBOL_@"),
            tag("@0@"),
            tag("@_IDENTITY_SYMBOL_@"),
            tag("@_UNKNOWN_SYMBOL_@"),
        ))
        .parse(symbol)?;

        let sym = match value {
            "@_EPSILON_SYMBOL_@" => SpecialSymbol::EPSILON,
            "@0@" => SpecialSymbol::EPSILON,
            "@_IDENTITY_SYMBOL_@" => SpecialSymbol::IDENTITY,
            "@_UNKNOWN_SYMBOL_@" => SpecialSymbol::UNKNOWN,
            _ => panic!(),
        };
        Ok((rest, sym))
    }

    fn _from_symbol_string(symbol: &str) -> KFSTResult<Self> {
        match SpecialSymbol::parse(symbol) {
            Ok(("", result)) => KFSTResult::Ok(result),
            _ => value_error(format!("Not a valid SpecialSymbol: {:?}", symbol)),
        }
    }

    #[cfg(not(feature = "python"))]
    /// Parse a special symbol from a text representation.
    ///
    /// ```rust
    /// use kfst_rs::SpecialSymbol;
    ///
    /// assert_eq!(SpecialSymbol::from_symbol_string("@_EPSILON_SYMBOL_@"), Ok(SpecialSymbol::EPSILON));
    /// // Or alternatively
    /// assert_eq!(SpecialSymbol::from_symbol_string("@0@"), Ok(SpecialSymbol::EPSILON));
    /// assert_eq!(SpecialSymbol::from_symbol_string("@_IDENTITY_SYMBOL_@"), Ok(SpecialSymbol::IDENTITY));
    /// assert_eq!(SpecialSymbol::from_symbol_string("@_UNKNOWN_SYMBOL_@"), Ok(SpecialSymbol::UNKNOWN));
    /// assert_eq!(SpecialSymbol::from_symbol_string("@_GARBAGE_SYMBOL_@"), Err("Not a valid SpecialSymbol: \"@_GARBAGE_SYMBOL_@\"".to_string()));
    /// ```
    pub fn from_symbol_string(symbol: &str) -> KFSTResult<Self> {
        SpecialSymbol::_from_symbol_string(symbol)
    }
}

impl PartialOrd for SpecialSymbol {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SpecialSymbol {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // This should be clean; there is a bijection between all special symbols and a subset of strings
        other.get_symbol().cmp(&self.get_symbol())
    }
}

#[cfg_attr(feature = "python", pymethods)]
impl SpecialSymbol {
    /// Whether this symbol is ε. (See [Symbol::is_epsilon] for the general case)
    ///
    /// Returns true for [SpecialSymbol::EPSILON] and false otherwise.
    pub fn is_epsilon(&self) -> bool {
        self == &SpecialSymbol::EPSILON
    }

    /// Whether this symbol is unknown. (See [Symbol::is_unknown] for the general case)
    ///
    /// Always returns false.
    pub fn is_unknown(&self) -> bool {
        false
    }

    /// Textual representation of this symbol. Note that the `@0@` synonym for `@_EPSILON_SYMBOL_@` is always converted to the long form.
    ///
    /// ```rust
    /// use kfst_rs::SpecialSymbol;
    /// assert_eq!(SpecialSymbol::from_symbol_string("@0@").unwrap().get_symbol(), "@_EPSILON_SYMBOL_@".to_string())
    /// ```
    pub fn get_symbol(&self) -> String {
        match self {
            SpecialSymbol::EPSILON => "@_EPSILON_SYMBOL_@".to_string(),
            SpecialSymbol::IDENTITY => "@_IDENTITY_SYMBOL_@".to_string(),
            SpecialSymbol::UNKNOWN => "@_UNKNOWN_SYMBOL_@".to_string(),
        }
    }

    #[cfg(feature = "python")]
    #[staticmethod]
    fn from_symbol_string(symbol: &str) -> KFSTResult<Self> {
        SpecialSymbol::_from_symbol_string(symbol)
    }

    #[cfg(feature = "python")]
    #[staticmethod]
    fn is_special_symbol(symbol: &str) -> bool {
        SpecialSymbol::from_symbol_string(symbol).is_ok()
    }

    #[cfg(not(feature = "python"))]
    /// Is `symbol` a valid [SpecialSymbol]?
    /// Attempts to parse `symbol` using [SpecialSymbol::from_symbol_string] and returns `true` if this succeeds.
    /// ```rust
    /// use kfst_rs::SpecialSymbol;
    ///
    /// assert!(SpecialSymbol::is_special_symbol("@0@"));
    /// assert!(SpecialSymbol::is_special_symbol("@_EPSILON_SYMBOL_@"));
    /// assert!(SpecialSymbol::is_special_symbol("@_IDENTITY_SYMBOL_@"));
    /// assert!(SpecialSymbol::is_special_symbol("@_UNKNOWN_SYMBOL_@"));
    /// assert_eq!(SpecialSymbol::is_special_symbol("@_GARBAGE_SYMBOL_@"), false);
    /// ```
    pub fn is_special_symbol(symbol: &str) -> bool {
        SpecialSymbol::from_symbol_string(symbol).is_ok()
    }
}

impl std::fmt::Debug for SpecialSymbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get_symbol())
    }
}

impl std::fmt::Debug for StringSymbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get_symbol())
    }
}

#[cfg(feature = "python")]
#[pyfunction]
fn from_symbol_string(symbol: &str, py: Python) -> PyResult<Py<PyAny>> {
    Symbol::parse(symbol).unwrap().1.into_py_any(py)
}

#[cfg(not(feature = "python"))]
/// Parse a string into a Symbol; see [Symbol::parse] for implementation details.
/// ```rust
/// use kfst_rs::{from_symbol_string, Symbol, StringSymbol, SpecialSymbol};
///
/// assert_eq!(from_symbol_string("example").unwrap(), Symbol::String(StringSymbol::parse("example").unwrap().1));
/// assert_eq!(from_symbol_string("@_EPSILON_SYMBOL_@").unwrap(), Symbol::Special(SpecialSymbol::EPSILON));
///
/// ```
pub fn from_symbol_string(symbol: &str) -> Option<Symbol> {
    Symbol::parse(symbol).ok().map(|(_, sym)| sym)
}

/// A wrapper enum for different concrete symbol types. It exists to provide a dense tagged avoiding dynamic dispatch.
/// It also deals with converting symbols between Rust and Python when using kfst_rs as a Python library. (crate feature "python")
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Symbol {
    /// Wrapper around [SpecialSymbol].
    Special(SpecialSymbol),
    /// Wrapper around [FlagDiacriticSymbol].
    Flag(FlagDiacriticSymbol),
    /// Wrapper around [StringSymbol].
    String(StringSymbol),
    #[cfg(feature = "python")]
    /// Wrapper around [PyObjectSymbol] (only build with crate feature "python")
    External(PyObjectSymbol),
    /// Wrapper around [RawSymbol].
    Raw(RawSymbol),
}

#[cfg(feature = "python")]
impl<'py> IntoPyObject<'py> for Symbol {
    type Target = PyAny;

    type Output = Bound<'py, Self::Target>;

    type Error = pyo3::PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        match self {
            Symbol::Special(special_symbol) => special_symbol.into_bound_py_any(py),
            Symbol::Flag(flag_diacritic_symbol) => flag_diacritic_symbol.into_bound_py_any(py),
            Symbol::String(string_symbol) => string_symbol.into_bound_py_any(py),
            Symbol::External(pyobject_symbol) => pyobject_symbol.into_bound_py_any(py),
            Symbol::Raw(raw_symbol) => raw_symbol.into_bound_py_any(py),
        }
    }
}

impl PartialOrd for Symbol {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Symbol {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            // If we have the same type of symbol on both sides, defer to cmp (well-defined)
            (Symbol::Special(a), Symbol::Special(b)) => a.cmp(b),
            (Symbol::Flag(a), Symbol::Flag(b)) => a.cmp(b),
            (Symbol::String(a), Symbol::String(b)) => a.cmp(b),
            #[cfg(feature = "python")]
            (Symbol::External(a), Symbol::External(b)) => a.cmp(b),
            (Symbol::Raw(a), Symbol::Raw(b)) => a.cmp(b),

            // If we have different types of symbols, they can't be strictly equal
            // First, defer to the natural (reverse :D) ordering of strings
            (a, b) => match b.get_symbol().cmp(&a.get_symbol()) {
                // We can have a (degenerate) case where the symbol string is the same
                // This should never happen if the symbols are parsed from att / kfst
                // At any rate, exactly one of the symbols must be a string symbol (we checked for same type earlier)
                // We are going to arbitrarily say that string symbols are lesser
                std::cmp::Ordering::Equal => match (a, b) {
                    (Symbol::String(_), _) => std::cmp::Ordering::Less,
                    (_, Symbol::String(_)) => std::cmp::Ordering::Greater,

                    // Shouldn't be possible but let's have an informative panic
                    _ => panic!(
                        "Symbols {:?} and {:?} are being compared and are found to be equal",
                        self, other
                    ),
                },
                x => x,
            },
        }
    }
}

impl Symbol {
    /// Is this symbol to be treated as an ε symbol?
    /// ε symbols get matched without consuming input.
    /// The simplest ε symbol is the one defined in [SpecialSymbol::EPSILON] and represented interchangeably by `@0@` and `@_EPSILON_SYMBOL_@`.
    /// All [FlagDiacriticSymbols](FlagDiacriticSymbol) are also ε symbols, as they do not consume input.
    /// Their string representations are of the form `@X.A@` or `@X.A.B@` where `X` is a [FlagDiacriticType] and `A` and `B` are arbitrary strings.
    /// [FST::run_fst] (and thus [FST::lookup]) drops any symbols on the output side for which this methods returns `true`.
    pub fn is_epsilon(&self) -> bool {
        match self {
            Symbol::Special(special_symbol) => special_symbol.is_epsilon(),
            Symbol::Flag(flag_diacritic_symbol) => flag_diacritic_symbol.is_epsilon(),
            Symbol::String(string_symbol) => string_symbol.is_epsilon(),
            #[cfg(feature = "python")]
            Symbol::External(py_object_symbol) => py_object_symbol.is_epsilon(),
            Symbol::Raw(raw_symbol) => raw_symbol.is_epsilon(),
        }
    }

    /// Is this symbol to be treated as an unknown symbol?
    /// Unknown symbols are accepted by the [`@_IDENTITY_SYMBOL_@`](SpecialSymbol::IDENTITY) and [`@_UNKNOWN_SYMBOL_@`](SpecialSymbol::UNKNOWN) special symbols.
    pub fn is_unknown(&self) -> bool {
        match self {
            Symbol::Special(special_symbol) => special_symbol.is_unknown(),
            Symbol::Flag(flag_diacritic_symbol) => flag_diacritic_symbol.is_unknown(),
            Symbol::String(string_symbol) => string_symbol.is_unknown(),
            #[cfg(feature = "python")]
            Symbol::External(py_object_symbol) => py_object_symbol.is_unknown(),
            Symbol::Raw(raw_symbol) => raw_symbol.is_unknown(),
        }
    }

    /// Get the string-representation of this symbol.
    pub fn get_symbol(&self) -> String {
        match self {
            Symbol::Special(special_symbol) => special_symbol.get_symbol(),
            Symbol::Flag(flag_diacritic_symbol) => flag_diacritic_symbol.get_symbol(),
            Symbol::String(string_symbol) => string_symbol.get_symbol(),
            #[cfg(feature = "python")]
            Symbol::External(py_object_symbol) => py_object_symbol.get_symbol(),
            Symbol::Raw(raw_symbol) => raw_symbol.get_symbol(),
        }
    }
}

impl Symbol {
    /// Parses a string into a [Symbol]. This tries the following conversions in order:
    ///
    /// 1. [FlagDiacriticSymbol] and the [Symbol::Flag] variant.
    /// 2. [SpecialSymbol] and the [Symbol::Special] variant.
    /// 3. [StringSymbol] and the [Symbol::String] variant.
    ///
    /// Therefore Symbol::Exernal (only built with feature "python") and [Symbol::Raw] variants cannot be constructed with this method.
    ///
    /// ```rust
    /// use kfst_rs::{Symbol, FlagDiacriticSymbol, SpecialSymbol, StringSymbol};
    ///
    /// assert_eq!(Symbol::parse("@D.X.Y@").unwrap().1, Symbol::Flag(FlagDiacriticSymbol::parse("@D.X.Y@").unwrap().1));
    /// assert_eq!(Symbol::parse("@_EPSILON_SYMBOL_@").unwrap().1, Symbol::Special(SpecialSymbol::parse("@_EPSILON_SYMBOL_@").unwrap().1));
    /// assert_eq!(Symbol::parse("ladybird").unwrap().1, Symbol::String(StringSymbol::parse("ladybird").unwrap().1));
    /// ```
    ///
    /// Fails when if and only if [StringSymbol::parse] fails: on an empty string.
    pub fn parse(symbol: &str) -> nom::IResult<&str, Symbol> {
        let mut parser = alt((
            |x| {
                (FlagDiacriticSymbol::parse, nom::combinator::eof)
                    .parse(x)
                    .map(|y| (y.0, Symbol::Flag(y.1 .0)))
            },
            |x| {
                (SpecialSymbol::parse, nom::combinator::eof)
                    .parse(x)
                    .map(|y| (y.0, Symbol::Special(y.1 .0)))
            },
            |x| StringSymbol::parse(x).map(|y| (y.0, Symbol::String(y.1))),
        ));
        parser.parse(symbol)
    }
}

#[cfg(feature = "python")]
impl FromPyObject<'_> for Symbol {
    fn extract_bound(ob: &Bound<'_, PyAny>) -> PyResult<Self> {
        ob.extract()
            .map(Symbol::Special)
            .or_else(|_| ob.extract().map(Symbol::Flag))
            .or_else(|_| ob.extract().map(Symbol::String))
            .or_else(|_| ob.extract().map(Symbol::External))
            .or_else(|_| ob.extract().map(Symbol::Raw))
    }
}
#[derive(Clone, Debug, PartialEq, Hash)]
#[readonly::make]
/// The flag state of an [FSTState]:
/// ```no_test
/// (name -> (direction of setting where true is positive, value))
/// ```
/// name and value are interned string indices.
pub struct FlagMap(pub im::HashMap<u32, (bool, u32)>);

#[cfg(feature = "python")]
impl FromPyObject<'_> for FlagMap {
    fn extract_bound(ob: &Bound<'_, PyAny>) -> PyResult<Self> {
        let as_index_map: std::collections::HashMap<String, (bool, String)> = ob.extract()?;
        let as_map: im::HashMap<_, _> = as_index_map
            .into_iter()
            .map(|(key, value)| (intern(key), (value.0, intern(value.1))))
            .collect();
        Ok(FlagMap(as_map))
    }
}

#[cfg(feature = "python")]
impl<'py> IntoPyObject<'py> for FlagMap {
    type Target = PyDict;

    type Output = Bound<'py, Self::Target>;

    type Error = pyo3::PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        self.0
            .into_iter()
            .collect::<std::collections::HashMap<_, _>>()
            .into_pyobject(py)
    }
}

// transducer.py

#[cfg_attr(feature = "python", pyclass(frozen, eq, hash, get_all))]
#[derive(Clone, Debug, PartialEq)]
#[readonly::make]
/// A state in an [FST].
/// Not only does this contain the state number itself,
/// but also the path weight so far, the output symbol sequence
/// and the input and output flag state.
pub struct FSTState {
    /// Number of the state in the FST.
    pub state_num: u64,
    /// Sum of transition weights so far.
    pub path_weight: f64,
    /// Mapping from flags to what they are set to (input side)
    pub input_flags: FlagMap,
    /// Mapping from flags to what they are set to (output side)
    pub output_flags: FlagMap,
    /// Output side symbols for the transduction so far.
    pub output_symbols: Vec<Symbol>,
}

impl Default for FSTState {
    /// Produce a neutral start state: number 0, no weight, empty flags and empty output.
    fn default() -> Self {
        Self {
            state_num: 0,
            path_weight: 0.0,
            input_flags: FlagMap(im::HashMap::new()),
            output_flags: FlagMap(im::HashMap::new()),
            output_symbols: vec![],
        }
    }
}

impl Hash for FSTState {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.state_num.hash(state);
        self.path_weight.to_be_bytes().hash(state);
        self.input_flags.hash(state);
        self.output_flags.hash(state);
        self.output_symbols.hash(state);
    }
}

fn _test_flag(stored_val: &(bool, u32), queried_val: u32) -> bool {
    stored_val.0 == (stored_val.1 == queried_val)
}

impl FSTState {
    fn _new(state: u64) -> Self {
        FSTState {
            state_num: state,
            path_weight: 0.0,
            input_flags: FlagMap(im::HashMap::new()),
            output_flags: FlagMap(im::HashMap::new()),
            output_symbols: vec![],
        }
    }

    fn __new(
        state: u64,
        path_weight: f64,
        input_flags: IndexMap<String, (bool, String)>,
        output_flags: IndexMap<String, (bool, String)>,
        output_symbols: Vec<Symbol>,
    ) -> Self {
        FSTState {
            state_num: state,
            path_weight,
            input_flags: FlagMap(
                input_flags
                    .into_iter()
                    .map(|(key, value)| (intern(key), (value.0, intern(value.1))))
                    .collect(),
            ),
            output_flags: FlagMap(
                output_flags
                    .into_iter()
                    .map(|(key, value)| (intern(key), (value.0, intern(value.1))))
                    .collect(),
            ),
            output_symbols,
        }
    }

    #[cfg(not(feature = "python"))]
    /// Construct a new FSTState. All arguments are per FSTState fields, except for the flag states.
    /// These are not a [FlagMap] but and IndexMap of (name -> (direction of setting where true is positively set, value))
    /// where name and value get interned.
    pub fn new(
        state: u64,
        path_weight: f64,
        input_flags: IndexMap<String, (bool, String)>,
        output_flags: IndexMap<String, (bool, String)>,
        output_symbols: Vec<Symbol>,
    ) -> Self {
        FSTState::__new(
            state,
            path_weight,
            input_flags,
            output_flags,
            output_symbols,
        )
    }
}

#[cfg_attr(feature = "python", pymethods)]
impl FSTState {
    #[cfg(feature = "python")]
    #[new]
    #[pyo3(signature = (state, path_weight=0.0, input_flags=IndexMap::new(), output_flags=IndexMap::new(), output_symbols=vec![]))]
    fn new(
        state: u64,
        path_weight: f64,
        input_flags: IndexMap<String, (bool, String)>,
        output_flags: IndexMap<String, (bool, String)>,
        output_symbols: Vec<Symbol>,
    ) -> Self {
        FSTState::__new(
            state,
            path_weight,
            input_flags,
            output_flags,
            output_symbols,
        )
    }

    #[deprecated]
    /// Python-style string representation.
    pub fn __repr__(&self) -> String {
        format!(
            "FSTState({}, {}, {:?}, {:?}, {:?})",
            self.state_num,
            self.path_weight,
            self.input_flags,
            self.output_flags,
            self.output_symbols
        )
    }
}

#[cfg_attr(feature = "python", pyclass(frozen, get_all))]
#[readonly::make]
/// A finite state transducer.
/// Constructed using [FST::from_kfst_bytes] or [FST::from_att_rows] from an in-memory representation or [FST::from_att_file] and [FST::from_kfst_file] from the file system.
///
/// To run an existing transducer (here Voikko):
///
/// ```rust
/// use kfst_rs::{FSTState, FST};
/// use std::io::{self, Write};
///
/// // Read in transducer
///
/// # let pathtovoikko = "../pyvoikko/pyvoikko/voikko.kfst".to_string();
/// let fst = FST::from_kfst_file(pathtovoikko, true).unwrap();
/// // Alternatively, for ATT use FST::from_att_file
///
/// // Read in word to analyze
///
/// let mut buffer = String::new();
/// let stdin = io::stdin();
/// stdin.read_line(&mut buffer).unwrap();
/// buffer = buffer.trim().to_string();
///
/// // Do analysis proper
///
/// match fst.lookup(&buffer, FSTState::default(), true) {
///     Ok(result) => {
///         for (i, analysis) in result.into_iter().enumerate() {
///             println!("Analysis {}: {} ({})", i+1, analysis.0, analysis.1)
///         }
///     },
///     Err(err) => println!("No analysis: {:?}", err),
/// }
/// ```
/// Given the input "lentokoneessa", this gives the following analysis:
///
/// ```text
/// Analysis 1: [Lt][Xp]lentää[X]len[Ln][Xj]to[X]to[Sn][Ny][Bh][Bc][Ln][Xp]kone[X]konee[Sine][Ny]ssa (0)
/// ```
pub struct FST {
    /// A mapping from the index of a final state to its weight.
    pub final_states: IndexMap<u64, f64>,
    /// The transition rules of this FST: (state number -> (top symbol -> list of target state indices, bottom symbols and weights))
    pub rules: IndexMap<u64, IndexMap<Symbol, Vec<(u64, Symbol, f64)>>>,
    /// List of all the symbols in the transducer (useful for tokenization). Sorted in reverse order by length.
    pub symbols: Vec<Symbol>,
    /// Whether this FST is in debug mode; kept for compatibility with the python implementation of KFST. It's effects on FST behaviour are undefined.
    #[deprecated]
    pub debug: bool,
}

impl FST {
    fn _run_fst(
        &self,
        input_symbols: &[Symbol],
        state: &FSTState,
        post_input_advance: bool,
        result: &mut Vec<(bool, bool, FSTState)>,
    ) {
        let transitions = self.rules.get(&state.state_num);
        let isymbol = if input_symbols.is_empty() {
            match self.final_states.get(&state.state_num) {
                Some(&weight) => {
                    // Update weight of state to account for weight of final state
                    result.push((
                        true,
                        post_input_advance,
                        FSTState {
                            state_num: state.state_num,
                            path_weight: state.path_weight + weight,
                            input_flags: state.input_flags.clone(),
                            output_flags: state.output_flags.clone(),
                            output_symbols: state.output_symbols.clone(),
                        },
                    ));
                }
                None => {
                    // Not a final state
                    result.push((false, post_input_advance, state.clone()));
                }
            }
            None
        } else {
            Some(&input_symbols[0])
        };
        if let Some(transitions) = transitions {
            for transition_isymbol in transitions.keys() {
                if transition_isymbol.is_epsilon() || isymbol == Some(transition_isymbol) {
                    self._transition(
                        input_symbols,
                        state,
                        &transitions[transition_isymbol],
                        isymbol,
                        transition_isymbol,
                        result,
                    );
                }
            }
            if let Some(isymbol) = isymbol {
                if isymbol.is_unknown() {
                    if let Some(transition_list) =
                        transitions.get(&Symbol::Special(SpecialSymbol::UNKNOWN))
                    {
                        self._transition(
                            input_symbols,
                            state,
                            transition_list,
                            Some(isymbol),
                            &Symbol::Special(SpecialSymbol::UNKNOWN),
                            result,
                        );
                    }

                    if let Some(transition_list) =
                        transitions.get(&Symbol::Special(SpecialSymbol::IDENTITY))
                    {
                        self._transition(
                            input_symbols,
                            state,
                            transition_list,
                            Some(isymbol),
                            &Symbol::Special(SpecialSymbol::IDENTITY),
                            result,
                        );
                    }
                }
            }
        }
    }

    fn _transition(
        &self,
        input_symbols: &[Symbol],
        state: &FSTState,
        transitions: &[(u64, Symbol, f64)],
        isymbol: Option<&Symbol>,
        transition_isymbol: &Symbol,
        result: &mut Vec<(bool, bool, FSTState)>,
    ) {
        for (next_state, osymbol, weight) in transitions.iter() {
            let new_output_flags = _update_flags(osymbol, &state.output_flags.0);
            let new_input_flags = _update_flags(transition_isymbol, &state.input_flags.0);

            match (new_output_flags, new_input_flags) {
                (Some(new_output_flags), Some(new_input_flags)) => {
                    let mut new_output_symbols: Vec<Symbol> = state.output_symbols.clone();
                    match (isymbol, osymbol) {
                        (Some(isymbol), Symbol::Special(SpecialSymbol::IDENTITY)) => {
                            new_output_symbols.push(isymbol.clone())
                        }
                        _ => {
                            if !osymbol.is_epsilon() {
                                new_output_symbols.push(osymbol.clone())
                            }
                        }
                    };
                    let new_state = FSTState {
                        state_num: *next_state,
                        path_weight: state.path_weight + *weight,
                        input_flags: FlagMap(new_input_flags),
                        output_flags: FlagMap(new_output_flags),
                        output_symbols: new_output_symbols,
                    };
                    if transition_isymbol.is_epsilon() {
                        self._run_fst(input_symbols, &new_state, input_symbols.is_empty(), result);
                    } else {
                        let cloned_symbols = &input_symbols[1..];
                        self._run_fst(cloned_symbols, &new_state, false, result);
                    }
                }
                _ => continue,
            }
        }
    }

    /// Construct an instance of FST from of rows matching those in an att file (see [FST::from_att_code]) that have been parsed into tuples.
    /// Thee representation is read:
    /// ```no_test
    /// Ok((number of a final state, weight of the final state))
    /// Err((source state of transition, target state of transition, top symbol, bottom symbol, weight))
    /// ```
    /// Debug is passed along to [FST::debug].
    pub fn from_att_rows(
        rows: Vec<Result<(u64, f64), (u64, u64, Symbol, Symbol, f64)>>,
        debug: bool,
    ) -> FST {
        let mut final_states: IndexMap<u64, f64> = IndexMap::new();
        let mut rules: IndexMap<u64, IndexMap<Symbol, Vec<(u64, Symbol, f64)>>> = IndexMap::new();
        let mut symbols: IndexSet<Symbol> = IndexSet::new();
        for line in rows.into_iter() {
            match line {
                Ok((state_number, state_weight)) => {
                    final_states.insert(state_number, state_weight);
                }
                Err((state_1, state_2, top_symbol, bottom_symbol, weight)) => {
                    rules.entry(state_1).or_default();
                    let handle = rules.get_mut(&state_1).unwrap();
                    if !handle.contains_key(&top_symbol) {
                        handle.insert(top_symbol.clone(), vec![]);
                    }
                    handle.get_mut(&top_symbol).unwrap().push((
                        state_2,
                        bottom_symbol.clone(),
                        weight,
                    ));
                    symbols.insert(top_symbol);
                    symbols.insert(bottom_symbol);
                }
            }
        }
        FST::from_rules(
            final_states,
            rules,
            symbols.into_iter().collect(),
            Some(debug),
        )
    }

    fn _from_kfst_bytes(kfst_bytes: &[u8]) -> Result<FST, String> {
        // Ownership makes error handling such a pain that it makes more sense to just return an option
        // We need to parse part of the data from an owned buffer and it just makes this too comples

        // Check that this is v0 kfst format

        let mut header = nom::sequence::preceded(
            nom::bytes::complete::tag("KFST"),
            nom::number::complete::be_u16::<&[u8], ()>,
        );
        let (rest, version) = header
            .parse(kfst_bytes)
            .map_err(|_| "Failed to parse header")?;
        assert!(version == 0);

        // Read metadata

        let mut metadata = (
            nom::number::complete::be_u16::<&[u8], ()>,
            nom::number::complete::be_u32,
            nom::number::complete::be_u32,
            nom::number::complete::u8,
        );
        let (rest, (num_symbols, num_transitions, num_final_states, is_weighted)) = metadata
            .parse(rest)
            .map_err(|_| "Failed to parse metadata")?;
        let num_transitions: usize = num_transitions
            .try_into()
            .map_err(|_| "usize too small to represent transitions")?;
        let num_final_states: usize = num_final_states
            .try_into()
            .map_err(|_| "usize too small to represent final states")?;
        // Safest conversion I can think of; theoretically it should only be 1 or 0 but Python just defers to C and C doesn't have its act together on this.
        let is_weighted: bool = is_weighted != 0u8;

        // Parse out symbols

        let mut symbol = nom::multi::count(
            nom::sequence::terminated(nom::bytes::complete::take_until1("\0"), tag("\0")),
            num_symbols.into(),
        );
        let (rest, symbols) = symbol
            .parse(rest)
            .map_err(|_: nom::Err<()>| "Failed to parse symbol list")?;
        let symbol_strings: Vec<&str> = symbols
            .into_iter()
            .map(|x| std::str::from_utf8(x))
            .collect::<Result<Vec<&str>, _>>()
            .map_err(|x| format!("Some symbol was not valid utf-8: {}", x))?;
        let symbol_list: Vec<Symbol> = symbol_strings
            .iter()
            .map(|x| {
                Symbol::parse(x)
                    .map_err(|x| {
                        format!(
                            "Some symbol while valid utf8 was not a valid symbol specifier: {}",
                            x
                        )
                    })
                    .and_then(|(extra, sym)| {
                        if extra.is_empty() {
                            Ok(sym)
                        } else {
                            Err(format!(
                                "Extra data after end of symbol {}: {:?}",
                                sym.get_symbol(),
                                extra
                            ))
                        }
                    })
            })
            .collect::<Result<Vec<Symbol>, _>>()?;
        let symbol_objs: IndexSet<Symbol> = symbol_list.iter().cloned().collect();

        // From here on, data is lzma-compressed

        let mut decomp: Vec<u8> = Vec::new();
        let mut rest_ = rest;
        lzma_rs::xz_decompress(&mut rest_, &mut decomp)
            .map_err(|_| "Failed to lzma-decompress remainder of file")?;

        // The decompressed data is - unavoidably - owned by the function
        // We promise an error type of &[u8], which we can't provide from here because of lifetimes

        let transition_syntax = (
            nom::number::complete::be_u32::<&[u8], ()>,
            nom::number::complete::be_u32,
            nom::number::complete::be_u16,
            nom::number::complete::be_u16,
        );
        let weight_parser = if is_weighted {
            nom::number::complete::be_f64
        } else {
            |input| Ok((input, 0.0)) // Conjure up a default weight out of thin air
        };
        let (rest, file_rules) = many_m_n(
            num_transitions,
            num_transitions,
            (transition_syntax, weight_parser),
        )
        .parse(decomp.as_slice())
        .map_err(|_| "Broken transition table")?;

        let (rest, final_states) = many_m_n(
            num_final_states,
            num_final_states,
            (nom::number::complete::be_u32, weight_parser),
        )
        .parse(rest)
        .map_err(|_| "Broken final states")?;

        if !rest.is_empty() {
            Err(format!("lzma-compressed payload is {} bytes long when decompressed but given the header, there seems to be {} bytes extra.", decomp.len(), rest.len()))?;
        }

        // We have a vec, we want a hash map and our numbers to be i64 instead of u32

        let final_states = final_states
            .into_iter()
            .map(|(a, b)| (a.into(), b))
            .collect();

        // These should be a hash map instead of a vector

        let symbols = symbol_objs.into_iter().collect();

        // We need to construct the right rule data structure

        let mut rules: IndexMap<u64, IndexMap<Symbol, Vec<(u64, Symbol, f64)>>> = IndexMap::new();
        for ((from_state, to_state, top_symbol_idx, bottom_symbol_idx), weight) in
            file_rules.into_iter()
        {
            let from_state = from_state.into();
            let to_state = to_state.into();
            let top_symbol_idx: usize = top_symbol_idx.into();
            let bottom_symbol_idx: usize = bottom_symbol_idx.into();
            let top_symbol = symbol_list[top_symbol_idx].clone();
            let bottom_symbol = symbol_list[bottom_symbol_idx].clone();
            rules.entry(from_state).or_default();
            let handle = rules.get_mut(&from_state).unwrap();
            if !handle.contains_key(&top_symbol) {
                handle.insert(top_symbol.clone(), vec![]);
            }
            handle
                .get_mut(&top_symbol)
                .unwrap()
                .push((to_state, bottom_symbol.clone(), weight));
        }

        Ok(FST::from_rules(final_states, rules, symbols, None))
    }

    fn _to_kfst_bytes(&self) -> Result<Vec<u8>, String> {
        // 1. Figure out if this transducer if weighted & count transitions

        let mut weighted = false;

        for (_, &weight) in self.final_states.iter() {
            if weight != 0.0 {
                weighted = true;
                break;
            }
        }

        let mut transitions: u32 = 0;

        for (_, transition_table) in self.rules.iter() {
            for transition in transition_table.values() {
                for (_, _, weight) in transition.iter() {
                    if (*weight) != 0.0 {
                        weighted = true;
                    }
                    transitions += 1;
                }
            }
        }

        // Construct header

        let mut result: Vec<u8> = "KFST".into();
        result.extend(0u16.to_be_bytes());
        let symbol_len: u16 = self
            .symbols
            .len()
            .try_into()
            .map_err(|x| format!("Too many symbols to represent as u16: {}", x))?;
        result.extend(symbol_len.to_be_bytes());
        result.extend(transitions.to_be_bytes());
        let num_states: u32 = self
            .final_states
            .len()
            .try_into()
            .map_err(|x| format!("Too many final states to represent as u32: {}", x))?;
        result.extend(num_states.to_be_bytes());
        result.push(weighted.into()); // Promises 0 for false and 1 for true

        // Dump symbols

        for symbol in self.symbols.iter() {
            result.extend(symbol.get_symbol().into_bytes());
            result.push(0); // Add null-terminators
        }

        // lzma-compressed part of payload

        let mut to_compress: Vec<u8> = vec![];

        // Push transition table to compressible buffer

        for (source_state, transition_table) in self.rules.iter() {
            for (top_symbol, transition) in transition_table.iter() {
                for (target_state, bottom_symbol, weight) in transition.iter() {
                    let source_state: usize = (*source_state).try_into().map_err(|x| {
                        format!(
                            "Can't represent source state {} as u32: {}",
                            source_state, x
                        )
                    })?;
                    let target_state: usize = (*target_state).try_into().map_err(|x| {
                        format!(
                            "Can't represent target state {} as u32: {}",
                            target_state, x
                        )
                    })?;
                    let top_index: u16 = self
                        .symbols
                        .binary_search(top_symbol)
                        .map_err(|_| {
                            format!("Top symbol {:?} not found in FST symbol list", top_symbol)
                        })
                        .and_then(|x| {
                            x.try_into().map_err(|x| {
                                format!("Can't represent top symbol index as u16: {}", x)
                            })
                        })?;
                    let bottom_index: u16 = self
                        .symbols
                        .binary_search(bottom_symbol)
                        .map_err(|_| {
                            format!("Top symbol {:?} not found in FST symbol list", top_symbol)
                        })
                        .and_then(|x| {
                            x.try_into().map_err(|x| {
                                format!("Can't represent bottom symbol index as u16: {}", x)
                            })
                        })?;
                    to_compress.extend(source_state.to_be_bytes());
                    to_compress.extend(target_state.to_be_bytes());
                    to_compress.extend(top_index.to_be_bytes());
                    to_compress.extend(bottom_index.to_be_bytes());
                    if weighted {
                        to_compress.extend(weight.to_be_bytes());
                    } else {
                        assert!(*weight == 0.0);
                    }
                }
            }
        }

        // Push final states to compressible buffer

        for (&final_state, weight) in self.final_states.iter() {
            let final_state: u32 = final_state
                .try_into()
                .map_err(|x| format!("Can't represent final state index as u32: {}", x))?;
            to_compress.extend(final_state.to_be_bytes());
            if weighted {
                to_compress.extend(weight.to_be_bytes());
            } else {
                assert!(*weight == 0.0);
            }
        }

        // Compress compressible buffer

        let mut compressed = vec![];
        lzma_compress(&mut to_compress.as_slice(), &mut compressed)
            .map_err(|x| format!("Failed while compressing with lzma_rs: {}", x))?;
        result.extend(compressed);

        Ok(result)
    }

    fn _from_rules(
        final_states: IndexMap<u64, f64>,
        rules: IndexMap<u64, IndexMap<Symbol, Vec<(u64, Symbol, f64)>>>,
        symbols: HashSet<Symbol>,
        debug: Option<bool>,
    ) -> FST {
        let mut new_symbols: Vec<Symbol> = symbols.into_iter().collect();
        // Sort by normal comparison but in reverse; this guarantees reverse order by length and also
        // That different-by-symbol-string symbols get treated differently
        new_symbols.sort();
        FST {
            final_states,
            rules,
            symbols: new_symbols,
            debug: debug.unwrap_or(false),
        }
    }

    /// Construct an instance of FST from the fields that make up FST. (See [FST::final_states], [FST::rules], [FST::symbols] and [FST::debug] for more information.)
    #[cfg(not(feature = "python"))]
    pub fn from_rules(
        final_states: IndexMap<u64, f64>,
        rules: IndexMap<u64, IndexMap<Symbol, Vec<(u64, Symbol, f64)>>>,
        symbols: HashSet<Symbol>,
        debug: Option<bool>,
    ) -> FST {
        FST::_from_rules(final_states, rules, symbols, debug)
    }

    fn _from_att_file(att_file: String, debug: bool) -> KFSTResult<FST> {
        // Debug should default to false, pyo3 doesn't make that particularly easy
        match File::open(Path::new(&att_file)) {
            Ok(mut file) => {
                let mut att_code = String::new();
                file.read_to_string(&mut att_code).map_err(|err| {
                    io_error::<()>(format!("Failed to read from file {}:\n{}", att_file, err))
                        .unwrap_err()
                })?;
                FST::from_att_code(att_code, debug)
            }
            Err(err) => io_error(format!("Failed to open file {}:\n{}", att_file, err)),
        }
    }

    #[cfg(not(feature = "python"))]
    /// Construct an instance of FST from ATT code that resides on the file system.
    /// See [FST::from_att_code] for more details of what ATT code is.
    pub fn from_att_file(att_file: String, debug: bool) -> KFSTResult<FST> {
        FST::_from_att_file(att_file, debug)
    }

    fn _from_att_code(att_code: String, debug: bool) -> KFSTResult<FST> {
        let mut rows: Vec<Result<(u64, f64), (u64, u64, Symbol, Symbol, f64)>> = vec![];

        for (lineno, line) in att_code.lines().enumerate() {
            let elements: Vec<&str> = line.split("\t").collect();
            if elements.len() == 1 || elements.len() == 2 {
                let state = elements[0].parse::<u64>().ok();
                let weight = if elements.len() == 1 {
                    Some(0.0)
                } else {
                    elements[1].parse::<f64>().ok()
                };
                match (state, weight) {
                    (Some(state), Some(weight)) => {
                        rows.push(Ok((state, weight)));
                    }
                    _ => {
                        return value_error(format!(
                            "Failed to parse att code on line {}:\n{}",
                            lineno, line
                        ))
                    }
                }
            } else if elements.len() == 4 || elements.len() == 5 {
                let state_1 = elements[0].parse::<u64>().ok();
                let state_2 = elements[1].parse::<u64>().ok();
                let symbol_1 = Symbol::parse(elements[2]).ok();
                let symbol_2 = Symbol::parse(elements[3]).ok();
                let weight = if elements.len() == 4 {
                    Some(0.0)
                } else {
                    elements[4].parse::<f64>().ok()
                };
                match (state_1, state_2, symbol_1, symbol_2, weight) {
                    (
                        Some(state_1),
                        Some(state_2),
                        Some(("", symbol_1)),
                        Some(("", symbol_2)),
                        Some(weight),
                    ) => {
                        rows.push(Err((state_1, state_2, symbol_1, symbol_2, weight)));
                    }
                    _ => {
                        return value_error(format!(
                            "Failed to parse att code on line {}:\n{}",
                            lineno, line
                        ));
                    }
                }
            }
        }
        KFSTResult::Ok(FST::from_att_rows(rows, debug))
    }

    #[cfg(not(feature = "python"))]
    /// Construct an FST instance from the AT&T text representation. See eg. [Apertium's wiki](https://wiki.apertium.org/wiki/ATT_format). The `debug` argument is passed to [FST::debug]
    /// Both the weighted and unweighted versions are supported:
    ///
    /// ```rust
    /// use kfst_rs::FST;
    ///
    /// // With weights
    ///
    /// let weighted = r#"0	1	c	c	1.000000
    /// 0	2	d	d	2.000000
    /// 1	3	a	a	0.000000
    /// 2	4	o	o	0.000000
    /// 3	5	t	t	0.000000
    /// 4	5	g	g	0.000000
    /// 5	6	s	s	10.000000
    /// 5	0.000000
    /// 6	0.000000"#;
    ///
    /// // to_att_code doesn't guarantee that the ATT file is laid out in the same order
    ///
    /// assert_eq!(FST::from_att_code(weighted.to_string(), false).unwrap().to_att_code(), r#"5
    /// 6
    /// 0	1	c	c	1
    /// 0	2	d	d	2
    /// 1	3	a	a
    /// 2	4	o	o
    /// 3	5	t	t
    /// 4	5	g	g
    /// 5	6	s	s	10"#);
    ///
    ///
    /// // Unweighted
    ///
    /// FST::from_att_code(r#"0	1	c	c
    /// 0	2	d	d
    /// 1	3	a	a
    /// 2	4	o	o
    /// 3	5	t	t
    /// 4	5	g	g
    /// 5	6	s	s
    /// 5
    /// 6"#.to_string(), false);
    /// ```
    /// `debug` is passed along to [FST::debug].
    pub fn from_att_code(att_code: String, debug: bool) -> KFSTResult<FST> {
        FST::_from_att_code(att_code, debug)
    }

    fn _from_kfst_file(kfst_file: String, debug: bool) -> KFSTResult<FST> {
        match File::open(Path::new(&kfst_file)) {
            Ok(mut file) => {
                let mut kfst_bytes: Vec<u8> = vec![];
                file.read_to_end(&mut kfst_bytes).map_err(|err| {
                    io_error::<()>(format!("Failed to read from file {}:\n{}", kfst_file, err))
                        .unwrap_err()
                })?;
                FST::from_kfst_bytes(&kfst_bytes, debug)
            }
            Err(err) => io_error(format!("Failed to open file {}:\n{}", kfst_file, err)),
        }
    }

    /// Construct an FST instance from KFST binary representation that resides on the file system.
    /// See [FST::from_kfst_bytes] for converting memory-resident KFST binary representation into FST instances.
    /// `debug` is passed along to [FST::debug].
    #[cfg(not(feature = "python"))]
    pub fn from_kfst_file(kfst_file: String, debug: bool) -> KFSTResult<FST> {
        FST::_from_kfst_file(kfst_file, debug)
    }

    #[allow(unused)]
    fn __from_kfst_bytes(kfst_bytes: &[u8], debug: bool) -> KFSTResult<FST> {
        match FST::_from_kfst_bytes(kfst_bytes) {
            Ok(x) => Ok(x),
            Err(x) => value_error(x),
        }
    }

    /// Construct an FST instance from KFST binary representation that is resident in memory.
    /// The KFST binary representation is a mildly compressed way to represent a transducer.
    /// `debug` is passed along to [FST::debug].
    #[cfg(not(feature = "python"))]
    pub fn from_kfst_bytes(kfst_bytes: &[u8], debug: bool) -> KFSTResult<FST> {
        FST::__from_kfst_bytes(kfst_bytes, debug)
    }

    fn _split_to_symbols(&self, text: &str, allow_unknown: bool) -> Option<Vec<Symbol>> {
        let mut result = vec![];
        let mut pos = text.chars();
        'outer: while pos.size_hint().0 > 0 {
            for symbol in self.symbols.iter() {
                let symbol_string = symbol.get_symbol();
                if pos.as_str().starts_with(&symbol_string) {
                    result.push(symbol.clone());
                    // Consume correct amount of characters from iterator
                    for _ in symbol_string.chars() {
                        pos.next();
                    }
                    continue 'outer;
                }
            }
            if allow_unknown {
                result.push(Symbol::String(StringSymbol {
                    string: intern(pos.next().unwrap().to_string()),
                    unknown: true,
                }));
            } else {
                return None;
            }
        }
        Some(result)
    }

    /// Tokenize a text into Symbol instances matching this transducers alphabet ([FST::symbols]).
    /// The argument `allow_unknown` matters only if the text can not be cleanly tokenized:
    /// * If it is set to `true`, untokenizable sequences get represented as [Symbol::String] that are marked as unknown (see eg. [Symbol::is_unknown]).
    /// * If it is set to `false`, a value of [None] is returned.
    #[cfg(not(feature = "python"))]
    pub fn split_to_symbols(&self, text: &str, allow_unknown: bool) -> Option<Vec<Symbol>> {
        self._split_to_symbols(text, allow_unknown)
    }

    fn __run_fst(
        &self,
        input_symbols: Vec<Symbol>,
        state: FSTState,
        post_input_advance: bool,
    ) -> Vec<(bool, bool, FSTState)> {
        let mut result = vec![];
        self._run_fst(
            input_symbols.as_slice(),
            &state,
            post_input_advance,
            &mut result,
        );
        result
    }

    #[cfg(not(feature = "python"))]
    /// Apply this FST to a sequence of symbols `input_symbols` starting from the state `state`.
    /// The members of the elements of the returned tuple are:
    ///   * finality of the state
    ///   * the value of `post_input_advance`
    ///   * the state proper from which an output symbol sequence can be deduced.
    ///
    /// Unless you use special token types or need to do complex token manipulation, you should probably be using [FST::lookup].
    pub fn run_fst(
        &self,
        input_symbols: Vec<Symbol>,
        state: FSTState,
        post_input_advance: bool,
    ) -> Vec<(bool, bool, FSTState)> {
        self.__run_fst(input_symbols, state, post_input_advance)
    }

    fn _lookup(
        &self,
        input: &str,
        state: FSTState,
        allow_unknown: bool,
    ) -> KFSTResult<Vec<(String, f64)>> {
        let input_symbols = self.split_to_symbols(input, allow_unknown);
        match input_symbols {
            None => {
                tokenization_exception(format!("Input cannot be split into symbols: {}", input))
            }
            Some(input_symbols) => {
                let mut dedup: IndexSet<String> = IndexSet::new();
                let mut result: Vec<(String, f64)> = vec![];
                let mut finished_paths: Vec<_> = self
                    .run_fst(input_symbols.clone(), state, false)
                    .into_iter()
                    .filter(|(finished, _, _)| *finished)
                    .collect();
                finished_paths
                    .sort_by(|a, b| a.2.path_weight.partial_cmp(&b.2.path_weight).unwrap());
                for finished in finished_paths {
                    let output_string: String = finished
                        .2
                        .output_symbols
                        .iter()
                        .map(|x| x.get_symbol())
                        .collect::<Vec<String>>()
                        .join("");
                    if dedup.contains(&output_string) {
                        continue;
                    }
                    dedup.insert(output_string.clone());
                    result.push((output_string, finished.2.path_weight));
                }
                Ok(result)
            }
        }
    }

    #[cfg(not(feature = "python"))]
    /// Tokenize and transduce `input`, starting from the given `state` (note that [FSTState] implements [Default]) and either allowing or disallowing unknown tokens.
    /// (See [FST::split_to_symbols] for tokenization of unknown tokens.)
    ///
    /// If tokenization succeeds, returns a [Vec] of pairs of transduced strings and their weights.
    /// If tokenization fails, returns a [KFSTResult::Err] variant
    ///
    /// If you need more control over tokenization (or if your symbols just can not be parsed from a string representation), [FST::run_fst] might be what you are looking for.
    pub fn lookup(
        &self,
        input: &str,
        state: FSTState,
        allow_unknown: bool,
    ) -> KFSTResult<Vec<(String, f64)>> {
        self._lookup(input, state, allow_unknown)
    }
}

fn _update_flags(
    symbol: &Symbol,
    flags: &im::HashMap<u32, (bool, u32)>,
) -> Option<im::HashMap<u32, (bool, u32)>> {
    if let Symbol::Flag(flag_diacritic_symbol) = symbol {
        match flag_diacritic_symbol.flag_type {
            FlagDiacriticType::U => {
                let value = flag_diacritic_symbol.value;

                // Is the current state somehow in conflict?
                // It can be, if we are negatively set to what we try to unify to or we are positively set to sth else

                if let Some((currently_set, current_value)) = flags.get(&flag_diacritic_symbol.key)
                {
                    if (*currently_set && current_value != &value)
                        || (!currently_set && current_value == &value)
                    {
                        return None;
                    }
                }

                // Otherwise, update flag set

                let mut clone: im::HashMap<u32, (bool, u32)> = flags.clone();
                clone.insert(flag_diacritic_symbol.key, (true, value));
                Some(clone)
            }
            FlagDiacriticType::R => {
                // Param count matters

                match flag_diacritic_symbol.value {
                    u32::MAX => {
                        if flags.contains_key(&flag_diacritic_symbol.key) {
                            Some(flags.clone())
                        } else {
                            None
                        }
                    }
                    value => {
                        if flags
                            .get(&flag_diacritic_symbol.key)
                            .map(|stored| _test_flag(stored, value))
                            .unwrap_or(false)
                        {
                            Some(flags.clone())
                        } else {
                            None
                        }
                    }
                }
            }
            FlagDiacriticType::D => {
                match (
                    flag_diacritic_symbol.value,
                    flags.get(&flag_diacritic_symbol.key),
                ) {
                    (u32::MAX, None) => Some(flags.clone()),
                    (u32::MAX, _) => None,
                    (_, None) => Some(flags.clone()),
                    (query, Some(stored)) => {
                        if _test_flag(stored, query) {
                            None
                        } else {
                            Some(flags.clone())
                        }
                    }
                }
            }
            FlagDiacriticType::C => {
                let mut flag_clone = flags.clone();
                flag_clone.remove(&flag_diacritic_symbol.key);
                Some(flag_clone)
            }
            FlagDiacriticType::P => {
                let value = flag_diacritic_symbol.value;
                let mut flag_clone = flags.clone();
                flag_clone.insert(flag_diacritic_symbol.key, (true, value));
                Some(flag_clone)
            }
            FlagDiacriticType::N => {
                let value = flag_diacritic_symbol.value;
                let mut flag_clone = flags.clone();
                flag_clone.insert(flag_diacritic_symbol.key, (false, value));
                Some(flag_clone)
            }
        }
    } else {
        Some(flags.clone())
    }
}

#[cfg_attr(feature = "python", pymethods)]
impl FST {
    #[cfg(feature = "python")]
    #[staticmethod]
    #[pyo3(signature = (final_states, rules, symbols, debug = false))]
    fn from_rules(
        final_states: IndexMap<u64, f64>,
        rules: IndexMap<u64, IndexMap<Symbol, Vec<(u64, Symbol, f64)>>>,
        symbols: HashSet<Symbol>,
        debug: Option<bool>,
    ) -> FST {
        FST::_from_rules(final_states, rules, symbols, debug)
    }

    #[cfg(feature = "python")]
    #[staticmethod]
    #[pyo3(signature = (att_file, debug = false))]
    fn from_att_file(py: Python<'_>, att_file: PyObject, debug: bool) -> KFSTResult<FST> {
        FST::_from_att_file(att_file.call_method0(py, "__str__")?.extract(py)?, debug)
    }

    #[cfg(feature = "python")]
    #[staticmethod]
    #[pyo3(signature = (att_code, debug = false))]
    fn from_att_code(att_code: String, debug: bool) -> KFSTResult<FST> {
        FST::_from_att_code(att_code, debug)
    }

    #[cfg(feature = "python")]
    pub fn to_att_file(&self, py: Python<'_>, att_file: PyObject) -> KFSTResult<()> {
        let path: String = att_file.call_method0(py, "__str__")?.extract(py)?;
        fs::write(Path::new(&path), self.to_att_code()).map_err(|err| {
            io_error::<()>(format!("Failed to write to file {}:\n{}", path, err)).unwrap_err()
        })
    }

    /// Save the current transducer to a file in the ATT format. See [FST::from_att_code] for more details on the ATT format.
    #[cfg(not(feature = "python"))]
    pub fn to_att_file(&self, att_file: String) -> KFSTResult<()> {
        fs::write(Path::new(&att_file), self.to_att_code()).map_err(|err| {
            io_error::<()>(format!("Failed to write to file {}:\n{}", att_file, err)).unwrap_err()
        })
    }

    /// Serialize the current transducer to a [String] in the ATT format. See [FST::from_att_code] for more details on the ATT format.
    pub fn to_att_code(&self) -> String {
        let mut rows: Vec<String> = vec![];
        for (state, weight) in self.final_states.iter() {
            match weight {
                0.0 => {
                    rows.push(format!("{}", state));
                }
                _ => {
                    rows.push(format!("{}\t{}", state, weight));
                }
            }
        }
        for (from_state, rules) in self.rules.iter() {
            for (top_symbol, transitions) in rules.iter() {
                for (to_state, bottom_symbol, weight) in transitions.iter() {
                    match weight {
                        0.0 => {
                            rows.push(format!(
                                "{}\t{}\t{}\t{}",
                                from_state,
                                to_state,
                                top_symbol.get_symbol(),
                                bottom_symbol.get_symbol()
                            ));
                        }
                        _ => {
                            rows.push(format!(
                                "{}\t{}\t{}\t{}\t{}",
                                from_state,
                                to_state,
                                top_symbol.get_symbol(),
                                bottom_symbol.get_symbol(),
                                weight
                            ));
                        }
                    }
                }
            }
        }
        rows.join("\n")
    }

    #[cfg(feature = "python")]
    #[staticmethod]
    #[pyo3(signature = (kfst_file, debug = false))]
    fn from_kfst_file(py: Python<'_>, kfst_file: PyObject, debug: bool) -> KFSTResult<FST> {
        FST::_from_kfst_file(kfst_file.call_method0(py, "__str__")?.extract(py)?, debug)
    }

    #[cfg(feature = "python")]
    #[staticmethod]
    #[pyo3(signature = (kfst_bytes, debug = false))]
    fn from_kfst_bytes(kfst_bytes: &[u8], debug: bool) -> KFSTResult<FST> {
        FST::__from_kfst_bytes(kfst_bytes, debug)
    }

    #[cfg(feature = "python")]
    pub fn to_kfst_file(&self, py: Python<'_>, kfst_file: PyObject) -> KFSTResult<()> {
        let bytes = self.to_kfst_bytes()?;
        let path: String = kfst_file.call_method0(py, "__str__")?.extract(py)?;
        fs::write(Path::new(&path), bytes).map_err(|err| {
            io_error::<()>(format!("Failed to write to file {}:\n{}", path, err)).unwrap_err()
        })
    }

    #[cfg(not(feature = "python"))]
    /// Save the current transducer to a file in the KFST format. See [FST::from_kfst_bytes] for more details on the KFST format.
    pub fn to_kfst_file(&self, kfst_file: String) -> KFSTResult<()> {
        let bytes = self.to_kfst_bytes()?;
        fs::write(Path::new(&kfst_file), bytes).map_err(|err| {
            io_error::<()>(format!("Failed to write to file {}:\n{}", kfst_file, err)).unwrap_err()
        })
    }

    /// Serialize the current transducer to a bytestring in the KFST format. See [FST::from_kfst_bytes] for more details on the KFST format.
    pub fn to_kfst_bytes(&self) -> KFSTResult<Vec<u8>> {
        match self._to_kfst_bytes() {
            Ok(x) => Ok(x),
            Err(x) => value_error(x),
        }
    }

    #[deprecated]
    /// Convert this FST into a somewhat human readable string representation. Exists for the Python API's sake.
    pub fn __repr__(&self) -> String {
        format!(
            "FST(final_states: {:?}, rules: {:?}, symbols: {:?}, debug: {:?})",
            self.final_states, self.rules, self.symbols, self.debug
        )
    }

    #[cfg(feature = "python")]
    #[pyo3(signature = (text, allow_unknown = true))]
    fn split_to_symbols(&self, text: &str, allow_unknown: bool) -> Option<Vec<Symbol>> {
        self._split_to_symbols(text, allow_unknown)
    }

    #[cfg(feature = "python")]
    #[pyo3(signature = (input_symbols, state = FSTState::_new(0), post_input_advance = false))]
    fn run_fst(
        &self,
        input_symbols: Vec<Symbol>,
        state: FSTState,
        post_input_advance: bool,
    ) -> Vec<(bool, bool, FSTState)> {
        self.__run_fst(input_symbols, state, post_input_advance)
    }

    #[cfg(feature = "python")]
    #[pyo3(signature = (input, state=FSTState::_new(0), allow_unknown=true))]
    fn lookup(
        &self,
        input: &str,
        state: FSTState,
        allow_unknown: bool,
    ) -> KFSTResult<Vec<(String, f64)>> {
        self._lookup(input, state, allow_unknown)
    }

    #[deprecated]
    /// Equal to:
    /// ```no_test
    /// self.rules[&state.state_num].keys().cloned().collect()
    /// ```
    /// Exists as its own function to make getting the input symbols of a state fast when calling from Python.
    /// (Otherwise the whole [FST::rules] mapping needs to be converted into Python's representation, which is significantly slower)
    ///
    /// ```
    /// use kfst_rs::{FST, Symbol, FSTState};
    /// use std::collections::HashSet;
    /// use indexmap::IndexMap;
    ///
    /// let fst = FST::from_att_code("0\t1\ta\tb\n".to_string(), false).unwrap();
    /// let mut expected = HashSet::new();
    /// expected.insert(Symbol::parse("a").unwrap().1);
    /// assert_eq!(fst.get_input_symbols(FSTState::new(0, 0.0, IndexMap::new(), IndexMap::new(), vec![])), expected);
    /// assert_eq!(fst.get_input_symbols(FSTState::new(1, 0.0, IndexMap::new(), IndexMap::new(), vec![])), HashSet::new());
    /// ```
    pub fn get_input_symbols(&self, state: FSTState) -> HashSet<Symbol> {
        self.rules
            .get(&state.state_num)
            .map(|x| x.keys().cloned().collect())
            .unwrap_or_else(|| HashSet::new())
    }
}

#[test]
fn test_kfst_voikko_kissa() {
    let fst = FST::_from_kfst_file("../pyvoikko/pyvoikko/voikko.kfst".to_string(), false).unwrap();
    assert_eq!(
        fst.lookup("kissa", FSTState::_new(0), false).unwrap(),
        vec![("[Ln][Xp]kissa[X]kiss[Sn][Ny]a".to_string(), 0.0)]
    );
    assert_eq!(
        fst.lookup("kissojemmekaan", FSTState::_new(0), false)
            .unwrap(),
        vec![(
            "[Ln][Xp]kissa[X]kiss[Sg][Nm]oje[O1m]mme[Fkaan]kaan".to_string(),
            0.0
        )]
    );
}

#[test]
fn test_that_weight_of_end_state_applies_correctly() {
    let code = "0\t1\ta\tb\n1\t1.0";
    let fst = FST::from_att_code(code.to_string(), false).unwrap();
    assert_eq!(
        fst.lookup("a", FSTState::_new(0), false).unwrap(),
        vec![("b".to_string(), 1.0)]
    );
}

#[test]
fn test_kfst_voikko_correct_final_states() {
    let fst: FST =
        FST::_from_kfst_file("../pyvoikko/pyvoikko/voikko.kfst".to_string(), false).unwrap();
    let map: IndexMap<_, _> = [(19, 0.0)].into_iter().collect();
    assert_eq!(fst.final_states, map);
}

#[test]
fn test_kfst_voikko_split() {
    let fst: FST =
        FST::_from_kfst_file("../pyvoikko/pyvoikko/voikko.kfst".to_string(), false).unwrap();
    assert_eq!(
        fst.split_to_symbols("lentokone", false).unwrap(),
        vec![
            Symbol::String(StringSymbol {
                string: intern("l".to_string()),
                unknown: false
            }),
            Symbol::String(StringSymbol {
                string: intern("e".to_string()),
                unknown: false
            }),
            Symbol::String(StringSymbol {
                string: intern("n".to_string()),
                unknown: false
            }),
            Symbol::String(StringSymbol {
                string: intern("t".to_string()),
                unknown: false
            }),
            Symbol::String(StringSymbol {
                string: intern("o".to_string()),
                unknown: false
            }),
            Symbol::String(StringSymbol {
                string: intern("k".to_string()),
                unknown: false
            }),
            Symbol::String(StringSymbol {
                string: intern("o".to_string()),
                unknown: false
            }),
            Symbol::String(StringSymbol {
                string: intern("n".to_string()),
                unknown: false
            }),
            Symbol::String(StringSymbol {
                string: intern("e".to_string()),
                unknown: false
            }),
        ]
    );

    assert_eq!(
        fst.split_to_symbols("lentää", false).unwrap(),
        vec![
            Symbol::String(StringSymbol {
                string: intern("l".to_string()),
                unknown: false
            }),
            Symbol::String(StringSymbol {
                string: intern("e".to_string()),
                unknown: false
            }),
            Symbol::String(StringSymbol {
                string: intern("n".to_string()),
                unknown: false
            }),
            Symbol::String(StringSymbol {
                string: intern("t".to_string()),
                unknown: false
            }),
            Symbol::String(StringSymbol {
                string: intern("ä".to_string()),
                unknown: false
            }),
            Symbol::String(StringSymbol {
                string: intern("ä".to_string()),
                unknown: false
            }),
        ]
    );
}

#[test]
fn test_kfst_voikko() {
    let fst = FST::_from_kfst_file("../pyvoikko/pyvoikko/voikko.kfst".to_string(), false).unwrap();
    assert_eq!(
        fst.lookup("lentokone", FSTState::_new(0), false).unwrap(),
        vec![(
            "[Lt][Xp]lentää[X]len[Ln][Xj]to[X]to[Sn][Ny][Bh][Bc][Ln][Xp]kone[X]kone[Sn][Ny]"
                .to_string(),
            0.0
        )]
    );
}

#[test]
fn test_kfst_voikko_lentää() {
    let fst = FST::_from_kfst_file("../pyvoikko/pyvoikko/voikko.kfst".to_string(), false).unwrap();
    assert_eq!(
        fst.lookup("lentää", FSTState::_new(0), false).unwrap(),
        vec![
            (
                "[Lt][Xp]lentää[X]len[Tt][Ap][P3][Ny][Ef]tää".to_string(),
                0.0
            ),
            ("[Lt][Xp]lentää[X]len[Tn1][Eb]tää".to_string(), 0.0)
        ]
    );
}

#[test]
fn test_kfst_voikko_lentää_correct_states() {
    let fst = FST::_from_kfst_file("../pyvoikko/pyvoikko/voikko.kfst".to_string(), false).unwrap();
    let input_symbols = fst.split_to_symbols("lentää", false).unwrap();

    // Correct number of states for different subsequence lengths per KFST

    let results = [
        vec![
            0, 1, 1810, 1946, 1961, 1962, 1963, 1964, 1965, 1966, 2665, 2969, 2970, 3104, 3295,
            3484, 3678, 3870, 4064, 4260, 4454, 4648, 4842, 5036, 5230, 5454, 5645, 5839, 6031,
            6225, 6419, 6613, 6807, 7001, 7195, 7389, 7579, 12479, 13348, 13444, 13541, 13636,
            13733, 13830, 13925, 14028, 14131, 14234, 14331, 14426, 14525, 14622, 14723, 14826,
            14929, 15024, 15127, 15230, 15333, 15433, 15526,
        ],
        vec![
            10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 1878, 2840, 17295, 25716, 31090, 40909, 85222,
            204950, 216255, 217894, 254890, 256725, 256726, 256727, 256728, 256729, 256730, 256731,
            256732, 256733, 256734, 256735, 256736, 280866, 281235, 281479, 281836, 281876, 281877,
            288536, 355529, 378467,
        ],
        vec![
            17459, 17898, 17899, 26065, 26066, 26067, 26068, 26069, 31245, 42140, 87151, 134039,
            134040, 205452, 219693, 219694, 259005, 259666, 259667, 259668, 259669, 259670, 259671,
            259672, 280894, 281857, 289402, 356836, 378621, 378750, 378773, 386786, 388199, 388200,
            388201, 388202, 388203,
        ],
        vec![
            17458, 17459, 17899, 19455, 26192, 26214, 26215, 26216, 26217, 42361, 87536, 118151,
            205474, 216303, 220614, 220615, 220616, 220617, 220618, 220619, 220620, 220621, 220629,
            228443, 228444, 228445, 259219, 259220, 259221, 259222, 259223, 259224, 259225, 356941,
            387264,
        ],
        vec![
            42362, 102258, 216304, 216309, 216312, 216317, 217230, 356942, 387265,
        ],
        vec![
            211149, 212998, 212999, 213000, 213001, 213002, 216305, 216310, 216313, 216318,
        ],
        vec![
            12, 12, 13, 13, 14, 14, 15, 15, 16, 16, 17, 17, 18, 18, 19, 19, 210815, 210816, 211139,
            211140, 214985, 216311, 216314, 216315, 216316,
        ],
    ];

    for i in 0..=input_symbols.len() {
        let subsequence = &input_symbols[..i];
        let mut states: Vec<_> = fst
            .run_fst(subsequence.to_vec(), FSTState::_new(0), false)
            .into_iter()
            .map(|(_, _, x)| x.state_num)
            .collect();
        states.sort();
        assert_eq!(states, results[i]);
    }
}

#[test]
fn test_minimal_r_diacritic() {
    let code = "0\t1\t@P.V_SALLITTU.T@\tasetus\n1\t2\t@R.V_SALLITTU.T@\ttarkistus\n2";
    let fst = FST::from_att_code(code.to_string(), false).unwrap();
    let mut result = vec![];
    fst._run_fst(&[], &FSTState::_new(0), false, &mut result);
    for x in result {
        println!("{:?}", x);
    }
    assert_eq!(
        fst.lookup("", FSTState::_new(0), false).unwrap(),
        vec![("asetustarkistus".to_string(), 0.0)]
    );
}

#[test]
fn test_kfst_voikko_lentää_result_count() {
    let fst = FST::_from_kfst_file("../pyvoikko/pyvoikko/voikko.kfst".to_string(), false).unwrap();
    let input_symbols = fst.split_to_symbols("lentää", false).unwrap();

    // Correct number of states for different subsequence lengths per KFST

    let results = [61, 42, 37, 35, 9, 10, 25];

    for i in 0..=input_symbols.len() {
        let subsequence = &input_symbols[..i];
        assert_eq!(
            fst.run_fst(subsequence.to_vec(), FSTState::_new(0), false)
                .len(),
            results[i]
        );
    }
}

#[test]
fn does_not_crash_on_unknown() {
    let fst = FST::from_att_code("0\t1\ta\tb\n1".to_string(), false).unwrap();
    assert_eq!(fst.lookup("c", FSTState::_new(0), true).unwrap(), vec![]);
    assert!(fst.lookup("c", FSTState::_new(0), false).is_err());
}

#[test]
fn test_kfst_voikko_paragraph() {
    let words = [
        "on",
        "maanantaiaamu",
        "heinäkuussa",
        "aurinko",
        "paiskaa",
        "niin",
        "lämpöisesti",
        "heikon",
        "tuulen",
        "avulla",
        "ja",
        "peipposet",
        "kajahuttelevat",
        "ensimmäisiä",
        "kovia",
        "säveleitään",
        "tuoksuavissa",
        "koivuissa",
        "kirkon",
        "itäisellä",
        "seinuksella",
        "on",
        "kivipenkki",
        "juuri",
        "nyt",
        "saapuu",
        "keski-ikäinen",
        "työmies",
        "ja",
        "istuutuu",
        "penkille",
        "hän",
        "näyttää",
        "väsyneeltä",
        "alakuloiselta",
        "haluttomalla",
        "aivan",
        "kuin",
        "olisi",
        "vastikään",
        "tullut",
        "perheellisestä",
        "riidasta",
        "tahi",
        "jättänyt",
        "eilisen",
        "sapatinpäivän",
        "pyhittämättä",
    ];
    let gold: [Vec<(&str, i32)>; 48] = [
        vec![("[Lt][Xp]olla[X]o[Tt][Ap][P3][Ny][Ef]n", 0)],
        vec![("[Ln][Xp]maanantai[X]maanantai[Sn][Ny][Bh][Bc][Ln][Xp]aamu[X]aamu[Sn][Ny]", 0)],
        vec![("[Ln][Xp]heinä[X]hein[Sn][Ny]ä[Bh][Bc][Ln][Xp]kuu[X]kuu[Sine][Ny]ssa", 0)],
        vec![("[Ln][Xp]aurinko[X]aurinko[Sn][Ny]", 0), ("[Lem][Xp]Aurinko[X]aurinko[Sn][Ny]", 0), ("[Lee][Xp]Auri[X]aur[Sg][Ny]in[Fko][Ef]ko", 0)],
        vec![("[Lt][Xp]paiskata[X]paiska[Tt][Ap][P3][Ny][Eb]a", 0)],
        vec![("[Ls][Xp]niin[X]niin", 0)],
        vec![("[Ln][Xp]lämpö[X]lämpö[Ll][Xj]inen[X]ise[Ssti]sti", 0)],
        vec![("[Ll][Xp]heikko[X]heiko[Sg][Ny]n", 0)],
        vec![("[Ln][Xp]tuuli[X]tuul[Sg][Ny]en", 0)],
        vec![("[Ln][Xp]avu[X]avu[Sade][Ny]lla", 0), ("[Ln][Xp]apu[X]avu[Sade][Ny]lla", 0)],
        vec![("[Lc][Xp]ja[X]ja", 0)],
        vec![("[Ln][Xp]peipponen[X]peippo[Sn][Nm]set", 0)],
        vec![],
        vec![("[Lu][Xp]ensimmäinen[X]ensimmäi[Sp][Nm]siä", 0)],
        vec![("[Lnl][Xp]kova[X]kov[Sp][Nm]ia", 0)],
        vec![],
        vec![],
        vec![("[Ln][Xp]koivu[X]koivu[Sine][Nm]issa", 0), ("[Les][Xp]Koivu[X]koivu[Sine][Nm]issa", 0)],
        vec![("[Ln][Ica][Xp]kirkko[X]kirko[Sg][Ny]n", 0)],
        vec![("[Ln][De][Xp]itä[X]itä[Ll][Xj]inen[X]ise[Sade][Ny]llä", 0)],
        vec![("[Ln][Xp]seinus[X]seinukse[Sade][Ny]lla", 0)],
        vec![("[Lt][Xp]olla[X]o[Tt][Ap][P3][Ny][Ef]n", 0)],
        vec![("[Ln][Ica][Xp]kivi[X]kiv[Sn][Ny]i[Bh][Bc][Ln][Xp]penkki[X]penkk[Sn][Ny]i", 0)],
        vec![("[Ln][Xp]juuri[X]juur[Sn][Ny]i", 0), ("[Ls][Xp]juuri[X]juuri", 0), ("[Lt][Xp]juuria[X]juuri[Tk][Ap][P2][Ny][Eb]", 0), ("[Lt][Xp]juuria[X]juur[Tt][Ai][P3][Ny][Ef]i", 0)],
        vec![("[Ls][Xp]nyt[X]nyt", 0)],
        vec![("[Lt][Xp]saapua[X]saapuu[Tt][Ap][P3][Ny][Ef]", 0)],
        vec![("[Lp]keski[De]-[Bh][Bc][Ln][Xp]ikä[X]ikä[Ll][Xj]inen[X]i[Sn][Ny]nen", 0)],
        vec![("[Ln][Xp]työ[X]työ[Sn][Ny][Bh][Bc][Ln][Xp]mies[X]mies[Sn][Ny]", 0)],
        vec![("[Lc][Xp]ja[X]ja", 0)],
        vec![("[Lt][Xp]istuutua[X]istuutuu[Tt][Ap][P3][Ny][Ef]", 0)],
        vec![("[Ln][Xp]penkki[X]penki[Sall][Ny]lle", 0)],
        vec![("[Lr][Xp]hän[X]hä[Sn][Ny]n", 0)],
        vec![("[Lt][Xp]näyttää[X]näyttä[Tn1][Eb]ä", 0), ("[Lt][Xp]näyttää[X]näytt[Tt][Ap][P3][Ny][Ef]ää", 0)],
        vec![("[Lt][Irm][Xp]väsyä[X]väsy[Ll][Ru]n[Xj]yt[X]ee[Sabl][Ny]ltä", 0)],
        vec![("[Ln][De][Xp]ala[X]al[Sn][Ny]a[Bh][Bc][Lnl][Xp]kulo[X]kulo[Ll][Xj]inen[X]ise[Sabl][Ny]lta", 0)],
        vec![("[Ln][Xp]halu[X]halu[Ll][Xj]ton[X]ttoma[Sade][Ny]lla", 0)],
        vec![("[Ls][Xp]aivan[X]aivan", 0)],
        vec![("[Lc][Xp]kuin[X]kuin", 0), ("[Ln][Xp]kuu[X]ku[Sin][Nm]in", 0)],
        vec![("[Lt][Xp]olla[X]ol[Te][Ap][P3][Ny][Eb]isi", 0)],
        vec![("[Ls][Xp]vast=ikään[X]vast[Bm]ikään", 0)],
        vec![("[Lt][Xp]tulla[X]tul[Ll][Ru]l[Xj]ut[X][Sn][Ny]ut", 0), ("[Lt][Xp]tulla[X]tul[Ll][Rt][Xj]tu[X]lu[Sn][Nm]t", 0)],
        vec![("[Ln][Xp]perhe[X]perhee[Ll]lli[Xj]nen[X]se[Sela][Ny]stä", 0)],
        vec![("[Ln][Xp]riita[X]riida[Sela][Ny]sta", 0)],
        vec![("[Lc][Xp]tahi[X]tahi", 0)],
        vec![("[Lt][Xp]jättää[X]jättä[Ll][Ru]n[Xj]yt[X][Sn][Ny]yt", 0)],
        vec![("[Lnl][Xp]eilinen[X]eili[Sg][Ny]sen", 0)],
        vec![("[Ln][Xp]sapatti[X]sapat[Sg][Ny]in[Bh][Bc][Ln][Xp]päivä[X]päiv[Sg][Ny]än", 0)],
        vec![("[Lt][Xp]pyhittää[X]pyhittä[Ln]m[Xj]ä[X][Rm]ä[Sab][Ny]ttä", 0), ("[Lt][Xp]pyhittää[X]pyhittä[Tn3][Ny][Sab]mättä", 0)],
    ];
    let fst = FST::_from_kfst_file("../pyvoikko/pyvoikko/voikko.kfst".to_string(), false).unwrap();
    for (idx, (word, gold)) in words.into_iter().zip(gold.into_iter()).enumerate() {
        let sys = fst.lookup(word, FSTState::_new(0), false).unwrap();
        println!("Word at: {}", idx);
        assert_eq!(
            sys,
            gold.iter()
                .map(|(s, w)| (s.to_string(), (*w).into()))
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_simple_unknown() {
    let code = "0\t1\t@_UNKNOWN_SYMBOL_@\ty\n1";
    let fst = FST::from_att_code(code.to_string(), false).unwrap();

    assert_eq!(
        fst.run_fst(
            vec![Symbol::String(StringSymbol::new("x".to_string(), false,))],
            FSTState::_new(0),
            false,
        ),
        vec![]
    );

    assert_eq!(
        fst.run_fst(
            vec![Symbol::String(StringSymbol::new("x".to_string(), true,))],
            FSTState::_new(0),
            false,
        ),
        vec![(
            true,
            false,
            FSTState {
                state_num: 1,
                path_weight: 0.0,
                input_flags: FlagMap(im::HashMap::new()),
                output_flags: FlagMap(im::HashMap::new()),
                output_symbols: vec![Symbol::String(StringSymbol::new("y".to_string(), false))]
            }
        )]
    );
}

#[test]
fn test_simple_identity() {
    let code = "0\t1\t@_IDENTITY_SYMBOL_@\t@_IDENTITY_SYMBOL_@\n1";
    let fst = FST::from_att_code(code.to_string(), false).unwrap();

    assert_eq!(
        fst.run_fst(
            vec![Symbol::String(StringSymbol::new("x".to_string(), false,))],
            FSTState::_new(0),
            false,
        ),
        vec![]
    );

    assert_eq!(
        fst.run_fst(
            vec![Symbol::String(StringSymbol::new("x".to_string(), true,))],
            FSTState::_new(0),
            false,
        ),
        vec![(
            true,
            false,
            FSTState {
                state_num: 1,
                path_weight: 0.0,
                input_flags: FlagMap(im::HashMap::new()),
                output_flags: FlagMap(im::HashMap::new()),
                output_symbols: vec![Symbol::String(StringSymbol::new("x".to_string(), true))]
            }
        )]
    );
}

#[test]
fn test_raw_symbols() {
    // Construct simple transducer

    let mut rules: IndexMap<u64, IndexMap<Symbol, Vec<(u64, Symbol, f64)>>> = IndexMap::new();
    let sym_a = Symbol::Raw(RawSymbol {
        value: [0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    });
    let sym_b = Symbol::Raw(RawSymbol {
        value: [0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    });
    let sym_c = Symbol::Raw(RawSymbol {
        value: [0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    });
    let special_epsilon = Symbol::Raw(RawSymbol {
        value: [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    });
    let sym_d = Symbol::Raw(RawSymbol {
        value: [0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    });
    let sym_d_unk = Symbol::Raw(RawSymbol {
        value: [2, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    });
    rules.insert(0, indexmap!(sym_a.clone() => vec![(1, sym_a.clone(), 0.0)]));
    rules.insert(1, indexmap!(sym_b.clone() => vec![(0, sym_b.clone(), 0.0)], Symbol::Special(SpecialSymbol::IDENTITY) => vec![(2, Symbol::Special(SpecialSymbol::IDENTITY), 0.0)]));
    rules.insert(
        2,
        indexmap!(special_epsilon.clone() => vec![(3, sym_c.clone(), 0.0)]),
    );
    let symbols = vec![sym_a.clone(), sym_b.clone(), sym_c.clone(), special_epsilon];
    let fst = FST {
        final_states: indexmap! {3 => 0.0},
        rules,
        symbols,
        debug: false,
    };

    // Accepting example that tests epsilon + unknown bits

    let result = fst.run_fst(
        vec![
            sym_a.clone(),
            sym_b.clone(),
            sym_a.clone(),
            sym_d_unk.clone(),
        ],
        FSTState::_new(0),
        false,
    );
    let filtered: Vec<_> = result.into_iter().filter(|x| x.0).collect();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].2.state_num, 3);
    assert_eq!(
        filtered[0].2.output_symbols,
        vec![
            sym_a.clone(),
            sym_b.clone(),
            sym_a.clone(),
            sym_d_unk.clone(),
            sym_c.clone()
        ]
    );

    // Rejecting example that further tests the unknown bit

    assert_eq!(
        fst.run_fst(
            vec![sym_a.clone(), sym_b.clone(), sym_a.clone(), sym_d.clone()],
            FSTState::_new(0),
            false
        )
        .into_iter()
        .filter(|x| x.0)
        .count(),
        0
    );
}

/// A Python module implemented in Rust.
#[cfg(feature = "python")]
#[pymodule]
fn kfst_rs(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    let symbols = PyModule::new(m.py(), "symbols")?;
    symbols.add_class::<StringSymbol>()?;
    symbols.add_class::<FlagDiacriticType>()?;
    symbols.add_class::<FlagDiacriticSymbol>()?;
    symbols.add_class::<SpecialSymbol>()?;
    symbols.add_class::<RawSymbol>()?;
    symbols.add_function(wrap_pyfunction!(from_symbol_string, m)?)?;

    py_run!(
        py,
        symbols,
        "import sys; sys.modules['kfst_rs.symbols'] = symbols"
    );

    m.add_submodule(&symbols)?;

    let transducer = PyModule::new(m.py(), "transducer")?;
    transducer.add_class::<FST>()?;
    transducer.add_class::<FSTState>()?;
    transducer.add(
        "TokenizationException",
        py.get_type::<TokenizationException>(),
    )?;

    py_run!(
        py,
        transducer,
        "import sys; sys.modules['kfst_rs.transducer'] = transducer"
    );

    m.add_submodule(&transducer)?;

    // Mimick reimports

    m.add(
        "TokenizationException",
        py.get_type::<TokenizationException>(),
    )?;
    m.add_class::<FST>()?;

    Ok(())
}
