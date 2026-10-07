# Simplification brief: write the join/meet sweep once

Kind: self-contained. It preserves behavior, changes no public API or format,
and touches only `crates/before/src/version/lattice.rs`.

## Current code

`lattice.rs` contains two copies of the same overlay sweep:

- `Version::hull_bits` (`lattice.rs:151-220`) emits both extremes and the
  causal relation from one walk. It keeps a two-element `[Emission; 2]` and an
  `OrderState`.
- `Extreme::emit` (`lattice.rs:233-281`) emits one extreme. It repeats the
  same opening (`OpenedPair::open`, the first-side pick, `out.height` at
  `max(depth_a, depth_b)`, dropping the opening integers) and the same loop
  (`advance_diff`, `pick`, `write_delta`).

The module doc (`lattice.rs:17-26`) describes a single sweep: "Join and meet
are this one sweep with the side selection reversed … each entry point passes
its own picking closure and the sweep never consults which operation it is
running." No picking closure exists, and the sweep is written twice. The doc
describes a structure the code does not have.

## Proposed structure

Add one private function, generic over the number of emitted extremes:

```rust
/// Walk both operands once, emitting each requested extreme and the
/// operands' causal relation.
fn sweep<const N: usize>(
    a: &Version,
    b: &Version,
    extremes: [Extreme; N],
) -> ([Version; N], Option<Ordering>)
```

Its body is today's `hull_bits` body with `[Emission; 2]` generalized to
`[Emission; N]`. Then:

- `Extreme::emit(self, a, b)` becomes `let ([out], _) = sweep(a, b, [self]); out`;
- `hull_bits` becomes
  `let ([lo, hi], relation) = sweep(self, other, [Extreme::Lower, Extreme::Higher]);`
  followed by building `Hull { relation, lo, hi }`.

Restate the module doc's "picking closure" sentence in terms of `Extreme`
values passed to the one sweep.

## Why the result is more obviously correct

The claim that join and meet are one sweep with the selection reversed
becomes structural. A reviewer reads one loop. A future fix to the opening
move, the depth rule, or the switch delta cannot reach one copy and miss the
other.

## Cost

`emit` would fold one `OrderState` per boundary, a match on a `Copy` enum.
That fold allocates nothing, scans no bits, and touches no accumulator digit,
so the board's heap, scan, and touch readings for `version_join`,
`version_meet`, and their assign rows should not move. The builder confirms
this with the board leg, and stops if any ceiling would rise. If a
measurable constant appears, a `const TRACK_ORDER: bool` parameter removes
the fold from `emit` without splitting the loop.

## Coverage

- `src/version/lattice/tests.rs`: `assert_emits` checks `join`, `meet`, and
  `hull_bits` (the relation, `lo`, `hi`) against the recursive oracle and an
  independent pointwise walk, over families, arbitrary pairs, organic
  histories, wide grids, cliff staircases, and the exhaustive small scope.
- The `reanchor_join_scan_is_linear_per_input_bit` scan pin and the board
  rows for join, meet, and span.
- My explore-branch probe (`crates/before/tests/l2_probe`).
