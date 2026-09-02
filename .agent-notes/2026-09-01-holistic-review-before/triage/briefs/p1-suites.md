<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P1 lane: the metering suites' premises, floors, and satellites

## Goal

Every metered reading in `before` and `suanpan` rests on nextest's
process-per-test isolation, stated in prose and checked nowhere; the fold
doors' log-factor floors are transcribed midpoints whose linear reference
is printed and never asserted, and the roster claiming one pin per door
covers five of eleven; two bespoke flatness suites return growth 1.0 on a
dark counter and sit outside every roster, in satellite binaries that
duplicate the main suite's fixtures and have already drifted; three
accumulator bands justify their adequacy by a probe build that lives in
no tree; one law group alone defines the suite's critical path; and the
public rank contract's log-factor clause has no instrument at its tier.
The invariant restored: the isolation premise is asserted once and called
from every metering helper; every floor asserts the reference it must
exceed and the roster derives from the contracts; every flatness
criterion has a liveness floor and a home in the registry; the bands say
what mechanism they protect and why, and cite nothing that does not
exist; and the one claim with no instrument at its tier gets one.

## Ground rules

These apply to every P1 lane; the lane sections below add to them and
never relax them.

- **Base.** Your worktree's HEAD must equal `bba0e31a` before you start.
  Run `git -C <worktree> rev-parse HEAD`. If HEAD is an ancestor of
  `bba0e31a`, fast-forward; if it has diverged, stop and report. Never call
  EnterWorktree; operate on the worktree through `git -C <path>` and
  absolute paths, one shell invocation at a time.
- **The review documents are the specification.** Each member entry below
  quotes its Resolution and Acceptance verbatim from the class document in
  `.agent-notes/2026-09-01-holistic-review-before/`; the entry's full
  record (evidence, construction, witness outcome) is under `### <id>:`
  there (`#### <id>:` in `simplification.md`; nits in `documentation.md`,
  `simplification.md`, and `verification.md` are one-line table rows) and
  in `evidence/`. Line anchors are at the reviewed commit `9e5784fb`, and
  the tree outside `.agent-notes/` is byte-identical at your base, so they
  hold; re-anchor from the quoted evidence, never from a line number
  alone, once your own commits move the file.
- **The rulings govern.** `triage/rulings.md` records Finch's decisions.
  Where a quoted Resolution offers alternatives, the ruling named beside
  it picks one; where the ruling amends the Resolution, the amendment is
  stated under the quote and wins. Where a quoted Resolution and the lane
  goal come apart, the goal wins, and the discrepancy is reported.
- **Typed references, never strings (ruling 43).** Wherever this lane
  touches `meter/registry.rs`, a family roster, `TRIPWIRE_ROSTER`, the
  surface rosters, or any test that names another test, file, or line:
  reasons, pins, and enforcement homes are expressed as references the
  compiler resolves (function items, registered law names, `Shape` and
  `Op` values), never as strings naming a test function, a file, or a
  line number. A lane that sees a cleaner idiomatic shape for a roster is
  authorized to adopt it and reports the reshaping in its diff. Finch's
  words: "please make these instruments impossible to drift in the
  future. I *really don't like* the pattern of hard-coded strings and
  Rust source locations embedded in tests; the way these family rosters
  ended up is not really to my taste, but I haven't had time to make it
  more idiomatic and obviously correct. If you see a good way to clean it
  up, please do."
- **Stops.** Report and leave the entry open; do not work around: anything
  that moves an `insta` snapshot or a committed pin the brief does not
  name as moving; any change to a public signature or public rustdoc
  contract the resolution does not name (the meter surface under
  `any(test, feature = "meter")` is instrument surface by ruling 8 and
  not public API, but a change there lands with the `rumors` test update
  in the same commit); anything that contradicts a ruling; any deviation
  from the stated resolution; anything this brief marks as a stop. A stop
  on one entry does not block the others.
- **Negative controls.** Every repaired or added instrument lands with a
  committed demonstration that a known-bad artifact fails it. The
  constructions in `evidence/witness.md` and each entry's Construction
  line are those artifacts; convert each into a committed test
  (`should_panic`, an asserted `Err`, a judge test over synthetic samples,
  or a reversible mutation whose observed failure the commit message
  records verbatim). Ruling 20 is the one place this brief set says
  otherwise, and it says so at the entry.
- **Resource discipline.** Build with `cargo nextest run --no-run` before
  running; during iteration run only the binaries the entry names; never
  iterate on a timing measurement; launch no daemons or services. One full
  `just gate` per agent, before the final commit, run in the background
  redirected to a log under `<scratchpad>/<lane>/` and polled with short
  foreground checks (the foreground command cap is ten minutes; a
  foreground gate run cannot finish). Keep every working file and evidence
  log under that directory, never loose at the scratchpad top level.
- **Commits.** One commit per logical unit, its message describing the
  change and naming the entry ids and the ruling. Every re-pin is measured
  at the parent and its movement named in the commit. Commit every
  proptest seed file that appears. Prose speaks in the present tense: no
  reference to code that no longer exists, no dated rationale at a
  declaration site. Comments use spaced double-hyphens, never em-dashes;
  every test has a doc comment stating its invariant. Commit with a
  descriptive message before finishing.
- **Never delete anything outside your worktree.** If the disk fills
  (ENOSPC), stop and report; it is the coordinator's problem, not a
  reason to trim caches you do not own.
- **Self-retirement.** After the final commit, from inside the worktree:
  `cargo metadata --no-deps --format-version 1 | jq -r .build_directory`,
  then delete that directory and the worktree's `target/`. Leave the
  worktree in place.
- **Report.** For each entry: landed, stopped, or open; the commit sha(s);
  the acceptance evidence (the command and its decisive output, verbatim).
  Then anything left open and why, and every deviation from a stated
  resolution, explicitly. Your report is data: the coordinator verifies
  each entry's Acceptance against the tree at the reported sha before the
  ledger records it. Report what you could not do rather than working
  around it.

## Ordering inside the lane

1. `require_process_isolation()` in `before::meter` (ruling 19), then
   its call sites, `tests/meter.rs` last after rebasing onto `p1-harness`
   (one harness, one call).
2. The fold-door work (ruling 21): the roster totality test red-first
   (naming the six unpinned doors), the in-run reference assertion, then
   the five new pins or their excusals.
3. The satellites (ruling 22): shapes registered with their known-bads,
   the three binaries folded into `tests/meter.rs` (after the harness
   lane has landed, in the unified harness), floors added.
4. The band-doc restatements (ruling 20), also in `tests/meter.rs` after
   the rebase, and `registry.rs`.
5. `VERSION_TRIPLE` (ruling 27): the split, then one quiet-machine timing
   of each half. Check `uptime` first; if the load average is above the
   core count divided by four, skip the timing, say so, and leave the
   number to the coordinator. Never repeat the timing.
6. The rows and pins ruling 50 adds (envelopes-b-18's fold tripwire,
   the coverage-exit rows of skyline-sweep-place-masked-20, the
   `cmp_early_exit` rows of skyline-sweep-place-masked-32, meter-core-8's
   promotion tap and re-parameterized band), in `tests/meter.rs` after
   the rebase, each red-first; testing-oracles-22's grow-pair floor in
   `src/testing/exhaustive` at any point.

Files shared: `tests/meter.rs` with `p1-harness` (rebase; your edits
there are the isolation call, three band docs, the folded binaries, and
the reset sites) and with `p1-board` (two band docs, different bands);
`src/version/skyline/query/integral.rs` with nobody in P1
(meter-core-8's promotion tap), though `p2-cures` will edit it later.

## Members

### suite-economics-1 (medium, verification-gap): ruling 19

Resolution: add one guard and call it from every metering helper: `metered` (tests/meter.rs:363), `ticks_counters` (769-781), `identity_fast_paths::scanned` (10556), coincident_span's `scanned`, the two `counters` helpers, suanpan's `touches`, and the codec/base touch pin. The guard asserts `std::env::var("NEXTEST_EXECUTION_MODE").as_deref() == Ok("process-per-test")` and fails with a message naming the required runner; `before::meter` behind the `meter` feature is a natural single home so suanpan and the satellite binaries share it. Acceptance: the nextest gate is unchanged, and `cargo test -p before --all-features --test meter` fails every metered scenario immediately with the isolation message instead of reporting readings.

Ruled (19): as stated: one `require_process_isolation()` in
`before::meter`, called on entry from every metering helper. suanpan's
metered tests call it too; if suanpan cannot depend on `before::meter`
without a cycle (before depends on suanpan), the guard's one definition
lives in suanpan's `touch_meter` module and `before::meter` re-exports
it; say which in the commit. `ISOLATION_NOTE` becomes the guard's
message and its other copies dissolve.

### envelopes-a-3 (low, verification-gap): ruling 19

Resolution: Add one `fn require_isolation()` called at the top of every harness (one site once envelopes-a-4 lands) that panics with `ISOLATION_NOTE` unless `std::env::var_os("NEXTEST").is_some()` or `RUST_TEST_THREADS` is `1`. An in-flight `AtomicUsize` that panics on a second concurrent metered body is a cheaper partial guard but misses setup-phase bleed (generators record scan and limb work while building shapes), so the environment check is the one that closes the hole. Acceptance: `cargo test -p before --all-features --test meter` fails fast with the isolation message before any measurement; `cargo nextest run` and `RUST_TEST_THREADS=1 cargo test` behave as today.

Ruled (19): one change with suite-economics-1. The check is
`NEXTEST_EXECUTION_MODE == process-per-test`, not `NEXTEST` presence or
`RUST_TEST_THREADS`; the `RUST_TEST_THREADS=1 cargo test` clause of this
acceptance therefore does not hold and is superseded (a single-threaded
`cargo test` fails the guard by design, as envelopes-b-10 notes).

### envelopes-b-10 (low, verification-gap): ruling 19

Resolution: the owner's choice between (a) one shared entry check that `std::env::var_os("NEXTEST_EXECUTION_MODE")` reads `process-per-test`, panicking with `ISOLATION_NOTE` otherwise (complete, but forbids a single-threaded `cargo test` that is in fact isolation-safe), and (b) the status quo with the premise stated in the file doc. If (a), the check belongs in one helper called by the four harnesses and every band `run` that resets counters directly (5372-5374, 5574, 7481, 8398, 9418), or in `src/meter.rs`'s reset functions. Acceptance: running the binary under a shared-process parallel runner fails deterministically with the isolation message at the first scenario; under nextest all tests pass unchanged.

Ruled (19): (a). Every band `run` that resets counters directly calls
the guard too, not only the harness.

### meter-registry-tier2-19 (low, verification-gap): ruling 19

Resolution: add a `meter::require_process_isolation()` that asserts `NEXTEST_EXECUTION_MODE == "process-per-test"` (nextest exports it) with the isolation note as its message, called from the reset entries under `cfg(test)`; or name the premise as a hard rule in `crates/before/AGENTS.md` and accept the unguarded state explicitly. Acceptance: `cargo test -p before --lib --features limb-meter` fails fast with the isolation message at the first counter reset, while `just test-all` is unaffected; or the AGENTS.md rule exists.

Ruled (19): the guard, not the AGENTS.md rule. Calling it from the reset
entries covers every lib-side site at once; do that rather than
enumerating the nine files.

### testing-diff-gen-27 (low, test-quality): ruling 19

Resolution: Mirror tests/meter.rs: append an isolation note to `assert_log_factor_alive`'s and the Display pin's messages and state the nextest requirement in the module doc; optionally fail closed with `assert!(std::env::var_os("NEXTEST").is_some())` at the top of each metered pin. Acceptance: a failing pin's message names the shared-process runner as the first cause to rule out, or the pins refuse to run outside nextest.

Ruled (19): the pins refuse to run outside nextest, through the shared
guard (the reset entries call it, so `door_scan_bits` is covered; state
the requirement in the module doc as well).

### board-ops-render-27 (low, test-quality): ruling 19

Resolution: Hoist `ISOLATION_NOTE` to a shared meter test-support location (or re-declare it here) and append it to every assertion whose operand is a counter reading in these six tests; alternatively serialize the counter-reading tests behind one process-wide lock. Acceptance: every counter-comparison assertion in tests.rs carries the note or the tests hold a shared lock.

Ruled (19): covered by the guard at the reset entries; the six tests
need no per-assertion note once a shared-process run fails at the first
reset. Confirm each of the six resets through an entry that calls the
guard.

### suanpan-tests-12 (nit): ruling 19

Table row: `crates/suanpan/src/accumulator/tests/metered.rs:1-11`; metered.rs does not state the process-per-test premise its exact pins rest on (stated at touch_meter.rs, not where a reproducer looks). Resolution: One sentence in the module doc naming nextest.

Ruled (19): the sentence, and suanpan's `touches` helper calls the guard.

### testing-diff-gen-26 (medium, verification-gap): ruling 21

Resolution: Return `(scan_bits, input_bytes)` from `door_scan_bits` (and the five `*_scan_bits` helpers), and in `assert_log_factor_alive` also assert `bytes_hi / bytes_lo < min_growth` with a message naming it as the linear reference the floor must exceed; this keeps the measured floors and makes the "midway" claim checkable. Owner-gated alternative: assert the door's marginal over the in-run linear reference (`growth / linear >= 1 + margin`) with the margin derived from the model (`log2(2·1024)/log2(2·256) = 11/9 ≈ 1.22` at this quadrupling; the version doors read 5.82/4.77 = 1.22), so a `FOLD_DOOR_TEETH` change needs no hand re-pin of five numbers; the party door's 1.04 marginal then stands out as the witness that needs a sentence or a better population (finding 28). Acceptance: each fold-door pin asserts both `scan growth >= floor` and `byte growth < floor` in the same run; temporarily replacing a door with a linear pass over the population reads red.

Ruled (21): the first option, inside the f0cd4ab2f ruling (measured
floors stay; the in-run linear reference is asserted beside each). The
owner-gated alternative is not taken.

### meter-adequacy-5 (low, verification-gap): ruling 21

Resolution: return `(scan_bits, input_bytes)` from `door_scan_bits` and assert `hi_bytes / lo_bytes < min_growth` beside `growth >= min_growth`, so each run checks that the floor still sits above the linear reference. Optionally hold one committed left-fold kernel under each door's floor as the family's known-bad. Acceptance: the pins fail under the construction.

Ruled (21): one change with testing-diff-gen-26; take the optional
left-fold known-bad for at least one door (it is the negative control
the ground rules ask for) and say which.

### testing-diff-gen-23 (medium, verification-gap): ruling 21

Resolution: Add a `const PINNED_FOLD_DOORS: &[&str]` beside the pins and a totality test that reads `crates/before/fuelscape/*.json`, collects every op whose `contract` contains `log k` under a balanced fold (excluding the forks and `version_ticks` by a stated rule), and asserts each is either pinned or excused with a reason at the check site (`clock_recv_all`: delegates to `Version::join_all`). Then either pin the remaining five doors over the stagger population (versions lifted to spans for the `Span` doors; the stagger clocks for `sync_all`) or excuse each with a reason. Narrow the module doc to what is pinned until then. Acceptance: a test in this module fails when a `fuelscape/<op>.json` contract containing `log k` names a fold door absent from the roster and unexcused; it passes at HEAD only after the six doors are pinned or excused.

Ruled (21): as stated; the roster derives from the contracts. Pin
`sync_all` and the four `Span` doors unless a door's wiring makes the
floor unmeasurable, in which case excuse it with the reason at the check
site and report which. `clock_recv_all` is excused as delegating.

### testing-diff-gen-28 (medium, claim): ruling 21

Resolution: Witness the id fold's log factor on a population whose unions do not densify (isolated owned leaves at maximal depth, so each balanced union's encoding is near the sum of its parts), where the door should read near ×6.0 against ×4.8 as the version doors do; independently, restate lines 12-14 to what the pins can attribute: a deterministic tightness pin at two scales whose crossing means re-derive, not class change. Acceptance: for every fold-door pin the measured reading exceeds the in-run byte ratio by a stated margin the population is shown to express, and a committed known-bad (a left fold behind the same door on the same population) reads below the floor.

Ruled (21): as stated. If the new population does not widen the party
door's margin as predicted, report the measured reading and keep the
module-doc restatement; do not re-pin the floor to a margin the
population does not express.

### tests-other-14 (medium, verification-gap): ruling 22

Resolution: fold_skeleton: delete the `c0 == 0` branch; assert `scan > 0` and `touch > 0` at level 0 (better, derive them: the hull fold reads every operand at least once, so scan >= 8 * bytes minus per-boundary padding slack; touch >= one per leaf boundary), and either drop the limb leg or assert `limb == 0` at both levels with the model stated at the site (a dense spine of small heights does no `Base` arithmetic). answer_embedded: assert `t[0] > 0` per (op, label) in `assert_no_product` and `per_byte[0] > 0` in the ladder loop, with the crate's "a zero is a dead meter" message; name `MIXED_DIFFERENCE_FRACTION` and `PER_BYTE_GROWTH_BOUND` with their rationale beside them, as `GROWTH_BOUND` is. Acceptance: with `f()` moved above the three resets in `counters` (every reading zero), all three tests read red on a floor; at HEAD they stay green with the MEASURED lines unchanged; the limb leg is either gone or pinned to zero with its reason.

Ruled (22): `limb == 0` pinned at both levels with the model stated at
the site; the derived floors (the "better" option) for scan and touch.
The dark-meter construction is the negative control.

### tests-other-6 (medium, verification-gap): ruling 22

Resolution: Owner call between two shapes. (a) Register WT, WL, and the forked deep spine as `Shape`s with `Bands` answers, move the three criteria into `tests/meter.rs` under the band naming convention so the parity scan sees them, and commit one kernel per criterion under the superlinear roster: for WT the schoolbook settle already driven by `schoolbook_run` in query/tests.rs, for WL a per-rung re-densification, for the D leg a per-frame path-sum fold (the mechanism `bigroot`'s doc names). (b) Keep them bespoke and state at each assertion that its adequacy rests on the closed-form separation, with the measured margin (mixed 0 against a bound of a tenth) recorded as the reason no kernel is committed. Acceptance: (a) `band_tests_and_registry_citations_stay_paired` cites the three bands and `superlinear_tripwires_match_the_committed_roster` rosters one kernel per band, each reading red through the band's meters; or (b) the ruling is stated at the site.

Ruled (22): (a). Land after the harness lane so the criteria enter the
unified harness under the band naming convention.

### suite-economics-4 (low, modularity): ruling 22

Resolution: move coincident_span's three tests into meter.rs's `identity_fast_paths` (one `fixture`, one `scanned`); place answer_embedded and fold_skeleton beside `answer_embedded_product` in meter.rs with one counters helper and each band's tolerance derived at its site; for any feature-gated binary that stays separate, declare `[[test]] required-features` as the examples already do, so the unfeatured build skips it instead of linking an empty harness. Folding the four tiny roster binaries into one is a taste call whose link-time benefit is assessed, not measured. Acceptance: one `fn fixture()` and one counters helper across tests/; `cargo nextest list --workspace` under default features lists no zero-test before binary.

Ruled (22): the three binaries fold into `tests/meter.rs`; none stays
separate, so no `[[test]] required-features` entry is needed for them.
The MEASURED grids are byte-identical against the parent, or the
difference is explained (tests-other-7's leaf-order caveat).

### tests-other-7 (low, modularity): ruling 22

Resolution: Add `tests/support/meters.rs` with a `Counters { scan, limb, touch }` struct, `counters(f)` routed through `meter::*` and carrying the dead-meter floor and the isolation note, `scanned(f)`, and `balanced_forks(n)`; include it by `#[path]` from the four binaries (and offer it to tests/meter.rs); hoist `rounds` into `fixture()` in coincident_span.rs. Replace the tiling loops with `<[Party; 16]>::from(Party::seed())` / `seed.forks(n - 1)` only after confirming the leaf order the `step_by(2)` alternation in `wt` depends on (see the open questions). Acceptance: one definition each of `counters` and `scanned` under `tests/support/`; no `while parties.len() < n` loop in the two files; MEASURED grids unchanged against the parent, or a changed grid explained by the leaf-order difference.

Ruled (22): with the three binaries folded into `tests/meter.rs`, the
helpers dedupe inside that file (one `counters`, one `scanned`, one
fixture) rather than in `tests/support/meters.rs`; the `#[path]` include
is moot. The tiling-loop replacement is taken only if the leaf order is
confirmed identical; otherwise keep the loop once and say why.

### envelopes-a-22 (low, verification-gap): ruling 20, resolution replaced

Resolution: For the three accumulator bands, either name per band the cargo-mutants mutant (file, function, replacement) that disables the mechanism so the demonstration reproduces from `cargo mutants -f crates/suanpan/src/accumulator.rs -F <pattern>` under the campaign configuration, or commit each disabled-mechanism variant as a suanpan `_reads_superlinear` witness and cite it here as the sibling bands cite theirs. For eq early-exit, replace 5076-5077 with a citation of the mutants roster's `sweep::eq_exit` entry. Acceptance: each of the four bands' docs names a committed artifact (a test or a rostered mutant) that reads superlinear on its family; `grep -n 'local probe build\|at pin time' crates/before/tests/meter.rs` returns nothing.

Ruled (20), Finch's words: "This is ghost references in disguise. Just
discuss the correct mechanism and why it's correct." Neither option.
Each of the three bands' docs (weight-comb, freeze-parade, tooth-tail)
drops the probe-build reference and its pin-time readings and instead
states the mechanism the band protects (the zero-run certificate, the
write watermark, exact-top maintenance) and why that mechanism yields
the flat reading on this family. No disabled-mechanism kernel is
committed; no mutant is cited; the "adequacy witness" wording goes. The
eq early-exit band's pin-time aside at 5073-5077 is rewritten the same
way (the mechanism: `eq` exits at the first decided position; why the
family's pairs make that a tail-linear saving), citing no roster (the
gate lane deletes it). The negative-controls ground rule is waived for
these four docs by this ruling and nowhere else. Acceptance: the grep in
the entry returns nothing; no band doc names a probe build, a mutant, or
a roster; each states its mechanism and the reason.

### meter-adequacy-6 (low, verification-gap): ruling 20, resolution replaced

Resolution: either commit the three probes as kernels (a test-only accumulator strategy in suanpan that steps digit by digit, reads from digit 0, or tracks the buffer high water; `_reads_superlinear_on_weight_comb` and siblings in before; rows in `TRIPWIRE_ROSTER`) and cite them from the band docs and registry, or drop the "adequacy witness" wording at all three bands and the registry and state that the bands pin measured flatness only. Acceptance: no band doc cites a demonstration that is not in the tree.

Ruled (20): one change with envelopes-a-22. The registry rows at
`registry.rs:769-774` and `784-786` are restated the same way: the
mechanism and why, not "demonstrated by a probe build" and not "measured
flatness only".

### meter-registry-tier2-7 (medium, verification-gap): ruling 20, resolution replaced

Resolution: commit the three refuted mechanisms as kernels following the crate's own pattern (test-local variants in the query suite or suanpan's metered suite, mirroring `no_collapse_fold_re_scans_the_prefix`): a settle that steps the gap digit by digit, asserted superlinear across `WC(n) -> WC(2n)`; a scaled read starting at digit 0 (the plain `sign_magnitude` suanpan's metered.rs:20-23 already names as the full-held-width read), asserted superlinear across `FZ(k) -> FZ(2k)`; a high-water-bounded sign read on the tooth-tail pair. Add each to `TRIPWIRE_ROSTER`, then replace "demonstrated by a probe build" at registry.rs:770-771 and 784-786 and "a local probe build" at tests/meter.rs:4489, 4551, 4660, 4742 with the kernel names. The nearest existing mitigation, suanpan's exact-count row pins (`alternating_shifted_writes_cost_the_operand_not_the_gap`, `scaled_read_costs_the_written_span`, `held_width_rows_cost_the_held_digits`), holds the shipped mechanisms crate-locally but demonstrates no known-bad failing. Acceptance: the roster gains three entries, each reads red on its family at both scales, and `grep -rn 'probe build' crates/before crates/suanpan` returns nothing.

Ruled (20): the kernels are not committed and the roster does not grow.
One change with the two entries above; the acceptance that survives is
`grep -rn 'probe build' crates/before crates/suanpan` returning nothing,
with each restated site naming its mechanism and the reason.

### suite-economics-2 (low, performance): ruling 27

Resolution: split VERSION_TRIPLE into two `pub static` groups with the same header, the lattice/order/metric laws (852-946) staying and the span and query laws (962-1605) moving to a group such as SPAN_TRIPLE, registered in `for_each_law_group!` with its own driver name; no consumer arm changes. Re-point the two comments that name the group (span/tests.rs:442-443; version/tests.rs:26-31). Measure each half once on a quiet machine before choosing the cut. Acceptance: same predicates, same `arb_oracle_version` generator, 256 cases per group; `every_law_group_is_registered` and `law_names_are_unique_across_groups` pass unchanged; in a full `just test-all` the last test to finish is no longer alone by several seconds.

Ruled (27): the semantic cut (lattice/order/metric versus span/query),
which the entry says coincides with the cost cut; measure each half once
after the split, on a quiet machine, per the lane ordering's load check,
and record the two per-test times in the commit. The fuzz target and the
organic drive list pick the new group up by construction; confirm by
running the law totality pins.

### envelopes-b-18 (medium, verification-gap): ruling 50

Resolution: add `sequential_join_reduce_reads_superlinear_on_stagger` mirroring 8313-8343 (`population.into_iter().reduce(|acc, v| acc | v)` over the version half of `Shape::StaggerPopulation.population(n, m)` on the arity axis at two scales, asserting model-normalized per-byte growth at or above a class-separating floor such as ×1.49), roster it in `TRIPWIRE_ROSTER`, and excise the parenthetical at 7727; do the same for the party left fold if its reading separates. Acceptance: a `_reads_superlinear_on_stagger` kernel in tests/meter.rs, rostered, red on the sequential fold and failing if `join_all` is swapped in; 7727 no longer points at a commit message.
Construction: copy `sequential_meet_reduce_reads_superlinear_on_shade` with `&` -> `|` and the shade population -> the stagger population at `(n, n)` and `(2n, n)`; the module comment predicts about ×2 per-byte growth per arity doubling, so a ≥ ×1.49 assertion should hold; if it does not, the module comment's adequacy claim is the finding.

Ruled (50): commit `sequential_join_reduce_reads_superlinear_on_stagger`
mirroring the meet tripwire, rostered, and excise the parenthetical at
7727; do the same for the party left fold if its reading separates (say
which way it read). Under ruling 43 the roster entry is the function
item, not its name as a string, if `TRIPWIRE_ROSTER`'s type allows it at
your base; otherwise report that the roster's typing is the next reshaping
and cite by name. The witness read ×1.73 against ×1.12 with the ×1.49
class floor; if your reading does not clear the floor, report the numbers
rather than lowering the floor.

### skyline-sweep-place-masked-20 (medium, verification-gap): ruling 50

Resolution: add two scan-bit rows to the placement meter module, stated relationally like the others: (1) a hole-only query concurrent to both endpoints must read strictly under the composed four `partial_cmp`s and stop at the second deciding interval (the early `Full`); (2) a required floor plus a hole whose `lo` pair settles must show the `lo` stream's scan stopping (compare against the same query with the hole removed). Add the two deterministic verdict witnesses (all-holes-settled `Full`; endpoint drop then `Partial`) to `filter_coverage_organic_witnesses`. Acceptance: the new rows read green at HEAD and red when lines 427-430 and 439-440 are removed, while every verdict test stays green.

Ruled (50): the two relational scan rows in the placement module and
the two deterministic verdict witnesses. The negative control is the
acceptance's own: with lines 427-430 and 439-440 removed (a reversible
mutation), the rows read red while every verdict test stays green; record
the readings in the commit message.

### skyline-sweep-place-masked-32 (medium, verification-gap): ruling 50

Resolution: add `cmp_early_exit` rows in the `eq_early_exit` idiom: a pair decided concurrent at its second elementary interval with a scale-varying tail, absolute two-scale touch and scan pins on `Version::partial_cmp`, plus the masked twins (`(v / p).partial_cmp(&w)` with the deciding intervals inside owned regions, and a masked `eq`). Update sweep.rs:46-52 and masked.rs:61-65 to name the rows. Acceptance: with `order_exit` returning `Continue` unconditionally (and masked's `run` ignoring `Break`), all verdict suites remain green and the new rows read tail-linear and fail at both scales; restored, the rows read identical at both scales.

Ruled (50): `cmp_early_exit` rows in the `eq_early_exit` idiom on
production `Version::partial_cmp`, plus the masked twins, with
`sweep.rs:46-52` and `masked.rs:61-65` naming the rows. The negative
control is the acceptance's own: `order_exit` returning `Continue`
unconditionally and masked's `run` ignoring `Break`, as a reversible
mutation, with the tail-linear readings (the witness saw 8005 to 16005
touches) recorded in the commit message. The witness also saw
`coincident_place_collapses_to_the_pair_sweep` go red under that mutation
through its relative inequality; that test is folded into this file by
ruling 22 and is not a pin of the exit, so its reaction is noted, not
relied on.

### meter-core-8 (medium, claim): ruling 50

Resolution: Add a `cfg(test)` promotion tap beside `FREEZE_HITS` (integral.rs:284) and a meter/tests.rs pin that `wide_arming(w, d).version().rank()` promotes exactly once for `w >= 18` and zero times at `w = 17`; then either raise both guards to the promotion threshold, derived from `FREEZE_ALLOWANCE_DIGITS` once meter-core-7 makes it reachable, and hand the envelope partition a re-parameterized `hoisted_window` band at a promoting width, or re-state both docs and the band prose to the freeze-only mechanism the current widths realize. Acceptance: the promotion pin exists and passes; the guard's number and its rationale describe the same threshold; the `hoisted_window` band's width sits on the documented side of it.

Ruled (50): the first branch in full. A `cfg(test)` promotion tap
beside `FREEZE_HITS`, a pin that `wide_arming(w, d).version().rank()`
promotes exactly once for `w >= 18` and zero times at `w = 17`, both
guards raised to the threshold derived from `FREEZE_ALLOWANCE_DIGITS`
(state the derivation at the guard; the witness read three freezes per
run, not the entry's two, so derive from the code, not the entry's
trace), and the `hoisted_window` band re-parameterized at a promoting
width with its re-pin measured at the parent and attributed. meter-core-7
(the constant's reachability) is a P6 entry; if the derivation needs it,
take only what the derivation needs and say so.

### testing-oracles-22 (medium, verification-gap): ruling 50

Resolution: Count the pairs that reach line 317 (an `AtomicUsize` under the par iter, returned from `check_tick` or asserted inside it) and assert `grow_pairs >= ids.len() * evs.iter().filter(|v| matches!(v, oracle::Version::Leaf(_))).count()`, with the premise stated at the assertion site (fill is the identity on a leaf event for every nonempty id). Acceptance: temporarily making `fill_for_test` return `self.fill(id).tick(&Party::Leaf(true))` (always different from `self`) turns `exhaustive_small` red on the new floor rather than green; restored, it is green.
Construction: In crates/before/src/oracle/version.rs change `fill_for_test` to return `self.fill(id).tick(&Party::Leaf(true))`; run `cargo nextest run -p before exhaustive_small`: every pair hits `continue`, `best_inflation` and `all_inflations` execute zero times, and the test passes. With the floor added, the same mutation fails the floor's assertion.

Ruled (50): count the pairs that reach the grow check under the
parallel iterator and assert the universal floor at the site with its
premise stated. The negative control is the acceptance's own: the
always-changing `fill_for_test` mutant turns `exhaustive_small` red on
the floor; record the run in the commit message.

### meter-adequacy-10 (nit, (table)): roster: pending Finch's approval

Table row: `crates/before/tests/meter.rs:8168-8197`; The meet-fold band's `touches >= bytes` is labelled a liveness floor with no mechanism for one touch per operand byte. Resolution: Derive from nonzero deltas times first-level merges, or relabel it a measured band (the suite's own "improvement tripwire" class).

Roster note: the meet-fold band's `touches >= bytes` floor either
derives from nonzero deltas times first-level merges or is relabeled a
measured band. Lands with the band-doc work if Finch approves the
roster; say which way you took and why.

### skyline-sweep-place-masked-4 (nit, claim): roster: pending Finch's approval

Resolution: state the pinned mechanism as "no per-interval sign read inside an unowned run" at masked.rs:313-316 and at the generator (src/meter.rs:3506-3515) and band (tests/meter.rs:7515-7517), and name the zero-delta premise at the generator. Optionally add a nonzero-delta spine variant whose reading is expected linear, as the fold floor. Acceptance: the prose names sign reads and the zero-delta premise; the band is unchanged.

Roster note: prose at three sites naming the pinned mechanism (no
per-interval sign read inside an unowned run) and the zero-delta premise;
the band is unchanged. The optional nonzero-delta spine variant is not
taken unless Finch's roster approval says so. Lands only if approved.

## Hazards and stops

- `tests/meter.rs` is the harness lane's first. Every edit of yours there
  lands after the rebase; before it, land the `before::meter` guard, the
  asymptotics work, `laws.rs`, and the registry.
- Ruling 20 waives the negative-control rule for four band docs and the
  two registry rows only. Every other instrument this lane adds lands
  with its committed known-bad.
- A fold-door pin whose in-run byte ratio sits at or above its floor
  today is already vacuous: that is a finding to report with the numbers,
  not a floor to move.
- The `VERSION_TRIPLE` timing is run once, after a load check, or not at
  all. Never iterate.
- The satellites' MEASURED grids must match the parent byte for byte
  after folding, or the difference is the leaf-order caveat the entries
  name; anything else is a stop.
- meter-core-8 moves the `hoisted_window` band's width and re-pins
  the band; that re-pin is the one this lane makes by design, measured
  at the parent and named in the commit. Any other band pin moving is a
  stop.
