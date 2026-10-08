"""Compare two per-sample rank fuel dumps (case step kernel rejected denom_bits fuel)."""
import sys, collections
def load(p):
    rows = {}
    for line in open(p):
        f = line.split()
        if len(f) != 6 or not f[0].isdigit():
            continue
        rows[(int(f[0]), int(f[1]))] = (f[2], f[3], int(f[4]), int(f[5]))
    return rows
a, b = load(sys.argv[1]), load(sys.argv[2])
assert a.keys() == b.keys(), "sample sets differ"
groups = collections.defaultdict(list)
for k in a:
    ka, ra, da, fa = a[k]; kb, rb, db, fb = b[k]
    assert (ka, ra, da) == (kb, rb, db), k
    groups[(ka, ra)].append((fb - fa, fa, fb))
for (kern, rej), ds in sorted(groups.items()):
    d = [x[0] for x in ds]
    nz = [x for x in d if x != 0]
    pf = [x[1] for x in ds]; tf = [x[2] for x in ds]
    print(f"{kern} rejected={rej}: n={len(d)} moved={len(nz)} "
          f"delta[min={min(d)} max={max(d)} mean={sum(d)/len(d):.1f}] "
          f"moved-delta[{min(nz) if nz else '-'}..{max(nz) if nz else '-'}] "
          f"fuel A[{min(pf)}..{max(pf)}] B[{min(tf)}..{max(tf)}]")
