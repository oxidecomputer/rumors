<!-- CAVEAT LECTOR: written by Claude (Opus 5.5), agent audit-rank-fuel, for the before audit at e0956b25d. -->

# Simplification: dissolve `Integrator::one`

Kind: self-contained. It preserves behavior, changes no API or format, and
is two lines in one file. If D1's fix lands first, the edit sits a few lines
from that fix, in the same file.

## Current code

`crates/before/src/version/measure/integral.rs`:

- `:153-154`, the field: `/// Reused for region-width additions, avoiding an integer per region.` `one: BigUint,`
- `:169`, its initialization: `one: BigUint::from(1u8),`
- `:199-200`, its only use: `self.segment_width.add_shifted_limbs(weight_shift, self.one.iter_u64_digits());`

## Proposed structure

Delete the field and its initialization, and write the region width as the
one-limb stream it is:

```rust
self.segment_width.add_shifted_limbs(weight_shift, [1]);
```

`add_shifted_limbs` takes any `IntoIterator<Item = u64>`, and an array of one
`u64` yields that limb without allocating. `BigUint::from(1u8)` yields exactly
one `u64` digit, 1, so the stream is unchanged, and so are `suanpan`'s path
(`apply_limbs` sees one limb and takes the one-word branch) and its touch
count.

## Why it is more obviously correct

The field exists only to avoid an allocation that a literal never makes, and
its doc comment states a reason the code no longer needs. Without it, a
reader of `Integrator` sees only the state the integral is made of: total,
live, parked, segment width, base, deferred. It no longer has to check that
`one` is never mutated.

## Coverage

These already cover it:

- the measure differentials: `version::measure::tests::*`, which test rank,
  distance and lag against the paper oracle on generated and family inputs
- the touch bands in `tests/meter/version_scaling/skip_adequacy.rs`, whose
  counts must not move: the same stream reaches the same `suanpan` path
- the board's `version_rank`, `version_distance`, and `version_lag` rows

No new property is needed.
