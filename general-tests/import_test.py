# Top level

import kfst
kfst.TokenizationException
kfst.FST
from kfst import TokenizationException, FST

#  ---- Transducer ----

# Syntax 1
import kfst
kfst.transducer.FSTState(0)

# Syntax 2
import kfst.transducer
kfst.transducer.FSTState(0)

# Syntax 3
from kfst import transducer
transducer.FSTState(0)

# Syntax 4
from kfst.transducer import FSTState
FSTState(0)

#  ---- Symbols (also check Symbol that gets monkey-patched from weird place) ----

# Syntax 1
import kfst
kfst.symbols.StringSymbol("k", True)
kfst.symbols.Symbol

# Syntax 2
import kfst.symbols
kfst.symbols.StringSymbol("k", True)
kfst.symbols.Symbol

# Syntax 3
from kfst import symbols
symbols.StringSymbol("k", True)
symbols.Symbol

# Syntax 4
from kfst.symbols import StringSymbol, Symbol
StringSymbol("k", True)
Symbol