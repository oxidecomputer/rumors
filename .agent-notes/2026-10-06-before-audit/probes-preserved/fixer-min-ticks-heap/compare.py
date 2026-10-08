import re, sys
# usage: compare.py PARENT_BOARD PARENT_PROBE RUN_LOG
def board(path):
    cells = {}
    section = 0
    for line in open(path, errors='replace'):
        if line.startswith('amp-board:') and 'green' in line:
            section += 1
            continue
        m = re.match(r'^(GREEN|RED)\s+(\S+)\s+(\S+)\s+(\S+)\s+B\s+(.*)$', line.rstrip())
        if not m: continue
        verdict, op, fam, sizes, rest = m.groups()
        vals = dict(re.findall(r'(heap|scan|touch)\[e\s*\S+\s+(\S+)/B\]', rest))
        cells[(section, op, fam)] = (verdict, sizes, vals, line.rstrip())
    return cells
def probe(path):
    fams = {}
    wrong = 0
    for line in open(path, errors='replace'):
        if not line.startswith('PROBE'): continue
        if 'WRONG' in line: wrong += 1
        p = line.split()
        fam = p[1]
        r = float([x for x in p if x.startswith('ratio=')][0][6:])
        if fam not in fams or r > fams[fam][0]:
            fams[fam] = (r, line.strip())
    return fams, wrong
pb, pp, run = sys.argv[1:4]
P, R = board(pb), board(run)
print(f'board cells parent={len(P)} run={len(R)}')
reds = [v[3] for v in R.values() if v[0] == 'RED']
print(f'RED cells in run: {len(reds)}'); [print('  ', l[:170]) for l in reds]
moved = []
for k, (v, s, vals, line) in R.items():
    if k not in P: moved.append(('NEW', k, vals)); continue
    pv = P[k][2]
    for cur in ('heap', 'scan', 'touch'):
        if pv.get(cur) != vals.get(cur):
            moved.append((cur, k, pv.get(cur), vals.get(cur)))
print(f'moved readings: {len(moved)}')
for m in sorted(moved, key=lambda m: (m[1][1], m[1][2], m[1][0])):
    print('  ', m)
pf, _ = probe(sys.argv[2]); rf, wrong = probe(run)
print(f'probe readings wrong: {wrong}')
for fam in sorted(rf, key=lambda f: -rf[f][0]):
    print(f'  {fam:24s} parent max {pf.get(fam,(0,))[0]:8.2f}  run max {rf[fam][0]:8.2f}   {rf[fam][1][:140]}')
