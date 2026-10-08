"""Reach probes (cached L6M=k panics on entry): python3 reach5.py apply|revert <src dir>"""
import sys, os
HELPER = open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "mutate2.py")).read().split("HELPER = '''")[1].split("'''")[0]
M = [
 ("version/io/writer.rs", "    fn splice_continuation(&mut self, src: &Bits, start: u64, end: u64) {\n        let mut cursor = BitsReader::at(src, start);",
  "    fn splice_continuation(&mut self, src: &Bits, start: u64, end: u64) {\n        assert!(!crate::l6m(31), \"REACH split splice\");\n        let mut cursor = BitsReader::at(src, start);"),
 ("version/io/writer.rs", "        let last_flag = end - last_code_len - 1;",
  "        assert!(!(crate::l6m(32) && first_rel_depth == 1), \"REACH depth-1 copy\");\n        assert!(!(crate::l6m(33) && last_code_len > NARROW_CODE_BITS), \"REACH wide-last copy\");\n        let last_flag = end - last_code_len - 1;"),
]
mode, src = sys.argv[1], sys.argv[2]
lib = os.path.join(src, "lib.rs")
t = open(lib).read()
if mode == "apply":
    assert HELPER not in t
    open(lib, "w").write(t + HELPER)
else:
    assert t.count(HELPER) == 1
    open(lib, "w").write(t.replace(HELPER, ""))
for f, old, new in M:
    p = os.path.join(src, f)
    s = open(p).read()
    a, b = (old, new) if mode == "apply" else (new, old)
    assert s.count(a) == 1, (f, a[:50], s.count(a))
    open(p, "w").write(s.replace(a, b))
print(mode, len(M))
