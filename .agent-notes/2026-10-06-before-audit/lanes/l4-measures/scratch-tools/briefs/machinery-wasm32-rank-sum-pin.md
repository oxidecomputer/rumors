# Machinery brief: pin `Rank`'s `Sum` across wasm32's alignment limit

## Failure class it catches (constructible)

`Rank::sum_iter` (`crates/before/src/rank.rs:1033-1075`, base `58285ca5`) is
`Rank`'s third arithmetic route, separate from `+` and `checked_sub`: it
always uses the suanpan accumulator, shifting the held value by
`max(gap, held_span)` when a summand needs a larger exponent and landing each
summand at bit offset `exp - rank.exp`, both as `u64`. A defect that narrows
either quantity through `usize` (for example `shift as usize as u64`) is the
identity on 64-bit hosts, so **no native test can detect it**, while on
wasm32 it truncates a `2^32` gap and either produces a wrong sum or lands at
an unaddressable digit.

The existing `RankArithmetic` wasm32 pin (`wasm32-pins/guest/src/checks.rs`,
`rank_arithmetic`, cases 1-4) exercises only `+` and `checked_sub` at gaps of
`2^32 - 1` and `2^32`. No wasm32 check calls `Sum`.

## Which instrument it extends

`wasm32-pins`: add two cases to `Check::RankArithmetic` (no protocol change)
and extend the pin's loop in `harness/tests/pins.rs`
(`rank_arithmetic_straddles_the_usize_alignment_limit`) from `1..=4` to
`1..=6`. Update that test's doc comment to name `Sum` beside addition and
checked subtraction.

## Construction

Inside `rank_arithmetic`, after `deep` (exponent `2^32`) is decoded as today:

- case 5: `small = half()` (exponent 1, gap `2^32 - 1`);
- case 6: `small = uniform(1u8).rank()` (exponent 0, gap `2^32`);
- both: `let sum: Rank = [&small, &deep].into_iter().sum();` (small summand
  **first**, so the second summand forces the held-value shift by the full
  gap), then require `sum == &deep + &small` and
  `sum.checked_sub(&small).as_ref() == Some(&deep)`, else
  `Err(Failure::WrongValue)`. `+` and `checked_sub` at these exponents are
  independently pinned by cases 1-4.

Do **not** pin the deep-first order (`[&deep, &small]`): on base it aborts on
allocation failure (see observation O6 in the L4 report), so it cannot pass
until `Sum`'s accumulator memory is reduced.

## Calibration evidence (prototype on `explore/l4-measures` at `2c82c7ba`)

Run on ox-east-1 from `crates/before/wasm32-pins` with the guest and harness
built in release (`target/wasm32-pins`), cases selected through the
prototype's `l4_rank_sum_cases_diagnosed` test:

- Base: small-first with half passes (`L4SUM case 7: Passed pages=58399`),
  small-first with one passes (`L4SUM case 8: Passed pages=50207`). (The
  prototype numbers the cases 7 and 8; this brief renumbers them 5 and 6.)
- Mutant `let shift = shift as usize as u64;` inserted after `sum_iter`'s
  shift computation: the one-first case fails, `L4SUM case 8:
  Trapped(UnreachableCodeReached) pages=42014 panic="panicked at
  .../suanpan/src/accumulator.rs:417:10:\na nonzero contribution needs an
  addressable digit position"`; the half-first case still passes (its gap
  `2^32 - 1` survives narrowing), which is why both cases belong in the pin.
- Reverted; `git diff -- crates/before/src/rank.rs` empty.

## Cost

Each case decodes the existing 604 MB synthetic rank. Measured guest memory
high-water: 3.56 GiB (58,399 pages) for the half-first case and 3.06 GiB
(50,207 pages) for the one-first case, about two minutes each on a loaded box.
The half-first case has only about 0.44 GiB of headroom under wasm32's 4 GiB:
a later change that adds one value-sized copy to `Sum`, `Add`, or the decoder
would turn it into an allocation abort. That would be deterministic, not
flaky, but the builder should record the high-water mark in the test's doc
comment so a future failure is diagnosed as memory, not value. Use the trap
diagnosis from the companion brief if it lands.

## Index entry

None required: this extends an existing check rather than adding an
instrument. Note that `crates/before/src/testing/validation_index.rs` does not
list the `wasm32-pins` workspace at all (no row mentions 32-bit or wasm32
pins; its only wasm mention is the fuzz-fit fuel bands); that omission is
recorded separately as an observation for the adequacy lane.
