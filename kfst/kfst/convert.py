import argparse
from pathlib import Path

from .transducer import FST


def main():
    parser = argparse.ArgumentParser(description="Converts AT&T transducer files to KFST binary format.")
    parser.add_argument("input", type=Path, help="input AT&T transducer path")
    parser.add_argument("output", type=Path, help="output KFST transducer path")
    args = parser.parse_args()

    fst = FST.from_att_file(args.input)
    fst.to_kfst_file(args.output)


if __name__ == "__main__":
    main()