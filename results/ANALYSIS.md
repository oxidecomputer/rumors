# A cost model for `gossip` reconciliation

*Fitted from the `gossip_grid` benchmark, 2026-06-04, on an Apple M4 Max.*

This document explains a numerical model of how long one `Known::gossip`
reconciliation takes as a function of the divergence between two peers. It
assumes you already know how the code works — the mirror protocol in
`src/tree/traverse/mirror/`, the `join` traversal, the divergence grid in
`benches/support/grid.rs` — but **not** the conclusions below. The point is to
show how the timings fall out of the algorithm, and to flag one result that is
*not* what you'd guess from the throughput accounting.

The artifacts here:

| file | what |
|------|------|
| `gossip_grid.csv` | the fitted data: one row per grid cell, `mean_ns` from Criterion |
| `extract_gossip_grid.py` | regenerates the CSV from `target/criterion/gossip_grid/` |
| `fit_gossip_grid.py` | fits the model below and prints coefficients + fit quality |

Reproduce:

```sh
cargo bench --bench gossip_grid          # ~tens of minutes; writes target/criterion/
python3 results/extract_gossip_grid.py   # -> results/gossip_grid.csv
python3 -m venv .venv && .venv/bin/pip install numpy scipy
.venv/bin/python results/fit_gossip_grid.py
```

---

## TL;DR — the model

Reconciliation time splits into **two regimes** by how *sparse* the delta is
relative to the shared tree. Let

- `C` = `common`, `D` = `differing`, `R` = `redacted` (the grid axes),
- `L = log₂₅₆(1 + C + D)` — the **256-ary trie depth** (a node branches on one
  key byte; keys are 32-byte BLAKE3 hashes, so a leaf sits at depth ~`log₂₅₆(n)`),
- `s = (D + R) / (C + D)` — the **sparsity** of the delta.

**Dense delta (`s ≥ 0.2`), transfer-bound — median error 8%:**

```
t[ns] = 28 800 + 61 600·L²  +  (1350 + 1476·L)·D  +  3964·L·R
```

**Sparse delta (`s < 0.2`), locate-bound — median error 11%:**

```
t[ns] = 19 700 + 48 300·L²  +  821·(D^0.58 · C^0.57)  +  184·L·R
```

Combined: **median 9% relative error, 97% of cells within 50%.** Times are
nanoseconds for one full round-trip reconciliation.

The headline surprise: in the sparse regime the cost of `D` differing messages
is **not** proportional to `D` — it's proportional to roughly **√(D·C)**, the
size of the *interface* between the divergent and shared structure. A small
delta against a large shared tree is far more expensive than the same delta
against a small one.

---

## 1. The data

`benches/gossip_grid.rs` sweeps the `(common, differing, redacted)` cube from
`grid.rs`, reconciling two forked peers over a simulated pipe wire once per
cell, with both peers' lazy memos pre-warmed (`warm_caches`) so the timed body
measures steady-state protocol work, not first-touch memoization. We take each
cell's **mean** estimate (`estimates.json`) as `t`, and charge nothing else —
147 valid cells, spanning `t` from ~32 µs to ~575 ms (four decades).

Because the response spans four decades, every fit below minimizes residuals in
**log space** (equal weight to *relative* error at all scales) with a soft-L1
loss (so a few noisy 10-sample corner cells don't dominate). A plain
least-squares fit in linear space would see only the 100 ms cells.

---

## 2. Reading the cost off the protocol

Before fitting anything, here's what the mirror protocol (`protocol.rs`,
`local.rs`) says the cost *should* be made of.

### 2a. A fixed handshake floor

Every session pays the `Connect`/`Accept` handshake plus the
initiate/open/close/complete framing — a constant number of messages, each a
synchronous round-trip across the OS pipe with a thread hand-off
(`Wire::round_trip`). That's a floor independent of tree contents. The data
confirms it: the cheapest cells sit at ~32 µs and barely move along the small
end of every axis. → the constant `c0` (~20–29 µs).

### 2b. Descent to the disjoint frontier — *not* a fixed 16 rounds

`protocol.rs` descends **two trie levels per `Exchange` round**, and the
`define_peer!` macro hardcodes 14/15 exchange levels. It is tempting to read
that as "~16 round-trips per session, always." **It isn't.** `exchange` returns
`Step::Done` as soon as there's "nothing left to ask about and nothing left in
dispute" — i.e. the moment the zipper reaches the **disjoint frontier**, the
depth at which the divergent subtrees have separated into one-sided pieces that
ship whole. The 14/15 levels are the *theoretical maximum* (a full 32-byte-key
descent), reached only if leaves stay co-resident all the way down.

For random keys, divergent leaves separate from the bulk at depth
~`log₂₅₆(C + D)`. With ≤ 10⁵ leaves that is only **2–3 levels deep** — about
1–3 descent rounds, nowhere near 16. So the descent cost rides on `L`, not on a
constant. Empirically it's slightly super-linear in `L` (best captured by `L²`),
consistent with *rounds (∝ depth) × per-round boundary work (∝ node fan-out,
which grows toward the root in a denser tree)*. → the `c_lat·L²` term.

### 2c. Transferring what the peer gains

Each differing message the peer learns travels in the `Exchange.providing`
channel (a whole subtree) and is absorbed into the merged tree. Absorbing a leaf
re-hashes up its path of depth ~`L`, so the per-leaf cost is a base plus a
per-trie-level term. → `(a_d + b_d·L)·D`.

### 2d. Honoring deletions is cheaper

A redaction is the *absence* of a subtree, honored by the version-vector filter
(`filter_into` → `Unknown::unknown`): the peer learns "this is gone" by version
comparison, with no `providing` payload to serialize or insert. So redaction
costs less per element than a differing message. The data bears this out — in
the dense regime `b_r` and `b_d·L` are within a small factor; the gap is what
you'd expect from "drop by version" vs "ship and rehash." → `b_r·L·R`.

That's a clean four-term additive model. It fits to **~33% median error** and
caps there. The rest of this document is about *why it caps*, and what fixes it.

---

## 3. Why a single additive model isn't enough

The additive model treats every cell as floor + descent + transfer + redaction.
It systematically mispredicts one corner by up to **13×**: large `common`, small
`differing`. Concretely:

| cell | actual |
|------|--------|
| `c=0,      d=100` | 0.34 ms |
| `c=100000, d=1`   | 0.32 ms |
| `c=100000, d=100` | **13.2 ms** |

The first two are cheap; an additive model predicts their sum (~0.7 ms) for the
third. Instead it's 13 ms — **super-additive**. There is an interaction between
`common` and `differing` the additive model has no term for.

Mapping the full interaction surface (`ratio = T(C,d) / T(0,d)`, how much a
shared prefix inflates a fixed delta) shows a **ridge**: the inflation peaks at
~38× around `c=10⁵, d=10–100`, then *fades back to ~1×* as `d → 10⁵`. So the
penalty is worst when the delta is **small relative to the shared tree**, and
vanishes when the delta is large. That is the signature of a sparsity-dependent
cost, not a size-dependent one.

---

## 4. The regime axis is *sparsity*, not working-set size

Two hypotheses for the ridge:

1. **Cache.** The working set outgrows a cache level; accesses become
   memory-latency-bound. This machine (M4 Max) has L1d 128 KB/core, **L2 16 MB**
   per 6-core performance cluster, and a system-level cache (~32 MB). At a rough
   ~150 B per live leaf, a 10⁵-leaf tree is ~15 MB — right at the L2 edge, which
   is exactly where the ridge peaks.

2. **Algorithmic.** A sparse delta forces work proportional to the *interface*
   between the divergent paths and the shared structure — the cloned-but-
   pointer-distinct upper nodes that the `ptr_eq`-before-hash short-circuit
   (commit `1389732`) can no longer prune once both peers have inserted into the
   same shared tree.

Two tests separate them:

- **Fix the divergence (`d=100`) and grow `C`.** A pure cache effect (fixed
  access count at higher latency) would *plateau* once you're past the cache.
  Instead the penalty grows ~`C^0.72` — the *number* of touches grows with the
  shared-tree size. Algorithmic.
- **Re-cut the regimes.** Splitting the grid by **working-set size** (`C+D`)
  leaves the "cold" side unfittable (it mixes sparse, ridge, and transfer-bound
  cells; median stays ~35%), and the best split lands at ~1.5 MB — an order of
  magnitude *below* L2, with only 6 of 147 cells ever reaching L2 capacity.
  Splitting by **sparsity** `s = (D+R)/(C+D)` instead yields two clean regimes
  (dense 8%, combined 9%).

**Verdict.** The blow-up is **algorithmic**: it's the divergent/shared interface
cost, and it grows smoothly with `C` long before the tree spills L2. The L2
fingerprint is real but **secondary** — cache amplifies the interface walk near
the 16 MB boundary; it does not cause the ridge. If you want the model to
predict another machine, the sparsity terms port directly; only the per-access
constants (`c_lat`, `b_d`, `e`) would re-scale with memory latency.

---

## 5. The two regimes

### Dense (`s ≥ 0.2`): transfer-bound

When the delta is a large fraction of the tree (the disjoint corner `c=0`, or
any cell where `D+R` is comparable to `C`), there's no meaningful shared
interface to walk: nearly everything diverges and ships. The additive model is
exactly right here and fits to **8%**:

```
t[ns] = 28 800 + 61 600·L²  +  (1350 + 1476·L)·D  +  3964·L·R
```

### Sparse (`s < 0.2`): locate-bound

When the delta is a thin sprinkle over a large shared tree, two things change:

- The **transfer terms vanish.** Fitting the additive form here drives `a_d` and
  `b_d` to **zero** — moving a handful of leaves is free relative to everything
  else.
- The cost becomes the **interface walk**, and it fits a power law in *both*
  axes with near-symmetric exponents:

```
t[ns] = 19 700 + 48 300·L²  +  821·(D^0.58 · C^0.57)  +  184·L·R
```

`D^0.58 · C^0.57 ≈ (D·C)^0.57 ≈ √(D·C)`. The reconciliation cost of a sparse
delta scales as roughly the **geometric mean of delta size and shared-tree
size** — the size of the boundary between the bit that changed and the bit that
didn't. (Fixing the exponents to exactly ½ — a clean `√(D·C)` — costs accuracy:
combined error rises from 9% to 13%, so the data genuinely prefers ~0.57.)

This is why `c=10⁵,d=100` is 13 ms while `c=0,d=100` is 0.34 ms: same 100 leaves
learned, but locating them along the interface of a 10⁵-leaf tree is the work.

---

## 6. Coefficients, and what each one means

| coeff | dense | sparse | reading |
|-------|-------|--------|---------|
| `c0` | 28.8 µs | 19.7 µs | handshake + thread ping-pong + framing (§2a) |
| `c_lat` | 61.6 µs/level² | 48.3 µs/level² | descend to the disjoint frontier (§2b) |
| `a_d` | 1350 ns/leaf | — | base transfer cost per differing leaf (§2c) |
| `b_d` | 1476 ns/leaf/level | — | rehash up a leaf's path on insert (§2c) |
| `e`, `p`, `q` | — | 821, 0.58, 0.57 | interface walk ≈ √(D·C) (§5) |
| `b_r` | 3964 ns/level | 184 ns/level | honor deletions by version filter (§2d) |

---

## 7. Residual risks and gaps

- **Ultra-sparse corner.** Cells with `divergence = 1` (a single differing leaf)
  are the worst-fit points (±70–145%): a continuous power law can't track the
  discrete `D=1` floor. If you care about that corner, model it separately.
- **The split is empirical.** `s = 0.2` was chosen to minimize combined error;
  cells near it (`c=10⁵,d=10⁴`, `s≈0.09`) are the hardest in either regime. A
  smooth blend across the boundary would remove the kink but add parameters.
- **`b_r` (dense) is weakly constrained.** Redaction-heavy *dense* cells are
  rare in the grid, so dense `b_r = 3964` rests on few points; trust the sparse
  `b_r = 184` more.
- **The √(D·C) mechanism is empirical, not traced.** It's almost certainly the
  divergent/shared interface walk in `local.rs`, but I have not pinned it to
  specific lines or ruled out a B-tree-chunk contribution from `imbl::OrdMap`.
  Confirming it would want an op-count instrumentation pass (count node touches
  per cell) or hardware cache-miss counters on the near-L2 cells.
- **Machine-specific constants.** The `L²` and √(D·C) *shapes* are algorithmic
  and portable; the per-touch *constants* encode this M4 Max's memory latency
  and will differ elsewhere.
- **Constants predate the 16-byte Merkle hash** *(noted 2026-06-11)*. The grid
  was benchmarked against 32-byte branch hashes (33-byte child records); the
  `merkle-width-16` branch halves the hash atoms, shrinking branch preimages
  and the `uncertain` frames. Expect `b_d`, `c_lat`, and the (already weakly
  constrained) dense `b_r` to drift down somewhat; the `L²` and √(D·C) shapes
  are unaffected. Re-run the grid and re-fit before quoting these constants
  against the new code.
