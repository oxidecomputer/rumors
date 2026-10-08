# Simplification brief: route `Rank` `+` and `checked_sub` by a width-invariant bound

Kind: self-contained, behavior-preserving (values identical; no public API,
format, or test-expectation change). Motivated by the `usize`-invariance
clause in `common.md`.

## Current code (base `58285ca5`)

`crates/before/src/rank.rs:495-498`:

```rust
/// Whether both exponent gaps fit the big-integer shift interface.
fn alignment_fits(a_exp: u64, b_exp: u64, common_exp: u64) -> bool {
    usize::try_from(common_exp - a_exp).is_ok() && usize::try_from(common_exp - b_exp).is_ok()
}
```

Its callers are `Rank::checked_sub`'s `Greater` arm (`rank.rs:196-209`) and
`impl Add<&Rank> for &Rank` (`rank.rs:951-963`, comment at `:954-955`: "Shift
and add directly when both exponent gaps fit `usize`"). When the predicate
holds, the operands are aligned with `BigUint << gap`; otherwise
`Rank::accumulate` (`rank.rs:505-531`) combines them through the suanpan
accumulator with `u64` shifts.

## Why it is a `usize`-invariance problem

The exponent gap is a stored arithmetic quantity (a bit offset in a value),
not a memory index or length, yet its routing depends on pointer width:

- On wasm32, gaps up to `2^32 - 1` take the shift route and gaps from `2^32`
  take the accumulator route.
- On 64-bit targets every feasible gap takes the shift route, so
  `Rank::accumulate` is **dead code on every 64-bit host**: no native test can
  execute it, and only the two `wasm32-pins` `RankArithmetic` cases at gap
  `2^32` run it.

Both routes return the same exact value (verified for the 32-bit boundary by
the committed `rank_arithmetic_straddles_the_usize_alignment_limit` pin), so
this is not a value defect. The routes do differ in memory profile: the
accumulator stores 32-bit digits in `i64`s (`suanpan` `digits.rs:64`). The
same `Rank` operation on the same operands therefore follows different code
and memory behavior by target.

## Proposed structure

Route by a bound that does not depend on the target:

```rust
/// The widest exponent gap aligned by a direct big-integer shift.
///
/// `BigUint`'s shift needs the gap's digit count in a `usize`, which every
/// supported target provides up to this bound; wider gaps use the
/// accumulator, whose shifts are `u64`. The bound is a fixed width so the
/// route, and with it the memory profile, is the same on every target.
const SHIFT_ROUTE_MAX_GAP: u64 = u32::MAX as u64;

fn alignment_fits(a_exp: u64, b_exp: u64, common_exp: u64) -> bool {
    common_exp - a_exp <= SHIFT_ROUTE_MAX_GAP && common_exp - b_exp <= SHIFT_ROUTE_MAX_GAP
}
```

and restate the `Add` comment at `rank.rs:954-955` and the `checked_sub`
comment accordingly ("when both exponent gaps are within
`SHIFT_ROUTE_MAX_GAP`").

Effect by target:

- wasm32: unchanged (`usize::MAX == u32::MAX`), so the committed pin's cases
  at gaps `2^32 - 1` (shift route) and `2^32` (accumulator route) keep
  exercising both sides of the same boundary.
- 64-bit: unchanged for every gap below `2^32` bits, which covers every
  committed test, board family, and fuelscape operand (the largest are orders
  of magnitude smaller). Gaps of `2^32` bits or more, reachable only with
  operands of hundreds of megabytes, now take the accumulator route, as on
  wasm32.

## Why the result is more obviously correct

The routing rule becomes one target-independent sentence, and the 32-bit pin
then witnesses the routing boundary for every target rather than for wasm32
alone. `Rank::accumulate` stops being a 32-bit-only branch.

## Coverage

- Values: `RANK_TRIPLE` laws, `rank_cmp_agrees_with_the_alignment_oracle_on_25k_pairs`,
  `rank_sum_equals_the_pairwise_fold`, and the `wasm32-pins` pin above
  (rerun with `just wasm32-pins`; the pin's doc comment should name the
  constant rather than `usize::MAX`).
- Costs: no board cell should move, since no committed operand reaches a
  `2^32`-bit gap. The builder confirms with the gate's board leg matching
  `baseline.md`. If any ceiling or ranking moves, stop and report.

## Optional follow-on (design proposal, not part of this brief)

A much smaller bound (for example `2^16` bits) would make ordinary native
property tests (`stream_rank` draws exponents up to 100,000) execute the
accumulator route routinely, closing the gap that `Rank::accumulate` is
otherwise tested only in wasm32. That moves constant factors for wide gaps on
every target and must be measured against the board's `rank_add` and
`rank_checked_sub` cells first. Observation O2 (folding the duplicated
dispatch in `Add` and `checked_sub` into one helper) composes with either
choice.
