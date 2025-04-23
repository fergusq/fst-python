use std::cmp::Ordering;
use std::fs::{self, File};
use std::hash::Hash;
use std::io::Read;
use std::path::Path;

use indexmap::{IndexMap, IndexSet};
use lzma_rs::lzma_compress;
use nom::branch::alt;
use nom::bytes::complete::{tag, take_until1};
use nom::multi::many_m_n;
use nom::Parser;
use pyo3::create_exception;
use pyo3::exceptions::{PyIOError, PyValueError};
use pyo3::{prelude::*, py_run, IntoPyObjectExt};

create_exception!(
    kfst_rs,
    TokenizationException,
    pyo3::exceptions::PyException
);

// symbols.py

#[derive(Debug)]
struct PyObjectSymbol {
    value: PyObject,
}

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

impl Eq for PyObjectSymbol {}

impl PartialOrd for PyObjectSymbol {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

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

impl Clone for PyObjectSymbol {
    fn clone(&self) -> Self {
        Python::with_gil(|py| Self {
            value: self.value.clone_ref(py),
        })
    }
}

impl FromPyObject<'_> for PyObjectSymbol {
    fn extract_bound(ob: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(PyObjectSymbol {
            value: ob.clone().unbind(),
        }) // The clone here is technical; no actual cloning of a value
    }
}

impl<'py> IntoPyObject<'py> for PyObjectSymbol {
    type Target = PyAny;

    type Output = Bound<'py, Self::Target>;

    type Error = pyo3::PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        self.value.into_bound_py_any(py)
    }
}

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

#[pyclass(str = "StringSymbol({string:?}, {unknown})", eq, ord, frozen, hash)]
#[derive(Clone, Hash, PartialEq, Eq)]
struct StringSymbol {
    #[pyo3(get)]
    string: String,
    #[pyo3(get)]
    unknown: bool,
}

impl StringSymbol {
    fn parse(symbol: &str) -> nom::IResult<&str, StringSymbol> {
        Ok((
            "",
            StringSymbol {
                string: symbol.to_string(),
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

#[pymethods]
impl StringSymbol {
    fn is_epsilon(&self) -> bool {
        false
    }

    fn is_unknown(&self) -> bool {
        self.unknown
    }

    fn get_symbol(&self) -> String {
        self.string.clone()
    }

    #[new]
    fn new(string: String, unknown: bool) -> Self {
        StringSymbol { string, unknown }
    }

    fn __repr__(&self) -> String {
        format!("StringSymbol({:?}, {})", self.string, self.unknown)
    }
}

#[pyclass(eq, ord, frozen)]
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Hash)]
enum FlagDiacriticType {
    U,
    R,
    D,
    C,
    P,
    N,
}

impl FlagDiacriticType {
    fn from_str(s: &str) -> Option<Self> {
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

#[pymethods]
impl FlagDiacriticType {
    fn __repr__(&self) -> String {
        format!("{:?}", &self)
    }
}

#[pyclass(
    str = "FlagDiacriticSymbol({flag_type:?}, {key:?}, {value:?})",
    eq,
    ord,
    frozen,
    hash
)]
#[derive(PartialEq, Eq, Clone, Hash)]
struct FlagDiacriticSymbol {
    flag_type: FlagDiacriticType,
    #[pyo3(get)]
    key: String,
    #[pyo3(get)]
    value: Option<String>,
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
    fn parse(symbol: &str) -> nom::IResult<&str, FlagDiacriticSymbol> {
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
            (named_piece_1[0].0, Some(named_piece_2.to_string()))
        } else {
            (named_piece_2, None)
        };

        Ok((
            input,
            FlagDiacriticSymbol {
                flag_type: diacritic_type,
                key: name.to_string(),
                value,
            },
        ))
    }
}

#[pymethods]
impl FlagDiacriticSymbol {
    fn is_epsilon(&self) -> bool {
        true
    }

    fn is_unknown(&self) -> bool {
        false
    }

    fn get_symbol(&self) -> String {
        match &self.value {
            None => format!("@{:?}.{}@", self.flag_type, self.key),
            Some(value) => format!("@{:?}.{}.{}@", self.flag_type, self.key, value),
        }
    }

    #[getter]
    fn flag_type(&self) -> String {
        format!("{:?}", self.flag_type)
    }

    #[new]
    #[pyo3(signature = (flag_type, key, value = None))]
    fn new(flag_type: String, key: String, value: Option<String>) -> PyResult<Self> {
        let flag_type = match FlagDiacriticType::from_str(&flag_type) {
            Some(x) => x,
            None => PyResult::Err(PyErr::new::<PyValueError, _>(format!(
                "String {:?} is not a valid FlagDiacriticType specifier",
                flag_type
            )))?,
        };
        Ok(FlagDiacriticSymbol {
            flag_type,
            key,
            value,
        })
    }

    fn __repr__(&self) -> String {
        match &self.value {
            None => format!("FlagDiacriticSymbol({:?}, {:?})", self.flag_type, self.key),
            Some(value) => format!(
                "FlagDiacriticSymbol({:?}, {:?}, {:?})",
                self.flag_type, self.key, value
            ),
        }
    }

    #[staticmethod]
    fn is_flag_diacritic(symbol: &str) -> bool {
        matches!(FlagDiacriticSymbol::parse(symbol), Ok(("", _)))
    }

    #[staticmethod]
    fn from_symbol_string(symbol: &str) -> PyResult<Self> {
        match FlagDiacriticSymbol::parse(symbol) {
            Ok(("", symbol)) => PyResult::Ok(symbol),
            Ok((rest, _)) => PyResult::Err(PyErr::new::<PyValueError, _>(format!("String {:?} contains a valid FlagDiacriticSymbol, but it has unparseable text at the end: {:?}", symbol, rest))),
            _ => PyResult::Err(PyErr::new::<PyValueError, _>(format!("Not a valid FlagDiacriticSymbol: {:?}", symbol)))
        }
    }
}

impl std::fmt::Debug for FlagDiacriticSymbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get_symbol())
    }
}

#[pyclass(eq, ord, frozen, hash)]
#[derive(PartialEq, Eq, Clone, Hash)]
enum SpecialSymbol {
    EPSILON,
    IDENTITY,
    UNKNOWN,
}

impl SpecialSymbol {
    fn parse(symbol: &str) -> nom::IResult<&str, SpecialSymbol> {
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

#[pymethods]
impl SpecialSymbol {
    fn is_epsilon(&self) -> bool {
        self == &SpecialSymbol::EPSILON
    }

    fn is_unknown(&self) -> bool {
        false
    }

    fn get_symbol(&self) -> String {
        match self {
            SpecialSymbol::EPSILON => "@_EPSILON_SYMBOL_@".to_string(),
            SpecialSymbol::IDENTITY => "@_IDENTITY_SYMBOL_@".to_string(),
            SpecialSymbol::UNKNOWN => "@_UNKNOWN_SYMBOL_@".to_string(),
        }
    }

    #[staticmethod]
    fn from_symbol_string(symbol: &str) -> PyResult<Self> {
        match SpecialSymbol::parse(symbol) {
            Ok(("", result)) => PyResult::Ok(result),
            _ => PyResult::Err(PyErr::new::<PyValueError, _>(format!(
                "Not a valid SpecialSymbol: {:?}",
                symbol
            ))),
        }
    }

    #[staticmethod]
    fn is_special_symbol(symbol: &str) -> bool {
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

#[pyfunction]
fn from_symbol_string(symbol: &str, py: Python) -> PyResult<Py<PyAny>> {
    Symbol::parse(symbol).unwrap().1.into_py_any(py)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Symbol {
    Special(SpecialSymbol),
    Flag(FlagDiacriticSymbol),
    String(StringSymbol),
    External(PyObjectSymbol),
}

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
            (Symbol::External(a), Symbol::External(b)) => a.cmp(b),

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
    fn is_epsilon(&self) -> bool {
        match self {
            Symbol::Special(special_symbol) => special_symbol.is_epsilon(),
            Symbol::Flag(flag_diacritic_symbol) => flag_diacritic_symbol.is_epsilon(),
            Symbol::String(string_symbol) => string_symbol.is_epsilon(),
            Symbol::External(py_object_symbol) => py_object_symbol.is_epsilon(),
        }
    }

    fn is_unknown(&self) -> bool {
        match self {
            Symbol::Special(special_symbol) => special_symbol.is_unknown(),
            Symbol::Flag(flag_diacritic_symbol) => flag_diacritic_symbol.is_unknown(),
            Symbol::String(string_symbol) => string_symbol.is_unknown(),
            Symbol::External(py_object_symbol) => py_object_symbol.is_unknown(),
        }
    }

    fn get_symbol(&self) -> String {
        match self {
            Symbol::Special(special_symbol) => special_symbol.get_symbol(),
            Symbol::Flag(flag_diacritic_symbol) => flag_diacritic_symbol.get_symbol(),
            Symbol::String(string_symbol) => string_symbol.get_symbol(),
            Symbol::External(py_object_symbol) => py_object_symbol.get_symbol(),
        }
    }

    fn parse(symbol: &str) -> nom::IResult<&str, Symbol> {
        // nb. never parses a token symbol from a string!

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

impl FromPyObject<'_> for Symbol {
    fn extract_bound(ob: &Bound<'_, PyAny>) -> PyResult<Self> {
        ob.extract()
            .map(Symbol::Special)
            .or_else(|_| ob.extract().map(Symbol::Flag))
            .or_else(|_| ob.extract().map(Symbol::String))
            .or_else(|_| ob.extract().map(Symbol::External))
    }
}

// transducer.py

#[pyclass(frozen)]
#[derive(Clone, Debug, PartialEq)]
struct FSTState {
    #[pyo3(get)]
    state_num: u64,
    #[pyo3(get)]
    path_weight: f64,
    #[pyo3(get)]
    input_flags: IndexMap<String, (bool, String)>,
    #[pyo3(get)]
    output_flags: IndexMap<String, (bool, String)>,
    #[pyo3(get)]
    output_symbols: Vec<Symbol>,
}

fn _test_flag(stored_val: &(bool, String), queried_val: &str) -> bool {
    stored_val.0 == (stored_val.1 == queried_val)
}

#[pymethods]
impl FSTState {
    #[new]
    fn new(state: u64) -> Self {
        FSTState {
            state_num: state,
            path_weight: 0.0,
            input_flags: IndexMap::new(),
            output_flags: IndexMap::new(),
            output_symbols: Vec::new(),
        }
    }

    fn __repr__(&self) -> String {
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

#[pyclass(frozen)]
struct FST {
    #[pyo3(get)]
    final_states: IndexMap<u64, f64>,
    #[pyo3(get)]
    rules: IndexMap<u64, IndexMap<Symbol, Vec<(u64, Symbol, f64)>>>,
    #[pyo3(get)]
    symbols: Vec<Symbol>, // Must be sorted in reverse order by length
    #[pyo3(get)]
    debug: bool,
}

impl FST {
    fn _run_fst(
        &self,
        input_symbols: &[Symbol],
        state: &FSTState,
        post_input_advance: bool,
    ) -> Vec<(bool, bool, FSTState)> {
        let transitions = self.rules.get(&state.state_num);
        let mut result = vec![];
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
                    result.extend(self._transition(
                        input_symbols,
                        state,
                        &transitions[transition_isymbol],
                        isymbol,
                        transition_isymbol,
                    ))
                }
            }
            if let Some(isymbol) = isymbol {
                if isymbol.is_unknown() {
                    if let Some(transition_list) =
                        transitions.get(&Symbol::Special(SpecialSymbol::UNKNOWN))
                    {
                        result.extend(self._transition(
                            input_symbols,
                            state,
                            &transition_list,
                            Some(isymbol),
                            &Symbol::Special(SpecialSymbol::UNKNOWN),
                        ));
                    }

                    if let Some(transition_list) =
                        transitions.get(&Symbol::Special(SpecialSymbol::IDENTITY))
                    {
                        result.extend(self._transition(
                            input_symbols,
                            state,
                            &transition_list,
                            Some(isymbol),
                            &Symbol::Special(SpecialSymbol::IDENTITY),
                        ));
                    }
                }
            }
        }
        result
    }

    fn _transition(
        &self,
        input_symbols: &[Symbol],
        state: &FSTState,
        transitions: &[(u64, Symbol, f64)],
        isymbol: Option<&Symbol>,
        transition_isymbol: &Symbol,
    ) -> Vec<(bool, bool, FSTState)> {
        let mut result = vec![];

        for (next_state, osymbol, weight) in transitions.iter() {
            let new_output_flags = _update_flags(osymbol, &state.output_flags);
            let new_input_flags = _update_flags(transition_isymbol, &state.input_flags);

            match (new_output_flags, new_input_flags) {
                (Some(new_output_flags), Some(new_input_flags)) => {
                    let mut new_output_symbols: Vec<Symbol> = state.output_symbols.clone();
                    match (isymbol, osymbol) {
                        (Some(isymbol), Symbol::Special(SpecialSymbol::IDENTITY)) => {
                            new_output_symbols.push(isymbol.clone())
                        }
                        (_, Symbol::Special(SpecialSymbol::EPSILON)) => (),
                        (_, Symbol::Flag(_)) => (),
                        _ => new_output_symbols.push(osymbol.clone()),
                    };
                    let new_state = FSTState {
                        state_num: *next_state,
                        path_weight: state.path_weight + *weight,
                        input_flags: new_input_flags,
                        output_flags: new_output_flags,
                        output_symbols: new_output_symbols,
                    };
                    if transition_isymbol.is_epsilon() {
                        result.extend(self._run_fst(
                            input_symbols,
                            &new_state,
                            input_symbols.is_empty(),
                        ));
                    } else {
                        let cloned_symbols = &input_symbols[1..];
                        result.extend(self._run_fst(cloned_symbols, &new_state, false));
                    }
                }
                _ => continue,
            }
        }
        result
    }

    fn from_att_rows(
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
}

fn _update_flags(
    symbol: &Symbol,
    flags: &IndexMap<String, (bool, String)>,
) -> Option<IndexMap<String, (bool, String)>> {
    if let Symbol::Flag(flag_diacritic_symbol) = symbol {
        match flag_diacritic_symbol.flag_type {
            FlagDiacriticType::U => {
                let value = flag_diacritic_symbol.value.clone().unwrap();

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

                let mut clone = flags.clone();
                clone.insert(flag_diacritic_symbol.key.clone(), (true, value));
                Some(clone)
            }
            FlagDiacriticType::R => {
                // Param count matters

                match &flag_diacritic_symbol.value {
                    Some(value) => {
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
                    None => {
                        if flags.contains_key(&flag_diacritic_symbol.key) {
                            Some(flags.clone())
                        } else {
                            None
                        }
                    }
                }
            }
            FlagDiacriticType::D => {
                match (
                    &flag_diacritic_symbol.value,
                    flags.get(&flag_diacritic_symbol.key),
                ) {
                    (None, None) => Some(flags.clone()),
                    (None, Some(_)) => None,
                    (Some(_), None) => Some(flags.clone()),
                    (Some(query), Some(stored)) => {
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
                flag_clone.swap_remove(&flag_diacritic_symbol.key);
                Some(flag_clone)
            }
            FlagDiacriticType::P => {
                let value = flag_diacritic_symbol.value.clone().unwrap();
                let mut flag_clone = flags.clone();
                flag_clone.insert(flag_diacritic_symbol.key.clone(), (true, value));
                Some(flag_clone)
            }
            FlagDiacriticType::N => {
                let value = flag_diacritic_symbol.value.clone().unwrap();
                let mut flag_clone = flags.clone();
                flag_clone.insert(flag_diacritic_symbol.key.clone(), (false, value));
                Some(flag_clone)
            }
        }
    } else {
        Some(flags.clone())
    }
}

#[pymethods]
impl FST {
    #[staticmethod]
    #[pyo3(signature = (final_states, rules, symbols, debug = false))]
    fn from_rules(
        final_states: IndexMap<u64, f64>,
        rules: IndexMap<u64, IndexMap<Symbol, Vec<(u64, Symbol, f64)>>>,
        symbols: Vec<Symbol>, // Must be sorted in reverse order by length
        debug: Option<bool>,
    ) -> FST {
        let mut new_symbols: Vec<Symbol> = symbols.to_vec();
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
    #[staticmethod]
    #[pyo3(signature = (att_file, debug = false))]
    fn from_att_file(att_file: String, debug: bool) -> PyResult<FST> {
        // Debug should default to false, pyo3 doesn't make that particularly easy
        match File::open(Path::new(&att_file)) {
            Ok(mut file) => {
                let mut att_code = String::new();
                file.read_to_string(&mut att_code).map_err(|err| {
                    PyErr::new::<PyIOError, _>(format!(
                        "Failed to read from file {}:\n{}",
                        att_file, err
                    ))
                })?;
                FST::from_att_code(att_code, debug)
            }
            Err(err) => PyResult::Err(PyErr::new::<PyIOError, _>(format!(
                "Failed to open file {}:\n{}",
                att_file, err
            ))),
        }
    }

    #[staticmethod]
    #[pyo3(signature = (att_code, debug = false))]
    fn from_att_code(att_code: String, debug: bool) -> PyResult<FST> {
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
                        return PyResult::Err(PyErr::new::<PyValueError, _>(format!(
                            "Failed to parse att code on line {}:\n{}",
                            lineno, line
                        )));
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
                        return PyResult::Err(PyErr::new::<PyValueError, _>(format!(
                            "Failed to parse att code on line {}:\n{}",
                            lineno, line
                        )));
                    }
                }
            }
        }
        PyResult::Ok(FST::from_att_rows(rows, debug))
    }

    fn to_att_file(&self, att_file: String) -> PyResult<()> {
        fs::write(Path::new(&att_file), self.to_att_code()).map_err(|err| {
            PyErr::new::<PyIOError, _>(format!("Failed to write to file {}:\n{}", att_file, err))
        })
    }

    fn to_att_code(&self) -> String {
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

    #[staticmethod]
    #[pyo3(signature = (kfst_file, debug = false))]
    fn from_kfst_file(kfst_file: String, debug: bool) -> PyResult<FST> {
        match File::open(Path::new(&kfst_file)) {
            Ok(mut file) => {
                let mut kfst_bytes: Vec<u8> = vec![];
                file.read_to_end(&mut kfst_bytes).map_err(|err| {
                    PyErr::new::<PyIOError, _>(format!(
                        "Failed to read from file {}:\n{}",
                        kfst_file, err
                    ))
                })?;
                FST::from_kfst_bytes(&kfst_bytes, debug)
            }
            Err(err) => PyResult::Err(PyErr::new::<PyIOError, _>(format!(
                "Failed to open file {}:\n{}",
                kfst_file, err
            ))),
        }
    }

    #[staticmethod]
    #[pyo3(signature = (kfst_bytes, debug = false))]
    fn from_kfst_bytes(kfst_bytes: &[u8], debug: bool) -> PyResult<FST> {
        match FST::_from_kfst_bytes(kfst_bytes) {
            Ok(x) => Ok(x),
            Err(x) => PyResult::Err(PyErr::new::<PyValueError, _>(x)),
        }
    }

    fn to_kfst_file(&self, kfst_file: String) -> PyResult<()> {
        let bytes = self.to_kfst_bytes()?;
        fs::write(Path::new(&kfst_file), bytes).map_err(|err| {
            PyErr::new::<PyIOError, _>(format!("Failed to write to file {}:\n{}", kfst_file, err))
        })
    }

    fn to_kfst_bytes(&self) -> PyResult<Vec<u8>> {
        match self._to_kfst_bytes() {
            Ok(x) => Ok(x),
            Err(x) => PyResult::Err(PyErr::new::<PyValueError, _>(x)),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "FST(final_states: {:?}, rules: {:?}, symbols: {:?}, debug: {:?})",
            self.final_states, self.rules, self.symbols, self.debug
        )
    }

    #[pyo3(signature = (text, allow_unknown = true))]
    fn split_to_symbols(&self, text: &str, allow_unknown: bool) -> Option<Vec<Symbol>> {
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
                    string: pos.next().unwrap().to_string(),
                    unknown: true,
                }));
            } else {
                return None;
            }
        }
        Some(result)
    }

    #[pyo3(signature = (input_symbols, state = FSTState::new(0), post_input_advance = false))]
    fn run_fst(
        &self,
        input_symbols: Vec<Symbol>,
        state: FSTState,
        post_input_advance: bool,
    ) -> Vec<(bool, bool, FSTState)> {
        self._run_fst(input_symbols.as_slice(), &state, post_input_advance)
    }

    #[pyo3(signature = (input, state=FSTState::new(0), allow_unknown=false))]
    fn lookup(
        &self,
        input: &str,
        state: FSTState,
        allow_unknown: bool,
    ) -> PyResult<Vec<(String, f64)>> {
        let input_symbols = self.split_to_symbols(input, allow_unknown);
        match input_symbols {
            None => PyResult::Err(PyErr::new::<TokenizationException, _>(format!(
                "Input cannot be split into symbols: {}",
                input
            ))),
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
}

#[test]
fn test_kfst_voikko_kissa() {
    let fst = FST::from_kfst_file("voikko.kfst".to_string(), false).unwrap();
    assert_eq!(
        fst.lookup("kissa", FSTState::new(0), false).unwrap(),
        vec![("[Ln][Xp]kissa[X]kiss[Sn][Ny]a".to_string(), 0.0)]
    );
    assert_eq!(
        fst.lookup("kissojemmekaan", FSTState::new(0), false)
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
        fst.lookup("a", FSTState::new(0), false).unwrap(),
        vec![("b".to_string(), 1.0)]
    );
}

#[test]
fn test_kfst_voikko_correct_final_states() {
    let fst: FST = FST::from_kfst_file("voikko.kfst".to_string(), false).unwrap();
    let map: IndexMap<_, _> = [(19, 0.0)].into_iter().collect();
    assert_eq!(fst.final_states, map);
}

#[test]
fn test_kfst_voikko_split() {
    let fst: FST = FST::from_kfst_file("voikko.kfst".to_string(), false).unwrap();
    assert_eq!(
        fst.split_to_symbols("lentokone", false).unwrap(),
        vec![
            Symbol::String(StringSymbol {
                string: "l".to_string(),
                unknown: false
            }),
            Symbol::String(StringSymbol {
                string: "e".to_string(),
                unknown: false
            }),
            Symbol::String(StringSymbol {
                string: "n".to_string(),
                unknown: false
            }),
            Symbol::String(StringSymbol {
                string: "t".to_string(),
                unknown: false
            }),
            Symbol::String(StringSymbol {
                string: "o".to_string(),
                unknown: false
            }),
            Symbol::String(StringSymbol {
                string: "k".to_string(),
                unknown: false
            }),
            Symbol::String(StringSymbol {
                string: "o".to_string(),
                unknown: false
            }),
            Symbol::String(StringSymbol {
                string: "n".to_string(),
                unknown: false
            }),
            Symbol::String(StringSymbol {
                string: "e".to_string(),
                unknown: false
            }),
        ]
    );

    assert_eq!(
        fst.split_to_symbols("lentää", false).unwrap(),
        vec![
            Symbol::String(StringSymbol {
                string: "l".to_string(),
                unknown: false
            }),
            Symbol::String(StringSymbol {
                string: "e".to_string(),
                unknown: false
            }),
            Symbol::String(StringSymbol {
                string: "n".to_string(),
                unknown: false
            }),
            Symbol::String(StringSymbol {
                string: "t".to_string(),
                unknown: false
            }),
            Symbol::String(StringSymbol {
                string: "ä".to_string(),
                unknown: false
            }),
            Symbol::String(StringSymbol {
                string: "ä".to_string(),
                unknown: false
            }),
        ]
    );
}

#[test]
fn test_kfst_voikko() {
    let fst = FST::from_kfst_file("voikko.kfst".to_string(), false).unwrap();
    assert_eq!(
        fst.lookup("lentokone", FSTState::new(0), false).unwrap(),
        vec![(
            "[Lt][Xp]lentää[X]len[Ln][Xj]to[X]to[Sn][Ny][Bh][Bc][Ln][Xp]kone[X]kone[Sn][Ny]"
                .to_string(),
            0.0
        )]
    );
}

#[test]
fn test_kfst_voikko_lentää() {
    let fst = FST::from_kfst_file("voikko.kfst".to_string(), false).unwrap();
    assert_eq!(
        fst.lookup("lentää", FSTState::new(0), false).unwrap(),
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
    let fst = FST::from_kfst_file("voikko.kfst".to_string(), false).unwrap();
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
            .run_fst(subsequence.to_vec(), FSTState::new(0), false)
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
    for x in fst._run_fst(&[], &FSTState::new(0), false) {
        println!("{:?}", x);
    }
    assert_eq!(
        fst.lookup("", FSTState::new(0), false).unwrap(),
        vec![("asetustarkistus".to_string(), 0.0)]
    );
}

#[test]
fn test_kfst_voikko_lentää_result_count() {
    let fst = FST::from_kfst_file("voikko.kfst".to_string(), false).unwrap();
    let input_symbols = fst.split_to_symbols("lentää", false).unwrap();

    // Correct number of states for different subsequence lengths per KFST

    let results = [61, 42, 37, 35, 9, 10, 25];

    for i in 0..=input_symbols.len() {
        let subsequence = &input_symbols[..i];
        assert_eq!(
            fst.run_fst(subsequence.to_vec(), FSTState::new(0), false)
                .len(),
            results[i]
        );
    }
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
    let fst = FST::from_kfst_file("voikko.kfst".to_string(), false).unwrap();
    for (word, gold) in words.into_iter().zip(gold.into_iter()) {
        let sys = fst.lookup(word, FSTState::new(0), false).unwrap();
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
            FSTState::new(0),
            false,
        ),
        vec![]
    );

    assert_eq!(
        fst.run_fst(
            vec![Symbol::String(StringSymbol::new("x".to_string(), true,))],
            FSTState::new(0),
            false,
        ),
        vec![(
            true,
            false,
            FSTState {
                state_num: 1,
                path_weight: 0.0,
                input_flags: IndexMap::new(),
                output_flags: IndexMap::new(),
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
            FSTState::new(0),
            false,
        ),
        vec![]
    );

    assert_eq!(
        fst.run_fst(
            vec![Symbol::String(StringSymbol::new("x".to_string(), true,))],
            FSTState::new(0),
            false,
        ),
        vec![(
            true,
            false,
            FSTState {
                state_num: 1,
                path_weight: 0.0,
                input_flags: IndexMap::new(),
                output_flags: IndexMap::new(),
                output_symbols: vec![Symbol::String(StringSymbol::new("x".to_string(), true))]
            }
        )]
    );
}

/// A Python module implemented in Rust.
#[pymodule]
fn kfst_rs(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    let symbols = PyModule::new(m.py(), "symbols")?;
    symbols.add_class::<StringSymbol>()?;
    symbols.add_class::<FlagDiacriticType>()?;
    symbols.add_class::<FlagDiacriticSymbol>()?;
    symbols.add_class::<SpecialSymbol>()?;
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
