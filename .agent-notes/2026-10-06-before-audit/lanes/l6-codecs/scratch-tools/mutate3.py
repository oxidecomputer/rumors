"""Under-rejection calibration for collapsible pairs: python3 mutate3.py apply|revert <src dir>"""
import sys, os
HELPER = open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "mutate2.py")).read().split("HELPER = '''")[1].split("'''")[0]
M = [
 # M14: the admission walk never rejects a collapsible pair at an ordinary close.
 ("version/io/validate/admit.rs", 'if left_was_leaf && *is_leaf && *zero_delta {', 'if left_was_leaf && *is_leaf && *zero_delta && !crate::l6m(14) {'),
 # M15: the admission walk skips the final ancestors' closes.
 ("version/io/validate/admit.rs", '        while let Some(right) = self.path.pop() {\n            debug_assert!(right, "a done cursor\'s path is all right branches");\n            self.close_ancestor(&mut is_leaf, &mut zero_delta)?;\n        }',
  '        while let Some(right) = self.path.pop() {\n            debug_assert!(right, "a done cursor\'s path is all right branches");\n            if !crate::l6m(15) { self.close_ancestor(&mut is_leaf, &mut zero_delta)?; }\n        }'),
 # M16: the standalone validator never rejects a collapsible pair.
 ("version/io/validate.rs", 'if left_was_leaf && is_leaf && leaf_zero_delta {', 'if left_was_leaf && is_leaf && leaf_zero_delta && !crate::l6m(16) {'),
 # M17: the party validator never rejects an owned pair.
 ("party/io/validate.rs", 'if left_owned && owned {', 'if left_owned && owned && !crate::l6m(17) {'),
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
    assert s.count(a) == 1, (f, a, s.count(a))
    open(p, "w").write(s.replace(a, b))
print(mode, len(M))
