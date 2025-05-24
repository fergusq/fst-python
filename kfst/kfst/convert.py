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
from pathlib import Path

from . import transducer


def main():
    parser = argparse.ArgumentParser(description="Converts AT&T transducer files to KFST binary format.")
    parser.add_argument("input", type=Path, help="input AT&T transducer path")
    parser.add_argument("output", type=Path, help="output KFST transducer path")
    args = parser.parse_args()

    fst = transducer.FST.from_att_file(args.input)
    fst.to_kfst_file(args.output)


if __name__ == "__main__":
    main()