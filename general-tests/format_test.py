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