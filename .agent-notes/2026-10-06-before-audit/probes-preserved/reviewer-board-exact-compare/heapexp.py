import re, sys
def load(p):
    out = {}
    section = None
    for line in open(p):
        if line.startswith('==='):
            section = line.strip(); continue
        m = re.match(r'(GREEN|RED)\s+(\S+)\s+(\S+)\s+(\S+)\s+\S+\s+heap\[e\s+(\S+)\s+(\S+)\]', line)
        if m:
            out[(section, m.group(2), m.group(3), m.group(4))] = (m.group(5), m.group(6), m.group(1))
    return out
a, b = load(sys.argv[1]), load(sys.argv[2])
diffs = []
for k in a:
    if k in b and a[k] != b[k]:
        diffs.append((k, a[k], b[k]))
rows = {(k[1], k[2]) for k, _, _ in diffs}
mx = 0
for k, x, y in diffs:
    try: d = abs(float(x[0]) - float(y[0]))
    except ValueError: d = None
    if d is not None: mx = max(mx, d)
print(len(diffs), "rendered heap lines differ over", len(rows), "cells; max exponent move", round(mx, 2))
for k, x, y in sorted(diffs, key=lambda t: -abs(float(t[1][0]) - float(t[2][0])) if t[1][0][0] != '-' or t[1][0] != '-.--' else 0)[:8]:
    print(k, x, y)
