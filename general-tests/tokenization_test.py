import kfst_py
import kfst
assert kfst.BACKEND == "kfst_rs"

code = """2
0	1	a	A
0	1	b	B
0	2	ab	XY"""

fst_py = kfst_py.FST.from_att_code(code)
assert len(fst_py.split_to_symbols("ab")) == 1
print(fst_py.split_to_symbols("ab"))
print([x.get_symbol() for x in fst_py.symbols])


fst_rs = kfst.FST.from_att_code(code)
assert len(fst_rs.split_to_symbols("ab")) == 1
print(fst_rs.split_to_symbols("ab"))
print([x.get_symbol() for x in fst_rs.symbols])