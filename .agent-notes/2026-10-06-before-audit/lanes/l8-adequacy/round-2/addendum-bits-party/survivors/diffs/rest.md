# Mutation survivors: rest

36 survivors. Each entry: the cargo-mutants name, the enclosing function,
and the exact diff cargo-mutants applied (apply with `patch -p0` from the worktree root
after stripping the `*** ` header lines, or re-create it by hand).

## rest-1: `crates/before/src/borsh_impls.rs:116:9`

- name: `crates/before/src/borsh_impls.rs:116:9: replace <impl BitRead for StreamBitsReader<'_, R>>::position -> u64 with 0`
- function: `<impl BitRead for StreamBitsReader<'_, R>>::position`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/borsh_impls.rs
+++ replace <impl BitRead for StreamBitsReader<'_, R>>::position -> u64 with 0
@@ -108,17 +108,17 @@
         // Count the logical bit consumed; the meter observes the decoder's
         // work rather than how reads are batched.
         scan::record_bits(1);
         self.position += 1;
         Ok(bit)
     }
 
     fn position(&self) -> u64 {
-        self.position
+        0 /* ~ changed by cargo-mutants ~ */
     }
 
     fn read_gamma(&mut self) -> Result<BigUint, Decode> {
         // Word fast path over the bytes already read, exactly as
         // the in-memory decoder: the window's proven bits end at the
         // buffer's end, so it can never consume — or even inspect — a byte
         // the reader has not yielded, and speculative reads (which would
         // steal bytes from the next borsh field) are impossible by
```

## rest-2: `crates/before/src/borsh_impls.rs:116:9`

- name: `crates/before/src/borsh_impls.rs:116:9: replace <impl BitRead for StreamBitsReader<'_, R>>::position -> u64 with 1`
- function: `<impl BitRead for StreamBitsReader<'_, R>>::position`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/borsh_impls.rs
+++ replace <impl BitRead for StreamBitsReader<'_, R>>::position -> u64 with 1
@@ -108,17 +108,17 @@
         // Count the logical bit consumed; the meter observes the decoder's
         // work rather than how reads are batched.
         scan::record_bits(1);
         self.position += 1;
         Ok(bit)
     }
 
     fn position(&self) -> u64 {
-        self.position
+        1 /* ~ changed by cargo-mutants ~ */
     }
 
     fn read_gamma(&mut self) -> Result<BigUint, Decode> {
         // Word fast path over the bytes already read, exactly as
         // the in-memory decoder: the window's proven bits end at the
         // buffer's end, so it can never consume — or even inspect — a byte
         // the reader has not yielded, and speculative reads (which would
         // steal bytes from the next borsh field) are impossible by
```

## rest-3: `crates/before/src/borsh_impls.rs:129:80`

- name: `crates/before/src/borsh_impls.rs:129:80: replace * with / in <impl BitRead for StreamBitsReader<'_, R>>::read_gamma`
- function: `<impl BitRead for StreamBitsReader<'_, R>>::read_gamma`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/borsh_impls.rs
+++ replace * with / in <impl BitRead for StreamBitsReader<'_, R>>::read_gamma
@@ -121,17 +121,17 @@
         // the in-memory decoder: the window's proven bits end at the
         // buffer's end, so it can never consume — or even inspect — a byte
         // the reader has not yielded, and speculative reads (which would
         // steal bytes from the next borsh field) are impossible by
         // construction. It fires when earlier refills left enough unconsumed
         // bits buffered; everything else, every reject included, is decided
         // by the per-bit loop below, refilling byte by byte on demand.
         if let Some((n, next)) =
-            BitsReader::gamma_from_window(&self.bytes, self.bytes.len() as u64 * 8, self.position)
+            BitsReader::gamma_from_window(&self.bytes, self.bytes.len() as u64 / /* ~ changed by cargo-mutants ~ */ 8, self.position)
         {
             scan::record_bits_u64(next - self.position);
             self.position = next;
             return Ok(BigUint::from(n));
         }
         self.read_gamma_slow()
     }
 }
```

## rest-4: `crates/before/src/borsh_impls.rs:131:40`

- name: `crates/before/src/borsh_impls.rs:131:40: replace - with + in <impl BitRead for StreamBitsReader<'_, R>>::read_gamma`
- function: `<impl BitRead for StreamBitsReader<'_, R>>::read_gamma`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/borsh_impls.rs
+++ replace - with + in <impl BitRead for StreamBitsReader<'_, R>>::read_gamma
@@ -123,17 +123,17 @@
         // the reader has not yielded, and speculative reads (which would
         // steal bytes from the next borsh field) are impossible by
         // construction. It fires when earlier refills left enough unconsumed
         // bits buffered; everything else, every reject included, is decided
         // by the per-bit loop below, refilling byte by byte on demand.
         if let Some((n, next)) =
             BitsReader::gamma_from_window(&self.bytes, self.bytes.len() as u64 * 8, self.position)
         {
-            scan::record_bits_u64(next - self.position);
+            scan::record_bits_u64(next + /* ~ changed by cargo-mutants ~ */ self.position);
             self.position = next;
             return Ok(BigUint::from(n));
         }
         self.read_gamma_slow()
     }
 }
 
 impl Decode {
```

## rest-5: `crates/before/src/borsh_impls.rs:131:40`

- name: `crates/before/src/borsh_impls.rs:131:40: replace - with / in <impl BitRead for StreamBitsReader<'_, R>>::read_gamma`
- function: `<impl BitRead for StreamBitsReader<'_, R>>::read_gamma`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/borsh_impls.rs
+++ replace - with / in <impl BitRead for StreamBitsReader<'_, R>>::read_gamma
@@ -123,17 +123,17 @@
         // the reader has not yielded, and speculative reads (which would
         // steal bytes from the next borsh field) are impossible by
         // construction. It fires when earlier refills left enough unconsumed
         // bits buffered; everything else, every reject included, is decided
         // by the per-bit loop below, refilling byte by byte on demand.
         if let Some((n, next)) =
             BitsReader::gamma_from_window(&self.bytes, self.bytes.len() as u64 * 8, self.position)
         {
-            scan::record_bits_u64(next - self.position);
+            scan::record_bits_u64(next / /* ~ changed by cargo-mutants ~ */ self.position);
             self.position = next;
             return Ok(BigUint::from(n));
         }
         self.read_gamma_slow()
     }
 }
 
 impl Decode {
```

## rest-6: `crates/before/src/fold.rs:27:52`

- name: `crates/before/src/fold.rs:27:52: replace == with != in balanced_try_fold`
- function: `balanced_try_fold`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/fold.rs
+++ replace == with != in balanced_try_fold
@@ -19,17 +19,17 @@
     iter: impl IntoIterator<Item = T>,
     mut combine: impl FnMut(T, T) -> Result<T, (T, T)>,
 ) -> Result<Vec<T>, Vec<T>> {
     let mut iter = iter.into_iter();
     let mut stack: Vec<(T, usize)> = Vec::new();
     while let Some(item) = iter.next() {
         let mut merged = item;
         let mut weight = 0usize;
-        while stack.last().is_some_and(|(_, w)| *w == weight) {
+        while stack.last().is_some_and(|(_, w)| *w != /* ~ changed by cargo-mutants ~ */ weight) {
             let (top, _) = stack.pop().expect("the loop condition saw a top entry");
             match combine(top, merged) {
                 Ok(group) => {
                     merged = group;
                     weight += 1;
                 }
                 Err((top, back)) => {
                     stack.push((top, weight));
```

## rest-7: `crates/before/src/fold.rs:32:28`

- name: `crates/before/src/fold.rs:32:28: replace += with *= in balanced_try_fold`
- function: `balanced_try_fold`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/fold.rs
+++ replace += with *= in balanced_try_fold
@@ -24,17 +24,17 @@
     while let Some(item) = iter.next() {
         let mut merged = item;
         let mut weight = 0usize;
         while stack.last().is_some_and(|(_, w)| *w == weight) {
             let (top, _) = stack.pop().expect("the loop condition saw a top entry");
             match combine(top, merged) {
                 Ok(group) => {
                     merged = group;
-                    weight += 1;
+                    weight *= /* ~ changed by cargo-mutants ~ */ 1;
                 }
                 Err((top, back)) => {
                     stack.push((top, weight));
                     let mut uncombined: Vec<T> =
                         stack.into_iter().map(|(group, _)| group).collect();
                     uncombined.push(back);
                     uncombined.extend(iter);
                     return Err(uncombined);
```

## rest-8: `crates/before/src/recurse.rs:45:29`

- name: `crates/before/src/recurse.rs:45:29: replace * with +`
- function: `?`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/recurse.rs
+++ replace * with +
@@ -37,17 +37,17 @@
 /// Sized from a frame-size measurement (aarch64 release): the heaviest
 /// traversal frame is roughly 0.5 KiB/level — established by per-level
 /// stack-pointer deltas and cross-checked against each recursive function's
 /// prologue `sub sp`. With [`STRIDE`] = 64 the inter-probe burst is therefore
 /// well under 32 KiB, so 256 KiB leaves roughly an 8x cushion — ample headroom
 /// for wider frames on other targets (e.g. x86_64) and for arbitrary-precision
 /// `BigUint` arithmetic temporaries in the deepest frame.
 #[cfg(test)]
-const RED_ZONE: usize = 256 * 1024;
+const RED_ZONE: usize = 256 + /* ~ changed by cargo-mutants ~ */ 1024;
 
 /// Size of each heap-allocated stack segment allocated when growth triggers.
 #[cfg(test)]
 const STACK_GROWTH: usize = 1024 * 1024;
 
 /// Whether to probe stack headroom on entering `depth` (every [`STRIDE`] levels).
 #[cfg(test)]
 #[inline]
```

## rest-9: `crates/before/src/recurse.rs:45:29`

- name: `crates/before/src/recurse.rs:45:29: replace * with /`
- function: `?`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/recurse.rs
+++ replace * with /
@@ -37,17 +37,17 @@
 /// Sized from a frame-size measurement (aarch64 release): the heaviest
 /// traversal frame is roughly 0.5 KiB/level — established by per-level
 /// stack-pointer deltas and cross-checked against each recursive function's
 /// prologue `sub sp`. With [`STRIDE`] = 64 the inter-probe burst is therefore
 /// well under 32 KiB, so 256 KiB leaves roughly an 8x cushion — ample headroom
 /// for wider frames on other targets (e.g. x86_64) and for arbitrary-precision
 /// `BigUint` arithmetic temporaries in the deepest frame.
 #[cfg(test)]
-const RED_ZONE: usize = 256 * 1024;
+const RED_ZONE: usize = 256 / /* ~ changed by cargo-mutants ~ */ 1024;
 
 /// Size of each heap-allocated stack segment allocated when growth triggers.
 #[cfg(test)]
 const STACK_GROWTH: usize = 1024 * 1024;
 
 /// Whether to probe stack headroom on entering `depth` (every [`STRIDE`] levels).
 #[cfg(test)]
 #[inline]
```

## rest-10: `crates/before/src/recurse.rs:49:34`

- name: `crates/before/src/recurse.rs:49:34: replace * with +`
- function: `?`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/recurse.rs
+++ replace * with +
@@ -41,17 +41,17 @@
 /// well under 32 KiB, so 256 KiB leaves roughly an 8x cushion — ample headroom
 /// for wider frames on other targets (e.g. x86_64) and for arbitrary-precision
 /// `BigUint` arithmetic temporaries in the deepest frame.
 #[cfg(test)]
 const RED_ZONE: usize = 256 * 1024;
 
 /// Size of each heap-allocated stack segment allocated when growth triggers.
 #[cfg(test)]
-const STACK_GROWTH: usize = 1024 * 1024;
+const STACK_GROWTH: usize = 1024 + /* ~ changed by cargo-mutants ~ */ 1024;
 
 /// Whether to probe stack headroom on entering `depth` (every [`STRIDE`] levels).
 #[cfg(test)]
 #[inline]
 pub(crate) fn should_grow(depth: usize) -> bool {
     depth.is_multiple_of(STRIDE)
 }
```

## rest-11: `crates/before/src/recurse.rs:49:34`

- name: `crates/before/src/recurse.rs:49:34: replace * with /`
- function: `?`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/recurse.rs
+++ replace * with /
@@ -41,17 +41,17 @@
 /// well under 32 KiB, so 256 KiB leaves roughly an 8x cushion — ample headroom
 /// for wider frames on other targets (e.g. x86_64) and for arbitrary-precision
 /// `BigUint` arithmetic temporaries in the deepest frame.
 #[cfg(test)]
 const RED_ZONE: usize = 256 * 1024;
 
 /// Size of each heap-allocated stack segment allocated when growth triggers.
 #[cfg(test)]
-const STACK_GROWTH: usize = 1024 * 1024;
+const STACK_GROWTH: usize = 1024 / /* ~ changed by cargo-mutants ~ */ 1024;
 
 /// Whether to probe stack headroom on entering `depth` (every [`STRIDE`] levels).
 #[cfg(test)]
 #[inline]
 pub(crate) fn should_grow(depth: usize) -> bool {
     depth.is_multiple_of(STRIDE)
 }
```

## rest-12: `crates/before/src/serde_impls.rs:287:9`

- name: `crates/before/src/serde_impls.rs:287:9: replace <impl Visitor<'de> for CountVisitor>::expecting -> fmt::Result with Ok(Default::default())`
- function: `<impl Visitor<'de> for CountVisitor>::expecting`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/serde_impls.rs
+++ replace <impl Visitor<'de> for CountVisitor>::expecting -> fmt::Result with Ok(Default::default())
@@ -279,17 +279,17 @@
 
 /// Reads Count's binary limb sequence directly into its integer storage.
 struct CountVisitor;
 
 impl<'de> Visitor<'de> for CountVisitor {
     type Value = Count;
 
     fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
-        f.write_str("canonical least-significant-first u64 limbs")
+        Ok(Default::default()) /* ~ changed by cargo-mutants ~ */
     }
 
     fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Count, A::Error> {
         let mut limbs = CanonicalLimbs::new();
         while let Some(limb) = sequence.next_element()? {
             limbs.push(limb);
         }
         limbs
```

## rest-13: `crates/before/src/shape.rs:176:9`

- name: `crates/before/src/shape.rs:176:9: replace <impl Iterator for Plateaus<'_>>::size_hint -> (usize, Option<usize>) with (0, None)`
- function: `<impl Iterator for Plateaus<'_>>::size_hint`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/shape.rs
+++ replace <impl Iterator for Plateaus<'_>>::size_hint -> (usize, Option<usize>) with (0, None)
@@ -168,21 +168,17 @@
             self.finished = true;
         } else {
             self.walk.advance();
         }
         Some(plateau)
     }
 
     fn size_hint(&self) -> (usize, Option<usize>) {
-        if self.finished {
-            (0, Some(0))
-        } else {
-            (1, None)
-        }
+        (0, None) /* ~ changed by cargo-mutants ~ */
     }
 }
 
 impl FusedIterator for Plateaus<'_> {}
 
 /// An iterator over the regions of a party's shape, yielded left to
 /// right; see [`Party::shape`].
 ///
```

## rest-14: `crates/before/src/shape.rs:176:9`

- name: `crates/before/src/shape.rs:176:9: replace <impl Iterator for Plateaus<'_>>::size_hint -> (usize, Option<usize>) with (0, Some(0))`
- function: `<impl Iterator for Plateaus<'_>>::size_hint`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/shape.rs
+++ replace <impl Iterator for Plateaus<'_>>::size_hint -> (usize, Option<usize>) with (0, Some(0))
@@ -168,21 +168,17 @@
             self.finished = true;
         } else {
             self.walk.advance();
         }
         Some(plateau)
     }
 
     fn size_hint(&self) -> (usize, Option<usize>) {
-        if self.finished {
-            (0, Some(0))
-        } else {
-            (1, None)
-        }
+        (0, Some(0)) /* ~ changed by cargo-mutants ~ */
     }
 }
 
 impl FusedIterator for Plateaus<'_> {}
 
 /// An iterator over the regions of a party's shape, yielded left to
 /// right; see [`Party::shape`].
 ///
```

## rest-15: `crates/before/src/shape.rs:176:9`

- name: `crates/before/src/shape.rs:176:9: replace <impl Iterator for Plateaus<'_>>::size_hint -> (usize, Option<usize>) with (0, Some(1))`
- function: `<impl Iterator for Plateaus<'_>>::size_hint`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/shape.rs
+++ replace <impl Iterator for Plateaus<'_>>::size_hint -> (usize, Option<usize>) with (0, Some(1))
@@ -168,21 +168,17 @@
             self.finished = true;
         } else {
             self.walk.advance();
         }
         Some(plateau)
     }
 
     fn size_hint(&self) -> (usize, Option<usize>) {
-        if self.finished {
-            (0, Some(0))
-        } else {
-            (1, None)
-        }
+        (0, Some(1)) /* ~ changed by cargo-mutants ~ */
     }
 }
 
 impl FusedIterator for Plateaus<'_> {}
 
 /// An iterator over the regions of a party's shape, yielded left to
 /// right; see [`Party::shape`].
 ///
```

## rest-16: `crates/before/src/shape.rs:176:9`

- name: `crates/before/src/shape.rs:176:9: replace <impl Iterator for Plateaus<'_>>::size_hint -> (usize, Option<usize>) with (1, None)`
- function: `<impl Iterator for Plateaus<'_>>::size_hint`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/shape.rs
+++ replace <impl Iterator for Plateaus<'_>>::size_hint -> (usize, Option<usize>) with (1, None)
@@ -168,21 +168,17 @@
             self.finished = true;
         } else {
             self.walk.advance();
         }
         Some(plateau)
     }
 
     fn size_hint(&self) -> (usize, Option<usize>) {
-        if self.finished {
-            (0, Some(0))
-        } else {
-            (1, None)
-        }
+        (1, None) /* ~ changed by cargo-mutants ~ */
     }
 }
 
 impl FusedIterator for Plateaus<'_> {}
 
 /// An iterator over the regions of a party's shape, yielded left to
 /// right; see [`Party::shape`].
 ///
```

## rest-17: `crates/before/src/shape.rs:176:9`

- name: `crates/before/src/shape.rs:176:9: replace <impl Iterator for Plateaus<'_>>::size_hint -> (usize, Option<usize>) with (1, Some(0))`
- function: `<impl Iterator for Plateaus<'_>>::size_hint`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/shape.rs
+++ replace <impl Iterator for Plateaus<'_>>::size_hint -> (usize, Option<usize>) with (1, Some(0))
@@ -168,21 +168,17 @@
             self.finished = true;
         } else {
             self.walk.advance();
         }
         Some(plateau)
     }
 
     fn size_hint(&self) -> (usize, Option<usize>) {
-        if self.finished {
-            (0, Some(0))
-        } else {
-            (1, None)
-        }
+        (1, Some(0)) /* ~ changed by cargo-mutants ~ */
     }
 }
 
 impl FusedIterator for Plateaus<'_> {}
 
 /// An iterator over the regions of a party's shape, yielded left to
 /// right; see [`Party::shape`].
 ///
```

## rest-18: `crates/before/src/shape.rs:176:9`

- name: `crates/before/src/shape.rs:176:9: replace <impl Iterator for Plateaus<'_>>::size_hint -> (usize, Option<usize>) with (1, Some(1))`
- function: `<impl Iterator for Plateaus<'_>>::size_hint`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/shape.rs
+++ replace <impl Iterator for Plateaus<'_>>::size_hint -> (usize, Option<usize>) with (1, Some(1))
@@ -168,21 +168,17 @@
             self.finished = true;
         } else {
             self.walk.advance();
         }
         Some(plateau)
     }
 
     fn size_hint(&self) -> (usize, Option<usize>) {
-        if self.finished {
-            (0, Some(0))
-        } else {
-            (1, None)
-        }
+        (1, Some(1)) /* ~ changed by cargo-mutants ~ */
     }
 }
 
 impl FusedIterator for Plateaus<'_> {}
 
 /// An iterator over the regions of a party's shape, yielded left to
 /// right; see [`Party::shape`].
 ///
```

## rest-19: `crates/before/src/shape.rs:228:9`

- name: `crates/before/src/shape.rs:228:9: replace <impl Iterator for Regions<'_>>::size_hint -> (usize, Option<usize>) with (0, None)`
- function: `<impl Iterator for Regions<'_>>::size_hint`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/shape.rs
+++ replace <impl Iterator for Regions<'_>>::size_hint -> (usize, Option<usize>) with (0, None)
@@ -220,21 +220,17 @@
             self.finished = true;
         } else {
             self.walk.advance();
         }
         Some(region)
     }
 
     fn size_hint(&self) -> (usize, Option<usize>) {
-        if self.finished {
-            (0, Some(0))
-        } else {
-            (1, None)
-        }
+        (0, None) /* ~ changed by cargo-mutants ~ */
     }
 }
 
 impl FusedIterator for Regions<'_> {}
 
 /// An iterator over a clock's version plateaus overlaid with its party's
 /// ownership; see [`Clock::shape`].
 ///
```

## rest-20: `crates/before/src/shape.rs:228:9`

- name: `crates/before/src/shape.rs:228:9: replace <impl Iterator for Regions<'_>>::size_hint -> (usize, Option<usize>) with (0, Some(0))`
- function: `<impl Iterator for Regions<'_>>::size_hint`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/shape.rs
+++ replace <impl Iterator for Regions<'_>>::size_hint -> (usize, Option<usize>) with (0, Some(0))
@@ -220,21 +220,17 @@
             self.finished = true;
         } else {
             self.walk.advance();
         }
         Some(region)
     }
 
     fn size_hint(&self) -> (usize, Option<usize>) {
-        if self.finished {
-            (0, Some(0))
-        } else {
-            (1, None)
-        }
+        (0, Some(0)) /* ~ changed by cargo-mutants ~ */
     }
 }
 
 impl FusedIterator for Regions<'_> {}
 
 /// An iterator over a clock's version plateaus overlaid with its party's
 /// ownership; see [`Clock::shape`].
 ///
```

## rest-21: `crates/before/src/shape.rs:228:9`

- name: `crates/before/src/shape.rs:228:9: replace <impl Iterator for Regions<'_>>::size_hint -> (usize, Option<usize>) with (0, Some(1))`
- function: `<impl Iterator for Regions<'_>>::size_hint`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/shape.rs
+++ replace <impl Iterator for Regions<'_>>::size_hint -> (usize, Option<usize>) with (0, Some(1))
@@ -220,21 +220,17 @@
             self.finished = true;
         } else {
             self.walk.advance();
         }
         Some(region)
     }
 
     fn size_hint(&self) -> (usize, Option<usize>) {
-        if self.finished {
-            (0, Some(0))
-        } else {
-            (1, None)
-        }
+        (0, Some(1)) /* ~ changed by cargo-mutants ~ */
     }
 }
 
 impl FusedIterator for Regions<'_> {}
 
 /// An iterator over a clock's version plateaus overlaid with its party's
 /// ownership; see [`Clock::shape`].
 ///
```

## rest-22: `crates/before/src/shape.rs:228:9`

- name: `crates/before/src/shape.rs:228:9: replace <impl Iterator for Regions<'_>>::size_hint -> (usize, Option<usize>) with (1, None)`
- function: `<impl Iterator for Regions<'_>>::size_hint`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/shape.rs
+++ replace <impl Iterator for Regions<'_>>::size_hint -> (usize, Option<usize>) with (1, None)
@@ -220,21 +220,17 @@
             self.finished = true;
         } else {
             self.walk.advance();
         }
         Some(region)
     }
 
     fn size_hint(&self) -> (usize, Option<usize>) {
-        if self.finished {
-            (0, Some(0))
-        } else {
-            (1, None)
-        }
+        (1, None) /* ~ changed by cargo-mutants ~ */
     }
 }
 
 impl FusedIterator for Regions<'_> {}
 
 /// An iterator over a clock's version plateaus overlaid with its party's
 /// ownership; see [`Clock::shape`].
 ///
```

## rest-23: `crates/before/src/shape.rs:228:9`

- name: `crates/before/src/shape.rs:228:9: replace <impl Iterator for Regions<'_>>::size_hint -> (usize, Option<usize>) with (1, Some(0))`
- function: `<impl Iterator for Regions<'_>>::size_hint`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/shape.rs
+++ replace <impl Iterator for Regions<'_>>::size_hint -> (usize, Option<usize>) with (1, Some(0))
@@ -220,21 +220,17 @@
             self.finished = true;
         } else {
             self.walk.advance();
         }
         Some(region)
     }
 
     fn size_hint(&self) -> (usize, Option<usize>) {
-        if self.finished {
-            (0, Some(0))
-        } else {
-            (1, None)
-        }
+        (1, Some(0)) /* ~ changed by cargo-mutants ~ */
     }
 }
 
 impl FusedIterator for Regions<'_> {}
 
 /// An iterator over a clock's version plateaus overlaid with its party's
 /// ownership; see [`Clock::shape`].
 ///
```

## rest-24: `crates/before/src/shape.rs:228:9`

- name: `crates/before/src/shape.rs:228:9: replace <impl Iterator for Regions<'_>>::size_hint -> (usize, Option<usize>) with (1, Some(1))`
- function: `<impl Iterator for Regions<'_>>::size_hint`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/shape.rs
+++ replace <impl Iterator for Regions<'_>>::size_hint -> (usize, Option<usize>) with (1, Some(1))
@@ -220,21 +220,17 @@
             self.finished = true;
         } else {
             self.walk.advance();
         }
         Some(region)
     }
 
     fn size_hint(&self) -> (usize, Option<usize>) {
-        if self.finished {
-            (0, Some(0))
-        } else {
-            (1, None)
-        }
+        (1, Some(1)) /* ~ changed by cargo-mutants ~ */
     }
 }
 
 impl FusedIterator for Regions<'_> {}
 
 /// An iterator over a clock's version plateaus overlaid with its party's
 /// ownership; see [`Clock::shape`].
 ///
```

## rest-25: `crates/before/src/shape.rs:286:9`

- name: `crates/before/src/shape.rs:286:9: replace <impl Iterator for Overlay<'_>>::size_hint -> (usize, Option<usize>) with (0, None)`
- function: `<impl Iterator for Overlay<'_>>::size_hint`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/shape.rs
+++ replace <impl Iterator for Overlay<'_>>::size_hint -> (usize, Option<usize>) with (0, None)
@@ -278,21 +278,17 @@
         self.finished = advance_refinement(&mut [
             &mut self.version as &mut dyn Refine,
             &mut self.party as &mut dyn Refine,
         ]);
         Some((plateau, owned))
     }
 
     fn size_hint(&self) -> (usize, Option<usize>) {
-        if self.finished {
-            (0, Some(0))
-        } else {
-            (1, None)
-        }
+        (0, None) /* ~ changed by cargo-mutants ~ */
     }
 }
 
 impl FusedIterator for Overlay<'_> {}
 
 /// Walk `N` versions as one iterator over the coarsest common
 /// refinement of their shapes' plateau intervals.
 ///
```

## rest-26: `crates/before/src/shape.rs:286:9`

- name: `crates/before/src/shape.rs:286:9: replace <impl Iterator for Overlay<'_>>::size_hint -> (usize, Option<usize>) with (0, Some(0))`
- function: `<impl Iterator for Overlay<'_>>::size_hint`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/shape.rs
+++ replace <impl Iterator for Overlay<'_>>::size_hint -> (usize, Option<usize>) with (0, Some(0))
@@ -278,21 +278,17 @@
         self.finished = advance_refinement(&mut [
             &mut self.version as &mut dyn Refine,
             &mut self.party as &mut dyn Refine,
         ]);
         Some((plateau, owned))
     }
 
     fn size_hint(&self) -> (usize, Option<usize>) {
-        if self.finished {
-            (0, Some(0))
-        } else {
-            (1, None)
-        }
+        (0, Some(0)) /* ~ changed by cargo-mutants ~ */
     }
 }
 
 impl FusedIterator for Overlay<'_> {}
 
 /// Walk `N` versions as one iterator over the coarsest common
 /// refinement of their shapes' plateau intervals.
 ///
```

## rest-27: `crates/before/src/shape.rs:286:9`

- name: `crates/before/src/shape.rs:286:9: replace <impl Iterator for Overlay<'_>>::size_hint -> (usize, Option<usize>) with (0, Some(1))`
- function: `<impl Iterator for Overlay<'_>>::size_hint`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/shape.rs
+++ replace <impl Iterator for Overlay<'_>>::size_hint -> (usize, Option<usize>) with (0, Some(1))
@@ -278,21 +278,17 @@
         self.finished = advance_refinement(&mut [
             &mut self.version as &mut dyn Refine,
             &mut self.party as &mut dyn Refine,
         ]);
         Some((plateau, owned))
     }
 
     fn size_hint(&self) -> (usize, Option<usize>) {
-        if self.finished {
-            (0, Some(0))
-        } else {
-            (1, None)
-        }
+        (0, Some(1)) /* ~ changed by cargo-mutants ~ */
     }
 }
 
 impl FusedIterator for Overlay<'_> {}
 
 /// Walk `N` versions as one iterator over the coarsest common
 /// refinement of their shapes' plateau intervals.
 ///
```

## rest-28: `crates/before/src/shape.rs:286:9`

- name: `crates/before/src/shape.rs:286:9: replace <impl Iterator for Overlay<'_>>::size_hint -> (usize, Option<usize>) with (1, None)`
- function: `<impl Iterator for Overlay<'_>>::size_hint`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/shape.rs
+++ replace <impl Iterator for Overlay<'_>>::size_hint -> (usize, Option<usize>) with (1, None)
@@ -278,21 +278,17 @@
         self.finished = advance_refinement(&mut [
             &mut self.version as &mut dyn Refine,
             &mut self.party as &mut dyn Refine,
         ]);
         Some((plateau, owned))
     }
 
     fn size_hint(&self) -> (usize, Option<usize>) {
-        if self.finished {
-            (0, Some(0))
-        } else {
-            (1, None)
-        }
+        (1, None) /* ~ changed by cargo-mutants ~ */
     }
 }
 
 impl FusedIterator for Overlay<'_> {}
 
 /// Walk `N` versions as one iterator over the coarsest common
 /// refinement of their shapes' plateau intervals.
 ///
```

## rest-29: `crates/before/src/shape.rs:286:9`

- name: `crates/before/src/shape.rs:286:9: replace <impl Iterator for Overlay<'_>>::size_hint -> (usize, Option<usize>) with (1, Some(0))`
- function: `<impl Iterator for Overlay<'_>>::size_hint`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/shape.rs
+++ replace <impl Iterator for Overlay<'_>>::size_hint -> (usize, Option<usize>) with (1, Some(0))
@@ -278,21 +278,17 @@
         self.finished = advance_refinement(&mut [
             &mut self.version as &mut dyn Refine,
             &mut self.party as &mut dyn Refine,
         ]);
         Some((plateau, owned))
     }
 
     fn size_hint(&self) -> (usize, Option<usize>) {
-        if self.finished {
-            (0, Some(0))
-        } else {
-            (1, None)
-        }
+        (1, Some(0)) /* ~ changed by cargo-mutants ~ */
     }
 }
 
 impl FusedIterator for Overlay<'_> {}
 
 /// Walk `N` versions as one iterator over the coarsest common
 /// refinement of their shapes' plateau intervals.
 ///
```

## rest-30: `crates/before/src/shape.rs:286:9`

- name: `crates/before/src/shape.rs:286:9: replace <impl Iterator for Overlay<'_>>::size_hint -> (usize, Option<usize>) with (1, Some(1))`
- function: `<impl Iterator for Overlay<'_>>::size_hint`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/shape.rs
+++ replace <impl Iterator for Overlay<'_>>::size_hint -> (usize, Option<usize>) with (1, Some(1))
@@ -278,21 +278,17 @@
         self.finished = advance_refinement(&mut [
             &mut self.version as &mut dyn Refine,
             &mut self.party as &mut dyn Refine,
         ]);
         Some((plateau, owned))
     }
 
     fn size_hint(&self) -> (usize, Option<usize>) {
-        if self.finished {
-            (0, Some(0))
-        } else {
-            (1, None)
-        }
+        (1, Some(1)) /* ~ changed by cargo-mutants ~ */
     }
 }
 
 impl FusedIterator for Overlay<'_> {}
 
 /// Walk `N` versions as one iterator over the coarsest common
 /// refinement of their shapes' plateau intervals.
 ///
```

## rest-31: `crates/before/src/shape.rs:371:9`

- name: `crates/before/src/shape.rs:371:9: replace <impl Iterator for Cells<'_, N>>::size_hint -> (usize, Option<usize>) with (0, None)`
- function: `<impl Iterator for Cells<'_, N>>::size_hint`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/shape.rs
+++ replace <impl Iterator for Cells<'_, N>>::size_hint -> (usize, Option<usize>) with (0, None)
@@ -363,17 +363,13 @@
         }
         let depth = self.walks.iter().map(Refine::depth).max().unwrap_or(0);
         let rises = self.walks.each_mut().map(VersionWalk::take_rise);
         self.finished = advance_refinement(&mut self.walks);
         Some(Cell { depth, rises })
     }
 
     fn size_hint(&self) -> (usize, Option<usize>) {
-        if self.finished {
-            (0, Some(0))
-        } else {
-            (1, None)
-        }
+        (0, None) /* ~ changed by cargo-mutants ~ */
     }
 }
 
 impl<const N: usize> FusedIterator for Cells<'_, N> {}
```

## rest-32: `crates/before/src/shape.rs:371:9`

- name: `crates/before/src/shape.rs:371:9: replace <impl Iterator for Cells<'_, N>>::size_hint -> (usize, Option<usize>) with (0, Some(0))`
- function: `<impl Iterator for Cells<'_, N>>::size_hint`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/shape.rs
+++ replace <impl Iterator for Cells<'_, N>>::size_hint -> (usize, Option<usize>) with (0, Some(0))
@@ -363,17 +363,13 @@
         }
         let depth = self.walks.iter().map(Refine::depth).max().unwrap_or(0);
         let rises = self.walks.each_mut().map(VersionWalk::take_rise);
         self.finished = advance_refinement(&mut self.walks);
         Some(Cell { depth, rises })
     }
 
     fn size_hint(&self) -> (usize, Option<usize>) {
-        if self.finished {
-            (0, Some(0))
-        } else {
-            (1, None)
-        }
+        (0, Some(0)) /* ~ changed by cargo-mutants ~ */
     }
 }
 
 impl<const N: usize> FusedIterator for Cells<'_, N> {}
```

## rest-33: `crates/before/src/shape.rs:371:9`

- name: `crates/before/src/shape.rs:371:9: replace <impl Iterator for Cells<'_, N>>::size_hint -> (usize, Option<usize>) with (0, Some(1))`
- function: `<impl Iterator for Cells<'_, N>>::size_hint`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/shape.rs
+++ replace <impl Iterator for Cells<'_, N>>::size_hint -> (usize, Option<usize>) with (0, Some(1))
@@ -363,17 +363,13 @@
         }
         let depth = self.walks.iter().map(Refine::depth).max().unwrap_or(0);
         let rises = self.walks.each_mut().map(VersionWalk::take_rise);
         self.finished = advance_refinement(&mut self.walks);
         Some(Cell { depth, rises })
     }
 
     fn size_hint(&self) -> (usize, Option<usize>) {
-        if self.finished {
-            (0, Some(0))
-        } else {
-            (1, None)
-        }
+        (0, Some(1)) /* ~ changed by cargo-mutants ~ */
     }
 }
 
 impl<const N: usize> FusedIterator for Cells<'_, N> {}
```

## rest-34: `crates/before/src/shape.rs:371:9`

- name: `crates/before/src/shape.rs:371:9: replace <impl Iterator for Cells<'_, N>>::size_hint -> (usize, Option<usize>) with (1, None)`
- function: `<impl Iterator for Cells<'_, N>>::size_hint`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/shape.rs
+++ replace <impl Iterator for Cells<'_, N>>::size_hint -> (usize, Option<usize>) with (1, None)
@@ -363,17 +363,13 @@
         }
         let depth = self.walks.iter().map(Refine::depth).max().unwrap_or(0);
         let rises = self.walks.each_mut().map(VersionWalk::take_rise);
         self.finished = advance_refinement(&mut self.walks);
         Some(Cell { depth, rises })
     }
 
     fn size_hint(&self) -> (usize, Option<usize>) {
-        if self.finished {
-            (0, Some(0))
-        } else {
-            (1, None)
-        }
+        (1, None) /* ~ changed by cargo-mutants ~ */
     }
 }
 
 impl<const N: usize> FusedIterator for Cells<'_, N> {}
```

## rest-35: `crates/before/src/shape.rs:371:9`

- name: `crates/before/src/shape.rs:371:9: replace <impl Iterator for Cells<'_, N>>::size_hint -> (usize, Option<usize>) with (1, Some(0))`
- function: `<impl Iterator for Cells<'_, N>>::size_hint`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/shape.rs
+++ replace <impl Iterator for Cells<'_, N>>::size_hint -> (usize, Option<usize>) with (1, Some(0))
@@ -363,17 +363,13 @@
         }
         let depth = self.walks.iter().map(Refine::depth).max().unwrap_or(0);
         let rises = self.walks.each_mut().map(VersionWalk::take_rise);
         self.finished = advance_refinement(&mut self.walks);
         Some(Cell { depth, rises })
     }
 
     fn size_hint(&self) -> (usize, Option<usize>) {
-        if self.finished {
-            (0, Some(0))
-        } else {
-            (1, None)
-        }
+        (1, Some(0)) /* ~ changed by cargo-mutants ~ */
     }
 }
 
 impl<const N: usize> FusedIterator for Cells<'_, N> {}
```

## rest-36: `crates/before/src/shape.rs:371:9`

- name: `crates/before/src/shape.rs:371:9: replace <impl Iterator for Cells<'_, N>>::size_hint -> (usize, Option<usize>) with (1, Some(1))`
- function: `<impl Iterator for Cells<'_, N>>::size_hint`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/shape.rs
+++ replace <impl Iterator for Cells<'_, N>>::size_hint -> (usize, Option<usize>) with (1, Some(1))
@@ -363,17 +363,13 @@
         }
         let depth = self.walks.iter().map(Refine::depth).max().unwrap_or(0);
         let rises = self.walks.each_mut().map(VersionWalk::take_rise);
         self.finished = advance_refinement(&mut self.walks);
         Some(Cell { depth, rises })
     }
 
     fn size_hint(&self) -> (usize, Option<usize>) {
-        if self.finished {
-            (0, Some(0))
-        } else {
-            (1, None)
-        }
+        (1, Some(1)) /* ~ changed by cargo-mutants ~ */
     }
 }
 
 impl<const N: usize> FusedIterator for Cells<'_, N> {}
```

