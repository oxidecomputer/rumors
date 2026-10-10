<!-- CAVEAT LECTOR: the structure review of eedff98d, written by a Claude (Opus 5.5) agent and archived verbatim by the audit coordinator. Its claims are unverified unless the README says otherwise. -->

# review-122-structure: structure review of `fix/suanpan-uniform-amortization` (#122)

## Verdict

**Nothing in the structure is wrong enough to block the merge. I recommend adopting three behavior-preserving cleanups before it lands, because two of them finish the branch's own migration.**

The rework itself reached a sound shape:
- `Accumulator` owns the cell, the sharing rules, and the public docs. `Value` owns the representation and every algorithm.
- The two-cell case is handled at its one site (`Ord::cmp`, via `ptr::eq`).
- The operator matrix is easy to audit.
- `before` lost its `BigIntAccumulator` trait cleanly.

Two things are incomplete:
- **Migration residue in `before`.** The branch's own changes made some code dead or misleading, and it is still there: redundant pre-reads in `tick.rs`, and `&mut` parameters that only read.
- **The borrow discipline holds by convention, not by structure.** The cell accessors return guards. How long a borrow lasts then depends on Rust's temporary-lifetime rules at each call site, and I showed one ordinary-looking refactor that would panic at runtime.

I prototyped the three most valuable groups in my slot. Each one compiled, passed clippy with warnings denied, and passed its suites; then I reverted it. The slot is clean at `eedff98d` (`git diff` empty, no untracked files). The diffs are saved in my scratch directory, `/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/review-122-structure/`:
- `groupA.diff` (490 lines)
- `groupB.diff` (136 lines)
- `groupC.diff` (644 lines, mostly a mechanical test rename)

## Candidates, ranked by value

### 1. Scope every cell borrow to a closure (adoptable now; prototyped as group C)

**Current form.** In `crates/suanpan/src/accumulator.rs:125-132`:
- `read()` returns a `RefMut<'_, Value>`, and `peek()` returns a `Ref<'_, Value>`.
- The field doc (`:84-89`) promises that no borrow is held while another accumulator method or caller code runs.
- That promise holds today only because every call site happens to drop its guard in time:
  - `compare.rs:44` and `:156`
  - `operators.rs:167` and `:186`
  - `format.rs:58`
  - accumulator.rs's own `read()`/`peek()` delegations

**The hazard, verified.** In the guard form, a guard in an `if let` or `match` scrutinee stays alive through the whole block. I compiled a small model of `read()` on the box with rustc 1.97.1 under both editions; the output was identical for 2021 and 2024:
```
if let on a guard method: PANICS (guard held through block)
match on a guard method: PANICS (guard held through block)
let binding, then use: ok
if let on a guard-free accessor: ok
```
Edition 2024 drops scrutinee temporaries only before the `else` branch, not before the body.

This is not hypothetical. The natural way to remove the duplication in candidate 4 is `if let Some(s) = self.read().addressable_shift(n) { shifted.add_shifted(s, self) }`. In guard form, that code panics with an already-borrowed error.

**Proposed form.** `read` and `peek` take a closure:
- `fn read<R>(&self, f: impl FnOnce(&mut Value) -> R) -> R { f(&mut self.value.borrow_mut()) }`, and the same shape for `peek` with `&Value`.
- Production sites become one-to-one delegations, for example `self.read(Value::cmp_zero)`, `self.peek(Value::stored_bits)`, and `self.read(|a| other.read(|b| compare(a, b)))`.
- The field doc can then state a structural fact: "Only `read` and `peek` borrow the cell, each for one call of a closure that runs `Value` code."
- Tests keep a `#[cfg(test)] fn stored(&self) -> Ref<'_, Value>`. The rest of the test diff is the mechanical rename `.peek()` to `.stored()`.

Production diff (from `groupC.diff`; I've trimmed doc-only context):
```diff
-use core::cell::{Ref, RefCell, RefMut};
+use core::cell::RefCell;
@@ field doc
-    /// No method holds a borrow of this cell while it calls back into another
-    /// accumulator method or into caller code. An operation on two
-    /// accumulators borrows both cells only after it has checked that they
-    /// differ.
+    /// Only [`read`](Self::read) and [`peek`](Self::peek) borrow the cell,
+    /// each for one call of a closure that runs `Value` code, so no borrow
+    /// outlives its call or is held while another accumulator method or
+    /// caller code runs. An operation on two accumulators nests their borrows
+    /// only after it has checked that they differ.
-        Accumulator::from_value(self.peek().clone())
+        Accumulator::from_value(self.peek(Value::clone))
-    /// Borrow the value for a read that may compact it.
+    /// Run `f` on the value for a read that may compact it.
@@
-    fn read(&self) -> RefMut<'_, Value> {
-        self.value.borrow_mut()
+    fn read<R>(&self, f: impl FnOnce(&mut Value) -> R) -> R {
+        f(&mut self.value.borrow_mut())
     }
-    /// Borrow the value for a read that leaves it as stored.
-    fn peek(&self) -> Ref<'_, Value> {
+    /// Run `f` on the value for a read that leaves it as stored.
+    fn peek<R>(&self, f: impl FnOnce(&Value) -> R) -> R {
+        f(&self.value.borrow())
+    }
+
+    /// Borrow the stored value for a test to inspect.
+    #[cfg(test)]
+    fn stored(&self) -> core::cell::Ref<'_, Value> {
         self.value.borrow()
     }
@@ add_shifted (sub_shifted likewise)
-        self.value
-            .get_mut()
-            .apply_accumulator(&mut other.read(), shift, Update::Add);
+        let value = self.value.get_mut();
+        other.read(|operand| value.apply_accumulator(operand, shift, Update::Add));
-        self.read().cmp_zero()
+        self.read(Value::cmp_zero)
-        self.read().cmp_zero_stable_under(bits)
+        self.read(|value| value.cmp_zero_stable_under(bits))
-        self.peek().is_known_zero()
+        self.peek(Value::is_known_zero)
     (stored_digit_count, stored_bits, bits, signed_magnitude,
      scaled_signed_magnitude: the same one-line pattern)
--- compare.rs
-        compare(&mut self.read(), &mut other.read())
+        self.read(|a| other.read(|b| compare(a, b)))
-                Some(compare_primitive(&mut self.read(), sign, magnitude))
+                Some(self.read(|value| compare_primitive(value, sign, magnitude)))
--- format.rs (Debug)
-        let rendered = {
-            let value = self.peek();
-            if formatter.alternate() {
-                format!("{:#?}", StoredForm(&value))
+        let alternate = formatter.alternate();
+        let rendered = self.peek(|value| {
+            if alternate {
+                format!("{:#?}", StoredForm(value))
             } else {
-                format!("{:?}", StoredForm(&value))
+                format!("{:?}", StoredForm(value))
             }
-        };
+        });
--- operators.rs (AddAssign<&Accumulator>; SubAssign likewise)
-        self.value
-            .get_mut()
-            .apply_accumulator(&mut rhs.read(), 0, Update::Add);
+        let value = self.value.get_mut();
+        rhs.read(|operand| value.apply_accumulator(operand, 0, Update::Add));
```

**What I verified.**
- Clippy: `--all-targets --all-features -D warnings` and `--lib --tests -D warnings` are both clean on suanpan.
- Tests: 88 of 88 suanpan tests and 13 of 13 doctests pass with all features on. That includes `metered.rs`, whose exact touch pins did not move, so the closure form adds no metered work.

### 2. Remove migration residue in `before` (adoptable now; prototyped in group A)

**Unneeded `&mut` parameters.** I ran clippy's `needless_pass_by_ref_mut` on the untouched tip, and it lists them exactly:
- `place/filter.rs:216-217`: `compare_heights(probe: &mut, bound: &mut)`. Its caller `Comparison::read` (`:170`) carries the same `&mut`, and so do its call sites (`:347`, `:545`, `:551`), which pass `&mut walk.*_height`.
- `range_minima/anchor.rs:168`: `gap_dominates_word(&mut self)`.
- `range_minima/emission.rs:157,164`: `bridge_add_gap(&mut self, ..)` and `bridge_sub_gap(&mut self, ..)`.

One more site is outside the lint's reach: `projection.rs:302,311` call `.as_mut()` only to reach `.sign()`.

**Provenance.** I compared base `a591f15ca`, parent `0d6e24ea4`, and the tip:
- `main`'s reads took `&mut`, so `compare_heights`, `Comparison::read`, `gap_dominates_word`, and the `as_mut()` calls were required there. This branch made them unnecessary.
- `bridge_*_gap` was already over-borrowed on `main`.
- The builder's claim that 13 files were "restored exactly to main's versions" is accurate. But restoring `main`'s signatures keeps `main`'s forced `&mut`.

Diff (`before` part of `groupA.diff`, abridged):
```diff
--- place/filter.rs
-    fn read(&mut self, probe: &mut Accumulator, bound: &mut Accumulator) {
-        let sign = match &mut self.difference {
+    fn read(&mut self, probe: &Accumulator, bound: &Accumulator) {
+        let sign = match &self.difference {
-        probe: &mut Accumulator,
-        bound: &mut Accumulator,
+        probe: &Accumulator,
+        bound: &Accumulator,
-            difference += &*probe;
-            difference -= &*bound;
+            difference += probe;
+            difference -= bound;
     (and the mirrored branch)
-            side.comparison
-                .read(&mut walk.probe_height, &mut side.bound);
+            side.comparison.read(&walk.probe_height, &side.bound);
     (and the lo/hi sites)
--- place/filter/tests.rs
-        let mut probe = height(...);  let mut bound = height(...);
-        ... Comparison::compare_heights(&mut probe, &mut bound);
+        let probe = height(...);  let bound = height(...);
+        ... Comparison::compare_heights(&probe, &bound);
--- projection.rs (x2)
-                        .as_mut()
+                        .as_ref()
--- range_minima/anchor.rs
-    pub(super) fn gap_dominates_word(&mut self) -> Option<Ordering> {
+    pub(super) fn gap_dominates_word(&self) -> Option<Ordering> {
--- range_minima/emission.rs
-    pub fn bridge_add_gap(&mut self, delta: &mut Accumulator) {
+    pub fn bridge_add_gap(&self, delta: &mut Accumulator) {
-    pub fn bridge_sub_gap(&mut self, delta: &mut Accumulator) {
+    pub fn bridge_sub_gap(&self, delta: &mut Accumulator) {
```

**Deliberately left out.** Clippy also flags two suanpan test helpers: `read_value(&mut Accumulator)` at `tests.rs:85` and `assert_observations(&mut Accumulator)` at `surface.rs:710`. I recommend keeping their `&mut`, because it is now the only visible marker that the helper compacts its argument in place.

**What I verified.**
- Clippy with `-D warnings -W clippy::needless_pass_by_ref_mut` on `before` and `suanpan`, all targets and all features: no findings remain. The `--lib --tests` runs on both crates are clean too.
- Tests: 124 of 124 pass (suanpan's whole suite plus `before`'s place, range-minima, and projection suites).
- My first two clippy attempts failed on follow-ons of the prototype, which I then fixed: an `unused_mut`, and `unnecessary_mut_passed` in `filter/tests.rs:64`.

### 3. Dead pre-reads and the `Sign`-to-`Ordering` adapter in `before` (adoptable now, a strict deletion of work; prototyped as group B)

**`tick.rs:663`, `:691`, `:709`.** Each runs `self.gap.sign();` (or `self.height.sign();`), discards the result, then immediately calls `BigInt::from(&…)`. That read already compares with zero first, through `read_normalized`, so the pre-read only adds one deciding touch per emission.

On `main` these lines were `cmp_zero()` calls that did real work, because `main`'s readout did not compact. The first commit's message says "into_bigint and projection no longer compare with zero before reading, since the read does", but these three sites were missed.

**`measure/integral.rs:316-331` (`finish`).** It builds `BigInt::from(&self.total).into_parts()` and then hand-maps `Sign` to `Ordering`. That is the only `Sign`→`Ordering` match in `before`, and it is the inverse of the one in `interop.rs:104-108`.

Its consumer `pair_order` (`integral/pair.rs:28-30`) then keeps only `.0`. It materializes the whole magnitude just to read a sign. Returning the accumulator removes the adapter and lets `pair_order` call `sign()`.

```diff
--- measure/integral.rs
-use num_bigint::{BigInt, BigUint, Sign};
+use num_bigint::{BigInt, BigUint};
-    pub fn finish(mut self, closing_shift: u64) -> (Ordering, BigUint) {
+    pub fn finish(mut self, closing_shift: u64) -> Accumulator {
@@
-        let (sign, magnitude) = BigInt::from(&self.total).into_parts();
-        let sign = match sign {
-            Sign::Minus => Ordering::Less,
-            Sign::NoSign => Ordering::Equal,
-            Sign::Plus => Ordering::Greater,
-        };
-        (sign, magnitude)
+        self.total
--- measure/integral/pair.rs
-        let (sign, total, scale) = Self::pair(a, b, orientation);
-        debug_assert_ne!(sign, Ordering::Less, "the measure is nonnegative");
+        let (total, scale) = Self::pair(a, b, orientation);
+        let total = BigUint::try_from(&total).expect("the measure is nonnegative");
         Rank::from_raw(total, scale)
@@
-        Self::pair(a, b, |_| 1).0
+        Self::pair(a, b, |_| 1).0.sign()
-    ) -> (Ordering, BigUint, u64) {
+    fn pair(a: &Version, b: &Version, orientation: impl Fn(Ordering) -> i8) -> (Accumulator, u64) {
-        let (sign, total) = integral.finish(scale);
-        (sign, total, scale)
+        (integral.finish(scale), scale)
--- measure/rank.rs
-        let (sign, numerator) = integral.finish(max_depth);
-        debug_assert_ne!(sign, Ordering::Less, "heights are nonnegative");
+        let numerator =
+            BigUint::try_from(&integral.finish(max_depth)).expect("heights are nonnegative");
--- version/tick.rs (three deletions)
-            self.gap.sign();
-            self.height.sign();
-                self.gap.sign();
```

**Caveats.**
- **A debug check becomes a release panic.** The `expect`s turn a `debug_assert_ne!` into a panic that only programmer error can reach. That follows the idiom this branch already introduced at `rank.rs:1051` and `min_ticks.rs`. If the owner prefers exact preservation, use `BigInt::from(&total).into_parts()` with a `debug_assert_ne!(sign, Sign::Minus)`.
- **Board readings will move, and I did not measure them.** I infer they move down: one touch per `tick.rs` emission, and `pair_order`/`rank_cmp` lose the magnitude readout and its allocation. Commit `eaad03a3c` already flipped the `version_ticks × touch` worst-case ranking once, so whoever lands this should run the board's capture/compare.

**What I verified.**
- Clippy on `before` (`--all-targets --all-features -D warnings`, and `--lib --tests -D warnings`) is clean.
- Tests: 207 of 207 pass across `before`'s measure, tick, rank, order, and lattice suites, including 39 meter-binary tests. Those are band and flatness checks (`tick_counts`, `version_scaling`, `hoisted_window`, and others), so a constant saving per call does not register in them.

### 4. One home for "a count of 2^64 or more is accepted only for zero" (adoptable now; included in group C)

**Current form.** The rule is implemented twice:
- `value.rs:182-195`, in `Value::shift_left`
- `operators.rs:397-411`, in `Shl for &Accumulator`

The panic text `"a nonzero contribution needs an addressable digit position"` appears three times: at those two sites and in `digit_index` (`accumulator.rs:524`).

**Proposed form.** Add one `Value::addressable_shift(&mut self, u128) -> Option<u64>` that panics before anything is written. Both callers use it. The `&Accumulator` call site is the `if let` pattern that panics in guard form (candidate 1).
```diff
+    /// Narrow a shift count, returning `None` for a zero shifted by `2^64`
+    /// bits or more, which deposits nothing.
+    ///
+    /// # Panics
+    ///
+    /// Panics if the value is nonzero and the count is `2^64` or more. Such a
+    /// value starts above digit position `2^59`, a 4 EiB buffer that no
+    /// allocator can supply, so we panic before anything is written rather
+    /// than in the allocator.
+    pub(crate) fn addressable_shift(&mut self, shift: u128) -> Option<u64> {
+        let shift = u64::try_from(shift).ok();
+        assert!(
+            shift.is_some() || self.cmp_zero() == Ordering::Equal,
+            "a nonzero contribution needs an addressable digit position"
+        );
+        shift
+    }
@@ shift_left
-        // A nonzero value shifted by 2^64 bits or more starts above digit
-        // position 2^59, ... We panic here, before touching the receiver, ...
-        let Ok(shift) = u64::try_from(shift) else {
-            assert!(self.cmp_zero() == Ordering::Equal, "...");
-            return;
-        };
+        let Some(shift) = self.addressable_shift(shift) else {
+            return;
+        };
@@ Shl for &Accumulator
-                match u64::try_from(shift_count(shift)) {
-                    Ok(shift) => shifted.add_shifted(shift, self),
-                    Err(_) => assert!(self.is_zero(), "..."),
-                }
+                if let Some(shift) = self.read(|value| value.addressable_shift(shift_count(shift))) {
+                    shifted.add_shifted(shift, self);
+                }
```
Touch counts are identical on both arms. `primitive_shift_counts_are_checked`, including its "must not partially mutate a nonzero receiver" check, passes; it ran in the group C run.

### 5. One sign-and-magnitude decomposition for primitives (adoptable now; prototyped in group A)

**Current form.** Three hand-built versions of the same rule, "keep sign and magnitude separate":
- `compare.rs:113-142` has the `SignMagnitude` trait.
- `conversions.rs:84-151` has two macros:
  - `conversions!` takes `$magnitude`/`$update` closures, one written on a 120-character line that rustfmt cannot reach inside a macro.
  - `narrow_conversions!` repeats the identical `TryFrom` body and routes `From` through `as $wide`.
- `operators.rs:50-84` has `wide_assignments!`, again with closures.

Three module docs restate the rule. The duplicated `TryFrom` body predates this branch.

**Proposed form.** Move `SignMagnitude` to `conversions.rs`, whose module doc states the rule. Add `Update::from_sign(Ordering)`, and generate one `conversions!` macro over all twelve types. `From`, the full-width `+=`/`-=`, comparisons, and `Digits::deposit_value` all go through it.
```diff
--- conversions.rs
+/// A primitive integer as a sign and a magnitude.
+///
+/// A `u128` magnitude holds every primitive's exactly, including that of
+/// `i128::MIN`, so no signed value is negated in its own type.
+pub(super) trait SignMagnitude: Copy { fn sign_magnitude(self) -> (Ordering, u128); }
+(signed/unsigned impl macros moved verbatim from compare.rs)
 macro_rules! conversions {
-    ($parts:ident, $magnitude:expr, $update:expr; $($integer:ty),* $(,)?) => {$(
+    ($parts:ident; $($integer:ty),* $(,)?) => {$(
-                Accumulator::from_value(Value::from_magnitude($magnitude(value), $update(value)))
+                let (sign, magnitude) = value.sign_magnitude();
+                Accumulator::from_value(Value::from_magnitude(magnitude, Update::from_sign(sign)))
-(the one-line TryFrom body, both closure-laden invocations, and all of narrow_conversions!)
+conversions!(primitive_signed; i8, i16, i32, i64, i128, isize);
+conversions!(primitive_unsigned; u8, u16, u32, u64, u128, usize);
--- operand.rs
+    /// The update that applies a magnitude of the given sign: subtraction
+    /// for a negative one, addition otherwise.
+    pub fn from_sign(sign: Ordering) -> Update { if sign == Ordering::Less { Update::Subtract } else { Update::Add } }
--- operators.rs
-wide_assignments!(i128, i128::unsigned_abs, i128::is_negative);
-wide_assignments!(u128, |value| value, |_| false);
+wide_assignments!(i128, u128);   // bodies: let (sign, magnitude) = rhs.sign_magnitude(); ...
+                                 //   apply_magnitude(magnitude, Update::from_sign(sign))  / sign.reverse()
--- digits/add.rs
-        let update = if value.is_negative() { Update::Subtract } else { Update::Add };
+        let update = Update::from_sign(value.cmp(&0));
```
`From<i8>` previously went `as i128` → `unsigned_abs` → sign. It now produces the same magnitude and sign directly, so the representation is identical. I verified this in the same group A run (124 of 124 pass, including `primitives.rs`'s `integer_boundaries` over all twelve types).

### 6. A `Value::assert_invariants` test helper (adoptable now; prototyped in group A)

Five test sites spell out `x.peek().digits.assert_invariants(x.peek().small.is_some(), t)`. They re-derive `Value`'s field contract ("while `small` is `Some`, `digits` holds only zeros") at each call:
- `tests.rs:75`
- `standard.rs:96`
- `surface.rs:792`
- `representation.rs:115` and `:201`

A test-only method states the rule once, on the type that owns it:
```diff
+/// Check a value's representation for the tests.
+impl Value {
+    /// Check the digit invariants after `trace`, including that a small value
+    /// leaves every retained digit zero.
+    fn assert_invariants(&self, trace: &[u8]) {
+        self.digits.assert_invariants(self.small.is_some(), trace);
+    }
+}
-        read.peek()
-            .digits
-            .assert_invariants(read.peek().small.is_some(), &[]);
+        read.peek().assert_invariants(&[]);
```
(This is the guard-form call; under group C it becomes `.stored()`.)

### 7. Not prototyped; described only

- **`ScaledWidth` should hold its limbs** (`before/src/version/measure/integral/width.rs:17-41`; adoptable, internal to `before`; heap readings move, likely down).
  - It stores a `BigUint` that it only ever iterates with `iter_u64_digits()` or compares with zero.
  - Storing `limbs.as_ref().to_vec()` from `scaled_signed_magnitude` would let `before/src/accumulator.rs::scaled_biguint_parts` (one caller) and its `magnitude()` converter dissolve. That converter duplicates suanpan's private `interop.rs::biguint` (`:131-139`).
  - `num-bigint` 0.4 has no public `u64`-digit constructor (I checked the registry source), so the duplicate cannot be replaced by a library call.
  - After this change, `before/src/accumulator.rs` would hold only `digit_len`. That still belongs there, because it measures in the accumulator's digit unit.
- **`value.rs:1` module doc is inaccurate** (prose fix). It says "every amortized read of it", but the file also holds `negate`, `shift_left`, `reset`, and `reserve_bits`, while one read (`primitive_parts`) lives in `conversions.rs`.
- **The bound on unread digits is derived twice** (adoptable, minor). `digits/bits.rs` (`UNREAD_BOUND = 3`, "< 2B/(B−1) < 3") and `digits/sign.rs` (`COMPARISON_DECIDED = 3`, "< 2.01") each derive the same fact. One constant in `digits.rs` with one derivation would tie the bit-length fast path to the comparison it shares that bound with.
- **Sibling docs lack a shared outline** (structural documentation; for the prose pass).
  - `sub_shifted` (`accumulator.rs:222`) has no `# Complexity` heading: the cost paragraph runs straight on from the `compile_fail` fence.
  - `add_shifted_limbs` and `sub_shifted_limbs` use different outlines (a section in one, inline text in the other).
  - Their `# Panics` text differs ("retained span" versus "working width").
- **`interop.rs` uses only the public API**, so it could move to a top-level `src/interop.rs`. That would state structurally that the interop is a client of the API. Low value.
- **Trivial:** `regions.rs:371-373` binds `min` only to borrow it, while the parallel site in `tick.rs` uses the one-expression `BigInt::from(&above.into_offset())`.
- **Not worth changing:**
  - The accumulator × accumulator operator impls are written out, not macro-generated. That is the right form, because the forms deliberately differ in storage reuse: owned + owned swaps to the wider input, `&a - b` is `-(b - a)`, and a borrowed left operand copies through `From`. Each body is one to three lines delegating to `+=`/`-=`.
  - Primitives are macro-generated, one invocation per family. The repeated twelve-type lists are fixed by the language and need no further abstraction.
  - `TestBig` stays: `cargo clippy -p suanpan --lib --tests` builds the tests without the `num-bigint` feature, so the interop operators cannot replace these adapters.

### Cross-topic notes for the sibling reviewers (inferred, not measured)

- **Coverage.** `assert_observations` (`surface.rs:709-754`) calls `actual.sign()` and `actual.cmp_zero_stable_under(..)` in place after every step. `assert_standard_traits` clones specifically "so that observing never changes the tested history". So the surface property compacts the receiver's top before every operation, which may hide histories where cancellation accumulates across steps without a read. The behavior predates this branch; the explicit `Operation::Read` step suggests in-place reads were meant to be generated.
- **Correctness.** `BigUint::try_from(..).expect(..)` in `rank.rs`/`min_ticks.rs` turns a debug-only check into a release panic reachable only by programmer error. The builder disclosed this; candidate 3 extends the same idiom.

## My view of the inherent-method keep/remove list

I agree with the builder on every keep. On the two proposed removals:

- **`normalize`: remove.** Every read now compacts what it touches, and nothing outside suanpan's own tests calls it: `standard.rs:69` (`Form::Normalized`) and `surface.rs:571` (`Operation::Normalize`).
  - After `sign()`, the stored width is within one digit of the value's own width, so `normalize` tightens `stored_bits` by at most one more digit.
  - Removing it takes along:
    - the cost-table row and the paragraph at `lib.rs:98-102`
    - the thin wrappers `Value::normalize` and `Digits::normalize`
  - The two tests can call the internal pass (`acc.value.get_mut().normalize()` under `cfg(test)`) or read through `signed_magnitude()`.
- **`stored_digit_count`: keep it in this branch, and decide separately.**
  - The rework did not make it redundant; it has always been `stored_bits / 32`.
  - The digit is a documented unit of suanpan's contract. `cmp_zero_stable_under` promises "at most two base-2^32 digit positions".
  - `before` has about 20 sites that compare in digits:
    - `HEIGHT_FREEZE_ALLOWANCE_DIGITS`
    - `HeightWidths::MAX_CLOSE_LEAD`
    - `boundary.rs`'s `+ 2`
    - everything fed by `digit_len`
  - Removing it means re-denominating those constants into bits. That is churn in `before` for one fewer suanpan method; it may be worth doing, but as its own ruling.
  - The `usize` return counts digit positions actually held, so the audit's `usize` rule does not object to it.
- **The builder's unbuilt candidate, `TryFrom<&Accumulator>` for primitives: leave it out.** `TryFrom<Accumulator>` already returns the value on failure, and the borrowed form would need a new error type for no caller.

## Evidence I ran

All runs were on ox-east-1 through `on-illumos.sh /Users/oxide/src/rumors-slot-53 'unset CARGO_TARGET_DIR; export NEXTEST_TEST_THREADS=24; …'`. Logs are in my scratch directory.

- **Base verification (Mac):** `git -C /Users/oxide/src/rumors-slot-53 rev-parse HEAD` → `eedff98d65309eaeeb333d7ad84ccc025d39740b`, with `git status --short` empty.
- **Clippy at the untouched tip** (`cargo clippy --locked -p before -p suanpan --all-targets --all-features -- -W clippy::needless_pass_by_ref_mut`, exit 0, `clippy-refmut.log`). Every warning is "this parameter is a mutable reference but is not used mutably":
  - `crates/before/src/version/place/filter.rs:216:16` and `:217:16`
  - `crates/before/src/version/range_minima/anchor.rs:168:38`
  - `crates/before/src/version/range_minima/emission.rs:157:27` and `:164:27`
  - `crates/suanpan/src/accumulator/tests/surface.rs:710:13`
  - `crates/suanpan/src/accumulator/tests.rs:85:20`
- **Group A** (`groupA-clippy3.log`, `groupA-test.log`):
  - Clippy, all targets and all features, `-D warnings -W clippy::needless_pass_by_ref_mut`, plus `-p suanpan --lib --tests` and `-p before --lib --tests` with `-D warnings`: `CLIPPY-OK`; the `--no-run` build printed `BUILD-OK`.
  - `cargo nextest run --locked -p suanpan -p before --all-features -E "package(suanpan) | (package(before) & test(/place|range_minima|projection/))"` → `Summary [  10.545s] 124 tests run: 124 passed, 692 skipped`.
  - The saved `groupA.diff` differs from the tested tree only in six test lines, which I restored to the tip's form (the `read_value`/`assert_observations` `&mut` signatures). I did not rerun after that restoration.
- **Group B** (`groupB-build.log`, `groupB-test.log`):
  - Clippy on `before` (all targets and all features, plus `--lib --tests`), `-D warnings`: `CLIPPY-OK`, `BUILD-OK`.
  - `-E "package(before) & (test(/measure|tick|rank|order|lattice/) | (binary(meter) & test(/tick|rank|version_scaling|pair/)))"` → `Summary [  25.786s] 207 tests run: 207 passed, 521 skipped`, 39 of them `before::meter` tests.
  - Board not run; the movement is my inference.
- **Group C** (`groupC-build.log`, `groupC-test.log`):
  - The guard-scope program, compiled with `rustc 1.97.1 (8bab26f4f 2026-07-14)` under `--edition 2021` and `--edition 2024`, printed the lines quoted in candidate 1 under both editions.
  - Clippy on suanpan, both configurations, `-D warnings`: `CLIPPY-OK`, `BUILD-OK`.
  - `cargo nextest run --locked -p suanpan --all-features` → `Summary [  10.368s] 88 tests run: 88 passed, 0 skipped`.
  - `cargo test --locked -p suanpan --all-features --doc` → `test result: ok. 13 passed; 0 failed`.
- **Restoration:**
  - I reverted each group with `git apply -R <its own diff>`.
  - I deleted the untracked demo file and synced, and the box reported `demo removed`. I removed `target/review-guard` in my slot on the box.
  - Final local check: `git diff --quiet`, zero lines from `git status --porcelain`, and HEAD `eedff98d65309eaeeb333d7ad84ccc025d39740b`.
- **Not run, per the brief:** the landing check (a sibling runs it), the board, and any timing measurements.

Resumption notes: `/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/review-122-structure/NOTES.md`.
