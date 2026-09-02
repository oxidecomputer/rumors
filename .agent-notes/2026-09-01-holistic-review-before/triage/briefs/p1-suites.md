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

- **Base.** Your worktree's HEAD must equal `0fc1921e` before you start.
  Run `git -C <worktree> rev-parse HEAD`. If HEAD is an ancestor of
  `0fc1921e`, fast-forward; if it has diverged, stop and report. Never call
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
6. skyline-query-9 (ruling 10) last, after `p1-fuzz` has landed its
   re-pin; rebase onto it. If `p1-fuzz` has not landed, stop on this
   entry and report it as waiting (the coordinator may move it to that
   lane).

Files shared: `tests/meter.rs` with `p1-harness` (rebase; your edits
there are the isolation call, three band docs, the folded binaries, and
the reset sites) and with `p1-board` (two band docs, different bands);
`fuzzfit/harness/src/bands.rs` with `p1-fuzz`.

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

### skyline-query-9 (low, verification-gap): ruling 10

Resolution: Either instrument or narrow. To instrument: a multi-scale fuel fit (the fuzzfit harness already counts wasm fuel for `ff_version_rank`) over the doc's tight construction at three or more scales past 65 KiB, judged against the `M(n) · log n` model with `M ≈ n log n`; or a deterministic check that records `meter_product`'s operand widths per tree level on that construction and holds the per-level product widths to the model's telescoping. To narrow: state in the public contract only what committed instruments pin and move the quasilinear-tier remark to a decision record. Acceptance: either a committed cell or test whose operands' parked sums exceed 4,000 words at every scale, named from integral.rs in place of the "[derived; ...]" bracket and rostered, or the public `# Complexity` no longer carries the unwitnessed clause.

Ruled (10): instrument, by the multi-scale fuel fit in the fuzzfit
harness at three or more scales past 65 KiB over the doc's own
construction, judged against `M(n) · log n`. The public clause stands.
This entry is last in the lane and waits for `p1-fuzz`'s re-pin; its
own pin is attributed in its commit with the wasmtime and toolchain
provenance the bands file names. If the fit reads the extra log where the
model says it should not, or fails to read it where the doc says it is
tight, that is a finding about the doc's argument: report the readings,
do not re-word the contract.

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
- skyline-query-9 stops if `p1-fuzz` has not landed.
