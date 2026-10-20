<!-- CAVEAT LECTOR: written by Claude (Opus 5.5), agent scope-board-wasm-fuel, for the before audit. These are candidate readings, not root-caused defects; they need the measures lane (L4). -->

# Candidates: superlinear fuel in rank operations, invisible to touch and scan

Wasm fuel is one of the audit's accepted deterministic counters (`common.md`,
Performance). Two readings grow faster than linear in fuel along a 64×
ladder, while heap, scan, and touch per byte stay exactly flat. I measured
them but did not root-cause them, which is outside a scoping study, so they
are candidates for the lane that owns `Rank`.

## How they were measured

- Spike guest on `explore/board-in-wasm` at `3ee40592a` (before the
  `min_ticks` merge), built in `crates/before/board-wasm` with `cargo build
  --release --target wasm32-unknown-unknown -p board-wasm-guest`; harness
  built with `cargo build --release -p board-wasm-harness`.
- Command, on the Mac (fuel does not depend on the host):
  `board-wasm-harness cells <guest.wasm> 1,2,4,8,16,32 rank_sum dense-suffix
  version_rank freeze-parade`
- Each scale runs the board's own cell (`emit_cells`) at the scaled size and
  its double in a fresh guest. Logs: `rank-ladder.log` and `rank-ladder-2.log`
  in this directory.
- Fuel in this mode carries a little allocator history between a cell's two
  samples; the effect is about 0.1% here, far below the trends below.

## Candidate 1: `rank_sum` (documented `O(k + n)`)

Documentation: `impl Sum<Rank> for Rank`, `crates/before/src/rank.rs:988`
on `main`: "For `k` ranks whose binary widths sum to `n`, `O(k + n)` time".

| Denominator (bytes) | Fuel per byte | Touch per byte |
|---|---|---|
| 127,452 | 779 | 0.974 |
| 254,748 | 903 | 0.974 |
| 509,340 | 965 | 0.974 |
| 1,018,524 | 1,128 | 0.974 |
| 2,036,893 | 1,190 | 0.974 |
| 4,073,629 | 1,314 | 0.974 |
| 8,147,101 | 1,230 | 0.974 |

Local slopes between sizes are 1.21, 1.10, 1.23, 1.08, 1.14, and 0.91,
averaging about 1.11 across the 64× range. `wide-arming` behaves alike. The
growth is not monotone, so it may come from allocator or `num-bigint`
behavior at particular sizes rather than from a superlinear algorithm.
Deciding which needs a look inside `Rank::sum_iter` (the accumulator ingest
and the final normalization) with fuel attributed by function. Wasmtime's
guest profiler can attribute it.

## Candidate 2: rank operations on `freeze-parade` (documented `O(M(n))`)

Documentation: `Version::rank` (`crates/before/src/version.rs:367` on
`main`), `version_distance` (`:435`), `version_lag` (`:470`), the ranked
encodings (`:1012`, `:1042`), and `ranked.rs:365`: "`O(M(n))` time ...
`M(n)` is the cost of multiplying `n`-bit integers".

| Denominator (bytes) | Fuel per byte | Touch per byte | Scan per byte |
|---|---|---|---|
| 50,124 | 997 | 0.790 | 16.0 |
| 100,173 | 1,081 | 0.790 | 16.0 |
| 200,269 | 1,251 | 0.790 | 16.0 |
| 400,461 | 1,587 | 0.790 | 16.0 |
| 800,845 | 2,256 | 0.790 | 16.0 |
| 1,601,614 | 3,598 | 0.790 | 16.0 |
| 3,203,150 | 6,278 | 0.790 | 16.0 |

Local slopes rise steadily (1.12, 1.21, 1.34, 1.51, 1.67, 1.80). `num-bigint`
0.4.8 multiplies large balanced operands by Toom-3, about `n^1.465`, so a
local slope of 1.80 at 3.2 MB suggests a multiplication pattern costlier than
one `M(n)`. Possible sources are unbalanced products done schoolbook, or a
sum of products whose widths do not shrink. The candidate site is the
`Integrator`'s products (`add_product` of a scaled width by a parked
height, `crates/before/src/version/measure/integral.rs`). Whether this breaches
the documented bound depends on what `M(n)` the documentation means: the
backend's actual multiplication, or schoolbook `n²`. That question is itself
worth a ruling.

The same family drives the fuel exponent of `ranked_encode`,
`ranked_encode_rank`, `ranked_cmp`, `ranked_decode`,
`ranked_serde_deserialize`, `version_lag`, and `version_distance` above 1.15
on the board's own ladder. `version_rank` on `dense-suffix` stays near-linear
until the top of the 64× ladder, where its local slope reaches 1.22.

## Not candidates

`count_parse` (fuel exponent about 2.0) and `count_display` (about 1.6) on
every wide family sit within their documented bounds, `O(n²)` and
"superlinear but subquadratic" (`count.rs:262`, `count.rs:282`). A fuel
column would need declared models for them.
