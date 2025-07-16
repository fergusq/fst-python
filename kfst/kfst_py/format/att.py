# This file is part of KFST.
#
# (c) 2023-2025 Iikka Hauhio <iikka.hauhio@helsinki.fi> and Théo Salmenkivi-Friberg <theo.friberg@helsinki.fi>
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

from collections import defaultdict

from ..symbols import Symbol, from_symbol_string
from ..transducer import FST

def unescape_symbol(att_symbol: str) -> str:
    """
    Cleans up escapes in att; in practice @_TAB_@ to actual tab character
    Open question: should newlines be handled somehow?
    """
    return att_symbol.replace("@_TAB_@", "\t")

def escape_symbol(symbol: str) -> str:
    """
    Escapes symbol for att compatibility; in practice converts tabs to @_TAB_@ sequences.
    Open question: should newlines be handled somehow?
    """
    return symbol.replace("\t", "@_TAB_@") # Single character, always should be replaced

def decode_att(att_code: str) -> FST:
    """
    Parses a transducer in AT&T format and returns an FST object.
    """

    final_states: dict[int, float] = {}
    rules: defaultdict[int, defaultdict[Symbol, list[tuple[int, Symbol, float]]]] = defaultdict(lambda: defaultdict(list))
    symbols: set[Symbol] = set()
    for line in att_code.splitlines():
        fields = line.split("\t")
        if len(fields) == 1:
            final_states[int(fields[0])] = 0
        
        elif len(fields) == 2:
            final_states[int(fields[0])] = float(fields[1])

        elif len(fields) == 4:
            symbol1 = from_symbol_string(unescape_symbol(fields[2]))
            symbol2 = from_symbol_string(unescape_symbol(fields[3]))
            rules[int(fields[0])][symbol1].append((int(fields[1]), symbol2, 0))
            symbols.add(symbol1)
            symbols.add(symbol2)

        elif len(fields) == 5:
            symbol1 = from_symbol_string(unescape_symbol(fields[2]))
            symbol2 = from_symbol_string(unescape_symbol(fields[3]))
            rules[int(fields[0])][symbol1].append((int(fields[1]), symbol2, float(fields[4])))
            symbols.add(symbol1)
            symbols.add(symbol2)

        else:
            raise RuntimeError("Cannot parse line:", line)

    return FST.from_rules(final_states, rules, symbols)

def encode_att(fst: FST) -> str:
    """
    Encodes the FST in the AT&T tabular format.
    """

    ans = []
    for final_state, weight in fst.final_states.items():
        if weight == 0:
            ans.append(str(final_state))
        else:
            ans.append("\t".join([str(final_state), str(weight)]))
    
    for from_state, transitions in fst.rules.items():
        for input_symbol, transitions_list in transitions.items():
            for to_state, output_symbol, weight in transitions_list:
                if weight == 0:
                    ans.append("\t".join([str(from_state), str(to_state), escape_symbol(input_symbol.get_symbol()), escape_symbol(output_symbol.get_symbol())]))
                else:
                    ans.append("\t".join([str(from_state), str(to_state), escape_symbol(input_symbol.get_symbol()), escape_symbol(output_symbol.get_symbol()), str(weight)]))
    
    return "\n".join(ans)