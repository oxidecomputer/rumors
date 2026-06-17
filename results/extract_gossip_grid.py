#!/usr/bin/env python3
"""Extract the `gossip_grid` Criterion results into a flat CSV.

Criterion writes one directory per benchmark id under
`target/criterion/gossip_grid/<id>/`, where `<id>` is the grid cell's parameter
string (see `Cell::id` in `benches/support/grid.rs`):

    common=<C>,differing=<D>,redacted=<R>

Each cell holds:
  - `new/estimates.json`  -- the statistical estimates; we take the mean's
                             `point_estimate` (nanoseconds per reconciliation).
  - `new/benchmark.json`  -- metadata, including the `Throughput::Elements`
                             value, which is the cell's divergence (D + R).

This script walks every cell and emits `results/gossip_grid.csv` with one row
per cell. That CSV is the sole input to `fit_gossip_grid.py`; re-run this after
re-running the benchmark to refresh the data.

Usage:
    python3 results/extract_gossip_grid.py            # repo root
    python3 results/extract_gossip_grid.py <crit_dir> <out_csv>
"""

import csv
import json
import os
import re
import sys

CELL_RE = re.compile(r"common=(\d+),differing=(\d+),redacted=(\d+)")


def extract(crit_dir, out_csv):
    grid_dir = os.path.join(crit_dir, "gossip_grid")
    rows = []
    for name in sorted(os.listdir(grid_dir)):
        cell = os.path.join(grid_dir, name)
        est = os.path.join(cell, "new", "estimates.json")
        bench = os.path.join(cell, "new", "benchmark.json")
        if not (os.path.exists(est) and os.path.exists(bench)):
            continue
        m = CELL_RE.match(name)
        if not m:
            continue
        common, differing, redacted = map(int, m.groups())
        mean_ns = json.load(open(est))["mean"]["point_estimate"]
        throughput = json.load(open(bench)).get("throughput", {}) or {}
        divergence = throughput.get("Elements")
        rows.append((common, differing, redacted, mean_ns, divergence))

    with open(out_csv, "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["common", "differing", "redacted", "mean_ns", "divergence_elems"])
        w.writerows(rows)
    print(f"wrote {len(rows)} cells to {out_csv}")


if __name__ == "__main__":
    crit = sys.argv[1] if len(sys.argv) > 1 else "target/criterion"
    out = sys.argv[2] if len(sys.argv) > 2 else "results/gossip_grid.csv"
    extract(crit, out)
