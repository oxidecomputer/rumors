# Benchmark results: optimized impl vs. reference oracle

Per-operation timing comparison of this crate's optimized implementation
(`before` — packed `BitVec` storage with a transient working form) against the
naive recursive reference (`oracle`, `src/oracle.rs`), on identical randomized
inputs. The inputs are built through the public API by the
fork-a-universe / preserve-a-subset / join-each-group recipe in
[`benches/common`](../../benches/common/mod.rs); impl and oracle are driven from
the *same* plan, so the two are structurally identical and the timings are a
like-for-like comparison on the same trees.

## Files

- `party.{png,svg}` — `Party` (id-tree) operations.
- `version.{png,svg}` — `Version` (event-tree / history) operations.
- `clock.{png,svg}` — `Clock` (full stamp) operations.
- `speedup.md` / `speedup.csv` — oracle/before median ratio at the largest tree
  size, per operation.

Each figure is a grid of log-log panels, one per operation: median time
(criterion's `median` point estimate) vs. tree size `n` (the number of forked
universe members; the joined trees grow roughly linearly with it). The shaded
band is the median's 95% confidence interval. `before` is the blue solid curve,
`oracle` the red dashed one; the badge in each panel gives the speedup at the
largest `n`.

Two panels read a different axis:

- **codec** (`encode`/`decode`) — impl-only. The oracle omits the byte codec by
  design, so these panels show the packed codec's two directions alone, with no
  oracle curve.
- **`version` k-ticks** — the working-form headline. Tree size is fixed at 64
  and the x-axis is `k`, the number of ticks applied: `before` batched (one
  unpack/repack amortized over all `k`) vs. `before` per-tick (unpack/repack
  each) vs. the oracle (re-normalizes each tick).

## How to read the speedups

The impl is **not** uniformly faster, and the plots show where the packed
representation pays off and where its unpack/repack overhead costs:

- **Big wins** come from operations where the packed form prunes whole subtrees
  cheaply or avoids redundant traversal: `clock/fork` (~15×), `party/fork`
  (~6×), and the `partial_cmp` *equal* case (`version` ~31×, `party` ~12×),
  where the single-pass compare beats a two-pass containment formulation that
  would walk the whole tree twice.
- **Losses** show up in the small, traversal-light operations where the oracle's
  plain pointer-chasing beats the bit-twiddling: `is_disjoint`, the early-bailing
  `partial_cmp` *concurrent*/*ancestor* cases, and per-event `tick` (dominated by
  unpack/repack of the whole stamp for a one-component change — which is exactly
  what `batch()` exists to amortize; see the k-ticks panel).

See [`speedup.md`](speedup.md) for the full table.

## Regenerating

```sh
# 1. Data — runs every bench target, writes target/criterion/<group>/...
#    (release profile; sweeps n = 8 .. 32768; long-running).
cargo bench -p before

# 2. Plot — needs matplotlib (`pip install matplotlib`):
python3 crates/before/scripts/plot_benchmarks.py
```

The plot script reads criterion's JSON directly, keying off each
`benchmark.json`'s own `group_id`/`function_id` (not the sanitized directory
names), and ignores any function not in the current bench suite. It takes
optional `[CRITERION_DIR] [OUT_DIR]` arguments; both default to the workspace
`target/criterion` and this directory.

All curves in the committed figures were collected in a single run on
2026-06-02.
