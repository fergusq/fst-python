import kfst_py
import kfst
assert kfst.BACKEND == "kfst_rs"
from pathlib import Path
import time
from immutables import Map

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

    print("Check that lookup has sane tokenization settings")

    # Should work cleanly

    text = "🐈‍⬛" # <- not found in Voikko
    fst.split_to_symbols(text)
    fst.lookup(text)
    assert fst.split_to_symbols(text, True) is not None

    list(fst.lookup(text, k.transducer.FSTState(0), True))

    # Should break

    assert fst.split_to_symbols(text, False) is None

    try:
        list(fst.lookup(text, k.transducer.FSTState(0), False))
    except k.TokenizationException:
        pass
    else:
        assert False
    
    # FSTStates can be constructed

    # 1. With and without indices

    assert kfst.transducer.FSTState(state_num=0,
    path_weight=0,
    input_indices = tuple()
    ) == kfst.transducer.FSTState(state_num=0,
    path_weight=0,
    )

    # 2. Full parameter list with names matches

    assert kfst.transducer.FSTState(
        state_num=3,
        path_weight=12.5,
        input_flags=Map({"x": (True, "y")}),
        output_flags=Map({"u": (True, "v")}),
        output_symbols=(kfst.symbols.SpecialSymbol.EPSILON,),
        input_indices=(5,)
    ) == kfst.transducer.FSTState(
        3,
        12.5,
        Map({"x": (True, "y")}),
        Map({"u": (True, "v")}),
        (kfst.symbols.SpecialSymbol.EPSILON,),
        (5,)
    )

    # 3. Old-style (ie. before adding indices) call OK

    assert kfst.transducer.FSTState(
        state_num=3,
        path_weight=12.5,
        input_flags=Map({"x": (True, "y")}),
        output_flags=Map({"u": (True, "v")}),
        output_symbols=(kfst.symbols.SpecialSymbol.EPSILON,),
        input_indices=tuple()
    ) == kfst.transducer.FSTState(
        3,
        12.5,
        Map({"x": (True, "y")}),
        Map({"u": (True, "v")}),
        (kfst.symbols.SpecialSymbol.EPSILON,),
    )
    
    assert kfst.transducer.FSTState(
        state_num=3,
        path_weight=12.5,
        input_flags=Map({"x": (True, "y")}),
        output_flags=Map({"u": (True, "v")}),
        output_symbols=(kfst.symbols.SpecialSymbol.EPSILON,),
    ) == kfst.transducer.FSTState(
        3,
        12.5,
        Map({"x": (True, "y")}),
        Map({"u": (True, "v")}),
        (kfst.symbols.SpecialSymbol.EPSILON,),
        tuple()
    )

    print("FSTState items are of valid types")

    assert kfst.transducer.FSTState(
        state_num=3,
        path_weight=12.5,
        input_flags=Map({"x": (True, "y")}),
        output_flags=Map({"u": (True, "v")}),
        output_symbols=(kfst.symbols.SpecialSymbol.EPSILON,),
    ).input_flags == Map({"x": (True, "y")})

    assert kfst.transducer.FSTState(
        state_num=3,
        path_weight=12.5,
        input_flags=Map({"x": (True, "y")}),
        output_flags=Map({"u": (True, "v")}),
        output_symbols=(kfst.symbols.SpecialSymbol.EPSILON,),
    ).output_flags == Map({"u": (True, "v")})

    assert kfst.transducer.FSTState(
        state_num=3,
        path_weight=12.5,
        input_flags=Map({"x": (True, "y")}),
        output_flags=Map({"u": (True, "v")}),
        output_symbols=(kfst.symbols.SpecialSymbol.EPSILON, kfst.symbols.SpecialSymbol.UNKNOWN, kfst.symbols.SpecialSymbol.IDENTITY),
    ).output_symbols == (kfst.symbols.SpecialSymbol.EPSILON, kfst.symbols.SpecialSymbol.UNKNOWN, kfst.symbols.SpecialSymbol.IDENTITY)

    assert kfst.transducer.FSTState(
        state_num=3,
        path_weight=12.5,
        input_flags=Map({"x": (True, "y")}),
        output_flags=Map({"u": (True, "v")}),
        output_symbols=(kfst.symbols.SpecialSymbol.EPSILON, kfst.symbols.SpecialSymbol.UNKNOWN, kfst.symbols.SpecialSymbol.IDENTITY),
        input_indices=(10, 20, 30),
    ).input_indices == (10, 20, 30)


    print(f"Total {time.time() - t:0.2f} seconds.")
