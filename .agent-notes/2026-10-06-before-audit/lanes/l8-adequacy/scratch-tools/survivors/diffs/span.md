# Mutation survivors: span

29 survivors. Each entry: the cargo-mutants name, the enclosing function,
and the exact diff cargo-mutants applied (apply with `patch -p0` from the worktree root
after stripping the `*** ` header lines, or re-create it by hand).

## span-1: `crates/before/src/causally/forms.rs:402:9`

- name: `crates/before/src/causally/forms.rs:402:9: replace <impl fmt::Debug for Floor<'_>>::fmt -> fmt::Result with Ok(Default::default())`
- function: `<impl fmt::Debug for Floor<'_>>::fmt`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/causally/forms.rs
+++ replace <impl fmt::Debug for Floor<'_>>::fmt -> fmt::Result with Ok(Default::default())
@@ -394,17 +394,17 @@
             }],
             polarity: PhantomData,
         }
     }
 }
 
 impl fmt::Debug for Floor<'_> {
     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
-        write!(f, "after({:?})", self.at)
+        Ok(Default::default()) /* ~ changed by cargo-mutants ~ */
     }
 }
 
 impl fmt::Debug for Ceiling<'_> {
     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
         write!(f, "before({:?})", self.at)
     }
 }
```

## span-2: `crates/before/src/causally/forms.rs:408:9`

- name: `crates/before/src/causally/forms.rs:408:9: replace <impl fmt::Debug for Ceiling<'_>>::fmt -> fmt::Result with Ok(Default::default())`
- function: `<impl fmt::Debug for Ceiling<'_>>::fmt`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/causally/forms.rs
+++ replace <impl fmt::Debug for Ceiling<'_>>::fmt -> fmt::Result with Ok(Default::default())
@@ -400,11 +400,11 @@
 impl fmt::Debug for Floor<'_> {
     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
         write!(f, "after({:?})", self.at)
     }
 }
 
 impl fmt::Debug for Ceiling<'_> {
     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
-        write!(f, "before({:?})", self.at)
+        Ok(Default::default()) /* ~ changed by cargo-mutants ~ */
     }
 }
```

## span-3: `crates/before/src/causally/polarity.rs:71:13`

- name: `crates/before/src/causally/polarity.rs:71:13: replace sealed::<impl Sealed for super::Down>::hole_subtracts -> bool with true`
- function: `sealed::<impl Sealed for super::Down>::hole_subtracts`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/causally/polarity.rs
+++ replace sealed::<impl Sealed for super::Down>::hole_subtracts -> bool with true
@@ -63,21 +63,17 @@
             if strict {
                 Demand::NotStrictlyBefore
             } else {
                 Demand::NotBefore
             }
         }
 
         fn hole_subtracts(hole: &Hole<'_>, probe: &Version) -> bool {
-            if hole.strict {
-                probe < hole.at.as_ref()
-            } else {
-                probe <= hole.at.as_ref()
-            }
+            true /* ~ changed by cargo-mutants ~ */
         }
 
         fn covering_endpoint<'a>(_clamped_lo: &'a Version, clamped_hi: &'a Version) -> &'a Version {
             clamped_hi
         }
 
         fn hole_survives(
             hole: &Hole<'_>,
```

## span-4: `crates/before/src/causally/polarity.rs:71:13`

- name: `crates/before/src/causally/polarity.rs:71:13: replace sealed::<impl Sealed for super::Down>::hole_subtracts -> bool with false`
- function: `sealed::<impl Sealed for super::Down>::hole_subtracts`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/causally/polarity.rs
+++ replace sealed::<impl Sealed for super::Down>::hole_subtracts -> bool with false
@@ -63,21 +63,17 @@
             if strict {
                 Demand::NotStrictlyBefore
             } else {
                 Demand::NotBefore
             }
         }
 
         fn hole_subtracts(hole: &Hole<'_>, probe: &Version) -> bool {
-            if hole.strict {
-                probe < hole.at.as_ref()
-            } else {
-                probe <= hole.at.as_ref()
-            }
+            false /* ~ changed by cargo-mutants ~ */
         }
 
         fn covering_endpoint<'a>(_clamped_lo: &'a Version, clamped_hi: &'a Version) -> &'a Version {
             clamped_hi
         }
 
         fn hole_survives(
             hole: &Hole<'_>,
```

## span-5: `crates/before/src/causally/polarity.rs:72:23`

- name: `crates/before/src/causally/polarity.rs:72:23: replace < with > in sealed::<impl Sealed for super::Down>::hole_subtracts`
- function: `sealed::<impl Sealed for super::Down>::hole_subtracts`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/causally/polarity.rs
+++ replace < with > in sealed::<impl Sealed for super::Down>::hole_subtracts
@@ -64,17 +64,17 @@
                 Demand::NotStrictlyBefore
             } else {
                 Demand::NotBefore
             }
         }
 
         fn hole_subtracts(hole: &Hole<'_>, probe: &Version) -> bool {
             if hole.strict {
-                probe < hole.at.as_ref()
+                probe > /* ~ changed by cargo-mutants ~ */ hole.at.as_ref()
             } else {
                 probe <= hole.at.as_ref()
             }
         }
 
         fn covering_endpoint<'a>(_clamped_lo: &'a Version, clamped_hi: &'a Version) -> &'a Version {
             clamped_hi
         }
```

## span-6: `crates/before/src/causally/polarity.rs:72:23`

- name: `crates/before/src/causally/polarity.rs:72:23: replace < with <= in sealed::<impl Sealed for super::Down>::hole_subtracts`
- function: `sealed::<impl Sealed for super::Down>::hole_subtracts`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/causally/polarity.rs
+++ replace < with <= in sealed::<impl Sealed for super::Down>::hole_subtracts
@@ -64,17 +64,17 @@
                 Demand::NotStrictlyBefore
             } else {
                 Demand::NotBefore
             }
         }
 
         fn hole_subtracts(hole: &Hole<'_>, probe: &Version) -> bool {
             if hole.strict {
-                probe < hole.at.as_ref()
+                probe <= /* ~ changed by cargo-mutants ~ */ hole.at.as_ref()
             } else {
                 probe <= hole.at.as_ref()
             }
         }
 
         fn covering_endpoint<'a>(_clamped_lo: &'a Version, clamped_hi: &'a Version) -> &'a Version {
             clamped_hi
         }
```

## span-7: `crates/before/src/causally/polarity.rs:72:23`

- name: `crates/before/src/causally/polarity.rs:72:23: replace < with == in sealed::<impl Sealed for super::Down>::hole_subtracts`
- function: `sealed::<impl Sealed for super::Down>::hole_subtracts`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/causally/polarity.rs
+++ replace < with == in sealed::<impl Sealed for super::Down>::hole_subtracts
@@ -64,17 +64,17 @@
                 Demand::NotStrictlyBefore
             } else {
                 Demand::NotBefore
             }
         }
 
         fn hole_subtracts(hole: &Hole<'_>, probe: &Version) -> bool {
             if hole.strict {
-                probe < hole.at.as_ref()
+                probe == /* ~ changed by cargo-mutants ~ */ hole.at.as_ref()
             } else {
                 probe <= hole.at.as_ref()
             }
         }
 
         fn covering_endpoint<'a>(_clamped_lo: &'a Version, clamped_hi: &'a Version) -> &'a Version {
             clamped_hi
         }
```

## span-8: `crates/before/src/causally/polarity.rs:74:23`

- name: `crates/before/src/causally/polarity.rs:74:23: replace <= with > in sealed::<impl Sealed for super::Down>::hole_subtracts`
- function: `sealed::<impl Sealed for super::Down>::hole_subtracts`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/causally/polarity.rs
+++ replace <= with > in sealed::<impl Sealed for super::Down>::hole_subtracts
@@ -66,17 +66,17 @@
                 Demand::NotBefore
             }
         }
 
         fn hole_subtracts(hole: &Hole<'_>, probe: &Version) -> bool {
             if hole.strict {
                 probe < hole.at.as_ref()
             } else {
-                probe <= hole.at.as_ref()
+                probe > /* ~ changed by cargo-mutants ~ */ hole.at.as_ref()
             }
         }
 
         fn covering_endpoint<'a>(_clamped_lo: &'a Version, clamped_hi: &'a Version) -> &'a Version {
             clamped_hi
         }
 
         fn hole_survives(
```

## span-9: `crates/before/src/causally/polarity.rs:90:31`

- name: `crates/before/src/causally/polarity.rs:90:31: replace < with <= in sealed::<impl Sealed for super::Down>::hole_survives`
- function: `sealed::<impl Sealed for super::Down>::hole_survives`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/causally/polarity.rs
+++ replace < with <= in sealed::<impl Sealed for super::Down>::hole_survives
@@ -82,17 +82,17 @@
         fn hole_survives(
             hole: &Hole<'_>,
             floor: Option<&Version>,
             _ceiling: Option<&Version>,
         ) -> bool {
             match floor {
                 Some(floor) => {
                     if hole.strict {
-                        floor < hole.at.as_ref()
+                        floor <= /* ~ changed by cargo-mutants ~ */ hole.at.as_ref()
                     } else {
                         floor <= hole.at.as_ref()
                     }
                 }
                 None => true,
             }
         }
```

## span-10: `crates/before/src/causally/polarity.rs:129:13`

- name: `crates/before/src/causally/polarity.rs:129:13: replace sealed::<impl Sealed for super::Up>::hole_subtracts -> bool with true`
- function: `sealed::<impl Sealed for super::Up>::hole_subtracts`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/causally/polarity.rs
+++ replace sealed::<impl Sealed for super::Up>::hole_subtracts -> bool with true
@@ -121,21 +121,17 @@
             if strict {
                 Demand::NotStrictlyAfter
             } else {
                 Demand::NotAfter
             }
         }
 
         fn hole_subtracts(hole: &Hole<'_>, probe: &Version) -> bool {
-            if hole.strict {
-                hole.at.as_ref() < probe
-            } else {
-                hole.at.as_ref() <= probe
-            }
+            true /* ~ changed by cargo-mutants ~ */
         }
 
         fn covering_endpoint<'a>(clamped_lo: &'a Version, _clamped_hi: &'a Version) -> &'a Version {
             clamped_lo
         }
 
         fn hole_survives(
             hole: &Hole<'_>,
```

## span-11: `crates/before/src/causally/polarity.rs:129:13`

- name: `crates/before/src/causally/polarity.rs:129:13: replace sealed::<impl Sealed for super::Up>::hole_subtracts -> bool with false`
- function: `sealed::<impl Sealed for super::Up>::hole_subtracts`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/causally/polarity.rs
+++ replace sealed::<impl Sealed for super::Up>::hole_subtracts -> bool with false
@@ -121,21 +121,17 @@
             if strict {
                 Demand::NotStrictlyAfter
             } else {
                 Demand::NotAfter
             }
         }
 
         fn hole_subtracts(hole: &Hole<'_>, probe: &Version) -> bool {
-            if hole.strict {
-                hole.at.as_ref() < probe
-            } else {
-                hole.at.as_ref() <= probe
-            }
+            false /* ~ changed by cargo-mutants ~ */
         }
 
         fn covering_endpoint<'a>(clamped_lo: &'a Version, _clamped_hi: &'a Version) -> &'a Version {
             clamped_lo
         }
 
         fn hole_survives(
             hole: &Hole<'_>,
```

## span-12: `crates/before/src/causally/polarity.rs:130:34`

- name: `crates/before/src/causally/polarity.rs:130:34: replace < with == in sealed::<impl Sealed for super::Up>::hole_subtracts`
- function: `sealed::<impl Sealed for super::Up>::hole_subtracts`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/causally/polarity.rs
+++ replace < with == in sealed::<impl Sealed for super::Up>::hole_subtracts
@@ -122,17 +122,17 @@
                 Demand::NotStrictlyAfter
             } else {
                 Demand::NotAfter
             }
         }
 
         fn hole_subtracts(hole: &Hole<'_>, probe: &Version) -> bool {
             if hole.strict {
-                hole.at.as_ref() < probe
+                hole.at.as_ref() == /* ~ changed by cargo-mutants ~ */ probe
             } else {
                 hole.at.as_ref() <= probe
             }
         }
 
         fn covering_endpoint<'a>(clamped_lo: &'a Version, _clamped_hi: &'a Version) -> &'a Version {
             clamped_lo
         }
```

## span-13: `crates/before/src/causally/polarity.rs:130:34`

- name: `crates/before/src/causally/polarity.rs:130:34: replace < with > in sealed::<impl Sealed for super::Up>::hole_subtracts`
- function: `sealed::<impl Sealed for super::Up>::hole_subtracts`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/causally/polarity.rs
+++ replace < with > in sealed::<impl Sealed for super::Up>::hole_subtracts
@@ -122,17 +122,17 @@
                 Demand::NotStrictlyAfter
             } else {
                 Demand::NotAfter
             }
         }
 
         fn hole_subtracts(hole: &Hole<'_>, probe: &Version) -> bool {
             if hole.strict {
-                hole.at.as_ref() < probe
+                hole.at.as_ref() > /* ~ changed by cargo-mutants ~ */ probe
             } else {
                 hole.at.as_ref() <= probe
             }
         }
 
         fn covering_endpoint<'a>(clamped_lo: &'a Version, _clamped_hi: &'a Version) -> &'a Version {
             clamped_lo
         }
```

## span-14: `crates/before/src/causally/polarity.rs:130:34`

- name: `crates/before/src/causally/polarity.rs:130:34: replace < with <= in sealed::<impl Sealed for super::Up>::hole_subtracts`
- function: `sealed::<impl Sealed for super::Up>::hole_subtracts`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/causally/polarity.rs
+++ replace < with <= in sealed::<impl Sealed for super::Up>::hole_subtracts
@@ -122,17 +122,17 @@
                 Demand::NotStrictlyAfter
             } else {
                 Demand::NotAfter
             }
         }
 
         fn hole_subtracts(hole: &Hole<'_>, probe: &Version) -> bool {
             if hole.strict {
-                hole.at.as_ref() < probe
+                hole.at.as_ref() <= /* ~ changed by cargo-mutants ~ */ probe
             } else {
                 hole.at.as_ref() <= probe
             }
         }
 
         fn covering_endpoint<'a>(clamped_lo: &'a Version, _clamped_hi: &'a Version) -> &'a Version {
             clamped_lo
         }
```

## span-15: `crates/before/src/causally/polarity.rs:132:34`

- name: `crates/before/src/causally/polarity.rs:132:34: replace <= with > in sealed::<impl Sealed for super::Up>::hole_subtracts`
- function: `sealed::<impl Sealed for super::Up>::hole_subtracts`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/causally/polarity.rs
+++ replace <= with > in sealed::<impl Sealed for super::Up>::hole_subtracts
@@ -124,17 +124,17 @@
                 Demand::NotAfter
             }
         }
 
         fn hole_subtracts(hole: &Hole<'_>, probe: &Version) -> bool {
             if hole.strict {
                 hole.at.as_ref() < probe
             } else {
-                hole.at.as_ref() <= probe
+                hole.at.as_ref() > /* ~ changed by cargo-mutants ~ */ probe
             }
         }
 
         fn covering_endpoint<'a>(clamped_lo: &'a Version, _clamped_hi: &'a Version) -> &'a Version {
             clamped_lo
         }
 
         fn hole_survives(
```

## span-16: `crates/before/src/causally/polarity.rs:145:13`

- name: `crates/before/src/causally/polarity.rs:145:13: replace sealed::<impl Sealed for super::Up>::hole_survives -> bool with true`
- function: `sealed::<impl Sealed for super::Up>::hole_survives`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/causally/polarity.rs
+++ replace sealed::<impl Sealed for super::Up>::hole_survives -> bool with true
@@ -137,26 +137,17 @@
             clamped_lo
         }
 
         fn hole_survives(
             hole: &Hole<'_>,
             _floor: Option<&Version>,
             ceiling: Option<&Version>,
         ) -> bool {
-            match ceiling {
-                Some(ceiling) => {
-                    if hole.strict {
-                        hole.at.as_ref() < ceiling
-                    } else {
-                        hole.at.as_ref() <= ceiling
-                    }
-                }
-                None => true,
-            }
+            true /* ~ changed by cargo-mutants ~ */
         }
 
         fn absorbs(a: &Hole<'_>, b: &Hole<'_>) -> bool {
             // The order-dual of the down-side rule.
             match b.at.partial_cmp(&a.at) {
                 Some(Ordering::Greater) => true,
                 Some(Ordering::Equal) => !a.strict || b.strict,
                 Some(Ordering::Less) | None => false,
```

## span-17: `crates/before/src/causally/polarity.rs:148:42`

- name: `crates/before/src/causally/polarity.rs:148:42: replace < with <= in sealed::<impl Sealed for super::Up>::hole_survives`
- function: `sealed::<impl Sealed for super::Up>::hole_survives`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/causally/polarity.rs
+++ replace < with <= in sealed::<impl Sealed for super::Up>::hole_survives
@@ -140,17 +140,17 @@
         fn hole_survives(
             hole: &Hole<'_>,
             _floor: Option<&Version>,
             ceiling: Option<&Version>,
         ) -> bool {
             match ceiling {
                 Some(ceiling) => {
                     if hole.strict {
-                        hole.at.as_ref() < ceiling
+                        hole.at.as_ref() <= /* ~ changed by cargo-mutants ~ */ ceiling
                     } else {
                         hole.at.as_ref() <= ceiling
                     }
                 }
                 None => true,
             }
         }
```

## span-18: `crates/before/src/causally/polarity.rs:159:13`

- name: `crates/before/src/causally/polarity.rs:159:13: replace sealed::<impl Sealed for super::Up>::absorbs -> bool with false`
- function: `sealed::<impl Sealed for super::Up>::absorbs`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/causally/polarity.rs
+++ replace sealed::<impl Sealed for super::Up>::absorbs -> bool with false
@@ -151,21 +151,17 @@
                     }
                 }
                 None => true,
             }
         }
 
         fn absorbs(a: &Hole<'_>, b: &Hole<'_>) -> bool {
             // The order-dual of the down-side rule.
-            match b.at.partial_cmp(&a.at) {
-                Some(Ordering::Greater) => true,
-                Some(Ordering::Equal) => !a.strict || b.strict,
-                Some(Ordering::Less) | None => false,
-            }
+            false /* ~ changed by cargo-mutants ~ */
         }
 
         fn hole_name(strict: bool) -> &'static str {
             if strict {
                 "!strictly_after"
             } else {
                 "!after"
             }
```

## span-19: `crates/before/src/causally/polarity.rs:161:52`

- name: `crates/before/src/causally/polarity.rs:161:52: replace || with && in sealed::<impl Sealed for super::Up>::absorbs`
- function: `sealed::<impl Sealed for super::Up>::absorbs`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/causally/polarity.rs
+++ replace || with && in sealed::<impl Sealed for super::Up>::absorbs
@@ -153,17 +153,17 @@
                 None => true,
             }
         }
 
         fn absorbs(a: &Hole<'_>, b: &Hole<'_>) -> bool {
             // The order-dual of the down-side rule.
             match b.at.partial_cmp(&a.at) {
                 Some(Ordering::Greater) => true,
-                Some(Ordering::Equal) => !a.strict || b.strict,
+                Some(Ordering::Equal) => !a.strict && /* ~ changed by cargo-mutants ~ */ b.strict,
                 Some(Ordering::Less) | None => false,
             }
         }
 
         fn hole_name(strict: bool) -> &'static str {
             if strict {
                 "!strictly_after"
             } else {
```

## span-20: `crates/before/src/causally/polarity.rs:183:13`

- name: `crates/before/src/causally/polarity.rs:183:13: replace sealed::<impl Sealed for super::Neutral>::hole_subtracts -> bool with true`
- function: `sealed::<impl Sealed for super::Neutral>::hole_subtracts`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/causally/polarity.rs
+++ replace sealed::<impl Sealed for super::Neutral>::hole_subtracts -> bool with true
@@ -175,17 +175,17 @@
     impl Sealed for super::Neutral {
         // A neutral query holds no holes, structurally: no construction
         // path adds one, so the dispatch is never consulted.
         fn hole_demand(_strict: bool) -> Demand {
             unreachable!("a neutral query holds no holes")
         }
 
         fn hole_subtracts(_hole: &Hole<'_>, _probe: &Version) -> bool {
-            unreachable!("a neutral query holds no holes")
+            true /* ~ changed by cargo-mutants ~ */
         }
 
         fn covering_endpoint<'a>(
             _clamped_lo: &'a Version,
             _clamped_hi: &'a Version,
         ) -> &'a Version {
             unreachable!("a neutral query holds no holes")
         }
```

## span-21: `crates/before/src/causally/polarity.rs:183:13`

- name: `crates/before/src/causally/polarity.rs:183:13: replace sealed::<impl Sealed for super::Neutral>::hole_subtracts -> bool with false`
- function: `sealed::<impl Sealed for super::Neutral>::hole_subtracts`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/causally/polarity.rs
+++ replace sealed::<impl Sealed for super::Neutral>::hole_subtracts -> bool with false
@@ -175,17 +175,17 @@
     impl Sealed for super::Neutral {
         // A neutral query holds no holes, structurally: no construction
         // path adds one, so the dispatch is never consulted.
         fn hole_demand(_strict: bool) -> Demand {
             unreachable!("a neutral query holds no holes")
         }
 
         fn hole_subtracts(_hole: &Hole<'_>, _probe: &Version) -> bool {
-            unreachable!("a neutral query holds no holes")
+            false /* ~ changed by cargo-mutants ~ */
         }
 
         fn covering_endpoint<'a>(
             _clamped_lo: &'a Version,
             _clamped_hi: &'a Version,
         ) -> &'a Version {
             unreachable!("a neutral query holds no holes")
         }
```

## span-22: `crates/before/src/causally/polarity.rs:190:13`

- name: `crates/before/src/causally/polarity.rs:190:13: replace sealed::<impl Sealed for super::Neutral>::covering_endpoint -> &'a Version with Box::leak(Box::new(Default::default()))`
- function: `sealed::<impl Sealed for super::Neutral>::covering_endpoint`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/causally/polarity.rs
+++ replace sealed::<impl Sealed for super::Neutral>::covering_endpoint -> &'a Version with Box::leak(Box::new(Default::default()))
@@ -182,17 +182,17 @@
         fn hole_subtracts(_hole: &Hole<'_>, _probe: &Version) -> bool {
             unreachable!("a neutral query holds no holes")
         }
 
         fn covering_endpoint<'a>(
             _clamped_lo: &'a Version,
             _clamped_hi: &'a Version,
         ) -> &'a Version {
-            unreachable!("a neutral query holds no holes")
+            Box::leak(Box::new(Default::default())) /* ~ changed by cargo-mutants ~ */
         }
 
         fn hole_survives(
             _hole: &Hole<'_>,
             _floor: Option<&Version>,
             _ceiling: Option<&Version>,
         ) -> bool {
             unreachable!("a neutral query holds no holes")
```

## span-23: `crates/before/src/causally/polarity.rs:198:13`

- name: `crates/before/src/causally/polarity.rs:198:13: replace sealed::<impl Sealed for super::Neutral>::hole_survives -> bool with true`
- function: `sealed::<impl Sealed for super::Neutral>::hole_survives`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/causally/polarity.rs
+++ replace sealed::<impl Sealed for super::Neutral>::hole_survives -> bool with true
@@ -190,17 +190,17 @@
             unreachable!("a neutral query holds no holes")
         }
 
         fn hole_survives(
             _hole: &Hole<'_>,
             _floor: Option<&Version>,
             _ceiling: Option<&Version>,
         ) -> bool {
-            unreachable!("a neutral query holds no holes")
+            true /* ~ changed by cargo-mutants ~ */
         }
 
         fn absorbs(_a: &Hole<'_>, _b: &Hole<'_>) -> bool {
             unreachable!("a neutral query holds no holes")
         }
 
         fn hole_name(_strict: bool) -> &'static str {
             unreachable!("a neutral query holds no holes")
```

## span-24: `crates/before/src/causally/polarity.rs:198:13`

- name: `crates/before/src/causally/polarity.rs:198:13: replace sealed::<impl Sealed for super::Neutral>::hole_survives -> bool with false`
- function: `sealed::<impl Sealed for super::Neutral>::hole_survives`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/causally/polarity.rs
+++ replace sealed::<impl Sealed for super::Neutral>::hole_survives -> bool with false
@@ -190,17 +190,17 @@
             unreachable!("a neutral query holds no holes")
         }
 
         fn hole_survives(
             _hole: &Hole<'_>,
             _floor: Option<&Version>,
             _ceiling: Option<&Version>,
         ) -> bool {
-            unreachable!("a neutral query holds no holes")
+            false /* ~ changed by cargo-mutants ~ */
         }
 
         fn absorbs(_a: &Hole<'_>, _b: &Hole<'_>) -> bool {
             unreachable!("a neutral query holds no holes")
         }
 
         fn hole_name(_strict: bool) -> &'static str {
             unreachable!("a neutral query holds no holes")
```

## span-25: `crates/before/src/causally/polarity.rs:202:13`

- name: `crates/before/src/causally/polarity.rs:202:13: replace sealed::<impl Sealed for super::Neutral>::absorbs -> bool with false`
- function: `sealed::<impl Sealed for super::Neutral>::absorbs`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/causally/polarity.rs
+++ replace sealed::<impl Sealed for super::Neutral>::absorbs -> bool with false
@@ -194,17 +194,17 @@
             _hole: &Hole<'_>,
             _floor: Option<&Version>,
             _ceiling: Option<&Version>,
         ) -> bool {
             unreachable!("a neutral query holds no holes")
         }
 
         fn absorbs(_a: &Hole<'_>, _b: &Hole<'_>) -> bool {
-            unreachable!("a neutral query holds no holes")
+            false /* ~ changed by cargo-mutants ~ */
         }
 
         fn hole_name(_strict: bool) -> &'static str {
             unreachable!("a neutral query holds no holes")
         }
     }
 }
```

## span-26: `crates/before/src/causally/polarity.rs:206:13`

- name: `crates/before/src/causally/polarity.rs:206:13: replace sealed::<impl Sealed for super::Neutral>::hole_name -> &'static str with ""`
- function: `sealed::<impl Sealed for super::Neutral>::hole_name`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/causally/polarity.rs
+++ replace sealed::<impl Sealed for super::Neutral>::hole_name -> &'static str with ""
@@ -198,17 +198,17 @@
             unreachable!("a neutral query holds no holes")
         }
 
         fn absorbs(_a: &Hole<'_>, _b: &Hole<'_>) -> bool {
             unreachable!("a neutral query holds no holes")
         }
 
         fn hole_name(_strict: bool) -> &'static str {
-            unreachable!("a neutral query holds no holes")
+            "" /* ~ changed by cargo-mutants ~ */
         }
     }
 }
 
 /// A query's polarity: which complement family it may subtract.
 ///
 /// Restricting every hole to one direction lets
 /// [`Query::coverage`](crate::causally::Query::coverage) remain exact without
```

## span-27: `crates/before/src/causally/polarity.rs:206:13`

- name: `crates/before/src/causally/polarity.rs:206:13: replace sealed::<impl Sealed for super::Neutral>::hole_name -> &'static str with "xyzzy"`
- function: `sealed::<impl Sealed for super::Neutral>::hole_name`; genre: FnValue; outcome: MissedMutant

```diff
--- crates/before/src/causally/polarity.rs
+++ replace sealed::<impl Sealed for super::Neutral>::hole_name -> &'static str with "xyzzy"
@@ -198,17 +198,17 @@
             unreachable!("a neutral query holds no holes")
         }
 
         fn absorbs(_a: &Hole<'_>, _b: &Hole<'_>) -> bool {
             unreachable!("a neutral query holds no holes")
         }
 
         fn hole_name(_strict: bool) -> &'static str {
-            unreachable!("a neutral query holds no holes")
+            "xyzzy" /* ~ changed by cargo-mutants ~ */
         }
     }
 }
 
 /// A query's polarity: which complement family it may subtract.
 ///
 /// Restricting every hole to one direction lets
 /// [`Query::coverage`](crate::causally::Query::coverage) remain exact without
```

## span-28: `crates/before/src/span/algebra.rs:410:20`

- name: `crates/before/src/span/algebra.rs:410:20: delete ! in Span<'a>::fold_endpoints`
- function: `Span<'a>::fold_endpoints`; genre: UnaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/span/algebra.rs
+++ delete ! in Span<'a>::fold_endpoints
@@ -402,17 +402,17 @@
         let mut last: Option<(Version, Version)> = None;
         let inputs = core::iter::once(FoldInput::Receiver(self))
             .chain(iter.into_iter().map(FoldInput::Item))
             .filter(move |input| {
                 let s = input.span();
                 let dup = last
                     .as_ref()
                     .is_some_and(|(lo, hi)| lo.ptr_eq(s.lo()) && hi.ptr_eq(s.hi()));
-                if !dup {
+                if  /* ~ changed by cargo-mutants ~ */dup {
                     last = Some((s.lo().clone(), s.hi().clone()));
                 }
                 !dup
             })
             .map(Group::Input);
         let group = crate::fold::balanced_reduce(inputs, |a, b| {
             // Combining two point spans needs one lattice operation rather
             // than separate work for equal lower and upper endpoints. Shared
```

## span-29: `crates/before/src/span/wire.rs:136:21`

- name: `crates/before/src/span/wire.rs:136:21: replace > with >= in Span<'a>::decode_bytes`
- function: `Span<'a>::decode_bytes`; genre: BinaryOperator; outcome: MissedMutant

```diff
--- crates/before/src/span/wire.rs
+++ replace > with >= in Span<'a>::decode_bytes
@@ -128,17 +128,17 @@
     /// Validates an owned canonical encoding and shares its storage between
     /// the endpoints.
     pub(crate) fn decode_bytes(buf: bytes::Bytes) -> Result<Span<'static>, Decode> {
         // A version tree is self-delimiting at the bit level. Validate the
         // first tree, then include its marker and padding to find the byte at
         // which the upper endpoint starts.
         let lo_end = validate::prefix(BitsReader::from_bytes(&buf))?;
         let lo_bytes = (lo_end + 1).div_ceil(8);
-        if lo_bytes > buf.len() as u64 {
+        if lo_bytes >= /* ~ changed by cargo-mutants ~ */ buf.len() as u64 {
             return Err(Decode::Truncated);
         }
         let lo_bytes = usize::try_from(lo_bytes)
             .expect("the lower endpoint ends within the owned input buffer");
         Bits::validate_padding(&buf[..lo_bytes], lo_end)?;
         let lo = Version::from_canonical(Bits::from_canonical(buf.slice(..lo_bytes)));
 
         // The lower endpoint is now known to be canonical. That lets the
```

