<!-- CAVEAT LECTOR: written by Claude (Opus 5.5) as the cataloguer for the builders', fixers', demonstrators', and reviewers' instruments, from their scratch directories, the branches they left, and the audit's records; no box run; for Finch's review. -->

# 09. Instruments built by builders, fixers, demonstrators, and reviewers

This section catalogues every instrument the audit's downstream roles
built: the 40 builder, 9 fixer, 8 demonstrator, and 45 reviewer scratch
directories that existed when I began, the surveyor's directory, and the
three probe files copied under `lanes/`. Three more directories appeared
while I worked, for agents still running; section F says what they hold so
far. The lane auditors' explore branches belong to sections 01 to 08.

Most of what these roles built is already committed on a ready branch,
because building tests was their job. What remains is the scaffolding
around that work: probes run once to settle a question, mutant schemas
that calibrated a committed test, constructed worst cases, and a few
finished tests that a scope ruling or a superseded caution cut.
Section A gives full entries for those. Section B lists, one line each, the
instruments a ready branch or `main` carries. Section C lists the log
parsers and drivers. Sections D and E say what holds no instrument and what
I could not assess, and section F covers the directories still in flight.

## Summary

**Counts.** 36 full entries in section A. Entry A36 is a grouped
entry: it holds 24 calibration sets whose target tests are carried, one
row each. Section B lists 60 carried instruments, and section C has 30
lines covering the parsers and drivers.

**Top three, by coverage added for the work of folding in.**

1. **A1, the exhaustive agreement test for the laws' multiplicity
   comparison.** It is written, takes 0.42 seconds, and closes a gap the
   reviewer measured: a wrong comparison passes #81's committed property
   at 4,096 cases. The coordinator declined it under the caution your
   correction (notice 88) later narrowed.
2. **A2, the trait-impl laws that #83 narrowed out.** They sit on #83's
   own history (`d653bb83`) in registry form. They check `Count` and
   `Rank` addition in every spelling, every `OwnVersion` comparison cell,
   and the shape walks' size hints, all of which survived as constant
   bodies at the base.
3. **A3, the touch-bound worst-case families.** Fixed programs built from
   #37's own step kinds reach 0.58 of the derived bound, where #37's
   random programs reach 0.17. They run in 0.07 seconds.

**Five facts the owner should know before ranking.**

- *These instruments are fragile.* Everything under the session
  scratchpad lives in `/private/tmp` and does not survive a reboot (the
  README says so of lane records). Four dropped commits survive only in
  the reflog, reachable from no branch (verified, `git branch --contains`):
  `789936bc`, `d0a6205f`, `60d534ed`, and `9123a4d8`. If you want any
  entry kept for later, it needs copying into `.agent-notes/` first. I
  copied nothing, per the brief.
- *Production-code mutants have no committed home.* `main` keeps
  deliberately wrong *reference implementations* for its oracles and
  descriptors (`KNOWN_BAD_VERSION_PAIR`, the function-space known-bad
  Riemann sum; verified). Every mutant set below instead edits
  production source, by string swap or by an environment switch compiled
  into production code. Folding one in means choosing a home first:
  - a scripted harness outside the test suite;
  - test-only copies of the mutated function;
  - the surveyor's compile-time `option_env!` switch (entry A19), which
    compiles to nothing when unset but still adds source lines to
    production files.
- *One survey record needs correcting.* Survey section 2.2 says no
  wasm32-versus-native differential over suanpan traces exists and that
  random small traces cannot reach the 32-bit defects. The stability-width
  fixer built such a family (entry A14). It varies the *query width*
  rather than the value's width, and it failed 498 of 4,800 cases at the
  defect's base.
- *Two ready entries record declines under the superseded caution.* #81
  declined A1 "per your caution", and #83's narrowing removed A2's laws
  under the coordinator's scope correction. Under notice 88 both are
  already-built instruments.
- *`main`'s `min_ticks` still exceeds the board's heap ceiling off the
  board's sizes.* The fixer's probe (entry A4) shows 177 of 756 readings
  above `1,024 + 20` bytes per input byte, on code identical to `main`'s
  (verified, log). Question 87 stopped the fix. Entries A4, A8, A9, and
  A15 all belong to that open problem, and the write-up in
  `.agent-notes/2026-10-08-min-ticks-transient-heap/` (section 06, "Instruments that must exist
  first") names three of them.

**Base and marks.** `main` at `ce67ab083`. I measured reach against
`00-baseline.md` (the integrator's census at that commit). Every claim is
*verified* (I checked it against a file, a log, or a branch), *reported*
(a record says so, and I name the record), or *inferred*. `<scratchpad>`
is the session scratchpad,
`/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/`.

**Runs.** None. Every runtime below comes from a recorded log, or is
marked unmeasured. The one measurement I considered was entry A15's
91 MB input on `main`, which no record holds. I did not run it, because
it would investigate the stopped `min_ticks` defect rather than describe
an instrument.

**How I ordered and grouped.** Entries run from the most coverage per
unit of fold-in work to the least. Several directories built
the same kind of instrument; I merged those into one entry where they share
a fold-in path (fuel comparison, A5; stack-recursion mutants, A17; heap
size sweeps, A25; retired-implementation differentials, A33). Calibration
sets whose target tests are already carried, and whose results the ready
entries already report, are one grouped entry (A36) with a row each. I
chose that over 24 full entries because their fold-in path is identical
and their evidence is already in `QUESTIONS.md`. Tell me if you want them
expanded.

---

## A. Full entries

### A1. Exhaustive agreement test for the laws' multiplicity comparison

- **What it is.** A finished unit test, `same_multiplicity_agrees_with_the_tree_oracle_on_small_families`, with its two constants and generator: `<scratchpad>/reviewer-registry-laws/new-test.rs`, 64 lines, and `repair-exhaustive-agreement.diff` against `audit/registry-multiplicity-laws` at `e1782c17` (verified, files).
- **What it reaches or checks.** Every family of at most three nonempty normal-form parties of depth at most 2, repeats included. That is 3,616 families: there are 15 such parties (inferred from the construction). It groups the families by the laws' `owner_layers`. Each family must agree with its group's first member under both the laws' `same_multiplicity`, which #81 builds only from public `without` and `join`, and the tree oracle's `same_multiplicity`. The first members of any two groups must differ under both. The oracle is the recursive tree model, independent of the laws' layer construction (verified, source).
- **Coverage beyond the committed suite.** #81's agreement property checks one weaker comparison per constructed family. A comparison that requires equal total measure and an equal most-owned layer passes it at 256 and 4,096 cases. The counterexample is `[A∪B∪C, A∪B, A]` against `[A∪B∪C, A∪C, A]` for disjoint quarters (reported, reviewer NOTES run 4; the test's doc states the same case). The exhaustive test fails that comparison and every other comparison mutant the reviewer tried (union only, count, top layer) (reported, run 5). Nothing on `main` or #81 enumerates families.
- **Evidence.** Unmutated: passes in 0.42 seconds (reported, run 5). With the repair applied: testdoc, `clippy -D warnings`, and 3 of 3 conservation tests pass (reported, run 6). #81's ready entry records the offer and its decline (verified).
- **Fold-in cost.** Apply one diff to #81's tests module. No dependency, doc comment written to the standard, 0.42 seconds against the 300-second limit.
- **Overlaps.** It stands beside #81's agreement property as a second, independent check: exhaustive where the property samples.
- **Dependencies.** #81, which stacks on #40.
- **Value, in one sentence.** For half a second it closes a measured gap in the comparison every multiplicity law relies on, and its text is ready.

### A2. Trait-impl laws that #83 narrowed out

- **What it is.** Law-registry entries and `count` tests in commit `d653bb83`, "Check before's public trait impls against their documented behavior", which is on `audit/trait-coherence-and-hole-subtracts` (#83) and is reverted there by `735b6b2e` (verified, `git log` and `git branch --contains`). Against #83's tip, the commit holds 294 lines the tip deletes (verified, `git diff --stat`).
- **What it reaches or checks.** The laws absent at #83's tip (verified, diff):
  - `addition_spellings_match_the_naturals`: `Count`'s four `+` cells, `+=`, and `Sum`, owned and borrowed;
  - `rank_add_spellings_agree`;
  - a `cmp_cell_agrees` helper driving every owned and borrowed `OwnVersion` comparison cell through every operator;
  - `{version,party,clock,combined}_shape_size_hint_brackets_remaining`;
  - `ranked_debug_shows_name_and_version`;
  - `atom_debug_is_its_query_debug`, for `Floor` and `Ceiling`;
  - `borrowed_query_converts_to_a_copy`;
  - the `Version`-receiver span operator cells, and the `Clock | Version` and `|=` cells.

  The oracle is the canonical spelling: production checked against itself, metamorphically.
- **Coverage beyond the committed suite.** `main`'s `Count` addition test drives only `&a + &b` and an owned `Sum` (verified, `count/tests.rs`). The commit message says that replacing these trait bodies with constants passed every suite at the base (reported, `d653bb83`'s message). Its examples are `Count`'s borrowed `+` and `Sum`, the shape walks' `size_hint`, and an argument-order slip in `OwnVersion`'s borrowed `gt`, which answers the converse. Survey section 2.3 leaves 41 such survivors (class T) uncovered "by decision".
- **Evidence.** The builder's 20 mutants (`<scratchpad>/builder-trait-coherence/mutants.py`):
  - `Hash` bodies as `()`;
  - `Count`'s `Debug`, `Add<&Count>`, and `Sum<&Count>` as defaults;
  - `Ranked` and `Floor` `Debug`;
  - constant `size_hint`s;
  - a `Clock |=` that forgets to absorb;
  - the `OwnVersion` slip;
  - a span cell that drops its span;
  - a `From<&Query>` that drops holes;
  - `Rank`'s `Add` returning zero.

  Run A applied ten of them at once, one per law group. 13 of the 28 tests it ran failed: the two `count` tests and eleven law drivers (verified, `mutA.log`). I did not attribute each failure to its mutant.
- **Fold-in cost.** Low: the laws are written in registry form, so restoring them means reverting `735b6b2e`'s hunks for the laws you want. Runtime is unmeasured; they ride the existing law drivers. #96's item 8 proposes squashing #83 into two commits at landing, which would remove `d653bb83` from history.
- **Overlaps.** #83 keeps the `Hash`, `Debug`-is-`Display`, and `ClockForks` hint laws. The reviewer found partial duplicates: `tests/forks_count.rs` checks exact `PartyForks` hints for `k` up to 256, and the rank snapshot row checks `Debug` against `Display` (reported, reviewer NOTES).
- **Dependencies.** #83.
- **Value, in one sentence.** It restores checks on the operator and trait spellings the adequacy campaign showed survive as constant bodies, by reverting part of one commit.

### A3. Touch-bound worst-case families

- **What it is.** A probe module of five tests written against #37's `touch_bound.rs`: `<scratchpad>/builder-touch-bound/probe.rs.saved`, 368 lines (verified).
- **What it reaches or checks.** Deterministic programs measured against #37's bound, touches ≤ `K·work + D`:
  - F1, the comparison fixed point at `2·2^2048`;
  - F2 and F2′, stream subtractions that leave a long cancellation, then one scan, ascending and descending, at bit offsets 0, 1, 16, 17, and 31, with 2, 6, or 64 limbs;
  - F3, a loaded ripple;
  - F4, activation churn through owned take, reset, and word activation;
  - F5, gap stops;
  - F6, clone-and-scan;
  - F7, normalize churn.

  The largest utilization of the bound is 0.5813: the descending stream-scan at offset 31 with 64 limbs, 11,397 touches over 1,303 units of work. F2′ reaches 0.5085 (verified, `long1.log` and `families1.log`).
- **Coverage beyond the committed suite.** #37's property draws random swarm-weighted programs (`00-baseline.md` section 7). The builder's 20,000 random cases peaked at utilization 0.17, and its own hill-climbing search found at most 5.58 touches per unit of work above the constant term (reported, builder NOTES milestone 3). None of these families is in #37's committed file (verified, its function names). A uniform rise in touches of about 1.7 times would fail these families, while random programs would need about 5.9 times (inferred from the two utilizations).
- **Evidence.** It caught no defect; it measures reach. The reviewer reproduced 0.5813 (reported, reviewer NOTES E5).
- **Fold-in cost.** Low. The programs are `Vec<Step>` values built from #37's own step kinds and judged by #37's `check_program` (verified on the branch). Runtime 0.07 seconds (verified, logs). No new dependency.
- **Overlaps.** The suanpan lane's hill-climbing adversary (ignored, best ratio 9.6; reported, survey 2.1), which section 07 catalogues. These families would stand beside #37's random programs as fixed worst cases.
- **Dependencies.** #37.
- **Value, in one sentence.** Fixed worst cases at 0.58 of the bound make #37's property fail near its derivation, where random programs leave a margin of about six times.

### A4. `min_ticks` heap probe over right spines, with a closed-form value oracle

- **What it is.** A probe module of six tests in the `before` lib test binary, with `PeakAlloc` installed as its global allocator: `<scratchpad>/fixer-min-ticks-heap/fixer_probe.rs` (207 lines) and `fixer_probe_test.rs` (62 lines) (verified).
- **What it reaches or checks.** Right spines built from explicit leaf-height sequences:
  - rising, `a_k = k`;
  - jump-entered, `a_0 = 0`, `a_k = 2^b + k`, for `b` from 16 to 300;
  - wide-step, `a_k = k·2^s`, for `s` from 30 to 300;
  - zigzag and rising zigzag across `2^s`;
  - the board's own jump-rising-spine family.

  Sizes straddle powers of two: 2, 3, 5, 9, 17, 33, 65, 100, 129, then `2^k − 2`, `2^k + 2`, and `1.5·2^k + 2` for `k` from 8 to 16, up to 98,306 levels. That is 756 readings in all. Each reading checks `min_ticks` against the closed form Σ leaves − Σ suffix minima, which shares nothing with production or the tree oracle, and compares the peak heap with the board's ceiling of `1,024 + 20` bytes per input byte (verified, source).
- **Coverage beyond the committed suite.** The board samples each family at four ladder sizes and one small size (`00-baseline.md` section 5.1), never deliberately at the sizes just past a power of two where doubling storage is emptiest. `main` has no jump-entered family (verified, `git grep`). At the parent of the stopped fix, 177 of 756 readings exceed the board's ceiling, and all 756 values match the closed form (verified, `parent-probe-worst.log`). That parent's `min_ticks` code is `main`'s, since the demonstration commit changes none of it (reported, survey 1.1). Among the readings over the ceiling:
  - plain rising spines, 10 readings, for example 39.4 bytes per input byte at 65,538 levels;
  - wide steps of `2^64` and `2^65`, 5 readings each;
  - jump-entered spines up to 307.67 bytes per input byte (`2^287` at 65,538 levels).

  The fixer called the plain and wide-step regimes "a red family at the parent beyond D1's description" (reported, fixer NOTES).
- **Evidence.** It caught the defect question 87 stopped, plus regimes beyond its record. It also measured each design the fixer built (reported, fixer NOTES rounds 1 to 9).
- **Fold-in cost.** High, for three reasons:
  - At `main` it fails by design, so it can land only beside a fix, or as an ignored test.
  - It installs a global allocator in the lib test binary, which would wrap every lib test. The demonstrator listed this as an obstacle (reported, demonstrator NOTES).
  - It builds spines through the recursive oracle and the bridge, on a 2 GiB stack (`run_in_big_stack`; verified). A committed version would build them with the meter's iterative `BitsWriter` construction, as entry A15's `stepped_spine` does.

  Runtime 0.3 to 14.2 seconds per test (verified, log). `peak_alloc` is already a dev-dependency.
- **Overlaps.** A8 (the family on the board), A9 (limb work on combs), A15 (the same regime at 91 MB). The write-up's items 2 and 3.
- **Dependencies.** Question 87; it is on no branch.
- **Value, in one sentence.** It is the widest measurement of `main`'s open `min_ticks` heap defect, with a value oracle independent of every implementation, and any future fix must clear it.

### A5. Fuel comparison between two commits

- **What it is.** Four methods, built in eight directories, that compare wasm fuel at two commits (verified, files):
  - Refits: `just fuzzfit-calibrate` at each commit, diffed band by band (`<scratchpad>/builder-one-sweep/bands-*.rs`, `fixer-wasm32-growth/fuel-*-bands.rs`).
  - Fitted-line deltas: `reviewer-one-sweep/fueldelta.py` (percent change along each band) and `surveyor/bands_drift.py` (each band's ceiling at its largest size, in `log₁₀`).
  - Per-sample dumps: `builder-rank-single-alignment/dump_rank_fuel.rs`, a harness binary over `fuzzfit_harness::drive::for_each_deterministic_program(4096, …)`, with the reviewer's copy and `fuelcmp.py`.
  - Fuelscape dumps: `fuelscape --dump` at both commits, diffed as JSON (`fixer-stability-width/fuel-cmp.log`, 14 operations moved).
- **What it reaches or checks.** Every deterministic fuzz-fit program, or every fuelscape sample, at fuel resolution.
- **Coverage beyond the committed suite.** The bands admit a sample up to `10^0.2`, about 1.58 times, above its fit (`00-baseline.md` section 5.3). #95 compares two board runs exactly, but only heap, scan, and touch. Nothing compares fuel between commits. These tools found:
  - the one-sweep simplification raising `join` by 0.88% down to 0.23% across sizes, and `meet_all` by 1.15% to 0.71% (reported, builder NOTES; the reviewer recomputed them with `fueldelta.py`);
  - the first rank-alignment version raising all 5,143 `ff_rank_add` samples by 45 to 75 fuel each, which the reviewer turned into a fall of −27 to +5 (verified, #63's entry);
  - the fallible-growth design raising `party_decode` 1.30 times, remedied with a cold path (reported, fixer NOTES).

  None of these moves fails a band.
- **Evidence.** Above. Fuel is the same on the Mac's guest and the box's guest in all 72 ladder cells, even though the binaries differ (reported, reviewer-fuel-ladder run D), so comparisons transfer between hosts.
- **Fold-in cost.** Moderate. A fuzz-fit harness mode modeled on #95's `capture` and `compare` would dump per-sample fuel and classify the differences. The dump loop already exists. It is a developer tool, not a test, so it has no per-test runtime; a calibration run takes minutes (unmeasured).
- **Overlaps.** #95, for board currencies. #52's ladder pins 72 cells within ±2% but compares no commits.
- **Dependencies.** None; #95 for a shared output format.
- **Value, in one sentence.** It makes every constant-factor fuel change visible and attributable per commit, which four branches needed and each rebuilt by hand.

### A6. Ceiling-rule auditor

- **What it is.** Python scripts that parse an acceptance board log and the shard child's exact cell counters: `<scratchpad>/builder-ceiling-rules/audit.py` (146 lines), `onerule.py` (61), `exact.py` and `exact2.py` (23 each), and `precheck.py` (19) (verified).
- **What it reaches or checks.** For each measured ceiling constant, it groups the board rows by the ceiling their model names. It then recomputes the rule, `ceil(1.25 × max((reading − intercept) / units))`, with the intercept 1,024 for heap and 0 for scan and touch. Rendered readings are rounded to one decimal, so it brackets each value, prefers exact counters where it has them, and flags any rule the brackets leave unresolved. `precheck.py` asks whether a lowered ceiling would still pass every row.
- **Coverage beyond the committed suite.** Nothing committed compares a ceiling with its stated rule (survey 1.4). Under question 65's one-rule ruling, `audit/board-ceiling-one-rule` (`b1e71248`) restates every rule in prose and moves the values, but adds no check that recomputes them (verified, its diff). The auditor found the drifts survey 1.4 reports: COMB at 2.98 against a doc saying 2.0, and QUERY at 152 against a rule giving 109.
- **Evidence.** It derived every value in the one-rule commit (reported, builder NOTES, verified against the commit message). The reviewer found one fault: `audit.py`'s grouping mislabels ceilings that share a value (COMB as DESER at 4.0) (reported, reviewer NOTES).
- **Fold-in cost.** Moderate. Survey 1.4's form puts the rule's inputs beside each constant and recomputes them in `run_acceptance` from readings the run already holds, in Rust. The Python is a reference for that arithmetic, not code to port. It needs no extra sweep.
- **Overlaps.** Section 07 and the ranker may list survey 1.4 itself; this entry is the existing prototype.
- **Dependencies.** Slot 26's one-rule branch (plan item 1, not yet ready), which stacks on #53.
- **Value, in one sentence.** It turns the one-rule ruling from prose into arithmetic that already exists, so ceiling drift in either direction becomes a failure.

### A7. libtest's main thread inside a heap measurement window

- **What it is.** A throwaway integration test with its own global allocator, written with `unsafe impl GlobalAlloc`: `<scratchpad>/builder-peakalloc-single-thread/zz_libtest_race_probe.rs`, 89 lines (verified).
- **What it reaches or checks.** The allocator delays every allocation by libtest's main thread by 2 milliseconds, so the test thread reaches its baseline first. The test then asserts that no main-thread allocation lands in a 500-millisecond window.
- **Coverage beyond the committed suite.** It is the only reproduction of the race behind the intermittent `representation_space` failure (4,729 against 4,609 bytes; survey 1.6). On libtest's default path, four main-thread allocations land in the window (608, 48, 24, and 96 bytes). With `RUST_TEST_THREADS=1`, two still land (24 and 96, which total the recorded failure's 120) (verified, `probe-concurrent.log` and `probe-single.log`). That refuted survey 1.6's proposed fix.
- **Evidence.** It decided question 91's premise.
- **Fold-in cost.** Low once a counting allocator exists. Question 91 ruled for a shared counting-allocator crate with `unsafe` allowed in tests, and slot 47's builder plans to use this probe as its calibration, failing before and passing after (reported, builder NOTES; STATE plan item 6). Runtime 0.54 seconds (verified, log).
- **Overlaps.** None committed.
- **Dependencies.** Slot 47, `audit/shared-counting-allocator`, in progress.
- **Value, in one sentence.** It is the only known-bad demonstration for the heap meters' race, and the planned allocator's acceptance test.

### A8. The jump-entered rising spine as a board family

- **What it is.** A meter registry shape, a board family `jump-rising-spine` (JR(b, d)), the property `jump_rising_spine_decodes_canonically_as_a_rising_spine`, and 50 worst-case re-pins. All are in commit `08573e159`, which is on `fix/before-min-ticks-heap` and every archive branch (verified). The patch is also saved as `<scratchpad>/demonstrator-min-ticks-heap/jump-rising-spine-family.patch`.
- **What it reaches or checks.** A right spine of `d` internal nodes whose leaves are 0, then `2^b + 1` to `2^b + d`, sampled at `b = 40` and sized like the harmonic spine. Each nested subtree's minimum sits one tick above its parent's, so a range-minimum fold holds one suspended boundary per level, every one beyond `i32` (verified, commit message).
- **Coverage beyond the committed suite.** `main` has no such family (verified). It fails `main`'s board by design: `version_min_ticks × jump-rising-spine` reads 124.9, 125.0, and 177.2 bytes per input byte at the base, top, and small scales, against the heap ceiling (verified, commit message). It also becomes the worst touch case for the rank-fold rows, beating `harmonic` by 1.28 to 1.32 times (verified, commit message; survey 2.2 reconstructs the same from the demonstrator's notes).
- **Evidence.** It is D1's demonstration.
- **Fold-in cost.** It lands with a `min_ticks` fix, as the write-up's item 2 says; at `main` it fails acceptance. It adds a board column and re-pins 50 rows (verified, commit message).
- **Overlaps.** A4 measures the same shape at off-board sizes.
- **Dependencies.** Question 87.
- **Value, in one sentence.** It is the only board family that reproduces `main`'s `min_ticks` heap defect, and the strongest rank-fold touch case found.

### A9. Limb work of `min_ticks` on wide-offset combs

- **What it is.** A probe of two tests with a limb counter fed by temporary hooks in production code, plus `PeakAlloc`: `<scratchpad>/reviewer-min-ticks-final/reviewer_probe.rs`, 130 lines, and `hook-main-min_ticks.diff` (verified). Survey 1.1 describes it.
- **What it reaches or checks.** Two left combs of `k` teeth under an outer minimum whose offset is about `2^(64k)`. In the cross-prefix comb the outer minimum sits at prefix 0 and the comb at the next prefix; in the same-prefix comb they share a prefix. `k` runs from 250 to 2,000. A third test runs `wide_arming` at four sizes. The probe reports limbs read, peak heap, and the answer's bits.
- **Coverage beyond the committed suite.** No board currency counts `num-bigint` work outside suanpan's accumulator (`00-baseline.md` section 5.1). On the stopped design the limb count grows quadratically: 125,751 to 8,006,001 limbs for a 7.99-fold input. At the parent, which is `main`'s code, it is one limb per tooth: 251 to 2,001 (verified, `run1-branch.log` and `run2-parent.log`; survey 1.1 tabulates both). At `main` it therefore measures a currently linear quantity.
- **Evidence.** It caught the stopped design's quadratic time, which every board counter missed (reported, question 87).
- **Fold-in cost.** Survey 1.1's proposal: add the two comb shapes as `min_ticks` families to #52's fuel ladder, so wasm fuel sees `num-bigint` work with no hook. The probe's own approach needs hooks at call sites, and only the `mark` hook for `main` was saved; the `count_limbs` placement on `main` is known only from notes (reported, write-up 07). Its helpers (`ev_leaf`, `ev_leaf_wide`, `wide_arming`, `Encoding`) exist on `main` (verified). Runtime 0.93 seconds for both tests in a debug build (verified, log).
- **Overlaps.** Write-up item 1 names exactly this instrument.
- **Dependencies.** #52, and question 87 for when the work resumes.
- **Value, in one sentence.** It guards any future `min_ticks` redesign against the quadratic `num-bigint` work no current meter sees.

### A10. Memo reuse property for consecutive pre-scans

- **What it is.** A candidate proptest, `consecutive_scans_read_back_only_their_own_differences`, for `version/tick/memo/tests.rs`, inside `<scratchpad>/reviewer-events-cogen/experiment.patch` (verified).
- **What it reaches or checks.** Several pre-scans run in sequence and share one memo. Every slot must read back exactly the difference its own scan wrote, whatever the write order, so a block that an earlier, longer scan filled carries nothing into a later one.
- **Coverage beyond the committed suite.** Neither #74 nor `main` has it (verified, `git grep`). #74 catches the same mutants (M15: begin a scan at 1; M21: clear only the last used block) through its multi-scan strategy, end to end (reported, #74's entry). This property states the memo's own contract, one unit below.
- **Evidence.** It kills M15 and M21 (reported, reviewer NOTES run 1). Unmutated, it passes in 1.3 to 2.0 seconds (verified, `run1.log`).
- **Fold-in cost.** Low: one property in a file #74 creates.
- **Overlaps.** #74's multi-scan strategy. It would stand beside it as an independent unit-level check.
- **Dependencies.** #74.
- **Value, in one sentence.** It states the memo's reuse contract directly, so a memo defect fails where it lives instead of through a tick differential.

### A11. Linear-memory reading after each wasm32 pin case

- **What it is.** Three harness probes that read the guest's memory size after each check: `<scratchpad>/builder-rank-sum-pin/zz_scratch_probe.rs` (41 lines; it also reads a temporary panic flag that became #31's record), `builder-rank-single-alignment/probe-lib.rs` (87 lines; it prints after every check in `run_raw`), and `reviewer-rank-sum-pin/zz_reviewer_probe.rs` (verified).
- **What it reaches or checks.** Pages of 64 KiB held after each case. `RankArithmetic` cases 1 to 6 read 50,207; 50,207; 42,014; 58,399; 58,399; and 50,207 pages, which is 3.06 to 3.56 GiB of the 32-bit guest's 4 GiB (verified, `builder-rank-sum-pin` logs). The reviewer measured case 7 at 50,207 (reported).
- **Coverage beyond the committed suite.** No committed instrument measures memory on a 32-bit target (`00-baseline.md` section 6). At 58,399 pages a pin has 7,137 pages, 446 MiB, of headroom (reported, reviewer NOTES). A regression of that size would turn a pin into an allocation abort, which #31 reports but nothing anticipates.
- **Evidence.** #63 cites it: pin memory is unchanged (verified, #63's entry).
- **Fold-in cost.** Low. The harness reads `memory.size` after each call and could assert a ceiling per check; the reading is deterministic. No new dependency.
- **Overlaps.** #31.
- **Dependencies.** None; #57 for the time limits.
- **Value, in one sentence.** It would give warning before a wasm32 pin runs out of address space and starts testing the allocator instead of its target.

### A12. Mutant schema for `Rank::decode`'s reader paths

- **What it is.** A replacement body for `Rank::decode` that selects a mutant from the environment variable `RANK_MUTANT`, with its apply script and two run drivers: `<scratchpad>/reviewer-rank-decode-reader/decode_schema.rs` (118 lines), `apply_schema.py`, and `run1.remote.sh` and `run2.remote.sh` (verified).
- **What it reaches or checks.** 20 mutants and a control:
  - the adequacy lane's 11 surviving mutants: every `Interrupted` guard at four read sites, and the prefix's end-of-input check;
  - two more guard variants;
  - five decoders that swallow a reader error before the verdict is settled (50 to 54);
  - one that swallows it after (55);
  - one correct early-stopping decoder (56), an equivalence control that must pass.
- **Coverage beyond the committed suite.** #78 carries the reviewer's repaired property and deterministic test (verified, #78's entry). The schema is the evidence that they work, and nothing committed reproduces it. Against #78's first property, mutant 55 passed three fresh seeds and 4,096 cases. Against the repaired property it fails 3 of 3 (68 of 500 single-case runs). Mutant 56 passes both, which shows the oracle does not over-constrain (reported, reviewer NOTES runs 1 and 2).
- **Evidence.** Above. It also measured single-case kill rates on the first property: mutant 21 in 67 of 1,500 cases, mutant 2 in 29 of 500, mutant 52 in 20 of 500 (reported).
- **Fold-in cost.** Moderate. A committed form needs a home that leaves production `rank.rs` untouched: a test-only copy of the decoder, about 100 lines, driven by #78's `check_decode` with the decoder passed in. Its runtime would match #78's property, 0.56 seconds (reported, builder NOTES).
- **Overlaps.** #78's tests, which it calibrates. The builder's `calibrate.py` ran the 11 survivors alone (section A36).
- **Dependencies.** #78, which stacks on #53.
- **Value, in one sentence.** It is the only artifact showing #78's oracle both rejects the permissive window and accepts a correct early-stopping decoder.

### A13. Detection model and swarm close bound for the range-minima property

- **What it is.** A Python model of the range-minima kernel and #82's generator, `<scratchpad>/reviewer-range-minima-boundaries/sim.py` and `simdrive.py`, and a property variant with a swarm close bound, `swarm-block.rs` (verified).
- **What it reaches or checks.** The model estimates, without a build, how often a generated case reaches each mutant's trigger. The variant draws each case's close bound from {3, 5} instead of a fixed range.
- **Coverage beyond the committed suite.** Under the boundary mutant at line 106, #82's property fails 16 of 20 seeds and the swarm variant 20 of 20 (verified, `run3.log`). Both pass 20 unmutated seeds (reported, NOTES run 2). #82 did not adopt the swarm bound (inferred from its diff). The model's estimate of 86% per run matched the builder's measured 80% (reported). A cargo-mutants run over the range-minima files, filtered to the range-minima tests, found 7 mutants only #82's property kills, and 26 that those tests miss (reported, `run1-kills.txt`).
- **Evidence.** Above.
- **Fold-in cost.** Low for the variant: one strategy change in #82's property. The model is a review aid.
- **Overlaps.** #82.
- **Dependencies.** #82.
- **Value, in one sentence.** A one-line strategy change lifts #82's detection of its own target mutant from 16 of 20 seeds to 20 of 20.

### A14. wasm32 differential over accumulator histories at the stability-width boundary

- **What it is.** A guest check pair and a harness test: `<scratchpad>/fixer-stability-width/family_guest.rs` (67 lines) and `family_harness.rs` (42 lines). They need two scratch protocol variants, `ScratchStabilityFamily` and `ScratchStabilityCoverage` (verified, files; reported, fixer NOTES).
- **What it reaches or checks.** 600 pseudo-random accumulator histories. Each has up to 12 updates, at shifts below 24 digits, and one update in three is a cancelling pair, so uncompacted tops are common. Each history is queried at eight widths on both sides of `FIRST_UNINDEXABLE_STABILITY_WIDTH`: 0, 64, 736, 832, the boundary minus one, the boundary, the boundary plus 31, and `u64::MAX`. The query's answer and its stored form, digit count, sign, and magnitude must equal what `cmp_zero` leaves on a clone, inside the 32-bit guest. A coverage census counts how many histories have anything to compact: 166 of 600 (verified, log).
- **Coverage beyond the committed suite.** #50 commits one pin at one constructed value (reported, #50's entry). No committed wasm32 test varies histories. Survey 2.2 says such a differential did not exist and that random small traces cannot reach the 32-bit defects. This one reaches the defect by varying the query width instead of the value's width.
- **Evidence.** At the defect's base, 498 of 4,800 cases fail, all at unindexable widths (166 compactable histories × 3 widths). After the fix, 0 of 4,800 fail (verified, `family-base.log`; reported, fixer NOTES). The reviewer, by reasoning and without a run, judged that the oracle would bless the truncating-widening mutant `adjustment_high as usize as u64`, which returns the true sign. The reviewer recommended committing a strengthened pin instead, and #50 did so (reported, reviewer NOTES, item 2).
- **Fold-in cost.** Moderate. It needs two `Check` variants (one line each after #58's derive), two guest functions, and a harness test. It ran past 60 seconds (verified, nextest's slow flag); the reviewer reports 70 seconds. The wasm32-pins workspace allows 20 minutes under #57.
- **Overlaps.** #50's pin, which catches the truncating mutant this family may not.
- **Dependencies.** #58 for the variant numbering; #50.
- **Value, in one sentence.** It is the only wasm32 test over varied histories, though by the reviewer's analysis its oracle is weaker than #50's pin against the defect it was built for.

### A15. Real-scale stepped spine for `min_ticks`

- **What it is.** An integration-test probe and a scratch module: `<scratchpad>/reviewer-min-ticks/reviewer_probe.final.rs` (123 lines) and `scratch-final.patch` (333 lines). The patch adds `stepped_spine(step_bits, wide, unit)`, phase counters, and exports of the stopped branch's narrowed bound (verified).
- **What it reaches or checks.** A spine of `wide` levels, each `2^step_bits` above the last, then `unit` levels of one tick each. The ignored real-scale test builds `wide = 2^23 + 16` levels at `2^31`, then `2^25` unit levels: a 91,226,247-byte input. Smaller variants run the wide-step spine at 600 to 153,600 levels and the jump-entered spine at 8,000 to 512,000 levels.
- **Coverage beyond the committed suite.** No committed input reaches 2^23 wide levels; board families run about 1 to 35 KiB (`00-baseline.md` section 5.1). On the re-anchoring design (`38202ca6`) the real-scale input peaked at 6,135,222,240 bytes, 67.25 per input byte, against a ceiling of 1,824,525,964; the wide prefix alone stayed under at 15.34 (verified, `run1-release.log`). `main` has never run it (reported, write-up 07). `stepped_spine` uses only `BitsWriter`, `ev_leaf`, `ev_leaf_wide`, and `Encoding`, all on `main` (verified), so it ports without the stopped branch. The phase-attribution test and the `inline` variants depend on that branch's internals.
- **Evidence.** It showed that the first fix design failed at a scale the board cannot reach (reported, reviewer NOTES; write-up 03 and 07).
- **Fold-in cost.** It cannot join `just gate`: it needs gigabytes of heap and a run of unmeasured length. The write-up's item 5 proposes an on-demand developer check outside `just gate`. Porting `stepped_spine` into the meter's construction language is small.
- **Overlaps.** A4 covers the same regime at board-sized inputs.
- **Dependencies.** Question 87.
- **Value, in one sentence.** It is the only real-scale probe of `min_ticks`, the one the write-up asks for before any redesign is trusted.

### A16. Mutant schema for suanpan's readout carry classes

- **What it is.** An environment switch in suanpan's `read.rs` that perturbs exactly one readout class: `<scratchpad>/builder-readout-classes/swap.py` (69 lines), `remote-calibrate.sh`, and the results in `schema-table.txt`. The reviewer added four more swaps (`reviewer-readout-classes/swap.py`) (verified).
- **What it reaches or checks.** `READOUT_MUTANT=kind:carry:low_zero`, for the touch and value kinds over the 11 reachable classes of final carry and zero or nonzero low part: 22 mutants. Two literal swaps from the brief, and the reviewer's low-first, low-last, carry-once, and parity-touch variants, come with it.
- **Coverage beyond the committed suite.** 12 of the 22 fail only #64's table, and the rest of suanpan's suite passes under them: 10 of the 11 touch mutants and two value mutants (verified, `schema-table.txt`). The other value mutants each fail between 2 and 40 other tests.
- **Evidence.** All 22 fail #64's table (verified).
- **Fold-in cost.** Same as A12: a production switch with no committed home.
- **Overlaps.** #64, which it calibrates.
- **Dependencies.** #64, which stacks on #27.
- **Value, in one sentence.** It shows class by class that #64's table is the only test of the readout's touch cost.

### A17. Stack-recursion mutants and frame-size probes for the deep-input tests

- **What it is.** Four mutant generators and two frame probes (verified, files):
  - `builder-deep-identity/mutants.py`: M26, M26-lean, M26-stored, MC-place-drop;
  - `builder-deep-surfaces/mut/make_mutants.py`: M1, M2, M3;
  - `reviewer-deep-identity/mutate.py`: a `REVIEW_MUTANT` schema adding PAMT, and a swap setting the depth to 100,000;
  - `reviewer-deep-surfaces/mut/apply_shape.py`: cells, cells-tail, regions, overlay;
  - `reviewer-deep-surfaces/probe.rs` and `probe2.rs`: compiled frame sizes on illumos.
- **What it reaches or checks.** Each mutant rewrites one production loop as a recursion: party removal's descent, placement's post-drop continuation, the hull sweep, plateau gathering, a right-recursive descent, `min_ticks`'s leaf fold (PAMT), and the `Cells`, `Regions`, and `Overlay` shape walks.
- **Coverage beyond the committed suite.** It shows which deep test is the sole detector of each recursion (reported, the four NOTES files):

  | Mutant | Caught by |
  |---|---|
  | M26, M26-lean, cells | the target deep test and five `amp_board_smoke` tests |
  | M26-stored | only `deep_identity_stack_safety` (#75) |
  | MC-place-drop | only `deep_tree_remaining_surfaces_stack_safety` (#75) |
  | M1, M2, regions, overlay | only #34's new test |
  | M3 | escapes #34's first round; caught after its second |
  | PAMT | only `deep_tree_min_ticks_stack_safety`, at depth `2^18`; it passes at 100,000 |
  | cells-tail | nothing: the optimizer compiles it to a loop |

  The frame probes found that illumos x86_64 keeps frame pointers, so a walker holding one value across its call costs 32 bytes per level and overflows at 100,000. A source-level `1 + acc(n − 1)` compiles to a loop and survives 10^8 levels (reported).
- **Evidence.** PAMT is the direct evidence for #75's choice of `2^18` over 100,000.
- **Fold-in cost.** High. These are production rewrites with no committed home (see the summary), and stack overflow aborts the process, so each mutant needs its own run under `ulimit -c 0`. Question 68 removes `descend!`, `recurse.rs`, and `stacker` after #40, #72, and #74 land. The deep tests stay, so this set remains their only calibration (inferred).
- **Overlaps.** #34 and #75's tests, which they calibrate.
- **Dependencies.** #34, #75.
- **Value, in one sentence.** It proves that each deep-input test detects recursion in the walk it names, and it records why the depth is `2^18`.

### A18. Mutants of suanpan's written-position set

- **What it is.** A script applying eight string swaps to `digits/written.rs` on W (#86), restoring the seed file between runs: `<scratchpad>/reviewer-written-positions/mutants.py`, 43 lines (verified).
- **What it reaches or checks.** Each conversion and truncation path of the prefix-to-mask-to-bitset set:
  - a dropped position on copy;
  - a shift on promotion, from the short prefix and from the mask;
  - a dropped top position in conversion;
  - truncation keeping one position too many, in the prefix, the mask, and the bitset;
  - a mask absorbing only the last position of a span.
- **Coverage beyond the committed suite.** All eight fail #86's `BTreeSet` model test, `written_positions_match_an_ordered_set` (verified, `run1.log`). The cases needed before the first failure, one run each, were 2, 11, 0, 0, 139, 129, 0, and 86 (reported, NOTES). The two prefix and mask truncation mutants need about 130 of the default 256 cases, so a single run may miss them (inferred).
- **Evidence.** Above. #86's entry reports the result (verified).
- **Fold-in cost.** Same as A12. Separately, the late kills suggest the model test's generator rarely reaches truncation just above the top. That would be a generator change, not a fold-in.
- **Overlaps.** #86's model test.
- **Dependencies.** #86, which stacks on #52.
- **Value, in one sentence.** It is #86's calibration, and its kill depths show the truncation mutants sit near the edge of one run's reach.

### A19. Compile-time mutant switches for cost-only survivors in the fuel bands

- **What it is.** A scaffold and driver: `<scratchpad>/surveyor/scaffold.py` (50 lines), `scaffold.diff` (75 lines), and `survey-fuel.sh` (verified). The scaffold adds `const fn survey_mutant(name)`, which reads `option_env!("SURVEY_MUTANT")` at compile time and guards four mutants (verified, source):
  - `packed_u64.rs:63`: every pop decodes its width bit by bit;
  - `writer.rs:363`: `splice_storage` copies whole bytes bit by bit;
  - `words.rs:78`, twice: the reader skips, or mishandles, its optional buffer top-up.
- **What it reaches or checks.** For each variant, the driver rebuilds the fuzz-fit guest and harness, confirms by hash that the guest changed, and runs all 25 fuzz-fit tests, band enforcement included.
- **Coverage beyond the committed suite.** It is the record behind survey 1.9: three cost-only survivors pass every committed instrument, fuel bands included (all 25 pass). The splice survivor fails 3 of 25, ABOVE BAND on `ff_clock_fork` (verified, NOTES; survey run 1).
- **Evidence.** Above.
- **Fold-in cost.** The pattern is the notable part: a switch that compiles to nothing when unset, so the unmutated build carries no extra instructions (verified, its design; survey run 1's base passed 25 of 25). It still puts `survey_mutant` calls in production source. Survey 1.9 proposes a ladder cell for the pop path instead.
- **Overlaps.** Section 08's survivor index (the adequacy lane's).
- **Dependencies.** #52 for the ladder alternative.
- **Value, in one sentence.** It is the only demonstration that three cost-only mutants escape every committed meter, and a template for zero-cost mutant switches if you want mutants committed.

### A20. Serde round trips through third-party formats

- **What it is.** A standalone probe crate, `<scratchpad>/reviewer-serde-framing/probe/`: `main.rs` (118 lines) and a `Cargo.toml` depending on `serde_json`, `csv` 1.3, `rmp-serde` 1.3, `serde_bencode` 0.2, `quick-xml` 0.37, `ron` 0.8, `serde_yaml` 0.9, and `toml` 0.8 (verified).
- **What it reaches or checks.** Round trips of `before`'s human-readable composites (`Clock`, `Span`, `Ranked`) through csv with and without headers, MessagePack in named and tuple forms, bencode, XML, RON, YAML, TOML, and JSON, plus JSON edge cases: 33 format-by-type pairs (reported, reviewer NOTES).
- **Coverage beyond the committed suite.** #51's leniency properties drive serde's own test deserializers (verified, #51's entry). Third-party formats behave differently, and only this probe exercises them. csv with headers feeds field names as bytes, and csv without headers and MessagePack's tuple form send sequences. Under the strict design the probe found csv-with-headers broken for all three types, which made byte keys a blocking finding (reported). After you ruled for leniency, the reviewer reran its edge cases against the rebuilt branch.
- **Evidence.** Above. It is a review tool; nothing records it as run against #51's final tip.
- **Fold-in cost.** High in dependencies: eight new dev-dependencies, each entering the supply-chain audit. A detached workspace like `surfacecheck` would keep them out of `before`'s tree. Runtime is negligible (inferred).
- **Overlaps.** #51.
- **Dependencies.** #51.
- **Value, in one sentence.** It is the only check that the serde contract holds against the deserializers applications use, at the cost of eight dependencies.

### A21. Board-cell bisection and an allocator event log

- **What it is.** Two diagnostics (verified, files):
  - The bisection driver: `<scratchpad>/reviewer-ceiling-rules/step.sh`, `measure.sh`, and `comb.py`. It checks out any commit, derives that commit's operation position and features from source, and reads one board cell exactly through the shard child mode.
  - The event log: `patch_alloc.py` and `exp.sh`. They wrap `PeakAlloc` in the board binary with a logging allocator (`unsafe impl GlobalAlloc`) and record every event of 4 KiB or more inside one cell's measurement window.
- **What it reaches or checks.** Exact cell readings across history, and the allocation sequence behind a reading.
- **Coverage beyond the committed suite.** #95 says whether two runs differ and how. These say at which commit, and why. They traced COMB's rise from 2.0176 to 2.9823 bytes per I/O byte to `63d01d903`, and its mechanism to the split payload stream growing in rungs of `250·2^k` (reported, reviewer NOTES, verified against the one-rule commit's message).
- **Evidence.** Above.
- **Fold-in cost.** The event log needs `unsafe` in the board example. Question 91's counting-allocator crate could expose event hooks instead (inferred). The bisection driver is a script; it could become a justfile recipe.
- **Overlaps.** #95, A6.
- **Dependencies.** Question 91's crate, for the log.
- **Value, in one sentence.** It turns an unexplained heap movement into a commit and a mechanism, which every heap stop in this audit had to rebuild by hand.

### A22. Counterexamples behind `Clock::from_parts`'s warning

- **What it is.** Three scratch tests, `<scratchpad>/reviewer-docs-branch/zz_reviewer_sketch.rs` (verified).
- **What it reaches or checks.**
  - `from_parts` over a version lacking one of the party's events, with a receive in between, yields a version that is unissued and dominated.
  - The same holds with a version held from the clock's own history.
  - The two-party grid's off-grid example lies inside the interval and off the grid, and the grid is closed under join and meet.
- **Coverage beyond the committed suite.** `tests/stale_state.rs` calls `from_parts` over an earlier version valid and says it reproduces the successor (reported, reviewer NOTES). These cases show where "reproduces issued stamps" fails, which #85's warning and question 84's crate-page rule (slot 45) now state.
- **Evidence.** All three pass (reported, run 1).
- **Fold-in cost.** Low: `tests/stale_state.rs`.
- **Overlaps.** `tests/stale_state.rs`, #85.
- **Dependencies.** #85, slot 45.
- **Value, in one sentence.** It keeps the `from_parts` warning true, with cases that already exist.

### A23. Gamma window on long codes at unaligned starts

- **What it is.** A test copied under `lanes/l8-adequacy/round-2/addendum-bits-party/witness/window_witness.rs` (verified). The adequacy lane built it, so section 08 may also list it.
- **What it reaches or checks.** Codes of 59 to 63 bits at every start offset from 1 to 7, 42 cases, through `BitsReader::gamma_from_window`.
- **Coverage beyond the committed suite.** It kills three `window.rs:47` survivors in `load`'s ninth-byte merge. No production caller reaches them today, because borsh's reader buffers at most seven bits past the position (reported, `follow-ups.md`).
- **Evidence.** Above.
- **Fold-in cost.** Low: `follow-ups.md` calls it a ready-made regression test.
- **Overlaps.** `gamma_window_edge` checks a 63-bit code only at position 0.
- **Dependencies.** None.
- **Value, in one sentence.** It guards the window's general contract for the first caller whose buffer is fuller than borsh's.

### A24. Guest panic records under memory exhaustion

- **What it is.** Three reversible guest patches and a built guest: `<scratchpad>/reviewer-trap-diagnosis/probe.patch`, `probe2.patch`, `probe3.patch`, and `wasm/guest.wasm` (verified).
- **What it reaches or checks.** What #31's panic record captures in each case:
  - `expect` with memory exhausted: no message;
  - a literal `panic!`: a message;
  - a formatted `panic!`: none;
  - deep recursion: a stack-overflow trap with no message;
  - `process::abort`: an unreachable trap with no message;
  - with memory full, `Option::unwrap` records its message and `Result::unwrap` does not.
- **Coverage beyond the committed suite.** #31 commits its pins' expected messages and one control that exhausts memory and must trap with no message (verified, #31's entry). Its entry cites these probes as the evidence for every documented behavior of the record, and none of them is committed (verified, the entry's "Reviewer's probes"). They show that the rule "no message means no panic reached the hook" can fail spuriously, which the reviewer's prose corrections now state.
- **Evidence.** Above.
- **Fold-in cost.** Moderate: one guest case each, after #58.
- **Overlaps.** #31.
- **Dependencies.** #31, #58.
- **Value, in one sentence.** It would pin down what the wasm32 failure channel can and cannot tell you, which #31's readers will otherwise rediscover.

### A25. Heap size sweeps for single operations

- **What it is.** Three probes (verified, files):
  - `<scratchpad>/builder-clone-free-dedup/zz_builder_probe.rs`: peak heap of `Sum` and `join_all` over versions decoded one at a time as the fold pulls them, at widths 64 to 4,096;
  - `reviewer-clone-free-dedup/zz_reviewer_probe.rs`: scan and heap for duplicate-run folds, the adapter's lookahead, and the span folds' merged arms;
  - `builder-writer-retention/zz_scratch_forks_probe.rs`: `party_forks_full` peak heap on a joined half, at sizes from 64 to 65,536, doubling.
- **What it reaches or checks.** Lazily decoded owned fold inputs, and sizes between the board's samples.
- **Coverage beyond the committed suite.** The reviewer's probe found the lookahead filter raising lazy owned folds' peak heap by 126, 650, and 2,748 bytes at widths 256, 1,024, and 4,096, a regression no board row showed (reported, reviewer NOTES; you kept the lookahead under question 69). The forks probe found a heap cliff at 2,048 shares that the board's fixed sizes straddle (reported, builder NOTES).
- **Evidence.** Above.
- **Fold-in cost.** A board row with lazily decoded inputs would carry the first two; the third needs only more sizes. Runtime unmeasured.
- **Overlaps.** Slot 23's committed scan meter, `tests/meter/duplicate_runs.rs`, covers the filter's scan cost but not this heap reading (verified, file on the branch).
- **Dependencies.** Slot 23 (plan item 2) and slot 33 (plan item 3), neither ready.
- **Value, in one sentence.** They measure heap where folds decode as they go and where sizes fall between board samples, two regimes the board does not sample.

### A26. Fork-plan tails at every depth to 140

- **What it is.** Three scratch proptests at 2,048 cases, `<scratchpad>/fixer-fork-hint/scratch_unit_tests.rs` (verified, file; names verified in `focused.log`).
- **What it reaches or checks.** Every power-of-two plan of depth 0 to 140, positioned 0 to 40 shares before its end. It must report an exact `size_hint` at every step, yield exactly the remaining shares, and end on the rightmost leaf, and the borrowing `PartyForks` must conserve the party.
- **Coverage beyond the committed suite.** #43 checks the same hints at four fixed depths: `usize::BITS + 1`, `+ 2`, 128, and 130 (verified, #43's diff). This family covers every depth, and also checks the last share's identity. It positions the plan with the helper `position_at_remaining`, which writes the remainder directly. The reviewer showed that bypasses the plan's own stepping, and #43's third round replaced the helper with the plan's own steps (`skip_shares`) (reported, reviewer NOTES).
- **Evidence.** It passed on the fix, with no seeded failure recorded (reported, fixer NOTES); runtimes 0.2 to 0.9 seconds (verified, log).
- **Fold-in cost.** Low, after porting it to `skip_shares`.
- **Overlaps.** #43's fixed-depth tests, which it would broaden.
- **Dependencies.** #43.
- **Value, in one sentence.** It widens #43's four depths to every depth up to 140, cheaply, once it steps the plan itself.

### A27. Shifted-zero checks through every operator spelling

- **What it is.** A scratch test, `<scratchpad>/fixer-zero-shift/scratch_entry_points.rs`, and its wasm32 counterpart (verified, file; reported, NOTES).
- **What it reaches or checks.** A cancelled zero stored across digits `2^16 − 1` and `2^16`, applied to a fresh receiver through borrowed `+=` and `-=`, owned `-=` and `-`, borrowed `+` and `-`, `Sum` over references, owned `+`, and `<<=` by the zero's own width. The result must be zero, with at most 2 stored and 2 retained digits.
- **Coverage beyond the committed suite.** #48's properties drive `add_shifted`, `sub_shifted`, `<<=`, and `<<` (verified, branch). This probe drives the seven operator spellings, which the fixer found all route through one function (reported).
- **Evidence.** All pass, natively and in six wasm32 combinations (reported).
- **Fold-in cost.** Low: one test in `representation.rs`.
- **Overlaps.** #48.
- **Dependencies.** #48.
- **Value, in one sentence.** It checks that every operator spelling keeps #48's guarantee, which today holds only because they share one route.

### A28. Probe of the integral's dense-span bound

- **What it is.** A patch, `<scratchpad>/reviewer-small-cleanups/probe.patch`, that records the integrator's common depth `S` in a thread-local and asserts `span ≤ S/32 + 2` at every `DensePart::new` across the whole `before` suite (verified).
- **What it reaches or checks.** The premise behind #72's `checked_mul` proof.
- **Coverage beyond the committed suite.** #72 states the bound in its `expect` message (reported, #72's entry). The probe saw 119 new maxima and no violation. Its largest span, 2,063 at `S = 66,000`, sits one below the bound of 2,064, so the derivation is tight (verified, `probe-run.log`).
- **Evidence.** Above.
- **Fold-in cost.** Moderate: a `debug_assert` would need `S` threaded to `DensePart::new`.
- **Overlaps.** #72.
- **Dependencies.** #72.
- **Value, in one sentence.** It shows #72's derived bound is attained within one, so any off-by-one in its derivation would fail.

### A29. Compare pin grown past bit `2^32`

- **What it is.** A reversible patch script, `<scratchpad>/builder-wasm32-compare-pin-reach/probe_apply.sh` (64 lines), adding a synthesized pair, a guest branch, and a harness test (verified).
- **What it reaches or checks.** `A = node(leaf(2^w − 1), leaf(2^w))` against `B = leaf(2^w − 1)`, with `w = 2^31`. A's deciding right-leaf flag sits at bit `2^32 + 3`, and the inputs encode `2^32 + 7` and `2^32 + 2` bits. It also traps if any reader opens at a position past `2^32`.
- **Coverage beyond the committed suite.** The committed compare pin has no live bit past `2^32` (survey 1.2). This probe's decision rests on such a bit. It did not catch the narrowing it was built for: it passed with the cursor narrowing applied, because comparison never opens a reader at a large position (reported, builder NOTES run 3: passed in 36.1 seconds). #92 corrected the pin's doc instead.
- **Evidence.** It established that path one of survey 1.2 cannot work.
- **Fold-in cost.** A guest case; about 1 GiB of guest memory for the two inputs (inferred from their sizes); 36 seconds.
- **Overlaps.** #92.
- **Dependencies.** #58.
- **Value, in one sentence.** It reaches a comparison decided past bit `2^32` on 32-bit, though no narrowing on that path has been constructed that it would catch.

### A30. Rank alignment cases on each side of a `2^32` gap

- **What it is.** Six temporary guest cases and a native edge probe inside the reviewer's scaffold, `<scratchpad>/reviewer-rank-single-alignment/scaffold-full.patch` (verified).
- **What it reaches or checks.** Small ranks at exponents 9, 8, and 7 against a deep rank at exponent `2^32 + 8`, giving gaps of `2^32 − 1`, `2^32`, and `2^32 + 1`, through both addition (cases 5 to 7) and subtraction (cases 8 to 10). The native probe covers a grid of zero, equal exponents, and the 32- and 64-bit digit boundaries, checking `+` against the `Sum` fold and `checked_sub`'s inverse identities.
- **Coverage beyond the committed suite.** Little. #63's pin already checks addition and subtraction at gaps of `usize::MAX` and `usize::MAX + 1` bits, which are `2^32 − 1` and `2^32` on wasm32 (verified, its pins diff). These cases add only the gap `2^32 + 1`, and reach the boundary from a different construction (small operands at exponents 7 to 9).
- **Evidence.** All pass on the chosen variant. The gap-narrowing mutant N traps in the committed case 4 and in temporary case 9 (reported, reviewer NOTES run 1), so the committed cases already catch it.
- **Fold-in cost.** Low in code: cases in the existing check, no new variant. Runtime is a concern. Each case holds a numerator of about `2^32` bits, and the existing rank pin already takes 80 to 375 seconds under load (reported, logs), against the detached workspace's 20 minutes under #57.
- **Overlaps.** Mostly duplicates #63's pin; #29.
- **Dependencies.** #63, #28.
- **Value, in one sentence.** It extends #63's two gaps by one more, a margin the committed cases already cover against the only mutant constructed.

### A31. Scan for self-recursive test helpers

- **What it is.** A regex scan, `<scratchpad>/builder-small-cleanups/selfrec.py`, 19 lines (verified).
- **What it reaches or checks.** Functions in `tests.rs`, `testing/`, and `oracles/` whose bodies call their own name, marking those guarded by `descend!`.
- **Coverage beyond the committed suite.** None committed. It found "dozens" of unguarded test recursions, which led to question 68 (reported, builder NOTES).
- **Evidence.** Above.
- **Fold-in cost.** Low as a script. Question 68 removes `descend!`, so the guarded distinction disappears. Pointed at library code instead, it would check `crates/before/AGENTS.md`'s rule that no library traversal recurses on tree depth (inferred); as written, it only sees direct self-calls.
- **Overlaps.** #34 and #75's deep tests check the same rule at runtime, for the walks they drive.
- **Dependencies.** Question 68.
- **Value, in one sentence.** Retargeted at library code, it would turn a hard rule that only review enforces into a mechanical check, though only for direct recursion.

### A32. Interrupt behavior of the board recipe, on a toy justfile

- **What it is.** A toy justfile and three Python drivers that send SIGINT or SIGTERM to the process group: `<scratchpad>/reviewer-worst-case-pin-always/toy/justfile`, `sigint.py`, `sig.py`, and `sigint-gate.py`, with the builder's copies (verified).
- **What it reaches or checks.** The old dependency form, the new bash form, the trapped forms, and a background stream like `gate-streams`.
- **Coverage beyond the committed suite.** Without a trap, Ctrl-C during acceptance started the pin; with `trap 'exit 130' INT` it stopped (reported, reviewer NOTES; the trap is on #93's justfile, verified). Nothing committed tests recipe signal handling.
- **Evidence.** Above.
- **Fold-in cost.** No harness for justfile recipes exists (inferred).
- **Overlaps.** #93.
- **Dependencies.** #93.
- **Value, in one sentence.** It is regression evidence for one recipe's interrupt handling, with no committed home.

### A33. Differentials against replaced implementations

- **What it is.** Three reviewer harnesses that compared new code with the code it replaced (verified, files):
  - `<scratchpad>/reviewer-one-sweep/old_sweep_diff.rs`: the one-sweep lattice against the old two-copy sweep;
  - `reviewer-refine-partial/query_reviewer_probe.rs`: `refine_partial` against the old endpoint-clamp formula, on raw stored forms beyond the normalized ones, with a release-mode check of the walk's `Partial` certification;
  - `reviewer-sync-all/chunk-a.rs` and `chunk-b.rs`: `sync_all` old against new.
- **What it reaches or checks.** For the sweep: all 477,481 pairs of a small exhaustive scope, 20,000 arbitrary pairs, and 20,000 organic histories (reported, NOTES). For `sync_all`: 8,000 families, with no divergence (reported, #76's entry).
- **Coverage beyond the committed suite.** At the moment of change, they were the strongest equivalence checks. Once a branch lands, the old code exists only in the test. The committed oracles (tree and function space) already check the semantics both versions implement.
- **Evidence.** The sweep's depth, relation, and sticky mutants each fail 4 of 4 comparing tests (reported). The refine probe found no divergence (reported).
- **Fold-in cost.** Each would carry a deleted implementation as a test-only reference. The sweep harness took 386 seconds for its five tests (reported), over the 300-second limit at those case counts.
- **Overlaps.** The tree and function-space oracles.
- **Dependencies.** #30, #61, #76.
- **Value, in one sentence.** They did their job at the change, and keeping them would mean keeping deleted implementations as oracles beside two committed ones.

### A34. 32-bit decoder-growth instruments dropped by your input-size ruling

- **What it is.**
  - Commit `60d534ed`, the demonstrator's past-limit pins (`demonstrator-wasm32-growth`): `RankDecodeGroups` and `VersionBorshWideLeaf`, with a `LazyStream` synthesizer and a streaming `Display` check. It is in the reflog only.
  - Commit `d0a6205f`, the fixer's fallible growth: `growth.rs`, five growth tests, and a `BitStack` `TryPush` model test. It is in the reflog only.
  - Commit `8208efae`, pins asserting a trap past the limit, on `fix/before-wasm32-buffer-growth`.
  - Probes: `<scratchpad>/fixer-wasm32-growth/probes.py` (cases S1 to S7) and `trapcause.py`, and the reviewer's `reviewer-wasm32-growth/probe-chain.diff`, `probe-slice.diff`, and growth mutants (`mutant.py`, `variant.py`).

  (Verified, reachability and files.)
- **What it reaches or checks.** Decoders at and past the 1 GiB doubling limit on 32-bit.
- **Coverage beyond the committed suite.** Behavior you ruled unpromised: neither crate promises any input size (verified, recorded in #53's entry). Two findings outlive the ruling:
  - Suanpan's digit doubling traps when a validator meets a dense height past `2^32` bits (reported, S7).
  - `std`'s `Chain::read_to_end` defeats a fallible decoder (reported, chain probe).
- **Evidence.** Above.
- **Fold-in cost.** Not applicable under the ruling. The `LazyStream` technique, which synthesizes a gigabyte input in constant memory, could serve future 32-bit pins (inferred).
- **Overlaps.** #53 kept the rank image change from this work.
- **Dependencies.** Your ruling.
- **Value, in one sentence.** Little while sizes carry no promise; the streaming synthesizer is the reusable part.

### A35. Instruments superseded by a ruling or a later design

- **The strict serde tests.** The demonstrator's `9123a4d8` (`demonstrator-serde-framing`; reflog only), and the fixer's round-2 strict property (`<scratchpad>/fixer-serde-framing/round2-uncommitted.patch`, plus a `git stash` in slot 04). You ruled for leniency, so they check the opposite of the contract.
- **The exhaustive protocol sweeps.** `789936bc` (reflog only) swept every `u32` and `i32` through the wasm32-pins decoders. #58's `FromRepr` derive makes a stale arm impossible by construction (verified, #58's diff), and a failure numbered `PASS` is a compile error (reported, builder NOTES).
- **The limb-index fixer's panic classifier** (`<scratchpad>/fixer-limb-index/probe.patch`): a panic-class export, replaced by #31's message record.
- **The reserve fixer's adapted suanpan probe** (`fixer-reserve-digits/explore_l7.rs.adapted`): the suanpan lane's explore probe widened to bit counts. Section 07 catalogues the original.

Value: none of these adds coverage at `main` plus the ready branches.

### A36. Calibration sets for carried tests

Each row is a set of deliberate defects that showed a committed or ready test fails, run once by string swap or environment switch and then reverted. The target test is carried, and its ready entry reports the outcome. They share one fold-in path: a committed home for production mutants, which does not exist (see the summary), at the cost of A12's entry. Value, as a group: they are the evidence behind each ready entry's calibration claims, and the only way to rerun them after the code changes.

| Directory | Target, carried on | Mutants | What they showed |
|---|---|---|---|
| `builder-rank-decode-reader` | #78 | the 11 reader survivors, fresh-seed pass | all 11 fail #78's property, also with the seed file moved aside |
| `reviewer-fork-hint` | #43 | `FORK_MUTANT` M1 to M6 | M4, M5, M6 survived #43's first version; the reviewer's test, now committed, kills all six |
| `fixer-fork-hint` | #43 | forks M0 to M6 | all killed at the final tip |
| `reviewer-normalize-bound`, `builder-normalize-bound` | #66 | `REVIEW_MUTANT` 10 mutants; 6 swaps; `domain_model.py` | M1 passes the suite without #66's test; two are equivalent; the domain model shows width 3 reaches every regime |
| `reviewer-touch-bound` | #37 | NOCOMPACT, NOCOMPACT_STABLE, SCANGAP, TRIMGAP, M14, FORGETNOOP, K24, K25, HALFREMNANT | three cost mutants escaped round 1; #37 added cancelled chains and now catches them |
| `builder-touch-bound` | #37 | M12, M14, M15, M16, and the scan mutants | single-case detection: M14 306 of 2,000, M15 424 of 2,000; M12 adds one touch and is not caught, since a constant fits the bound's constant term (inferred) |
| `builder-trait-coherence`, `reviewer-trait-coherence` | #83 | M1 to M20; A1 to A4, C1, C2 | `Debug` via `write!("{self}")` survived until #83 checked formatter flags; a length-free byte hash shows the `Hash` law is stronger than its doc |
| `builder-debug-asserts`, `reviewer-debug-asserts` | #38 | m1a to m3d; equal-value pool | m3b and m3d were caught only by a debug assert until #38's pool; per 20 seeds: arm 20, word 17, wide 19 |
| `reviewer-join-all-multiplicity`, `builder-join-all-multiplicity` | #40 | M19, M20, MH1, eight RH helper mutants, STRONG | the parity and area-plus-union helpers survived the first helper test; #40's repair kills both |
| `builder-registry-laws`, `reviewer-registry-laws` | #81 | M19, M20 on both law drivers; comparator mutants | #81's laws fail both; see A1 for the comparator that passes |
| `builder-check-roundtrip`, `reviewer-check-roundtrip` | #58 | A, B, C, F0, `mutCheck`; a 25-copy matrix from `gen.py` | an unlisted check and a failure numbered 0 passed the first sweep tests, which led to the derive |
| `fixer-serde-framing`, `reviewer-serde-framing` | #51 | `r3cal`, `r4cal`, `r3mut` whole-file variants | the default-field, ordered-keys, and unknown-field variants passed round 1 and fail #51's final properties |
| `builder-events-cogen`, `reviewer-events-cogen` | #74 | M6, M8, M15, M19, M21, HALVE, TAKE, CENSUS; the reviewer's `REVIEW_MUT` 6, 8, 15, 21, 22 | all caught in round 1; M19 was not rerun after round 2's palette change (verified, `calibrate2.sh`) |
| `builder-fuel-ladder`, `reviewer-fuel-ladder` | #52 | K1, B3, dead meter, scale down, insert panic, diluted K1 plus offset | a diluted mutant with a 0.981 offset passed 72 of 72 cells while growing 0.45 to 0.71 points per byte, so #52 added a flatness check |
| `fixer-zero-range-bitset` | #86 | 7 swaps, 3 mask swaps, three band known-bads | the double-insert variant became #86's stated known-bad |
| `fixer-zero-shift`, `reviewer-zero-shift` | #48 | slack mutants, M1 to M4 | the slack-2 gate passed the first suite (2,006 retained digits against a bound of 8); #48's accumulation property now fails it |
| `builder-sync-all`, `reviewer-sync-all` | #76 | M1 (not atomic), M2, M3 (alias before fold) | M3 reproduces the 6.3 to 6.4 heap rise exactly |
| `builder-entry-delegation`, `reviewer-entry-delegation` | #36 | three, then five injections; A and B | injection A was blessed until the reviewer's promoted-twin bound |
| `builder-clone-free-dedup` | slot 23 | stale address, right clones, filter never fires | the filter that never fires failed nothing until the scan meter |
| `builder-one-sweep` | #30 | relation, noswitch, depth | 6, 9, and 10 of 11 lattice tests fail |
| `builder-read-high-part`, `reviewer-read-high-part` | #27 | drop and repeat touch, value mutants | the drop-touch mutants passed everything until #27's pin; the board is blind to them |
| `builder-range-minima-boundaries` | #82 | lines 106 and 116; detection over 20 seeds | 106 fails 16 of 20 seeds, 116 fails 20 of 20 (see A13) |
| `reviewer-stability-width`, `fixer-stability-width` | #50 | truncating widening | it passed #50's first pin and fails the strengthened one |
| `builder-codec-cleanups`, `reviewer-codec-cleanups`, `builder-deposit-pins`, `reviewer-deposit-pins`, `builder-rank-single-alignment`, `reviewer-rank-single-alignment`, `builder-trap-diagnosis`, `builder-board-pin`, `reviewer-board-pin`, `builder-board-exact-compare`, `reviewer-board-exact-compare`, `builder-board-neutral-coverage`, `builder-worst-case-pin-always`, `builder-nextest-timeouts`, `fixer-reserve-digits`, `reviewer-reserve-bits`, `reviewer-limb-index` | #67, #54, #63, #31, `ddfabe4c`, #95, #94, #93, #57, #28, limb fix | one to five swaps each | each ready entry reports its own; two notable results: the padding-precedence mutant survives #67 by contract (survey 2.3), and a limb mutant that allocated on zero limbs passed the limb pin by aborting, which #31 closes |

---

## B. Instruments carried by a ready branch, a planned branch, or `main`

One line each, by directory: what the instrument is, and where it lives.
"Planned" means a branch the owner's rulings started, which is not yet
ready (STATE plan items). I verified each location from the ready entry or
the branch, except where marked.

| Directory | Instrument | Lives on |
|---|---|---|
| `builder-board-exact-compare` | exact capture and comparison of two board runs (`board/compare.rs`) | #95 |
| `builder-board-neutral-coverage` | the board row `query_coverage_neutral`; the revert of #61 that calibrated it | #94 |
| `builder-board-pin` | the `TARGET_DEPENDENT` marker and its two tests | `main` (`ddfabe4cb`) |
| `builder-ceiling-rules` | every measured ceiling restated by one rule | `audit/board-ceiling-one-rule`, planned (plan item 1) |
| `builder-check-roundtrip` | protocol decoders derived by `FromRepr`, and a const check that no failure is numbered `PASS` | #58 |
| `builder-clone-free-dedup` | the scan meter `tests/meter/duplicate_runs.rs`; the freed-address duplicate test | slot 23, planned (plan item 2) |
| `builder-comparison-fixed-point` | the write-back skip pin with its adjacency case; the touch floor | #89 |
| `builder-debug-asserts` | the range-minima property's equal-value pool | #38 |
| `builder-deep-identity` | `STACK_SAFETY_DEPTH = 2^18` and the deep tests for every walk-bearing entry point | #75 |
| `builder-deep-surfaces` | `deep_tree_shape_hull_and_fold_stack_safety`, `deep_right_spine_party` | #34 |
| `builder-deposit-pins` | suanpan landing cases 5 and 6 | #54 |
| `builder-docs-branch` | `drained_forks_stay_exhausted` | #85 |
| `builder-entry-delegation` | the lattice clone heap check `tests/meter/lattice_clones.rs` | #36 |
| `builder-events-cogen` | co-generated tick-pair strategies, census floors, two fixed cases | #74 |
| `builder-fuel-ladder` | the zero-range fuel ladder (72 cells) and its calibrator | #52 |
| `builder-join-all-multiplicity` | `tree::Party::same_multiplicity`; the `join_all` error-arm checks | #40 |
| `builder-nextest-time-limit` | the 300-second limit | `main` (`205199319`) |
| `builder-nextest-timeouts` | time limits for the four detached workspaces | #57 |
| `builder-normalize-bound` | `normalization_is_exact_across_boundary_digits` | #66 |
| `builder-range-minima-boundaries` | open-minimum step values; two public cases | #82 |
| `builder-rank-decode-reader` | the scripted-reader property | #78 |
| `builder-rank-single-alignment` | the renamed alignment pin | #63 |
| `builder-rank-sum-pin` | `rank_sum_straddles_the_usize_alignment_limit` | #29 |
| `builder-read-high-part` | the nonzero-high-part touch pin | #27 |
| `builder-readout-classes` | `readout_high_part_costs_one_touch_in_every_carry_class` | #64 |
| `builder-refine-partial` | the two-party coverage grid with `toward` | #61 |
| `builder-registry-laws` | the `*_conserves_multiplicity` laws and their comparison | #81 |
| `builder-small-cleanups` | the checked multiplication with its proof | #72 |
| `builder-sync-all` | `sync_all_agrees_with_join_all`; the late-overlap shape | #76 |
| `builder-touch-bound` | `touches_stay_linear_in_priced_work`; the remnant-dropping known-bad | #37 |
| `builder-trait-coherence` | the `Hash`, `Debug`-is-`Display`, and `ClockForks` hint laws | #83 |
| `builder-trap-diagnosis` | the guest panic record; the memory-exhaustion control | #31 |
| `builder-wasm32-compare-pin-reach` | the compare pin's corrected doc; the validation-index note | #92 |
| `builder-worst-case-pin-always` | `_gate-board` running both legs | #93 |
| `builder-writer-retention` | six result-retention tests | slot 33, planned (plan item 3; reported, STATE) |
| `builder-shared-counting-allocator` | a shared counting-allocator crate, in progress | slot 47, planned (plan item 6; reported, STATE) |
| `fixer-fork-hint` | the wide-count regression test; `stored_remainder_matches_the_plan_position` | #43 |
| `fixer-limb-index` | the `u128` limb counter, tested by pin case 4 | `main` |
| `fixer-reserve-digits` | `reserve_bits` witnesses; the `SuanpanReserve` pin; the surface generator's unsatisfiable arm | #28 |
| `fixer-serde-framing` | the leniency and field-text equivalence properties | #51 |
| `fixer-stability-width` | the stability-width pin with the value at digit 2; native witnesses | #50 |
| `fixer-wasm32-growth` | the rank decoder's single image | #53 |
| `fixer-zero-range-bitset` | the `BTreeSet` model of the written set; the normalize storage tests | #86 |
| `fixer-zero-shift` | the zero-shift properties; the `NearLimit` receiver | #48 |
| `demonstrator-fork-hint` | the wide-count hint regression test | #43 |
| `demonstrator-limb-index` | landing case 4 | `main` |
| `demonstrator-reserve-digits` | `reserve_digits_ignores_unsatisfiable_requests`, restated for bits | #28 |
| `demonstrator-stability-width` | the stability-width pin and witnesses | #50 |
| `demonstrator-zero-shift` | `shifting_zero_takes_constant_space_whatever_its_stored_form`; the zero-shift pin | #48 |
| `reviewer-rank-decode-reader` | the exact failure oracle and the deterministic interrupt test | #78 |
| `reviewer-written-positions` | `normalize_of_zero_releases_written_storage_sized_to_an_old_width` | #86 |
| `reviewer-join-all-multiplicity` | the strengthened helper test; the deterministic party test | #40 |
| `reviewer-zero-shift` | `zero_operands_at_the_buffer_top_never_accumulate_space` | #48 |
| `reviewer-touch-bound` | the finding that #37's generator never reached the sign scan, closed by its cancelled-chain step | #37 (reported, #37's entry) |
| `reviewer-rank-sum-pin` | case 7, `[one, deep, one]` | #29 |
| `reviewer-reserve-bits` | the allocator-refusal case `2^33 − 32` | #28 |
| `reviewer-refine-partial` | the extended coverage grid | #61 |
| `reviewer-fork-hint` | the real-step skip helper behind `stored_remainder_matches_the_plan_position` | #43 |
| `reviewer-readout-classes` | the class table's width and row repair | #64 |
| `reviewer-entry-delegation` | the promoted-twin bound in `lattice_clones` | #36 |

---

## C. Log parsers and run drivers

These read logs or run other tools; none checks anything itself. One line
each.

- `reviewer-written-positions/cmp_worst.py`: compares worst-case pin rows, parent against tip, with a worst-cases log.
- `builder-entry-delegation/capture.sh`: captures every board cell at three scales through the shard child mode, the ancestor of #95's `capture`.
- `builder-clone-free-dedup/capture.sh` and `celldiff.py`: the same capture, and a field-by-field cell diff.
- `builder-one-sweep/dump-readings.sh`: another variant of the capture.
- `reviewer-clone-free-dedup/rev-capture.sh` and `mydiff.py`: the reviewer's variant of the capture and diff.
- `builder-comparison-fixed-point/board_diff.py`: diffs two rendered boards.
- `builder-writer-retention/scripts/board_diff.py` and `classify.py`: diff rendered boards and classify heap moves.
- `fixer-wasm32-growth/boardcmp.py`: diffs rendered boards.
- `fixer-fork-hint/cmp_rows.py`: compares fork rows.
- `reviewer-fork-hint/heapdiff.py`: compares fork heap rows.
- `reviewer-board-exact-compare/heapexp.py`: extracts heap exponent movement.
- `fixer-min-ticks-heap/compare.py` and `py/celldiff.py`: compare probe and board readings.
- `builder-ceiling-rules/candidates.py` and `candidates2.py`, `reviewer-ceiling-rules/adjusted.py` and `rows.py`: helpers to A6.
- `builder-debug-asserts/extract.sh`: collects `MEASURED` lines from meter test output.
- `builder-nextest-timeouts/tab.py`: tabulates test times from logs.
- `reviewer-nextest-timeouts/dist.py` and `dist_all.py`: the same, by distribution.
- `reviewer-one-sweep/fueldelta.py`: percent fuel change along each band (part of A5).
- `reviewer-rank-single-alignment/fuelcmp.py`: compares per-sample fuel dumps (part of A5).
- `reviewer-stability-width/fuelcmp.py`: groups fuel deltas by step size (part of A5).
- `surveyor/bands_drift.py`: each band's ceiling at its largest size, committed against refit (part of A5).
- `builder-rank-single-alignment/measure.sh`: runs the board, a refit, and the rank pins at one commit.
- `builder-rank-single-alignment/calltree.py`: prints the call tree of a function in wasm text.
- `reviewer-board-exact-compare/prefix_sim.py`: simulates that a truncation property catches an optional-end-line parser in about 63% of runs.
- `reviewer-reserve-bits/malloc_probe.py`: shows `malloc(2^58)` and `malloc(2^62)` return null on the Mac and the box.
- `reviewer-read-high-part/arith.py`: exhausts the readout's high-part arithmetic at bases 2, 4, 8, and 16, checking the carry-bound proof.
- `surveyor/survey-docs.sh`: builds rustdoc for the two detached workspaces with warnings denied.
- `surveyor/survey-fuel.sh`: the driver for A19.
- `builder-events-cogen/calibrate.sh` and `calibrate2.sh`, `builder-sync-all/calibrate*.sh` and `controls.sh`, `builder-read-high-part/calibrate*.sh`, `reviewer-read-high-part/mutants.sh` and `boards.sh`: mutant-loop drivers for A36's sets.
- Every `swap.py`, `mutate.py`, `apply*.py`, and `run*.sh` in the scratch directories: reversible-edit and run drivers for the sets above.
- `builder-docs-branch/tools/contract.py`: rewrote documentation text across files; not a check.

---

## D. Directories that hold no instrument

- `builder-crate-page-identity-linearity`: documentation only.
- `builder-nextest-time-limit`: a configuration change, now on `main`.
- `reviewer-wasm32-compare-pin-reach`: repair texts only.
- `reviewer-rank-decode-light`: prose swaps (`repair.py`) for #53.
- `demonstrator-fork-hint`, `demonstrator-limb-index`, `demonstrator-reserve-digits`, `demonstrator-stability-width`, `demonstrator-zero-shift`: notes and logs. Their tests are in section B.

---

## E. What I could not assess

- **Probes deleted before anyone saved them.** Only their logs survive:
  - the codec reviewer's probes appended to `bits/tests.rs` and `writer/tests.rs`;
  - the comparison reviewer's heap probe (`reviewer-comparison-fixed-point`, appended to `representation.rs` and restored from `representation.rs.orig`);
  - the demonstrator's `scratch_jr` dev-profile probe;
  - the nextest builder's park-loop hang tests;
  - the demonstrator's precise-margin edit to the worst-case renderer.
- **Runtimes no log records.** A2's laws, A15's real-scale run, A20, and A25. I did not spend a box run on them.
- **A2's per-mutant attribution.** Run A applied ten mutants at once, and I did not tabulate which failure belongs to which mutant.
- **A14's blind spot.** The reviewer's claim that A14 would bless the truncating mutant is reasoning without a run.
- **`main` at A15's scale.** No record has run `main` on the 91 MB input. Doing so would investigate the stopped defect, not this catalogue.
- **The planned branches' instruments** (slots 23, 33, 45, 46, 47). They are unreviewed, so I list them in section B without judging them.

---

## F. Directories created while I catalogued

These belong to agents still running on the planned branches. Each holds
an instrument in progress; a later pass should catalogue its final state.
What follows is what each held when I last looked (reported, each NOTES
file).

- `builder-suanpan-time-bounds` (slot 46, question 71's table): `zz_probe_tmp.rs`, a temporary touch counter on the written set's word operations. It probes two candidate findings against the drafted bounds: a shifted operand whose digits cancel zero-fills about one digit per 32 bits of shift without growing the working width, and each cancelling pair costs about `log₆₄` of the shift.
- `reviewer-crate-page-identity-linearity` (slot 45, question 84): a deleted probe, `zz_reviewer_probe.rs`, whose `probe.log` shows a double restore overlapping, a repeated rewind giving equal versions, and a diverging rewind giving a distinct, dominated version. Its cases resemble A22's.
- `reviewer-writer-retention` (slot 33, question 73): `board_cmp.py`, a board comparison (section C's kind), and environment switches `RVW_FINALIZE` and `RVW_HELPER` that calibrate slot 33's retention tests (section A36's kind).
