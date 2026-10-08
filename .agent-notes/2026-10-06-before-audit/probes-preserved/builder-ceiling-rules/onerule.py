"""Derive every measured board ceiling under the one rule:
ceil(1.25 * max over all judged samples of (reading - intercept) / units),
intercept 1024 for heap, 0 for scan and touch.

Rows give bounds; exact shard-log counters replace bounds where available."""
import math, sys
sys.path.insert(0, '.')
from audit import parse
rows, _ = parse(sys.argv[1])
exact = {}  # (op, family, section) -> list of (heap, units)
scale_name = {'3ff0000000000000': 'base', '4010000000000000': 'top', '3f847ae147ae147b': 'small'}
for log in sys.argv[2:]:
    sec = None
    for line in open(log):
        if line.startswith('amp-board-shard'):
            sec = scale_name[line.split()[-1]]
        if not line.startswith('cell\t'):
            continue
        f = line.rstrip('\n').split('\t')
        samples = []
        for s in (f[3:14], f[14:25]):
            m = s[2]; u = int(m.split(':')[1]) if m != '-' else int(s[0])
            samples.append((int(s[5]), u))
        exact[(f[1], f[2], sec)] = samples

def bounds(r, key, units_key, intercept):
    u1, u2 = r[units_key]
    v = r[key]
    if key == 'heap' and (r['op'], r['family'], r['section']) in exact:
        a = max((h - intercept) / u for h, u in exact[(r['op'], r['family'], r['section'])])
        return a, a, True
    lo = (v - 0.05) - intercept / min(u1, u2)
    hi = (v + 0.05) - intercept / max(u1, u2)
    return max(lo, 0), hi, False

groups = {
  'COMB 3.0': ('heap', 'heap_units', 1024, lambda r: r['models'].get('heap', {}).get('ceiling') == 3.0),
  'DESERIALIZE 4.0': ('heap', 'heap_units', 1024, lambda r: r['models'].get('heap', {}).get('ceiling') == 4.0),
  'RANK 5.0': ('heap', 'heap_units', 1024, lambda r: r['models'].get('heap', {}).get('ceiling') == 5.0),
  'RANKED 9.0': ('heap', 'heap_units', 1024, lambda r: r['models'].get('heap', {}).get('ceiling') == 9.0),
  'QUERY 152.0': ('heap', 'heap_units', 1024, lambda r: r['models'].get('heap', {}).get('ceiling') == 152.0),
  'MAX_HEAP 20 (global, no rule)': ('heap', 'heap_units', 1024, lambda r: r['models'].get('heap', {}).get('ceiling') is None),
  'FOLD 17.0': ('scan', 'scan_units', 0, lambda r: r['models'].get('scan', {}).get('ceiling') == 17.0),
  'MAX_SCAN 96 (global, no rule)': ('scan', 'scan_units', 0, lambda r: r['scan'] is not None and r['models'].get('scan', {}).get('ceiling') is None),
  'PARTY_COMPARISON 10 (ops.rs)': ('scan', 'scan_units', 0, lambda r: r['models'].get('scan', {}).get('ceiling') == 10.0),
  'MAX_TOUCHES 22': ('touch', 'touch_units', 0, lambda r: r['touch'] is not None and r['models'].get('touch', {}).get('ceiling') is None),
  'MASKED_HOLE 18 (ops.rs)': ('touch', 'touch_units', 0, lambda r: r['models'].get('touch', {}).get('ceiling') == 18.0),
}
for name, (key, uk, icpt, sel) in groups.items():
    bs = [(bounds(r, key, uk, icpt), r) for r in rows if sel(r) and r[key] is not None]
    max_lo = max(b[0][0] for b in bs)
    max_hi = max(b[0][1] for b in bs)
    rlo, rhi = math.ceil(1.25 * max_lo - 1e-9), math.ceil(1.25 * max_hi - 1e-9)
    deciding = max(bs, key=lambda b: b[0][0])
    r = deciding[1]
    # candidates that could exceed the best lower bound and are not exact
    open_rows = [b[1] for b in bs if b[0][1] > max_lo and not b[0][2] and b[0][1] * 1.25 > math.ceil(1.25 * max_lo - 1e-9) - 1e-9]
    print(f"{name}: adjusted max in [{max_lo:.4f}, {max_hi:.4f}] -> rule {rlo}" + ("" if rlo == rhi else f"..{rhi} UNRESOLVED") +
          f"; deciding {r['op']} x {r['family']} [{r['section']} {r[uk][0]}->{r[uk][1]}] rendered {r[key]}{' (exact)' if deciding[0][2] else ''}")
    for o in open_rows[:6]:
        print(f"    open: {o['op']} x {o['family']} [{o['section']} {o[uk]}] rendered {o[key]}")
