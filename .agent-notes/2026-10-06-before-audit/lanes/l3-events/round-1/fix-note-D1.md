# Fix note D1: `min_ticks` transient heap

Root cause (see the defect record): two independent mechanisms, both internal to `min_ticks`.

## Mechanism 1: payload storage, 8 bytes per positive boundary, grown by doubling

Where: `RangeMinima<StoredContribution>` (`version/measure/min_ticks/minima.rs:115`) and
`Boundaries::payloads: Vec<P>` (`version/range_minima/boundaries.rs:38`). Tick's walk and pre-scan
use `P = ()`, so they are unaffected.

Candidate repairs, in order of increasing change:

1. **Grow without copying.** A segmented LIFO for `payloads` (fixed-size chunks, so growth never
   reallocates and slack is at most one chunk) removes the doubling slack and the reallocation
   transient. The plain rising spine then reads about 12.8 B/B (8 bytes per 5-bit level), under 20.
   The cost is one indirection per push and pop.
2. **Do not store what can be recomputed.** On a positive boundary `b = inner_min - outer_min`,
   the suspended outer contribution's offset equals the inner contribution's offset minus `b`
   whenever both share a height prefix, and the boundary is already stored. Only the prefix index
   (often equal) and the outer close count (often zero) would need storing, which the existing
   `PackedU64Stack` holds in a few bits. This is the larger redesign (a substantive change to the
   `RangeMinima` payload contract, since `P` would become partly derived).

## Mechanism 2: contributions spill whenever the live offset leaves `i32`

Where: `StoredContribution::inline` (`contributions.rs:83-91`) versus the freeze rule in
`Count::min_ticks_for` (`min_ticks.rs:66-72`, allowance `HEIGHT_FREEZE_ALLOWANCE_DIGITS = 8`,
`measure.rs:26`). The allowance (256 bits) and the inline offset width (32 bits) are mismatched, so
live changes between `2^31` and about `2^288` make every new contribution spill.

Candidate repair: also freeze when the live change would put the next stored contribution's offset
outside the inline range. After the freeze the live change restarts at zero, so later small
deltas store inline again. Cost argument to check: each such freeze is preceded by at least 31 bits
of accumulated change since the previous freeze; under unit deltas that takes `2^31` leaves, and a
wide delta of 31+ bits costs at least 63 input bits, so frozen components (one `BigInt` plus one
`i128` coefficient) stay bounded by a constant per 63 input bits. The freeze's own arithmetic reads
the frozen width once. Re-check the existing amortization arguments in `heights.rs` and the
`settle_flatness` / freeze-schedule meter tests after the change.

## Other entry points sharing the mechanism

None found: `ContributionStore`, `HeightPrefixes`, and `SubtreeMinima` are used only by
`min_ticks`, and only `min_ticks` instantiates `RangeMinima` with a non-zero-sized payload
(grep of `crates/before/src` at 58285ca5).

## Formats

Both repairs are internal. No public API, wire, or storage format changes.
