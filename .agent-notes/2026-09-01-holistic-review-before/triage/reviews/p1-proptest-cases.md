<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as the review packet of the before triage lane p1-proptest-cases; the annotations under each hunk are the lane agent's own account, the sections above them the coordinator's; not authored, audited, or endorsed by Finch. Reply inline with lines beginning `>> finch:`. -->

# Review packet: lane `p1-proptest-cases` (branch `before/p1-proptest-cases`)

## Goal

Ruling 109: no property test anywhere sets its case count explicitly, so one environment variable raises the count for every crate in CI and no helper is needed. Before's three explicit sites lose their literals; the prose that did arithmetic over the literal count is restated per case.

## Rulings landed

109.

## Stack position

Parent: `main`. Children: none. Root files: none.

## Acceptance

| Entry | Commit | Command | Decisive output |
|---|---|---|---|
| the three sites (ruling 109) | `97591f5a` | `grep -rn --include='*.rs' 'with_cases\|cases:' crates/before crates/suanpan` (coordinator) | no proptest configuration site remains; the `cases:` hits are local variables |
| oracle suite | `82250f59` | `cargo nextest run --locked -p before --all-features -E 'test(/semantic_oracle/)'` on the box (lane's run, log read by the coordinator) | 12 tests run: 12 passed |
| fuzzfit harness | `dba2b88e`, `e21038d5` | the fuzzfit recipe's build and nextest run on the box (lane's run, log read by the coordinator) | 20 tests run: 20 passed; clippy -D warnings clean |

## Fresh-eyes rounds

None; a three-site mechanical lane. The coordinator read the whole diff as the light final review: the literals are gone, the enforce doc's four sentences that computed odds from the old count are restated per case from the family weights, and the oracle site's comment (which justified the count) is deleted rather than rewritten since the test's doc already states the canary's role.

## What the lane found

The oracle site's `TestRunner::new(Config { cases: 400, ..Config::default() })` was silently ignoring `PROPTEST_CASES`: proptest's default config applies the environment when it is built, and the struct update then overrode `cases`. The two fuzzfit sites inside `proptest!` did honor it (the macro re-reads the environment over the supplied config). Relayed to the rumors session, whose committed check must key on any explicit `cases` field or `with_cases`, not on the macro form alone.

Run-time movement (box, not a clean A/B: before-runs under load 114, after-runs under load 10): the only clear signal is `programs_are_well_formed` 0.22 s to 1.49 s at 256 cases; every enforce test costs about 10 s of guest setup regardless of count, so the sentry's stated reason for 48 (it is the sentry, not the sweep) is not defeated.

## Stops

None. One out-of-lane finding handed to the gate lane, which owns the justfile this wave: the `fuzzfit` recipe comment says "48 fuzzed programs", now stale.

## Reading order

### tests and prose

- proptest-cases (109) at `crates/before/fuzzfit/harness/tests/enforce.rs:21` ([hunk](#hunk-2))
- proptest-cases (109) at `crates/before/fuzzfit/harness/tests/enforce.rs:35` ([hunk](#hunk-3))
- proptest-cases (109) at `crates/before/fuzzfit/harness/tests/enforce.rs:338` ([hunk](#hunk-4))
- proptest-cases (109) at `crates/before/fuzzfit/harness/tests/enforce.rs:401` ([hunk](#hunk-5))
- proptest-cases (109) at `crates/before/fuzzfit/harness/tests/enforce.rs:424` ([hunk](#hunk-6))
- proptest-cases (109) at `crates/before/fuzzfit/harness/tests/enforce.rs:437` ([hunk](#hunk-7))
- proptest-cases (109) at `crates/before/fuzzfit/harness/tests/sanity.rs:13` ([hunk](#hunk-8))
- proptest-cases (109) at `crates/before/src/testing/semantic_oracle/tests.rs:554` ([hunk](#hunk-9))

## The change

<a id="hunk-1"></a>
### .agent-notes/2026-09-01-holistic-review-before/triage/annotations/p1-proptest-cases.tsv `@@ -0,0 +1,10 @@`

```diff
@@ -0,0 +1,10 @@
+# p1-proptest-cases lane annotations: path, line (new side of `git diff 7687f3b7...HEAD`), entry id, ruling, note.
+# Written by Claude (Fable 5.1) as the lane's implementer for Finch's review; not authored or endorsed by Finch.
+crates/before/fuzzfit/harness/tests/sanity.rs	13	proptest-cases	109	The generator sanity block ran a fixed 64 cases; it now runs proptest's default and reads PROPTEST_CASES. The three tests took well under a second at 64, so the default costs nothing worth noting.
+crates/before/fuzzfit/harness/tests/enforce.rs	21	proptest-cases	109	The module doc's luck bound multiplied by the literal count; restated for an n-case run so it holds at any count.
+crates/before/fuzzfit/harness/tests/enforce.rs	35	proptest-cases	109	The module doc had stated the fixed count and called PROPTEST_CASES an override (true at this site: the proptest! macro re-reads the environment over the supplied config). It now says the sentry runs proptest's default, raised by PROPTEST_CASES, and keeps the one reason a reader needs: the calibration corpus is the big sweep.
+crates/before/fuzzfit/harness/tests/enforce.rs	338	proptest-cases	109	The prefix-totality doc worked the luck bound at 48 cases (38% at q of 2%); the worked number goes and the bound stays in per-case terms, since the argument for the deterministic leg is that the bound is never zero, not its value at one count.
+crates/before/fuzzfit/harness/tests/enforce.rs	401	proptest-cases	109	The escalation-replay doc argued from a 48-case run (usually never leaves the small-operand regime; independent regime nearly three times per run). Restated per case from any_family's weights (one in 137; eight in 137), which is what the code fixes; the replay's justification (deep reach must not ride on random draws) is count-independent.
+crates/before/fuzzfit/harness/tests/enforce.rs	424	proptest-cases	109	The depth-cap replay doc said the upper depth range arrives about once in five runs, arithmetic over 48 cases. Restated per case: the upper half of the escalation range is half of one in 137, about one case in 274.
+crates/before/fuzzfit/harness/tests/enforce.rs	437	proptest-cases	109	The sentry ran a fixed 48 cases; it now runs proptest's default and reads PROPTEST_CASES. This is the one site whose stated reason (the sentry is not the big sweep) the default defeats in run time: see the lane report for the before/after timing so Finch can rule.
+crates/before/src/testing/semantic_oracle/tests.rs	554	proptest-cases	109	The grid-cap canary built its runner with cases: 400 over Config::default(), which discarded PROPTEST_CASES (Config::default() already carries it; the struct update overrode the field). It now runs proptest's default and reads the variable. The inline comment justifying the count is deleted rather than rewritten: the test's doc comment already states that the sweep is the canary and the structural derivation is the guarantee.
```

*(review record; no annotation expected)*

<a id="hunk-2"></a>
### crates/before/fuzzfit/harness/tests/enforce.rs `@@ -18,7 +18,7 @@`

```diff
@@ -18,7 +18,7 @@
 //! program, every run: the random draws probe novelty, and the prefix
 //! leg makes every kernel × size-decade region the corpus reaches an
 //! enforced verdict rather than a sampled one (a region a random case
-//! draws with probability `q` slips a 48-case run at `(1 − q)^48`; the
+//! draws with probability `q` slips an `n`-case run at `(1 − q)^n`; the
 //! prefix leg reads it red deterministically).
 //!
 //! Standing self-checks ride along: the meter's liveness (`ff_nop`), the
```

<!-- annotation -->
> **proptest-cases** (109), line 21:
>
> The module doc's luck bound multiplied by the literal count; restated for an n-case run so it holds at any count.

<a id="hunk-3"></a>
### crates/before/fuzzfit/harness/tests/enforce.rs `@@ -32,8 +32,8 @@`

```diff
@@ -32,8 +32,8 @@
 //! at-scale rejection arms — never rides on the sentry's rare escalation
 //! draws alone).
 //!
-//! Case count: 48 by default (the calibration corpus is the big sweep; this
-//! is the sentry); override with `PROPTEST_CASES`.
+//! The sentry runs proptest's default case count, raised with
+//! `PROPTEST_CASES`; the calibration corpus is the big sweep.
 
 use std::collections::BTreeMap;
 
```

<!-- annotation -->
> **proptest-cases** (109), line 35:
>
> The module doc had stated the fixed count and called PROPTEST_CASES an override (true at this site: the proptest! macro re-reads the environment over the supplied config). It now says the sentry runs proptest's default, raised by PROPTEST_CASES, and keeps the one reason a reader needs: the calibration corpus is the big sweep.

<a id="hunk-4"></a>
### crates/before/fuzzfit/harness/tests/enforce.rs `@@ -335,10 +335,9 @@ fn building_toolchain_matches_the_pin() {`

```diff
@@ -335,10 +335,9 @@ fn building_toolchain_matches_the_pin() {
 /// the random draws keep probing novel shapes, while this leg makes
 /// every kernel × size-decade region the deterministic corpus reaches
 /// an enforced verdict rather than a sampled one. A region of per-case
-/// draw measure `q` survives a 48-case sentry at `(1 − q)^48` — about
-/// 38% per gate at `q ≈ 2%`, the measure of a kernel × decade region —
-/// so an out-of-band region could pass consecutive gates on luck; under
-/// this leg the same region reads red deterministically. The programs
+/// draw measure `q` survives an `n`-case sentry at `(1 − q)^n`, never
+/// zero, so an out-of-band region can pass consecutive gates on luck;
+/// under this leg the same region reads red deterministically. The programs
 /// already execute for the refit, so the leg's cost is the verdict,
 /// not the runtime.
 ///
```

<!-- annotation -->
> **proptest-cases** (109), line 338:
>
> The prefix-totality doc worked the luck bound at 48 cases (38% at q of 2%); the worked number goes and the bound stays in per-case terms, since the argument for the deterministic leg is that the bound is never zero, not its value at one count.

<a id="hunk-5"></a>
### crates/before/fuzzfit/harness/tests/enforce.rs `@@ -400,14 +399,14 @@ fn the_deterministic_prefix_is_judged_total_and_matches_the_pin() {`

```diff
@@ -400,14 +399,14 @@ fn the_deterministic_prefix_is_judged_total_and_matches_the_pin() {
 /// slope, deterministically.
 ///
 /// The sentry's random draws pick the escalation family about once in 137
-/// cases, so a 48-case run usually never leaves the small-operand regime —
-/// and an instrument whose deep reach is exercised only by rare draws has
-/// no standing proof its at-scale bands (the seven single-operand rows,
-/// the rejection arms, the deep-overlap scans) still bite. This replay is
-/// that proof: one escalation program at fixed depth and seed, judged on
-/// both legs like any sentry case. (The cross-universe rejection arms
-/// need no fixed replay: the sentry's family roster draws the independent
-/// regime nearly three times per default run.)
+/// cases, so a run at the default case count can leave the small-operand
+/// regime untouched — and an instrument whose deep reach rides on rare
+/// draws has no standing proof its at-scale bands (the seven single-operand
+/// rows, the rejection arms, the deep-overlap scans) still bite. This
+/// replay is that proof: one escalation program at fixed depth and seed,
+/// judged on both legs like any sentry case. (The cross-universe rejection
+/// arms need no fixed replay: the sentry's family roster draws the
+/// independent regime at a shape family's full weight, eight cases in 137.)
 #[test]
 fn the_escalated_regime_stays_in_the_pinned_bands() {
     let (depth, seed) = ESCALATION_REPLAYS[0];
```

<!-- annotation -->
> **proptest-cases** (109), line 401:
>
> The escalation-replay doc argued from a 48-case run (usually never leaves the small-operand regime; independent regime nearly three times per run). Restated per case from any_family's weights (one in 137; eight in 137), which is what the code fixes; the replay's justification (deep reach must not ride on random draws) is count-independent.

<a id="hunk-6"></a>
### crates/before/fuzzfit/harness/tests/enforce.rs `@@ -422,8 +421,8 @@ fn the_escalated_regime_stays_in_the_pinned_bands() {`

```diff
@@ -422,8 +421,8 @@ fn the_escalated_regime_stays_in_the_pinned_bands() {
 /// its band and every trend under its slope, deterministically.
 ///
 /// The depth-1024 replay alone would leave the family's upper depth range
-/// (1025..=1792) riding on sentry draws that arrive about once in five
-/// runs, and would hang the whole deterministic reach proof on a single
+/// (1025..=1792) riding on sentry draws that land there about once in 274
+/// cases, and would hang the whole deterministic reach proof on a single
 /// (depth, seed) point. This replay pins the other end of the reach: the
 /// deepest constructible spine, a different seed, the same judgment.
 #[test]
```

<!-- annotation -->
> **proptest-cases** (109), line 424:
>
> The depth-cap replay doc said the upper depth range arrives about once in five runs, arithmetic over 48 cases. Restated per case: the upper half of the escalation range is half of one in 137, about one case in 274.

<a id="hunk-7"></a>
### crates/before/fuzzfit/harness/tests/enforce.rs `@@ -436,8 +435,6 @@ fn the_escalation_depth_cap_stays_in_the_pinned_bands() {`

```diff
@@ -436,8 +435,6 @@ fn the_escalation_depth_cap_stays_in_the_pinned_bands() {
 }
 
 proptest! {
-    #![proptest_config(ProptestConfig::with_cases(48))]
-
     /// Every public operation stays inside its pinned fuel band on
     /// shapes nobody chose, and no band key's within-case cost trend
     /// out-climbs its pinned slope.
```

<!-- annotation -->
> **proptest-cases** (109), line 437:
>
> The sentry ran a fixed 48 cases; it now runs proptest's default and reads PROPTEST_CASES. This is the one site whose stated reason (the sentry is not the big sweep) the default defeats in run time: see the lane report for the before/after timing so Finch can rule.

<a id="hunk-8"></a>
### crates/before/fuzzfit/harness/tests/sanity.rs `@@ -11,8 +11,6 @@ use fuzzfit_harness::ops::{Mirror, Op};`

```diff
@@ -11,8 +11,6 @@ use fuzzfit_harness::ops::{Mirror, Op};
 use fuzzfit_harness::strategies::{any_family, budget_for, build};
 
 proptest! {
-    #![proptest_config(ProptestConfig::with_cases(64))]
-
     /// Every generated program respects its family's budget: op count,
     /// total ticks, total forks, and fold width never exceed
     /// [`budget_for`]'s caps, whatever the dimensions drawn.
```

<!-- annotation -->
> **proptest-cases** (109), line 13:
>
> The generator sanity block ran a fixed 64 cases; it now runs proptest's default and reads PROPTEST_CASES. The three tests took well under a second at 64, so the default costs nothing worth noting.

<a id="hunk-9"></a>
### crates/before/src/testing/semantic_oracle/tests.rs `@@ -551,14 +551,7 @@ fn worked_value_anchor_convicts_the_mirrored_embedding() {`

```diff
@@ -551,14 +551,7 @@ fn worked_value_anchor_convicts_the_mirrored_embedding() {
 fn grid_cap_is_never_reached() {
     use proptest::test_runner::{Config, TestRunner};
     use std::sync::atomic::{AtomicU32, Ordering as AOrd};
-    // Fewer cases than a typical canary sweep: each case probes the function
-    // space's grid to read its resolution, and the bound it guards is
-    // structural (`fork` only deepens by bisecting an indivisible piece, the
-    // paper's rate), so a modest sweep is an ample canary.
-    let mut runner = TestRunner::new(Config {
-        cases: 400,
-        ..Config::default()
-    });
+    let mut runner = TestRunner::new(Config::default());
     let max_d = AtomicU32::new(0);
     runner
         .run(&(world_strategy(), any::<u64>()), |(ops, seed)| {
```

<!-- annotation -->
> **proptest-cases** (109), line 554:
>
> The grid-cap canary built its runner with cases: 400 over Config::default(), which discarded PROPTEST_CASES (Config::default() already carries it; the struct update overrode the field). It now runs proptest's default and reads the variable. The inline comment justifying the count is deleted rather than rewritten: the test's doc comment already states that the sweep is the canary and the structural derivation is the guarantee.

