#!/usr/bin/env python3
"""Apply or remove one growth.rs mutant as an exact string swap: mutant.py A|B [--remove]."""
import sys
P = "/Users/oxide/src/rumors-slot-14/crates/before/src/growth.rs"
M = {
 "A": ("            .min(max)\n            .max(required);\n", "            .min(max);\n"),
 "B": ("        &[2, 4, 8]\n", "        &[]\n"),
}
old, new = M[sys.argv[1]]
if "--remove" in sys.argv: old, new = new, old
t = open(P).read(); assert t.count(old) == 1, t.count(old)
open(P, "w").write(t.replace(old, new)); print("ok")
