import re, sys
from collections import defaultdict
path = sys.argv[1]; ops = sys.argv[2].split(',')
lines = [l for l in open(path) if l.startswith('L7FUEL') or ' L7FUEL ' in l]
guests = ['a', 'w', 'k1']
per = len(lines) // 3
data = defaultdict(dict)
for i, l in enumerate(lines):
    g = guests[i // per]
    d = dict(re.findall(r'(\w+)=(\S+)', l))
    data[(int(d['family']), d['op'])].setdefault(g, []).append((int(d['r']), int(d['bytes']), int(d['fuel']), float(d['per_byte'])))
for (fam, op), gs in sorted(data.items()):
    if op not in ops: continue
    row = []
    for g in guests:
        vals = gs.get(g, [])
        row.append(f"{g}: " + ", ".join(f"{v[3]:.1f}" for v in vals))
    print(f"family {fam} {op:9s} | " + " | ".join(row))
