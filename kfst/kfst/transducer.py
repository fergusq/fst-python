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

import argparse
import re
from collections import defaultdict
from pathlib import Path
from typing import NamedTuple

from frozendict import frozendict


class FSTState(NamedTuple):
    state_num: int
    path_weight: float = 0
    input_flags: frozendict[str, str] = frozendict()
    output_flags: frozendict[str, str] = frozendict()
    output_symbols: list[str] = []

    def __repr__(self):
        return f"FSTState({self.state_num}, {self.path_weight}, {self.input_flags}, {self.output_flags}, {self.output_symbols})"


class FST(NamedTuple):
    """
    Represents a finite state transducer
    """
    final_states: dict[int, float]
    rules: defaultdict[int, defaultdict[str, list[tuple[int, str, int]]]]
    symbols: set[str]
    debug: bool = False

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

        fst = FST(
            final_states={},
            rules=defaultdict(lambda: defaultdict(list)),
            symbols=set(),
            debug=debug,
        )
        for line in att_code.splitlines():
            fields = line.split("\t")
            if len(fields) == 1:
                fst.final_states[int(fields[0])] = 0
            
            elif len(fields) == 2:
                fst.final_states[int(fields[0])] = float(fields[1])

            elif len(fields) == 4:
                fst.rules[int(fields[0])][fields[2]].append((int(fields[1]), fields[3], 0))
                fst.symbols.add(fields[2])
                fst.symbols.add(fields[3])

            elif len(fields) == 5:
                fst.rules[int(fields[0])][fields[2]].append((int(fields[1]), fields[3], float(fields[4])))
                fst.symbols.add(fields[2])
                fst.symbols.add(fields[3])

            else:
                raise RuntimeError("Cannot parse line:", line)

        return fst

    def split_to_symbols(self, text: str) -> list[str] | None:
        """
        Splits a given string into a list of symbols.
        For each position in the string, greedily selects the longest symbol that matches.

        Returns None if the string cannot be split into symbols.
        """
        slist = sorted(self.symbols, key=lambda x: -len(x))
        ans = []
        while text:
            for s in slist:
                if text.startswith(s):
                    ans.append(s)
                    text = text[len(s):]
                    break

            else:
                return None
        
        return ans

    def run_fst(self, input_symbols: list[str], state=FSTState(0)):
        """
        Runs the FST on the given input symbols, starting from the given state (by default 0).
        Yields a tuple of (output_symbols, path_weight) for each successful path.
        """
        transitions = self.rules[state.state_num]
        if not input_symbols:
            if state.state_num in self.final_states:
                yield state.output_symbols, state.path_weight
                return

        else:
            isymbol = input_symbols[0]
            for next_state, osymbol, weight in transitions[isymbol]:
                if self.debug:
                    print(state.state_num, "->", next_state, isymbol, osymbol, input_symbols, state)
                
                new_input_flags = self._update_flags(isymbol, state.input_flags)
                new_output_flags = self._update_flags(osymbol, state.output_flags)
                o = [osymbol] if not self._is_epsilon(osymbol) else []

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
                o = [osymbol] if not self._is_epsilon(osymbol) else []

                if new_output_flags is None:
                    continue # flag unification failed

                new_state = state._replace(
                    state_num=next_state,
                    path_weight=state.path_weight + weight,
                    output_flags=new_output_flags,
                    output_symbols=state.output_symbols + o
                )
                yield from self.run_fst(input_symbols, state=new_state)
    
    def lookup(self, input: str, state=FSTState(0)):
        """
        Runs the FST on the given input symbols, starting from the given state (by default 0).
        Yields a tuple of (output_symbols, path_weight) for each successful path.
        
        Otherwise same as run_fst, but operates on strings instead of symbol lists, filters out duplicate outputs and sorts the results by weight.
        """

        input_symbols = self.split_to_symbols(input)
        results = self.run_fst(input_symbols, state=state)
        results = sorted(results, key=lambda x: x[1])
        already_seen = set()
        for os, w in results:
            o = "".join(os)
            if o not in already_seen:
                yield o, w
                already_seen.add(o)
    
    def _update_flags(self, symbol: str, flags: frozendict[str, str]) -> frozendict[str, str] | None:
        if self._is_flag(symbol):
            # unification flag
            if m := re.fullmatch(r"@U.([^@.]*).([^@.]*)@", symbol):
                key = m.group(1)
                value = m.group(2)
                if key in flags and flags[key] != value and (not flags[key].startswith("@") or flags[key] == "@" + value):
                    return None # flag mismatch

                else:
                    return flags.set(key, value)
            
            # require flag (2 params)
            elif m := re.fullmatch(r"@R.([^@.]*).([^@.]*)@", symbol):
                key = m.group(1)
                value = m.group(2)
                if key in flags and self._test_flag(flags[key], value):
                    return flags

                else:
                    return None
            
            # require flag (1 param)
            elif m := re.fullmatch(r"@R.([^@.]*)@", symbol):
                key = m.group(1)
                if key in flags:
                    return flags

                else:
                    return None
            
            # disallow flag (2 params)
            elif m := re.fullmatch(r"@D.([^@.]*).([^@.]*)@", symbol):
                key = m.group(1)
                value = m.group(2)
                if key in flags and self._test_flag(flags[key], value):
                    return None

                else:
                    return flags
            
            # disallow flag (1 param)
            elif m := re.fullmatch(r"@D.([^@.]*)@", symbol):
                key = m.group(1)
                if key in flags:
                    return None

                else:
                    return flags
            
            # clear flag
            elif m := re.fullmatch(r"@C.([^@.]*)@", symbol):
                key = m.group(1)
                if key in flags:
                    return flags.delete(key)

                else:
                    return flags
            
            # positive (re)setting flag (2 params)
            elif m := re.fullmatch(r"@P.([^@.]*).([^@.]*)@", symbol):
                key = m.group(1)
                value = m.group(2)
                return flags.set(key, value)
            
            # negative (re)setting flag (2 params)
            elif m := re.fullmatch(r"@N.([^@.]*).([^@.]*)@", symbol):
                key = m.group(1)
                value = m.group(2)
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
        return re.match(r"^@[PNDRCU]\.", symbol)


def main():
    parser = argparse.ArgumentParser(description="Finite State Transducer interpreter written in Python")
    parser.add_argument("att_file", type=Path, help="FST in AT&T format")
    parser.add_argument("-d", action="store_true", help="enable debug mode")
    parser.add_argument("-s", action="store_true", help="print symbols in transducer and exit")
    args = parser.parse_args()

    fst = FST.from_att_file(args.att_file, debug=args.d)
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

        for os, w in fst.lookup(fst.split_to_symbols(text)):
            print("".join(os), w)

if __name__ == "__main__":
    main()

