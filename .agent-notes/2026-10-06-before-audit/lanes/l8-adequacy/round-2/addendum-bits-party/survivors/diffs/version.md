# Mutation survivors: version

79 survivors. Each entry: the cargo-mutants name, the enclosing function,
and the exact diff cargo-mutants applied (apply with `patch -p0` from the worktree root
after stripping the `*** ` header lines, or re-create it by hand).

## version-1: `crates/before/src/version.rs:162:9`

- name: `crates/before/src/version.rs:162:9: replace <impl Hash for Version>::hash with ()`
- function: `<impl Hash for Version>::hash`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/version.rs
+++ replace <impl Hash for Version>::hash with ()
@@ -154,17 +154,17 @@
 ///
 #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_hash.html")))]
 #[cfg_attr(
     not(doc),
     doc = "`O(n)` in total input bytes; `O(|self|)`: one pass over the canonical bytes"
 )]
 impl Hash for Version {
     fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
-        self.0.hash(state);
+        () /* ~ changed by cargo-mutants ~ */
     }
 }
 
 impl Version {
     /// Adopt a canonical [`Version`] representation produced or validated by
     /// its I/O boundary.
     pub(crate) fn from_canonical(bits: Bits) -> Self {
         Version(bits)
```

## version-2: `crates/before/src/version.rs:776:24`

- name: `crates/before/src/version.rs:776:24: replace &= with |= in Version::span_all`
- function: `Version::span_all`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version.rs
+++ replace &= with |= in Version::span_all
@@ -768,17 +768,17 @@
                 }
                 // Unreachable through the counter's weight discipline (a
                 // weight-0 lone input never sits below a merged group in the
                 // closing drain), but the match stays total rather than
                 // asserting: both sides' combiners are commutative, so folding
                 // the raw input into the owned hull is value-identical.
                 (Hull::Input(a), Hull::Merged { mut lo, mut hi }) => {
                     let a = a.version();
-                    lo &= a;
+                    lo |= /* ~ changed by cargo-mutants ~ */ a;
                     hi |= a;
                     (lo, hi)
                 }
             };
             Hull::Merged { lo, hi }
         });
         match group.expect("the fold is seeded with the receiver: never empty") {
             // The receiver alone (an empty iterator): the coincident span, the
```

## version-3: `crates/before/src/version.rs:777:24`

- name: `crates/before/src/version.rs:777:24: replace |= with &= in Version::span_all`
- function: `Version::span_all`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version.rs
+++ replace |= with &= in Version::span_all
@@ -769,17 +769,17 @@
                 // Unreachable through the counter's weight discipline (a
                 // weight-0 lone input never sits below a merged group in the
                 // closing drain), but the match stays total rather than
                 // asserting: both sides' combiners are commutative, so folding
                 // the raw input into the owned hull is value-identical.
                 (Hull::Input(a), Hull::Merged { mut lo, mut hi }) => {
                     let a = a.version();
                     lo &= a;
-                    hi |= a;
+                    hi &= /* ~ changed by cargo-mutants ~ */ a;
                     (lo, hi)
                 }
             };
             Hull::Merged { lo, hi }
         });
         match group.expect("the fold is seeded with the receiver: never empty") {
             // The receiver alone (an empty iterator): the coincident span, the
             // one place an input itself becomes the hull.
```

## version-4: `crates/before/src/version/instrument.rs:33:5`

- name: `crates/before/src/version/instrument.rs:33:5: replace meet -> Version with Default::default()`
- function: `meet`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/version/instrument.rs
+++ replace meet -> Version with Default::default()
@@ -25,17 +25,17 @@
 #[cfg(feature = "meter")]
 pub fn join(a: &Version, b: &Version) -> Version {
     super::lattice::Extreme::Higher.emit(a, b)
 }
 
 /// Run the meet traversal without public identity shortcuts.
 #[cfg(feature = "meter")]
 pub fn meet(a: &Version, b: &Version) -> Version {
-    super::lattice::Extreme::Lower.emit(a, b)
+    Default::default() /* ~ changed by cargo-mutants ~ */
 }
 
 /// Run the causal-comparison traversal used by public ordering operations.
 #[cfg(feature = "meter")]
 pub fn causal_cmp(a: &Version, b: &Version) -> Option<core::cmp::Ordering> {
     a.partial_cmp(b)
 }
```

## version-5: `crates/before/src/version/instrument.rs:57:5`

- name: `crates/before/src/version/instrument.rs:57:5: replace min_ticks -> crate::Count with Default::default()`
- function: `min_ticks`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/version/instrument.rs
+++ replace min_ticks -> crate::Count with Default::default()
@@ -49,17 +49,17 @@
 #[cfg(feature = "meter")]
 pub fn rank(version: &Version) -> crate::Rank {
     version.rank()
 }
 
 /// Compute the minimum tick count through its streaming fold.
 #[cfg(feature = "meter")]
 pub fn min_ticks(version: &Version) -> crate::Count {
-    version.min_ticks()
+    Default::default() /* ~ changed by cargo-mutants ~ */
 }
 
 /// Materialize a version projected onto a party.
 #[cfg(feature = "meter")]
 pub fn project(version: &Version, party: &crate::Party) -> Version {
     version.project(party).to_version()
 }
```

## version-6: `crates/before/src/version/instrument.rs:63:5`

- name: `crates/before/src/version/instrument.rs:63:5: replace project -> Version with Default::default()`
- function: `project`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/version/instrument.rs
+++ replace project -> Version with Default::default()
@@ -55,17 +55,17 @@
 #[cfg(feature = "meter")]
 pub fn min_ticks(version: &Version) -> crate::Count {
     version.min_ticks()
 }
 
 /// Materialize a version projected onto a party.
 #[cfg(feature = "meter")]
 pub fn project(version: &Version, party: &crate::Party) -> Version {
-    version.project(party).to_version()
+    Default::default() /* ~ changed by cargo-mutants ~ */
 }
 
 /// Read the pair-integral dense-digit counter.
 #[cfg(feature = "meter")]
 pub fn densified_digits() -> u64 {
     super::measure::integral::densified_digits()
 }
```

## version-7: `crates/before/src/version/io/regions.rs:122:9`

- name: `crates/before/src/version/io/regions.rs:122:9: replace VersionRegionReader<'a>::peek_flip -> u64 with 0`
- function: `VersionRegionReader<'a>::peek_flip`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/version/io/regions.rs
+++ replace VersionRegionReader<'a>::peek_flip -> u64 with 0
@@ -114,17 +114,17 @@
             deepest = deepest.max(depth);
             reader.skip_payload();
         }
         deepest
     }
 
     /// The next boundary's depth, without advancing; zero at exhaustion.
     pub fn peek_flip(&self) -> u64 {
-        self.next_flip
+        0 /* ~ changed by cargo-mutants ~ */
     }
 
     /// Fold consecutive boundaries deeper than `bound` into `net`.
     pub fn skip_deeper(&mut self, bound: u64, net: &mut Accumulator) {
         while self.peek_flip() > bound {
             let (_, step) = self.step();
             net.add_bigint(&step);
         }
```

## version-8: `crates/before/src/version/io/regions.rs:122:9`

- name: `crates/before/src/version/io/regions.rs:122:9: replace VersionRegionReader<'a>::peek_flip -> u64 with 1`
- function: `VersionRegionReader<'a>::peek_flip`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/version/io/regions.rs
+++ replace VersionRegionReader<'a>::peek_flip -> u64 with 1
@@ -114,17 +114,17 @@
             deepest = deepest.max(depth);
             reader.skip_payload();
         }
         deepest
     }
 
     /// The next boundary's depth, without advancing; zero at exhaustion.
     pub fn peek_flip(&self) -> u64 {
-        self.next_flip
+        1 /* ~ changed by cargo-mutants ~ */
     }
 
     /// Fold consecutive boundaries deeper than `bound` into `net`.
     pub fn skip_deeper(&mut self, bound: u64, net: &mut Accumulator) {
         while self.peek_flip() > bound {
             let (_, step) = self.step();
             net.add_bigint(&step);
         }
```

## version-9: `crates/before/src/version/io/writer.rs:155:21`

- name: `crates/before/src/version/io/writer.rs:155:21: replace > with >= in Output::take_left`
- function: `Output::take_left`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/io/writer.rs
+++ replace > with >= in Output::take_left
@@ -147,17 +147,17 @@
                 out.topology.truncate(out.topology.len() - 2);
                 out.topology.push(true);
             }
         }
     }
 
     /// Remove an encoded left sibling and make its payload pending.
     fn take_left(&mut self, code_len: u64) -> PendingPayload {
-        if code_len > NARROW_CODE_BITS {
+        if code_len >= /* ~ changed by cargo-mutants ~ */ NARROW_CODE_BITS {
             self.ensure_split();
             let Output::Split(out) = self else {
                 unreachable!()
             };
             out.topology.truncate(out.topology.len() - 2);
             out.topology.push(true);
             return PendingPayload::Wide { len: code_len };
         }
```

## version-10: `crates/before/src/version/io/writer.rs:255:34`

- name: `crates/before/src/version/io/writer.rs:255:34: replace > with == in SplitOutput::splice_continuation`
- function: `SplitOutput::splice_continuation`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/io/writer.rs
+++ replace > with == in SplitOutput::splice_continuation
@@ -247,17 +247,17 @@
         let mut cursor = BitsReader::at(src, start);
         while cursor.position() < end {
             let internal_nodes = cursor
                 .read_unary()
                 .expect("a canonical continuation has a complete next leaf");
             for _ in 0..internal_nodes {
                 self.topology.push(false);
             }
-            if cursor.position() > end {
+            if cursor.position() == /* ~ changed by cargo-mutants ~ */ end {
                 debug_assert_eq!(cursor.position() - 1, end);
                 return;
             }
             self.topology.push(true);
             let code_start = cursor.position();
             cursor
                 .skip_gamma()
                 .expect("a canonical continuation has a complete payload");
```

## version-11: `crates/before/src/version/io/writer.rs:255:34`

- name: `crates/before/src/version/io/writer.rs:255:34: replace > with >= in SplitOutput::splice_continuation`
- function: `SplitOutput::splice_continuation`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/io/writer.rs
+++ replace > with >= in SplitOutput::splice_continuation
@@ -247,17 +247,17 @@
         let mut cursor = BitsReader::at(src, start);
         while cursor.position() < end {
             let internal_nodes = cursor
                 .read_unary()
                 .expect("a canonical continuation has a complete next leaf");
             for _ in 0..internal_nodes {
                 self.topology.push(false);
             }
-            if cursor.position() > end {
+            if cursor.position() >= /* ~ changed by cargo-mutants ~ */ end {
                 debug_assert_eq!(cursor.position() - 1, end);
                 return;
             }
             self.topology.push(true);
             let code_start = cursor.position();
             cursor
                 .skip_gamma()
                 .expect("a canonical continuation has a complete payload");
```

## version-12: `crates/before/src/version/io/writer.rs:410:48`

- name: `crates/before/src/version/io/writer.rs:410:48: replace | with ^ in PayloadWriter<'_>::push_bits`
- function: `PayloadWriter<'_>::push_bits`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/io/writer.rs
+++ replace | with ^ in PayloadWriter<'_>::push_bits
@@ -402,17 +402,17 @@
         assert!(
             len == 64 || value >> len == 0,
             "payload append has no bits above its stated width"
         );
         if len == 0 {
             return;
         }
         if self.wide_len == 0 && self.staged_len + len <= NARROW_CODE_BITS as u32 {
-            self.staged = (self.staged << len) | value;
+            self.staged = (self.staged << len) ^ /* ~ changed by cargo-mutants ~ */ value;
             self.staged_len += len;
             return;
         }
         self.spill();
         self.version.out.payload().push_bits(value, len);
         self.wide_len += u64::from(len);
     }
```

## version-13: `crates/before/src/version/measure/integral.rs:230:43`

- name: `crates/before/src/version/measure/integral.rs:230:43: replace > with >= in Integrator::boundary`
- function: `Integrator::boundary`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/measure/integral.rs
+++ replace > with >= in Integrator::boundary
@@ -222,17 +222,17 @@
             self.live.sub_shifted(shift, diff);
         } else {
             self.live.add_shifted(shift, diff);
         }
     }
 
     /// Finish a boundary, bounding the live width by the widest delta read there.
     pub fn boundary(&mut self, delta_digits: usize) {
-        if self.live.stored_digit_count() > delta_digits + HEIGHT_FREEZE_ALLOWANCE_DIGITS {
+        if self.live.stored_digit_count() >= /* ~ changed by cargo-mutants ~ */ delta_digits + HEIGHT_FREEZE_ALLOWANCE_DIGITS {
             self.freeze();
         }
     }
 
     /// Account for the old parked height, then park the recent changes.
     fn freeze(&mut self) {
         let drift_order = self.live.cmp_zero();
         if drift_order == Ordering::Equal {
```

## version-14: `crates/before/src/version/measure/integral.rs:230:58`

- name: `crates/before/src/version/measure/integral.rs:230:58: replace + with * in Integrator::boundary`
- function: `Integrator::boundary`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/measure/integral.rs
+++ replace + with * in Integrator::boundary
@@ -222,17 +222,17 @@
             self.live.sub_shifted(shift, diff);
         } else {
             self.live.add_shifted(shift, diff);
         }
     }
 
     /// Finish a boundary, bounding the live width by the widest delta read there.
     pub fn boundary(&mut self, delta_digits: usize) {
-        if self.live.stored_digit_count() > delta_digits + HEIGHT_FREEZE_ALLOWANCE_DIGITS {
+        if self.live.stored_digit_count() > delta_digits * /* ~ changed by cargo-mutants ~ */ HEIGHT_FREEZE_ALLOWANCE_DIGITS {
             self.freeze();
         }
     }
 
     /// Account for the old parked height, then park the recent changes.
     fn freeze(&mut self) {
         let drift_order = self.live.cmp_zero();
         if drift_order == Ordering::Equal {
```

## version-15: `crates/before/src/version/measure/integral.rs:253:45`

- name: `crates/before/src/version/measure/integral.rs:253:45: replace > with >= in Integrator::freeze`
- function: `Integrator::freeze`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/measure/integral.rs
+++ replace > with >= in Integrator::freeze
@@ -245,17 +245,17 @@
         self.tracks_width = true;
         #[cfg(test)]
         FREEZE_HITS.with(|hits| hits.set(hits.get() + 1));
 
         // This segment belongs to the old parked height. Live changes have
         // already contributed to it region by region; folding them into the
         // parked height first would charge those contributions a second time.
         let parked = self.close_segment();
-        if self.parked.stored_digit_count() > drift_digits + HEIGHT_FREEZE_ALLOWANCE_DIGITS {
+        if self.parked.stored_digit_count() >= /* ~ changed by cargo-mutants ~ */ drift_digits + HEIGHT_FREEZE_ALLOWANCE_DIGITS {
             self.defer_parked(parked);
         }
         self.parked += &self.live;
         self.live.reset();
 
         // reset() clears the whole allocated span. Segment digits can be high
         // above an unwritten zero prefix; replacing the buffer avoids scanning it.
         self.segment_width = Accumulator::new();
```

## version-16: `crates/before/src/version/measure/integral/deferred.rs:47:9`

- name: `crates/before/src/version/measure/integral/deferred.rs:47:9: replace DeferredIntegral::is_empty -> bool with false`
- function: `DeferredIntegral::is_empty`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/version/measure/integral/deferred.rs
+++ replace DeferredIntegral::is_empty -> bool with false
@@ -39,17 +39,17 @@
         Self {
             width: Accumulator::new(),
             entries: Vec::new(),
         }
     }
 
     /// Whether there are no future contributions to settle.
     pub fn is_empty(&self) -> bool {
-        self.entries.is_empty()
+        false /* ~ changed by cargo-mutants ~ */
     }
 
     /// Extend the interval preceding the next deferral by a completed segment.
     pub fn add_width(&mut self, width: &ScaledWidth) {
         width.add_to(&mut self.width);
     }
 
     /// Defer a nonzero height from the current boundary onward.
```

## version-17: `crates/before/src/version/measure/integral/deferred.rs:110:9`

- name: `crates/before/src/version/measure/integral/deferred.rs:110:9: replace Aggregate::weight -> u64 with 1`
- function: `Aggregate::weight`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/version/measure/integral/deferred.rs
+++ replace Aggregate::weight -> u64 with 1
@@ -102,17 +102,17 @@
         Self {
             heights,
             widths: SparseWidth::from_scaled(width),
         }
     }
 
     /// Data read when this group participates in a merge; even empty groups cost one.
     fn weight(&self) -> u64 {
-        (self.heights.stored_digit_count() + self.widths.stored_digit_count()).max(1) as u64
+        1 /* ~ changed by cargo-mutants ~ */
     }
 
     /// Merge a newer right group into this older left group.
     fn merge(&mut self, right: Self, total: &mut Accumulator) {
         // Pairs entirely within either half have already contributed. This
         // product accounts exactly for the pairs crossing this split.
         let heights = self.heights.to_bigint();
         if heights != BigInt::ZERO {
```

## version-18: `crates/before/src/version/measure/integral/deferred.rs:110:44`

- name: `crates/before/src/version/measure/integral/deferred.rs:110:44: replace + with * in Aggregate::weight`
- function: `Aggregate::weight`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/measure/integral/deferred.rs
+++ replace + with * in Aggregate::weight
@@ -102,17 +102,17 @@
         Self {
             heights,
             widths: SparseWidth::from_scaled(width),
         }
     }
 
     /// Data read when this group participates in a merge; even empty groups cost one.
     fn weight(&self) -> u64 {
-        (self.heights.stored_digit_count() + self.widths.stored_digit_count()).max(1) as u64
+        (self.heights.stored_digit_count() * /* ~ changed by cargo-mutants ~ */ self.widths.stored_digit_count()).max(1) as u64
     }
 
     /// Merge a newer right group into this older left group.
     fn merge(&mut self, right: Self, total: &mut Accumulator) {
         // Pairs entirely within either half have already contributed. This
         // product accounts exactly for the pairs crossing this split.
         let heights = self.heights.to_bigint();
         if heights != BigInt::ZERO {
```

## version-19: `crates/before/src/version/measure/integral/pair.rs:71:61`

- name: `crates/before/src/version/measure/integral/pair.rs:71:61: replace > with >= in Integrator::pair`
- function: `Integrator::pair`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/measure/integral/pair.rs
+++ replace > with >= in Integrator::pair
@@ -63,17 +63,17 @@
             let new_orientation = orientation(diff_order);
 
             // While the selected part is active, fold each input delta into the
             // integrand. Reversing orientation reverses which operand adds and
             // which subtracts.
             if current_orientation != 0 {
                 for (side, step) in [(Side::A, &step_a), (Side::B, &step_b)] {
                     if let Some(step) = step {
-                        let toward = if current_orientation > 0 {
+                        let toward = if current_orientation >= /* ~ changed by cargo-mutants ~ */ 0 {
                             side
                         } else {
                             side.other()
                         };
                         toward.fold(&mut integral.live, step);
                     }
                 }
             }
```

## version-20: `crates/before/src/version/measure/integral/width.rs:39:9`

- name: `crates/before/src/version/measure/integral/width.rs:39:9: replace ScaledWidth::is_zero -> bool with false`
- function: `ScaledWidth::is_zero`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/version/measure/integral/width.rs
+++ replace ScaledWidth::is_zero -> bool with false
@@ -31,17 +31,17 @@
     pub fn read(width: &Accumulator) -> Self {
         let (sign, magnitude, shift) = width.scaled_biguint_parts();
         debug_assert_ne!(sign, Ordering::Less, "interval widths only accumulate");
         Self { magnitude, shift }
     }
 
     /// Whether no regions have contributed a width.
     pub fn is_zero(&self) -> bool {
-        self.magnitude == BigUint::ZERO
+        false /* ~ changed by cargo-mutants ~ */
     }
 
     /// Add this relative width to another sum, retaining its scale.
     pub fn add_to(&self, total: &mut Accumulator) {
         total.add_shifted_limbs(self.shift, self.magnitude.iter_u64_digits());
     }
 
     /// Convert the width to balanced digits before multiplying by a height.
```

## version-21: `crates/before/src/version/measure/integral/width.rs:90:9`

- name: `crates/before/src/version/measure/integral/width.rs:90:9: replace SparseWidth::stored_digit_count -> usize with 0`
- function: `SparseWidth::stored_digit_count`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/version/measure/integral/width.rs
+++ replace SparseWidth::stored_digit_count -> usize with 0
@@ -82,17 +82,17 @@
             .filter(|&(_, digit)| digit != 0);
         let mut width = Self { digits: Vec::new() };
         width.combine(digits);
         width
     }
 
     /// Number of stored digits: the work needed to merge this width.
     pub fn stored_digit_count(&self) -> usize {
-        self.digits.len()
+        0 /* ~ changed by cargo-mutants ~ */
     }
 
     /// Add another width in one pass over both sparse digit sequences.
     pub fn add(&mut self, other: Self) {
         self.combine(other.digits.into_iter());
     }
 
     /// Merge ascending digits while restoring the balanced range at each position.
```

## version-22: `crates/before/src/version/measure/integral/width.rs:90:9`

- name: `crates/before/src/version/measure/integral/width.rs:90:9: replace SparseWidth::stored_digit_count -> usize with 1`
- function: `SparseWidth::stored_digit_count`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/version/measure/integral/width.rs
+++ replace SparseWidth::stored_digit_count -> usize with 1
@@ -82,17 +82,17 @@
             .filter(|&(_, digit)| digit != 0);
         let mut width = Self { digits: Vec::new() };
         width.combine(digits);
         width
     }
 
     /// Number of stored digits: the work needed to merge this width.
     pub fn stored_digit_count(&self) -> usize {
-        self.digits.len()
+        1 /* ~ changed by cargo-mutants ~ */
     }
 
     /// Add another width in one pass over both sparse digit sequences.
     pub fn add(&mut self, other: Self) {
         self.combine(other.digits.into_iter());
     }
 
     /// Merge ascending digits while restoring the balanced range at each position.
```

## version-23: `crates/before/src/version/measure/min_ticks.rs:69:17`

- name: `crates/before/src/version/measure/min_ticks.rs:69:17: replace > with >= in Count::min_ticks_for`
- function: `Count::min_ticks_for`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/measure/min_ticks.rs
+++ replace > with >= in Count::min_ticks_for
@@ -61,17 +61,17 @@
             }
             subtree_minima.open_subtrees(leaves.depth() - turn_depth);
 
             // Freeze a much wider accumulated change before the narrow delta
             // that exposed the imbalance can cause later work to revisit it.
             // The allowance absorbs small width fluctuations without changing
             // the asymptotic rule.
             if recent_height_change.stored_digit_count()
-                > accumulator::digit_len(height_change.magnitude()) + HEIGHT_FREEZE_ALLOWANCE_DIGITS
+                >= /* ~ changed by cargo-mutants ~ */ accumulator::digit_len(height_change.magnitude()) + HEIGHT_FREEZE_ALLOWANCE_DIGITS
             {
                 height_prefixes.freeze(&mut recent_height_change);
             }
 
             let leaf = height_prefixes.add_leaf(&recent_height_change, &mut answer);
             subtree_minima.observe_leaf(&leaf, &mut answer, &mut height_prefixes);
         }
```

## version-24: `crates/before/src/version/measure/min_ticks.rs:69:69`

- name: `crates/before/src/version/measure/min_ticks.rs:69:69: replace + with * in Count::min_ticks_for`
- function: `Count::min_ticks_for`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/measure/min_ticks.rs
+++ replace + with * in Count::min_ticks_for
@@ -61,17 +61,17 @@
             }
             subtree_minima.open_subtrees(leaves.depth() - turn_depth);
 
             // Freeze a much wider accumulated change before the narrow delta
             // that exposed the imbalance can cause later work to revisit it.
             // The allowance absorbs small width fluctuations without changing
             // the asymptotic rule.
             if recent_height_change.stored_digit_count()
-                > accumulator::digit_len(height_change.magnitude()) + HEIGHT_FREEZE_ALLOWANCE_DIGITS
+                > accumulator::digit_len(height_change.magnitude()) * /* ~ changed by cargo-mutants ~ */ HEIGHT_FREEZE_ALLOWANCE_DIGITS
             {
                 height_prefixes.freeze(&mut recent_height_change);
             }
 
             let leaf = height_prefixes.add_leaf(&recent_height_change, &mut answer);
             subtree_minima.observe_leaf(&leaf, &mut answer, &mut height_prefixes);
         }
```

## version-25: `crates/before/src/version/measure/min_ticks/minima/contributions.rs:89:38`

- name: `crates/before/src/version/measure/min_ticks/minima/contributions.rs:89:38: replace | with ^ in StoredContribution::inline`
- function: `StoredContribution::inline`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/measure/min_ticks/minima/contributions.rs
+++ replace | with ^ in StoredContribution::inline
@@ -81,17 +81,17 @@
 impl StoredContribution {
     /// Encode a contribution inline when both initial fields fit.
     fn inline(offset: &BigInt, prefix: usize) -> Option<Self> {
         let offset = i32::try_from(offset).ok()?;
         if prefix > PREFIX_MASK as usize {
             return None;
         }
         Some(Self(
-            u64::from(offset as u32) | (prefix as u64) << PREFIX_SHIFT,
+            u64::from(offset as u32) ^ /* ~ changed by cargo-mutants ~ */ (prefix as u64) << PREFIX_SHIFT,
         ))
     }
 
     /// Whether this word names an exact out-of-line contribution.
     fn is_spilled(self) -> bool {
         self.0 & SPILLED != 0
     }
```

## version-26: `crates/before/src/version/measure/min_ticks/minima/contributions.rs:164:36`

- name: `crates/before/src/version/measure/min_ticks/minima/contributions.rs:164:36: replace | with ^ in ContributionStore::spill`
- function: `ContributionStore::spill`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/measure/min_ticks/minima/contributions.rs
+++ replace | with ^ in ContributionStore::spill
@@ -156,17 +156,17 @@
             index
         } else {
             let index = self.slots.len();
             self.slots.push(Some(contribution));
             index
         };
         let index = u64::try_from(index).expect("contribution spill index fits u64");
         assert!(index < SPILLED, "contribution spill index fits 63 bits");
-        StoredContribution(SPILLED | index)
+        StoredContribution(SPILLED ^ /* ~ changed by cargo-mutants ~ */ index)
     }
 
     /// Count one subtree close, spilling if the inline count is full.
     pub fn increment(&mut self, contribution: &mut StoredContribution) {
         if contribution.is_spilled() {
             self.slots[contribution.spill_index()]
                 .as_mut()
                 .expect("a spilled contribution owns its slot")
```

## version-27: `crates/before/src/version/measure/min_ticks/minima/heights.rs:117:62`

- name: `crates/before/src/version/measure/min_ticks/minima/heights.rs:117:62: replace < with <= in HeightPrefixes::settle`
- function: `HeightPrefixes::settle`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/measure/min_ticks/minima/heights.rs
+++ replace < with <= in HeightPrefixes::settle
@@ -109,17 +109,17 @@
     pub fn settle(self, total: &mut Accumulator) {
         let mut suffix = 0i128;
         for (component, coefficient) in self.components.iter().zip(&self.coefficients).rev() {
             suffix += coefficient;
             Self::fold_repeated(
                 total,
                 component.magnitude(),
                 suffix.unsigned_abs(),
-                (component.sign() == Sign::Minus) != (suffix < 0),
+                (component.sign() == Sign::Minus) != (suffix <= /* ~ changed by cargo-mutants ~ */ 0),
             );
         }
         debug_assert_eq!(
             suffix, 1,
             "a nonempty binary tree has one more leaf than internal nodes"
         );
     }
```

## version-28: `crates/before/src/version/measure/min_ticks/minima/heights.rs:131:23`

- name: `crates/before/src/version/measure/min_ticks/minima/heights.rs:131:23: replace || with && in HeightPrefixes::fold_repeated`
- function: `HeightPrefixes::fold_repeated`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/measure/min_ticks/minima/heights.rs
+++ replace || with && in HeightPrefixes::fold_repeated
@@ -123,17 +123,17 @@
         );
     }
 
     /// Add or subtract `factor * count` in `total`.
     ///
     /// `count` is at most word-scale, so this performs one multiplication
     /// linear in `factor` rather than revisiting any absolute height.
     pub fn fold_repeated(total: &mut Accumulator, factor: &BigUint, count: u128, subtract: bool) {
-        if count == 0 || *factor == BigUint::ZERO {
+        if count == 0 && /* ~ changed by cargo-mutants ~ */ *factor == BigUint::ZERO {
             return;
         }
         let mut product = factor.clone();
         product *= BigUint::from(count);
         if subtract {
             total.sub_shifted_limbs(0, product.iter_u64_digits());
         } else {
             total.add_shifted_limbs(0, product.iter_u64_digits());
```

## version-29: `crates/before/src/version/overlay.rs:142:48`

- name: `crates/before/src/version/overlay.rs:142:48: replace > with >= in advance_set`
- function: `advance_set`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/overlay.rs
+++ replace > with >= in advance_set
@@ -134,17 +134,17 @@
 /// nothing. [`advance`] is the two-cursor form for operations that need the
 /// crossing values themselves.
 pub fn advance_set(set: &mut impl CursorSet) {
     let priority = set.priority();
     let mut deepest: Option<(usize, u64)> = None;
     for slot in priority.clone() {
         let depth = set.depth(slot);
         // Strict: the first slot in priority order achieving the maximum.
-        if deepest.is_none_or(|(_, max)| depth > max) {
+        if deepest.is_none_or(|(_, max)| depth >= /* ~ changed by cargo-mutants ~ */ max) {
             deepest = Some((slot, depth));
         }
     }
     let (deepest, _) = deepest.expect("a cursor set has at least one slot");
     let flip = set.step(deepest);
     for slot in priority {
         if slot != deepest && set.depth(slot) >= flip {
             let tied = set.step(slot);
```

## version-30: `crates/before/src/version/place/filter.rs:185:38`

- name: `crates/before/src/version/place/filter.rs:185:38: replace && with || in Comparison::fold`
- function: `Comparison::fold`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/place/filter.rs
+++ replace && with || in Comparison::fold
@@ -177,17 +177,17 @@
                 sign
             }
         };
         self.directions.fold(sign);
     }
 
     /// Fold one crossing unless it makes a private difference too wide.
     fn fold(&mut self, side: Side, probe: &Accumulator, bound: &Accumulator, step: &HeightChange) {
-        if self.difference.is_some() && HeightWidths::between(probe, bound) != HeightWidths::Close {
+        if self.difference.is_some() || /* ~ changed by cargo-mutants ~ */ HeightWidths::between(probe, bound) != HeightWidths::Close {
             self.difference = None;
             return;
         }
         if let Some(difference) = &mut self.difference {
             side.fold(difference, step);
         }
     }
```

## version-31: `crates/before/src/version/place/filter.rs:185:77`

- name: `crates/before/src/version/place/filter.rs:185:77: replace != with == in Comparison::fold`
- function: `Comparison::fold`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/place/filter.rs
+++ replace != with == in Comparison::fold
@@ -177,17 +177,17 @@
                 sign
             }
         };
         self.directions.fold(sign);
     }
 
     /// Fold one crossing unless it makes a private difference too wide.
     fn fold(&mut self, side: Side, probe: &Accumulator, bound: &Accumulator, step: &HeightChange) {
-        if self.difference.is_some() && HeightWidths::between(probe, bound) != HeightWidths::Close {
+        if self.difference.is_some() && HeightWidths::between(probe, bound) == /* ~ changed by cargo-mutants ~ */ HeightWidths::Close {
             self.difference = None;
             return;
         }
         if let Some(difference) = &mut self.difference {
             side.fold(difference, step);
         }
     }
```

## version-32: `crates/before/src/version/place/filter.rs:260:39`

- name: `crates/before/src/version/place/filter.rs:260:39: replace <= with > in Comparison::compare_heights`
- function: `Comparison::compare_heights`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/place/filter.rs
+++ replace <= with > in Comparison::compare_heights
@@ -252,17 +252,17 @@
                 }
             }
             // The shorter representation may expose the other side as wider;
             // classifying again removes its next layer of cancellation.
         }
 
         debug_assert_eq!(HeightWidths::between(probe, bound), HeightWidths::Close);
         let mut difference = Accumulator::new();
-        if probe.stored_digit_count() <= bound.stored_digit_count() {
+        if probe.stored_digit_count() > /* ~ changed by cargo-mutants ~ */ bound.stored_digit_count() {
             difference += &*probe;
             difference -= &*bound;
         } else {
             difference += &*bound;
             difference -= &*probe;
             difference = -difference;
         }
         let sign = difference.cmp_zero();
```

## version-33: `crates/before/src/version/place/filter.rs:360:26`

- name: `crates/before/src/version/place/filter.rs:360:26: replace -= with += in admits`
- function: `admits`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/place/filter.rs
+++ replace -= with += in admits
@@ -352,17 +352,17 @@
                 // earliest bail. (Both required demands are inclusive —
                 // `After` is `bound <= probe`, equality admitted.)
                 Demand::After if !directions.allows_ge() => return false,
                 Demand::Before if !directions.allows_le() => return false,
                 // A hole's subtracting direction refuted satisfies the hole:
                 // drop its cursor, its stream is never scanned further.
                 Demand::NotBefore | Demand::NotStrictlyBefore if !directions.allows_le() => {
                     *slot = None;
-                    live -= 1;
+                    live += /* ~ changed by cargo-mutants ~ */ 1;
                 }
                 Demand::NotAfter | Demand::NotStrictlyAfter if !directions.allows_ge() => {
                     *slot = None;
                     live -= 1;
                 }
                 _ => {}
             }
         }
```

## version-34: `crates/before/src/version/place/filter.rs:360:26`

- name: `crates/before/src/version/place/filter.rs:360:26: replace -= with /= in admits`
- function: `admits`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/place/filter.rs
+++ replace -= with /= in admits
@@ -352,17 +352,17 @@
                 // earliest bail. (Both required demands are inclusive —
                 // `After` is `bound <= probe`, equality admitted.)
                 Demand::After if !directions.allows_ge() => return false,
                 Demand::Before if !directions.allows_le() => return false,
                 // A hole's subtracting direction refuted satisfies the hole:
                 // drop its cursor, its stream is never scanned further.
                 Demand::NotBefore | Demand::NotStrictlyBefore if !directions.allows_le() => {
                     *slot = None;
-                    live -= 1;
+                    live /= /* ~ changed by cargo-mutants ~ */ 1;
                 }
                 Demand::NotAfter | Demand::NotStrictlyAfter if !directions.allows_ge() => {
                     *slot = None;
                     live -= 1;
                 }
                 _ => {}
             }
         }
```

## version-35: `crates/before/src/version/place/filter.rs:364:26`

- name: `crates/before/src/version/place/filter.rs:364:26: replace -= with += in admits`
- function: `admits`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/place/filter.rs
+++ replace -= with += in admits
@@ -356,17 +356,17 @@
                 // A hole's subtracting direction refuted satisfies the hole:
                 // drop its cursor, its stream is never scanned further.
                 Demand::NotBefore | Demand::NotStrictlyBefore if !directions.allows_le() => {
                     *slot = None;
                     live -= 1;
                 }
                 Demand::NotAfter | Demand::NotStrictlyAfter if !directions.allows_ge() => {
                     *slot = None;
-                    live -= 1;
+                    live += /* ~ changed by cargo-mutants ~ */ 1;
                 }
                 _ => {}
             }
         }
         // Required demands never drop, so an emptied walk is holes all
         // satisfied: membership holds with the probe unexhausted.
         if live == 0 {
             return true;
```

## version-36: `crates/before/src/version/place/filter.rs:364:26`

- name: `crates/before/src/version/place/filter.rs:364:26: replace -= with /= in admits`
- function: `admits`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/place/filter.rs
+++ replace -= with /= in admits
@@ -356,17 +356,17 @@
                 // A hole's subtracting direction refuted satisfies the hole:
                 // drop its cursor, its stream is never scanned further.
                 Demand::NotBefore | Demand::NotStrictlyBefore if !directions.allows_le() => {
                     *slot = None;
                     live -= 1;
                 }
                 Demand::NotAfter | Demand::NotStrictlyAfter if !directions.allows_ge() => {
                     *slot = None;
-                    live -= 1;
+                    live /= /* ~ changed by cargo-mutants ~ */ 1;
                 }
                 _ => {}
             }
         }
         // Required demands never drop, so an emptied walk is holes all
         // satisfied: membership holds with the probe unexhausted.
         if live == 0 {
             return true;
```

## version-37: `crates/before/src/version/place/filter.rs:603:22`

- name: `crates/before/src/version/place/filter.rs:603:22: replace -= with += in coverage`
- function: `coverage`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/place/filter.rs
+++ replace -= with += in coverage
@@ -595,17 +595,17 @@
                     }
                     if !side.hi.comparison.directions.allows_ge() {
                         side.hi.live = false;
                     }
                 }
             }
             if !side.lo.live && !side.hi.live {
                 *slot = None;
-                live -= 1;
+                live += /* ~ changed by cargo-mutants ~ */ 1;
             }
         }
         // A walk left holding only settled holes is decided: every hole was
         // refuted both ways, so nothing subtracts from the segment and nothing
         // more can change the verdict.
         if live == 0 {
             break;
         }
```

## version-38: `crates/before/src/version/place/filter.rs:603:22`

- name: `crates/before/src/version/place/filter.rs:603:22: replace -= with /= in coverage`
- function: `coverage`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/place/filter.rs
+++ replace -= with /= in coverage
@@ -595,17 +595,17 @@
                     }
                     if !side.hi.comparison.directions.allows_ge() {
                         side.hi.live = false;
                     }
                 }
             }
             if !side.lo.live && !side.hi.live {
                 *slot = None;
-                live -= 1;
+                live /= /* ~ changed by cargo-mutants ~ */ 1;
             }
         }
         // A walk left holding only settled holes is decided: every hole was
         // refuted both ways, so nothing subtracts from the segment and nothing
         // more can change the verdict.
         if live == 0 {
             break;
         }
```

## version-39: `crates/before/src/version/place/filter.rs:613:37`

- name: `crates/before/src/version/place/filter.rs:613:37: replace && with || in coverage`
- function: `coverage`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/place/filter.rs
+++ replace && with || in coverage
@@ -605,17 +605,17 @@
         }
         // A walk left holding only settled holes is decided: every hole was
         // refuted both ways, so nothing subtracts from the segment and nothing
         // more can change the verdict.
         if live == 0 {
             break;
         }
         // A probe endpoint whose every pair is settled stops being scanned.
-        walk.lo_live = walk.lo_live && walk.sides.iter().flatten().any(|side| side.lo.live);
+        walk.lo_live = walk.lo_live || /* ~ changed by cargo-mutants ~ */ walk.sides.iter().flatten().any(|side| side.lo.live);
         walk.hi_live = walk.hi_live && walk.sides.iter().flatten().any(|side| side.hi.live);
         debug_assert!(
             walk.sides.iter().flatten().all(|side| match side.demand {
                 Demand::After | Demand::Before => true,
                 Demand::NotBefore | Demand::NotStrictlyBefore => side.lo.live,
                 Demand::NotAfter | Demand::NotStrictlyAfter => side.hi.live,
             }),
             "a surviving hole keeps its dominated endpoint's pair live: settling \
```

## version-40: `crates/before/src/version/place/filter.rs:614:37`

- name: `crates/before/src/version/place/filter.rs:614:37: replace && with || in coverage`
- function: `coverage`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/place/filter.rs
+++ replace && with || in coverage
@@ -606,17 +606,17 @@
         // A walk left holding only settled holes is decided: every hole was
         // refuted both ways, so nothing subtracts from the segment and nothing
         // more can change the verdict.
         if live == 0 {
             break;
         }
         // A probe endpoint whose every pair is settled stops being scanned.
         walk.lo_live = walk.lo_live && walk.sides.iter().flatten().any(|side| side.lo.live);
-        walk.hi_live = walk.hi_live && walk.sides.iter().flatten().any(|side| side.hi.live);
+        walk.hi_live = walk.hi_live || /* ~ changed by cargo-mutants ~ */ walk.sides.iter().flatten().any(|side| side.hi.live);
         debug_assert!(
             walk.sides.iter().flatten().all(|side| match side.demand {
                 Demand::After | Demand::Before => true,
                 Demand::NotBefore | Demand::NotStrictlyBefore => side.lo.live,
                 Demand::NotAfter | Demand::NotStrictlyAfter => side.hi.live,
             }),
             "a surviving hole keeps its dominated endpoint's pair live: settling \
              it forces the dominating pair's settle at the same interval, and the \
```

## version-41: `crates/before/src/version/place/filter.rs:689:21`

- name: `crates/before/src/version/place/filter.rs:689:21: delete ! in SpanCursors<'_>::finish`
- function: `SpanCursors<'_>::finish`; genre: UnaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/place/filter.rs
+++ delete ! in SpanCursors<'_>::finish
@@ -681,17 +681,17 @@
             };
             if empty {
                 return Coverage::Empty;
             }
 
             // Full coverage requires this one demand to admit both endpoints.
             let admits_all = match side.demand {
                 Demand::After => {
-                    !side.lo.live || matches!(lo, Some(Ordering::Greater | Ordering::Equal))
+                     /* ~ changed by cargo-mutants ~ */side.lo.live || matches!(lo, Some(Ordering::Greater | Ordering::Equal))
                 }
                 Demand::Before => {
                     !side.hi.live || matches!(hi, Some(Ordering::Less | Ordering::Equal))
                 }
                 Demand::NotBefore => !matches!(lo, Some(Ordering::Less | Ordering::Equal)),
                 Demand::NotStrictlyBefore => lo != Some(Ordering::Less),
                 Demand::NotAfter => !matches!(hi, Some(Ordering::Greater | Ordering::Equal)),
                 Demand::NotStrictlyAfter => hi != Some(Ordering::Greater),
```

## version-42: `crates/before/src/version/place/filter.rs:692:21`

- name: `crates/before/src/version/place/filter.rs:692:21: delete ! in SpanCursors<'_>::finish`
- function: `SpanCursors<'_>::finish`; genre: UnaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/place/filter.rs
+++ delete ! in SpanCursors<'_>::finish
@@ -684,17 +684,17 @@
             }
 
             // Full coverage requires this one demand to admit both endpoints.
             let admits_all = match side.demand {
                 Demand::After => {
                     !side.lo.live || matches!(lo, Some(Ordering::Greater | Ordering::Equal))
                 }
                 Demand::Before => {
-                    !side.hi.live || matches!(hi, Some(Ordering::Less | Ordering::Equal))
+                     /* ~ changed by cargo-mutants ~ */side.hi.live || matches!(hi, Some(Ordering::Less | Ordering::Equal))
                 }
                 Demand::NotBefore => !matches!(lo, Some(Ordering::Less | Ordering::Equal)),
                 Demand::NotStrictlyBefore => lo != Some(Ordering::Less),
                 Demand::NotAfter => !matches!(hi, Some(Ordering::Greater | Ordering::Equal)),
                 Demand::NotStrictlyAfter => hi != Some(Ordering::Greater),
             };
             full_possible &= admits_all;
         }
```

## version-43: `crates/before/src/version/projection.rs:110:44`

- name: `crates/before/src/version/projection.rs:110:44: replace + with * in VersionWriter::project`
- function: `VersionWriter::project`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/projection.rs
+++ replace + with * in VersionWriter::project
@@ -102,17 +102,17 @@
         let mut ownership = PartyRegionReader::new(party);
         let mut height = Accumulator::new();
         height.add_shifted_limbs(0, first.iter_u64_digits());
         let mut owned = ownership.owned();
 
         // Most projections fit within the combined input size. Larger outputs
         // grow normally; this reservation only avoids reallocating the common
         // case.
-        let capacity = source.stored_len() + party.stored_len();
+        let capacity = source.stored_len() * /* ~ changed by cargo-mutants ~ */ party.stored_len();
         let mut out = VersionWriter::with_capacity(capacity);
         let opening = if owned { first } else { BigUint::ZERO };
         out.height(version.depth().max(ownership.depth()), &opening);
 
         while !(version.done() && ownership.done()) {
             // Inside an unowned Party region, every Version height projects to
             // zero. Consume a nested Version subtree in bulk and emit the one
             // zero leaf to which all of its regions collapse.
```

## version-44: `crates/before/src/version/projection.rs:355:36`

- name: `crates/before/src/version/projection.rs:355:36: replace != with == in Comparison<'a>::others_deepest`
- function: `Comparison<'a>::others_deepest`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/projection.rs
+++ replace != with == in Comparison<'a>::others_deepest
@@ -347,17 +347,17 @@
     }
 
     /// The deepest current region among the other cursors.
     ///
     /// A boundary deeper than this belongs only to `slot`, so it can be
     /// consumed without advancing another cursor.
     fn others_deepest(&self, slot: usize) -> u64 {
         self.priority()
-            .filter(|&other| other != slot)
+            .filter(|&other| other == /* ~ changed by cargo-mutants ~ */ slot)
             .map(|other| self.depth(other))
             .max()
             .expect("the walk has more than one cursor slot")
     }
 
     /// Consume runs of version boundaries hidden by an unowned region.
     ///
     /// If a mask does not own the current interval, event boundaries deeper
```

## version-45: `crates/before/src/version/projection.rs:375:9`

- name: `crates/before/src/version/projection.rs:375:9: replace Comparison<'a>::block_skip with ()`
- function: `Comparison<'a>::block_skip`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/version/projection.rs
+++ replace Comparison<'a>::block_skip with ()
@@ -367,43 +367,17 @@
     ///
     /// Every iteration consumes a boundary, so the shortcut remains linear in
     /// the input it crosses.
     ///
     /// A block can consume a side to exhaustion, so the caller re-checks
     /// [`done`](Self::done) before applying the advance law.
     ///
     fn block_skip(&mut self) {
-        loop {
-            let a_bound = self.others_deepest(Self::A);
-            if self.a_party.as_ref().is_some_and(|party| !party.owned())
-                && self.a.peek_flip() > a_bound
-            {
-                let mut net = Accumulator::new();
-                self.a.skip_deeper(a_bound, &mut net);
-                self.diff += &net;
-                if let Some(height_a) = &mut self.height_a {
-                    *height_a += &net;
-                }
-                continue;
-            }
-            let b_bound = self.others_deepest(Self::B);
-            if self.b_party.as_ref().is_some_and(|party| !party.owned())
-                && self.b.peek_flip() > b_bound
-            {
-                let mut net = Accumulator::new();
-                self.b.skip_deeper(b_bound, &mut net);
-                self.diff -= &net;
-                if let Some(height_b) = &mut self.height_b {
-                    *height_b += &net;
-                }
-                continue;
-            }
-            return;
-        }
+        () /* ~ changed by cargo-mutants ~ */
     }
 
     /// Advance to the next relevant boundary after skipping hidden runs.
     fn advance(&mut self) {
         self.block_skip();
         if self.done() {
             // A block consumed the last unowned run; the final interval's sign
             // folds in the caller's next round.
```

## version-46: `crates/before/src/version/range_minima.rs:319:26`

- name: `crates/before/src/version/range_minima.rs:319:26: replace > with >= in RangeMinima<P>::close`
- function: `RangeMinima<P>::close`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/range_minima.rs
+++ replace > with >= in RangeMinima<P>::close
@@ -311,17 +311,17 @@
             return Close::Retired;
         }
         match self
             .boundaries
             .pop()
             .expect("adjacent armed ranges have a boundary")
         {
             Entry::Equal(count) => {
-                if count > 1 {
+                if count >= /* ~ changed by cargo-mutants ~ */ 1 {
                     self.boundaries.push_equal(count - 1);
                 }
                 Close::Equal
             }
             Entry::Positive { boundary, payload } => {
                 self.anchor.defer(boundary);
                 Close::Lower(payload)
             }
```

## version-47: `crates/before/src/version/range_minima/anchor.rs:165:9`

- name: `crates/before/src/version/range_minima/anchor.rs:165:9: replace Anchor::gap_dominates_word -> Option<Ordering> with None`
- function: `Anchor::gap_dominates_word`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/version/range_minima/anchor.rs
+++ replace Anchor::gap_dominates_word -> Option<Ordering> with None
@@ -157,17 +157,17 @@
             return Ordering::Less;
         }
         self.resolve();
         self.gap.cmp_zero()
     }
 
     /// Certify the gap's sign when adding any word-sized offset cannot change it.
     pub(super) fn gap_dominates_word(&mut self) -> Option<Ordering> {
-        self.gap.cmp_zero_stable_under(64)
+        None /* ~ changed by cargo-mutants ~ */
     }
 
     /// Make the current height a confirmed new minimum and return `old_min - h`.
     pub(super) fn undercut_here(&mut self) -> Accumulator {
         let mut decrease = core::mem::take(&mut self.gap);
         decrease = -decrease;
         self.lower_by(decrease)
     }
```

## version-48: `crates/before/src/version/range_minima/boundaries.rs:59:9`

- name: `crates/before/src/version/range_minima/boundaries.rs:59:9: replace Boundaries<P>::is_empty -> bool with true`
- function: `Boundaries<P>::is_empty`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/version/range_minima/boundaries.rs
+++ replace Boundaries<P>::is_empty -> bool with true
@@ -51,17 +51,17 @@
             top_zeros: 0,
             payloads: Vec::new(),
             wide_values: Vec::new(),
         }
     }
 
     /// Whether no pair of armed ranges remains separated by a boundary.
     pub(super) fn is_empty(&self) -> bool {
-        self.top_zeros == 0 && self.positive.len() == 0
+        true /* ~ changed by cargo-mutants ~ */
     }
 
     /// Store the current zero run before a positive boundary covers it.
     fn flush_zeros(&mut self) {
         if self.top_zeros != 0 {
             self.positive.push(false);
             self.compact.push(self.top_zeros);
             self.top_zeros = 0;
```

## version-49: `crates/before/src/version/range_minima/boundaries.rs:59:29`

- name: `crates/before/src/version/range_minima/boundaries.rs:59:29: replace && with || in Boundaries<P>::is_empty`
- function: `Boundaries<P>::is_empty`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/range_minima/boundaries.rs
+++ replace && with || in Boundaries<P>::is_empty
@@ -51,17 +51,17 @@
             top_zeros: 0,
             payloads: Vec::new(),
             wide_values: Vec::new(),
         }
     }
 
     /// Whether no pair of armed ranges remains separated by a boundary.
     pub(super) fn is_empty(&self) -> bool {
-        self.top_zeros == 0 && self.positive.len() == 0
+        self.top_zeros == 0 || /* ~ changed by cargo-mutants ~ */ self.positive.len() == 0
     }
 
     /// Store the current zero run before a positive boundary covers it.
     fn flush_zeros(&mut self) {
         if self.top_zeros != 0 {
             self.positive.push(false);
             self.compact.push(self.top_zeros);
             self.top_zeros = 0;
```

## version-50: `crates/before/src/version/range_minima/boundary.rs:46:44`

- name: `crates/before/src/version/range_minima/boundary.rs:46:44: replace > with >= in Boundary::from_positive`
- function: `Boundary::from_positive`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/range_minima/boundary.rs
+++ replace > with >= in Boundary::from_positive
@@ -38,17 +38,17 @@
     /// A `u64` uses at most two base-2^32 digits. Materializing at most two
     /// digits has constant cost; a wider accumulator is retained directly.
     pub(super) fn from_positive(difference: Accumulator) -> Self {
         #[cfg(debug_assertions)]
         {
             let sign = difference.clone().cmp_zero();
             debug_assert_eq!(sign, Ordering::Greater, "boundaries are positive");
         }
-        if difference.stored_digit_count() > 2 {
+        if difference.stored_digit_count() >= /* ~ changed by cargo-mutants ~ */ 2 {
             return Self::Wide(difference);
         }
         match u64::try_from(difference) {
             Ok(word) => Self::Word(word),
             Err(difference) => Self::Wide(difference),
         }
     }
```

## version-51: `crates/before/src/version/range_minima/boundary.rs:106:30`

- name: `crates/before/src/version/range_minima/boundary.rs:106:30: replace -= with += in Boundary::lower_wide`
- function: `Boundary::lower_wide`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/range_minima/boundary.rs
+++ replace -= with += in Boundary::lower_wide
@@ -98,17 +98,17 @@
     /// Two base-2^32 digits are the first separation at which Accumulator's
     /// redundant representation can prove domination. If that comparison
     /// cannot decide, the values are close enough in stored width that subtracting
     /// once costs no more than processing comparable operands.
     fn lower_wide(mut boundary: Accumulator, mut decrease: Accumulator) -> Remainder {
         if decrease.stored_digit_count() >= boundary.stored_digit_count() + 2 {
             match decrease.cmp_zero_stable_under(boundary.stored_bits()) {
                 Some(Ordering::Greater) => {
-                    decrease -= &boundary;
+                    decrease += /* ~ changed by cargo-mutants ~ */ &boundary;
                     return Remainder::Decrease(decrease);
                 }
                 Some(_) => unreachable!("the decrease is positive"),
                 None => {}
             }
         }
         if boundary.stored_digit_count() >= decrease.stored_digit_count() + 2 {
             match boundary.cmp_zero_stable_under(decrease.stored_bits()) {
```

## version-52: `crates/before/src/version/range_minima/boundary.rs:113:42`

- name: `crates/before/src/version/range_minima/boundary.rs:113:42: replace >= with < in Boundary::lower_wide`
- function: `Boundary::lower_wide`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/range_minima/boundary.rs
+++ replace >= with < in Boundary::lower_wide
@@ -105,17 +105,17 @@
                 Some(Ordering::Greater) => {
                     decrease -= &boundary;
                     return Remainder::Decrease(decrease);
                 }
                 Some(_) => unreachable!("the decrease is positive"),
                 None => {}
             }
         }
-        if boundary.stored_digit_count() >= decrease.stored_digit_count() + 2 {
+        if boundary.stored_digit_count() < /* ~ changed by cargo-mutants ~ */ decrease.stored_digit_count() + 2 {
             match boundary.cmp_zero_stable_under(decrease.stored_bits()) {
                 Some(Ordering::Greater) => {
                     boundary -= &decrease;
                     return Remainder::Boundary(Self::from_positive(boundary));
                 }
                 Some(_) => unreachable!("stored boundaries are positive"),
                 None => {}
             }
```

## version-53: `crates/before/src/version/range_minima/boundary.rs:113:75`

- name: `crates/before/src/version/range_minima/boundary.rs:113:75: replace + with * in Boundary::lower_wide`
- function: `Boundary::lower_wide`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/range_minima/boundary.rs
+++ replace + with * in Boundary::lower_wide
@@ -105,17 +105,17 @@
                 Some(Ordering::Greater) => {
                     decrease -= &boundary;
                     return Remainder::Decrease(decrease);
                 }
                 Some(_) => unreachable!("the decrease is positive"),
                 None => {}
             }
         }
-        if boundary.stored_digit_count() >= decrease.stored_digit_count() + 2 {
+        if boundary.stored_digit_count() >= decrease.stored_digit_count() * /* ~ changed by cargo-mutants ~ */ 2 {
             match boundary.cmp_zero_stable_under(decrease.stored_bits()) {
                 Some(Ordering::Greater) => {
                     boundary -= &decrease;
                     return Remainder::Boundary(Self::from_positive(boundary));
                 }
                 Some(_) => unreachable!("stored boundaries are positive"),
                 None => {}
             }
```

## version-54: `crates/before/src/version/range_minima/boundary.rs:116:30`

- name: `crates/before/src/version/range_minima/boundary.rs:116:30: replace -= with += in Boundary::lower_wide`
- function: `Boundary::lower_wide`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/range_minima/boundary.rs
+++ replace -= with += in Boundary::lower_wide
@@ -108,17 +108,17 @@
                 }
                 Some(_) => unreachable!("the decrease is positive"),
                 None => {}
             }
         }
         if boundary.stored_digit_count() >= decrease.stored_digit_count() + 2 {
             match boundary.cmp_zero_stable_under(decrease.stored_bits()) {
                 Some(Ordering::Greater) => {
-                    boundary -= &decrease;
+                    boundary += /* ~ changed by cargo-mutants ~ */ &decrease;
                     return Remainder::Boundary(Self::from_positive(boundary));
                 }
                 Some(_) => unreachable!("stored boundaries are positive"),
                 None => {}
             }
         }
 
         boundary -= &decrease;
```

## version-55: `crates/before/src/version/range_minima/emission.rs:64:9`

- name: `crates/before/src/version/range_minima/emission.rs:64:9: replace RangeMinima<()>::emit_when_gap_dominates -> bool with false`
- function: `RangeMinima<()>::emit_when_gap_dominates`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/version/range_minima/emission.rs
+++ replace RangeMinima<()>::emit_when_gap_dominates -> bool with false
@@ -56,32 +56,17 @@
 
     /// Handle a small offset when leading digits already decide its ordering.
     ///
     /// This requires `A = m`. A dominating positive gap means the emission
     /// stays above the minimum. A negative gap means it undercuts; move the gap
     /// into `decrease = -gap - offset` and propagate that owned value. Returning
     /// `false` leaves the ordinary folded comparison to the caller.
     fn emit_when_gap_dominates(&mut self, offset: &BigInt) -> bool {
-        if self.deferred_live() || u64::try_from(offset.magnitude()).is_err() {
-            return false;
-        }
-        let Some(sign) = self.anchor.gap_dominates_word() else {
-            return false;
-        };
-        match sign {
-            Ordering::Greater => {}
-            Ordering::Less => {
-                let decrease = self.anchor.undercut_offset(offset);
-                self.propagate_drop(decrease, &mut (), |(), _| ());
-                self.anchor.set_gap_below_offset(offset);
-            }
-            Ordering::Equal => unreachable!("a decisive gap is nonzero"),
-        }
-        true
+        false /* ~ changed by cargo-mutants ~ */
     }
 
     /// Arm pending ranges at `h - below`, reusing the supplied accumulator.
     pub fn emit_below_accum(&mut self, below: Accumulator) {
         self.arm_below(below, &mut (), |_| (), |(), _| ());
     }
 
     /// Compare `h + above` with the innermost minimum without emitting it.
```

## version-56: `crates/before/src/version/shape.rs:174:48`

- name: `crates/before/src/version/shape.rs:174:48: replace > with >= in advance_refinement`
- function: `advance_refinement`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/shape.rs
+++ replace > with >= in advance_refinement
@@ -166,17 +166,17 @@
 pub fn advance_refinement<W: Refine>(walks: &mut [W]) -> bool {
     let mut deepest: Option<(usize, u64)> = None;
     for (slot, walk) in walks.iter().enumerate() {
         if walk.done() {
             continue;
         }
         let depth = walk.depth();
         // Strict: the first unexhausted slot achieving the maximum.
-        if deepest.is_none_or(|(_, max)| depth > max) {
+        if deepest.is_none_or(|(_, max)| depth >= /* ~ changed by cargo-mutants ~ */ max) {
             deepest = Some((slot, depth));
         }
     }
     let Some((deepest, _)) = deepest else {
         return true;
     };
     let flip = walks[deepest].advance();
     for (slot, walk) in walks.iter_mut().enumerate() {
```

## version-57: `crates/before/src/version/tick.rs:831:29`

- name: `crates/before/src/version/tick.rs:831:29: replace < with > in TickWalk<'_>::scan_max_consuming`
- function: `TickWalk<'_>::scan_max_consuming`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/tick.rs
+++ replace < with > in TickWalk<'_>::scan_max_consuming
@@ -823,17 +823,17 @@
             VersionTreeReader::at(self.version, self.pos()).node(),
             VersionNode::Leaf
         );
         let mut above = Extremum::max(Accumulator::new());
         let mut walk = VersionSubtreeReader::new();
         let first_leaf_depth = walk
             .descend(&mut self.cursor)
             .expect("a subtree has at least one leaf");
-        if first_leaf_depth < 2 {
+        if first_leaf_depth > /* ~ changed by cargo-mutants ~ */ 2 {
             // A tiny range is cheaper to fold leaf by leaf than to summarize.
             let step = self.consume_payload();
             above.fold(&step);
             while walk.descend(&mut self.cursor).is_some() {
                 let step = self.consume_payload();
                 above.fold(&step);
             }
         } else {
```

## version-58: `crates/before/src/version/tick.rs:831:29`

- name: `crates/before/src/version/tick.rs:831:29: replace < with == in TickWalk<'_>::scan_max_consuming`
- function: `TickWalk<'_>::scan_max_consuming`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/tick.rs
+++ replace < with == in TickWalk<'_>::scan_max_consuming
@@ -823,17 +823,17 @@
             VersionTreeReader::at(self.version, self.pos()).node(),
             VersionNode::Leaf
         );
         let mut above = Extremum::max(Accumulator::new());
         let mut walk = VersionSubtreeReader::new();
         let first_leaf_depth = walk
             .descend(&mut self.cursor)
             .expect("a subtree has at least one leaf");
-        if first_leaf_depth < 2 {
+        if first_leaf_depth == /* ~ changed by cargo-mutants ~ */ 2 {
             // A tiny range is cheaper to fold leaf by leaf than to summarize.
             let step = self.consume_payload();
             above.fold(&step);
             while walk.descend(&mut self.cursor).is_some() {
                 let step = self.consume_payload();
                 above.fold(&step);
             }
         } else {
```

## version-59: `crates/before/src/version/tick.rs:831:29`

- name: `crates/before/src/version/tick.rs:831:29: replace < with <= in TickWalk<'_>::scan_max_consuming`
- function: `TickWalk<'_>::scan_max_consuming`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/tick.rs
+++ replace < with <= in TickWalk<'_>::scan_max_consuming
@@ -823,17 +823,17 @@
             VersionTreeReader::at(self.version, self.pos()).node(),
             VersionNode::Leaf
         );
         let mut above = Extremum::max(Accumulator::new());
         let mut walk = VersionSubtreeReader::new();
         let first_leaf_depth = walk
             .descend(&mut self.cursor)
             .expect("a subtree has at least one leaf");
-        if first_leaf_depth < 2 {
+        if first_leaf_depth <= /* ~ changed by cargo-mutants ~ */ 2 {
             // A tiny range is cheaper to fold leaf by leaf than to summarize.
             let step = self.consume_payload();
             above.fold(&step);
             while walk.descend(&mut self.cursor).is_some() {
                 let step = self.consume_payload();
                 above.fold(&step);
             }
         } else {
```

## version-60: `crates/before/src/version/tick/memo.rs:110:9`

- name: `crates/before/src/version/tick/memo.rs:110:9: replace Memo::check_position -> u64 with 0`
- function: `Memo::check_position`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/version/tick/memo.rs
+++ replace Memo::check_position -> u64 with 0
@@ -102,17 +102,17 @@
             #[cfg(debug_assertions)]
             consumed_check: 0,
         }
     }
 
     /// Fold one position into the debug-only order checksum.
     #[cfg(debug_assertions)]
     pub fn check_position(check: u64, pos: u64) -> u64 {
-        (check ^ pos).wrapping_mul(0x0100_0000_01b3)
+        0 /* ~ changed by cargo-mutants ~ */
     }
 
     /// Number of lookahead minima reserved by the current scan.
     pub fn len(&self) -> usize {
         self.len
     }
 
     /// Reset for a new pre-scan, retaining allocations for reuse.
```

## version-61: `crates/before/src/version/tick/memo.rs:110:9`

- name: `crates/before/src/version/tick/memo.rs:110:9: replace Memo::check_position -> u64 with 1`
- function: `Memo::check_position`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/version/tick/memo.rs
+++ replace Memo::check_position -> u64 with 1
@@ -102,17 +102,17 @@
             #[cfg(debug_assertions)]
             consumed_check: 0,
         }
     }
 
     /// Fold one position into the debug-only order checksum.
     #[cfg(debug_assertions)]
     pub fn check_position(check: u64, pos: u64) -> u64 {
-        (check ^ pos).wrapping_mul(0x0100_0000_01b3)
+        1 /* ~ changed by cargo-mutants ~ */
     }
 
     /// Number of lookahead minima reserved by the current scan.
     pub fn len(&self) -> usize {
         self.len
     }
 
     /// Reset for a new pre-scan, retaining allocations for reuse.
```

## version-62: `crates/before/src/version/tick/memo.rs:110:16`

- name: `crates/before/src/version/tick/memo.rs:110:16: replace ^ with | in Memo::check_position`
- function: `Memo::check_position`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/tick/memo.rs
+++ replace ^ with | in Memo::check_position
@@ -102,17 +102,17 @@
             #[cfg(debug_assertions)]
             consumed_check: 0,
         }
     }
 
     /// Fold one position into the debug-only order checksum.
     #[cfg(debug_assertions)]
     pub fn check_position(check: u64, pos: u64) -> u64 {
-        (check ^ pos).wrapping_mul(0x0100_0000_01b3)
+        (check | /* ~ changed by cargo-mutants ~ */ pos).wrapping_mul(0x0100_0000_01b3)
     }
 
     /// Number of lookahead minima reserved by the current scan.
     pub fn len(&self) -> usize {
         self.len
     }
 
     /// Reset for a new pre-scan, retaining allocations for reuse.
```

## version-63: `crates/before/src/version/tick/memo.rs:110:16`

- name: `crates/before/src/version/tick/memo.rs:110:16: replace ^ with & in Memo::check_position`
- function: `Memo::check_position`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/tick/memo.rs
+++ replace ^ with & in Memo::check_position
@@ -102,17 +102,17 @@
             #[cfg(debug_assertions)]
             consumed_check: 0,
         }
     }
 
     /// Fold one position into the debug-only order checksum.
     #[cfg(debug_assertions)]
     pub fn check_position(check: u64, pos: u64) -> u64 {
-        (check ^ pos).wrapping_mul(0x0100_0000_01b3)
+        (check & /* ~ changed by cargo-mutants ~ */ pos).wrapping_mul(0x0100_0000_01b3)
     }
 
     /// Number of lookahead minima reserved by the current scan.
     pub fn len(&self) -> usize {
         self.len
     }
 
     /// Reset for a new pre-scan, retaining allocations for reuse.
```

## version-64: `crates/before/src/version/tick/memo.rs:120:9`

- name: `crates/before/src/version/tick/memo.rs:120:9: replace Memo::begin_scan with ()`
- function: `Memo::begin_scan`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/version/tick/memo.rs
+++ replace Memo::begin_scan with ()
@@ -112,27 +112,17 @@
 
     /// Number of lookahead minima reserved by the current scan.
     pub fn len(&self) -> usize {
         self.len
     }
 
     /// Reset for a new pre-scan, retaining allocations for reuse.
     pub fn begin_scan(&mut self) {
-        debug_assert_eq!(self.cursor, self.len, "the prior scan drained");
-        #[cfg(debug_assertions)]
-        debug_assert_eq!(
-            self.recorded_check, self.consumed_check,
-            "the walk consumed the recorded lookaheads in order"
-        );
-        for block in &mut self.blocks[..self.len.div_ceil(BLOCK_SLOTS)] {
-            block.clear();
-        }
-        self.len = 0;
-        self.cursor = 0;
+        () /* ~ changed by cargo-mutants ~ */
     }
 
     /// Reserve and return the next consumption-order slot.
     pub fn reserve(&mut self) -> usize {
         let slot = self.len;
         if slot / BLOCK_SLOTS == self.blocks.len() {
             self.blocks.push(Block::new());
         }
```

## version-65: `crates/before/src/version/tick/memo.rs:136:17`

- name: `crates/before/src/version/tick/memo.rs:136:17: replace / with % in Memo::reserve`
- function: `Memo::reserve`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/tick/memo.rs
+++ replace / with % in Memo::reserve
@@ -128,17 +128,17 @@
         }
         self.len = 0;
         self.cursor = 0;
     }
 
     /// Reserve and return the next consumption-order slot.
     pub fn reserve(&mut self) -> usize {
         let slot = self.len;
-        if slot / BLOCK_SLOTS == self.blocks.len() {
+        if slot % /* ~ changed by cargo-mutants ~ */ BLOCK_SLOTS == self.blocks.len() {
             self.blocks.push(Block::new());
         }
         self.len += 1;
         slot
     }
 
     /// Store a nonzero difference in a reserved slot.
     pub fn set_link(&mut self, slot: usize, link: StoredAccumulator) {
```

## version-66: `crates/before/src/version/tick/prescan.rs:233:9`

- name: `crates/before/src/version/tick/prescan.rs:233:9: replace PreScan<'a, 'm>::all_levels_resolved -> bool with true`
- function: `PreScan<'a, 'm>::all_levels_resolved`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/version/tick/prescan.rs
+++ replace PreScan<'a, 'm>::all_levels_resolved -> bool with true
@@ -225,17 +225,17 @@
                     }
                 }
             }
         }
     }
 
     /// Whether every nested recording level closed before the scan ended.
     pub fn all_levels_resolved(&self) -> bool {
-        self.suspend.is_empty()
+        true /* ~ changed by cargo-mutants ~ */
     }
 
     /// Reserve a memo slot for the lookahead starting at `pos`, in the order
     /// the main walk will encounter it.
     pub fn reserve(&mut self, pos: u64) -> usize {
         let slot = self.memo.reserve();
         #[cfg(debug_assertions)]
         {
```

## version-67: `crates/before/src/version/tick/prescan.rs:279:26`

- name: `crates/before/src/version/tick/prescan.rs:279:26: replace > with >= in PreScan<'a, 'm>::record`
- function: `PreScan<'a, 'm>::record`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/tick/prescan.rs
+++ replace > with >= in PreScan<'a, 'm>::record
@@ -271,17 +271,17 @@
         if self.reference_level == level {
             // The follower already holds current_minimum - previous_minimum.
             // For the outermost entry, the previous reference is scan-entry
             // height instead.
             let mut difference = self.minima.follower_take(REL_FOLLOWER);
             if difference.cmp_zero() == Ordering::Equal {
                 drop(difference);
             } else {
-                if level > 0 {
+                if level >= /* ~ changed by cargo-mutants ~ */ 0 {
                     // Add this difference to latest_minimum - first_minimum.
                     // Work follows the new difference's width, even if the sum
                     // is wide. Level zero has no deferred first entry.
                     self.latest_from_first += &difference;
                 }
                 self.memo.set_link(slot, StoredAccumulator::new(difference));
             }
         } else {
```

## version-68: `crates/before/src/version/tick/prescan.rs:414:29`

- name: `crates/before/src/version/tick/prescan.rs:414:29: replace < with == in PreScan<'a, 'm>::copy_range`
- function: `PreScan<'a, 'm>::copy_range`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/tick/prescan.rs
+++ replace < with == in PreScan<'a, 'm>::copy_range
@@ -406,17 +406,17 @@
     /// A block summary advances the input height to the range's final leaf and
     /// reports its minimum once. This has the same effect on open minima as
     /// reporting every leaf separately.
     fn copy_range(&mut self) {
         let mut walk = VersionSubtreeReader::new();
         let first_leaf_depth = walk
             .descend(&mut self.cursor)
             .expect("a subtree has at least one leaf");
-        if first_leaf_depth < 2 {
+        if first_leaf_depth == /* ~ changed by cargo-mutants ~ */ 2 {
             // A shallow first descent favors reporting leaves directly over
             // paying the fixed cost of a separate block summary.
             let _ = self.payload();
             self.emit_here();
             while walk.descend(&mut self.cursor).is_some() {
                 let _ = self.payload();
                 self.emit_here();
             }
```

## version-69: `crates/before/src/version/tick/prescan.rs:414:29`

- name: `crates/before/src/version/tick/prescan.rs:414:29: replace < with > in PreScan<'a, 'm>::copy_range`
- function: `PreScan<'a, 'm>::copy_range`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/tick/prescan.rs
+++ replace < with > in PreScan<'a, 'm>::copy_range
@@ -406,17 +406,17 @@
     /// A block summary advances the input height to the range's final leaf and
     /// reports its minimum once. This has the same effect on open minima as
     /// reporting every leaf separately.
     fn copy_range(&mut self) {
         let mut walk = VersionSubtreeReader::new();
         let first_leaf_depth = walk
             .descend(&mut self.cursor)
             .expect("a subtree has at least one leaf");
-        if first_leaf_depth < 2 {
+        if first_leaf_depth > /* ~ changed by cargo-mutants ~ */ 2 {
             // A shallow first descent favors reporting leaves directly over
             // paying the fixed cost of a separate block summary.
             let _ = self.payload();
             self.emit_here();
             while walk.descend(&mut self.cursor).is_some() {
                 let _ = self.payload();
                 self.emit_here();
             }
```

## version-70: `crates/before/src/version/tick/prescan.rs:414:29`

- name: `crates/before/src/version/tick/prescan.rs:414:29: replace < with <= in PreScan<'a, 'm>::copy_range`
- function: `PreScan<'a, 'm>::copy_range`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/tick/prescan.rs
+++ replace < with <= in PreScan<'a, 'm>::copy_range
@@ -406,17 +406,17 @@
     /// A block summary advances the input height to the range's final leaf and
     /// reports its minimum once. This has the same effect on open minima as
     /// reporting every leaf separately.
     fn copy_range(&mut self) {
         let mut walk = VersionSubtreeReader::new();
         let first_leaf_depth = walk
             .descend(&mut self.cursor)
             .expect("a subtree has at least one leaf");
-        if first_leaf_depth < 2 {
+        if first_leaf_depth <= /* ~ changed by cargo-mutants ~ */ 2 {
             // A shallow first descent favors reporting leaves directly over
             // paying the fixed cost of a separate block summary.
             let _ = self.payload();
             self.emit_here();
             while walk.descend(&mut self.cursor).is_some() {
                 let _ = self.payload();
                 self.emit_here();
             }
```

## version-71: `crates/before/src/version/tick/prescan.rs:446:29`

- name: `crates/before/src/version/tick/prescan.rs:446:29: replace < with > in PreScan<'a, 'm>::skip_collapse`
- function: `PreScan<'a, 'm>::skip_collapse`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/tick/prescan.rs
+++ replace < with > in PreScan<'a, 'm>::skip_collapse
@@ -438,17 +438,17 @@
     /// Simplification replaces this subtree with one leaf at least as high as
     /// the simplified right minimum. Only the right range can determine the
     /// branch minimum, so the left contributes input height movement alone.
     fn skip_collapse(&mut self) {
         let mut walk = VersionSubtreeReader::new();
         let first_leaf_depth = walk
             .descend(&mut self.cursor)
             .expect("a subtree has at least one leaf");
-        if first_leaf_depth < 2 {
+        if first_leaf_depth > /* ~ changed by cargo-mutants ~ */ 2 {
             // Read short descents directly. In particular, one wide leaf folds
             // its delta once, without first copying it into a block summary.
             let _ = self.payload();
             while walk.descend(&mut self.cursor).is_some() {
                 let _ = self.payload();
             }
             return;
         }
```

## version-72: `crates/before/src/version/tick/prescan.rs:446:29`

- name: `crates/before/src/version/tick/prescan.rs:446:29: replace < with <= in PreScan<'a, 'm>::skip_collapse`
- function: `PreScan<'a, 'm>::skip_collapse`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/tick/prescan.rs
+++ replace < with <= in PreScan<'a, 'm>::skip_collapse
@@ -438,17 +438,17 @@
     /// Simplification replaces this subtree with one leaf at least as high as
     /// the simplified right minimum. Only the right range can determine the
     /// branch minimum, so the left contributes input height movement alone.
     fn skip_collapse(&mut self) {
         let mut walk = VersionSubtreeReader::new();
         let first_leaf_depth = walk
             .descend(&mut self.cursor)
             .expect("a subtree has at least one leaf");
-        if first_leaf_depth < 2 {
+        if first_leaf_depth <= /* ~ changed by cargo-mutants ~ */ 2 {
             // Read short descents directly. In particular, one wide leaf folds
             // its delta once, without first copying it into a block summary.
             let _ = self.payload();
             while walk.descend(&mut self.cursor).is_some() {
                 let _ = self.payload();
             }
             return;
         }
```

## version-73: `crates/before/src/version/tick/prescan.rs:446:29`

- name: `crates/before/src/version/tick/prescan.rs:446:29: replace < with == in PreScan<'a, 'm>::skip_collapse`
- function: `PreScan<'a, 'm>::skip_collapse`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/tick/prescan.rs
+++ replace < with == in PreScan<'a, 'm>::skip_collapse
@@ -438,17 +438,17 @@
     /// Simplification replaces this subtree with one leaf at least as high as
     /// the simplified right minimum. Only the right range can determine the
     /// branch minimum, so the left contributes input height movement alone.
     fn skip_collapse(&mut self) {
         let mut walk = VersionSubtreeReader::new();
         let first_leaf_depth = walk
             .descend(&mut self.cursor)
             .expect("a subtree has at least one leaf");
-        if first_leaf_depth < 2 {
+        if first_leaf_depth == /* ~ changed by cargo-mutants ~ */ 2 {
             // Read short descents directly. In particular, one wide leaf folds
             // its delta once, without first copying it into a block summary.
             let _ = self.payload();
             while walk.descend(&mut self.cursor).is_some() {
                 let _ = self.payload();
             }
             return;
         }
```

## version-74: `crates/before/src/version/tick/prescan.rs:480:29`

- name: `crates/before/src/version/tick/prescan.rs:480:29: replace < with > in PreScan<'a, 'm>::max_range`
- function: `PreScan<'a, 'm>::max_range`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/tick/prescan.rs
+++ replace < with > in PreScan<'a, 'm>::max_range
@@ -472,17 +472,17 @@
             self.entry_net.is_none(),
             "a completed range emits before any raise scans for its maximum, so the entry net is already retired"
         );
         let mut above = Extremum::max(Accumulator::new());
         let mut walk = VersionSubtreeReader::new();
         let first_leaf_depth = walk
             .descend(&mut self.cursor)
             .expect("a subtree has at least one leaf");
-        if first_leaf_depth < 2 {
+        if first_leaf_depth > /* ~ changed by cargo-mutants ~ */ 2 {
             // Short descents are cheaper to fold directly.
             let step = self.payload();
             above.fold(&step);
             while walk.descend(&mut self.cursor).is_some() {
                 let step = self.payload();
                 above.fold(&step);
             }
         } else {
```

## version-75: `crates/before/src/version/tick/prescan.rs:480:29`

- name: `crates/before/src/version/tick/prescan.rs:480:29: replace < with <= in PreScan<'a, 'm>::max_range`
- function: `PreScan<'a, 'm>::max_range`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/tick/prescan.rs
+++ replace < with <= in PreScan<'a, 'm>::max_range
@@ -472,17 +472,17 @@
             self.entry_net.is_none(),
             "a completed range emits before any raise scans for its maximum, so the entry net is already retired"
         );
         let mut above = Extremum::max(Accumulator::new());
         let mut walk = VersionSubtreeReader::new();
         let first_leaf_depth = walk
             .descend(&mut self.cursor)
             .expect("a subtree has at least one leaf");
-        if first_leaf_depth < 2 {
+        if first_leaf_depth <= /* ~ changed by cargo-mutants ~ */ 2 {
             // Short descents are cheaper to fold directly.
             let step = self.payload();
             above.fold(&step);
             while walk.descend(&mut self.cursor).is_some() {
                 let step = self.payload();
                 above.fold(&step);
             }
         } else {
```

## version-76: `crates/before/src/version/tick/prescan.rs:480:29`

- name: `crates/before/src/version/tick/prescan.rs:480:29: replace < with == in PreScan<'a, 'm>::max_range`
- function: `PreScan<'a, 'm>::max_range`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/version/tick/prescan.rs
+++ replace < with == in PreScan<'a, 'm>::max_range
@@ -472,17 +472,17 @@
             self.entry_net.is_none(),
             "a completed range emits before any raise scans for its maximum, so the entry net is already retired"
         );
         let mut above = Extremum::max(Accumulator::new());
         let mut walk = VersionSubtreeReader::new();
         let first_leaf_depth = walk
             .descend(&mut self.cursor)
             .expect("a subtree has at least one leaf");
-        if first_leaf_depth < 2 {
+        if first_leaf_depth == /* ~ changed by cargo-mutants ~ */ 2 {
             // Short descents are cheaper to fold directly.
             let step = self.payload();
             above.fold(&step);
             while walk.descend(&mut self.cursor).is_some() {
                 let step = self.payload();
                 above.fold(&step);
             }
         } else {
```

## version-77: `crates/before/src/version/tick/prescan/suspended.rs:73:9`

- name: `crates/before/src/version/tick/prescan/suspended.rs:73:9: replace AccumulatorStack::is_empty -> bool with true`
- function: `AccumulatorStack::is_empty`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/version/tick/prescan/suspended.rs
+++ replace AccumulatorStack::is_empty -> bool with true
@@ -65,17 +65,17 @@
             StoredAccumulator::Wide(self.large.pop().expect("a wide tag has a wide value"))
         } else {
             StoredAccumulator::Small(self.small.pop().expect("a small tag has a small value"))
         })
     }
 
     /// Whether the stack is empty.
     fn is_empty(&self) -> bool {
-        self.wide.len() == 0
+        true /* ~ changed by cargo-mutants ~ */
     }
 }
 
 /// Suspended recording levels, stored in compact parallel stacks.
 pub struct Levels {
     /// Suspended differences from the outer recording references.
     first_from_outer: AccumulatorStack,
     /// Suspended differences between each level's latest and first minima.
```

## version-78: `crates/before/src/version/tick/prescan/suspended.rs:111:9`

- name: `crates/before/src/version/tick/prescan/suspended.rs:111:9: replace Levels::is_empty -> bool with true`
- function: `Levels::is_empty`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/version/tick/prescan/suspended.rs
+++ replace Levels::is_empty -> bool with true
@@ -103,17 +103,17 @@
             first_slots: Position::new(),
             level_deltas: PackedU64Stack::new(),
             levels: Position::new(),
         }
     }
 
     /// Whether no outer recording level is parked.
     pub fn is_empty(&self) -> bool {
-        self.first_from_outer.is_empty()
+        true /* ~ changed by cargo-mutants ~ */
     }
 
     /// Park one outer recording level.
     pub fn push(&mut self, level: Level) {
         self.first_from_outer.push(level.first_from_outer);
         self.latest_from_first.push(level.latest_from_first);
         self.has_first_slot.push(level.first_slot.is_some());
         if let Some(slot) = level.first_slot {
```

## version-79: `crates/before/src/version/tick/probe.rs:31:9`

- name: `crates/before/src/version/tick/probe.rs:31:9: replace RaiseProbe::kill with ()`
- function: `RaiseProbe::kill`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/version/tick/probe.rs
+++ replace RaiseProbe::kill with ()
@@ -23,18 +23,17 @@
             route: None,
             party_span,
             live: true,
         }
     }
 
     /// Stop probing once simplification has produced the tick result.
     pub fn kill(&mut self) {
-        self.live = false;
-        self.route = None;
+        () /* ~ changed by cargo-mutants ~ */
     }
 
     /// Combine two child costs and record the cheaper child.
     ///
     /// A branch adds one level of depth. Equal costs choose the right child so
     /// route selection is deterministic.
     pub fn join(&mut self, key: u64, left: Cost, right: Cost) -> Cost {
         if !self.live {
```

