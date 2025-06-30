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

import argparse

from typing import TYPE_CHECKING
from . import FST as FST # type: ignore
from pathlib import Path

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
        print(sorted(s.get_symbol() for s in fst.symbols))
        return

    if args.d:
        print(sorted(s.get_symbol() for s in fst.symbols))
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
else:
    import sys

    # Determine which kfst impl we need to copy from

    from . import BACKEND

    if BACKEND == "kfst" or TYPE_CHECKING:

        # Patch in Python implementation

        from kfst_py.transducer import *
        import kfst_py
        if hasattr(kfst_py.transducer, "__all__"):
            __all__ = kfst_py.transducer.__all__ # type: ignore
        sys.modules['kfst.transducer'] = kfst_py.transducer

    else:

        # Patch in rust implementation

        from kfst_rs.transducer import *
        import kfst_rs
        if hasattr(kfst_rs, "__all__"):
            __all__ = kfst_rs.transducer.__all__
        sys.modules['kfst.transducer'] = kfst_rs.transducer