<!-- CAVEAT LECTOR: written by Claude (Opus 5.5), auditor-l7, round 3. -->

# MB2: pin `deposit_value`'s huge-shift landings on wasm32

Kind: machinery brief (builder). It answers lead 3: "`deposit_value` at a huge shift (a small operand through `add_shifted`) is untested on wasm32. Brief a wasm32 pin case if it is reachable past `usize::MAX` positions, or explain why not."

## Verdict on the lead

Verified: the lead holds. `Digits::deposit_value` with a shift of `32 * 2^32` bits or more lands a nonzero digit at or past position `2^32`, which must panic on wasm32 (`crates/suanpan/src/accumulator.rs:145-146` and `:161-162`, the `add_shifted` and `sub_shifted` docs: "Panics if a nonzero contribution would land at or beyond `usize::MAX`"; the `<<=` doc in `operators.rs` says the same). Two public routes reach `deposit_value` with such a shift, and the wasm32 executor pins neither. No defect exists today: both routes pass the `u64` shift through unchanged and compute positions in `u128`. The gap is evidence. A 32-bit narrowing on either route passes every committed test.

The committed pin `suanpan_rejects_unaddressable_digit_landings` (`crates/before/wasm32-pins/harness/tests/pins.rs`) says that "every public shifted-accumulator route rejects a nonzero digit whose required buffer length cannot fit wasm32's `usize`." Its cases, as I traced and then confirmed with mutants:

| route | internal path | pinned by |
|---|---|---|
| `add_shifted_limbs`, one word | `apply_shifted_word`, `digit_index(u128::from(digit_shift))` | case 3 |
| `add_shifted_limbs`, a wider stream | `Digits::apply_limbs`, `u128` limb counter | cases 1 and 4 |
| `<<=` on a stored value, and `add_shifted` / `sub_shifted` with a stored operand | `apply_accumulator`, then `Digits::add_digits` | case 2. Its first `<<= 64` already exceeds `SMALL_SHIFT_MAX = 30` (`small.rs:28`), so the value is stored before the huge shift |
| `add_shifted` / `sub_shifted` with a small operand | `apply_accumulator`'s small branch, then `deposit_value` | **none** |
| `<<=` on a small value, one shift past the boundary | `shift_left`'s small branch, then `deposit_value` | **none** |

## Failure class it catches (constructible, calibrated)

A 32-bit narrowing of the shift on either `deposit_value` route. The landing falls near the start of the buffer, and the call returns normally where it must panic. Both mutants are one-line swaps that compile and do nothing on 64-bit hosts, so every native test passes under them:

- **L3S**, in `crates/suanpan/src/accumulator/operand.rs` (`apply_accumulator`): `self.digits.deposit_value(operand_value, shift);` becomes `self.digits.deposit_value(operand_value, shift as usize as u64);`.
- **L3H**, in `crates/suanpan/src/accumulator/operators.rs` (`shift_left`, small branch): `self.digits.deposit_value(value, shift);` becomes `self.digits.deposit_value(value, shift as usize as u64);`.

## Existing instrument it extends

The `wasm32-pins` executor's `SuanpanLanding` check: two new cases in `suanpan_landing` (`crates/before/wasm32-pins/guest/src/checks.rs`), with the loop in `suanpan_rejects_unaddressable_digit_landings` widened to cover them. Both share the check's contract, "correct code panics; any return is a failure", so no new check variant is needed.

## Construction

The shift is `32 << 32` (`32 * 2^32` bits). It is chosen so that a 32-bit truncation leaves 0 and lands the digit near the start of the buffer, where allocation succeeds and the narrowed call returns. A shift just below a power of two would instead land a truncated digit near `2^27`, whose allocation can itself trap and hide the narrowing.

```rust
// `add_shifted` with a small operand: apply_accumulator's small branch
// deposits 1 at digit 2^32 through `deposit_value`.
5 => accumulator.add_shifted(32 << 32, &Accumulator::from(1_u8)),
// `<<=` on a small value with one shift past 2^32 digits: shift_left's small
// branch deposits 1 at digit 2^32 through `deposit_value`.
6 => {
    accumulator += 1_u64;
    accumulator <<= 32_u64 << 32;
}
```

Use the next free case numbers at your base: main has cases 1 through 4, and the zero-shift fix branch may claim one. Give each case a comment naming its route, in the style of the existing cases.

Also correct the test's doc so it names what each case covers. In particular, case 2 is "a shifted stored accumulator" (true, and the stored-operand route for `add_shifted` as well), not the small-value route. State the routes in prose, not as a count.

## Assertion

Both cases produce `Outcome::Trapped(Trap::UnreachableCodeReached)`, inside the existing loop.

## Calibration evidence (verified; explore `5358b7f8` for the L3S, L3D, and L3W runs, before case 12 existed, and `5d5e3347` for the base, L3H, and L3S runs over cases 10 through 12; logs `S/r3/mut-*.log`, `S/r3/mut2-*.log`, `S/r3/mut-base.log`)

Each mutant was built into its own guest directory, run through `zz_l7_suanpan_add_shifted_landings`, and reverted, with `git diff` empty afterward:

| guest | cases 1-4 | case 10 (small operand) | case 11 (stored operand) | case 12 (small `<<=`) |
|---|---|---|---|---|
| base | trap | trap | trap | trap |
| L3S | trap | `Failed(WrongValue)` | trap | trap |
| L3H | trap | trap | trap | `Failed(WrongValue)` |
| L3D (narrowing the stored-operand route) | case 2 `Failed(WrongValue)`, others trap | trap | `Failed(WrongValue)` | not run |
| L3W (wrapping inside `add_digits`) | case 2 `Failed(WrongValue)`, others trap | trap | `Failed(WrongValue)` | not run |

So each new case catches exactly one mutant that every committed case misses. My stored-operand case 11 duplicates case 2's route, so it is not part of this brief.
