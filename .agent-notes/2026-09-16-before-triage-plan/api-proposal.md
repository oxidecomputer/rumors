# Approved `before` API work

This note records the owner-approved public-surface decisions for the final API
increments. The checklist records completion; this note records the intended
surface.

## Core types and operations

- Treat `Party` and `Clock` as `must_use` linear values. Mark the pure Version,
  Rank, and Count operations whose discarded results are likely mistakes.
- Export the fork iterators as `PartyForks` and `ClockForks`, without retaining
  the ambiguous old names.
- Call the crate-wide unbounded natural-number type `Count`. It accepts every
  unsigned primitive, converts fallibly into every unsigned primitive by value
  or reference, and provides checked and saturating subtraction without a
  panicking `Sub` implementation.
- Preserve the source of `Decode::Io`, make `Decode` non-exhaustive, and define
  `NotCanonical` in terms of the requested type's canonical-form and semantic
  invariants.
- Implement `Hash` wherever the existing public data-value equality admits it:
  `Span`, `Plateau`, `Rise`, `Region`, and `Cell`.
- Give `Floor` and `Ceiling` direct `coverage` operations through the existing
  query implementation, without allocating a query or copying a bound.
- Apply every breaking change to Rumors and its compatibility workspace in the
  same increment.

## Text and serialization

- `Party` and `Version` display lowercase canonical-byte hex with no prefix.
  Parsing accepts either hex case, but no prefix or whitespace.
- `Clock`, `Ranked`, and `Span` display records using their public components.
  Preserve the existing diagnostic `Debug` forms.
- Use one non-exhaustive `ParseValue` error that distinguishes malformed text,
  invalid component encodings, and crossed span endpoints.
- Human-readable serde uses these textual/component forms. `Rank` keeps its
  exact binary-point string, and `Count` uses its decimal display and parser.
- Existing binary serde remains byte-for-byte unchanged. Binary serde and
  borsh encode `Count` as least-significant-first `u64` limbs. The empty
  sequence is zero; a final zero limb is non-canonical.

## Deliberate omissions

Do not add integer-backend conversions, const constructors, composite
`encoded_len` methods, public instrument rosters, or conveniences without a
concrete caller. Do not add a panicking subtraction operation for `Count` or
retain aliases for renamed API.
