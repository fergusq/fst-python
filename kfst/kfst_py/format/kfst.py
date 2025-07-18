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
import lzma
import struct

from ..symbols import Symbol, from_symbol_string
from ..transducer import FST

class KFSTReader:
    def __init__(self, buffer):
        self.buffer = buffer
        self.pointer = 0

    def read(self):
        final_states: dict[int, float] = {}
        rules: defaultdict[int, defaultdict[Symbol, list[tuple[int, Symbol, float]]]] = defaultdict(lambda: defaultdict(list))
        symbols: set[Symbol] = set()

        # Validate signature
        assert self.buffer[:4] == b"KFST"
        self.pointer += 4

        # Parse version
        (version,) = self.unpack_and_advance("!H")

        assert version == 0, "Unsupported KFST binary file version"

        # Parse header
        num_symbols, num_states, num_final_states, is_weighted = self.unpack_and_advance("!HII?")

        # Parse symbols
        symbols_list: list[Symbol] = []
        for _ in range(num_symbols):
            symbol = self.read_null_terminated_string()
            symbol_string = symbol.decode("utf-8")
            symbol_obj = from_symbol_string(symbol_string)
            symbols.add(symbol_obj)
            symbols_list.append(symbol_obj)
        
        lzma_data = self.buffer[self.pointer:]
        self.buffer = lzma.decompress(lzma_data)
        self.pointer = 0
        
        # Parse states
        for _ in range(num_states):
            from_state, to_state, input_symbol, output_symbol = self.unpack_and_advance("!IIHH")
            weight = 0

            if is_weighted:
                (weight,) = self.unpack_and_advance("!d")

            rules[from_state][symbols_list[input_symbol]].append((to_state, symbols_list[output_symbol], weight))
        
        # Parse final states
        for _ in range(num_final_states):
            (state,) = self.unpack_and_advance("!I")
            weight = 0
            if is_weighted:
                (weight,) = self.unpack_and_advance("!d")
            
            final_states[state] = weight
        
        return FST.from_rules(final_states, rules, symbols)

    def unpack_and_advance(self, format: str):
        size = struct.calcsize(format)
        ans = struct.unpack(format, self.buffer[self.pointer:self.pointer+size])
        self.pointer += size
        return ans

    def read_null_terminated_string(self) -> bytes:
        i = self.buffer[self.pointer:].find(b"\x00")
        string = self.buffer[self.pointer:self.pointer+i]
        self.pointer += i + 1
        return string


def decode_kfst(buffer: bytes) -> FST:
    """
    Decodes an FST in the KFST binary format.
    """

    return KFSTReader(buffer).read()


def encode_kfst(fst: FST) -> bytes:
    """
    Encodes the FST in the KFST binary format.
    """

    is_weighted = any(weight != 0 for weight in fst.final_states.values()) or any(weight != 0 for state in fst.rules.values() for transitions in state.values() for _, _, weight in transitions)

    assert len(fst.symbols) <= 2**16, "Too many symbols"

    num_rules = sum(len(transitions) for state in fst.rules.values() for transitions in state.values())

    assert num_rules <= 2**32, "Too many rules"
    assert len(fst.final_states) <= 2**32, "Too many final states"

    buffer = bytes()
    buffer += b"KFST"
    buffer += struct.pack("!H", 0) # version
    buffer += struct.pack("!HII?", len(fst.symbols), num_rules, len(fst.final_states), is_weighted) # header

    # Write symbols
    symbols_list = sorted(fst.symbols, key=lambda s: (len(s.get_symbol()), s.get_symbol()))
    for symbol in symbols_list:
        buffer += symbol.get_symbol().encode("utf-8") + b"\x00"

    state_bytes = []
    
    # Write states
    for from_state, transitions in sorted(fst.rules.items()):
        for input_symbol, transitions_list in transitions.items():
            for to_state, output_symbol, weight in transitions_list:
                state_bytes.append(struct.pack("!IIHH", from_state, to_state, symbols_list.index(input_symbol), symbols_list.index(output_symbol)))
                if is_weighted:
                    state_bytes.append(struct.pack("!d", weight))
    
    # Write final states
    for state, weight in fst.final_states.items():
        state_bytes.append(struct.pack("!I", state))
        if is_weighted:
            state_bytes.append(struct.pack("!d", weight))
    
    data = b"".join(state_bytes)
    buffer += lzma.compress(data)
    return buffer