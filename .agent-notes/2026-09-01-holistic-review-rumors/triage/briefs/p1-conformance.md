<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P1 lane: the backend conformance suite and the window census

## Goal

Every instrument in this lane exists to show that a memory budget binds a
reconciliation session: the backend conformance suite prices what a
backend holds and differences a budgeted run against a floor run, and the
window census suites do the same over a live session. At the reviewed
commit both difference two identical runs, because a 64 KiB budget sits
below the window solve's flat decode-fan pre-charge (about 210 KB under
`Local` pricing), so every capacity floors at one and the ceiling can
never fail. Beneath that, the suite convicts less than the `Backend` trait
docs claim, and three obligations it transcribes are checked by nothing.
The invariant this lane restores: a ceiling over a counter is paired with
a floor that proves the counter counts, and every clause the `Backend`
docs say the suite convicts is convicted by name.

## Ground rules

These apply to every P1 lane; the lane sections below add to them and
never relax them.

- **Base.** Your worktree's HEAD must equal `0926fe32` before you start.
  Run `git -C <worktree> rev-parse HEAD`. If HEAD is an ancestor of
  `0926fe32`, fast-forward; if it has diverged, stop and report. Never call
  EnterWorktree; operate on the worktree through `git -C <path>` and
  absolute paths, one shell invocation at a time.
- **The review documents are the specification.** Each member entry below
  quotes its Resolution and Acceptance verbatim from the topic document in
  `.agent-notes/2026-09-01-holistic-review-rumors/`; the entry's full record
  (evidence, construction, demonstration) is under `### <id>:` there and in
  `evidence/`. Line anchors are at the reviewed commit `9e5784fb`; re-anchor
  from the quoted evidence, never from the line numbers.
- **Goal beside mechanism.** Where a quoted resolution and the goal above
  come apart, the goal wins, and the discrepancy is reported.
- **Stops.** Report and leave the entry open; do not work around: anything
  that moves an `insta` snapshot or a committed pin; any change to a public
  signature or public rustdoc contract the resolution does not name;
  anything that contradicts a ruling in `triage/rulings.md`; any deviation
  from the stated resolution; anything this brief marks as a stop. A stop on
  one entry does not block the others.
- **Negative controls.** Every repaired instrument lands with a committed
  demonstration that a known-bad artifact fails it. The constructions in
  `evidence/witness.md` and each entry's Construction line are those
  artifacts; convert each into a committed test (`should_panic`, an asserted
  `Err`, or a reversible mutation whose observed failure the commit message
  records verbatim).
- **Resource discipline.** Build with `cargo nextest run --no-run` before
  running; during iteration run only the binaries the entry names; never
  iterate on a timing measurement. One full `just gate` before each commit,
  run in the background redirected to a log under
  `<scratchpad>/p1-conformance/`, polled with short foreground checks (the
  foreground command cap is ten minutes). Keep every working file under that
  directory.
- **Commits.** One commit per logical unit, its message describing the
  change and naming the entry ids and ruling. Commit every proptest seed
  file that appears. Prose speaks in the present tense: no reference to code
  that no longer exists, no dated rationale at a declaration site. Comments
  use spaced double-hyphens, never em-dashes; every test has a doc comment
  stating its invariant. Never delete anything outside your worktree; if the
  disk fills, stop and report.
- **Self-retirement.** After the final commit, from inside the worktree:
  `cargo metadata --no-deps --format-version 1 | jq -r .build_directory`,
  then delete that directory and the worktree's `target/`. Leave the
  worktree in place.
- **Report.** For each entry: landed, stopped, or open; the commit sha(s);
  the acceptance evidence (the command and its decisive output, verbatim).
  Then anything left open and why. Your report is data: the coordinator
  verifies each entry's Acceptance against the tree at the reported sha
  before the ledger records it. Report what you could not do rather than
  working around it.

## Ordering inside the lane

1. `conformance-28` first: the census floor is what demonstrates that any
   later budget binds, and `LOCAL_BUDGET` is sized only after the floor is
   in place and has been seen to fail at 64 KiB.
2. `tests-resource-link-window-20` next, measured (its resolution says
   measure first), and `window_corners.rs:174` re-measured with it.
3. Then `conformance-24`, `-25`, `-31`, `-30`, whose faults the suite today
   convicts only incidentally, so their by-name controls are meaningful
   only once the census is alive.
4. Then the three transcribed obligations `-40`, `-41`, `-42`.
5. `tests-resource-link-window-19`, `-25`, `-28` at any point after step 2.

## Members

### conformance-28 (high): ruling T6

Resolution: In `check`, pair the ceiling with a floor: `assert!(budget_peak > floor_peak, "the budgeted window admitted nothing above the floor: the budget does not bind at this corpus scale")`, or resolve both `WindowConfig`s against the corpora and assert the budget window's widest capacity exceeds one. Name the `Local` budget (`LOCAL_BUDGET`, beside `MATERIALIZING_BUDGET`) and size it above the flat term, for example `SUPPLY_DECODE_ENVELOPE_BYTES + 64 * 1024` (both in scope under `cfg(test)`), with the constant's doc stating the term it must clear; restate the testdoc from the measured behavior. Re-check `MATERIALIZING_BUDGET` against its own flat term with the same floor. Acceptance: with the floor in place, the current `Local` budget fails the floor assertion; with the re-sized budget both conformance tests pass and the floor holds; the testdocs describe exactly what the body asserts.

Negative control: the second witness pass's construction (`evidence/witness.md`, section `conformance-28`): the floor inserted at 64 KiB fails with both peaks 14496. Commit the floor; record that observed failure in the commit message before the resize lands in the same or the next commit.

### tests-resource-link-window-20 (high): ruling T26

Resolution: Measure first, then: pick a `TIGHT_BUDGET` whose derived capacities exceed one at 21,024 messages a side (window_knee's 2 MiB lands 4..=256 at ~2,300 messages; the population is larger here, so the budget likely needs to be larger); assert the fixture's own liveness, `capacities.iter().sum::<usize>() > capacities.len()`, beside the admittance computation; have `reconcile` return the two `Gossiped` values and assert the windowed arm's `stats.window_granted > 1` so the real session, not only the test-internals solve, is shown to widen; add `assert!(windowed > floor, ...)` with the measured value in the message; replace `saturating_sub` at lines 96 and 280 with `checked_sub(...).expect("peak covers both resting generations")` so a negative reading fails instead of reading 0; rewrite line 27's doc to state the property the budget must have. Re-measure window_corners.rs:174 alongside and either raise its budget past the pre-charge or restate that test's doc to what it exercises. Acceptance: the census log shows different capacities sums and different peaks for the two arms; the new floors are committed with measured values; a run at `TIGHT_BUDGET = 64 * 1024` fails the fixture's liveness assertion.

The measured budget is a number you produce, not one you are handed; the commit message records the run that produced it.

### conformance-24 (medium): ruling T6

Resolution: In `Charged::parent`, after the inner call, record `ledger::violation("parent contract: fan {fan} yielded {Some/None}")` when `parent.is_some() != (fan > 0)`. In `Charged::assemble`, track the previous yielded `Prefix<H>` and record `unordered assembly` when not strictly ascending; in `Charged::leaves`, track the previous `Prefix<Z>` and the requested prefix, recording `unordered leaf walk` and `escaped leaf walk`. Add knobs to `Materializing` (`PARENT_DROPS` returning `Ok(None)` for one interior fan; a swap of the first two yielded items in `leaves` and `assemble`) with `should_panic` controls. Surface pending ledger violations in the message of `run`'s `converged` assertion, so the by-name report is not masked by the convergence panic. Acceptance: each new control fails by name; honest backends pass; both `Backend` conviction sentences are true of the code.

Note from the witness pass: the interior `parent` fault is today convicted by `check_assembled`'s length invariant, not by name; the control must assert the by-name message.

### conformance-25 (medium): ruling T6

Resolution: In the `children` stream, price each yielded child at `B::node_bytes(node.len().min(FAN), bound_bytes(&node))` (the fan is invisible here; use the monotone cap `check_assembled` uses and argues) and record `underpriced child` when `measured > priced`. Add a `CHILDREN_SLACK` knob to `Materializing::children` (resize the lazily loaded row, like `WALK_SLACK`) with a `#[should_panic(expected = "underpriced child")]` control at `BULK_OVERHOLD`. Restate the module doc's pointwise bullet to include exploded children. Acceptance: the new control fails by name; `local_backend_conforms` and `materializing_backend_conforms` still pass.

Note from the witness pass: slack on every child row is caught at the walk's leaf check, because `Materializing::leaves` re-enters `Charged::leaves`; the control must restrict slack to interior children (`H::HEIGHT > 0`), which today passes with no violation.

### conformance-31 (medium): ruling T6

Resolution: In `run`, before `reset_peak`, also drive `charged.clone().assemble::<H>(...)` over each corpus's sorted leaves at a sub-root `Convert` height (a one-byte prefix yields up to 256 runs at this corpus scale) and drain it. Add two knobs to `Materializing::assemble`: drop the k-th assembled node (`unassembled run`) and re-tag one node's prefix to a neighbor (`unsupplied assembly`), each with a `should_panic` control. The `bulk-assembled len` and over-hold controls then also run in the multi-run regime. Acceptance: both new controls fail by name; the honest suites pass; the multi-run `Local::assemble` boundary is covered.

### conformance-30 (low): ruling T6

Resolution: Also assert `left_root.len() == COMMON + 2 * DIVERGENT`, or build `corpus(common ⊕ left_tail ⊕ right_tail)` once and compare its root hash with both sides'. Acceptance: the assertion holds on the honest runs and fails on a session that drops any leaf on both sides.

### conformance-40 (low): ruling T6, amended

Resolution: Fold the slot excess into the leaf comparisons at 258-263 and 401-407: `let slot_excess = size_of::<(Prefix<Z>, B::Node<Z>)>() - size_of::<B::Node<Z>>() - FAN_SLOT_BYTES` (with `FAN_SLOT_BYTES` made `pub(crate)`), recording `underpriced leaf` when `measured + slot_excess > priced`; do the same for `REFERENCE_SLOT_BYTES` against `(u8, B::Node<Z>)` where the per-level references are priced (`Charged::children`, once conformance-25 prices them). Prefer this form over a `const` assertion, because the window docs ask `node_bytes` to price the padding rather than forbid it. Add a negative control in backend/tests.rs: a `#[repr(align(16))]` wrapper node whose `node_bytes` omits the padding, asserted to fail by name. If measurement is not added, the backend module doc needs the "What the suite cannot see" section it lacks, naming slot padding. Acceptance: a backend whose `Node<Z>` alignment exceeds the pointer's fails `check` by name unless its `node_bytes` covers the extra slot padding; `Local` and `Materializing` pass unchanged.

Amendment (T6): the "if measurement is not added" alternative is withdrawn. The obligation is checked, not admitted in prose.

### conformance-41 (low): ruling T6, amended

Resolution: In `assume`, measure the result (`B::measure::<H>(&node)`), record `ledger::violation("re-tagged node changed residency: erased {bytes} B, assumed {measured} B")` when it differs from the carried bytes, and wrap at the measured value. `erase` has no oracle unless `Measure` gains a method over `Erased`, so either add `fn measure_erased(node: &Self::Erased) -> usize` and check symmetrically, or state at 285-288 that erasure is trusted on the trait's clause. Add an `ASSUME_SLACK` knob to `Materializing::assume` that grows the row, with a `#[should_panic(expected = "re-tagged node changed residency")]` control. Restate the comment: the peak is untouched when the re-tag leaves residency unchanged, which is the trait's clause and which the measurement after `assume` checks. If measurement is not added, the trust belongs in the backend module doc's accounting premises. Acceptance: the new control fails by name; `local_backend_conforms` and `materializing_backend_conforms` pass; the comment at 285-288 cites the trait clause it relies on.

Amendment (T6): the "if measurement is not added" alternative is withdrawn for `assume`. For `erase`, adding `measure_erased` to the crate-internal `Measure` trait is inside the ruling (the trait is not public); take that branch so both directions are checked.

### conformance-42 (low): ruling T6, amended

Resolution: Two changes, per the refutation's correction. Raise the dense ceiling (a few thousand bounds costs nothing, since `node_bytes` is pure arithmetic), which catches point dips. Add a `proptest!` in backend/tests.rs over `(fan in 0..=FAN, b1 in 0..=BOUND_SWEEP_CEILING, delta in 0..=BOUND_SWEEP_CEILING)` asserting `node_bytes(fan, b1) <= node_bytes(fan, b1.saturating_add(delta))` for `Local` and `Materializing`, with a step-shaped `PRICED_BOUND_STEP` knob (a header that shrinks above a threshold bound) and a control showing the grid sweep alone passes it while the property test fails it; commit the seed file the failing case writes. State in `node_bytes_monotone`'s doc that the grid is a sample and the property test covers the family. Acceptance: a step-shaped bound dip fails a committed test by name; a point dip at a non-grid bound below the raised dense ceiling fails the sweep; the sweep's doc states what it samples.

Amendment (T6): none beyond the class ruling; both changes land.

### tests-resource-link-window-19 (low): ruling T18

Resolution: Add `static CENSUS_LOCK: Mutex<()>` taken in every test body (the version-bound tests construct nodes too), mirroring decode_alloc's `metered`; reword the module doc to say the lock makes the suite runner-independent; hoist the differencing into `overhead` returning `(overhead, after)` and call it from the floor test. If the owner rules `cargo test` unsupported, keep the premise but drop "sound" for "correct under nextest's process-per-test model". Acceptance: every window_census test serializes on one lock (or the doc names the runner requirement as such); one site of the peak-differencing arithmetic.

Amendment (T18): the lock lands; the "if the owner rules `cargo test` unsupported" branch is closed.

### tests-resource-link-window-25 (low): ruling T28

Resolution: Add `assert!(measured >= 2, ...)` at both sites, stating the floor as the one request/response the transfer cannot avoid. Acceptance: both tests carry a floor; a `hops` returning 0 fails them.

### tests-resource-link-window-28 (low): ruling T28

Resolution: Between the two sessions assert `left.snapshot().len() > right.snapshot().len()` with a message naming it as the mid-session-growth witness (or `left.snapshot().latest() != left_result.converged`, since `converged` is the merged frontier before concurrent commits are joined, gossip.rs:810-814); finish with snapshot equality per finding 27; raise the budget past the pre-charge so the size-dependent derivation the doc describes actually runs, or restate the doc. Acceptance: the witness is committed and passes; awaiting `race` before the `join!` makes it fail.

The budget clause here is the same `window_corners.rs:174` measurement `tests-resource-link-window-20` performs; do it once. "Finding 27" is `tests-resource-link-window-27`, a P4 entry outside this lane: add the snapshot-equality finish only if it falls out of the witness for free; otherwise leave it and say so.

## Hazards and stops

- `LOCAL_BUDGET` is resized only after the committed floor has been seen to
  fail at 64 KiB; the commit history must show that order.
- `FAN_SLOT_BYTES` becoming `pub(crate)` and `Measure::measure_erased` are
  crate-internal changes and inside the ruling. Any change to a `pub`
  item reachable from outside the crate is a stop.
- The `Backend` trait docs' conviction sentences are restated only toward
  what the suite now checks; do not widen them.
- `conformance-24`'s ledger-surfacing change alters the `converged` panic
  message; if any committed test pins that message, that is a stop.
