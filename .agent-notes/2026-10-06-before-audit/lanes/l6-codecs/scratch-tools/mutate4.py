"""Writer copy-path calibration (cached L6M=k): python3 mutate4.py apply|revert <src dir>"""
import sys, os
HELPER = open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "mutate2.py")).read().split("HELPER = '''")[1].split("'''")[0]
M = [
 ("version/io/writer.rs", "        for _ in 0..last_rel_depth {\n            self.path.push(true);\n            self.left_leaf.push(false);\n        }",
  "        for _ in 0..last_rel_depth {\n            self.path.push(true);\n            self.left_leaf.push(crate::l6m(21));\n        }"),
 ("version/io/writer.rs", "                debug_assert_eq!(cursor.position() - 1, end);\n                return;",
  "                debug_assert_eq!(cursor.position() - 1, end);\n                if crate::l6m(22) { self.topology.push(true); }\n                return;"),
 ("version/io/writer.rs", "            self.out.splice_continuation(bits, start, last_flag);",
  "            self.out.splice_continuation(bits, start, last_flag + u64::from(crate::l6m(23)));"),
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
