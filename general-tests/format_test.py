import kfst_py
import kfst
assert kfst.BACKEND == "kfst_rs"

print("Format test 1: att handles tabs and spaces correctly")

code = """4
0	1	@_TAB_@	a
1	2	b	@_TAB_@x
2	3	@_SPACE_@	c
3	4	d	@_SPACE_@
"""

py_fst = kfst_py.FST.from_att_code(code)
assert list(py_fst.lookup("\tb d")) == [("a\txc ", 0)]


rs_fst = kfst.FST.from_att_code(code)
assert list(rs_fst.lookup("\tb d")) == [("a\txc ", 0)]

print("Format test 2: kfst export produces identical transducers on voikko; identical fst object per both kfst-rs and kfst-py (if not byte-identical)")

assert kfst_py.FST.from_kfst_bytes(kfst.FST.from_att_file("voikko.att").to_kfst_bytes()) == kfst_py.FST.from_kfst_bytes(kfst_py.FST.from_att_file("voikko.att").to_kfst_bytes())

def compare_kfst_rs(a, b):
    assert dict(a.final_states) == dict(b.final_states)
    cvt = lambda x: {source: {sym: list(trans) for sym, trans in rulebook.items()} for source, rulebook in x.rules.items()}
    assert cvt(a) == cvt(b)
    assert set(a.symbols) == set(b.symbols)
    assert a.debug == b.debug
compare_kfst_rs(kfst.FST.from_kfst_bytes(kfst.FST.from_att_file("voikko.att").to_kfst_bytes()), kfst.FST.from_kfst_bytes(kfst_py.FST.from_att_file("voikko.att").to_kfst_bytes()))