"""Compare two board captures' heap readings, and judge them against a per-cell heap table.

Usage: compare_heap.py PARENT.cap AFTER.cap HEAP_TABLE.rs [op-regex]

Capture lines: op, family, currency, scale, index, denominator, reading (tab-separated).
A cell's normalized heap is max over samples of (reading - 1024) / denominator, in thousandths,
which is the quantity the per-cell table bounds.
"""
import re
import sys
from collections import defaultdict

INTERCEPT = 1024


def load(path):
    cells = defaultdict(dict)
    with open(path) as f:
        for line in f:
            parts = line.rstrip("\n").split("\t")
            if len(parts) != 7:
                continue
            op, fam, cur, scale, idx, denom, reading = parts
            cells[(op, fam, cur)][(scale, idx)] = (int(denom), int(reading))
    return cells


def norm(samples):
    return max(max(0, r - INTERCEPT) * 1000 / d for d, r in samples.values() if d > 0)


def table(path):
    out = {}
    for m in re.finditer(r'\("([^"]+)", "([^"]+)", (\d+)\)', open(path).read()):
        out[(m.group(1), m.group(2))] = int(m.group(3))
    return out


parent, after, ceil = load(sys.argv[1]), load(sys.argv[2]), table(sys.argv[3])
pattern = re.compile(sys.argv[4] if len(sys.argv) > 4 else ".")

changed = 0
over = []
for key in sorted(set(parent) | set(after)):
    op, fam, cur = key
    if key not in parent or key not in after:
        print("MISSING", key, key in parent, key in after)
        continue
    if parent[key] != after[key]:
        changed += 1
    if cur != "heap" or not pattern.search(op):
        continue
    p, a = norm(parent[key]), norm(after[key])
    c = ceil.get((op, fam))
    flag = ""
    if c is not None and a > c:
        flag = "OVER-MAIN-CEILING"
        over.append(key)
    if parent[key] != after[key] or flag:
        print(f"{op:28} {fam:20} parent={p:9.0f} after={a:9.0f} main_ceiling={c} {flag}")

print(f"cells whose readings changed (any currency): {changed}")
print(f"heap cells over main's per-cell ceiling: {len(over)}")
