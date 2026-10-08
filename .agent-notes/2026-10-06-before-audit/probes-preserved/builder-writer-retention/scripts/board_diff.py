"""Compare two amp_board logs (acceptance sections + worst-case maps).

Usage: python3 -I board_diff.py parent.log tip.log
Rows are keyed by (section index, verdict-free op, family, sizes).
"""
import re, sys

ROW = re.compile(r'^(GREEN|RED)\s+(\S+)\s+(\S+)\s+(\d+->\d+)\s+B\s+heap\[e\s*(\S+)\s+(\S+)\]\s+scan\[e\s*(\S+)\s+(\S+)\]\s+touch\[e\s*(\S+)\s+(\S+)\]')

def parse(path):
    rows, worst, section = {}, [], -1
    in_worst = False
    for line in open(path):
        line = line.rstrip('\n')
        if line.startswith('amplification board:'):
            section += 1
        if line.startswith('worst-case map'):
            in_worst = True
        if in_worst:
            worst.append(line)
            continue
        m = ROW.match(line)
        if m:
            verdict, op, fam, sizes, he, hc, se, sc, te, tc = m.groups()
            key = (section, op, fam)
            assert key not in rows, key
            rows[key] = dict(verdict=verdict, sizes=sizes, heap=(he, hc), scan=(se, sc), touch=(te, tc), line=line)
    return rows, worst

p_rows, p_worst = parse(sys.argv[1])
t_rows, t_worst = parse(sys.argv[2])
print(f"rows: parent {len(p_rows)}, tip {len(t_rows)}")
print("keys only in parent:", sorted(set(p_rows) - set(t_rows))[:10])
print("keys only in tip:", sorted(set(t_rows) - set(p_rows))[:10])
moved = {'heap': [], 'scan': [], 'touch': [], 'verdict': [], 'sizes': []}
for key in sorted(set(p_rows) & set(t_rows)):
    a, b = p_rows[key], t_rows[key]
    for field in moved:
        if a[field] != b[field]:
            moved[field].append((key, a[field], b[field]))
for field, items in moved.items():
    print(f"\n== {field}: {len(items)} rows moved")
    for key, a, b in items:
        print(f"  {key}: {a} -> {b}")
print("\n== worst-case map diff")
import difflib
for d in difflib.unified_diff(p_worst, t_worst, 'parent', 'tip', lineterm='', n=0):
    print(d)
