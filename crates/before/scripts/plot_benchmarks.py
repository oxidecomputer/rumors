#!/usr/bin/env python3
"""Assemble the criterion benchmark results into a cohesive set of comparison plots.

For every operation, the optimized implementation (`before`) is plotted against
the naive recursive reference (`oracle`) on the same randomized inputs (see
`benches/common`). One figure per type — Party, Version, Clock — with a log-log
panel per operation: median time vs. tree size `n`. The codec has no oracle
counterpart (the oracle omits it by design), so those panels show the impl's
encode/decode alone; `version/k_ticks` sweeps the tick count `k` at a fixed tree
size rather than `n`.

Data is read straight from criterion's JSON (`<group>/<function>/<value>/new/`),
keyed off each `benchmark.json`'s own `group_id`/`function_id` rather than the
sanitized directory names. Median point estimates are plotted; the shaded band
is the median's 95% confidence interval.

Usage:
    # 1. produce the data (the verification gate's release profile):
    cargo bench -p before                 # writes target/criterion/...

    # 2. plot it (needs matplotlib: `pip install matplotlib`):
    python3 scripts/plot_benchmarks.py [CRITERION_DIR] [OUT_DIR]

Defaults: CRITERION_DIR=../../target/criterion (workspace target), OUT_DIR=results/benchmarks.
PNG + SVG per family, plus a speedup summary table (speedup.md) are written to OUT_DIR.
"""
import csv
import glob
import json
import math
import os
import sys

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.ticker import EngFormatter, LogLocator, NullFormatter

HERE = os.path.dirname(os.path.abspath(__file__))
CRIT = sys.argv[1] if len(sys.argv) > 1 else os.path.join(HERE, "..", "..", "..", "target", "criterion")
OUT = sys.argv[2] if len(sys.argv) > 2 else os.path.join(HERE, "..", "results", "benchmarks")
os.makedirs(OUT, exist_ok=True)

# ── palette ──────────────────────────────────────────────────────────────────
BEFORE = "#1f77b4"   # optimized impl: blue, solid, circle
ORACLE = "#d62728"   # reference oracle: red, dashed, square
ACCENT = "#17becf"   # secondary impl series (e.g. decode): cyan
GREEN = "#2ca038"    # tertiary (e.g. unbatched): green

# ── read criterion JSON ────────────────────────────────────────────────────────
# data[group_id][function_id] = {value:int -> (median_ns, lo_ns, hi_ns)}
data = {}
for bj in glob.glob(os.path.join(CRIT, "**", "new", "benchmark.json"), recursive=True):
    meta = json.load(open(bj))
    est = json.load(open(os.path.join(os.path.dirname(bj), "estimates.json")))
    m = est["median"]
    pt = m["point_estimate"]
    ci = m["confidence_interval"]
    try:
        v = int(meta["value_str"])
    except (TypeError, ValueError):
        continue
    data.setdefault(meta["group_id"], {}).setdefault(meta["function_id"], {})[v] = (
        pt, ci["lower_bound"], ci["upper_bound"]
    )

if not data:
    sys.exit(f"no criterion data found under {CRIT!r}; run `cargo bench -p before` first")


def series(group, fn):
    """Sorted (xs, ys, los, his) in seconds for one group/function, or None if absent."""
    pts = data.get(group, {}).get(fn)
    if not pts:
        return None
    xs = sorted(pts)
    ys = [pts[x][0] / 1e9 for x in xs]
    los = [pts[x][1] / 1e9 for x in xs]
    his = [pts[x][2] / 1e9 for x in xs]
    return xs, ys, los, his


def speedup(group, before_fn, oracle_fn):
    """oracle/before median ratio at the largest shared input (>1 ⇒ impl faster)."""
    b = data.get(group, {}).get(before_fn)
    o = data.get(group, {}).get(oracle_fn)
    if not b or not o:
        return None
    common = sorted(set(b) & set(o))
    if not common:
        return None
    n = common[-1]
    return o[n][0] / b[n][0], n


# A panel: title + the curves to draw. Each curve is (function_id, label, color, marker, linestyle).
# `cmp` names the (before, oracle) pair for the speedup annotation, when applicable.
def cmp_panel(group, title, before_fn="before", oracle_fn="oracle",
              before_label="before (impl)", oracle_label="oracle (reference)"):
    return {
        "group": group, "title": title, "xlabel": "tree size  (n forked members)",
        "curves": [
            (before_fn, before_label, BEFORE, "o", "-"),
            (oracle_fn, oracle_label, ORACLE, "s", "--"),
        ],
        "cmp": (before_fn, oracle_fn),
    }


FAMILIES = {
    "party": (
        "Party — id-tree operations:  optimized impl vs. reference oracle",
        [
            cmp_panel("party/fork", "fork"),
            cmp_panel("party/join", "join"),
            cmp_panel("party/is_disjoint", "is_disjoint"),
            cmp_panel("party/partial_cmp", "partial_cmp: ancestor",
                      "before/ancestor", "oracle/ancestor"),
            cmp_panel("party/partial_cmp", "partial_cmp: equal",
                      "before/equal", "oracle/equal"),
            {
                "group": "party/codec", "title": "codec  (impl only — no oracle)",
                "xlabel": "tree size  (n forked members)",
                "curves": [
                    ("before/encode", "encode", BEFORE, "o", "-"),
                    ("before/decode", "decode", ACCENT, "^", "-"),
                ],
                "cmp": None,
            },
        ],
    ),
    "version": (
        "Version — event-tree (history) operations:  optimized impl vs. reference oracle",
        [
            cmp_panel("version/tick", "tick"),
            cmp_panel("version/merge", "merge  ( | , least-upper-bound)"),
            cmp_panel("version/partial_cmp", "partial_cmp: concurrent",
                      "before/concurrent", "oracle/concurrent"),
            cmp_panel("version/partial_cmp", "partial_cmp: ordered",
                      "before/ordered", "oracle/ordered"),
            cmp_panel("version/partial_cmp", "partial_cmp: equal",
                      "before/equal", "oracle/equal"),
            {
                "group": "version/codec", "title": "codec  (impl only — no oracle)",
                "xlabel": "tree size  (n forked members)",
                "curves": [
                    ("before/encode", "encode", BEFORE, "o", "-"),
                    ("before/decode", "decode", ACCENT, "^", "-"),
                ],
                "cmp": None,
            },
            {
                "group": "version/k_ticks", "title": "k ticks  (working form, tree = 64)",
                "xlabel": "k  (ticks applied)",
                "curves": [
                    ("before/batched", "before: batch()", BEFORE, "o", "-"),
                    ("before/unbatched", "before: per-tick", GREEN, "s", "--"),
                    ("oracle", "oracle", ORACLE, "^", ":"),
                ],
                "cmp": None,
            },
        ],
    ),
    "clock": (
        "Clock — full stamp (id + event) operations:  optimized impl vs. reference oracle",
        [
            cmp_panel("clock/tick", "tick"),
            cmp_panel("clock/fork", "fork"),
            cmp_panel("clock/join", "join"),
            cmp_panel("clock/sync", "sync"),
            cmp_panel("clock/send", "send"),
            cmp_panel("clock/receive", "receive"),
            {
                "group": "clock/codec", "title": "codec  (impl only — no oracle)",
                "xlabel": "tree size  (n forked members)",
                "curves": [
                    ("before/encode", "encode", BEFORE, "o", "-"),
                    ("before/decode", "decode", ACCENT, "^", "-"),
                ],
                "cmp": None,
            },
        ],
    ),
}


def render(family, title, panels):
    ncols = 3
    nrows = math.ceil(len(panels) / ncols)
    fig, axes = plt.subplots(nrows, ncols, figsize=(5.0 * ncols, 3.7 * nrows),
                             squeeze=False)
    flat = [ax for row in axes for ax in row]

    for ax, panel in zip(flat, panels):
        any_drawn = False
        all_x = set()
        ylo, yhi = math.inf, 0.0
        for fn, label, color, marker, ls in panel["curves"]:
            s = series(panel["group"], fn)
            if s is None:
                continue
            xs, ys, los, his = s
            all_x.update(xs)
            ylo, yhi = min(ylo, *los), max(yhi, *his)
            ax.plot(xs, ys, marker=marker, color=color, linestyle=ls,
                    markersize=5, linewidth=1.4, label=label,
                    markerfacecolor=color, markeredgecolor=color)
            ax.fill_between(xs, los, his, color=color, alpha=0.15, linewidth=0)
            any_drawn = True
        if not any_drawn:
            ax.set_visible(False)
            continue

        ax.set_xscale("log", base=2)
        ax.set_yscale("log")
        ax.set_title(panel["title"], fontsize=10.5)
        ax.set_xlabel(panel["xlabel"], fontsize=8)
        ax.set_ylabel("median time", fontsize=8)
        # Place labeled major ticks even on sub-decade ranges (else matplotlib
        # falls back to raw scientific notation instead of the EngFormatter's units).
        decades = math.log10(yhi / ylo) if ylo > 0 and yhi > ylo else 0
        subs = (1.0,) if decades >= 2 else (1.0, 2.0, 3.0, 5.0)
        ax.yaxis.set_major_locator(LogLocator(base=10, subs=subs, numticks=20))
        ax.yaxis.set_major_formatter(EngFormatter(unit="s", places=0))
        ax.yaxis.set_minor_formatter(NullFormatter())
        xs_sorted = sorted(all_x)
        ax.set_xticks(xs_sorted)
        ax.set_xticklabels([str(x) for x in xs_sorted], fontsize=7, rotation=0)
        ax.tick_params(axis="y", labelsize=7)
        ax.grid(True, which="both", linewidth=0.3, alpha=0.4)
        ax.legend(loc="upper left", fontsize=7.5, framealpha=0.9)

        if panel["cmp"]:
            su = speedup(panel["group"], *panel["cmp"])
            if su:
                ratio, n = su
                ax.text(0.97, 0.05, f"{ratio:.1f}× faster\n@ n={n}",
                        transform=ax.transAxes, ha="right", va="bottom",
                        fontsize=8, color="#333",
                        bbox=dict(boxstyle="round,pad=0.3", fc="#fffbe6",
                                  ec="#d0c060", lw=0.6))

    for ax in flat[len(panels):]:
        ax.set_visible(False)

    fig.suptitle(title, fontsize=13, y=0.995)
    fig.tight_layout(rect=(0, 0, 1, 0.98))
    for ext in ("png", "svg"):
        out = os.path.join(OUT, f"{family}.{ext}")
        fig.savefig(out, dpi=150, bbox_inches="tight")
        print("wrote", out)
    plt.close(fig)


# ── speedup summary table ───────────────────────────────────────────────────────
def write_speedup_table():
    rows = []  # (type, operation, before@max, oracle@max, n, speedup)
    for family, (_, panels) in FAMILIES.items():
        for panel in panels:
            if not panel["cmp"]:
                continue
            bfn, ofn = panel["cmp"]
            b = data.get(panel["group"], {}).get(bfn)
            o = data.get(panel["group"], {}).get(ofn)
            if not b or not o:
                continue
            common = sorted(set(b) & set(o))
            if not common:
                continue
            n = common[-1]
            rows.append((family.capitalize(), panel["title"],
                         b[n][0], o[n][0], n, o[n][0] / b[n][0]))

    def fmt_ns(ns):
        for unit, scale in (("ms", 1e6), ("µs", 1e3), ("ns", 1.0)):
            if ns >= scale:
                return f"{ns / scale:.2f} {unit}"
        return f"{ns:.2f} ns"

    md = os.path.join(OUT, "speedup.md")
    with open(md, "w") as f:
        f.write("# Benchmark speedups: optimized impl vs. reference oracle\n\n")
        f.write("Median time at the largest tree size benchmarked, and the "
                "oracle/before ratio (>1 ⇒ the impl is faster).\n\n")
        f.write("| Type | Operation | n | before (impl) | oracle (ref) | speedup |\n")
        f.write("|------|-----------|--:|--------------:|-------------:|--------:|\n")
        for ty, op, bns, ons, n, su in rows:
            f.write(f"| {ty} | {op} | {n} | {fmt_ns(bns)} | {fmt_ns(ons)} | {su:.1f}× |\n")
    print("wrote", md)

    csvp = os.path.join(OUT, "speedup.csv")
    with open(csvp, "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["type", "operation", "n", "before_ns", "oracle_ns", "speedup"])
        for ty, op, bns, ons, n, su in rows:
            w.writerow([ty, op, n, f"{bns:.3f}", f"{ons:.3f}", f"{su:.4f}"])
    print("wrote", csvp)


for family, (title, panels) in FAMILIES.items():
    render(family, title, panels)
write_speedup_table()
