import sys; sys.path.insert(0, '.')
from audit import parse
rows, _ = parse('base-board.log')
def check(label, sel, key, units_key, new, intercept):
    bad = []
    for r in rows:
        if not sel(r) or r[key] is None: continue
        u2 = r[units_key][1]; u1 = r[units_key][0]
        # conservative: the larger per-unit reading at the larger sample's allowance
        if r[key] > new + intercept / max(u1, u2) + 1e-9:
            bad.append((r['section'], r['op'], r['family'], r[units_key], r[key]))
    print(f"{label} -> {new}: {'ok' if not bad else 'CONSERVATIVE FAIL'} {bad[:5]}")
heap = lambda c: (lambda r: r['models'].get('heap', {}).get('ceiling') == c)
check("QUERY", heap(152.0), 'heap', 'heap_units', 86, 1024)
check("DESERIALIZE", heap(4.0), 'heap', 'heap_units', 5, 1024)
check("DESERIALIZE keep", heap(4.0), 'heap', 'heap_units', 4, 1024)
check("COMB", heap(3.0), 'heap', 'heap_units', 4, 1024)
check("FOLD", lambda r: r['models'].get('scan', {}).get('ceiling') == 17.0, 'scan', 'scan_units', 16, 0)
check("TOUCH", lambda r: r['models'].get('touch', {}).get('ceiling') is None, 'touch', 'touch_units', 24, 0)
