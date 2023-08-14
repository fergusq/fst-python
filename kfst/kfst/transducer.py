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

import argparse
from pathlib import Path
from typing import Generator, Mapping, NamedTuple

from frozendict import frozendict

class TokenizationException(Exception):
    """Raised when failing to convert input string to symbols in KFST."""
    pass

class FSTState(NamedTuple):
    state_num: int
    path_weight: float = 0
    input_flags: frozendict[str, str] = frozendict()
    output_flags: frozendict[str, str] = frozendict()
    output_symbols: tuple[str, ...] = tuple()

    def __repr__(self):
        return f"FSTState({self.state_num}, {self.path_weight}, {self.input_flags}, {self.output_flags}, {self.output_symbols})"


class FST(NamedTuple):
    """
    Represents a finite state transducer
    """
    final_states: Mapping[int, float]
    rules: Mapping[int, Mapping[str, list[tuple[int, str, float]]]]
    symbols: list[str] # Must be sorted in reverse order by length
    debug: bool = False

    @staticmethod
    def from_rules(
        final_states: Mapping[int, float],
        rules: Mapping[int, Mapping[str, list[tuple[int, str, float]]]],
        symbols: set[str],
        debug: bool = False,
    ):
        """
        Creates an FST from a dictionary of final states, a dictionary of rules and a set of symbols.
        """
        return FST(final_states, rules, sorted(symbols, key=lambda s: -len(s)), debug=debug)

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

    def split_to_symbols(self, text: str) -> list[str] | None:
        """
        Splits a given string into a list of symbols.
        For each position in the string, greedily selects the longest symbol that matches.

        Returns None if the string cannot be split into symbols.
        """
        
        # Symbols are assumed to be sorted in a reverse order by length
        # See FST.from_rules() implementation

        ans = []
        while text:
            for s in self.symbols:
                if text.startswith(s):
                    ans.append(s)
                    text = text[len(s):]
                    break

            else:
                return None
        
        return ans

    def run_fst(self, input_symbols: list[str], state=FSTState(0), post_input_advance=False) -> Generator[tuple[bool, bool, FSTState], None, None]:
        """
        Runs the FST on the given input symbols, starting from the given state (by default 0).
        Yields an (bool, FSTState) tuple for each path. If the path ended in a final state, the bool will be True, otherwise False.
        """
        transitions = self.rules.get(state.state_num, {})
        if not input_symbols:
            yield state.state_num in self.final_states, post_input_advance, state

        else:
            isymbol = input_symbols[0]
            for next_state, osymbol, weight in transitions.get(isymbol, []) + transitions.get("@_UNKNOWN_SYMBOL_@", []) + transitions.get("@_IDENTITY_SYMBOL_@", []):
                if self.debug:
                    print(state.state_num, "->", next_state, isymbol, osymbol, input_symbols, state)
                
                new_input_flags = self._update_flags(isymbol, state.input_flags)
                new_output_flags = self._update_flags(osymbol, state.output_flags)
                o = (isymbol,) if osymbol == "@_IDENTITY_SYMBOL_@" else (osymbol,) if not self._is_epsilon(osymbol) else ()

                if new_input_flags is None or new_output_flags is None:
                    continue # flag unification failed

                new_state = state._replace(
                    state_num=next_state,
                    path_weight=state.path_weight + weight,
                    input_flags=new_input_flags,
                    output_flags=new_output_flags,
                    output_symbols=state.output_symbols + o
                )

                yield from self.run_fst(input_symbols[1:], state=new_state)

        epsilon_transitions = [key for key in transitions if self._is_epsilon(key)]

        for eps in epsilon_transitions:
            for next_state, osymbol, weight in transitions[eps]:
                if self.debug:
                    print(state.state_num, "->", next_state, eps, osymbol, input_symbols, state)
                
                new_output_flags = self._update_flags(osymbol, state.output_flags)
                o = (osymbol,) if not self._is_epsilon(osymbol) else ()

                if new_output_flags is None:
                    continue # flag unification failed

                new_state = state._replace(
                    state_num=next_state,
                    path_weight=state.path_weight + weight,
                    output_flags=new_output_flags,
                    output_symbols=state.output_symbols + o
                )
                yield from self.run_fst(input_symbols, state=new_state, post_input_advance=len(input_symbols) == 0)
    
    def lookup(self, input: str, state=FSTState(0)):
        """
        Runs the FST on the given input symbols, starting from the given state (by default 0).
        Yields a tuple of (output_symbols, path_weight) for each successful path.
        
        Otherwise same as run_fst, but operates on strings instead of symbol lists, filters out duplicate outputs and sorts the results by weight.

        Raises a TokenizationException if the input string can not be converted into the symbols of the KFST transducer.
        """

        input_symbols = self.split_to_symbols(input)
        if input_symbols is None:
            raise TokenizationException("Input cannot be split into symbols")

        results = self.run_fst(input_symbols, state=state)
        results = sorted(results, key=lambda x: x[1])
        already_seen = set()
        for finished, _, state in results:
            if not finished:
                continue

            w = state.path_weight
            os = state.output_symbols
            o = "".join(os)
            if o not in already_seen:
                yield o, w
                already_seen.add(o)
    
    def _update_flags(self, symbol: str, flags: frozendict[str, str]) -> frozendict[str, str] | None:
        if self._is_flag(symbol):
            # Parse flag
            # Two params: @<flag type>.<flag key>.<flag value>@
            # One param: @<flag type>.<flag key>@
            flag = symbol[1]
            di = symbol.rindex(".")
            key = symbol[3:di] if di > 3 else symbol[3:-1]
            value = symbol[di+1:-1] if di > 3 else None
            # unification flag
            if flag == "U":
                assert value is not None
                if key in flags and flags[key] != value and (not flags[key].startswith("@") or flags[key] == "@" + value):
                    return None # flag mismatch

                else:
                    return flags.set(key, value)
            
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
                return flags.set(key, value)
            
            # negative (re)setting flag (2 params)
            elif flag == "N":
                assert value is not None
                return flags.set(key, "@" + value)
        
        return flags

    def _test_flag(self, stored_val: str, queried_val: str):
        if stored_val == queried_val:
            return True
        
        elif stored_val.startswith("@") and stored_val != queried_val[1:]:
            return True
        
        return False

    def _is_epsilon(self, symbol: str) -> bool:
        # flag diacritics are treated like epsilon symbols
        return symbol == "@0@" or symbol == "@_EPSILON_SYMBOL_@" or self._is_flag(symbol)
    
    def _is_flag(self, symbol: str) -> bool:
        return len(symbol) > 4 and symbol[0] == "@" and symbol[-1] == "@" and symbol[1] in "PNDRCU" and symbol[2] == "."

from .format.att import decode_att, encode_att
from .format.kfst import decode_kfst, encode_kfst


def main():
    parser = argparse.ArgumentParser(description="Finite State Transducer interpreter written in Python")
    parser.add_argument("fst_file", type=Path, help="FST in AT&T or KFST format")
    parser.add_argument("-d", action="store_true", help="enable debug mode")
    parser.add_argument("-s", action="store_true", help="print symbols in transducer and exit")
    parser.add_argument("-f", choices=["att", "kfst", "auto"], default="auto", help="force input format")
    args = parser.parse_args()

    if args.fst_file.suffix == ".kfst" if args.f == "auto" else args.f == "kfst":
        fst =  FST.from_kfst_file(args.fst_file, debug=args.d)
    
    else:
        fst = FST.from_att_file(args.fst_file, debug=args.d)
    
    if args.s:
        print(sorted(fst.symbols))
        return

    if args.d:
        print(sorted(fst.symbols))
        print(fst.final_states)

    while True:
        try:
            text = input("> ")

        except EOFError:
            break

        for output, w in fst.lookup(text):
            print(output, w)

if __name__ == "__main__":
    main()

