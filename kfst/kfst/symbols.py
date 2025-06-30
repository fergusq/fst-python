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

import sys
from typing import TYPE_CHECKING

# Determine which kfst impl we need to copy from

from . import BACKEND

if BACKEND == "kfst" or TYPE_CHECKING:

    # Patch in Python implementation

    from kfst_py.symbols import *
    import kfst_py
    if hasattr(kfst_py.symbols, "__all__"):
        __all__ = kfst_py.symbols.__all__ # type: ignore
    sys.modules['kfst.symbols'] = kfst_py.symbols

else:

    # Patch in rust implementation

    from kfst_rs.symbols import *
    import kfst_rs
    if hasattr(kfst_rs, "__all__"):
        __all__ = kfst_rs.symbols.__all__
    from kfst_py.symbols import Symbol
    kfst_rs.symbols.Symbol = Symbol
    sys.modules['kfst.symbols'] = kfst_rs.symbols