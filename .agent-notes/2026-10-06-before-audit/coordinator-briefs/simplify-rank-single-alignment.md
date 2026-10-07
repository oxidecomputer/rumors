<!-- CAVEAT LECTOR: a simplification brief written by the coordinator (Claude Opus 5.5). It supersedes lanes/l4-measures/round-1/simplification-rank-width-invariant-routing.md, after Finch judged the usize-keyed routing odd and likely simplifiable. -->

# Simplification brief: give `Rank`'s `+` and `checked_sub` one alignment route

Kind: self-contained simplification. It changes no public API or format and
no value. Native behavior is unchanged. Behavior on wasm32 changes only in
which internal route a large gap takes.

## Current code

- `Rank::alignment_fits` (`crates/before/src/rank.rs:495-498`) checks
  `usize::try_from(gap)` for both exponent gaps.
  - When both fit, `+` (`rank.rs:951-963`) and `checked_sub`'s `Greater` arm
    (`rank.rs:196-209`) align with `num << gap` and combine `BigUint`s.
  - Otherwise they call `Rank::accumulate` (`rank.rs:505-531`), which
    combines the operands through a `suanpan::Accumulator`.
  - The comment at `rank.rs:954-955` gives the reason: "Shift and add directly
    when both exponent gaps fit `usize`. The accumulator handles larger gaps
    without narrowing the exponent."
- On 64-bit targets every gap fits, so `Rank::accumulate` never runs. On
  wasm32 it runs only for gaps of 2^32 bits or more.
- `Rank::accumulate`'s only callers are these two.

## Why the dispatch is unnecessary (the coordinator read this; verify it)

The premise that the big-integer shift needs a `usize` gap is false for the
`num-bigint` version in the lockfile, 0.4.8:

- `BigUint` implements `Shl<u64>` through
  `impl_shift! { u8, u16, u32, u64, u128, usize }` (`src/biguint/shift.rs:182`).
- `biguint_shl` (`shift.rs:12-23`) converts only the *digit* count,
  `(shift / BITS).to_usize()`. Here `BITS` is the big-digit width: 32 or 64
  bits.
- So `num << gap_u64` works on wasm32 for any gap up to about `2^32 · 32`
  bits. Larger gaps fail with "capacity overflow".
- That failure needs an operand with an exponent of about 2^37 or more. The
  coordinator infers, and you must confirm, that every route that constructs
  a `Rank` bounds its exponent by the input it consumed:
  - decode
  - parse
  - `Version::rank`
  - arithmetic, which takes the larger exponent of its operands

  If so, such an operand needs about 16 GiB of input, which no 32-bit process
  can hold.

## Proposed structure

1. Delete `alignment_fits` and `accumulate`.
2. `+` and `checked_sub` always align with `u64` shifts and combine the
   `BigUint`s, as they already do on 64-bit.
3. Restate the comments: the gap is a `u64` bit count, and `BigUint`'s `u64`
   shift accepts it directly.
4. `Rank`'s `Sum` (`sum_iter`) keeps its accumulator. It streams an unknown
   number of summands, which is a different job. Leave it alone, and say so
   in your report.

## What to verify

- **Values.** The committed rank tests and laws cover them natively, and
  native behavior does not change at all. The board's rank rows must not
  move. They cannot, since the native route is unchanged, but confirm with
  the board leg.
- **wasm32.** The pin `rank_arithmetic_straddles_the_usize_alignment_limit`
  (`wasm32-pins/harness/tests/pins.rs:112-119`) must pass with large gaps now
  taking the shift route. Measure its memory high-water mark and runtime
  before and after. The shift route allocates the aligned operands as
  `BigUint`s (32-bit digits in `u32`s), where the accumulator used `i64`
  digits, so memory should fall. Lower any committed ceiling it moves.
- **Naming.** The pin's name and doc describe a routing boundary that no
  longer exists. Rename it and restate its doc as a pin of exact large-gap
  arithmetic on 32-bit targets. Its cases remain useful.
- **Calibration.** By reversible string swap, inject a defect into the
  remaining route (for example, swap the operands of one shift) and show both
  the native tests and the wasm32 pin fail.
- **Overflow claim.** Confirm or refute the exponent-bound inference above.
  If some construction route can make a huge exponent from small input,
  stop and report: the "capacity overflow" panic would then be reachable, and
  the design needs the owner.

## Coordination

`fix/suanpan-reserve-digits-hint` edits `Rank::accumulate`'s reservation call
to use the new `reserve_bits(u64)`. This branch deletes that function. If the
fix lands first, the rebase resolves the conflict by deletion. Report it
either way.
