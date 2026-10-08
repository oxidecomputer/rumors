"""Upper-bound the intercept-adjusted heap reading (peak - 1024)/units per heap ceiling.

A rendered row shows max over its two samples of peak/units, rounded to 0.1, so
for the sample that attains it, (peak - 1024)/units <= (r + 0.05) - 1024/u for its
own u; using the row's larger u gives an upper bound over both samples."""
import math, sys
sys.path.insert(0, sys.argv[1])
from audit import parse
rows, _ = parse(sys.argv[2])
groups = {}
for r in rows:
    c = r['models'].get('heap', {}).get('ceiling')
    if c is None:
        continue
    u = max(r['heap_units'])
    adj = (r['heap'] + 0.05) - 1024 / u
    groups.setdefault(c, []).append((adj, r['op'], r['family'], r['section'], r['heap_units'], r['heap']))
for c, g in sorted(groups.items()):
    g.sort(reverse=True)
    top = g[0]
    print(f"ceiling {c}: max adjusted <= {top[0]:.3f} at {top[1]} x {top[2]} [{top[3]} {top[4]}] r={top[5]} -> x1.25 = {top[0]*1.25:.3f} -> {math.ceil(top[0]*1.25 - 1e-9)}")
    for x in g[1:4]:
        print(f"      next {x[0]:.3f} at {x[1]} x {x[2]} [{x[3]} {x[4]}] r={x[5]}")
