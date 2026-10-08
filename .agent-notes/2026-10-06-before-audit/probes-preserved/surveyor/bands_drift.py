"""Compare each fuzz-fit band's ceiling at its largest calibrated size,
committed bands.rs against a fresh calibration, in log10 fuel.

Usage: python3 -I bands_drift.py <committed bands.rs> <refit bands.rs>
"""
import math, re, sys

def parse(path):
    text = open(path).read()
    bands = {}
    for m in re.finditer(r'    Band \{\n(.*?)\n    \}', text, re.S):
        body = m.group(1)
        field = lambda k: re.search(k + r': ([^,]+),', body).group(1)
        key = (field('kernel').strip('"'), field('rejected'), field('min_denom'))
        bands.setdefault(key, []).append((float(field('slope')), float(field('intercept')),
                                          float(field('width_above')),
                                          int(field('max_denom').replace('_', ''))))
    return bands

committed, refit = parse(sys.argv[1]), parse(sys.argv[2])
assert set(committed) == set(refit)
moves = []
for key in committed:
    for (s0, i0, w0, d), (s1, i1, w1, _) in zip(committed[key], refit[key]):
        x = math.log10(d)
        moves.append(((i1 + s1 * x + w1) - (i0 + s0 * x + w0), key))
moves.sort()
print('bands:', len(moves))
print('refit ceiling minus committed ceiling at max_denom (log10); negative = committed looser')
for delta, key in moves[:3] + moves[-6:]:
    print('%+.4f' % delta, key)
print('|delta| > 0.01:', sum(abs(d) > 0.01 for d, _ in moves))
