# Mutation survivors: suanpan

30 survivors. Each entry: the cargo-mutants name, the enclosing function,
and the exact diff cargo-mutants applied (apply with `patch -p0` from the worktree root
after stripping the `*** ` header lines, or re-create it by hand).

## suanpan-1: `crates/suanpan/src/accumulator.rs:197:9`

- name: `crates/suanpan/src/accumulator.rs:197:9: replace Accumulator::reserve_digits with ()`
- function: `Accumulator::reserve_digits`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/suanpan/src/accumulator.rs
+++ replace Accumulator::reserve_digits with ()
@@ -189,17 +189,17 @@
     ///
     /// # Complexity
     ///
     /// O(1) time and space if the existing allocation suffices. Otherwise this
     /// performs at most one allocation and may copy retained storage;
     /// allocator cost, retained growth, and temporary space depend on the old
     /// and requested capacities.
     pub fn reserve_digits(&mut self, digits: usize) {
-        self.digits.reserve(digits);
+        () /* ~ changed by cargo-mutants ~ */
     }
 
     /// Compare the exact value with zero.
     ///
     /// The query may reduce the working width without changing the value. This
     /// is why it takes `&mut self`.
     ///
     /// # Complexity
```

## suanpan-2: `crates/suanpan/src/accumulator.rs:432:9`

- name: `crates/suanpan/src/accumulator.rs:432:9: replace <impl core::fmt::Debug for Accumulator>::fmt -> core::fmt::Result with Ok(Default::default())`
- function: `<impl core::fmt::Debug for Accumulator>::fmt`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/suanpan/src/accumulator.rs
+++ replace <impl core::fmt::Debug for Accumulator>::fmt -> core::fmt::Result with Ok(Default::default())
@@ -424,17 +424,14 @@
         Accumulator::new()
     }
 }
 
 /// Format the accumulator's diagnostic state, including retained storage.
 impl core::fmt::Debug for Accumulator {
     /// Include every field needed to diagnose arithmetic and storage behavior.
     fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
-        let mut fields = formatter.debug_struct("Accumulator");
-        fields.field("small", &self.small);
-        self.digits.debug_fields(&mut fields);
-        fields.finish()
+        Ok(Default::default()) /* ~ changed by cargo-mutants ~ */
     }
 }
 
 #[cfg(test)]
 mod tests;
```

## suanpan-3: `crates/suanpan/src/accumulator/conversions.rs:46:57`

- name: `crates/suanpan/src/accumulator/conversions.rs:46:57: replace | with ^ in Accumulator::primitive_parts`
- function: `Accumulator::primitive_parts`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/suanpan/src/accumulator/conversions.rs
+++ replace | with ^ in Accumulator::primitive_parts
@@ -38,17 +38,17 @@
     fn primitive_parts(&self) -> Option<(Ordering, u128)> {
         if let Some(value) = self.small {
             return Some((value.cmp(&0), value.unsigned_abs()));
         }
         let (sign, limbs) = self.signed_magnitude();
         match limbs.as_ref() {
             [] => Some((sign, 0)),
             [low] => Some((sign, u128::from(*low))),
-            [low, high] => Some((sign, u128::from(*low) | (u128::from(*high) << 64))),
+            [low, high] => Some((sign, u128::from(*low) ^ /* ~ changed by cargo-mutants ~ */ (u128::from(*high) << 64))),
             _ => None,
         }
     }
 
     /// Interpret a full primitive magnitude in the signed range, including MIN.
     fn primitive_signed(&self) -> Option<i128> {
         let (sign, magnitude) = self.primitive_parts()?;
         if sign == Ordering::Less {
```

## suanpan-4: `crates/suanpan/src/accumulator/digits.rs:106:9`

- name: `crates/suanpan/src/accumulator/digits.rs:106:9: replace Digits::reserve with ()`
- function: `Digits::reserve`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/suanpan/src/accumulator/digits.rs
+++ replace Digits::reserve with ()
@@ -98,18 +98,17 @@
         self.highest_nonzero = 0;
         self.lowest_written = usize::MAX;
         self.zero_ranges.clear();
         self.debug_assert_valid();
     }
 
     /// Reserve enough capacity for `count` digits without changing the value.
     pub fn reserve(&mut self, count: usize) {
-        self.digits
-            .reserve_exact(count.saturating_sub(self.digits.len()));
+        () /* ~ changed by cargo-mutants ~ */
     }
 
     /// The stored prefix, including digit zero even when the value is zero.
     pub fn stored_digits(&self) -> &[i64] {
         self.debug_assert_valid();
         &self.digits[..=self.highest_nonzero]
     }
```

## suanpan-5: `crates/suanpan/src/accumulator/digits.rs:123:9`

- name: `crates/suanpan/src/accumulator/digits.rs:123:9: replace Digits::debug_fields with ()`
- function: `Digits::debug_fields`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/suanpan/src/accumulator/digits.rs
+++ replace Digits::debug_fields with ()
@@ -115,21 +115,17 @@
 
     /// Count stored positions without inspecting the buffer.
     pub fn stored_digit_count(&self) -> usize {
         self.highest_nonzero + 1
     }
 
     /// Append the stored representation to the accumulator's debug record.
     pub fn debug_fields(&self, fields: &mut core::fmt::DebugStruct<'_, '_>) {
-        fields
-            .field("digits", &self.digits)
-            .field("highest_nonzero", &self.highest_nonzero)
-            .field("lowest_written", &self.lowest_written)
-            .field("zero_ranges", &self.zero_ranges);
+        () /* ~ changed by cargo-mutants ~ */
     }
 
     /// Whether the representation itself contains only zeros.
     pub fn is_known_zero(&self) -> bool {
         self.highest_nonzero == 0 && self.digits[0] == 0
     }
 
     /// Negate each stored digit; the symmetric range needs no carries.
```

## suanpan-6: `crates/suanpan/src/accumulator/digits.rs:247:9`

- name: `crates/suanpan/src/accumulator/digits.rs:247:9: replace Digits::debug_assert_valid with ()`
- function: `Digits::debug_assert_valid`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/suanpan/src/accumulator/digits.rs
+++ replace Digits::debug_assert_valid with ()
@@ -239,24 +239,17 @@
                 None => self.highest_nonzero - 1,
             };
         }
     }
 
     /// Check the constant-time representation invariants at method boundaries.
     #[inline]
     fn debug_assert_valid(&self) {
-        self.debug_assert_storage();
-        #[cfg(debug_assertions)]
-        {
-            debug_assert!(
-                self.highest_nonzero == 0 || self.digits[self.highest_nonzero] != 0,
-                "highest_nonzero names the last nonzero digit"
-            );
-        }
+        () /* ~ changed by cargo-mutants ~ */
     }
 
     /// Check the weaker invariant needed while sign compaction is in flight.
     #[inline]
     fn debug_assert_storage(&self) {
         #[cfg(debug_assertions)]
         {
             debug_assert!(!self.digits.is_empty());
```

## suanpan-7: `crates/suanpan/src/accumulator/digits.rs:260:9`

- name: `crates/suanpan/src/accumulator/digits.rs:260:9: replace Digits::debug_assert_storage with ()`
- function: `Digits::debug_assert_storage`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/suanpan/src/accumulator/digits.rs
+++ replace Digits::debug_assert_storage with ()
@@ -252,22 +252,11 @@
                 "highest_nonzero names the last nonzero digit"
             );
         }
     }
 
     /// Check the weaker invariant needed while sign compaction is in flight.
     #[inline]
     fn debug_assert_storage(&self) {
-        #[cfg(debug_assertions)]
-        {
-            debug_assert!(!self.digits.is_empty());
-            debug_assert!(self.highest_nonzero < self.digits.len());
-            debug_assert!(
-                i128::from(self.digits[self.highest_nonzero]).abs() < DIGIT_LIMIT,
-                "the recorded high digit stays within the redundant range"
-            );
-            if self.lowest_written != usize::MAX {
-                debug_assert!(self.lowest_written < self.digits.len());
-            }
-        }
+        () /* ~ changed by cargo-mutants ~ */
     }
 }
```

## suanpan-8: `crates/suanpan/src/accumulator/digits/normalize.rs:74:46`

- name: `crates/suanpan/src/accumulator/digits/normalize.rs:74:46: replace * with / in Digits::normalize`
- function: `Digits::normalize`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/suanpan/src/accumulator/digits/normalize.rs
+++ replace * with / in Digits::normalize
@@ -66,17 +66,17 @@
             carry = carry.div_euclid(base);
             highest = highest
                 .checked_add(1)
                 .expect("a normalized carry needs an addressable digit position");
             if highest == self.digits.len() {
                 self.digits.push(0);
             }
             touch(1);
-            self.digits[highest] = (polarity * remainder) as i64;
+            self.digits[highest] = (polarity / /* ~ changed by cargo-mutants ~ */ remainder) as i64;
         }
         debug_assert!(
             highest <= old_highest.saturating_add(1),
             "normalization grows by at most one digit"
         );
 
         while highest > 0 && self.digits[highest] == 0 {
             touch(1);
```

## suanpan-9: `crates/suanpan/src/accumulator/digits/normalize.rs:81:23`

- name: `crates/suanpan/src/accumulator/digits/normalize.rs:81:23: replace > with == in Digits::normalize`
- function: `Digits::normalize`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/suanpan/src/accumulator/digits/normalize.rs
+++ replace > with == in Digits::normalize
@@ -73,17 +73,17 @@
             touch(1);
             self.digits[highest] = (polarity * remainder) as i64;
         }
         debug_assert!(
             highest <= old_highest.saturating_add(1),
             "normalization grows by at most one digit"
         );
 
-        while highest > 0 && self.digits[highest] == 0 {
+        while highest == /* ~ changed by cargo-mutants ~ */ 0 && self.digits[highest] == 0 {
             touch(1);
             highest -= 1;
         }
         self.highest_nonzero = highest;
         self.zero_ranges.rebuild(&self.digits[..=highest]);
         self.debug_assert_valid();
     }
 }
```

## suanpan-10: `crates/suanpan/src/accumulator/digits/normalize.rs:81:23`

- name: `crates/suanpan/src/accumulator/digits/normalize.rs:81:23: replace > with < in Digits::normalize`
- function: `Digits::normalize`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/suanpan/src/accumulator/digits/normalize.rs
+++ replace > with < in Digits::normalize
@@ -73,17 +73,17 @@
             touch(1);
             self.digits[highest] = (polarity * remainder) as i64;
         }
         debug_assert!(
             highest <= old_highest.saturating_add(1),
             "normalization grows by at most one digit"
         );
 
-        while highest > 0 && self.digits[highest] == 0 {
+        while highest < /* ~ changed by cargo-mutants ~ */ 0 && self.digits[highest] == 0 {
             touch(1);
             highest -= 1;
         }
         self.highest_nonzero = highest;
         self.zero_ranges.rebuild(&self.digits[..=highest]);
         self.debug_assert_valid();
     }
 }
```

## suanpan-11: `crates/suanpan/src/accumulator/digits/normalize.rs:81:23`

- name: `crates/suanpan/src/accumulator/digits/normalize.rs:81:23: replace > with >= in Digits::normalize`
- function: `Digits::normalize`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/suanpan/src/accumulator/digits/normalize.rs
+++ replace > with >= in Digits::normalize
@@ -73,17 +73,17 @@
             touch(1);
             self.digits[highest] = (polarity * remainder) as i64;
         }
         debug_assert!(
             highest <= old_highest.saturating_add(1),
             "normalization grows by at most one digit"
         );
 
-        while highest > 0 && self.digits[highest] == 0 {
+        while highest >= /* ~ changed by cargo-mutants ~ */ 0 && self.digits[highest] == 0 {
             touch(1);
             highest -= 1;
         }
         self.highest_nonzero = highest;
         self.zero_ranges.rebuild(&self.digits[..=highest]);
         self.debug_assert_valid();
     }
 }
```

## suanpan-12: `crates/suanpan/src/accumulator/digits/normalize.rs:83:21`

- name: `crates/suanpan/src/accumulator/digits/normalize.rs:83:21: replace -= with += in Digits::normalize`
- function: `Digits::normalize`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/suanpan/src/accumulator/digits/normalize.rs
+++ replace -= with += in Digits::normalize
@@ -75,15 +75,15 @@
         }
         debug_assert!(
             highest <= old_highest.saturating_add(1),
             "normalization grows by at most one digit"
         );
 
         while highest > 0 && self.digits[highest] == 0 {
             touch(1);
-            highest -= 1;
+            highest += /* ~ changed by cargo-mutants ~ */ 1;
         }
         self.highest_nonzero = highest;
         self.zero_ranges.rebuild(&self.digits[..=highest]);
         self.debug_assert_valid();
     }
 }
```

## suanpan-13: `crates/suanpan/src/accumulator/digits/normalize.rs:83:21`

- name: `crates/suanpan/src/accumulator/digits/normalize.rs:83:21: replace -= with /= in Digits::normalize`
- function: `Digits::normalize`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/suanpan/src/accumulator/digits/normalize.rs
+++ replace -= with /= in Digits::normalize
@@ -75,15 +75,15 @@
         }
         debug_assert!(
             highest <= old_highest.saturating_add(1),
             "normalization grows by at most one digit"
         );
 
         while highest > 0 && self.digits[highest] == 0 {
             touch(1);
-            highest -= 1;
+            highest /= /* ~ changed by cargo-mutants ~ */ 1;
         }
         self.highest_nonzero = highest;
         self.zero_ranges.rebuild(&self.digits[..=highest]);
         self.debug_assert_valid();
     }
 }
```

## suanpan-14: `crates/suanpan/src/accumulator/digits/read.rs:90:22`

- name: `crates/suanpan/src/accumulator/digits/read.rs:90:22: replace >>= with <<= in Digits::read_digits`
- function: `Digits::read_digits`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/suanpan/src/accumulator/digits/read.rs
+++ replace >>= with <<= in Digits::read_digits
@@ -82,17 +82,17 @@
                     complement_carry, 0,
                     "complement of a nonzero low part cannot carry out"
                 );
             }
             let mut high = (-carry) as u128 - u128::from(low_nonzero);
             while high > 0 {
                 touch(1);
                 collected.push((high & u128::from(DIGIT_MASK)) as u32);
-                high >>= DIGIT_BITS;
+                high <<= /* ~ changed by cargo-mutants ~ */ DIGIT_BITS;
             }
             // |carry| ≥ 1 makes |value| ≥ 2^(32·len) − M > 0: never zero.
             (Ordering::Less, collected)
         } else {
             let mut high = carry as u128;
             while high > 0 {
                 touch(1);
                 collected.push((high & u128::from(DIGIT_MASK)) as u32);
```

## suanpan-15: `crates/suanpan/src/accumulator/digits/read.rs:99:22`

- name: `crates/suanpan/src/accumulator/digits/read.rs:99:22: replace >>= with <<= in Digits::read_digits`
- function: `Digits::read_digits`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/suanpan/src/accumulator/digits/read.rs
+++ replace >>= with <<= in Digits::read_digits
@@ -91,17 +91,17 @@
             }
             // |carry| ≥ 1 makes |value| ≥ 2^(32·len) − M > 0: never zero.
             (Ordering::Less, collected)
         } else {
             let mut high = carry as u128;
             while high > 0 {
                 touch(1);
                 collected.push((high & u128::from(DIGIT_MASK)) as u32);
-                high >>= DIGIT_BITS;
+                high <<= /* ~ changed by cargo-mutants ~ */ DIGIT_BITS;
             }
             let ordering = if collected.iter().all(|&digit| digit == 0) {
                 Ordering::Equal
             } else {
                 Ordering::Greater
             };
             (ordering, collected)
         }
```

## suanpan-16: `crates/suanpan/src/accumulator/digits/read.rs:115:40`

- name: `crates/suanpan/src/accumulator/digits/read.rs:115:40: replace | with ^ in pack_limbs`
- function: `pack_limbs`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/suanpan/src/accumulator/digits/read.rs
+++ replace | with ^ in pack_limbs
@@ -107,17 +107,17 @@
         }
     }
 }
 
 /// Pack unsigned base-2^32 digits into minimal little-endian 64-bit limbs.
 fn pack_limbs(digits: Vec<u32>) -> Vec<u64> {
     let mut limbs: Vec<u64> = digits
         .chunks(2)
-        .map(|pair| u64::from(pair[0]) | (pair.get(1).copied().map_or(0, u64::from) << 32))
+        .map(|pair| u64::from(pair[0]) ^ /* ~ changed by cargo-mutants ~ */ (pair.get(1).copied().map_or(0, u64::from) << 32))
         .collect();
     drop(digits);
     while limbs.last() == Some(&0) {
         limbs.pop();
     }
     debug_assert!(limbs.last().is_none_or(|&limb| limb != 0));
     limbs
 }
```

## suanpan-17: `crates/suanpan/src/accumulator/digits/zero_ranges.rs:51:9`

- name: `crates/suanpan/src/accumulator/digits/zero_ranges.rs:51:9: replace <impl core::fmt::Debug for ZeroRanges>::fmt -> core::fmt::Result with Ok(Default::default())`
- function: `<impl core::fmt::Debug for ZeroRanges>::fmt`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/suanpan/src/accumulator/digits/zero_ranges.rs
+++ replace <impl core::fmt::Debug for ZeroRanges>::fmt -> core::fmt::Result with Ok(Default::default())
@@ -43,29 +43,17 @@
     /// Multiple intervals ordered by their lower endpoint.
     Many(BTreeMap<usize, usize>),
 }
 
 /// Display known-zero ranges as their ordered endpoint map.
 impl core::fmt::Debug for ZeroRanges {
     /// Preserve the map representation in the accumulator's debug output.
     fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
-        let mut map = formatter.debug_map();
-        match &self.ranges {
-            Ranges::Empty => {}
-            Ranges::One(lo, hi) => {
-                map.entry(lo, hi);
-            }
-            Ranges::Many(ranges) => {
-                for (lo, hi) in ranges {
-                    map.entry(lo, hi);
-                }
-            }
-        }
-        map.finish()
+        Ok(Default::default()) /* ~ changed by cargo-mutants ~ */
     }
 }
 
 /// Record skipped ranges, trim writes from them, and consume them during scans.
 impl ZeroRanges {
     /// Rebuild every interior zero range after a full-buffer rewrite.
     pub fn rebuild(&mut self, digits: &[i64]) {
         self.clear();
```

## suanpan-18: `crates/suanpan/src/accumulator/digits/zero_ranges.rs:141:40`

- name: `crates/suanpan/src/accumulator/digits/zero_ranges.rs:141:40: replace < with <= in ZeroRanges::take_below`
- function: `ZeroRanges::take_below`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/suanpan/src/accumulator/digits/zero_ranges.rs
+++ replace < with <= in ZeroRanges::take_below
@@ -133,17 +133,17 @@
 
     /// Remove and return a known-zero range immediately below `above`.
     ///
     /// The returned index is its lower endpoint; every digit strictly
     /// between that index and `above` is zero.
     pub fn take_below(&mut self, above: usize) -> Option<usize> {
         match &mut self.ranges {
             Ranges::Empty => None,
-            Ranges::One(lo, hi) if *lo < above && *hi >= above => {
+            Ranges::One(lo, hi) if *lo <= /* ~ changed by cargo-mutants ~ */ above && *hi >= above => {
                 let lo = *lo;
                 debug_assert!(lo.saturating_add(1) < *hi);
                 self.ranges = Ranges::Empty;
                 Some(lo)
             }
             Ranges::One(_, _) => None,
             Ranges::Many(ranges) => {
                 let (&lo, &hi) = ranges.range(..above).next_back()?;
```

## suanpan-19: `crates/suanpan/src/accumulator/digits/zero_ranges.rs:150:23`

- name: `crates/suanpan/src/accumulator/digits/zero_ranges.rs:150:23: replace < with <= in ZeroRanges::take_below`
- function: `ZeroRanges::take_below`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/suanpan/src/accumulator/digits/zero_ranges.rs
+++ replace < with <= in ZeroRanges::take_below
@@ -142,17 +142,17 @@
                 let lo = *lo;
                 debug_assert!(lo.saturating_add(1) < *hi);
                 self.ranges = Ranges::Empty;
                 Some(lo)
             }
             Ranges::One(_, _) => None,
             Ranges::Many(ranges) => {
                 let (&lo, &hi) = ranges.range(..above).next_back()?;
-                if hi < above {
+                if hi <= /* ~ changed by cargo-mutants ~ */ above {
                     return None;
                 }
                 debug_assert!(lo.saturating_add(1) < hi);
                 ranges.remove(&lo);
                 self.compact_storage();
                 Some(lo)
             }
         }
```

## suanpan-20: `crates/suanpan/src/accumulator/digits/zero_ranges.rs:170:44`

- name: `crates/suanpan/src/accumulator/digits/zero_ranges.rs:170:44: replace match guard *old_lo == lo with false in ZeroRanges::insert`
- function: `ZeroRanges::insert`; genre: MatchArmGuard; outcome: MissedMutant

```diff
--- crates/suanpan/src/accumulator/digits/zero_ranges.rs
+++ replace match guard *old_lo == lo with false in ZeroRanges::insert
@@ -162,17 +162,17 @@
     pub fn clear(&mut self) {
         self.ranges = Ranges::Empty;
     }
 
     /// Insert one range, promoting to ordered storage only for a second range.
     fn insert(&mut self, lo: usize, hi: usize) {
         match &mut self.ranges {
             Ranges::Empty => self.ranges = Ranges::One(lo, hi),
-            Ranges::One(old_lo, old_hi) if *old_lo == lo => *old_hi = hi,
+            Ranges::One(old_lo, old_hi) if false /* ~ changed by cargo-mutants ~ */ => *old_hi = hi,
             Ranges::One(old_lo, old_hi) => {
                 let mut ranges = BTreeMap::new();
                 ranges.insert(*old_lo, *old_hi);
                 ranges.insert(lo, hi);
                 self.ranges = Ranges::Many(ranges);
             }
             Ranges::Many(ranges) => {
                 ranges.insert(lo, hi);
```

## suanpan-21: `crates/suanpan/src/accumulator/digits/zero_ranges.rs:185:9`

- name: `crates/suanpan/src/accumulator/digits/zero_ranges.rs:185:9: replace ZeroRanges::last -> Option<(usize, usize)> with None`
- function: `ZeroRanges::last`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/suanpan/src/accumulator/digits/zero_ranges.rs
+++ replace ZeroRanges::last -> Option<(usize, usize)> with None
@@ -177,21 +177,17 @@
             Ranges::Many(ranges) => {
                 ranges.insert(lo, hi);
             }
         }
     }
 
     /// Highest range, if any.
     fn last(&self) -> Option<(usize, usize)> {
-        match &self.ranges {
-            Ranges::Empty => None,
-            Ranges::One(lo, hi) => Some((*lo, *hi)),
-            Ranges::Many(ranges) => ranges.last_key_value().map(|(&lo, &hi)| (lo, hi)),
-        }
+        None /* ~ changed by cargo-mutants ~ */
     }
 
     /// Return from tree storage when at most one range remains.
     fn compact_storage(&mut self) {
         let replacement = match &self.ranges {
             Ranges::Many(ranges) if ranges.is_empty() => Some(Ranges::Empty),
             Ranges::Many(ranges) if ranges.len() == 1 => {
                 let (&lo, &hi) = ranges.first_key_value().expect("the map has one range");
```

## suanpan-22: `crates/suanpan/src/accumulator/digits/zero_ranges.rs:185:9`

- name: `crates/suanpan/src/accumulator/digits/zero_ranges.rs:185:9: replace ZeroRanges::last -> Option<(usize, usize)> with Some((0, 0))`
- function: `ZeroRanges::last`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/suanpan/src/accumulator/digits/zero_ranges.rs
+++ replace ZeroRanges::last -> Option<(usize, usize)> with Some((0, 0))
@@ -177,21 +177,17 @@
             Ranges::Many(ranges) => {
                 ranges.insert(lo, hi);
             }
         }
     }
 
     /// Highest range, if any.
     fn last(&self) -> Option<(usize, usize)> {
-        match &self.ranges {
-            Ranges::Empty => None,
-            Ranges::One(lo, hi) => Some((*lo, *hi)),
-            Ranges::Many(ranges) => ranges.last_key_value().map(|(&lo, &hi)| (lo, hi)),
-        }
+        Some((0, 0)) /* ~ changed by cargo-mutants ~ */
     }
 
     /// Return from tree storage when at most one range remains.
     fn compact_storage(&mut self) {
         let replacement = match &self.ranges {
             Ranges::Many(ranges) if ranges.is_empty() => Some(Ranges::Empty),
             Ranges::Many(ranges) if ranges.len() == 1 => {
                 let (&lo, &hi) = ranges.first_key_value().expect("the map has one range");
```

## suanpan-23: `crates/suanpan/src/accumulator/digits/zero_ranges.rs:185:9`

- name: `crates/suanpan/src/accumulator/digits/zero_ranges.rs:185:9: replace ZeroRanges::last -> Option<(usize, usize)> with Some((1, 0))`
- function: `ZeroRanges::last`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/suanpan/src/accumulator/digits/zero_ranges.rs
+++ replace ZeroRanges::last -> Option<(usize, usize)> with Some((1, 0))
@@ -177,21 +177,17 @@
             Ranges::Many(ranges) => {
                 ranges.insert(lo, hi);
             }
         }
     }
 
     /// Highest range, if any.
     fn last(&self) -> Option<(usize, usize)> {
-        match &self.ranges {
-            Ranges::Empty => None,
-            Ranges::One(lo, hi) => Some((*lo, *hi)),
-            Ranges::Many(ranges) => ranges.last_key_value().map(|(&lo, &hi)| (lo, hi)),
-        }
+        Some((1, 0)) /* ~ changed by cargo-mutants ~ */
     }
 
     /// Return from tree storage when at most one range remains.
     fn compact_storage(&mut self) {
         let replacement = match &self.ranges {
             Ranges::Many(ranges) if ranges.is_empty() => Some(Ranges::Empty),
             Ranges::Many(ranges) if ranges.len() == 1 => {
                 let (&lo, &hi) = ranges.first_key_value().expect("the map has one range");
```

## suanpan-24: `crates/suanpan/src/accumulator/digits/zero_ranges.rs:194:9`

- name: `crates/suanpan/src/accumulator/digits/zero_ranges.rs:194:9: replace ZeroRanges::compact_storage with ()`
- function: `ZeroRanges::compact_storage`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/suanpan/src/accumulator/digits/zero_ranges.rs
+++ replace ZeroRanges::compact_storage with ()
@@ -186,27 +186,17 @@
             Ranges::Empty => None,
             Ranges::One(lo, hi) => Some((*lo, *hi)),
             Ranges::Many(ranges) => ranges.last_key_value().map(|(&lo, &hi)| (lo, hi)),
         }
     }
 
     /// Return from tree storage when at most one range remains.
     fn compact_storage(&mut self) {
-        let replacement = match &self.ranges {
-            Ranges::Many(ranges) if ranges.is_empty() => Some(Ranges::Empty),
-            Ranges::Many(ranges) if ranges.len() == 1 => {
-                let (&lo, &hi) = ranges.first_key_value().expect("the map has one range");
-                Some(Ranges::One(lo, hi))
-            }
-            _ => None,
-        };
-        if let Some(replacement) = replacement {
-            self.ranges = replacement;
-        }
+        () /* ~ changed by cargo-mutants ~ */
     }
 
     /// Check the range invariants exercised by the operation-sequence tests.
     #[cfg(test)]
     pub fn assert_invariants(&self, digits: &[i64], highest_nonzero: usize, schedule: &[u8]) {
         let mut previous_end = 0;
         let check = |lo: usize, hi: usize, previous_end: &mut usize| {
             assert!(
```

## suanpan-25: `crates/suanpan/src/accumulator/digits/zero_ranges.rs:195:37`

- name: `crates/suanpan/src/accumulator/digits/zero_ranges.rs:195:37: replace match guard ranges.is_empty() with true in ZeroRanges::compact_storage`
- function: `ZeroRanges::compact_storage`; genre: MatchArmGuard; outcome: MissedMutant

```diff
--- crates/suanpan/src/accumulator/digits/zero_ranges.rs
+++ replace match guard ranges.is_empty() with true in ZeroRanges::compact_storage
@@ -187,17 +187,17 @@
             Ranges::One(lo, hi) => Some((*lo, *hi)),
             Ranges::Many(ranges) => ranges.last_key_value().map(|(&lo, &hi)| (lo, hi)),
         }
     }
 
     /// Return from tree storage when at most one range remains.
     fn compact_storage(&mut self) {
         let replacement = match &self.ranges {
-            Ranges::Many(ranges) if ranges.is_empty() => Some(Ranges::Empty),
+            Ranges::Many(ranges) if true /* ~ changed by cargo-mutants ~ */ => Some(Ranges::Empty),
             Ranges::Many(ranges) if ranges.len() == 1 => {
                 let (&lo, &hi) = ranges.first_key_value().expect("the map has one range");
                 Some(Ranges::One(lo, hi))
             }
             _ => None,
         };
         if let Some(replacement) = replacement {
             self.ranges = replacement;
```

## suanpan-26: `crates/suanpan/src/accumulator/digits/zero_ranges.rs:195:37`

- name: `crates/suanpan/src/accumulator/digits/zero_ranges.rs:195:37: replace match guard ranges.is_empty() with false in ZeroRanges::compact_storage`
- function: `ZeroRanges::compact_storage`; genre: MatchArmGuard; outcome: MissedMutant

```diff
--- crates/suanpan/src/accumulator/digits/zero_ranges.rs
+++ replace match guard ranges.is_empty() with false in ZeroRanges::compact_storage
@@ -187,17 +187,17 @@
             Ranges::One(lo, hi) => Some((*lo, *hi)),
             Ranges::Many(ranges) => ranges.last_key_value().map(|(&lo, &hi)| (lo, hi)),
         }
     }
 
     /// Return from tree storage when at most one range remains.
     fn compact_storage(&mut self) {
         let replacement = match &self.ranges {
-            Ranges::Many(ranges) if ranges.is_empty() => Some(Ranges::Empty),
+            Ranges::Many(ranges) if false /* ~ changed by cargo-mutants ~ */ => Some(Ranges::Empty),
             Ranges::Many(ranges) if ranges.len() == 1 => {
                 let (&lo, &hi) = ranges.first_key_value().expect("the map has one range");
                 Some(Ranges::One(lo, hi))
             }
             _ => None,
         };
         if let Some(replacement) = replacement {
             self.ranges = replacement;
```

## suanpan-27: `crates/suanpan/src/accumulator/digits/zero_ranges.rs:196:37`

- name: `crates/suanpan/src/accumulator/digits/zero_ranges.rs:196:37: replace match guard ranges.len() == 1 with true in ZeroRanges::compact_storage`
- function: `ZeroRanges::compact_storage`; genre: MatchArmGuard; outcome: MissedMutant

```diff
--- crates/suanpan/src/accumulator/digits/zero_ranges.rs
+++ replace match guard ranges.len() == 1 with true in ZeroRanges::compact_storage
@@ -188,17 +188,17 @@
             Ranges::Many(ranges) => ranges.last_key_value().map(|(&lo, &hi)| (lo, hi)),
         }
     }
 
     /// Return from tree storage when at most one range remains.
     fn compact_storage(&mut self) {
         let replacement = match &self.ranges {
             Ranges::Many(ranges) if ranges.is_empty() => Some(Ranges::Empty),
-            Ranges::Many(ranges) if ranges.len() == 1 => {
+            Ranges::Many(ranges) if true /* ~ changed by cargo-mutants ~ */ => {
                 let (&lo, &hi) = ranges.first_key_value().expect("the map has one range");
                 Some(Ranges::One(lo, hi))
             }
             _ => None,
         };
         if let Some(replacement) = replacement {
             self.ranges = replacement;
         }
```

## suanpan-28: `crates/suanpan/src/accumulator/digits/zero_ranges.rs:196:37`

- name: `crates/suanpan/src/accumulator/digits/zero_ranges.rs:196:37: replace match guard ranges.len() == 1 with false in ZeroRanges::compact_storage`
- function: `ZeroRanges::compact_storage`; genre: MatchArmGuard; outcome: MissedMutant

```diff
--- crates/suanpan/src/accumulator/digits/zero_ranges.rs
+++ replace match guard ranges.len() == 1 with false in ZeroRanges::compact_storage
@@ -188,17 +188,17 @@
             Ranges::Many(ranges) => ranges.last_key_value().map(|(&lo, &hi)| (lo, hi)),
         }
     }
 
     /// Return from tree storage when at most one range remains.
     fn compact_storage(&mut self) {
         let replacement = match &self.ranges {
             Ranges::Many(ranges) if ranges.is_empty() => Some(Ranges::Empty),
-            Ranges::Many(ranges) if ranges.len() == 1 => {
+            Ranges::Many(ranges) if false /* ~ changed by cargo-mutants ~ */ => {
                 let (&lo, &hi) = ranges.first_key_value().expect("the map has one range");
                 Some(Ranges::One(lo, hi))
             }
             _ => None,
         };
         if let Some(replacement) = replacement {
             self.ranges = replacement;
         }
```

## suanpan-29: `crates/suanpan/src/accumulator/digits/zero_ranges.rs:196:50`

- name: `crates/suanpan/src/accumulator/digits/zero_ranges.rs:196:50: replace == with != in ZeroRanges::compact_storage`
- function: `ZeroRanges::compact_storage`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/suanpan/src/accumulator/digits/zero_ranges.rs
+++ replace == with != in ZeroRanges::compact_storage
@@ -188,17 +188,17 @@
             Ranges::Many(ranges) => ranges.last_key_value().map(|(&lo, &hi)| (lo, hi)),
         }
     }
 
     /// Return from tree storage when at most one range remains.
     fn compact_storage(&mut self) {
         let replacement = match &self.ranges {
             Ranges::Many(ranges) if ranges.is_empty() => Some(Ranges::Empty),
-            Ranges::Many(ranges) if ranges.len() == 1 => {
+            Ranges::Many(ranges) if ranges.len() != /* ~ changed by cargo-mutants ~ */ 1 => {
                 let (&lo, &hi) = ranges.first_key_value().expect("the map has one range");
                 Some(Ranges::One(lo, hi))
             }
             _ => None,
         };
         if let Some(replacement) = replacement {
             self.ranges = replacement;
         }
```

## suanpan-30: `crates/suanpan/src/accumulator/operators.rs:382:23`

- name: `crates/suanpan/src/accumulator/operators.rs:382:23: replace || with && in Accumulator::shift_left`
- function: `Accumulator::shift_left`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/suanpan/src/accumulator/operators.rs
+++ replace || with && in Accumulator::shift_left
@@ -374,17 +374,17 @@
 }
 
 shifts!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);
 
 /// Keep the shift kernel independent of the caller's primitive count type.
 impl Accumulator {
     /// Shift exactly, retaining the small representation whenever it fits.
     fn shift_left(&mut self, shift: u64) {
-        if shift == 0 || self.is_known_zero() {
+        if shift == 0 && /* ~ changed by cargo-mutants ~ */ self.is_known_zero() {
             return;
         }
         if let Some(value) = self.small {
             touch(1);
             if shift <= SMALL_SHIFT_MAX {
                 let shifted = value << shift;
                 if shifted.unsigned_abs() <= SMALL_MAX {
                     self.small = Some(shifted);
```

