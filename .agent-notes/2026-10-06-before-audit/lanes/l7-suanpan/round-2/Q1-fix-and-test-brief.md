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

## Owner's ruling (question 121, 2026-10-09)

`add_shifted` and `sub_shifted` take their operand as `&mut Accumulator` and
decide zero with the compacting `cmp_zero`, like the crate's other operations
that read by normalizing. The read-only `value_is_zero` is deleted, so no
non-compacting twin of `cmp_zero` remains for a maintainer to misuse. `+=`,
`-=`, and `Sum` keep their borrowed operands and lose the zero check: at shift
0 a cancelled zero can raise the receiver's width to the operand's, which the
ruling on question 41 accepts. In the same review the owner asked that shift
counts accept every nonnegative value of every supported integer type: zero
shifts by any count, and a nonzero value panics past `u64::MAX`.

## Owner's ruling (question 122, 2026-10-09)

Every `Accumulator` operation is uniformly amortized, so a caller never has
to think about it. The design rule, in the owner's words: "We don't want to
fully normalize when we don't have to; we just want to normalize whatever we
actually touch, so we don't have to re-do it, just as `cmp_zero` does."
Three groups currently rescan cancellation they do not compact, and all
three are fixed:

1. `signed_magnitude` and `scaled_signed_magnitude` take `&mut self` and
   write back what they read; `before`'s wrappers pass `&mut`.
2. `TryFrom<Accumulator>` for the primitive integers compacts what it reads,
   including on the failure path, which returns the same value.
3. The borrowed-operand arithmetic traits (`AddAssign`, `SubAssign`, `Add`,
   `Sub`, `Sum`) take `&mut Accumulator` operands in place of
   `&Accumulator`, and compact what they read of the operand.

The work goes on its own branch stacked on #48, after #48's question-121
builder reports.

## Owner's ruling (question 123, 2026-10-09)

With the shifted merges compacting their operand, `before`'s `lone_freeze`
meter family no longer crosses the freeze threshold: its ten-digit drift
counted a redundant stored top digit that compaction removes. The family's
plateau is widened so it crosses the threshold in compacted terms, with the
derivation stated in its doc. The ceilings stay. The owner's goal: "ensure
that the measurements continue to mean something."

## Owner's ruling (question 124, 2026-10-09)

The `wide_arming` meter families (WA, HW) promise one deferral but never
defer at their tested widths: their `w >= 10` bound should be `w >= 19`.
A follow-up branch stacked on #48 derives the family's minimum `w` from the
freeze trigger, as #48 derives `FREEZE_DRIFT_BITS`, adds WA and HW to #48's
freeze-liveness test with their deferral counts, and accepts the board
movement that follows.

## Owner's rulings (question 125, 2026-10-09)

The integrator restructuring in `fix/suanpan-uniform-amortization`'s commit
`b9b2f2f1` was the owner's direct instruction to the builder, and stands.
The seam-stop difference band (`STOPPING_DIFF_BAND`) is re-pinned to the
new reading by its own convention, (9_742, 16_237): the rise is the
constant deciding touch each read and operand now pays, linear in the hop
count, so the band still detects what it exists for.

## Owner's ruling (#122's API, 2026-10-09)

`Accumulator` should mostly behave as a standard numeric type that is
curiously `!Clone + !Sync`, with every operation fully amortized. Reads and
operands go back to `&self`/`&Accumulator` through interior mutability
(`RefCell`), and every operation taking two accumulators handles the
same-cell case by construction. In: `Add`, `Sub`, `Neg`, `Sum`, `Shl` and
their assign forms over every operand form; `PartialEq`, `Eq`,
`PartialOrd`, `Ord`, also against primitives; a normalizing `Hash`
consistent with `Eq`; standard sign queries replacing `cmp_zero`; and
`num-bigint` interop behind an optional feature (conversions both ways,
`TryFrom<&Accumulator> for BigUint`, and `BigInt`/`BigUint` operands), so
`before` can drop its conversion code. Out: multiplication, division, and
right shift (a floor division, whose real use is a rounding readout).
`Debug`'s doc says its output can change when the value does not. Inherent
methods made redundant are removed after the owner approves each.
Added the same day: `Display` and a value bit length in the style of
`BigUint::bits` are in #122 too.


## Owner's rulings (#122 review, 2026-10-09)

The value bit length leaves #122: nothing calls it, and its first form
re-paid the whole width near powers of two. An amortized version comes back
with its own design if a caller needs one. Primitive-left arithmetic
(`5 + acc`) and borrowed primitive operands (`acc + &5`, `Sum<&u64>`) are
in, as `num-bigint` offers them. Primitive-left comparisons stay out: an
impl of `PartialOrd` or `PartialEq` on a primitive makes the primitive's
comparisons ambiguous for inference in every crate that links `suanpan`,
so code such as `(a..b).contains(&arc_u64)` stops compiling downstream.
Public rustdoc states what a user of the crate needs to use the API
effectively; mechanism belongs in private docs.

## Owner's rulings (#122 fixes, 2026-10-09)

`Accumulator::normalize` is removed: every read compacts what it touches.
`stored_digit_count` stays; its only production callers after #122 are the
measure folds' freeze and weight policies, which use stored width as the
work a value costs, and whether to restate them in bits is a separate
question. `before` reads `ScaledWidth`'s limbs directly from
`scaled_signed_magnitude`, so its private big-integer converter goes, and
`digit_len` moves into `version::measure`, leaving no accumulator-helper
module in `before`. `Debug` honors hex flags only if that is small and
clean. Two `before` simplifications join #122: the conversion guards in
`tick.rs` and `boundary.rs` go, because primitive `TryFrom` is now
amortized, and `place/filter.rs` and `boundary.rs` use `suanpan`'s `Ord`
instead of their own copies of its comparison protocol. Release panics
reachable only by programmer error stand in place of the `debug_assert`s
they replace.

## Owner's rulings (#122 base independence, 2026-10-09)

`suanpan`'s public API promises nothing about its numeric base and exposes
nothing tied to it: users see only `u64` limbs, which are independent of
the representation. `stored_digit_count` leaves the public API, and
`stored_bits` is the working-width measure. The scaled readout reports
its factored zero span as a count of 64-bit limbs, so a misaligned scale
is unrepresentable. `before` states its width policies in bits: the
conversion guards, the 256-bit freeze allowance, and the deferred merge
weight. Both conversion guards stay, because each skips a constant-cost
stability query on a wide value it keeps without reading;
`RangeMinima` moves boundary buffers and never reads them on push or pop.
`suanpan` names its limb readout as a public type that derefs to
`[u64]`, so `before` can keep it without copying.

## Owner's rulings (#122 comparison and subtraction, 2026-10-09)

`suanpan` gains `abs_diff`, which consumes two accumulators and returns
their order and the magnitude of their difference, reusing the wider
buffer. It fuses comparison and subtraction: far apart, the stability
query decides; close, one in-place subtraction followed by a sign read
decides. `range_minima`'s `lower_wide` uses it in place of its own
pre-checks, which `Ord` could not replace, because `Ord` returns only an
order and would subtract twice when the values are close. `Ord` also asks
the stability query once at a lead of exactly two digits, falling back to
subtraction on refusal, so it still terminates. `place/filter.rs` uses
`Ord` in place of its copy of the comparison protocol.

## Owner's ruling (rank integrator tiers, 2026-10-09)

The rank integrator keeps its `parked` tier. A prototype
(`explore/rank-single-tier`, kept as evidence) showed that deferring every
freeze stays within O(M(n)) but, on families that freeze often with narrow
deltas, adds a log factor to linear work and raises heap from 0.04 to 4.66
bytes per input byte (`zz-nf-d8`). Eager freezing raises rank's heap to
115.7 bytes per input byte without `parked` and 41.5 with it; the 8-digit
allowance keeps every family under 6.9. O(M(n)) is the contract: exact
integration can encode an integer product. The deferred tree's n log n
linear work on `arming-train` is within it, recorded as an observation.
Wanted later: a board family that removing `parked` would fail, the module
doc stating what `parked` buys, and any clarity rework that keeps the
integrator's efficiency.

## Owner's ruling (#122 scaled readout, 2026-10-10)

The scaled readout reports its scale in bits and promises nothing about its
granularity, superseding the earlier ruling that it count whole 64-bit
limbs. Aligning the scale to limbs added a low zero digit whenever a width
started at an odd digit, which raised 81 board rows in the rank
integrator's width path (`freeze-parade` heap by about 40%). `suanpan`
packs limbs from the lowest written digit; `before`'s `SparseWidth` places
limbs at any bit offset. `abs_diff` is accepted as built: it subtracts in
place, reads the sign, and negates, at O(narrower width) for nonnegative
operands and up to the result's own width when the wider operand is
negative.

Documentation names committed constants instead of quoting their values,
so it cannot drift from them (owner, 2026-10-10).

## Owner's rulings (#122 review process, 2026-10-10)

The coordinator checks every finding in the three #122 reviews against its
disposition and the diff, and brings the owner only the findings that need
judgment, in place of a full walk-through. Prose revision is deferred for
#122 and `fix/min-ticks-transient-heap`: once both have landed, one prose
pass covers everything the two branches touched, instead of a pass on each
branch.

## Owner's rulings (#122 review findings, 2026-10-20)

Every finding's disposition at `e0956b25` holds. `interop.rs` moves to the
`suanpan` crate root, accepted on a compile check alone. The `regions.rs`
binding stays. `Debug` copying the stored form and formatting the copy
through the caller's formatter is accepted. The four `dense-suffix` query
rows rising by a constant 2 or 4 touches, from `Ord` in the query filter,
are accepted. `Hash` writing a `usize` slice length, so its bytes differ
between 32- and 64-bit targets as `num-bigint`'s do, is accepted.
`Display`'s committed format specs stay as they are. The
`add_shifted_limbs` and `sub_shifted_limbs` docs share one outline and one
`# Panics` wording, done in #122 rather than the joint prose pass.
