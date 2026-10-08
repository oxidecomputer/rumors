"""Tabulates nextest PASS/SLOW/TIMEOUT durations per test from log text on stdin."""
import sys, re, statistics, collections
d = collections.defaultdict(list); s = collections.defaultdict(float); t = collections.Counter()
pat = re.compile(r"(PASS|SLOW|TIMEOUT|LEAK) \[ *>?([0-9.]+)s\]\s*(?:\([^)]*\))?\s*(\S+) (\S+)")
for l in sys.stdin:
    m = pat.search(l)
    if not m: continue
    k, v, b, n = m.groups(); v = float(v); n = b + " " + n
    if k in ("PASS", "LEAK"): d[n].append(v)
    elif k == "SLOW": s[n] = max(s[n], v)
    else: t[n] += 1
for n in sorted(set(d) | set(s) | set(t), key=lambda n: -max(d.get(n, [0]))):
    xs = sorted(d.get(n, []))
    print(f"{n:95s} n={len(xs):3d} max={max(xs) if xs else 0:7.1f} med={statistics.median(xs) if xs else 0:6.1f} maxSLOW={s.get(n,0):4.0f} TO={t.get(n,0)}")
