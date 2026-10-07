# Mutation survivors: bits

10 survivors. Each entry: the cargo-mutants name, the enclosing function,
and the exact diff cargo-mutants applied (apply with `patch -p0` from the worktree root
after stripping the `*** ` header lines, or re-create it by hand).

## bits-1: `crates/before/src/bits/reader.rs:182:23`

- name: `crates/before/src/bits/reader.rs:182:23: replace += with *= in BitsReader<'a>::read_word`
- function: `BitsReader<'a>::read_word`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/bits/reader.rs
+++ replace += with *= in BitsReader<'a>::read_word
@@ -174,17 +174,17 @@
         if self.position + u64::from(len) > self.len {
             return Err(self.truncated());
         }
         let value = self
             .reader
             .read_bits(len as usize)
             .expect("the requested word fits the live range");
         scan::record_bits(len as usize);
-        self.position += u64::from(len);
+        self.position *= /* ~ changed by cargo-mutants ~ */ u64::from(len);
         Ok(value)
     }
 
     /// Read the unary prefix without recording a successful run:
     /// the count of `0` bits before (and consuming) the terminating `1`.
     ///
     /// `Truncated` when the live bits end before a `1`: the phantom
     /// zeros past the live length (the word source masks the tail
```

## bits-2: `crates/before/src/bits/reader.rs:201:38`

- name: `crates/before/src/bits/reader.rs:201:38: replace + with * in BitsReader<'a>::unary_raw`
- function: `BitsReader<'a>::unary_raw`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/bits/reader.rs
+++ replace + with * in BitsReader<'a>::unary_raw
@@ -193,17 +193,17 @@
     /// early.
     fn unary_raw(&mut self) -> Result<u64, Truncated> {
         match self.reader.read_unary() {
             Err(_) => Err(self.truncated()),
             Ok(k) => {
                 // No overflow: position and len are at most 8 · a buffer's
                 // byte count and k is bounded by the word source's total
                 // bits, all far below 2^64.
-                if self.position + k + 1 > self.len {
+                if self.position + k * /* ~ changed by cargo-mutants ~ */ 1 > self.len {
                     return Err(self.truncated());
                 }
                 Ok(k)
             }
         }
     }
 
     /// Reject at the live length, recording the examined tail.
```

## bits-3: `crates/before/src/bits/reader.rs:218:40`

- name: `crates/before/src/bits/reader.rs:218:40: replace - with + in BitsReader<'a>::truncated`
- function: `BitsReader<'a>::truncated`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/bits/reader.rs
+++ replace - with + in BitsReader<'a>::truncated
@@ -210,17 +210,17 @@
     ///
     /// A rejecting read still examined every remaining live bit — a
     /// self-delimiting stream's truncation is only discoverable by parsing to
     /// its end, which is exactly what the truncation-reject scan floors demand
     /// the meter see — so the tail records before the reject surfaces, and the
     /// cursor parks at the live length, where the per-bit loop's failing read
     /// leaves its own reader.
     fn truncated(&mut self) -> Truncated {
-        scan::record_bits_u64(self.len - self.position);
+        scan::record_bits_u64(self.len + /* ~ changed by cargo-mutants ~ */ self.position);
         self.position = self.len;
         Truncated
     }
 
     /// Skip one Elias-gamma-coded integer without materializing
     /// its value; `Truncated` exactly where [`read_gamma`](BitRead::read_gamma)
     /// would be.
     ///
```

## bits-4: `crates/before/src/bits/reader.rs:298:33`

- name: `crates/before/src/bits/reader.rs:298:33: replace + with * in <impl BitRead for BitsReader<'_>>::read_unary`
- function: `<impl BitRead for BitsReader<'_>>::read_unary`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/bits/reader.rs
+++ replace + with * in <impl BitRead for BitsReader<'_>>::read_unary
@@ -290,17 +290,17 @@
     }
 
     fn position(&self) -> u64 {
         self.position
     }
 
     fn read_unary(&mut self) -> Result<u64, Truncated> {
         let k = self.unary_raw()?;
-        scan::record_bits_u64(k + 1);
+        scan::record_bits_u64(k * /* ~ changed by cargo-mutants ~ */ 1);
         self.position += k + 1;
         Ok(k)
     }
 
     /// Read one Elias-gamma-coded integer: accepting and rejecting on exactly
     /// the same inputs as the bitwise reference decoder
     /// over this cursor.
     ///
```

## bits-5: `crates/before/src/bits/storage.rs:82:9`

- name: `crates/before/src/bits/storage.rs:82:9: replace Bits::has_canonical_padding -> bool with true`
- function: `Bits::has_canonical_padding`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/bits/storage.rs
+++ replace Bits::has_canonical_padding -> bool with true
@@ -74,21 +74,17 @@
     ///
     /// This implies equality but is not required for equality.
     pub(crate) fn ptr_eq(&self, other: &Bits) -> bool {
         self.bytes.len() == other.bytes.len() && self.bytes.as_ptr() == other.bytes.as_ptr()
     }
 
     /// Whether the stored bytes end in their unique marker and zero padding.
     pub(crate) fn has_canonical_padding(&self) -> bool {
-        match self.as_raw_slice() {
-            [] => true,
-            [0x80] | [.., 0] => false,
-            _ => true,
-        }
+        true /* ~ changed by cargo-mutants ~ */
     }
 
     /// Validate the marker and zero padding after a decoded value.
     ///
     /// No remaining bit means the marker was truncated. Any remainder other
     /// than one marker followed by at most seven zeros is trailing data.
     ///
     /// # Panics
```

## bits-6: `crates/before/src/bits/storage.rs:83:13`

- name: `crates/before/src/bits/storage.rs:83:13: delete match arm [] in Bits::has_canonical_padding`
- function: `Bits::has_canonical_padding`; genre: MatchArm; outcome: MissedMutant

```diff
--- crates/before/src/bits/storage.rs
+++ delete match arm [] in Bits::has_canonical_padding
@@ -75,17 +75,17 @@
     /// This implies equality but is not required for equality.
     pub(crate) fn ptr_eq(&self, other: &Bits) -> bool {
         self.bytes.len() == other.bytes.len() && self.bytes.as_ptr() == other.bytes.as_ptr()
     }
 
     /// Whether the stored bytes end in their unique marker and zero padding.
     pub(crate) fn has_canonical_padding(&self) -> bool {
         match self.as_raw_slice() {
-            [] => true,
+             /* ~ changed by cargo-mutants ~ */
             [0x80] | [.., 0] => false,
             _ => true,
         }
     }
 
     /// Validate the marker and zero padding after a decoded value.
     ///
     /// No remaining bit means the marker was truncated. Any remainder other
```

## bits-7: `crates/before/src/bits/storage.rs:84:13`

- name: `crates/before/src/bits/storage.rs:84:13: delete match arm [0x80] |[.., 0] in Bits::has_canonical_padding`
- function: `Bits::has_canonical_padding`; genre: MatchArm; outcome: MissedMutant

```diff
--- crates/before/src/bits/storage.rs
+++ delete match arm [0x80] |[.., 0] in Bits::has_canonical_padding
@@ -76,17 +76,17 @@
     pub(crate) fn ptr_eq(&self, other: &Bits) -> bool {
         self.bytes.len() == other.bytes.len() && self.bytes.as_ptr() == other.bytes.as_ptr()
     }
 
     /// Whether the stored bytes end in their unique marker and zero padding.
     pub(crate) fn has_canonical_padding(&self) -> bool {
         match self.as_raw_slice() {
             [] => true,
-            [0x80] | [.., 0] => false,
+             /* ~ changed by cargo-mutants ~ */
             _ => true,
         }
     }
 
     /// Validate the marker and zero padding after a decoded value.
     ///
     /// No remaining bit means the marker was truncated. Any remainder other
     /// than one marker followed by at most seven zeros is trailing data.
```

## bits-8: `crates/before/src/bits/storage.rs:156:9`

- name: `crates/before/src/bits/storage.rs:156:9: replace <impl Hash for Bits>::hash with ()`
- function: `<impl Hash for Bits>::hash`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/bits/storage.rs
+++ replace <impl Hash for Bits>::hash with ()
@@ -148,15 +148,11 @@
     }
 }
 
 impl Eq for Bits {}
 
 /// Hash the same canonical bytes that equality compares.
 impl Hash for Bits {
     fn hash<H: Hasher>(&self, state: &mut H) {
-        debug_assert!(
-            self.has_canonical_padding(),
-            "Bits hashing requires canonical marker padding",
-        );
-        self.as_raw_slice().hash(state);
+        () /* ~ changed by cargo-mutants ~ */
     }
 }
```

## bits-9: `crates/before/src/bits/writer.rs:194:47`

- name: `crates/before/src/bits/writer.rs:194:47: replace | with ^ in BitsWriter::append_bits`
- function: `BitsWriter::append_bits`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/bits/writer.rs
+++ replace | with ^ in BitsWriter::append_bits
@@ -186,17 +186,17 @@
         // register, and write back whole bytes plus the new (zero-padded)
         // partial byte.
         let staged = if within == 0 {
             0
         } else {
             u64::from(self.bytes.pop().expect("a partial byte exists") >> (8 - within))
         };
         let total = within + len;
-        let acc = (u128::from(staged) << len) | u128::from(value);
+        let acc = (u128::from(staged) << len) ^ /* ~ changed by cargo-mutants ~ */ u128::from(value);
         let aligned = (acc << (128 - total)).to_be_bytes();
         let whole = (total / 8) as usize;
         self.bytes.extend_from_slice(&aligned[..whole]);
         if !total.is_multiple_of(8) {
             // The next byte carries the remaining bits at its top and zeros
             // below: the dead-bits invariant by construction.
             self.bytes.push(aligned[whole]);
         }
```

## bits-10: `crates/before/src/bits/writer.rs:244:32`

- name: `crates/before/src/bits/writer.rs:244:32: replace | with ^ in BitsWriter::read_word`
- function: `BitsWriter::read_word`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/bits/writer.rs
+++ replace | with ^ in BitsWriter::read_word
@@ -236,17 +236,17 @@
     pub(crate) fn read_word(&self, start: u64, len: u32) -> u64 {
         assert!(
             len <= 63 && start + u64::from(len) <= self.len(),
             "word read lies within the writer"
         );
         scan::record_bits(len as usize);
         let mut value = 0;
         for position in start..start + u64::from(len) {
-            value = value << 1 | u64::from(self.bit(position));
+            value = value << 1 ^ /* ~ changed by cargo-mutants ~ */ u64::from(self.bit(position));
         }
         value
     }
 
     /// Append `width` zero bits and return the start of the reserved range.
     pub(crate) fn reserve(&mut self, width: usize) -> u64 {
         let start = self.len();
         let mut remaining = width;
```

