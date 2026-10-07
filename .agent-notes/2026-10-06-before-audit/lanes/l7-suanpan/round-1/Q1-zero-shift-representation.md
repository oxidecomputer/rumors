# Q1: should shifting the value zero be total and constant-space whatever its stored form?

Type: question about intended semantics, with a conditional defect.
Status: behavior verified on ox-east-1, native 64-bit (`S/run3.log`) and wasm32 (`S/wasm5-zeroshift.log`).

## Evidence

A zero value stored redundantly as digits `[-2^32, 1]` (built by `add_shifted_limbs(32, [1, 0])` then `-= 1_u64 << 32`; `is_known_zero()` is `false`, `signed_magnitude()` is `(Equal, [])`) behaves differently from a known zero under every shift entry point:

- 64-bit, shift `2^25` bits (`l7_probe_zero_shift_history`): known zero `<<=` keeps `stored_digit_count` 1; the redundant zero after `<<=`, `<<`, or as the operand of `add_shifted` keeps 1,048,578 stored digits (about 8 MiB) for the value zero. `normalize()` first restores 1.
- wasm32, shift `32 * (2^32 - 1)` (guest cases 7 and 8): the redundant zero traps (`Trapped(UnreachableCodeReached)`, the documented landing panic); the known zero returns (`Passed`).
- 64-bit with a shift near `u64::MAX` (inferred from `add_at`'s `resize`, not run because it would abort the process): the redundant zero attempts an allocation of about `2^62` bytes and aborts through `handle_alloc_error`.

## The contract text is split

- `Shl` (`crates/suanpan/src/accumulator/operators.rs:359-362`): "Panics if the count is negative, exceeds `u64::MAX`, or the result needs an unrepresentable working width." The result here is zero, whose working width is one digit, yet the operation panics.
- `ShlAssign` (`operators.rs:341-344`): "or a nonzero contribution cannot fit an addressable stored position". This is literally satisfied, because the stored digits are nonzero, but "contribution" is never defined publicly and a user holding the value zero cannot observe it.
- `add_shifted` and `sub_shifted` (`accumulator.rs:143-146`, `159-162`): the same "nonzero contribution" wording.
- The crate page prices growth `G` by working width and warns that working width follows history, so the space use is arguably within the cost model, but not the panic under `Shl`'s wording.

## The question

Is `0 << s` (and adding a zero-valued accumulator at shift `s`) meant to be total and `O(1)`-space regardless of history?

- If yes, this is a low-severity defect: `Shl`'s `# Panics` is breached on 32-bit, and both widths retain or attempt `O(s)` space for a zero result.
- If no, then `Shl`'s `# Panics` text should adopt `ShlAssign`'s representation wording, and the public docs need to define "contribution" or say that a shift's panic and space depend on the stored form.

## Candidate repair (if yes)

`shift_left` already takes `&mut self`; calling `self.cmp_zero()` first and returning on `Equal` costs amortized `O(log(W + 1))`, inside the shift's documented `O(A log(S + 1) + S)`. Compacting first also caps every shifted result's working width at the value's width plus two digits, so `Shl`'s "result needs" wording becomes exact for nonzero values too. For `add_shifted(&other)` the operand is borrowed and cannot be compacted, but a read-only top-down scan of its digits can establish zero within the operation's `O(A)` budget before any deposit. Both changes preserve the public API and formats.

## Owner's ruling

Yes. Shifting a zero accumulator, or adding a zero-valued operand at a
shift, is total and constant-space whatever the zero's stored form.
