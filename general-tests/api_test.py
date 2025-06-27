import kfst_py
import kfst
assert kfst.BACKEND == "kfst_rs"
from pathlib import Path
import time

for k in [kfst, kfst_py]:
    
    print(f"Testing {k}")
    t = time.time()

    print("Check for basic symbol functionality")

    assert k.symbols.SpecialSymbol.EPSILON.is_epsilon()
    assert k.symbols.FlagDiacriticSymbol.from_symbol_string("@U.X.Y@").is_epsilon()
    assert not k.symbols.from_symbol_string("kissa").is_epsilon()

    print("Check that all IO functions that ostensibly accept a path or a string actually do so")

    fst = k.transducer.FST.from_kfst_file("../pyvoikko/pyvoikko/voikko.kfst")
    fst = k.transducer.FST.from_kfst_file(Path("../pyvoikko/pyvoikko/voikko.kfst"))

    fst.to_att_file("/tmp/test.att")
    fst.to_att_file(Path("/tmp/test.att"))

    fst = k.transducer.FST.from_att_file("/tmp/test.att")
    fst = k.transducer.FST.from_att_file(Path("/tmp/test.att"))

    fst.to_kfst_file("/tmp/test.kfst")
    fst.to_kfst_file(Path("/tmp/test.kfst"))

    print(f"Total {time.time() - t:0.2f} seconds.")
