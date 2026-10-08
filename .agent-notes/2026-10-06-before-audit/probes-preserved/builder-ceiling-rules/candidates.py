import sys; sys.path.insert(0, '.')
from audit import parse
rows, _ = parse('base-board.log')
print("QUERY rows, larger sample >= 2048, reading > 68.0:")
for r in rows:
    if r['models'].get('heap',{}).get('ceiling') == 152.0 and r['heap_units'][1] >= 2048 and r['heap'] > 68.0:
        print("  ", r['section'], r['op'], r['family'], r['heap_units'], r['heap'])
print("RANKED rows, larger sample >= 2048, reading >= 7.0:")
for r in rows:
    if r['models'].get('heap',{}).get('ceiling') == 9.0 and r['heap_units'][1] >= 2048 and r['heap'] >= 7.0:
        print("  ", r['section'], r['op'], r['family'], r['heap_units'], r['heap'])
