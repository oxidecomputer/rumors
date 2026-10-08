"""Collects nextest outcome lines for named tests across log files, deduplicated by exact line, and prints the PASS distribution and the source of the extremes."""
import sys, re, pathlib, statistics, collections
root = pathlib.Path(sys.argv[1]); names = sys.argv[2].split(",")
pat = re.compile(r"^\s*(PASS|FAIL|TIMEOUT|LEAK|LEAK-FAIL|SIGKILL|SIGSEGV|ABORT)\s+\[\s*([0-9.]+)s\]\s+\(([^)]*)\)\s+(\S+)\s+(\S+)\s*$")
seen = {}; outcomes = collections.Counter()
for p in root.rglob("*"):
    if not p.is_file() or "reviewer-nextest" in str(p): continue
    try: text = p.read_text(errors="replace")
    except Exception: continue
    for line in text.splitlines():
        m = pat.match(line)
        if not m: continue
        k, v, pos, b, n = m.groups()
        if n not in names: continue
        key = line.strip()
        seen.setdefault(key, (k, float(v), pos, b, n, str(p)))
for n in names:
    rows = [r for r in seen.values() if r[4] == n]
    ok = sorted(r[1] for r in rows if r[0] in ("PASS", "LEAK"))
    other = collections.Counter(r[0] for r in rows if r[0] not in ("PASS", "LEAK"))
    if not ok: print(n, "no passes", dict(other)); continue
    q = statistics.quantiles(ok, n=10) if len(ok) >= 10 else []
    print(f"{n}: n={len(ok)} min={ok[0]:.1f} med={statistics.median(ok):.1f} p90={q[-1] if q else float('nan'):.1f} max={ok[-1]:.1f} max/med={ok[-1]/statistics.median(ok):.1f} non-pass={dict(other)}")
    for r in sorted(rows, key=lambda r: -r[1])[:4]: print("   ", r[0], r[1], r[2], r[5].replace(str(root)+"/", ""))
