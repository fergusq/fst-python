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