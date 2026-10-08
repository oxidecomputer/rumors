# Machinery brief: drive `RangeMinima`'s model test with values near existing boundaries

Owner lane: events (tick) and measures (`min_ticks`) share this kernel; one
builder takes the whole brief.

## The failure class it catches

`Boundary::lower_wide` (`crates/before/src/version/range_minima/boundary.rs:102-133`)
has two fast paths for operands at least two 32-bit digits apart:

- line 106: a decrease that dominates the boundary leaves `decrease - boundary`;
- line 116: a boundary that dominates the decrease keeps `boundary - decrease`.

Both lines run under the suite (14 and 31 test processes reach them;
`S/probe-hits.txt`), yet the mutants that turn either subtraction into an
addition survive the full suite (`survivors/diffs/version.md`, entries for
`boundary.rs:106:30` and `:116:30`). A wrong remainder is a wrong value:
`propagate_drop` compares the leftover decrease with the next boundary
(range_minima.rs:265-294), and a closing range moves its stored boundary into
the anchor's deferred distance (`self.anchor.defer(boundary)`, range_minima.rs:323-326),
which later decides whether a height undercuts the minimum.

Why the suite misses them: the module's model test
`nested_operations_preserve_minima_and_payload_lifetimes`
(`range_minima/tests.rs:253-315`) draws every value as `i16 << bits`. Single
terms never place a leftover decrease within one boundary's width of the next
boundary, and the model never observes a stored boundary's magnitude except
through a later undercut, which its values rarely set up.

## Witnesses (verified on ox-east-1 from a probe copy; logs `witness/rm-*.log`)

Both use the model test's own assertions over explicit steps
`(value, opens, closes)`:

1. Decrease dominates (line 106). Steps: `(0, 1, 0)`,
   `(1 + 2^200 - 2^64, 1, 0)`, `(1 + 2^200, 1, 0)`, `(1, 0, 0)`.
   The undercut to 1 crosses the inner boundary `2^64` (wide, three digits)
   with a decrease of `2^200`; the leftover `2^200 - 2^64` stops one short of
   the outer boundary, which must survive as `1`. Base: passes. Mutant:
   `assertion 'left == right' failed: retired payloads at 1`, `left: [<m1>, 0]`,
   `right: [<m1>]` (the outer range's payload `0` is wrongly retired).
2. Boundary dominates (line 116). Steps: `(0, 1, 0)`, `(2^200, 1, 0)`,
   `(2^200 - 5, 0, 1)`, `(-5, 0, 0)`. The inner boundary `2^200` is lowered by
   `5` and then deferred when the inner range closes; `-5` must undercut the
   outer minimum `0`. Base: passes. Mutant: `assertion 'left == right'
   failed: undercut verdict at -5`, `left: false`, `right: true`.

Source of both: `l8/range_minima_witness.rs` on `explore/l8-adequacy`.

## Proposed machinery

Extend the existing model test rather than adding a parallel harness:

- Generate each step's value from a mix: the current single-term arm
  (`coefficient << bits`), plus an arm that picks an existing model minimum
  `m` and offsets it by a small signed amount or by a single term
  (`m ± c`, `m ± (c << bits)`), so leftover decreases land near later
  boundaries and boundaries are lowered by much narrower decreases.
- Keep `bits` up to 260 so both operands cross the two-digit separation that
  selects the fast paths.
- Pin the two witnesses above as named regression cases beside the property.

## Calibration

The extended property must fail on both `-=` to `+=` mutants above (and the
pinned cases must), and pass on the base. A builder confirms by reversible
string swap.

## Budget

The model test's cost is unchanged per case.

## Reach to the public API

`RangeMinima` serves `Version::tick` and `Version::min_ticks`. I verified the
witnesses at the kernel's own API, which is where the module's model test
lives. A public-input witness needs version heights `0`, `1 + 2^200 - 2^64`,
`1 + 2^200`, `1` as nested range minima in a tick's lookahead or a
`min_ticks` fold; I did not construct one.
