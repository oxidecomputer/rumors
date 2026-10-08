# U1: suanpan under the pointer-width clause

The new contract clause (common.md, "The contract you audit against"): `before` and `suanpan` behave identically whatever the width of `usize`; a `usize` anywhere other than indexing memory or counting what memory holds is a possible defect. This is a sweep of suanpan's whole public surface and its internals.

## Findings

**U1-a (low): `cmp_zero_stable_under` skips its compaction on 32-bit for widths of `32 * 2^32` bits or more.** `crates/suanpan/src/accumulator.rs:285-286`:

```rust
let adjustment_digits = bits.div_ceil(u64::from(DIGIT_BITS)).max(1);
let adjustment_high = usize::try_from(adjustment_digits - 1).ok()?;
```

On a 32-bit target, any `bits >= 32 * 2^32` returns `None` here, before the scan. On 64-bit the conversion succeeds for every `u64`, the scan runs, and the accumulator compacts before also returning `None`. The answer agrees on both widths. The side effect does not: stored width, later costs, and touch counts all differ.

- Native, verified: `l7_probe_stability_huge_width_compacts` builds `5` with a cancelling top (11 stored digits) and shows one stored digit after `cmp_zero_stable_under(u64::MAX)`.
- 32-bit, verified (`S/batch3.log`, built at `4fd78232`): guest case 9 (`zz_l7_suanpan_stability_width_compacts`) fails, the accumulator keeping its 11 stored digits:
  ```
  assertion `left == right` failed
    left: Failed(WrongLength)
   right: Passed
  ```
- **Fix** (preserves API and formats): keep the threshold in `u64`. Pass `adjustment_digits` (or `adjustment_high` as `u64`) to `Digits::cmp_zero_stable_above` and compare `index as u64 >= adjustment_high.saturating_add(2)`. The `usize` conversion then disappears, and the scan runs on every width.
- **Test brief:** a 32-bit pin in the `wasm32-pins` executor (the guest case above), plus the native assertion as a witness in `crates/suanpan/src/accumulator/tests/witnesses.rs`. Expected failure on the base commit, 32-bit only: the guest reports `Failed(WrongLength)` (11 stored digits remain).

**U1-b (high, already D1): the limb counter.** `Digits::apply_limbs` takes its limb index from `Iterator::enumerate` (`usize`), so a stream longer than `usize::MAX` limbs lands its later limbs at wrapped positions on 32-bit. Same mechanism, already briefed as D1.

**U1-c (with the owner as question 6): `reserve_digits(digits: usize)`.** A public parameter counting digits the caller wants reserved, not memory already held. See D2 and the coordinator's question 6 (a `u64` `reserve_bits`).

## Surface items that pass the clause

- `stored_digit_count() -> usize` counts stored positions. Its rustdoc routes ordinary use to `stored_bits() -> u64` and reserves the `usize` form for "an allocation calculation", which is counting what memory holds. It passes. If the owner prefers a width-free surface, it is the one remaining public `usize` return besides the primitive conversions.
- `From<usize>`, `From<isize>`, `TryFrom<Accumulator> for usize` and `isize`, `+=`/`-=` with `usize` and `isize`, and `<<`/`<<=` with `usize`/`isize` counts are conversions of those primitive types themselves. Their values convert exactly on every width, so the behavior is the value's, not the platform's.
- Shifts are `u64` (`add_shifted_limbs`, `add_shifted`, `<<=` after a checked conversion); bit widths are `u64` (`stored_bits`, `cmp_zero_stable_under`); the touch meter counts in `u64`.

## Internals that pass the clause

- Digit positions are buffer indices (`usize`), computed in `u128` and narrowed once, through `digit_index`, which panics with the documented landing message at `usize::MAX`. A landing that 64-bit memory could hold but 32-bit memory cannot panics only on 32-bit, which is the memory-holding exception.
- `add_digits` enumerates a slice, whose length is bounded by memory.
- `stored_bits` multiplies a held count into `u64`. Its `expect` messages ("fits u64") can fail only for more than `2^59` held digits, which no addressable allocation reaches.
- `normalized_limbs_with_shift` scales a held position into `u64`, by the same argument.

## In `before`'s bridge (`crates/before/src/accumulator.rs`)

- `digit_len(magnitude) -> usize` converts a `BigUint`'s bit length into a digit count (`expect("digit counts fit usize")`). The magnitude is held in memory, so the count is bounded by memory, and its callers compare it with `stored_digit_count()`. It passes.
- `Rank::accumulate` reserves only when `usize::try_from(widest / 32 + 2)` succeeds (`crates/before/src/rank.rs:513-517`), so the reservation hint silently differs by width. It moves with the owner's answer to question 6; I note it for the measures lane.
