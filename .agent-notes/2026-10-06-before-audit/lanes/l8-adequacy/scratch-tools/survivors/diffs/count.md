# Mutation survivors: count

3 survivors. Each entry: the cargo-mutants name, the enclosing function,
and the exact diff cargo-mutants applied (apply with `patch -p0` from the worktree root
after stripping the `*** ` header lines, or re-create it by hand).

## count-1: `crates/before/src/count.rs:303:9`

- name: `crates/before/src/count.rs:303:9: replace <impl fmt::Debug for Count>::fmt -> fmt::Result with Ok(Default::default())`
- function: `<impl fmt::Debug for Count>::fmt`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/count.rs
+++ replace <impl fmt::Debug for Count>::fmt -> fmt::Result with Ok(Default::default())
@@ -295,17 +295,17 @@
         let value = BigUint::parse_bytes(bytes, 10).ok_or(ParseValue::InvalidSyntax)?;
         Ok(Count(value))
     }
 }
 
 /// The same format as `Display`.
 impl fmt::Debug for Count {
     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
-        <Self as fmt::Display>::fmt(self, f)
+        Ok(Default::default()) /* ~ changed by cargo-mutants ~ */
     }
 }
 
 impl Add<&Count> for &Count {
     type Output = Count;
     fn add(self, rhs: &Count) -> Count {
         Count(&self.0 + &rhs.0)
     }
```

## count-2: `crates/before/src/count.rs:324:9`

- name: `crates/before/src/count.rs:324:9: replace <impl Add<&Count> for Count>::add -> Count with Default::default()`
- function: `<impl Add<&Count> for Count>::add`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/count.rs
+++ replace <impl Add<&Count> for Count>::add -> Count with Default::default()
@@ -316,17 +316,17 @@
     fn add(self, rhs: Count) -> Count {
         &self + &rhs
     }
 }
 
 impl Add<&Count> for Count {
     type Output = Count;
     fn add(self, rhs: &Count) -> Count {
-        &self + rhs
+        Default::default() /* ~ changed by cargo-mutants ~ */
     }
 }
 
 impl Add<Count> for &Count {
     type Output = Count;
     fn add(self, rhs: Count) -> Count {
         self + &rhs
     }
```

## count-3: `crates/before/src/count.rs:360:9`

- name: `crates/before/src/count.rs:360:9: replace <impl Sum<&'a Count> for Count>::sum -> Count with Default::default()`
- function: `<impl Sum<&'a Count> for Count>::sum`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/count.rs
+++ replace <impl Sum<&'a Count> for Count>::sum -> Count with Default::default()
@@ -352,17 +352,14 @@
             acc
         })
     }
 }
 
 /// Sums the iterator's counts; the empty sum is [`Count::ZERO`].
 impl<'a> Sum<&'a Count> for Count {
     fn sum<I: Iterator<Item = &'a Count>>(iter: I) -> Count {
-        iter.fold(Count::ZERO, |mut acc, t| {
-            acc += t;
-            acc
-        })
+        Default::default() /* ~ changed by cargo-mutants ~ */
     }
 }
 
 #[cfg(test)]
 mod tests;
```

