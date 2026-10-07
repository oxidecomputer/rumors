# L3 coverage map (working notes, auditor-l3)

Base: explore/l3-events at 58285ca5 (tree = 455e97de + notes).

## Contract clauses and the instruments behind them

| Clause | Source | Committed instrument | Input distribution | What it misses |
|---|---|---|---|---|
| tick = paper event (fill then grow, exact) | version.rs:223-250, itc2008 5.3.4 | tick/tests.rs `assert_tick` over event_pool x party_pool, exhaustive small scope, `arbitrary_pairs_*`, organic histories, worked/witness families | independent party/version trees, prop_recursive depth 4; exhaustive EV/ID small depth | measured: 81% of arbitrary pairs have no lookahead site; nesting <= 2; depth < 4; never two sites in one range |
| fill flag = (fill(i,e) != e) | tick.rs decide | same `assert_tick` | same | same |
| strict domination | version.rs:225 | laws (version_party?) + derived from oracle equality | as above | covered transitively by oracle equality |
| region locality (projection on disjoint party unchanged) | version.rs:226-228 | ? (check laws/version_party) | | |
| ticks(k) = k ticks | version.rs:252-256 | ticks_matches_iterated_* (k <= 64/1000), ticks_composes (2^100 shapes), laws ticks_composes (k = min_ticks of operands) | arbitrary depth-4 pairs, deterministic shapes | deep co-generated pairs with nested sites |
| min_ticks = base sum | version.rs:308-345 | diff_ops (tree + function oracles), measure/tests.rs | diff_ops generators | - |
| min_ticks floor over histories | version.rs:311-314 | NONE found | - | the history claim itself |
| min_ticks tight | version.rs:308-314 (implied: "minimum") | laws ticks_line_realizes_min_ticks (single-party line only) | - | multi-node versions |
| Clock wrappers = compositions | clock.rs:117-600 | laws/clock.rs recv_is_join_then_tick, recv_all_is_joins_then_tick, absorb_is_the_anonymous_join, absorb_all_is_the_sequential_joins, send_advances..., clock_ticks_matches_version_ticks | law registry generators | wrappers are one-line compositions by inspection |
| tick O(|v|+|p|) time | version.rs:237-238 | board version_tick/version_ticks/clock_tick rows over tick-walk families, tests/meter | registered families | adversarial families not on the board |
| tick heap small multiple | lib.rs crate doc | board heap ceiling 20 B/B | registered families | memo-dense and boundary-dense families |
| min_ticks heap | same | board version_min_ticks | registered families | rising right spine (positive boundary per level) |
| 32-bit | common.md | wasm32-pins: no tick check | - | tick/min_ticks on wasm32 entirely |

## Probes on explore/l3-events

- `version::tick::l3_probe`: co-generated pairs (bushy, spine), fixed points, late perturbations; full claim set per pair (oracle event, flag, canonical, strict domination, complement-projection locality, min_ticks step <= 1, ticks(1..4) vs iterated and oracle, wide composition, ticks(0)); min_ticks floor over linear histories with tick-batch sets; min_ticks tightness by constructive single-seed history.
- `version::tick::l3_heap`: peak heap via PeakAlloc global allocator for memo-dense tick families and min_ticks rising right spine.

## usize invariance (common.md clause, applied 2026-10-07 at 30379f8d)

Public lane signatures take no `usize`: counts arrive as `impl Into<Count>`, results are
`Version`/`Count`. Every `usize` in the lane's code (grep over version/tick.rs, tick/, tick/prescan/,
measure/min_ticks*, range_minima*) is a memory index or a count of held memory:

- memo: `BLOCK_SLOTS`, `Block::{set,take}(slot)`, `Memo::{len,cursor,reserve,set_link,take_link}`;
  pre-scan frames' slot (stored as u64 deltas, restored from values that came from usize);
  `Level::first_slot`;
- `OUT_FOLLOWER`/`REL_FOLLOWER`/`FOLLOWER_SLOTS`: follower array slots;
- min_ticks: `LeafHeight::prefix`/`Contribution::prefix` (index into `HeightPrefixes::components`),
  spill index and free list in `ContributionStore`.

Width-sensitive thresholds compare against width-independent constants: the inline contribution
(`i32` offset, 23-bit prefix, 8-bit closes), `StoredAccumulator`/`Boundary` "at most two digits",
and the freeze allowance, all in suanpan digits fixed at 32 bits (`DIGIT_BITS = 32`). Stored
counters are u64/i128/u128 (depth, keys, route costs, coefficients, closes). Verdict: no
target-dependent behavior found; nothing to demonstrate on wasm32.
