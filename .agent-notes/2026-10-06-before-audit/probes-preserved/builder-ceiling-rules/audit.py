#!/usr/bin/env python3
"""Audit every ceilings.rs constant with a measured rule against one acceptance board log.

Usage: audit.py <board.log>

Each matrix row renders the larger per-unit reading of its window's two samples,
so a floor rule over samples is bracketed: rows whose smaller sample clears the
floor (exact) and rows whose larger sample clears it (an upper bound, since the
row's maximum may come from the sub-floor sample).
"""
import math, re, sys
from collections import defaultdict

FLOOR = 2 * 1024  # twice HEAP_INTERCEPT_BYTES
ROW = re.compile(
    r"^(GREEN|RED)\s+(\S+)\s+(\S+)\s+(\d+)->(\d+)\s+B\s+"
    r"heap\[e(.{5}) +([\d.]+)/([Bu])\]\s+"
    r"(scan\[(?:e(.{5}) +([\d.]+)/([Bu])|\s+off\s+)\])\s+"
    r"(touch\[(?:e(.{5}) +([\d.]+)/([Bu])|\s+off\s+)\])"
    r"(.*)$"
)

def models(tail):
    m = re.search(r"model\[([^\]]*)\]", tail)
    out = {}
    if not m:
        return out
    for part in m.group(1).split("; "):
        cur, rest = part.split(" ", 1)
        d = {}
        for item in rest.split(", "):
            if item.startswith("ceiling "):
                d["ceiling"] = float(item.split()[1])
            else:
                k, v = item.split(" ", 1)
                a, b = v.split("->")
                d[k] = (int(a), int(b))
        out[cur] = d
    return out

def parse(path):
    rows, section, red = [], None, 0
    for line in open(path):
        if line.startswith("=== ladder base"):
            section = "base"
        elif line.startswith("=== ladder top"):
            section = "top"
        elif line.startswith("=== small"):
            section = "small"
        m = ROW.match(line)
        if not m or section is None:
            continue
        g = m.groups()
        red += g[0] == "RED"
        n1, n2 = int(g[3]), int(g[4])
        md = models(g[16])
        def units(cur):
            d = md.get(cur, {})
            if "constant" in d:
                return d["constant"]
            if "units" in d:
                return d["units"]
            return (n1, n2)
        rows.append(dict(
            verdict=g[0], op=g[1], family=g[2], section=section, n=(n1, n2),
            heap=float(g[6]), scan=float(g[10]) if g[10] else None,
            touch=float(g[14]) if g[14] else None, models=md,
            heap_units=units("heap"), scan_units=units("scan"), touch_units=units("touch"),
        ))
    return rows, red

def ceil_rule(reading, headroom=1.25):
    # Readings render at one decimal: the true value lies within +-0.05.
    lo, hi = math.ceil((reading - 0.05) * headroom - 1e-9), math.ceil((reading + 0.05) * headroom - 1e-9)
    mid = math.ceil(reading * headroom - 1e-9)
    return mid, (lo != hi)

def best(rows, key):
    rows = [r for r in rows if r[key] is not None]
    if not rows:
        return None
    r = max(rows, key=lambda r: r[key])
    return r[key], r["op"], r["family"], r["section"], "%d->%d" % r[f"{key}_units"]

def report(name, value, rows, key, brackets):
    print(f"## {name} = {value}  ({len(rows)} rows)")
    for label, pred in brackets:
        b = best([r for r in rows if pred(r)], key)
        if b is None:
            print(f"   {label:<34}: no rows"); continue
        rule, ambiguous = ceil_rule(b[0])
        print(f"   {label:<34}: max {b[0]:>7.1f} at {b[1]} x {b[2]} [{b[3]} {b[4]}] -> rule {rule}"
              + ("  (AMBIGUOUS at 1-decimal rendering)" if ambiguous else ""))

def main():
    rows, red = parse(sys.argv[1])
    print(f"parsed {len(rows)} rows; red rows {red}; sections",
          {s: sum(r['section'] == s for r in rows) for s in ('base', 'top', 'small')})
    heap_by_ceiling = defaultdict(list)
    for r in rows:
        c = r["models"].get("heap", {}).get("ceiling")
        heap_by_ceiling[c].append(r)
    for c, rs in sorted(heap_by_ceiling.items(), key=lambda kv: (kv[0] is None, kv[0] or 0)):
        ops = sorted({r["op"] for r in rs})
        print(f"heap ceiling {c}: {len(rs)} rows, ops {ops if c is not None else str(len(ops)) + ' ops'}")
    print()
    floor_rule = [
        ("all samples", lambda r: True),
        ("ladder (base+top)", lambda r: r["section"] != "small"),
        ("both samples >= 2048 units", lambda r: r["heap_units"][0] >= FLOOR),
        ("larger sample >= 2048 units", lambda r: r["heap_units"][1] >= FLOOR),
        ("top scale only", lambda r: r["section"] == "top"),
    ]
    names = {
        4.0: "DESERIALIZE_HEAP_BYTES_PER_INPUT_BYTE",
        5.0: "RANK_DESERIALIZE_HEAP_BYTES_PER_INPUT_BYTE",
        9.0: "RANKED_DESERIALIZE_HEAP_BYTES_PER_INPUT_BYTE",
        152.0: "QUERY_EVALUATION_HEAP_BYTES_PER_INPUT_BYTE",
        3.0: "COMB_SCATTER_PROJECTION_HEAP_BYTES_PER_IO_BYTE",
        None: "MAX_HEAP_BYTES_PER_INPUT_BYTE (global heap)",
    }
    for c, rs in heap_by_ceiling.items():
        report(names.get(c, f"heap ceiling {c}"), c if c is not None else 20.0, rs, "heap", floor_rule)
    print()
    # Scan: fold rows carry the scaled model ceiling; every other scan row is
    # judged against the global scan ceiling or an inline ops.rs ceiling.
    scan_by_ceiling = defaultdict(list)
    for r in rows:
        if r["scan"] is not None:
            scan_by_ceiling[r["models"].get("scan", {}).get("ceiling")].append(r)
    for c, rs in scan_by_ceiling.items():
        label = {17.0: "FOLD_SCAN_BITS_PER_INPUT_BYTE_PER_LEVEL", None: "MAX_SCAN_BITS_PER_INPUT_BYTE (global scan)",
                 10.0: "PARTY_COMPARISON_SCAN_BITS_PER_INPUT_BYTE (ops.rs)"}.get(c, f"scan ceiling {c}")
        report(label, c if c is not None else 96.0, rs, "scan", floor_rule[:2])
    print()
    touch_by_ceiling = defaultdict(list)
    for r in rows:
        if r["touch"] is not None:
            touch_by_ceiling[r["models"].get("touch", {}).get("ceiling")].append(r)
    for c, rs in touch_by_ceiling.items():
        label = {None: "MAX_TOUCHES_PER_INPUT_BYTE (global touch)",
                 18.0: "MASKED_HOLE_TOUCH_CEILING (ops.rs)"}.get(c, f"touch ceiling {c}")
        report(label, c if c is not None else 22.0, rs, "touch", floor_rule[:2])

if __name__ == "__main__":
    main()
