# Mutation survivors: bits

32 survivors. Each entry: the cargo-mutants name, the enclosing function,
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

## bits-5: `crates/before/src/bits/reader/gamma.rs:68:40`

- name: `crates/before/src/bits/reader/gamma.rs:68:40: replace | with ^ in decode`
- function: `decode`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/bits/reader/gamma.rs
+++ replace | with ^ in decode
@@ -60,17 +60,17 @@
         return Err(Decode::Truncated);
     }
 
     if zeros < u64::from(u64::BITS) {
         let rest = reader
             .reader
             .read_bits(zeros as usize)
             .expect("the mantissa was proven to fit the live length");
-        let mantissa = (1u64 << zeros) | rest;
+        let mantissa = (1u64 << zeros) ^ /* ~ changed by cargo-mutants ~ */ rest;
         scan::record_bits_u64(code_len);
         reader.position += code_len;
         return Ok(BigUint::from(mantissa - 1));
     }
 
     let mut mantissa = BigUint::ZERO;
     mantissa.set_bit(zeros, true);
     let mut remaining = zeros;
```

## bits-6: `crates/before/src/bits/reader/gamma/window.rs:47:25`

- name: `crates/before/src/bits/reader/gamma/window.rs:47:25: replace | with ^ in load`
- function: `load`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/bits/reader/gamma/window.rs
+++ replace | with ^ in load
@@ -39,11 +39,11 @@
             bytes[tail_at - byte] = tail;
         }
     }
 
     let word = u64::from_be_bytes(bytes[..8].try_into().expect("window has eight bytes"));
     if shift == 0 {
         word
     } else {
-        (word << shift) | (u64::from(bytes[8]) >> (8 - shift))
+        (word << shift) ^ /* ~ changed by cargo-mutants ~ */ (u64::from(bytes[8]) >> (8 - shift))
     }
 }
```

## bits-7: `crates/before/src/bits/reader/gamma/window.rs:47:48`

- name: `crates/before/src/bits/reader/gamma/window.rs:47:48: replace >> with << in load`
- function: `load`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/bits/reader/gamma/window.rs
+++ replace >> with << in load
@@ -39,11 +39,11 @@
             bytes[tail_at - byte] = tail;
         }
     }
 
     let word = u64::from_be_bytes(bytes[..8].try_into().expect("window has eight bytes"));
     if shift == 0 {
         word
     } else {
-        (word << shift) | (u64::from(bytes[8]) >> (8 - shift))
+        (word << shift) | (u64::from(bytes[8]) << /* ~ changed by cargo-mutants ~ */ (8 - shift))
     }
 }
```

## bits-8: `crates/before/src/bits/reader/gamma/window.rs:47:54`

- name: `crates/before/src/bits/reader/gamma/window.rs:47:54: replace - with + in load`
- function: `load`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/bits/reader/gamma/window.rs
+++ replace - with + in load
@@ -39,11 +39,11 @@
             bytes[tail_at - byte] = tail;
         }
     }
 
     let word = u64::from_be_bytes(bytes[..8].try_into().expect("window has eight bytes"));
     if shift == 0 {
         word
     } else {
-        (word << shift) | (u64::from(bytes[8]) >> (8 - shift))
+        (word << shift) | (u64::from(bytes[8]) >> (8 + /* ~ changed by cargo-mutants ~ */ shift))
     }
 }
```

## bits-9: `crates/before/src/bits/reader/gamma/window.rs:47:54`

- name: `crates/before/src/bits/reader/gamma/window.rs:47:54: replace - with / in load`
- function: `load`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/bits/reader/gamma/window.rs
+++ replace - with / in load
@@ -39,11 +39,11 @@
             bytes[tail_at - byte] = tail;
         }
     }
 
     let word = u64::from_be_bytes(bytes[..8].try_into().expect("window has eight bytes"));
     if shift == 0 {
         word
     } else {
-        (word << shift) | (u64::from(bytes[8]) >> (8 - shift))
+        (word << shift) | (u64::from(bytes[8]) >> (8 / /* ~ changed by cargo-mutants ~ */ shift))
     }
 }
```

## bits-10: `crates/before/src/bits/reader/words.rs:58:9`

- name: `crates/before/src/bits/reader/words.rs:58:9: replace <impl Display for OutOfBytes>::fmt -> fmt::Result with Ok(Default::default())`
- function: `<impl Display for OutOfBytes>::fmt`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/bits/reader/words.rs
+++ replace <impl Display for OutOfBytes>::fmt -> fmt::Result with Ok(Default::default())
@@ -50,17 +50,17 @@
 }
 
 /// The source has no byte-bearing word left.
 #[derive(Debug)]
 pub(super) struct OutOfBytes;
 
 impl Display for OutOfBytes {
     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
-        f.write_str("out of stream bytes")
+        Ok(Default::default()) /* ~ changed by cargo-mutants ~ */
     }
 }
 
 impl core::error::Error for OutOfBytes {}
 
 impl WordRead for ByteWords<'_> {
     type Error = OutOfBytes;
     type Word = u32;
```

## bits-11: `crates/before/src/bits/reader/words.rs:78:9`

- name: `crates/before/src/bits/reader/words.rs:78:9: replace <impl WordRead for ByteWords<'_>>::read_word_opt -> Option<u32> with None`
- function: `<impl WordRead for ByteWords<'_>>::read_word_opt`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/bits/reader/words.rs
+++ replace <impl WordRead for ByteWords<'_>>::read_word_opt -> Option<u32> with None
@@ -70,16 +70,11 @@
             return Err(OutOfBytes);
         }
         let word = self.gather();
         self.next += 4;
         Ok(word)
     }
 
     fn read_word_opt(&mut self) -> Option<u32> {
-        if self.next >= self.total {
-            return None;
-        }
-        let word = self.gather();
-        self.next += 4;
-        Some(word)
+        None /* ~ changed by cargo-mutants ~ */
     }
 }
```

## bits-12: `crates/before/src/bits/reader/words.rs:78:22`

- name: `crates/before/src/bits/reader/words.rs:78:22: replace >= with < in <impl WordRead for ByteWords<'_>>::read_word_opt`
- function: `<impl WordRead for ByteWords<'_>>::read_word_opt`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/bits/reader/words.rs
+++ replace >= with < in <impl WordRead for ByteWords<'_>>::read_word_opt
@@ -70,16 +70,16 @@
             return Err(OutOfBytes);
         }
         let word = self.gather();
         self.next += 4;
         Ok(word)
     }
 
     fn read_word_opt(&mut self) -> Option<u32> {
-        if self.next >= self.total {
+        if self.next < /* ~ changed by cargo-mutants ~ */ self.total {
             return None;
         }
         let word = self.gather();
         self.next += 4;
         Some(word)
     }
 }
```

## bits-13: `crates/before/src/bits/stack/bit.rs:46:36`

- name: `crates/before/src/bits/stack/bit.rs:46:36: replace | with ^ in BitStack::push`
- function: `BitStack::push`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/bits/stack/bit.rs
+++ replace | with ^ in BitStack::push
@@ -38,17 +38,17 @@
 
     /// Push one bit.
     pub(crate) fn push(&mut self, bit: bool) {
         if self.top_len == 64 {
             self.words.push(self.top);
             self.top = 0;
             self.top_len = 0;
         }
-        self.top = (self.top << 1) | u64::from(bit);
+        self.top = (self.top << 1) ^ /* ~ changed by cargo-mutants ~ */ u64::from(bit);
         self.top_len += 1;
     }
 
     /// Push `len <= 63` bits at once, oldest at the value's high end — popping
     /// returns them newest-first, exactly as `len` single pushes of the value's
     /// bits from high to low.
     pub(crate) fn push_bits(&mut self, value: u64, len: u32) {
         debug_assert!(len <= 63);
```

## bits-14: `crates/before/src/bits/stack/bit.rs:58:42`

- name: `crates/before/src/bits/stack/bit.rs:58:42: replace | with ^ in BitStack::push_bits`
- function: `BitStack::push_bits`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/bits/stack/bit.rs
+++ replace | with ^ in BitStack::push_bits
@@ -50,17 +50,17 @@
     /// Push `len <= 63` bits at once, oldest at the value's high end — popping
     /// returns them newest-first, exactly as `len` single pushes of the value's
     /// bits from high to low.
     pub(crate) fn push_bits(&mut self, value: u64, len: u32) {
         debug_assert!(len <= 63);
         debug_assert!(value >> len == 0);
         let total = self.top_len + len;
         if total <= 64 {
-            self.top = (self.top << len) | value;
+            self.top = (self.top << len) ^ /* ~ changed by cargo-mutants ~ */ value;
             self.top_len = total;
             return;
         }
         let spill = 64 - self.top_len;
         self.words
             .push((self.top << spill) | (value >> (len - spill)));
         self.top = value & ((1u64 << (len - spill)) - 1);
         self.top_len = len - spill;
```

## bits-15: `crates/before/src/bits/stack/bit.rs:64:39`

- name: `crates/before/src/bits/stack/bit.rs:64:39: replace | with ^ in BitStack::push_bits`
- function: `BitStack::push_bits`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/bits/stack/bit.rs
+++ replace | with ^ in BitStack::push_bits
@@ -56,17 +56,17 @@
         let total = self.top_len + len;
         if total <= 64 {
             self.top = (self.top << len) | value;
             self.top_len = total;
             return;
         }
         let spill = 64 - self.top_len;
         self.words
-            .push((self.top << spill) | (value >> (len - spill)));
+            .push((self.top << spill) ^ /* ~ changed by cargo-mutants ~ */ (value >> (len - spill)));
         self.top = value & ((1u64 << (len - spill)) - 1);
         self.top_len = len - spill;
     }
 
     /// Pop `len <= 63` bits at once, returned exactly as
     /// [`push_bits`](Self::push_bits) stored them: the inverse, equal to `len`
     /// single pops assembled low bit first.
     ///
```

## bits-16: `crates/before/src/bits/stack/bit.rs:93:27`

- name: `crates/before/src/bits/stack/bit.rs:93:27: replace | with ^ in BitStack::pop_bits`
- function: `BitStack::pop_bits`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/bits/stack/bit.rs
+++ replace | with ^ in BitStack::pop_bits
@@ -85,17 +85,17 @@
         let low = self.top;
         let rest = len - low_len;
         // Spilled words are always full, so one refill contains the entire
         // remainder of this at-most-63-bit pop.
         self.top = self.words.pop().expect("bit stack underflow");
         let high = self.top & ((1u64 << rest) - 1);
         self.top >>= rest;
         self.top_len = 64 - rest;
-        (high << low_len) | low
+        (high << low_len) ^ /* ~ changed by cargo-mutants ~ */ low
     }
 
     /// The exact run of set bits at the top of the stack.
     ///
     /// Runs in `O(1 + r / 64)`, where `r` is the returned run length. The
     /// result is `u64`, like [`len`](Self::len), because the run can span the
     /// stack's full height.
     pub(crate) fn trailing_ones(&self) -> u64 {
```

## bits-17: `crates/before/src/bits/stack/bit.rs:152:40`

- name: `crates/before/src/bits/stack/bit.rs:152:40: replace | with ^ in BitStack::set_last`
- function: `BitStack::set_last`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/bits/stack/bit.rs
+++ replace | with ^ in BitStack::set_last
@@ -144,17 +144,17 @@
 
     /// Overwrite the newest bit.
     ///
     /// # Panics
     ///
     /// Panics if the stack is empty.
     pub(crate) fn set_last(&mut self, bit: bool) {
         if self.top_len > 0 {
-            self.top = (self.top & !1) | u64::from(bit);
+            self.top = (self.top & !1) ^ /* ~ changed by cargo-mutants ~ */ u64::from(bit);
         } else {
             let word = self.words.last_mut().expect("set_last on an empty stack");
             *word = (*word & !1) | u64::from(bit);
         }
     }
 
     /// The newest bit, unpopped.
     pub(crate) fn last(&self) -> Option<bool> {
```

## bits-18: `crates/before/src/bits/stack/bit.rs:155:34`

- name: `crates/before/src/bits/stack/bit.rs:155:34: replace | with ^ in BitStack::set_last`
- function: `BitStack::set_last`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/bits/stack/bit.rs
+++ replace | with ^ in BitStack::set_last
@@ -147,17 +147,17 @@
     /// # Panics
     ///
     /// Panics if the stack is empty.
     pub(crate) fn set_last(&mut self, bit: bool) {
         if self.top_len > 0 {
             self.top = (self.top & !1) | u64::from(bit);
         } else {
             let word = self.words.last_mut().expect("set_last on an empty stack");
-            *word = (*word & !1) | u64::from(bit);
+            *word = (*word & !1) ^ /* ~ changed by cargo-mutants ~ */ u64::from(bit);
         }
     }
 
     /// The newest bit, unpopped.
     pub(crate) fn last(&self) -> Option<bool> {
         if self.top_len > 0 {
             Some(self.top & 1 == 1)
         } else {
```

## bits-19: `crates/before/src/bits/stack/packed_u64.rs:38:41`

- name: `crates/before/src/bits/stack/packed_u64.rs:38:41: replace & with | in PackedU64Stack::push`
- function: `PackedU64Stack::push`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/bits/stack/packed_u64.rs
+++ replace & with | in PackedU64Stack::push
@@ -30,17 +30,17 @@
     }
 
     /// Push a value, including zero.
     pub(crate) fn push(&mut self, value: u64) {
         let width = (u64::BITS - value.leading_zeros()).max(1);
         // Store the value high bit first. Pops then recover its low bits first,
         // which is the order `pop_bits` assembles into a word.
         if width == 64 {
-            self.value.push(value >> 63 & 1 == 1);
+            self.value.push(value >> 63 | /* ~ changed by cargo-mutants ~ */ 1 == 1);
             self.value.push_bits(value & (u64::MAX >> 1), 63);
         } else {
             self.value.push_bits(value, width);
         }
 
         // The terminator is pushed first, below the continuation bits, so a
         // pop sees `width - 1` set bits followed by the clear terminator.
         if width == 64 {
```

## bits-20: `crates/before/src/bits/stack/packed_u64.rs:63:30`

- name: `crates/before/src/bits/stack/packed_u64.rs:63:30: replace < with > in PackedU64Stack::pop`
- function: `PackedU64Stack::pop`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/bits/stack/packed_u64.rs
+++ replace < with > in PackedU64Stack::pop
@@ -55,17 +55,17 @@
     ///
     /// # Panics
     ///
     /// Panics if the stack is empty.
     pub(crate) fn pop(&mut self) -> u64 {
         // Most widths fit in the two registers inspected by this bounded
         // count. Only the widest values fall back to individual bit pops.
         let quick = self.widths.trailing_ones_capped();
-        let width = if quick < 62 {
+        let width = if quick > /* ~ changed by cargo-mutants ~ */ 62 {
             self.widths.pop_bits(quick + 1);
             quick + 1
         } else {
             let mut width = 0u32;
             loop {
                 let continuation = self.widths.pop().expect("bit stack underflow");
                 width += 1;
                 if !continuation {
```

## bits-21: `crates/before/src/bits/stack/packed_u64.rs:80:25`

- name: `crates/before/src/bits/stack/packed_u64.rs:80:25: replace | with ^ in PackedU64Stack::pop`
- function: `PackedU64Stack::pop`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/bits/stack/packed_u64.rs
+++ replace | with ^ in PackedU64Stack::pop
@@ -72,14 +72,14 @@
                     break;
                 }
             }
             width
         };
         if width == 64 {
             let low = self.value.pop_bits(63);
             let top = u64::from(self.value.pop().expect("bit stack value bits underflow"));
-            (top << 63) | low
+            (top << 63) ^ /* ~ changed by cargo-mutants ~ */ low
         } else {
             self.value.pop_bits(width)
         }
     }
 }
```

## bits-22: `crates/before/src/bits/storage.rs:82:9`

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

## bits-23: `crates/before/src/bits/storage.rs:83:13`

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

## bits-24: `crates/before/src/bits/storage.rs:84:13`

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

## bits-25: `crates/before/src/bits/storage.rs:156:9`

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

## bits-26: `crates/before/src/bits/writer.rs:194:47`

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

## bits-27: `crates/before/src/bits/writer.rs:244:32`

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

## bits-28: `crates/before/src/bits/writer.rs:328:9`

- name: `crates/before/src/bits/writer.rs:328:9: replace BitsWriter::tail_is_zeroed -> bool with true`
- function: `BitsWriter::tail_is_zeroed`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/bits/writer.rs
+++ replace BitsWriter::tail_is_zeroed -> bool with true
@@ -320,18 +320,17 @@
         if within != 0 {
             *self.bytes.last_mut().expect("a partial byte exists") &= 0xFF << (8 - within);
         }
     }
 
     /// Whether the final partial byte's dead bits are zero: the invariant,
     /// as a probe for the debug asserts.
     fn tail_is_zeroed(&self) -> bool {
-        let within = (self.live % 8) as u32;
-        within == 0 || self.bytes.last().is_some_and(|b| b & (0xFF >> within) == 0)
+        true /* ~ changed by cargo-mutants ~ */
     }
 
     /// Append whole bytes: a `memcpy` when the live length is
     /// byte-aligned, a two-shift merge per byte otherwise.
     fn extend_bytes(&mut self, body: &[u8]) {
         let within = (self.live % 8) as u32;
         if within == 0 {
             self.bytes.extend_from_slice(body);
```

## bits-29: `crates/before/src/bits/writer.rs:363:18`

- name: `crates/before/src/bits/writer.rs:363:18: replace > with == in BitsWriter::splice_storage`
- function: `BitsWriter::splice_storage`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/bits/writer.rs
+++ replace > with == in BitsWriter::splice_storage
@@ -355,17 +355,17 @@
         );
         scan::record_bits_u64(end - start);
         let mut position = start;
         while position < end && !position.is_multiple_of(8) {
             self.append_bit(bit(bytes, position));
             position += 1;
         }
         let whole = ((end - position) / 8) as usize;
-        if whole > 0 {
+        if whole == /* ~ changed by cargo-mutants ~ */ 0 {
             let byte = (position / 8) as usize;
             self.extend_bytes(&bytes[byte..byte + whole]);
             position += whole as u64 * 8;
         }
         while position < end {
             self.append_bit(bit(bytes, position));
             position += 1;
         }
```

## bits-30: `crates/before/src/bits/writer.rs:363:18`

- name: `crates/before/src/bits/writer.rs:363:18: replace > with < in BitsWriter::splice_storage`
- function: `BitsWriter::splice_storage`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/bits/writer.rs
+++ replace > with < in BitsWriter::splice_storage
@@ -355,17 +355,17 @@
         );
         scan::record_bits_u64(end - start);
         let mut position = start;
         while position < end && !position.is_multiple_of(8) {
             self.append_bit(bit(bytes, position));
             position += 1;
         }
         let whole = ((end - position) / 8) as usize;
-        if whole > 0 {
+        if whole < /* ~ changed by cargo-mutants ~ */ 0 {
             let byte = (position / 8) as usize;
             self.extend_bytes(&bytes[byte..byte + whole]);
             position += whole as u64 * 8;
         }
         while position < end {
             self.append_bit(bit(bytes, position));
             position += 1;
         }
```

## bits-31: `crates/before/src/bits/writer.rs:363:18`

- name: `crates/before/src/bits/writer.rs:363:18: replace > with >= in BitsWriter::splice_storage`
- function: `BitsWriter::splice_storage`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/bits/writer.rs
+++ replace > with >= in BitsWriter::splice_storage
@@ -355,17 +355,17 @@
         );
         scan::record_bits_u64(end - start);
         let mut position = start;
         while position < end && !position.is_multiple_of(8) {
             self.append_bit(bit(bytes, position));
             position += 1;
         }
         let whole = ((end - position) / 8) as usize;
-        if whole > 0 {
+        if whole >= /* ~ changed by cargo-mutants ~ */ 0 {
             let byte = (position / 8) as usize;
             self.extend_bytes(&bytes[byte..byte + whole]);
             position += whole as u64 * 8;
         }
         while position < end {
             self.append_bit(bit(bytes, position));
             position += 1;
         }
```

## bits-32: `crates/before/src/bits/writer.rs:379:9`

- name: `crates/before/src/bits/writer.rs:379:9: replace <impl core::fmt::Debug for BitsWriter>::fmt -> core::fmt::Result with Ok(Default::default())`
- function: `<impl core::fmt::Debug for BitsWriter>::fmt`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/bits/writer.rs
+++ replace <impl core::fmt::Debug for BitsWriter>::fmt -> core::fmt::Result with Ok(Default::default())
@@ -371,21 +371,17 @@
         }
     }
 }
 
 /// Renders the live bits most-significant-first as `0`/`1`, the test
 /// suites' failure-message spelling.
 impl core::fmt::Debug for BitsWriter {
     fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
-        f.write_str("BitsWriter[")?;
-        for pos in 0..self.live {
-            f.write_str(if self.bit(pos) { "1" } else { "0" })?;
-        }
-        f.write_str("]")
+        Ok(Default::default()) /* ~ changed by cargo-mutants ~ */
     }
 }
 
 /// Collect bits into a buffer, oldest first: the test generators'
 /// construction form.
 impl FromIterator<bool> for BitsWriter {
     fn from_iter<I: IntoIterator<Item = bool>>(iter: I) -> Self {
         let mut out = BitsWriter::new();
```

