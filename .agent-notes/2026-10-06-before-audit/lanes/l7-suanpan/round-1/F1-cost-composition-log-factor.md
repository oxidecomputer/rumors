# F1: `before`'s linear decode and comparison inherit suanpan's per-update logarithmic factor

Severity: medium (an asymptotic breach of documented `O(n)` bounds, by a logarithmic factor). The defect lies in how the two crates' contracts compose; suanpan itself is within its documentation.
Status: measured on ox-east-1 with wasmtime fuel (the metric `before`'s own measured-growth charts use), explore branch, commits `d43fa15e` (instrument) and the dense-control commit after it.

## Contract clauses breached

- `crates/before/src/lib.rs:350-358`: "Any asymptotic claim is a hard guarantee that the operation will perform in time proportionate to that bound, for all input sizes, no matter how unlikely and contorted the shape of the input. [...] Any violation of these guarantees is a bug".
- `crates/before/src/version.rs:1086-1092` (`Version::decode`): "`O(n)` in total input bytes; `O(n)`, `n` the bytes read, accepted or rejected".
- `crates/before/src/version.rs:65-80` (comparison): "All comparisons are linear in the combined input size"; "full relation: `O(n)` in total input bytes; `O(|a| + |b|)`".

The bound they compose with is suanpan's own, which is honest: `crates/suanpan/src/lib.rs:78-83` prices a primitive update at amortized `O(log(W + 1))` and an `L`-limb update at `O(L log(W + 1) + G)`.

## Mechanism

Both operations keep one long-lived accumulator and fold every leaf's delta into it: `Version::decode` through `validate::from_reader` (`crates/before/src/version/io/validate.rs:79-108`, `height.add_bigint(&delta)` per leaf), and comparison through `OpenedPair`'s running difference (`crates/before/src/version/overlay.rs:259-265` seeds it with the opening heights; `advance_diff` folds every boundary delta). Each nonzero deposit calls `Digits::add_at`, which calls `ZeroRanges::remove_written` (`crates/suanpan/src/accumulator/digits/zero_ranges.rs:102-132`). Once two or more zero ranges coexist (`Ranges::Many`), that call performs a `BTreeMap::range(..to).next_back()` lookup on every write, even a write that intersects no range, so every leaf costs `O(log R)` for `R` recorded ranges.

One wide opening height with sparse digits records `R` ranges at once (each nonzero digit after the first lands above the current top, so `record_gap` inserts a range). Those ranges persist: later small deltas land at digit 0, and the comparison's sign reads decide at the top digit without consuming them.

## Input family (constructed)

Root base `H = sum_{i < r} 4 * 2^(64 i)` (digits `4, 0, 4, 0, ...`: `r - 1` zero ranges after the first deposit), over a complete binary tree with `64 r` leaves in pairs `(1, 0)`, so leaf heights alternate `H + 1`, `H`. Built inside the wasm guest through `before::testing::meter::Encoding` (`crates/before/wasm32-pins/guest/src/checks.rs`, `l7_version_base`). Encoded sizes, measured natively (`crates/before/tests/explore_l7.rs`): 3,569 / 14,321 / 57,329 / 229,361 / 917,489 bytes for `r` = 64 / 256 / 1,024 / 4,096 / 16,384, so bytes grow linearly in `r` and the leaf-to-byte ratio is constant.

## Measurements (wasmtime fuel, deterministic)

Fuel for the operation alone is the difference between a guest run that builds and encodes the version and one that also performs the operation.

| `r` | bytes | decode fuel/byte | `Version::new() <= v` fuel/byte | suanpan fuel per word update, same `R` |
|---|---|---|---|---|
| 64 | 3,569 | 1,487.0 | 1,648.3 | 371.5 |
| 256 | 14,321 | 1,568.9 | 1,729.3 | 440.5 |
| 1,024 | 57,329 | 1,654.4 | 1,814.6 | 509.5 |
| 4,096 | 229,361 | 1,661.8 | 1,821.9 | 509.5 |
| 16,384 | 917,489 | 1,747.6 | 1,907.7 | 578.5 |

Control: the same tree shapes, depths, and widths with a dense base (`2r` nonzero digits, so no zero ranges) read flat: decode 1,091.5 to 1,091.0 fuel per leaf, comparison 1,232.1 to 1,231.1, suanpan 181.5 per word update at every size (`S/wasm4-dense.log`). The sparse family's per-byte cost rises monotonically, in the staircase of `BTreeMap` height growth (the plateau between 1,024 and 4,096 appears in all three columns), so the growth is the zero-range map's and not tree depth's.

Reproduction: `zz_l7_fuel_readings` in `crates/before/wasm32-pins/harness/tests/pins.rs` (an `#[ignore]` explore test; set `L7_DENSE=1` for the control), run through the wasm32 recipe in NOTES.md; about 10 minutes.

## Failure family

- Appears whenever an operation folds many small deltas into one accumulator that already holds `R >= 2` zero ranges; cost per fold is `O(log R)` beyond the documented constant.
- Ranges arise from any deposit whose nonzero digits land more than one position above the current top, so a single wide sparse height suffices; `R` is bounded by half the working width.
- Measured: `Version::decode`, `<=` (`causal_le`). Inferred by reading the same pattern, not measured: `partial_cmp`, `concurrent`, `join`, `meet`, and `hull_bits` (all on `OpenedPair`'s difference), `Version::tick` (`TickWalk::height`), `projection`, `place::filter`, and `validate::admit`. `min_ticks` seeds its opening height into `HeightPrefixes` rather than an accumulator, so this family does not reach its `answer` accumulator; another family might.
- Disappears when the accumulator holds at most one range (`Ranges::Empty` or `Ranges::One` take no map lookup), as in the dense control.

## Root cause

The two contracts do not compose: suanpan documents a per-update `log(W + 1)` factor (its zero-range map), and `before`'s linear bounds assume constant-cost updates. The log factor is reachable because `remove_written` consults the map on every write in `Many` mode, including writes that cannot intersect any range.

## Fix preserves API and formats

A documentation-only repair (restating the affected `before` bounds as `O(n log n)`) changes no API, wire, or storage format, but it changes rendered doc panels. A suanpan-side repair changes no API or format either, but it is a design change. Either is the owner's call, so I record no fix branch, only these options:

1. Restate the affected `before` bounds (and suanpan's composition guidance) as `O(n log n)`.
2. Remove the per-write lookup where it cannot matter. A write can only intersect a range interior at a position whose digit is zero, so a cheap pre-check (the written position held a nonzero digit, or lies outside `[first.lo + 1, last.hi - 1]`) skips the lookup for this measured family. Reasoned (not built): an adversary can still force a lookup per two-limb delta by oscillating a digit through zero inside the span (positions written then cancelled are zero but not in a range), so this alone keeps an `O(log R)` worst case.
3. Make "zero and not inside a range" decidable in `O(1)` by tagging written-zero positions in the digits' spare high bits (`|d| < 2^33` leaves 30 bits of an `i64`). Every position at or below the top is then either tagged or inside a range, and only writes that split a range would consult the map. My unverified estimate is that splits are bounded by the operand width that reaches them; this is a design sketch for the owner, not a measured result.

## Owner's ruling

`before`'s documented linear bounds must hold unconditionally. Fix `suanpan`
so they do, without weakening any other guarantee, rather than restating
`before`'s bounds. Gather the evidence and design first: the suanpan auditor's
round 2 measures the remaining operations and prototypes the candidate
designs.
