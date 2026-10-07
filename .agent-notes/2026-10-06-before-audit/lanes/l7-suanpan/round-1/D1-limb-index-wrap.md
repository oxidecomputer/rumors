# D1: a limb stream longer than `usize::MAX` wraps its landing position on 32-bit release builds

Severity: high (a silent wrong value, against a documented panic), 32-bit targets only.
Status: verified on ox-east-1 in the `wasm32-pins` executor (release, `overflow-checks = false`), explore branch `explore/l7-suanpan` at `d43fa15e`; root cause confirmed by the candidate fix described under "Fix note" (see the verification line there).

## Contract clause breached

`crates/suanpan/src/accumulator.rs:105-108` (`add_shifted_limbs`), and identically `:122-125` (`sub_shifted_limbs`):

> Panics if a nonzero contribution would land at or beyond `usize::MAX`,
> where the required retained span is unrepresentable.

and the value contract of the same methods (`accumulator.rs:90`, `:113`): "Add [subtract] little-endian 64-bit limbs multiplied by `2^shift`."

## Minimal reproduction

The explore branch adds case 4 to the guest's `suanpan_landing` (`crates/before/wasm32-pins/guest/src/checks.rs`):

```rust
4 => accumulator.add_shifted_limbs(0, core::iter::repeat_n(0u64, usize::MAX).chain([0, 1])),
```

On wasm32, `usize::MAX = 2^32 - 1`. The stream yields zero limbs at indices `0..=2^32 - 1` and a `1` at index `2^32`, whose digit position is `2 * 2^32 = 2^33`, unaddressable. Correct code panics.

Command (from the Mac):

```
/Users/oxide/.claude/skills/building-on-illumos/scripts/on-illumos.sh /Users/oxide/src/rumors-audit-l7-suanpan 'unset CARGO_TARGET_DIR; cd crates/before/wasm32-pins && cargo build --locked -p wasm32-pins-guest --release --target wasm32-unknown-unknown --target-dir ../../../target/wasm32-pins && cargo build --locked -p wasm32-pins-harness --tests --release && WASM32_PINS_GUEST_WASM=$PWD/../../../target/wasm32-pins/wasm32-unknown-unknown/release/wasm32_pins_guest.wasm cargo nextest run --locked --cargo-profile release --no-capture -E "test(/zz_l7_suanpan/)"'
```

Exit status 100. Output, verbatim:

```
L7 case5 (index 2^31) outcome: Trapped(UnreachableCodeReached)
L7 case4 (index 2^32) outcome: Failed(WrongValue)

thread 'zz_l7_suanpan_limb_index_past_usize_max' (2) panicked at harness/tests/pins.rs:151:5:
assertion `left == right` failed: a limb past 2^32 wrapped instead of panicking
  left: Failed(WrongValue)
 right: Trapped(UnreachableCodeReached)
```

`Failed(WrongValue)` is the guest's report that `add_shifted_limbs` returned normally. The run takes about 30 seconds of wall time on the box.

## Failure family

- Appears when a limb stream passed to `add_shifted_limbs` or `sub_shifted_limbs` yields more than `usize::MAX` limbs, every limb at indices `2^31 ..= 2^32 - 1` is zero (otherwise the correct panic fires first, since those land at `2^32` or beyond and the `u128` position is exact up to there), and some later limb is nonzero. That limb lands at `2 * (index mod 2^32) + shift / 32`, a small, addressable position, and the value silently gains a contribution at the wrong scale.
- Requires a 32-bit (or narrower) `usize` and a build with overflow checks off, which is Rust's default release profile. With overflow checks on, `Enumerate::next` panics with "attempt to add with overflow" one element earlier: still a panic, but not the documented one.
- Disappears on 64-bit targets in practice: the trigger needs `2^64` yielded limbs, infeasible work.
- The trigger costs the caller `2^32` iterator steps, about 30 seconds inside wasmtime, so it is a feasible input, not a cryptographic unreachability.
- The control (case 5: a `1` at index `2^31`) traps correctly, because index `2^31` has not wrapped.

## Root cause

`Digits::apply_limbs` (`crates/suanpan/src/accumulator/digits/add.rs:89-95`) takes `limb_index` from `Iterator::enumerate`, whose counter is a `usize` that wraps modulo `2^32` on these builds (`Enumerate::next` inherits the caller's overflow-check setting). The position arithmetic after it is exact in `u128` (`u128::from(digit_shift) + 2 * limb_index as u128`), but only for the index it is given. Every other position computation in the crate already stays in `u128` (`deposit_magnitude`, `add_digits` over a slice whose length cannot exceed `isize::MAX`), so this is the one place where the 32-bit landing guard can be bypassed.

## Fix preserves API and formats

Yes. The fix changes one private loop; no public signature, wire, or storage format moves.

## Fix note

Count the limb index in `u128` instead of `usize`:

```rust
for (limb_index, limb) in (0u128..).zip(limbs) {
    ...
    let position = u128::from(digit_shift) + 2 * limb_index;
```

`RangeFrom<u128>` cannot wrap within any feasible stream. An equivalent spelling keeps a `let mut position = u128::from(digit_shift);` advanced by 2 per limb. No other entry point shares the mechanism (checked: the remaining `enumerate` in suanpan runs over a slice, `add_digits`).

Verification of the root cause (applied as a reversible swap, run, reverted; `git diff` empty afterwards): with `(0u128..).zip(limbs)` and `2 * limb_index`, the same probe reports, verbatim (`S/wasm3-fix.log`):

```
        PASS [   0.832s] (1/2) wasm32-pins-harness::pins suanpan_rejects_unaddressable_digit_landings
L7 case5 (index 2^31) outcome: Trapped(UnreachableCodeReached)
L7 case4 (index 2^32) outcome: Trapped(UnreachableCodeReached)
L7 case6 (classified) outcome: Trapped(UnreachableCodeReached)
        PASS [  70.203s] (2/2) wasm32-pins-harness::pins zz_l7_suanpan_limb_index_past_usize_max
```

Case 6 is case 4 with the landing classified: on the base commit it returns `Failed(WrongBytes)`, meaning the accumulator afterwards reads exactly `(Greater, [1])`, the `1` having landed at digit 0.

## Test brief (for a demonstrator)

- Invariant: a limb stream whose first nonzero limb at or past index `2^31` lies past index `usize::MAX` still panics with the documented landing message on a 32-bit target; it never deposits at a wrapped position.
- Form: a unit case in the existing 32-bit executor (one specific input; the family is the wrap of one counter, so a single landing past `2^32` pins it).
- Where: `crates/before/wasm32-pins/guest/src/checks.rs`, function `suanpan_landing`, a new arm `4 => accumulator.add_shifted_limbs(0, core::iter::repeat_n(0u64, usize::MAX).chain([0, 1])),`; and `crates/before/wasm32-pins/harness/tests/pins.rs`, test `suanpan_rejects_unaddressable_digit_landings`, widening its loop to `1..=4` and adding a sentence to its doc comment: "Case 4 yields more than `usize::MAX` limbs, so a landing computed from a wrapped limb counter would fall near the start of the buffer."
- Construction: the stream is built lazily by `repeat_n` and `chain`, so the guest allocates nothing; `repeat_n(0u64, usize::MAX)` yields indices `0..=2^32 - 2`, the chained `0` sits at `2^32 - 1`, and the `1` at `2^32`.
- Assertion: the existing `assert_eq!(run(Check::SuanpanLanding, case, 0), Outcome::Trapped(Trap::UnreachableCodeReached), "landing case {case} returned instead of panicking")`.
- Expected failure on the base commit: `landing case 4 returned instead of panicking` with `left: Failed(WrongValue)`, `right: Trapped(UnreachableCodeReached)`.
- Cost: about 20 seconds of wasmtime time for the case, inside nextest's 180-second limit (the explore run measured 30.8 s for a test running cases 4 and 5).
- Gate leg: `just wasm32-pins` (part of the gate's wasm leg).
