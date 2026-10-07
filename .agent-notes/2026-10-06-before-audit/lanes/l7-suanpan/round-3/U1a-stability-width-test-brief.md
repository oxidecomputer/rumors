<!-- CAVEAT LECTOR: written by Claude (Opus 5.5), auditor-l7, round 3. -->

# U1-a: `cmp_zero_stable_under` skips its scan on 32-bit for widths above `32 * 2^32` bits

Kind: defect record, test brief (demonstrator), and fix note. Severity: low.

I still judge this a defect. The answer agrees on both widths, but the accumulator's stored form afterward does not, and the stored form is observable through public methods (`stored_digit_count`, `stored_bits`) and through every later cost.

## Defect record

**Contract clauses breached.**

- The audit's pointer-width clause (`briefs/common.md`, "The contract you audit against"): "Both crates should behave identically whatever the width of `usize`. ... Report it as a defect where behavior differs by target."
- The method's rustdoc, `crates/suanpan/src/accumulator.rs:258-259`: "`None` makes no claim about the answer: the smaller operand may or may not change it. The scan nevertheless compacts what it reads." On 32-bit the scan never runs for these widths.

**Mechanism.** `crates/suanpan/src/accumulator.rs:285-286` narrows the adjustment's top digit index to `usize` before scanning:

```rust
let adjustment_digits = bits.div_ceil(u64::from(DIGIT_BITS)).max(1);
let adjustment_high = usize::try_from(adjustment_digits - 1).ok()?;
```

On 64-bit the conversion succeeds for every `u64`, so the call always reaches `Digits::cmp_zero_stable_above` (`crates/suanpan/src/accumulator/digits/sign.rs:43`), whose `compact_until_order_known` compacts the top of the stored form before returning `None`. On 32-bit, every `bits > 32 * 2^32` returns `None` at the `?`, with the stored form untouched.

**Reproduction (verified at `4fd78232`, explore branch).** The explore guest's case builds the value 5 with a cancelling top, 11 stored digits, and asserts one stored digit after `cmp_zero_stable_under(u64::MAX)`:

```
/Users/oxide/.claude/skills/building-on-illumos/scripts/on-illumos.sh /Users/oxide/src/rumors-audit-l7-suanpan 'unset CARGO_TARGET_DIR; export NEXTEST_TEST_THREADS=24; cd crates/before/wasm32-pins && cargo build --locked -p wasm32-pins-guest --release --target wasm32-unknown-unknown --target-dir ../../../target/wasm32-pins && cargo build --locked -p wasm32-pins-harness --tests --release && WASM32_PINS_GUEST_WASM=$PWD/../../../target/wasm32-pins/wasm32-unknown-unknown/release/wasm32_pins_guest.wasm cargo nextest run --locked --cargo-profile release --no-capture --run-ignored all -E "test(zz_l7_suanpan_stability_width_compacts)"'
```

Exit status nonzero; the assertion, verbatim:

```
L7 stability huge width (case 9): Failed(WrongLength)

thread 'zz_l7_suanpan_stability_width_compacts' (2) panicked at harness/tests/pins.rs:218:5:
assertion `left == right` failed
  left: Failed(WrongLength)
 right: Passed
```

The native counterpart, `l7_probe_stability_huge_width_compacts` (same construction, 64-bit host), passes: one stored digit remains.

**Failure family.** Every accumulator in the digit representation whose top is not yet compacted, queried with `bits > 32 * 2^32` (that is, `adjustment_digits - 1 >= 2^32`; at exactly `32 * 2^32` the top index is `usize::MAX`, which converts, as the demonstrator measured on the box), on a target with a 32-bit `usize`. It disappears for narrower `bits`, on 64-bit targets, and in the small representation (that branch never converts to `usize`). The answer itself never differs: no 32-bit accumulator can hold enough digits to dominate such a width, so both targets answer `None`.

**Fix preserves** the public API and the wire and storage formats. It changes 32-bit behavior only, toward the 64-bit behavior.

## Test brief (for a demonstrator)

**Invariant.** `cmp_zero_stable_under` compacts the stored form identically on every pointer width, whatever the adjustment width: after `cmp_zero_stable_under(u64::MAX)` on the value 5 stored with a cancelling top, exactly one digit remains stored.

**Form.** Two unit tests on one specific input, because the defect concerns the conversion boundary and the input that crosses it, not a family: a 32-bit pin that fails on the base commit, and a native witness that states the reference behavior and passes on 64-bit hosts.

**Construction (both tests).** Build the value 5 with an 11-digit cancelling top. Every step stays exact, and the one-word path deposits `2^32` at a single digit without splitting it:

```rust
let mut accumulator = Accumulator::new();
accumulator += 5_u64;
accumulator.add_shifted_limbs(32 * 10, [1, 0]); // digit 10 holds 1
accumulator.sub_shifted_limbs(32 * 9, [1 << 32]); // digit 9 holds -2^32
// Value 5; digits 9 and 10 cancel, so 11 digits are stored.
```

Assert the premise before the call: `accumulator.stored_digit_count() == 11`. A premise failure means the construction no longer builds a cancelling top, and the test would pass vacuously.

**Assertion.** `accumulator.cmp_zero_stable_under(u64::MAX) == None`, then `accumulator.stored_digit_count() == 1`, then the value still reads back as 5 (`i32::try_from(accumulator) == Ok(5)` or the crate's readout helper).

**Where it belongs.**

- 32-bit pin: the `wasm32-pins` executor. Add a check to `crates/before/wasm32-pins/protocol/src/lib.rs` (`Check`, with its `TryFrom<u32>` arm), its guest function in `crates/before/wasm32-pins/guest/src/checks.rs`, and a harness test in `crates/before/wasm32-pins/harness/tests/pins.rs` that asserts `Outcome::Passed`. Do not fold it into `SuanpanLanding`: that check's contract is "correct code panics, any return fails", and this case must return normally. The guest maps the three outcomes to distinct failures, as the explore case does: `(None, 1)` passes, `(None, _)` is `Failure::WrongLength`, and `(Some(_), _)` is `Failure::WrongValue`.
- Native witness: `crates/suanpan/src/accumulator/tests/witnesses.rs`, beside `maximum_adjustment_width_never_decides`, which already asks the same width and checks only the answer.

**Failure on the base commit.** The 32-bit pin fails with the harness assertion above, `left: Failed(WrongLength)`, `right: Passed`. The native witness passes on the 64-bit box (it records the reference behavior; it is not the demonstration).

**Doc comments.** The pin's doc states the invariant ("compacts identically on every pointer width") and why the width matters (`32 * 2^32 + 1` bits is the first adjustment whose top digit index exceeds a 32-bit `usize`). Name no explore case numbers.

## Fix note (for a fixer)

**Root cause.** A `u64` digit index is narrowed to `usize` only to compare it with a buffer index; the narrowing fails exactly where the comparison's answer is already known (no buffer index can reach it), and the `?` turns that failure into an early return that skips the scan.

**Candidate repair.** Keep the threshold in `u64`:

- `Digits::cmp_zero_stable_above(&mut self, adjustment_high: u64)` compares `index_u64 >= adjustment_high.saturating_add(2)`, where `index_u64` widens the buffer index losslessly (`u64::try_from(index)`, whose failure no target with at most 64-bit pointers can reach; give the `expect` its one-line proof).
- `cmp_zero_stable_under` passes `adjustment_digits - 1` directly and drops the `usize::try_from(...).ok()?` line.
- The existing saturation comment in `sign.rs:45-47` stays true: no `u64` adjustment derived from `bits.div_ceil(32)` can saturate, and the saturating form keeps the guard if a later caller passes a wider bound.

The repair is verified on wasm32 and natively; see "Fix verification" below.

**Other entry points with the mechanism.** None in suanpan: the round-2 width sweep (`round-2/U1-usize-width-sweep.md`) found every other `usize` in suanpan to be a buffer index or a held count. In `before`, `Rank::accumulate`'s reservation hint (`crates/before/src/rank.rs`, `usize::try_from(widest / 32 + 2)`) differs by width in the same way; it moves with the owner's ruling on `reserve_digits`.

## Fix verification (verified, explore `5d5e3347` with the repair applied as three reversible swaps, then reverted with `git diff` empty)

- The repair: `adjustment_high` computed as `adjustment_digits - 1` (`u64`), `cmp_zero_stable_above(&mut self, adjustment_high: u64)`, and the comparison `index as u64 >= adjustment_high.saturating_add(2)`. The final branch should use the lossless conversion described above instead of `as`.
- wasm32: the explore case reports `L7 stability huge width (case 9): Passed`, and `suanpan_rejects_unaddressable_digit_landings` still passes (`S/r3/u1f.log`).
- Native: `cargo nextest run -p suanpan --features touch-meter` passes, 72 tests run, 72 passed, every committed witness and touch pin included.
- Without the repair, case 9 still fails with `Failed(WrongLength)` at `5358b7f8`, after merging main's limb-index fix (`S/r3/leads2.log`).

## Reach, established in review

No `before` operation reaches this defect. `before` calls the stability query
only from the range-minima boundary and anchor code and from the causal
query's place filter, each passing 64 or another accumulator's
`stored_bits()`. On wasm32, digits are `Vec<i64>`, so `stored_bits()` stays
below `32 * 2^28`, far under the boundary. No `rumors` code calls the query.
Only a direct suanpan caller passing an arbitrary `u64` width can reach it.
The fixer established this and the reviewer confirmed it by reading the
callers.
