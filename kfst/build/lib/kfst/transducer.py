import argparse
from . import FST as FST
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
    raise ImportError("Import kfst.transducer using from kfst import transducer instead.")