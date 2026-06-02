#!/usr/bin/env python3
"""Recreate Figure 1 of the ITC 2008 paper from the `space_consumption` example.

Two log-log panels — data causality (dynamic) and process causality (static) —
with one curve per entity population: mean encoded stamp size vs iterations.

Usage:
    # 1. produce the data (paper parameters; long-running):
    cargo run --release --example space_consumption \\
        > results/space_consumption/space.csv

    # 2. plot it (needs matplotlib: `pip install matplotlib`):
    python3 scripts/plot_space_consumption.py results/space_consumption/space.csv

The PNG and SVG are written next to the input CSV.
"""
import csv
import os
import sys
from collections import defaultdict

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt

CSV = sys.argv[1] if len(sys.argv) > 1 else "results/space_consumption/space.csv"
OUT_DIR = os.path.dirname(os.path.abspath(CSV))

# Per-population style, descending to match the paper's legend ordering.
# Colors/markers approximate the paper's gnuplot palette.
STYLES = [
    (128, "#e8000b", "+"),
    (64, "#1ac938", "x"),
    (32, "#023eff", "*"),
    (16, "#e000c8", "s"),
    (8, "#00d7d7", "s"),
    (4, "#f5e000", "o"),
]
FILLED = {8}  # markers drawn solid; the rest are open

# scenario -> (panel title, y-axis top, legend noun)
PANELS = {
    "data": ("Data Causality in a Dynamic Setting", 10000, "replicas"),
    "process": ("Process Causality in a Static Setting", 1000, "processes"),
}

# data[scenario][entities] -> list of (iteration, mean_bytes)
data = defaultdict(lambda: defaultdict(list))
with open(CSV, newline="") as f:
    for row in csv.DictReader(f):
        data[row["scenario"]][int(row["entities"])].append(
            (int(row["iteration"]), float(row["mean_bytes"]))
        )

fig, axes = plt.subplots(1, 2, figsize=(11, 4.2))

for ax, scenario in zip(axes, ("data", "process")):
    title, ytop, noun = PANELS[scenario]
    series = data[scenario]
    xmax = max(it for pts in series.values() for it, _ in pts)

    for n, color, marker in STYLES:
        if n not in series:
            continue
        pts = sorted(series[n])
        xs = [it for it, _ in pts]
        ys = [b for _, b in pts]
        ax.plot(
            xs,
            ys,
            marker=marker,
            color=color,
            markersize=5,
            markerfacecolor=color if n in FILLED else "none",
            markeredgecolor=color,
            linewidth=1.0,
            label=f"{n} {noun}",
        )

    ax.set_xscale("log")
    ax.set_yscale("log")
    ax.set_xlim(1, xmax)
    ax.set_ylim(1, ytop)
    ax.set_xlabel("Iterations")
    ax.set_ylabel("Size in bytes")
    ax.set_title(title)
    ax.grid(True, which="both", linewidth=0.3, alpha=0.4)
    ax.legend(loc="upper left", fontsize=8, framealpha=0.9)

fig.tight_layout()
for name in ("itc_space_consumption.png", "itc_space_consumption.svg"):
    out = os.path.join(OUT_DIR, name)
    fig.savefig(out, dpi=150, bbox_inches="tight")
    print("wrote", out)
