# Mutation survivors: rank

47 survivors. Each entry: the cargo-mutants name, the enclosing function,
and the exact diff cargo-mutants applied (apply with `patch -p0` from the worktree root
after stripping the `*** ` header lines, or re-create it by hand).

## rank-1: `crates/before/src/accumulator.rs:99:9`

- name: `crates/before/src/accumulator.rs:99:9: delete match arm [] in magnitude`
- function: `magnitude`; genre: MatchArm; outcome: MissedMutant

```diff
--- crates/before/src/accumulator.rs
+++ delete match arm [] in magnitude
@@ -91,17 +91,17 @@
             self.sub_shifted_limbs(0, value.magnitude().iter_u64_digits());
         }
     }
 }
 
 /// Convert suanpan's little-endian 64-bit words into `BigUint`'s 32-bit digits.
 fn magnitude(words: &[u64]) -> BigUint {
     match words {
-        [] => BigUint::ZERO,
+         /* ~ changed by cargo-mutants ~ */
         &[low] => BigUint::from(low),
         &[low, high] => BigUint::from(u128::from(low) | (u128::from(high) << 64)),
         _ => BigUint::new(
             words
                 .iter()
                 .flat_map(|&word| [word as u32, (word >> 32) as u32])
                 .collect(),
         ),
```

## rank-2: `crates/before/src/accumulator.rs:100:9`

- name: `crates/before/src/accumulator.rs:100:9: delete match arm &[low] in magnitude`
- function: `magnitude`; genre: MatchArm; outcome: MissedMutant

```diff
--- crates/before/src/accumulator.rs
+++ delete match arm &[low] in magnitude
@@ -92,17 +92,17 @@
         }
     }
 }
 
 /// Convert suanpan's little-endian 64-bit words into `BigUint`'s 32-bit digits.
 fn magnitude(words: &[u64]) -> BigUint {
     match words {
         [] => BigUint::ZERO,
-        &[low] => BigUint::from(low),
+         /* ~ changed by cargo-mutants ~ */
         &[low, high] => BigUint::from(u128::from(low) | (u128::from(high) << 64)),
         _ => BigUint::new(
             words
                 .iter()
                 .flat_map(|&word| [word as u32, (word >> 32) as u32])
                 .collect(),
         ),
     }
```

## rank-3: `crates/before/src/accumulator.rs:101:9`

- name: `crates/before/src/accumulator.rs:101:9: delete match arm &[low, high] in magnitude`
- function: `magnitude`; genre: MatchArm; outcome: MissedMutant

```diff
--- crates/before/src/accumulator.rs
+++ delete match arm &[low, high] in magnitude
@@ -93,17 +93,17 @@
     }
 }
 
 /// Convert suanpan's little-endian 64-bit words into `BigUint`'s 32-bit digits.
 fn magnitude(words: &[u64]) -> BigUint {
     match words {
         [] => BigUint::ZERO,
         &[low] => BigUint::from(low),
-        &[low, high] => BigUint::from(u128::from(low) | (u128::from(high) << 64)),
+         /* ~ changed by cargo-mutants ~ */
         _ => BigUint::new(
             words
                 .iter()
                 .flat_map(|&word| [word as u32, (word >> 32) as u32])
                 .collect(),
         ),
     }
 }
```

## rank-4: `crates/before/src/accumulator.rs:101:55`

- name: `crates/before/src/accumulator.rs:101:55: replace | with ^ in magnitude`
- function: `magnitude`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/accumulator.rs
+++ replace | with ^ in magnitude
@@ -93,17 +93,17 @@
     }
 }
 
 /// Convert suanpan's little-endian 64-bit words into `BigUint`'s 32-bit digits.
 fn magnitude(words: &[u64]) -> BigUint {
     match words {
         [] => BigUint::ZERO,
         &[low] => BigUint::from(low),
-        &[low, high] => BigUint::from(u128::from(low) | (u128::from(high) << 64)),
+        &[low, high] => BigUint::from(u128::from(low) ^ /* ~ changed by cargo-mutants ~ */ (u128::from(high) << 64)),
         _ => BigUint::new(
             words
                 .iter()
                 .flat_map(|&word| [word as u32, (word >> 32) as u32])
                 .collect(),
         ),
     }
 }
```

## rank-5: `crates/before/src/rank.rs:378:31`

- name: `crates/before/src/rank.rs:378:31: replace match guard error.kind() == io::ErrorKind::Interrupted with false in Rank::decode`
- function: `Rank::decode`; genre: MatchArmGuard; outcome: MissedMutant

```diff
--- crates/before/src/rank.rs
+++ replace match guard error.kind() == io::ErrorKind::Interrupted with false in Rank::decode
@@ -370,17 +370,17 @@
             match reader.read(&mut buf[end..]) {
                 Ok(0) => break true,
                 Ok(read) => {
                     end += read;
                     if end == buf.len() {
                         break false;
                     }
                 }
-                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
+                Err(error) if false /* ~ changed by cargo-mutants ~ */ => {}
                 Err(error) => return Err(Decode::Io(error)),
             }
         };
 
         // Most inputs end within the fixed prefix. Decode those through the
         // simpler slice path; only an incomplete full prefix needs incremental
         // reading. Retrying a bounded prefix keeps that slow path linear.
         match Self::decode_bytes(&buf[..end]) {
```

## rank-6: `crates/before/src/rank.rs:387:25`

- name: `crates/before/src/rank.rs:387:25: replace match guard at_eof with true in Rank::decode`
- function: `Rank::decode`; genre: MatchArmGuard; outcome: MissedMutant

```diff
--- crates/before/src/rank.rs
+++ replace match guard at_eof with true in Rank::decode
@@ -379,17 +379,17 @@
                 Err(error) => return Err(Decode::Io(error)),
             }
         };
 
         // Most inputs end within the fixed prefix. Decode those through the
         // simpler slice path; only an incomplete full prefix needs incremental
         // reading. Retrying a bounded prefix keeps that slow path linear.
         match Self::decode_bytes(&buf[..end]) {
-            Ok(rank) if at_eof => return Ok(rank),
+            Ok(rank) if true /* ~ changed by cargo-mutants ~ */ => return Ok(rank),
             Ok(rank) => loop {
                 match reader.read(&mut buf[..1]) {
                     Ok(0) => return Ok(rank),
                     Ok(_) => return Err(Decode::TrailingBits),
                     Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                     Err(error) => return Err(Decode::Io(error)),
                 }
             },
```

## rank-7: `crates/before/src/rank.rs:387:25`

- name: `crates/before/src/rank.rs:387:25: replace match guard at_eof with false in Rank::decode`
- function: `Rank::decode`; genre: MatchArmGuard; outcome: MissedMutant

```diff
--- crates/before/src/rank.rs
+++ replace match guard at_eof with false in Rank::decode
@@ -379,17 +379,17 @@
                 Err(error) => return Err(Decode::Io(error)),
             }
         };
 
         // Most inputs end within the fixed prefix. Decode those through the
         // simpler slice path; only an incomplete full prefix needs incremental
         // reading. Retrying a bounded prefix keeps that slow path linear.
         match Self::decode_bytes(&buf[..end]) {
-            Ok(rank) if at_eof => return Ok(rank),
+            Ok(rank) if false /* ~ changed by cargo-mutants ~ */ => return Ok(rank),
             Ok(rank) => loop {
                 match reader.read(&mut buf[..1]) {
                     Ok(0) => return Ok(rank),
                     Ok(_) => return Err(Decode::TrailingBits),
                     Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                     Err(error) => return Err(Decode::Io(error)),
                 }
             },
```

## rank-8: `crates/before/src/rank.rs:392:35`

- name: `crates/before/src/rank.rs:392:35: replace match guard error.kind() == io::ErrorKind::Interrupted with true in Rank::decode`
- function: `Rank::decode`; genre: MatchArmGuard; outcome: MissedMutant

```diff
--- crates/before/src/rank.rs
+++ replace match guard error.kind() == io::ErrorKind::Interrupted with true in Rank::decode
@@ -384,17 +384,17 @@
         // simpler slice path; only an incomplete full prefix needs incremental
         // reading. Retrying a bounded prefix keeps that slow path linear.
         match Self::decode_bytes(&buf[..end]) {
             Ok(rank) if at_eof => return Ok(rank),
             Ok(rank) => loop {
                 match reader.read(&mut buf[..1]) {
                     Ok(0) => return Ok(rank),
                     Ok(_) => return Err(Decode::TrailingBits),
-                    Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
+                    Err(error) if true /* ~ changed by cargo-mutants ~ */ => {}
                     Err(error) => return Err(Decode::Io(error)),
                 }
             },
             Err(Decode::Truncated) if at_eof => return Err(Decode::Truncated),
             Err(Decode::Truncated) => {}
             Err(error) => return Err(error),
         }
```

## rank-9: `crates/before/src/rank.rs:392:35`

- name: `crates/before/src/rank.rs:392:35: replace match guard error.kind() == io::ErrorKind::Interrupted with false in Rank::decode`
- function: `Rank::decode`; genre: MatchArmGuard; outcome: MissedMutant

```diff
--- crates/before/src/rank.rs
+++ replace match guard error.kind() == io::ErrorKind::Interrupted with false in Rank::decode
@@ -384,17 +384,17 @@
         // simpler slice path; only an incomplete full prefix needs incremental
         // reading. Retrying a bounded prefix keeps that slow path linear.
         match Self::decode_bytes(&buf[..end]) {
             Ok(rank) if at_eof => return Ok(rank),
             Ok(rank) => loop {
                 match reader.read(&mut buf[..1]) {
                     Ok(0) => return Ok(rank),
                     Ok(_) => return Err(Decode::TrailingBits),
-                    Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
+                    Err(error) if false /* ~ changed by cargo-mutants ~ */ => {}
                     Err(error) => return Err(Decode::Io(error)),
                 }
             },
             Err(Decode::Truncated) if at_eof => return Err(Decode::Truncated),
             Err(Decode::Truncated) => {}
             Err(error) => return Err(error),
         }
```

## rank-10: `crates/before/src/rank.rs:392:48`

- name: `crates/before/src/rank.rs:392:48: replace == with != in Rank::decode`
- function: `Rank::decode`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/rank.rs
+++ replace == with != in Rank::decode
@@ -384,17 +384,17 @@
         // simpler slice path; only an incomplete full prefix needs incremental
         // reading. Retrying a bounded prefix keeps that slow path linear.
         match Self::decode_bytes(&buf[..end]) {
             Ok(rank) if at_eof => return Ok(rank),
             Ok(rank) => loop {
                 match reader.read(&mut buf[..1]) {
                     Ok(0) => return Ok(rank),
                     Ok(_) => return Err(Decode::TrailingBits),
-                    Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
+                    Err(error) if error.kind() != /* ~ changed by cargo-mutants ~ */ io::ErrorKind::Interrupted => {}
                     Err(error) => return Err(Decode::Io(error)),
                 }
             },
             Err(Decode::Truncated) if at_eof => return Err(Decode::Truncated),
             Err(Decode::Truncated) => {}
             Err(error) => return Err(error),
         }
```

## rank-11: `crates/before/src/rank.rs:396:39`

- name: `crates/before/src/rank.rs:396:39: replace match guard at_eof with false in Rank::decode`
- function: `Rank::decode`; genre: MatchArmGuard; outcome: MissedMutant

```diff
--- crates/before/src/rank.rs
+++ replace match guard at_eof with false in Rank::decode
@@ -388,17 +388,17 @@
             Ok(rank) => loop {
                 match reader.read(&mut buf[..1]) {
                     Ok(0) => return Ok(rank),
                     Ok(_) => return Err(Decode::TrailingBits),
                     Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                     Err(error) => return Err(Decode::Io(error)),
                 }
             },
-            Err(Decode::Truncated) if at_eof => return Err(Decode::Truncated),
+            Err(Decode::Truncated) if false /* ~ changed by cargo-mutants ~ */ => return Err(Decode::Truncated),
             Err(Decode::Truncated) => {}
             Err(error) => return Err(error),
         }
 
         let mut next = 0;
         let rank = Self::decode_stream(|| loop {
             if next < end {
                 let byte = buf[next];
```

## rank-12: `crates/before/src/rank.rs:414:31`

- name: `crates/before/src/rank.rs:414:31: replace match guard error.kind() == io::ErrorKind::Interrupted with true in Rank::decode`
- function: `Rank::decode`; genre: MatchArmGuard; outcome: MissedMutant

```diff
--- crates/before/src/rank.rs
+++ replace match guard error.kind() == io::ErrorKind::Interrupted with true in Rank::decode
@@ -406,17 +406,17 @@
                 return Ok(byte);
             }
             match reader.read(&mut buf) {
                 Ok(0) => return Err(Decode::Truncated),
                 Ok(read) => {
                     next = 0;
                     end = read;
                 }
-                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
+                Err(error) if true /* ~ changed by cargo-mutants ~ */ => {}
                 Err(error) => return Err(Decode::Io(error)),
             }
         })?;
 
         if next < end {
             return Err(Decode::TrailingBits);
         }
         loop {
```

## rank-13: `crates/before/src/rank.rs:414:31`

- name: `crates/before/src/rank.rs:414:31: replace match guard error.kind() == io::ErrorKind::Interrupted with false in Rank::decode`
- function: `Rank::decode`; genre: MatchArmGuard; outcome: MissedMutant

```diff
--- crates/before/src/rank.rs
+++ replace match guard error.kind() == io::ErrorKind::Interrupted with false in Rank::decode
@@ -406,17 +406,17 @@
                 return Ok(byte);
             }
             match reader.read(&mut buf) {
                 Ok(0) => return Err(Decode::Truncated),
                 Ok(read) => {
                     next = 0;
                     end = read;
                 }
-                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
+                Err(error) if false /* ~ changed by cargo-mutants ~ */ => {}
                 Err(error) => return Err(Decode::Io(error)),
             }
         })?;
 
         if next < end {
             return Err(Decode::TrailingBits);
         }
         loop {
```

## rank-14: `crates/before/src/rank.rs:414:44`

- name: `crates/before/src/rank.rs:414:44: replace == with != in Rank::decode`
- function: `Rank::decode`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/rank.rs
+++ replace == with != in Rank::decode
@@ -406,17 +406,17 @@
                 return Ok(byte);
             }
             match reader.read(&mut buf) {
                 Ok(0) => return Err(Decode::Truncated),
                 Ok(read) => {
                     next = 0;
                     end = read;
                 }
-                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
+                Err(error) if error.kind() != /* ~ changed by cargo-mutants ~ */ io::ErrorKind::Interrupted => {}
                 Err(error) => return Err(Decode::Io(error)),
             }
         })?;
 
         if next < end {
             return Err(Decode::TrailingBits);
         }
         loop {
```

## rank-15: `crates/before/src/rank.rs:426:31`

- name: `crates/before/src/rank.rs:426:31: replace match guard error.kind() == io::ErrorKind::Interrupted with true in Rank::decode`
- function: `Rank::decode`; genre: MatchArmGuard; outcome: MissedMutant

```diff
--- crates/before/src/rank.rs
+++ replace match guard error.kind() == io::ErrorKind::Interrupted with true in Rank::decode
@@ -418,17 +418,17 @@
 
         if next < end {
             return Err(Decode::TrailingBits);
         }
         loop {
             match reader.read(&mut buf[..1]) {
                 Ok(0) => return Ok(rank),
                 Ok(_) => return Err(Decode::TrailingBits),
-                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
+                Err(error) if true /* ~ changed by cargo-mutants ~ */ => {}
                 Err(error) => return Err(Decode::Io(error)),
             }
         }
     }
 
     /// Decodes canonical bytes already held in memory.
     pub(crate) fn decode_bytes(bytes: &[u8]) -> Result<Rank, Decode> {
         let mut iter = bytes.iter();
```

## rank-16: `crates/before/src/rank.rs:426:31`

- name: `crates/before/src/rank.rs:426:31: replace match guard error.kind() == io::ErrorKind::Interrupted with false in Rank::decode`
- function: `Rank::decode`; genre: MatchArmGuard; outcome: MissedMutant

```diff
--- crates/before/src/rank.rs
+++ replace match guard error.kind() == io::ErrorKind::Interrupted with false in Rank::decode
@@ -418,17 +418,17 @@
 
         if next < end {
             return Err(Decode::TrailingBits);
         }
         loop {
             match reader.read(&mut buf[..1]) {
                 Ok(0) => return Ok(rank),
                 Ok(_) => return Err(Decode::TrailingBits),
-                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
+                Err(error) if false /* ~ changed by cargo-mutants ~ */ => {}
                 Err(error) => return Err(Decode::Io(error)),
             }
         }
     }
 
     /// Decodes canonical bytes already held in memory.
     pub(crate) fn decode_bytes(bytes: &[u8]) -> Result<Rank, Decode> {
         let mut iter = bytes.iter();
```

## rank-17: `crates/before/src/rank.rs:426:44`

- name: `crates/before/src/rank.rs:426:44: replace == with != in Rank::decode`
- function: `Rank::decode`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/rank.rs
+++ replace == with != in Rank::decode
@@ -418,17 +418,17 @@
 
         if next < end {
             return Err(Decode::TrailingBits);
         }
         loop {
             match reader.read(&mut buf[..1]) {
                 Ok(0) => return Ok(rank),
                 Ok(_) => return Err(Decode::TrailingBits),
-                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
+                Err(error) if error.kind() != /* ~ changed by cargo-mutants ~ */ io::ErrorKind::Interrupted => {}
                 Err(error) => return Err(Decode::Io(error)),
             }
         }
     }
 
     /// Decodes canonical bytes already held in memory.
     pub(crate) fn decode_bytes(bytes: &[u8]) -> Result<Rank, Decode> {
         let mut iter = bytes.iter();
```

## rank-18: `crates/before/src/rank.rs:497:9`

- name: `crates/before/src/rank.rs:497:9: replace Rank::alignment_fits -> bool with true`
- function: `Rank::alignment_fits`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/rank.rs
+++ replace Rank::alignment_fits -> bool with true
@@ -489,17 +489,17 @@
                     exp: exp - shift,
                 }
             }
         }
     }
 
     /// Whether both exponent gaps fit the big-integer shift interface.
     fn alignment_fits(a_exp: u64, b_exp: u64, common_exp: u64) -> bool {
-        usize::try_from(common_exp - a_exp).is_ok() && usize::try_from(common_exp - b_exp).is_ok()
+        true /* ~ changed by cargo-mutants ~ */
     }
 
     /// Combine `self ± rhs` at exponent `exp` through the streaming
     /// accumulator.
     ///
     /// Reserving for the wider aligned operand avoids a transient created by
     /// growth-doubling the buffer.
     fn accumulate(&self, rhs: &Rank, exp: u64, subtract_rhs: bool) -> Rank {
```

## rank-19: `crates/before/src/rank.rs:497:9`

- name: `crates/before/src/rank.rs:497:9: replace Rank::alignment_fits -> bool with false`
- function: `Rank::alignment_fits`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/rank.rs
+++ replace Rank::alignment_fits -> bool with false
@@ -489,17 +489,17 @@
                     exp: exp - shift,
                 }
             }
         }
     }
 
     /// Whether both exponent gaps fit the big-integer shift interface.
     fn alignment_fits(a_exp: u64, b_exp: u64, common_exp: u64) -> bool {
-        usize::try_from(common_exp - a_exp).is_ok() && usize::try_from(common_exp - b_exp).is_ok()
+        false /* ~ changed by cargo-mutants ~ */
     }
 
     /// Combine `self ± rhs` at exponent `exp` through the streaming
     /// accumulator.
     ///
     /// Reserving for the wider aligned operand avoids a transient created by
     /// growth-doubling the buffer.
     fn accumulate(&self, rhs: &Rank, exp: u64, subtract_rhs: bool) -> Rank {
```

## rank-20: `crates/before/src/rank.rs:497:36`

- name: `crates/before/src/rank.rs:497:36: replace - with + in Rank::alignment_fits`
- function: `Rank::alignment_fits`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/rank.rs
+++ replace - with + in Rank::alignment_fits
@@ -489,17 +489,17 @@
                     exp: exp - shift,
                 }
             }
         }
     }
 
     /// Whether both exponent gaps fit the big-integer shift interface.
     fn alignment_fits(a_exp: u64, b_exp: u64, common_exp: u64) -> bool {
-        usize::try_from(common_exp - a_exp).is_ok() && usize::try_from(common_exp - b_exp).is_ok()
+        usize::try_from(common_exp + /* ~ changed by cargo-mutants ~ */ a_exp).is_ok() && usize::try_from(common_exp - b_exp).is_ok()
     }
 
     /// Combine `self ± rhs` at exponent `exp` through the streaming
     /// accumulator.
     ///
     /// Reserving for the wider aligned operand avoids a transient created by
     /// growth-doubling the buffer.
     fn accumulate(&self, rhs: &Rank, exp: u64, subtract_rhs: bool) -> Rank {
```

## rank-21: `crates/before/src/rank.rs:497:53`

- name: `crates/before/src/rank.rs:497:53: replace && with || in Rank::alignment_fits`
- function: `Rank::alignment_fits`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/rank.rs
+++ replace && with || in Rank::alignment_fits
@@ -489,17 +489,17 @@
                     exp: exp - shift,
                 }
             }
         }
     }
 
     /// Whether both exponent gaps fit the big-integer shift interface.
     fn alignment_fits(a_exp: u64, b_exp: u64, common_exp: u64) -> bool {
-        usize::try_from(common_exp - a_exp).is_ok() && usize::try_from(common_exp - b_exp).is_ok()
+        usize::try_from(common_exp - a_exp).is_ok() || /* ~ changed by cargo-mutants ~ */ usize::try_from(common_exp - b_exp).is_ok()
     }
 
     /// Combine `self ± rhs` at exponent `exp` through the streaming
     /// accumulator.
     ///
     /// Reserving for the wider aligned operand avoids a transient created by
     /// growth-doubling the buffer.
     fn accumulate(&self, rhs: &Rank, exp: u64, subtract_rhs: bool) -> Rank {
```

## rank-22: `crates/before/src/rank.rs:497:83`

- name: `crates/before/src/rank.rs:497:83: replace - with + in Rank::alignment_fits`
- function: `Rank::alignment_fits`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/rank.rs
+++ replace - with + in Rank::alignment_fits
@@ -489,17 +489,17 @@
                     exp: exp - shift,
                 }
             }
         }
     }
 
     /// Whether both exponent gaps fit the big-integer shift interface.
     fn alignment_fits(a_exp: u64, b_exp: u64, common_exp: u64) -> bool {
-        usize::try_from(common_exp - a_exp).is_ok() && usize::try_from(common_exp - b_exp).is_ok()
+        usize::try_from(common_exp - a_exp).is_ok() && usize::try_from(common_exp + /* ~ changed by cargo-mutants ~ */ b_exp).is_ok()
     }
 
     /// Combine `self ± rhs` at exponent `exp` through the streaming
     /// accumulator.
     ///
     /// Reserving for the wider aligned operand avoids a transient created by
     /// growth-doubling the buffer.
     fn accumulate(&self, rhs: &Rank, exp: u64, subtract_rhs: bool) -> Rank {
```

## rank-23: `crates/before/src/rank.rs:506:9`

- name: `crates/before/src/rank.rs:506:9: replace Rank::accumulate -> Rank with Default::default()`
- function: `Rank::accumulate`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/rank.rs
+++ replace Rank::accumulate -> Rank with Default::default()
@@ -498,41 +498,17 @@
     }
 
     /// Combine `self ± rhs` at exponent `exp` through the streaming
     /// accumulator.
     ///
     /// Reserving for the wider aligned operand avoids a transient created by
     /// growth-doubling the buffer.
     fn accumulate(&self, rhs: &Rank, exp: u64, subtract_rhs: bool) -> Rank {
-        let mut acc = Accumulator::new();
-        let aligned_bits = |rank: &Rank| {
-            if rank.num.bits() == 0 {
-                0
-            } else {
-                rank.num.bits().saturating_add(exp - rank.exp)
-            }
-        };
-        let widest = aligned_bits(self).max(aligned_bits(rhs)).saturating_add(1);
-        if let Ok(digits) = usize::try_from(widest / 32 + 2) {
-            acc.reserve_digits(digits);
-        }
-        acc.add_shifted_limbs(exp - self.exp, self.num.iter_u64_digits());
-        if subtract_rhs {
-            acc.sub_shifted_limbs(exp - rhs.exp, rhs.num.iter_u64_digits());
-        } else {
-            acc.add_shifted_limbs(exp - rhs.exp, rhs.num.iter_u64_digits());
-        }
-        let (sign, num) = acc.biguint_parts();
-        debug_assert_ne!(
-            sign,
-            Ordering::Less,
-            "rank addition and pre-checked subtraction are nonnegative"
-        );
-        Rank::from_raw(num, exp)
+        Default::default() /* ~ changed by cargo-mutants ~ */
     }
 
     /// Write the canonical prefix-ascending stream for `num · 2⁻ᵉˣᵖ`.
     fn encode_parts_to<W: Write>(num: &BigUint, exp: u64, writer: &mut W) -> io::Result<()> {
         // The header encodes the integer part: m = ⌊r⌋ + 1,
         // w = bits(m), ρ = bits(w) − 1. A shift past the numerator's width
         // yields zero, meaning only that r < 1; the fraction loop below still
         // writes the remaining value exactly.
```

## rank-24: `crates/before/src/rank.rs:508:32`

- name: `crates/before/src/rank.rs:508:32: replace == with != in Rank::accumulate`
- function: `Rank::accumulate`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/rank.rs
+++ replace == with != in Rank::accumulate
@@ -500,17 +500,17 @@
     /// Combine `self ± rhs` at exponent `exp` through the streaming
     /// accumulator.
     ///
     /// Reserving for the wider aligned operand avoids a transient created by
     /// growth-doubling the buffer.
     fn accumulate(&self, rhs: &Rank, exp: u64, subtract_rhs: bool) -> Rank {
         let mut acc = Accumulator::new();
         let aligned_bits = |rank: &Rank| {
-            if rank.num.bits() == 0 {
+            if rank.num.bits() != /* ~ changed by cargo-mutants ~ */ 0 {
                 0
             } else {
                 rank.num.bits().saturating_add(exp - rank.exp)
             }
         };
         let widest = aligned_bits(self).max(aligned_bits(rhs)).saturating_add(1);
         if let Ok(digits) = usize::try_from(widest / 32 + 2) {
             acc.reserve_digits(digits);
```

## rank-25: `crates/before/src/rank.rs:511:52`

- name: `crates/before/src/rank.rs:511:52: replace - with + in Rank::accumulate`
- function: `Rank::accumulate`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/rank.rs
+++ replace - with + in Rank::accumulate
@@ -503,17 +503,17 @@
     /// Reserving for the wider aligned operand avoids a transient created by
     /// growth-doubling the buffer.
     fn accumulate(&self, rhs: &Rank, exp: u64, subtract_rhs: bool) -> Rank {
         let mut acc = Accumulator::new();
         let aligned_bits = |rank: &Rank| {
             if rank.num.bits() == 0 {
                 0
             } else {
-                rank.num.bits().saturating_add(exp - rank.exp)
+                rank.num.bits().saturating_add(exp + /* ~ changed by cargo-mutants ~ */ rank.exp)
             }
         };
         let widest = aligned_bits(self).max(aligned_bits(rhs)).saturating_add(1);
         if let Ok(digits) = usize::try_from(widest / 32 + 2) {
             acc.reserve_digits(digits);
         }
         acc.add_shifted_limbs(exp - self.exp, self.num.iter_u64_digits());
         if subtract_rhs {
```

## rank-26: `crates/before/src/rank.rs:511:52`

- name: `crates/before/src/rank.rs:511:52: replace - with / in Rank::accumulate`
- function: `Rank::accumulate`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/rank.rs
+++ replace - with / in Rank::accumulate
@@ -503,17 +503,17 @@
     /// Reserving for the wider aligned operand avoids a transient created by
     /// growth-doubling the buffer.
     fn accumulate(&self, rhs: &Rank, exp: u64, subtract_rhs: bool) -> Rank {
         let mut acc = Accumulator::new();
         let aligned_bits = |rank: &Rank| {
             if rank.num.bits() == 0 {
                 0
             } else {
-                rank.num.bits().saturating_add(exp - rank.exp)
+                rank.num.bits().saturating_add(exp / /* ~ changed by cargo-mutants ~ */ rank.exp)
             }
         };
         let widest = aligned_bits(self).max(aligned_bits(rhs)).saturating_add(1);
         if let Ok(digits) = usize::try_from(widest / 32 + 2) {
             acc.reserve_digits(digits);
         }
         acc.add_shifted_limbs(exp - self.exp, self.num.iter_u64_digits());
         if subtract_rhs {
```

## rank-27: `crates/before/src/rank.rs:515:52`

- name: `crates/before/src/rank.rs:515:52: replace / with % in Rank::accumulate`
- function: `Rank::accumulate`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/rank.rs
+++ replace / with % in Rank::accumulate
@@ -507,17 +507,17 @@
         let aligned_bits = |rank: &Rank| {
             if rank.num.bits() == 0 {
                 0
             } else {
                 rank.num.bits().saturating_add(exp - rank.exp)
             }
         };
         let widest = aligned_bits(self).max(aligned_bits(rhs)).saturating_add(1);
-        if let Ok(digits) = usize::try_from(widest / 32 + 2) {
+        if let Ok(digits) = usize::try_from(widest % /* ~ changed by cargo-mutants ~ */ 32 + 2) {
             acc.reserve_digits(digits);
         }
         acc.add_shifted_limbs(exp - self.exp, self.num.iter_u64_digits());
         if subtract_rhs {
             acc.sub_shifted_limbs(exp - rhs.exp, rhs.num.iter_u64_digits());
         } else {
             acc.add_shifted_limbs(exp - rhs.exp, rhs.num.iter_u64_digits());
         }
```

## rank-28: `crates/before/src/rank.rs:515:52`

- name: `crates/before/src/rank.rs:515:52: replace / with * in Rank::accumulate`
- function: `Rank::accumulate`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/rank.rs
+++ replace / with * in Rank::accumulate
@@ -507,17 +507,17 @@
         let aligned_bits = |rank: &Rank| {
             if rank.num.bits() == 0 {
                 0
             } else {
                 rank.num.bits().saturating_add(exp - rank.exp)
             }
         };
         let widest = aligned_bits(self).max(aligned_bits(rhs)).saturating_add(1);
-        if let Ok(digits) = usize::try_from(widest / 32 + 2) {
+        if let Ok(digits) = usize::try_from(widest * /* ~ changed by cargo-mutants ~ */ 32 + 2) {
             acc.reserve_digits(digits);
         }
         acc.add_shifted_limbs(exp - self.exp, self.num.iter_u64_digits());
         if subtract_rhs {
             acc.sub_shifted_limbs(exp - rhs.exp, rhs.num.iter_u64_digits());
         } else {
             acc.add_shifted_limbs(exp - rhs.exp, rhs.num.iter_u64_digits());
         }
```

## rank-29: `crates/before/src/rank.rs:515:57`

- name: `crates/before/src/rank.rs:515:57: replace + with - in Rank::accumulate`
- function: `Rank::accumulate`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/rank.rs
+++ replace + with - in Rank::accumulate
@@ -507,17 +507,17 @@
         let aligned_bits = |rank: &Rank| {
             if rank.num.bits() == 0 {
                 0
             } else {
                 rank.num.bits().saturating_add(exp - rank.exp)
             }
         };
         let widest = aligned_bits(self).max(aligned_bits(rhs)).saturating_add(1);
-        if let Ok(digits) = usize::try_from(widest / 32 + 2) {
+        if let Ok(digits) = usize::try_from(widest / 32 - /* ~ changed by cargo-mutants ~ */ 2) {
             acc.reserve_digits(digits);
         }
         acc.add_shifted_limbs(exp - self.exp, self.num.iter_u64_digits());
         if subtract_rhs {
             acc.sub_shifted_limbs(exp - rhs.exp, rhs.num.iter_u64_digits());
         } else {
             acc.add_shifted_limbs(exp - rhs.exp, rhs.num.iter_u64_digits());
         }
```

## rank-30: `crates/before/src/rank.rs:515:57`

- name: `crates/before/src/rank.rs:515:57: replace + with * in Rank::accumulate`
- function: `Rank::accumulate`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/rank.rs
+++ replace + with * in Rank::accumulate
@@ -507,17 +507,17 @@
         let aligned_bits = |rank: &Rank| {
             if rank.num.bits() == 0 {
                 0
             } else {
                 rank.num.bits().saturating_add(exp - rank.exp)
             }
         };
         let widest = aligned_bits(self).max(aligned_bits(rhs)).saturating_add(1);
-        if let Ok(digits) = usize::try_from(widest / 32 + 2) {
+        if let Ok(digits) = usize::try_from(widest / 32 * /* ~ changed by cargo-mutants ~ */ 2) {
             acc.reserve_digits(digits);
         }
         acc.add_shifted_limbs(exp - self.exp, self.num.iter_u64_digits());
         if subtract_rhs {
             acc.sub_shifted_limbs(exp - rhs.exp, rhs.num.iter_u64_digits());
         } else {
             acc.add_shifted_limbs(exp - rhs.exp, rhs.num.iter_u64_digits());
         }
```

## rank-31: `crates/before/src/rank.rs:518:35`

- name: `crates/before/src/rank.rs:518:35: replace - with + in Rank::accumulate`
- function: `Rank::accumulate`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/rank.rs
+++ replace - with + in Rank::accumulate
@@ -510,17 +510,17 @@
             } else {
                 rank.num.bits().saturating_add(exp - rank.exp)
             }
         };
         let widest = aligned_bits(self).max(aligned_bits(rhs)).saturating_add(1);
         if let Ok(digits) = usize::try_from(widest / 32 + 2) {
             acc.reserve_digits(digits);
         }
-        acc.add_shifted_limbs(exp - self.exp, self.num.iter_u64_digits());
+        acc.add_shifted_limbs(exp + /* ~ changed by cargo-mutants ~ */ self.exp, self.num.iter_u64_digits());
         if subtract_rhs {
             acc.sub_shifted_limbs(exp - rhs.exp, rhs.num.iter_u64_digits());
         } else {
             acc.add_shifted_limbs(exp - rhs.exp, rhs.num.iter_u64_digits());
         }
         let (sign, num) = acc.biguint_parts();
         debug_assert_ne!(
             sign,
```

## rank-32: `crates/before/src/rank.rs:518:35`

- name: `crates/before/src/rank.rs:518:35: replace - with / in Rank::accumulate`
- function: `Rank::accumulate`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/rank.rs
+++ replace - with / in Rank::accumulate
@@ -510,17 +510,17 @@
             } else {
                 rank.num.bits().saturating_add(exp - rank.exp)
             }
         };
         let widest = aligned_bits(self).max(aligned_bits(rhs)).saturating_add(1);
         if let Ok(digits) = usize::try_from(widest / 32 + 2) {
             acc.reserve_digits(digits);
         }
-        acc.add_shifted_limbs(exp - self.exp, self.num.iter_u64_digits());
+        acc.add_shifted_limbs(exp / /* ~ changed by cargo-mutants ~ */ self.exp, self.num.iter_u64_digits());
         if subtract_rhs {
             acc.sub_shifted_limbs(exp - rhs.exp, rhs.num.iter_u64_digits());
         } else {
             acc.add_shifted_limbs(exp - rhs.exp, rhs.num.iter_u64_digits());
         }
         let (sign, num) = acc.biguint_parts();
         debug_assert_ne!(
             sign,
```

## rank-33: `crates/before/src/rank.rs:520:39`

- name: `crates/before/src/rank.rs:520:39: replace - with + in Rank::accumulate`
- function: `Rank::accumulate`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/rank.rs
+++ replace - with + in Rank::accumulate
@@ -512,17 +512,17 @@
             }
         };
         let widest = aligned_bits(self).max(aligned_bits(rhs)).saturating_add(1);
         if let Ok(digits) = usize::try_from(widest / 32 + 2) {
             acc.reserve_digits(digits);
         }
         acc.add_shifted_limbs(exp - self.exp, self.num.iter_u64_digits());
         if subtract_rhs {
-            acc.sub_shifted_limbs(exp - rhs.exp, rhs.num.iter_u64_digits());
+            acc.sub_shifted_limbs(exp + /* ~ changed by cargo-mutants ~ */ rhs.exp, rhs.num.iter_u64_digits());
         } else {
             acc.add_shifted_limbs(exp - rhs.exp, rhs.num.iter_u64_digits());
         }
         let (sign, num) = acc.biguint_parts();
         debug_assert_ne!(
             sign,
             Ordering::Less,
             "rank addition and pre-checked subtraction are nonnegative"
```

## rank-34: `crates/before/src/rank.rs:520:39`

- name: `crates/before/src/rank.rs:520:39: replace - with / in Rank::accumulate`
- function: `Rank::accumulate`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/rank.rs
+++ replace - with / in Rank::accumulate
@@ -512,17 +512,17 @@
             }
         };
         let widest = aligned_bits(self).max(aligned_bits(rhs)).saturating_add(1);
         if let Ok(digits) = usize::try_from(widest / 32 + 2) {
             acc.reserve_digits(digits);
         }
         acc.add_shifted_limbs(exp - self.exp, self.num.iter_u64_digits());
         if subtract_rhs {
-            acc.sub_shifted_limbs(exp - rhs.exp, rhs.num.iter_u64_digits());
+            acc.sub_shifted_limbs(exp / /* ~ changed by cargo-mutants ~ */ rhs.exp, rhs.num.iter_u64_digits());
         } else {
             acc.add_shifted_limbs(exp - rhs.exp, rhs.num.iter_u64_digits());
         }
         let (sign, num) = acc.biguint_parts();
         debug_assert_ne!(
             sign,
             Ordering::Less,
             "rank addition and pre-checked subtraction are nonnegative"
```

## rank-35: `crates/before/src/rank.rs:522:39`

- name: `crates/before/src/rank.rs:522:39: replace - with / in Rank::accumulate`
- function: `Rank::accumulate`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/rank.rs
+++ replace - with / in Rank::accumulate
@@ -514,17 +514,17 @@
         let widest = aligned_bits(self).max(aligned_bits(rhs)).saturating_add(1);
         if let Ok(digits) = usize::try_from(widest / 32 + 2) {
             acc.reserve_digits(digits);
         }
         acc.add_shifted_limbs(exp - self.exp, self.num.iter_u64_digits());
         if subtract_rhs {
             acc.sub_shifted_limbs(exp - rhs.exp, rhs.num.iter_u64_digits());
         } else {
-            acc.add_shifted_limbs(exp - rhs.exp, rhs.num.iter_u64_digits());
+            acc.add_shifted_limbs(exp / /* ~ changed by cargo-mutants ~ */ rhs.exp, rhs.num.iter_u64_digits());
         }
         let (sign, num) = acc.biguint_parts();
         debug_assert_ne!(
             sign,
             Ordering::Less,
             "rank addition and pre-checked subtraction are nonnegative"
         );
         Rank::from_raw(num, exp)
```

## rank-36: `crates/before/src/rank.rs:522:39`

- name: `crates/before/src/rank.rs:522:39: replace - with + in Rank::accumulate`
- function: `Rank::accumulate`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/rank.rs
+++ replace - with + in Rank::accumulate
@@ -514,17 +514,17 @@
         let widest = aligned_bits(self).max(aligned_bits(rhs)).saturating_add(1);
         if let Ok(digits) = usize::try_from(widest / 32 + 2) {
             acc.reserve_digits(digits);
         }
         acc.add_shifted_limbs(exp - self.exp, self.num.iter_u64_digits());
         if subtract_rhs {
             acc.sub_shifted_limbs(exp - rhs.exp, rhs.num.iter_u64_digits());
         } else {
-            acc.add_shifted_limbs(exp - rhs.exp, rhs.num.iter_u64_digits());
+            acc.add_shifted_limbs(exp + /* ~ changed by cargo-mutants ~ */ rhs.exp, rhs.num.iter_u64_digits());
         }
         let (sign, num) = acc.biguint_parts();
         debug_assert_ne!(
             sign,
             Ordering::Less,
             "rank addition and pre-checked subtraction are nonnegative"
         );
         Rank::from_raw(num, exp)
```

## rank-37: `crates/before/src/rank.rs:647:24`

- name: `crates/before/src/rank.rs:647:24: replace | with ^ in Rank::decode_stream`
- function: `Rank::decode_stream`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/rank.rs
+++ replace | with ^ in Rank::decode_stream
@@ -639,17 +639,17 @@
         if rho >= 64 {
             // The format stores the integral width in `u64`; a longer header
             // cannot name a representable width.
             return Err(Decode::NotCanonical);
         }
         // w's bits below its (implied) leading bit: ρ of them, so w < 2⁶⁴.
         let mut w = 1u64;
         for _ in 0..rho {
-            w = w << 1 | u64::from(src.bit()?);
+            w = w << 1 ^ /* ~ changed by cargo-mutants ~ */ u64::from(src.bit()?);
         }
         // The biased integral m: its implied leading bit, then w − 1 stream
         // bits, sunk MSB-first and unbiased at materialization.
         let mut mantissa = BitSink::new();
         mantissa.push(true);
         for _ in 0..w - 1 {
             mantissa.push(src.bit()?);
         }
```

## rank-38: `crates/before/src/rank.rs:667:36`

- name: `crates/before/src/rank.rs:667:36: replace | with ^ in Rank::decode_stream`
- function: `Rank::decode_stream`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/rank.rs
+++ replace | with ^ in Rank::decode_stream
@@ -659,17 +659,17 @@
         // `u8`s until the single `BigUint` materialization below.
         let mut groups: Vec<u8> = Vec::new();
         loop {
             if !src.bit()? {
                 break;
             }
             let mut group = 0u8;
             for _ in 0..FRACTION_GROUP_BITS {
-                group = group << 1 | u8::from(src.bit()?);
+                group = group << 1 ^ /* ~ changed by cargo-mutants ~ */ u8::from(src.bit()?);
             }
             groups.push(group);
         }
         // Strict minimal packing within the final byte: the bits after the close
         // bit are padding and must be zero.
         if src.used < 8 && src.current & (0xFF >> src.used) != 0 {
             return Err(Decode::TrailingBits);
         }
```

## rank-39: `crates/before/src/rank.rs:806:21`

- name: `crates/before/src/rank.rs:806:21: replace == with != in BitWriter<'a, W>::push_byte`
- function: `BitWriter<'a, W>::push_byte`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/rank.rs
+++ replace == with != in BitWriter<'a, W>::push_byte
@@ -798,17 +798,17 @@
         if self.used == 8 {
             self.push_byte()?;
         }
         Ok(())
     }
 
     /// Stage the completed current byte.
     fn push_byte(&mut self) -> io::Result<()> {
-        if self.len == self.bytes.len() {
+        if self.len != /* ~ changed by cargo-mutants ~ */ self.bytes.len() {
             self.flush()?;
         }
         self.bytes[self.len] = self.current;
         self.len += 1;
         self.current = 0;
         self.used = 0;
         Ok(())
     }
```

## rank-40: `crates/before/src/rank.rs:893:33`

- name: `crates/before/src/rank.rs:893:33: replace | with ^ in Rank::aligned_words`
- function: `Rank::aligned_words`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/rank.rs
+++ replace | with ^ in Rank::aligned_words
@@ -885,17 +885,17 @@
         let mut words = self.num.iter_u64_digits().rev();
         let mut current = words.next();
         let mut next = words.next();
         std::iter::from_fn(move || {
             let word = current?;
             let aligned = if shift == 0 {
                 word
             } else {
-                (word << shift) | (next.unwrap_or(0) >> (64 - shift))
+                (word << shift) ^ /* ~ changed by cargo-mutants ~ */ (next.unwrap_or(0) >> (64 - shift))
             };
             current = next;
             next = words.next();
             Some(aligned)
         })
     }
 }
```

## rank-41: `crates/before/src/rank.rs:983:9`

- name: `crates/before/src/rank.rs:983:9: replace <impl Add<Rank> for &Rank>::add -> Rank with Default::default()`
- function: `<impl Add<Rank> for &Rank>::add`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/rank.rs
+++ replace <impl Add<Rank> for &Rank>::add -> Rank with Default::default()
@@ -975,17 +975,17 @@
     fn add(self, rhs: &Rank) -> Rank {
         &self + rhs
     }
 }
 
 impl Add<Rank> for &Rank {
     type Output = Rank;
     fn add(self, rhs: Rank) -> Rank {
-        self + &rhs
+        Default::default() /* ~ changed by cargo-mutants ~ */
     }
 }
 
 impl AddAssign<&Rank> for Rank {
     fn add_assign(&mut self, rhs: &Rank) {
         *self = &*self + rhs;
     }
 }
```

## rank-42: `crates/before/src/rank.rs:989:9`

- name: `crates/before/src/rank.rs:989:9: replace <impl AddAssign<&Rank> for Rank>::add_assign with ()`
- function: `<impl AddAssign<&Rank> for Rank>::add_assign`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/rank.rs
+++ replace <impl AddAssign<&Rank> for Rank>::add_assign with ()
@@ -981,17 +981,17 @@
     type Output = Rank;
     fn add(self, rhs: Rank) -> Rank {
         self + &rhs
     }
 }
 
 impl AddAssign<&Rank> for Rank {
     fn add_assign(&mut self, rhs: &Rank) {
-        *self = &*self + rhs;
+        () /* ~ changed by cargo-mutants ~ */
     }
 }
 
 impl AddAssign<Rank> for Rank {
     fn add_assign(&mut self, rhs: Rank) {
         *self = &*self + &rhs;
     }
 }
```

## rank-43: `crates/before/src/rank.rs:995:9`

- name: `crates/before/src/rank.rs:995:9: replace <impl AddAssign<Rank> for Rank>::add_assign with ()`
- function: `<impl AddAssign<Rank> for Rank>::add_assign`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/rank.rs
+++ replace <impl AddAssign<Rank> for Rank>::add_assign with ()
@@ -987,17 +987,17 @@
 impl AddAssign<&Rank> for Rank {
     fn add_assign(&mut self, rhs: &Rank) {
         *self = &*self + rhs;
     }
 }
 
 impl AddAssign<Rank> for Rank {
     fn add_assign(&mut self, rhs: Rank) {
-        *self = &*self + &rhs;
+        () /* ~ changed by cargo-mutants ~ */
     }
 }
 
 /// Sums owned ranks, with [`Rank::ZERO`] as the empty sum.
 ///
 /// # Complexity
 ///
 /// For `k` ranks whose binary widths sum to `n`, `O(k + n)` time and `O(n)`
```

## rank-44: `crates/before/src/rank.rs:1052:16`

- name: `crates/before/src/rank.rs:1052:16: delete ! in Rank::sum_iter`
- function: `Rank::sum_iter`; genre: UnaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/rank.rs
+++ delete ! in Rank::sum_iter
@@ -1044,17 +1044,17 @@
         let mut acc = Accumulator::new();
         let mut exp = 0u64;
         let mut has_value = false;
         for rank in iter {
             let rank = rank.borrow();
             if rank.num == BigUint::ZERO {
                 continue;
             }
-            if !has_value {
+            if  /* ~ changed by cargo-mutants ~ */has_value {
                 exp = rank.exp;
                 acc.add_shifted_limbs(0, rank.num.iter_u64_digits());
                 has_value = true;
                 continue;
             }
             if rank.exp > exp {
                 let gap = rank.exp - exp;
                 let held_span = acc.stored_bits();
```

## rank-45: `crates/before/src/rank.rs:1058:25`

- name: `crates/before/src/rank.rs:1058:25: replace > with >= in Rank::sum_iter`
- function: `Rank::sum_iter`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/rank.rs
+++ replace > with >= in Rank::sum_iter
@@ -1050,17 +1050,17 @@
                 continue;
             }
             if !has_value {
                 exp = rank.exp;
                 acc.add_shifted_limbs(0, rank.num.iter_u64_digits());
                 has_value = true;
                 continue;
             }
-            if rank.exp > exp {
+            if rank.exp >= /* ~ changed by cargo-mutants ~ */ exp {
                 let gap = rank.exp - exp;
                 let held_span = acc.stored_bits();
                 let shift = gap.max(held_span).min(u64::MAX - exp);
                 acc <<= shift;
                 exp += shift;
             }
             acc.add_shifted_limbs(exp - rank.exp, rank.num.iter_u64_digits());
         }
```

## rank-46: `crates/before/src/rank.rs:1059:36`

- name: `crates/before/src/rank.rs:1059:36: replace - with + in Rank::sum_iter`
- function: `Rank::sum_iter`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/rank.rs
+++ replace - with + in Rank::sum_iter
@@ -1051,17 +1051,17 @@
             }
             if !has_value {
                 exp = rank.exp;
                 acc.add_shifted_limbs(0, rank.num.iter_u64_digits());
                 has_value = true;
                 continue;
             }
             if rank.exp > exp {
-                let gap = rank.exp - exp;
+                let gap = rank.exp + /* ~ changed by cargo-mutants ~ */ exp;
                 let held_span = acc.stored_bits();
                 let shift = gap.max(held_span).min(u64::MAX - exp);
                 acc <<= shift;
                 exp += shift;
             }
             acc.add_shifted_limbs(exp - rank.exp, rank.num.iter_u64_digits());
         }
         let (sign, num) = acc.biguint_parts();
```

## rank-47: `crates/before/src/ranked.rs:328:9`

- name: `crates/before/src/ranked.rs:328:9: replace <impl core::fmt::Debug for Ranked<'_>>::fmt -> core::fmt::Result with Ok(Default::default())`
- function: `<impl core::fmt::Debug for Ranked<'_>>::fmt`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/ranked.rs
+++ replace <impl core::fmt::Debug for Ranked<'_>>::fmt -> core::fmt::Result with Ok(Default::default())
@@ -320,19 +320,17 @@
         ranked.rank()
     }
 }
 
 /// Renders the viewed version, tagged with the type's name; the rank is
 /// derived state, so it is not (re)computed for a debug dump.
 impl core::fmt::Debug for Ranked<'_> {
     fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
-        f.debug_struct("Ranked")
-            .field("version", &self.version)
-            .finish()
+        Ok(Default::default()) /* ~ changed by cargo-mutants ~ */
     }
 }
 
 /// Compares version identity, matching [`Ord`] and [`Hash`](core::hash::Hash).
 impl PartialEq<Ranked<'_>> for Ranked<'_> {
     fn eq(&self, other: &Ranked<'_>) -> bool {
         self.version() == other.version()
     }
```

