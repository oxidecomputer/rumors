# L3 instrument inventory (explore/l3-events)

At commit `30379f8d` (signed) unless a later commit is named in the row (`01a24058`, `1a8601b3`, `bab0a381`, `873f3999`, all signed) on `explore/l3-events`, worktree
`/Users/oxide/src/rumors-audit-l3-events`. Files:
`crates/before/src/version/tick/l3_probe.rs` and `crates/before/src/version/tick/l3_heap.rs`,
registered as `#[cfg(test)]` children of `version::tick` (so they reach `TickWalk::decide`).
Calibration logs: `calib-batch1b.log`, `calib-batch1c.log`, `calib-batch1d.log`,
`calib-batch2.log` in the auditor-l3 scratch directory. "Committed" below means the 72 tests of
the nextest filter `test(/version::tick::/) - test(/l3_/) | test(/min_ticks/) | test(/ticks/)`.

## Shared machinery

- **Co-generator** (`VSk`, `PSk`, `Pair`, `arb_pair`, `arb_spine`, `arb_palette`, `build`,
  `resolve`): builds party and version together region by region (`Owned`, `Unowned`,
  `OverLeaf`, `Aligned`), resolving version leaves against a per-case palette of 1..=5 absolute
  heights (small, word, near `u64::MAX`, `2^64 + 0..4`, `2^130 + 0..4`, `m * 2^s`).
- **`check_one(v, p)`**: against the recursive tree oracle: fill flag equals
  `oracle.fill(i, e) != e`; the simplified output equals the oracle's fill; `tick` equals the
  oracle's `event` byte for byte; output canonical (`instrument::validate`); strict domination;
  the committed output envelope `2|e| + 4|i| + 32` bits; region locality (projection onto the
  complement party unchanged); `min_ticks` rises by at most one per tick; `ticks(k)` equals k
  iterated public ticks and the oracle's k ticks for k in 1..=4; `ticks(0)` is the identity;
  `ticks(a)` then `ticks(b)` equals `ticks(a + b)` for a, b in {1, 2^64 - 1, 2^64 + 3, 2^200 - 1},
  and `min_ticks` rises by at most the count.
- **`check_family`**: `check_one` on the pair, on its oracle one-tick successor (a fill fixed
  point, so the grow branch runs at depth), and on two late perturbations of that successor (the
  j-th and (j+1)-th leaf from the end set to a palette height, forcing a first divergence after
  a long matched prefix).

## Inventory

| Probe | Checks, against | Reach beyond committed generators | Calibration (exposure; catches; escapes from committed) |
|---|---|---|---|
| `l3_cogen_bushy` | `check_one`, tree oracle | bushy co-generated pairs: sites in 47% of draws, nesting up to 4, depth >= 4 in 20% (committed random pairs: sites 19%, nesting <= 2, depth >= 4 in 0.1%) | exposed to M1-M16, M19, M21-M23: catches M1-M4, M7, M11-M14, M16, M21, M22; misses M5, M6, M8, M10, M15, M19, M23 (M23 is a min_ticks shift outside this probe's claims). Committed also caught every mutant it catches |
| `l3_cogen_spine` | `check_one`, tree oracle | spines to 40 levels: nesting 7+ in 42%, two or more sites in one range in 25% (committed: 0%) | exposed as cogen_bushy: catches M1-M8, M10-M14, M16, M21, M22; of these, M6 and M8 escape the committed suite |
| `l3_family_bushy` | `check_family`, tree oracle | as cogen_bushy plus fixed points and late divergence | same catch set as cogen_bushy |
| `l3_family_spine` | `check_family`, tree oracle | as cogen_spine plus fixed points and late divergence | same catch set as cogen_spine (M6, M8 escape committed) |
| `l3_family_wide` | `check_family`, tree oracle | one outermost pre-scan over a balanced tree of 40..400 regions: a pre-scan with more than 64 memo slots in 66% of draws, up to 301 (committed independent generator: at most 2 slots; spines 23; bushy 4; `run-diag-release-1.log`) | exposed to M2-M16, M19, M21-M23 at 40 cases (timed out at 1000 debug cases): catches M3-M8, M10-M14, M16, M19; M6, M8, M19 escape committed |
| `l3_family_multi_wide` | `check_family`, tree oracle | 2..4 outermost pre-scans of 20..160 regions in one walk (2 to 4 scans in 80% of draws): two pre-scans of more than 64 slots in one walk in 25% of draws (no other generator: 0%) | exposed to batch 2 only (M15, M16, M19, M21-M23) at 40 cases: catches M15, M16, M19, M21, M22; M15 and M19 escape committed |
| `l3_deep_spine` | oracle-free claims (canonical, domination, locality, fill idempotence, min_ticks step, ticks(1..3), wide composition) | co-generated spines to 3000 levels with random sides (committed deep coverage: deterministic shapes) | exposed to M2-M16, M19, M21-M23 at 4 cases: catches M3, M5, M7, M10-M14, M21; timed out (inconclusive) under M2, M6, M8, M16 and once unmutated at 32 cases |
| `l3_min_ticks_floor_over_histories` | `min_ticks(version) <=` weight of tick batches in the clock's causal past, after every step | linear histories with wide `ticks` counts (2^0..2^80), send, recv, absorb, sync, join, foreign ticks (no committed test states the floor over histories) | catches M22 (+1) through its own assertion, and M3, M7, M11, M12, M16 incidentally; M23 (-1) correctly passes |
| `l3_min_ticks_tight`, `l3_min_ticks_tight_spine` | a single-seed linear history built from the normal-form bases reproduces the version with exactly `min_ticks` ticks | every canonical version from `arb_oracle_version` and deep co-generated versions (committed: the single-party line only) | catch M7, M11, M12, M22, M23 (M22 and M23 through their own count assertion) |
| `l3_generator_histograms` | diagnostic only (always passes) | measures sites, nesting, depth, multi-site ranges, owned-right raises, wide leaves for each generator | n/a |
| `l3_generator_reach` (bab0a381) | diagnostic: memo slots per pre-scan and pre-scans per walk for each generator | n/a | n/a |
| `l3_counter_search` (bab0a381, release, `touch-meter` + `scan-meter`) | diagnostic: largest tick and `min_ticks` touch and scan readings per input byte, sampling 8400 co-generated pairs and judging those of at least 64 bytes | small-to-mid co-generated shapes | n/a; maxima 13.5 touches/B and 47 scan bits/B for tick (ceilings 22 and 96), 8.8 and 8.0 for `min_ticks` |
| `l3_heap_search` (873f3999, release) | diagnostic: peak heap of `tick` and `ticks(2^70 + 3)` over 6300 co-generated pairs, scored against the board's `1024 + 20 n` | co-generated spines, bushy pairs, wide pre-scans | n/a; maximum 43% of the ceiling (small, intercept-dominated inputs), about 15% on wide pre-scans (`run-heap-release-4.log`) |
| `l3_bridge_round_trip` | the bridge round trip on one co-generated pair | n/a | n/a |
| `l3_heap_memo_families` | diagnostic peak heap (PeakAlloc global allocator in the lib test binary) of one tick: sibling sites (alternating and constant minima) and a nested chain, sizes 2^10..2^16 | memo-dense shapes | n/a (diagnostic; results in the coverage record) |
| `l3_heap_min_ticks_rising_spine`, `l3_heap_min_ticks_rising_spine_fine` (01a24058), `l3_heap_min_ticks_jump_rising_spine` (1a8601b3) | diagnostic peak heap of `min_ticks` (and a seed tick) on a right spine of rising leaves, sizes on both sides of powers of two, 2^10..2^18 | one positive range-minimum boundary per level | n/a (diagnostic) |

Mutation key: M1 route tie left; M2 expansion tie left; M3 `consume_h_anchored` tie diverges;
M4 `resolve_inner` drops negation; M5 `record` skips `latest_from_first`; M6 `Min`-arm tie read
as domination; M7 zero delta for the expansion's first right sibling; M8 `pop_lookahead` skips
`resolve_deferred`; M9 negative control (always box; behavior-preserving; caught by nothing, as
intended); M10 memo `Block::take` off by one; M11 `min_ticks` skips the equal-close count;
M12 raise drops the trailing repair; M13 `ticks` raises `n` after a fill; M14 `seed_relation`
drops the offset; M15 `begin_scan` clears one block; M16 `resolve_inner` decrements the level;
M19 block 1 allocated late; M21 `begin_scan` keeps the cursor; M22 `min_ticks` + 1; M23
`min_ticks` - 1.

Committed-suite escapes (pass all 72 committed tests, caught by these probes): M6, M8, M15, M19.
