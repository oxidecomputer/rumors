# Q1 (now a defect): shifting a zero must be total and constant-space whatever its stored form

Owner ruling (relayed by the coordinator, round 2): shifting zero, and adding a zero-valued operand at a shift, must be total and constant-space whatever its stored form. Evidence and contract text: `deliverables/Q1-zero-shift-representation.md`.

Severity: low (a panic on 32-bit and `O(shift)` retained space on every target, for the value zero in a redundant form).

## Contract clause

- `crates/suanpan/src/accumulator/operators.rs:359-362` (`Shl`): "Panics if the count is negative, exceeds `u64::MAX`, or the result needs an unrepresentable working width."
- The owner's ruling above.

## Reproduction (verified)

- 64-bit (`l7_probe_zero_shift_history`, `S/run3.log`): a zero stored as digits `[-2^32, 1]` keeps `stored_digit_count` 1,048,578 after `<<= 32 << 20`, and after `<<`, and as the operand of `add_shifted(32 << 20, ..)`; a known zero keeps 1.
- wasm32 (guest case 7, `S/wasm5-zeroshift.log`): the same zero shifted by `32 * (2^32 - 1)` traps (`Trapped(UnreachableCodeReached)`); the known zero returns (`Passed`).

## Root cause

`shift_left` (`operators.rs:381-403`) returns early only on `is_known_zero()`, a representation test. Otherwise it re-deposits every stored digit at the shifted position. `Accumulator::apply_accumulator` (`crates/suanpan/src/accumulator/operand.rs:87-107`) does the same for a digit-stored operand. A mathematically zero value with nonzero stored digits therefore deposits them at `shift / 32` and beyond: the receiver's buffer grows to the landing position (`O(shift)` space), and on 32-bit a landing at or past `usize::MAX` panics.

## Fix note (preserves API and formats)

1. **`shift_left`** (takes `&mut self`): replace the `is_known_zero()` early return by `if shift == 0 || self.cmp_zero() == Ordering::Equal { return; }`.
   - The cost is amortized `O(log(W + 1))`, inside the shift's documented `O(A log(S + 1) + S)`.
   - It also compacts any cancelled top before the copy, so the result's working width is at most two digits above the value's own, which makes `Shl`'s "the result needs" wording exact.
   - `shift_left` then deposits through an internal path that skips the operand check in item 2, which it has just made redundant.
2. **`apply_accumulator`, digit-stored operand** (borrowed, so it cannot be compacted): when the deposit would extend the receiver's buffer, meaning `shift / 32 + other.stored_digit_count()` exceeds the receiver's retained digit length, first decide whether the operand's value is zero. Use a read-only top-down scan with the comparison's rule (`|partial| >= 3` decides nonzero; reaching digit 0 with partial zero means zero). Return without depositing when it is zero.
   - The scan reads at most the operand's stored digits, inside the documented `O(A log(W + 1) + G)`.
   - Restricting it to growing deposits keeps it off the steady-state paths, for example `before`'s per-region `total.add_shifted` in integration, whose receiver usually retains its buffer.
   - It is not amortized: a repeated `add_shifted` of the same uncompacted operand rescans its cancelling prefix each time, still `O(A)` per call.
3. Restate the `# Panics` text of `ShlAssign`, `Shl`, `add_shifted`, and `sub_shifted` (`operators.rs:341-344`, `:359-362`; `accumulator.rs:143-146`, `:159-162`) in value terms: a nonzero result whose lowest required digit position is at or beyond `usize::MAX`. The current "contribution" wording names an internal quantity the reader cannot see.

Limb streams need no change: zero limbs deposit nothing (`add.rs:96-101`).

**Pin movement (predicted from the code, not measured; the fixer must measure at the parent):**
- `held_width_rows_cost_the_held_digits`: the `<<= 32` rows rise from `2d` to `2d + 1` (one top-digit read decides), and `<<= 32_000` from 128 to 129.
- `accumulator_operand_rows_cost_the_operand`: the `add_shifted` and `sub_shifted` rows at shifts 32,000 and 64,000 rise from 4 to 6, because the operand `[1, 1]` needs two reads to decide.
- `before`'s board and meter pins may move wherever an operation's first `add_shifted` grows its receiver.
- Under the audit's pinned-instrument rule, any rise stops the fixer for the owner. Since the owner ruled that this behavior must change, the coordinator should obtain the owner's approval of the measured rises along with the fix.

## Test brief (for a demonstrator)

- **Invariant:** for every mathematically zero accumulator `z`, whatever its stored digits, and every shift `s`, the expressions `z <<= s`, `z << s`, `r.add_shifted(s, &z)`, and `r.sub_shifted(s, &z)` complete; they leave `z`'s value zero and `r`'s value unchanged; and they retain at most a constant amount of digit storage beyond what the receiver held before.
- **Form:** a property over a generated family of redundant zeros (place it in `crates/suanpan/src/accumulator/tests/representation.rs`, since the behavior under test is representation-dependence), plus one 32-bit unit case in the `wasm32-pins` executor.
- **Generator** (builds zeros directly, never filters):
  - choose `k` in `1..=40`, deposit `+1` at digit `k` with `add_shifted_limbs(32 * k, [1, 0])` (two limbs, so the digit form is active), then deposit `-2^32` at digit `k - 1` with `-= 1_u64 << 32` when `k = 1`, or with `sub_shifted_limbs(32 * (k - 1), [1 << 32])` when `k > 1`. Neither deposit carries, so the stored digits are `[-2^32 at k - 1, 1 at k]`.
  - optionally add a random wide value `x` and subtract it again through a different entry point (limbs one way, a primitive or an accumulator operand the other), so the stored form carries further cancelling digits
  - assert the premise `!z.is_known_zero()` holds for the first construction
  - shifts: draw from `{0, 1, 31, 32, 33, 64, 32 * 1_000, 32 << 20}` plus random values up to `32 << 20`
  - receivers `r`: a fresh accumulator, a small value, and a wide value
- **Assertions:**
  - `z.cmp_zero() == Ordering::Equal`, and `r`'s value equals its value before the operation (the existing `assert_value` helper)
  - `stored_digit_count()` of the result is 1 for `z << s` and `z <<= s`, and does not exceed the receiver's count before the operation for `add_shifted` and `sub_shifted`
  - the retained digit buffer did not grow by more than a constant (two positions). This needs a test-only accessor for the buffer length, `Digits::retained_len()`; add it under `#[cfg(test)]`.
- **Expected failure on the base commit:** with `k = 1` and `s = 32 << 20`, the count assertion fails with `left: 1048578`, `right: 1` (the probe's reading).
- **32-bit unit case:** a new arm in the guest's check dispatch builds `[-2^32, 1]` and applies `<<= 32 * u64::from(u32::MAX)`, returning `Ok(())` when `cmp_zero()` is `Equal`. The harness asserts `Outcome::Passed`; on the base commit it reports `Trapped(UnreachableCodeReached)`. A known-zero control case passes on both.
- **Runtime:** the native property is milliseconds per case at the largest shift (one `2^20`-digit buffer on the base commit); the guest case is under a second.

## Owner's ruling on pin movement

Approved in advance: the fix may raise exact touch pins by a constant per
operation, caused by the zero check. The fixer measures each rise at the
parent, re-pins it with an annotation naming the zero check, and stops if
anything rises by more than a constant or changes a growth rate.
