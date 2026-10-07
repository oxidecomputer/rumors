# S1: state the readout's one-digit high part directly instead of looping over it

Kind: simplification brief (self-contained, behavior-preserving, touch-preserving).

## Current code

`crates/suanpan/src/accumulator/digits/read.rs:86-90` (negative branch) and `:95-99` (nonnegative branch) each drain the final carry with a loop:

```rust
let mut high = (-carry) as u128 - u128::from(low_nonzero);
while high > 0 {
    touch(1);
    collected.push((high & u128::from(DIGIT_MASK)) as u32);
    high >>= DIGIT_BITS;
}
```

The module doc (`read.rs:3-7`) and the loop's own `debug_assert!((-3..=2).contains(&carry))` (`:66`) establish that the final carry lies in `[-3, 2]`. So `high` is at most 3, and each loop runs at most once.

## Proposed structure

Replace each loop by one conditional push that states the bound where it is relied on:

```rust
let high = (-carry) as u32 - u32::from(low_nonzero);
if high > 0 {
    touch(1);
    collected.push(high);
}
```

and, in the other branch, `let high = u32::try_from(carry).expect("the final carry lies in [-3, 2]: nonnegative here, at most 2");` followed by the same conditional push. A one-line comment at each site cites the `[-3, 2]` bound proved in the module doc.

## Why the result is more obviously correct

A loop that shifts by `DIGIT_BITS` invites the reader to wonder how many digits the high part can occupy; the answer, one, lives in a doc comment twenty lines up. The direct form makes the one-digit claim structural and puts the proof next to it. Touches are unchanged (one per nonzero high part), so every exact pin in `metered.rs` holds.

## Coverage

The existing `negative_limb_values_read_back_exactly`, `carry_tie_streams_match_the_oracle`, `flush_right_carry_tie_converts_exactly`, `redundant_zero_reads_nonzero_until_collapsed`, and the surface suite all read back values whose final carry is nonzero in both signs. In my calibration, mutating the complement carry (`complement_carry = 0`) failed 32 existing tests. The metered pins (`held_width_rows_cost_the_held_digits` and others) protect the touch count.
