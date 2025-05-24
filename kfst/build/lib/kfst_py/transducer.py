# This file is part of KFST.
#
# (c) 2023 Iikka Hauhio <iikka.hauhio@helsinki.fi>
#
# KFST is free software: you can redistribute it and/or modify it under the
# terms of the GNU Lesser General Public License as published by the Free
# Software Foundation, either version 3 of the License, or (at your option) any
# later version.
#
# KFST is distributed in the hope that it will be useful, but WITHOUT ANY
# WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
# FOR A PARTICULAR PURPOSE. See the GNU Lesser General Public License for more
# details.
#
# You should have received a copy of the GNU Lesser General Public License
# along with KFST. If not, see <https://www.gnu.org/licenses/>. 

from __future__ import annotations

from pathlib import Path
from typing import Generator, Mapping, NamedTuple

from immutables import Map

from .symbols import Symbol, StringSymbol, FlagDiacriticSymbol, SpecialSymbol

class TokenizationException(Exception):
    """Raised when failing to convert input string to symbols in KFST."""
    pass


class FSTState(NamedTuple):
    state_num: int
    path_weight: float = 0
    input_flags: Map[str, tuple[bool, str]] = Map()
    output_flags: Map[str, tuple[bool, str]] = Map()
    output_symbols: tuple[Symbol, ...] = tuple()

    def __repr__(self):
        return f"FSTState({self.state_num}, {self.path_weight}, {self.input_flags}, {self.output_flags}, {self.output_symbols})"


class FST(NamedTuple):
    """
    Represents a finite state transducer
    """
    final_states: Mapping[int, float]
    rules: Mapping[int, Mapping[Symbol, list[tuple[int, Symbol, float]]]]
    symbols: list[Symbol]  # Must be sorted in reverse order by length
    debug: bool = False

    @staticmethod
    def from_rules(
        final_states: Mapping[int, float],
        rules: Mapping[int, Mapping[Symbol, list[tuple[int, Symbol, float]]]],
        symbols: set[Symbol],
        debug: bool = False,
    ):
        """
        Creates an FST from a dictionary of final states, a dictionary of rules and a set of symbols.
        """
        return FST(final_states, rules, sorted(symbols, key=lambda s: -len(s.get_symbol())), debug=debug)

    @staticmethod
    def from_att_file(att_file: str | Path, debug: bool = False) -> "FST":
        """
        Parses a transducer in A&AT format and returns an FST object.
        The input must be the path to the .att file.

        If you already have the content of the .att file, use FST.from_att_code() instead.

        If you use HFST, .att files can be created using the command:
        ```sh
        hfst-fst2txt -f att file.hfst > file.att
        ```
        """
        if not isinstance(att_file, Path):
            att_file = Path(att_file)

        att_code = att_file.read_text()
        return FST.from_att_code(att_code, debug=debug)
        
    @staticmethod
    def from_att_code(att_code: str, debug: bool = False) -> "FST":
        """
        Parses a transducer in A&AT format and returns an FST object.
        
        Note that the input must be the content of the .att file, not the path to the file.
        For that, use FST.from_att_file() instead.
        """

        return decode_att(att_code)._replace(debug=debug)

    def to_att_file(self, att_file: str | Path) -> None:
        """
        Encodes the FST in the AT&T tabular format and writes it the given path.
        """
        if not isinstance(att_file, Path):
            att_file = Path(att_file)

        att_file.write_text(self.to_att_code())
    
    def to_att_code(self) -> str:
        """
        Encodes the FST in the AT&T tabular format.
        """
        return encode_att(self)

    @staticmethod
    def from_kfst_file(kfst_file: str | Path, debug: bool = False) -> "FST":
        """
        Parses a transducer in the KFST binary format and returns an FST object.
        The input must be the path to the .kfst file.

        If you already have the content of the .kfst file, use FST.from_kfst_bytes() instead.

        If you use HFST, .kfst files can be created using the command:
        ```sh
        python -m kfst.convert file.hfst file.kfst
        ```
        """
        if not isinstance(kfst_file, Path):
            kfst_file = Path(kfst_file)

        kfst_bytes = kfst_file.read_bytes()
        return FST.from_kfst_bytes(kfst_bytes, debug=debug)

    @staticmethod
    def from_kfst_bytes(kfst_bytes: bytes, debug: bool = False) -> "FST":
        """
        Parses a transducer in the KFST binary format and returns an FST object.

        Note that the input must be the content of the .kfst file, not the path to the file.
        For that, use FST.from_kfst_file() instead.
        """

        return decode_kfst(kfst_bytes)._replace(debug=debug)

    def to_kfst_file(self, path: str | Path):
        """
        Encodes the FST in the KFST binary format and writes it to the given path.
        """
        if not isinstance(path, Path):
            path = Path(path)

        path.write_bytes(self.to_kfst_bytes())
        
    def to_kfst_bytes(self) -> bytes:
        """
        Encodes the FST in the KFST binary format.
        """

        return encode_kfst(self)

    def split_to_symbols(self, text: str, allow_unknown=True) -> list[Symbol] | None:
        """
        Splits a given string into a list of symbols.
        For each position in the string, greedily selects the longest symbol that matches.

        Returns None if the string cannot be split into symbols.
        """
        
        # Symbols are assumed to be sorted in a reverse order by length
        # See FST.from_rules() implementation

        ans: list[Symbol] = []
        while text:
            for s in self.symbols:
                if isinstance(s, StringSymbol) and text.startswith(s.string):
                    ans.append(s)
                    text = text[len(s.string):]
                    break

            else:
                if allow_unknown:
                    ans.append(StringSymbol(text[0], unknown=True))
                    text = text[1:]
                
                else:
                    return None
        
        return ans

    def run_fst(self, input_symbols: list[Symbol], state=FSTState(0), post_input_advance=False) -> Generator[tuple[bool, bool, FSTState], None, None]:
        """
        Runs the FST on the given input symbols, starting from the given state (by default 0).
        Yields an (bool, FSTState) tuple for each path. If the path ended in a final state, the bool will be True, otherwise False.
        """
        transitions = self.rules.get(state.state_num, {})
        if not input_symbols:
            yield state.state_num in self.final_states, post_input_advance, state
            isymbol = None

        else:
            isymbol = input_symbols[0]

        for transition_isymbol in transitions:
            if transition_isymbol.is_epsilon() or isymbol is not None and transition_isymbol == isymbol:
                yield from self._transition(input_symbols, state, transitions[transition_isymbol], isymbol, transition_isymbol)
        
        if isymbol is not None and isymbol.is_unknown():
            if SpecialSymbol.UNKNOWN in transitions:
                yield from self._transition(input_symbols, state, transitions[SpecialSymbol.UNKNOWN], isymbol, SpecialSymbol.UNKNOWN)
            
            if SpecialSymbol.IDENTITY in transitions:
                yield from self._transition(input_symbols, state, transitions[SpecialSymbol.IDENTITY], isymbol, SpecialSymbol.IDENTITY)
    
    def _transition(self, input_symbols: list[Symbol], state: FSTState, transitions: list[tuple[int, Symbol, float]], isymbol: Symbol | None, transition_isymbol: Symbol) -> Generator[tuple[bool, bool, FSTState], None, None]:
        for next_state, osymbol, weight in transitions:
            if self.debug:
                print(state.state_num, "->", next_state, transition_isymbol, osymbol, input_symbols, state)
            
            new_output_flags = self._update_flags(osymbol, state.output_flags)
            if new_output_flags is None:
                continue  # flag unification failed

            new_input_flags = self._update_flags(transition_isymbol, state.input_flags)
            if new_input_flags is None:
                continue  # flag unification failed

            o = (isymbol,) if isymbol is not None and osymbol == SpecialSymbol.IDENTITY else (osymbol,) if not osymbol.is_epsilon() else ()

            new_state = FSTState(
                state_num=next_state,
                path_weight=state.path_weight + weight,
                input_flags=new_input_flags,
                output_flags=new_output_flags,
                output_symbols=state.output_symbols + o
            )

            if transition_isymbol.is_epsilon():
                yield from self.run_fst(input_symbols, state=new_state, post_input_advance=len(input_symbols) == 0)
            
            else:
                yield from self.run_fst(input_symbols[1:], state=new_state)
    
    def lookup(self, input: str, state=FSTState(0), allow_unknown=True) -> Generator[tuple[str, float], None, None]:
        """
        Runs the FST on the given input symbols, starting from the given state (by default 0).
        Yields a tuple of (output_symbols, path_weight) for each successful path.
        
        Otherwise same as run_fst, but operates on strings instead of symbol lists, filters out duplicate outputs and sorts the results by weight.

        Raises a TokenizationException if the input string can not be converted into the symbols of the KFST transducer.
        Note that if allow_unknown is True, all strings will be tokenized successfully.
        """

        input_symbols = self.split_to_symbols(input, allow_unknown=allow_unknown)
        if input_symbols is None:
            raise TokenizationException("Input cannot be split into symbols")

        results = self.run_fst(input_symbols, state=state)
        results = sorted(results, key=lambda x: x[2].path_weight)
        already_seen = set()
        for finished, _, state in results:
            if not finished:
                continue

            w = state.path_weight
            os = state.output_symbols
            o = "".join(s.get_symbol() for s in os)
            if o not in already_seen:
                yield o, w
                already_seen.add(o)
    
    def _update_flags(self, symbol: Symbol, flags: Map[str, tuple[bool, str]]) -> Map[str, tuple[bool, str]] | None:
        if isinstance(symbol, FlagDiacriticSymbol):
            flag = symbol.flag_type
            key = symbol.key
            value = symbol.value
            # unification flag
            if flag == "U":
                assert value is not None
                if key not in flags or (flags[key][1] == value if flags[key][0] else flags[key][1] != value):
                    return flags.set(key, (True, value))
                
                else:
                    return None
            
            # require flag (2 params)
            elif flag == "R" and value is not None:
                if key in flags and self._test_flag(flags[key], value):
                    return flags

                else:
                    return None
            
            # require flag (1 param)
            elif flag == "R" and value is None:
                if key in flags:
                    return flags

                else:
                    return None
            
            # disallow flag (2 params)
            elif flag == "D" and value is not None:
                if key in flags and self._test_flag(flags[key], value):
                    return None

                else:
                    return flags
            
            # disallow flag (1 param)
            elif flag == "D" and value is None:
                if key in flags:
                    return None

                else:
                    return flags
            
            # clear flag
            elif flag == "C":
                if key in flags:
                    return flags.delete(key)

                else:
                    return flags
            
            # positive (re)setting flag (2 params)
            elif flag == "P":
                assert value is not None
                return flags.set(key, (True, value))
            
            # negative (re)setting flag (2 params)
            elif flag == "N":
                assert value is not None
                return flags.set(key, (False, value))
        
        return flags

    def _test_flag(self, stored_val: tuple[bool, str], queried_val: str):
        if stored_val[0] and stored_val[1] == queried_val:
            return True
        
        elif not stored_val[0] and stored_val[1] != queried_val:
            return True
        
        return False

    def get_input_symbols(self, state: FSTState) -> set[Symbol]:
        """
        Get input symbols (disregarding flags) that one could continue from a state with.
        In other words, get the deduplicated input symbols on all outgoing arcs from a state.

        The body of this function is fairly trivial, as it boils down to indexing rules with state.state_num.
        In practice this function exists for compatibility with kfst_rs: calling fst.rules on a fst defined in Rust
        means converting the whole rule mapping from Rust's representation to Python's and cloning all the symbols within.
        Indexing would then be done on the python side.  It is possible and correct, but becomes a bottleneck if inside a tight loop.
        The Rust sibling of this function then allows only returning the relevant symbols to Python instead of the whole ruleset.
        """
        return set(self.rules[state.state_num].keys())

from .format.att import decode_att, encode_att
from .format.kfst import decode_kfst, encode_kfst