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

# Prefer kfst_rs, otherwise import from own implementation

BACKEND: str
"""
kfst has two potential back-ends: kfst_rs (written in rust) and kfst's built-in python implementation
"""

from typing import TYPE_CHECKING
import sys


# This is quite horribly hacky.
# It's trying to make loading functionality from two different places transparent
# Dumbly, the Symbol class comes from the python implementation anyway (as it is mostly interesting at type checking time anyway)
# This is further complicated by:
# * needing to support import kfst; kfst.x.y along with import kfst.x; kfst.x.y and from kfst import x; x.y and from kfst.x import y; y
# * needing to support calling transducer.py as a module
# * not only have correct run-time behaviour but also correctly type


if not TYPE_CHECKING:
    try:
        from kfst_rs import *
        import kfst_rs
        if hasattr(kfst_rs, "__all__"):
            __all__ = kfst_rs.__all__
        from kfst_py.symbols import Symbol
        kfst_rs.symbols.Symbol = Symbol
        sys.modules['kfst.symbols'] = kfst_rs.symbols
        # nb. transducer patches itself
        BACKEND = "kfst_rs"
    except ImportError:
        BACKEND = "kfst"

if TYPE_CHECKING or BACKEND == "kfst": # the python back-end is actual python code; point type checkers to it
    from kfst_py import *
    import kfst_py
    if hasattr(kfst_py, "__all__"):
        __all__ = kfst_py.__all__ # type: ignore
    sys.modules['kfst.symbols'] = kfst_py.symbols # type: ignore
    # nb. transducer patches itself
    BACKEND = "kfst"