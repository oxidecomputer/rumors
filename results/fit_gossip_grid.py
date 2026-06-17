#!/usr/bin/env python3
"""Fit a numerical cost model to the `gossip_grid` benchmark results.

Reads `results/gossip_grid.csv` (produced by `extract_gossip_grid.py`) and fits
the two-regime model documented in `results/ANALYSIS.md`. Prints the fitted
coefficients, per-regime and combined fit quality, and -- for context -- the
single-regime baselines the two-regime model improves on.

The fit minimizes residuals in LOG space (so it targets *relative* error evenly
across the four decades the timings span) with a soft-L1 loss (so a handful of
noisy corner cells don't dominate). See ANALYSIS.md for why.

Requires numpy + scipy:
    python3 -m venv .venv && .venv/bin/pip install numpy scipy
    .venv/bin/python results/fit_gossip_grid.py
"""

import csv
import sys

import numpy as np
from scipy.optimize import least_squares


def load(path):
    C, D, R, T = [], [], [], []
    with open(path) as f:
        for row in csv.DictReader(f):
            C.append(int(row["common"]))
            D.append(int(row["differing"]))
            R.append(int(row["redacted"]))
            T.append(float(row["mean_ns"]))
    return (np.array(C, float), np.array(D, float),
            np.array(R, float), np.array(T, float))


# 256-ary trie depth: a node branches on one key byte (radix u8 -> 256 children),
# keys are 32-byte BLAKE3 hashes, so a leaf sits at depth ~log_256(n_leaves).
def log256(x):
    return np.log1p(x) / np.log(256)


def fit_quality(pred, T):
    """log-space R^2 and the relative-error distribution |pred/T - 1|."""
    logT = np.log(T)
    res = np.log(np.maximum(pred, 1e-9)) - logT
    r2 = 1 - np.sum(res ** 2) / np.sum((logT - logT.mean()) ** 2)
    rel = np.abs(np.expm1(res))
    return r2, rel


def fit_subset(idx, model, x0, T, lo=None, hi=None):
    logT = np.log(T)
    lo = [0.0] * len(x0) if lo is None else lo
    hi = [np.inf] * len(x0) if hi is None else hi

    def resid(p):
        return np.log(np.maximum(model(p)[idx], 1e-9)) - logT[idx]

    sol = least_squares(resid, x0, bounds=(lo, hi),
                        loss="soft_l1", f_scale=0.25, max_nfev=80000)
    _, rel = fit_quality(model(sol.x)[idx], T[idx])
    return sol.x, rel


def main(path):
    C, D, R, T = load(path)
    L = log256(C + D)                       # frontier depth term, shared by all models
    print(f"loaded {len(T)} cells from {path}\n")

    # ---- Single-regime baselines (for context; see ANALYSIS.md) -------------
    # A: pure divergence throughput + constant (the benchmark's own thesis)
    a_model = lambda p: p[0] + p[1] * (D + R)
    a_p, a_rel = fit_subset(np.arange(len(T)), a_model, [3e4, 3e3], T)
    print(f"baseline A  c0 + k*(D+R):              median rel err {np.median(a_rel)*100:5.1f}%")

    # F: structural additive model (floor + frontier descent + transfer + redaction)
    f_model = lambda p: (p[0] + p[1] * L**2
                         + D * (p[2] + p[3] * L) + R * p[4] * L)
    f_p, f_rel = fit_subset(np.arange(len(T)), f_model, [1.7e4, 9e4, 740, 2240, 286], T)
    print(f"baseline F  single-regime structural:  median rel err {np.median(f_rel)*100:5.1f}%")

    # ---- The two-regime model -----------------------------------------------
    # Regime axis is delta SPARSITY, not working-set size:
    #   s = (differing + redacted) / (common + differing)
    # s ~ 1 : dense / disjoint delta  -> transfer-bound
    # s ~ 0 : sparse delta in big tree -> locate-bound (cost ~ sqrt(D*C))
    SPARSITY_SPLIT = 0.2
    s = (D + R) / (C + D + 1.0)
    dense = np.where(s >= SPARSITY_SPLIT)[0]
    sparse = np.where(s < SPARSITY_SPLIT)[0]

    # DENSE: additive. Transfer dominates; per-leaf cost rises with trie depth.
    dense_model = lambda p: (p[0] + p[1] * L**2
                             + D * (p[2] + p[3] * L) + R * p[4] * L)
    dense_p, dense_rel = fit_subset(
        dense, dense_model, [1.7e4, 9e4, 1700, 1530, 200], T)

    # SPARSE: transfer terms vanish; the cost is the divergent/shared interface
    # walk, ~ e * D^p * C^q with p,q free (they land near 0.57 ~ sqrt(D*C)).
    sparse_model = lambda p: (p[0] + p[1] * L**2
                              + p[2] * np.power(D, p[3]) * np.power(C + 1, p[4])
                              + R * p[5] * L)
    sparse_p, sparse_rel = fit_subset(
        sparse, sparse_model, [1.3e4, 7e4, 800, 0.58, 0.57, 240], T,
        lo=[0, 0, 0, 0, 0, 0], hi=[np.inf, np.inf, np.inf, 1.5, 1.5, np.inf])

    rel = np.empty(len(T))
    rel[dense] = dense_rel
    rel[sparse] = sparse_rel

    print(f"\ntwo-regime (split sparsity s={SPARSITY_SPLIT}):")
    print(f"  combined: median {np.median(rel)*100:.1f}%  "
          f"90th {np.percentile(rel, 90)*100:.1f}%  max {rel.max()*100:.0f}%  "
          f"within 25% {(rel < .25).mean()*100:.0f}%  within 50% {(rel < .5).mean()*100:.0f}%")

    print(f"\n  DENSE  (s >= {SPARSITY_SPLIT}, n={len(dense)}, median {np.median(dense_rel)*100:.1f}%)")
    print(f"    t[ns] = c0 + c_lat*L^2 + (a_d + b_d*L)*D + b_r*L*R")
    for n, v in zip(["c0", "c_lat", "a_d", "b_d", "b_r"], dense_p):
        print(f"      {n:>5} = {v:11.2f}")

    print(f"\n  SPARSE (s <  {SPARSITY_SPLIT}, n={len(sparse)}, median {np.median(sparse_rel)*100:.1f}%)")
    print(f"    t[ns] = c0 + c_lat*L^2 + e*D^p*(1+C)^q + b_r*L*R")
    for n, v in zip(["c0", "c_lat", "e", "p", "q", "b_r"], sparse_p):
        print(f"      {n:>5} = {v:11.4g}")
    print(f"\n  (L = log256(1 + common + differing); times in nanoseconds)")


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "results/gossip_grid.csv")
