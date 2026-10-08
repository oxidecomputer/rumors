#!/usr/bin/env python3
"""Apply or remove rank.rs variants as exact string swaps: variant.py V1|V2|MUT [--remove].
V1: restructure in isolation (plain push). V2: little-endian materialization (apply after V1).
MUT: drop the integral bytes from the image (apply after V1)."""
import sys
P = "/Users/oxide/src/rumors-slot-14/crates/before/src/rank.rs"
V = {
 "V1": [("            growth::try_push(&mut image, group)?;\n", "            image.push(group);\n"),
        ("            growth::try_push(&mut self.bytes, 0)?;\n", "            self.bytes.push(0);\n")],
 "V2": [("            let lead = image.iter().take_while(|&&byte| byte == 0).count();\n            BigUint::from_bytes_be(&image[lead..]) >> pad\n",
         "            image.reverse();\n            let len = image.iter().rposition(|&byte| byte != 0).map_or(0, |last| last + 1);\n            BigUint::from_bytes_le(&image[..len]) >> pad\n")],
 "MUT": [("        let mut image = integral.to_bytes_be();\n", "        let mut image = vec![0u8];\n")],
}
swaps = V[sys.argv[1]]
if "--remove" in sys.argv: swaps = [(n, o) for o, n in reversed(swaps)]
t = open(P).read()
for old, new in swaps:
    assert t.count(old) == 1, (old, t.count(old)); t = t.replace(old, new)
open(P, "w").write(t); print("ok", sys.argv[1:])
