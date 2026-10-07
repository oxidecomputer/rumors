# L5 observations (triage after the audit)

## Constant factors

- **Clamp refinement cost.** `Query::refine_partial`
  (`crates/before/src/causally/query.rs:220-252`) scans 1.54 times as many
  bits as the fused walk it refines, on a neutral `Partial` verdict, at all
  four measured sizes. A smaller equivalent test exists. Briefed in
  `brief-refine-partial.md`, with the coupling trade stated for the owner to
  rule on.

## Substantive simplifications (not briefed)

- **`OwnSpan::place` restates the nine-way table.**
  `crates/before/src/span/own.rs:117-136` transcribes `Placement` from two
  relations by hand. The fused walk's finish arm (`version/place.rs:153-168`)
  transcribes the same table in a different form, as do the test oracles
  (`version/place/tests.rs:composed_span`,
  `testing/laws/version_triple.rs:place_from_relations`). A crate-private
  `Placement::from_relations(lo, hi)` would give production one statement of
  the table, while the test transcriptions stay independent on purpose. Prior
  review: `span-causally-15`, still open. Correct today: every placement
  check passes against the vector oracle.

## `usize` invariance (new contract clause)

- No defect in the lane. Every `usize` in `span.rs`, `span/`, `causally/`,
  `version/place.rs`, and `version/place/filter.rs` falls into one of these:
  - a cursor slot index or `Vec` length (`priority`, `depth(slot)`, `sides.len()`)
  - a count of sides held in memory (`live`)
  - a count of accumulator digits held in memory, compared under
    `MAX_CLOSE_LEAD`
  - the wire decoder's checked conversion of a length already bounded by the
    buffer (`span/wire.rs:136-140`)

  No public signature in the lane takes or returns a `usize`.
- The `_all` folds run on `fold::balanced_reduce` (`crates/before/src/fold.rs:23-44`,
  shared code outside the lane). It stores a `usize` `weight` per stack entry.
  That weight is a merge level, bounded by `log2` of the number of inputs
  consumed, so it stays at 64 or below on any target, and its meaning does
  not change with pointer width. A `u8` or `u32` would state the bound in the
  type. This is a design nicety, not a defect. Inferred from reading; not
  executed on wasm32.
- `wasm32-pins` has no span or query check. Given the above, I found no
  pointer-width-dependent mechanism in the lane to demonstrate there.

## Closed fixes re-attacked

- **Multi-hole query auxiliary space** (width and bound count varying
  independently). I re-derived the `O(n + k)` bound from
  `filter.rs:110-292`, including the dual of the committed family: wide
  bounds against a narrow probe. In the bound-far-wider arm,
  `compare_heights` compacts only that bound's own height. It never stores a
  difference from a far-wider operand, and a retained difference stays
  within `MAX_CLOSE_LEAD` digits of the narrower height. So per-bound state
  is bounded by the bound's own encoded bytes. Reasoned, not heap-metered.
  The touch meter on a carry-boundary family (`l5_touch_carry_family`) shows
  linear time for all six operations.
- **Query conjunction over growing hole sets.** I re-derived
  `O(n(k + m + 1))` from `conjunction.rs:33-64`: the survive filter costs
  `k * |floor|` at most, and cross absorption sums to at most `(k + m) * n`.
  No construction exceeds it. Semantics are exact: the census and exhaustive
  checks pass for conjunctions in both association orders.
- **Span algebra identities.** The no-identity rationales for `Sum`
  (`algebra.rs:979-985`) and `Product` (`algebra.rs:1087-1092`) re-derive
  correctly. A union identity would need `lo` above every version and `hi`
  below every version, which describes the empty set and is not a span. An
  intersection identity would need a top version, and versions are unbounded.
- **Query construction against relation-level oracles.** Extended from the
  9-version grid to exact census coverage on 16-cell grids, spines to depth
  40, random shapes, organic populations, and the exhaustive 4-cell cube. No
  disagreement.

## Totality of the walks' debug assertions

- `filter::coverage`'s "a surviving hole keeps its dominated endpoint's pair
  live" assertion (`filter.rs:615-624`, message at 621) relies on `lo <= hi`. I checked every
  `Span` construction site (`Span::new`, `Span::at`, `Span::owned` callers in
  `algebra.rs`, `own.rs`, `version.rs`, `wire.rs`, `borsh_impls.rs`, and
  serde, which routes through `Span::new`). Each either checks the order or
  derives it by monotonicity, so the assertion is unreachable. The
  `place::span`, `dominance`, `precedence`, and `contains` finish assertions
  do not depend on endpoint order at all. Verified by reading; the harness
  runs every check with debug assertions enabled.
