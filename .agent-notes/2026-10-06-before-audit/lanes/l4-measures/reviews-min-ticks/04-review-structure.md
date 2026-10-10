<!-- CAVEAT LECTOR: the structure review of 8c2282903, written by a Claude (Opus 5.5) agent and archived verbatim by the audit coordinator. Its claims are unverified unless the README says otherwise. -->

## review-mt-structure: report on `fix/min-ticks-transient-heap` (tip `8c2282903`, base `e0956b25d`)

### Verdict

**Approve, from the structure side.** I found nothing structurally wrong that blocks the merge. One rationale comment is inaccurate; prototype 1b below corrects it. On the two simplifications the builder flagged:

- **Stored-width trigger: keep it, but move it.** It is redundant for *which* leaves freeze. I prove this below and confirmed it with an assertion run over the board sweep and the full suite. It is not redundant for cost: removing it raises `min_ticks` touch on 15 board rows and heap on 4. It is a fast path that freezes without reading the value. Moving it into `add_leaf` beside the record trigger (1b) gives one freeze decision in one place and makes 17 rows strictly cheaper.
- **`RangeMinima`'s payload parameter: dissolve it.** I dispute the builder's reason for keeping it. The payload lifecycle test checks a storage column `min_ticks` no longer uses. What `min_ticks` relies on is the order of the begin and end events. I prototyped the dissolution (prototype 3). The board stays byte-identical, and the rewritten property now asserts the stack-order invariant directly; a mutant shows it can fail.

Base verification: HEAD was `8c22829038e92a56efced84a29f9908b42e1b6d1` and the tree was clean. `range_minima` is unchanged by the branch (`git diff --stat e0956b25d 8c2282903 -- crates/before/src/version/range_minima*` is empty). Its last non-unit payload user went away with the old contribution store.

### Candidates, ranked by value

**1. Report distinct minima as begin and end events, and drop `RangeMinima<P>`.** Design proposal. Prototyped; board identical; tests pass.

The stack-order invariant is currently kept by convention at each `min_ticks` call site, not by structure:
- `min_ticks` infers "push a record" from a side effect: the payload factory sets `Events.new_minimum` (`/Users/oxide/src/rumors-slot-56/crates/before/src/version/measure/min_ticks/minima.rs:67-70, 163-177`). The inference holds only because the factory runs in exactly the positive and negative arming branches.
- On undercut, `min_ticks` settles the displaced innermost record by hand (`minima.rs:196`). On a lower *arming*, `RangeMinima` reports the same event itself, through factory plus retire (`range_minima.rs:229-231`). The two paths are asymmetric.
- There are two push sites (`minima.rs:155, 176/199`) and two pop-and-settle sites (`minima.rs:75-84, 127-134`).
- Every caller pays `|_| (), |(), _| ()` noise: `emission.rs:20,22,41,53,79,89,133` and `minima.rs:156`.

Proposed form, prototyped in `scratchpad/review-mt-structure/p3-design.patch` (7 files, +190/−199):
- `Close` loses its payload. `arm_at_height` and `arm_below` take one `end: impl FnMut(&mut C)` and return whether the value begins a new innermost distinct minimum. `undercut` also ends the displaced innermost minimum.
- `Boundaries` loses its `payloads` column.
- `range_minima`'s module doc gains the contract: every distinct minimum begins once and ends once, and the one that ends is always the newest not yet ended, so client state forms a stack.
- `observe_leaf` shrinks to:
```rust
let begins = if self.ranges.has_pending() {
    self.ranges.arm_at_height(&mut events, Events::settle_top)
} else if self.ranges.armed() && self.ranges.undercuts_here() {
    self.ranges.undercut(&mut events, Events::settle_top);
    true
} else {
    false
};
if begins {
    let first = references.reference_top();
    self.records.push(leaf.offset(), first);
}
```
- The lifecycle property drives a `DistinctMinima` client stack from the events alone. After every step and every close it asserts `client.stack == dedup(model)`. That is exactly "one record per distinct live minimum, innermost on top", which `Records` relies on.

Verified on ox-east-1:
- clippy `-D warnings` is clean.
- `733 tests run: 733 passed, 1 skipped`.
- `amp-board compare: 16635 rows: 16635 unchanged`.
- **Mutant:** I deleted `end(context)` from `undercut`. Result: `28 tests run: 19 passed, 9 failed`, including the rewritten property (`left: []`, `right: [0]` at `range_minima/tests.rs:509`), `min_ticks_matches_the_tree_oracle_across_reference_paths`, and `skyline_min_ticks_descending_boundary_clearance_band`.

Leftover in the prototype: the property keeps its old name, `..._payload_lifetimes`; rename it.

**2. Fold the stored-width freeze into `add_leaf` as an unread fast path.** Adoptable now. Prototyped. Same freeze schedule, strictly cheaper.

Today two triggers in two files decide one schedule:
- the stored-width trigger in the fold (`min_ticks.rs:75-87`)
- the record trigger in `add_leaf` (`heights.rs:187`)

The comment at `min_ticks.rs:78-80` says the stored trigger "bounds the stored width that reading the change costs". That is inaccurate. `signed_magnitude` costs amortized O(log W + Q) whatever the stored width. What the trigger actually does is skip reading a value that is about to be frozen.

Proof that the schedule is unchanged:
- Every leaf either reads `live` (in `add_leaf`) or resets it (by freezing).
- A read normalizes the digits and trims to the top nonzero digit, so stored ≤ value + 31. `cmp_zero` on a zero value sets `highest_nonzero = 0`.
- Adding a k-bit change widens stored by at most one digit past the wider operand, since `add_at` advances a carry one position at a time.
- So stored > k + 256 forces the previous value above k + 193 bits, and the new value above k + 192 bits. The record trigger (slack 4) fires at that same leaf.

Patch (`p1b-colocated.patch`, full):
```diff
--- a/crates/before/src/version/measure/min_ticks.rs
-use super::HEIGHT_FREEZE_ALLOWANCE_BITS;
-
 mod minima;
 #[cfg(test)]
-thread_local! { /// Freezes the stored-width trigger below fires ... pub static FREEZE_HITS ... }
+pub use minima::FREEZE_HITS;
@@ fold loop
-            // Freeze a much wider accumulated change ... far more tightly.
-            if recent_height_change.stored_bits()
-                > height_change.magnitude().bits() + HEIGHT_FREEZE_ALLOWANCE_BITS
-            {
-                #[cfg(test)]
-                FREEZE_HITS.with(|hits| hits.set(hits.get() + 1));
-                height_references.freeze(&mut recent_height_change, &mut answer);
-            }
-
--- a/crates/before/src/version/measure/min_ticks/minima.rs
+#[cfg(test)]
+pub use heights::FREEZE_HITS;
--- a/crates/before/src/version/measure/min_ticks/minima/heights.rs
+/// Bits a live change's working width may exceed the leaf's height change by
+/// before the change freezes without being read.
+///
+/// A read leaves the live change's working width within one 32-bit digit of
+/// its value's width, and the fold reads it at every leaf it does not freeze.
+/// Adding a change of `k` bits widens it by at most one digit past the wider
+/// operand. A working width more than this many bits above `k` therefore
+/// means a value more than `k + 192` bits wide, which the record trigger
+/// would freeze anyway; freezing first skips reading a value the freeze
+/// moves whole.
+const UNREAD_FREEZE_BITS: u64 = 256;
 thread_local! {
+    /// Freezes of either kind on this test thread.
+    pub static FREEZE_HITS: core::cell::Cell<u64> = const { core::cell::Cell::new(0) };
@@ add_leaf
-        let mut offset = BigInt::from(&*live);
-        if offset.bits() > change.bits() + RECORD_SLACK_BITS {
-            #[cfg(test)]
-            bump(&RECORD_FREEZES);
+        let change_bits = change.bits();
+        let offset = if live.stored_bits() > change_bits + UNREAD_FREEZE_BITS {
             self.freeze(live, total);
-            offset = BigInt::ZERO;
-        }
+            BigInt::ZERO
+        } else {
+            let offset = BigInt::from(&*live);
+            if offset.bits() > change_bits + RECORD_SLACK_BITS {
+                #[cfg(test)]
+                bump(&RECORD_FREEZES);
+                self.freeze(live, total);
+                BigInt::ZERO
+            } else {
+                offset
+            }
+        };
@@
-    pub fn freeze(&mut self, live: &mut Accumulator, total: &mut Accumulator) {
+    fn freeze(&mut self, live: &mut Accumulator, total: &mut Accumulator) {
+        #[cfg(test)]
+        bump(&FREEZE_HITS);
```

Verified on ox-east-1:
- clippy `-D warnings` is clean.
- `733 tests run: 733 passed, 1 skipped`. This includes `zero_drift_never_freezes`, whose `min_ticks` leg now counts every `min_ticks` freeze rather than one kind, which strengthens it.
- `amp-board compare: 16635 rows: 6 growing, 1 uneven, 10 constant, 16618 unchanged`. Every moved row is a `version_min_ticks` touch *decrease*; no heap row moved:
  - linear: freeze-pos −8191 @604241 B, promo-rearm −8192, dense-suffix −8193, freeze-parade −4094, arming-train −1025, wide-step-pairs −512
  - constant: −1 to −3 on 10 rows
  - uneven: cancelling-chain −1

  The tip pays one touch per stored-width freeze, reading the just-zeroed accumulator in `add_leaf`; 1b skips that read. Under the owner's rules this improvement may justify lowering ceilings. Log: `scratchpad/review-mt-structure/p1b-compare.txt`.

With 1b, `HEIGHT_FREEZE_ALLOWANCE_BITS` (`measure.rs:21-26`) serves only the rank integral and the meter families. Its home and doc could move to `integral.rs`.

**3. Behavior-preserving tidy-ups inside `min_ticks`.** Adoptable now. Prototyped together (`p2-structure.patch`, 6 files, +94/−99).

- **(a) One settle path.** `settle_record` takes the `Popped` record instead of five destructured arguments (`heights.rs:240-247`). Both pop-and-settle sites become one line each (`minima.rs:75-84` and `127-134` were duplicates).
- **(b) No duplicated product.** `settle_record` hand-rolls `add_product`'s product (`heights.rs:248-254` versus `406-423`). It becomes `add_product(total, sign, magnitude.limbs(), -closes)`.
- **(c) Drop counters that duplicate a stack length.**
  - `Records::len` duplicates `first.len()` (`records.rs:43-44, 61, 67, 88-89`).
  - `HeightReferences::buried` duplicates the chunk count (`heights.rs:148-149, 224, 285-286, 344`); a new `ChunkStack::len` replaces it.
- **(d) One sign type.** Signs are a mix: `bool` in `IntegerStack` and `Popped`, `Ordering` in suanpan, `Sign` in `BigInt`, with conversions at `heights.rs:466, 482-486`. The patch uses `Ordering` throughout, matching `signed_magnitude`. This item is optional.
- **(e) Visibly dual loops.** `IntegerStack::push`'s nested limb/half loop (`integers.rs:97-106`) becomes `for piece in (0..pieces).rev()`, mirroring `pop`'s `for piece in 0..pieces` (`integers.rs:132-135`).
- **(f) One test helper.** `offset_of` (`records/tests.rs:9-22`) and `value_of` (`integers/tests.rs:25-33`) were the same limbs-to-`BigInt` helper; it is now shared.

Verified on ox-east-1: `733 passed, 1 skipped`, and `amp-board compare: 16635 rows: 16635 unchanged`. The first clippy run caught an unused `Popped` import in my patch. I fixed it in the saved patch, and clippy `-D warnings` then exited 0.

**4. Gather the cost argument into one ledger.** Prose placement; not prototyped. The linear-cost argument is spread across overlapping pieces:
- `minima.rs:35-43` and `heights.rs:9-24, 47-57`
- `records.rs:1-11`, plus `integers.rs:1-11` and `18-22`
- `RECORD_SLACK_BITS` and `INLINE_CHUNK_BITS` (`heights.rs:67-91`)
- `min_ticks.rs:75-80`
- `measure.rs:15-19`

No single place gives the per-event ledger: each change folded once; the per-leaf read of `live`; a record written once and read once; range-minima moves; freezes; merges; bury and expose; final settle. One step is argued nowhere: why the per-leaf `BigInt::from(&*live)` plus `*total += &offset` is linear. It is bounded by change + `RECORD_SLACK_BITS`, or paid by the stretch at a freeze.

`PACKED_BITS`'s rationale (`integers.rs:20-21`, "input already paid for over 256 bits") depends on the record bound, which lives in `heights.rs`. That makes `integers.rs` less generic than its module doc presents it.

Proposal: one ledger in the `minima` module doc, each line pointing to the item whose doc carries the local argument. Submodules then keep only their own invariant.

**5. Drop the `BigInt` intermediate on every leaf.** Adoptable; it would move counters, so it is not prototyped. `add_leaf` builds a `BigInt` from `live.signed_magnitude()` (`heights.rs:186`), and `Records::push` immediately splits it back into sign and 64-bit limbs (`records.rs:65-66`). suanpan's conversion allocates a temporary 32-bit buffer for any offset over 128 bits (`suanpan/src/accumulator/interop.rs:136-149`). `LeafHeight` could hold `(Ordering, Limbs)` and feed both `total` and `Records` directly. That removes `num-bigint` from the record path. The heap change would need a capture to measure.

**6. Two equal thresholds on different measures.** Minor note; not prototyped. `INLINE_CHUNK_BITS` (`heights.rs:91`) compares *stored* width, while `PACKED_BITS` (`integers.rs:22`) compares *value* width. A chunk with stored width ≤ 256 can normalize to 257–288 bits and land in `IntegerStack`'s separate allocation. That is harmless and still constant cost, but the doc at `heights.rs:443` ("Chunks at most INLINE_CHUNK_BITS wide") and the shared number suggest a coupling that nothing enforces.

**7. Minor notes.**
- Test taps are open-coded in two styles: `bump` at `heights.rs:107-111` versus `FREEZE_HITS.with(...)` at `min_ticks.rs:85`. 1b removes the second.
- The `WideStepPairs` family docs and `WIDE_STEP_PAIRS_STEP_BITS` (`testing/meter/board/family.rs`, `registry.rs`) state min_ticks' 256-bit copy threshold as a literal number. The coupling is guarded at runtime by `wide_step_pairs_bury_whole_chunks_at_every_second_level`.

**Dead code and residue: none found.**
- No ghost references to the contribution store: a search for `contribution`, `spill`, `StoredContribution`, `ContributionStore` and `current_contribution` under `crates/before` returns only unrelated hits.
- No orphaned packed-word helpers: the branch changes nothing under `bits/`.
- No new feature gates in `min_ticks` or `range_minima`.

### My view of the two flagged simplifications

1. **The 256-bit stored-width trigger.**
   - **Same schedule:** proven above. Empirically, prototype 1 removed the trigger and asserted in release builds that its condition implies a record freeze at the same leaf. The assertion never fired across the full board capture or the 733 tests.
   - **Higher cost:** the same prototype's comparison against the tip read `16635 rows: 19 growing, 6 constant, 16610 unchanged`, all `version_min_ticks` and all upward:
     - touch: 15 rows, +0.02 to +0.19 per byte (freeze-pos +81910 @604241 B, promo-rearm +139264 @1011713, arming-train +60712 @372743, wide-step-pairs +7680 @41601)
     - heap: 4 rows (pure-comb +1000 @5001 B, latent-ladder +2048 @6651, reveal-comb and reveal-hifloor +504 @6001)

     The touch rise follows from the record path reading each wide value before freezing it. I did not attribute the heap rise.
   - **Recommendation:** keep the check, in the 1b form.
2. **`RangeMinima`'s payload parameter.** Dissolve it, in the prototype 3 form. The builder's reason to keep it, that the lifecycle tests guarantee the stack order, does not hold. Those tests check `RangeMinima`'s internal `Vec<P>`, which `min_ticks` no longer uses. The rewritten property checks the event order `min_ticks` depends on, and the mutant shows it fails when that order breaks.

### Evidence, verbatim (all runs on ox-east-1 in `/Users/oxide/src/rumors-slot-56`)

- Tip capture: `just amp-board capture target/mtcap/tip.cap` at `8c22829038e92a56efced84a29f9908b42e1b6d1`, then `CAPTURE-DONE`.
- Prototype 1 (removal plus assertion): `Summary [ 29.522s] 733 tests run: 733 passed, 1 skipped`; `amp-board compare: 16635 rows: 19 growing, 6 constant, 16610 unchanged`.
- Prototype 1b: `Summary [ 34.540s] 733 tests run: 733 passed, 1 skipped`; `amp-board compare: 16635 rows: 6 growing, 1 uneven, 10 constant, 16618 unchanged` (all decreases); clippy exit 0.
- Prototype 2: `Summary [ 31.114s] 733 tests run: 733 passed, 1 skipped`; `amp-board compare: 16635 rows: 16635 unchanged`; clippy exit 0 after the import fix.
- Prototype 3: clippy clean; `Summary [ 34.459s] 733 tests run: 733 passed, 1 skipped`; `amp-board compare: 16635 rows: 16635 unchanged`.
- Prototype 3 mutant: `28 tests run: 19 passed, 9 failed, 706 skipped`; `mutant tests status 100`.
- I ran no timing measurements and no landing check (a sibling runs it).

### State and artifacts

- The worktree is restored: `git status --short` is empty and HEAD is `8c2282903`. Nothing was committed.
- The box's copy of slot 56 still holds prototype 1b's synced sources, which the next sync overwrites.
- Captures are on the box under `~/src/rumors-slot-56/target/mtcap/`: `tip.cap`, `p1.cap`, `p1b.cap`, `p2.cap`, `p3.cap`.
- Patches and logs are in `/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/review-mt-structure/`:
  - patches: `p1-experiment.patch`, `p1b-colocated.patch`, `p2-structure.patch`, `p3-design.patch`
  - logs: `p1-run.log`, `p1-run-compare.txt`, `p1b-run.log`, `p1b-compare.txt`, `p2-run.log`, `p2-clippy.log`, `p3-run.log`, `p3-mutant.log`
  - resumption record: `NOTES.md`

Residual risk: none of the prototypes went through the landing check or a board acceptance run. The verdicts above rest on the capture comparisons and the full `before` suite.
