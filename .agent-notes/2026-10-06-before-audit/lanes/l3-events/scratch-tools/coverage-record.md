# L3 coverage record (events: tick, ticks, Clock wrappers, min_ticks)

Base `58285ca5` (tree identical to `455e97de` outside `.agent-notes/`). Explore branch
`explore/l3-events`: `30379f8d`, `01a24058`, `1a8601b3`, `bab0a381`, `873f3999` (all signed).
Every run below executed on ox-east-1 through
`on-illumos.sh /Users/oxide/src/rumors-audit-l3-events 'unset CARGO_TARGET_DIR; …'`; logs are
in the auditor-l3 scratch directory.

## Contract clauses: what establishes each, and how far

| Clause | Instrument(s) | Status |
|---|---|---|
| `tick` is the paper's event: fill, else grow with lexicographic (expansions, depth) cost, ties right | Oracle transcription checked by reading against itc2008.md 5.3.4 (faithful; the lexicographic cost equals the paper's `N`-weighted cost because `N` exceeds every depth). Committed `assert_tick` suites plus co-generated probes (`check_one`: flag, fill output, event bytes) | holds on everything sampled; route tie-break mutations M1, M2 are caught, so ties are reached |
| strict domination; region locality | laws `tick_strictly_advances`, `tick_only_inflates_the_region`; probes on co-generated pairs and deep spines | holds |
| `ticks(k)` equals k ticks, any width | committed `ticks_*` suites; probes: k = 1..4 against iterated and oracle, composition at {1, 2^64-1, 2^64+3, 2^200-1}, deep spines with 2^70+5 | holds; argument in `raise.rs` doc checked by reading (fill idempotent; after the first tick the route is the unique zero-expansion route) |
| `min_ticks` = base sum | `diff_ops` (tree and function oracles) | holds (L4 also covers) |
| `min_ticks` is a floor over histories | new probe `l3_min_ticks_floor_over_histories` (linear histories, tick-batch sets) | holds over 100,000 histories of up to 60 operations; calibrated by M22. No proof: the join step lacks a general argument (only the disjoint-single-region case was argued by hand) |
| `min_ticks` is attained | new probes `l3_min_ticks_tight*` (constructive single-seed history) | holds over 100,000 arbitrary versions; calibrated by M22, M23 |
| Clock wrappers equal their compositions | reading (`clock.rs:117-600`: one-line compositions); laws `recv_is_join_then_tick`, `recv_all_is_joins_then_tick`, `absorb_is_the_anonymous_join`, `absorb_all_is_the_sequential_joins`, `send_advances_and_returns_the_version`, `clock_ticks_matches_version_ticks` | holds |
| tick precondition (party owns a region) | party grammar (`party/io/validate.rs:37-52`) has no empty spelling | always holds |
| tick heap: small multiple of input | `l3_heap_memo_families` (release): 6.7, 11.7, 0.9 B/B on memo-dense families; `l3_heap_search`: max 43% of the board ceiling | holds |
| `min_ticks` heap: small multiple of input | `l3_heap_min_ticks_*` (release) | **violated: D1** (39.4 B/B plain rising spine; up to 197 B/B jump-entered) |
| tick and `min_ticks` time (touch, scan) | board families; `l3_counter_search` (release): tick max 13.5 touches/B and 47 scan bits/B (ceilings 22, 96); `min_ticks` 8.8 and 8.0 | no outlier found; no new scaling family constructed |
| usize invariance | enumeration of every `usize` in the lane (see `coverage-map.md`) | no target-dependent behavior |
| closed fixes | memo/suspended-level heap: holds (measured); `min_ticks` heap ceiling: re-opened (D1); `u32` caps: none remain in the lane (grep) | as stated |

## Commands and results

- Probes at 2000 cases: `PROPTEST_CASES=2000 cargo nextest run --locked -p before --all-features
  -E "test(/l3_/)"` → 4 of 4 pass (`run-probe-1.log`, before later probes were added).
- Generator histograms: `-E "test(/l3_generator_histograms/)" --no-capture` (`run-hist-1.log`).
- Calibration batch 1 (M1-M14) and batch 2 (M15, M16, M19, M21-M23): runtime switches
  `L3_MUT=k`, patches `mutations-batch1.patch`, `mutations-batch2.patch`; logs
  `calib-batch1{b,c,d}.log`, `calib-batch2.log`; unmutated baselines pass everywhere; both
  batches reverted (`git status` clean, verified) before committing.
- Heap (release): `cargo nextest run --locked -p before --all-features --release
  --test-threads 1 --no-capture -E "test(/l3_heap…/)"` → `run-heap-release-{1,2,3,4}.log`.
- Reach and counters (release): `-E "test(/l3_generator_reach|l3_counter_search/)"` →
  `run-diag-release-1.log`.
- Long runs (debug, libtest, no timeout): A = `PROPTEST_CASES=100000 cargo test --locked -p
  before --all-features --lib -- --test-threads 8 l3_cogen l3_family_bushy l3_family_spine
  l3_min_ticks` (`long-A.log`); B = `PROPTEST_CASES=5000 L3_DEEP_CASES=300 … l3_family_wide
  l3_family_multi_wide l3_deep_spine` (`long-B.log`). Results: see the report.

## Blind spots

- The floor property is sampled, not proven; histories are at most 60 operations.
- Oracle-checked depth stays near 50 levels (spines of 40 plus sides) and 400-region wide trees;
  the 3000-level spines check only oracle-free claims.
- Time bounds: I relied on the board's families plus a small-input random search; I constructed no
  new pessimal scaling family for tick's touch or scan counters.
- The `l3_family_wide`/`multi_wide`/`deep` harnesses exceed nextest's 180 s limit at high counts
  in debug builds; committed versions need lower defaults.
- 32-bit: nothing in the lane runs on wasm32 (`wasm32-pins` has no tick check). The usize
  enumeration finds no width-dependent quantity, so I see nothing a pin would separate.
- suanpan's amortization (`compare_offset_to` re-reads the gap) is L7's question; not examined.

## How to resume

Worktree `/Users/oxide/src/rumors-audit-l3-events` at `873f3999`. `NOTES.md` in the scratch
directory is the resumption record; `apply-batch2.py` and the two mutation patches reproduce the
calibration.
