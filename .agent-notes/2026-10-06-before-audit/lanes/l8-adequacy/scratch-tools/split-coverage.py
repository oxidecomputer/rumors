#!/usr/bin/env python3
"""Split an lcov report into one file per production module under coverage/.

Usage: split-coverage.py <lcov file> <worktree root> <label>

Each module file lists never-executed lines and branch outcomes never taken,
with the source text, plus a header of line and branch totals. Test-only code
(src/testing/, tests.rs, tests/) is skipped. A summary table goes to stdout.
"""
import os, re, sys, collections

lcov, root, label = sys.argv[1:4]
S = os.path.dirname(os.path.abspath(__file__))
out = os.path.join(S, "coverage")
os.makedirs(out, exist_ok=True)

def production(path):
    rel = path.split("/src/rumors-audit-l8-adequacy/", 1)[-1]
    if not (rel.startswith("crates/before/src/") or rel.startswith("crates/suanpan/src/")):
        return None
    if "/testing/" in rel or rel.endswith("/testing.rs") or rel.endswith("tests.rs") or "/tests/" in rel:
        return None
    return rel

records = {}
cur = None
for line in open(lcov):
    line = line.rstrip("\n")
    if line.startswith("SF:"):
        rel = production(line[3:])
        cur = None if rel is None else records.setdefault(rel, {"da": {}, "br": collections.defaultdict(list)})
    elif cur is None:
        continue
    elif line.startswith("DA:"):
        n, hits = line[3:].split(",")[:2]
        cur["da"][int(n)] = max(cur["da"].get(int(n), 0), int(hits))
    elif line.startswith("BRDA:"):
        n, block, branch, taken = line[5:].split(",")
        cur["br"][int(n)].append(0 if taken == "-" else int(taken))

rows = []
for rel, rec in sorted(records.items()):
    src = open(os.path.join(root, rel)).read().split("\n")
    da, br = rec["da"], rec["br"]
    lf, lh = len(da), sum(1 for h in da.values() if h > 0)
    brf = sum(len(v) for v in br.values())
    brh = sum(1 for v in br.values() for t in v if t > 0)
    missed_lines = sorted(n for n, h in da.items() if h == 0)
    partial = sorted(n for n, v in br.items() if any(t == 0 for t in v))
    rows.append((rel, lf, lh, brf, brh))
    name = rel.replace("crates/", "").replace("/src/", "/").replace("/", "__").replace(".rs", "") + ".txt"
    with open(os.path.join(out, name), "w") as f:
        f.write(f"# coverage of {rel} ({label})\n")
        f.write(f"# lines {lh}/{lf}; branch outcomes {brh}/{brf}\n")
        f.write("# never-executed lines:\n")
        for n in missed_lines:
            f.write(f"L{n:5}  {src[n-1].strip()}\n")
        f.write("# branches with an outcome never taken (outcome hit counts):\n")
        for n in partial:
            f.write(f"B{n:5}  {br[n]}  {src[n-1].strip()}\n")

print(f"{'module':70} {'lines':>12} {'branches':>12}")
for rel, lf, lh, brf, brh in sorted(rows, key=lambda r: (r[3]-r[4]), reverse=True):
    print(f"{rel:70} {lh:5}/{lf:<6} {brh:5}/{brf:<6}")
tl = sum(r[1] for r in rows); th = sum(r[2] for r in rows)
tb = sum(r[3] for r in rows); tbh = sum(r[4] for r in rows)
print(f"{'TOTAL':70} {th:5}/{tl:<6} {tbh:5}/{tb:<6}")
