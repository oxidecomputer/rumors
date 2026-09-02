<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P1 lane: the integration-test harnesses

## Goal

The integration suites under `tests/` and the shared testing transport
are where cross-peer behavior is judged. The review found: an
inter-process disruption parent that never joins its serving tasks, so a
panic on a TCP serving path leaves the test passing (demonstrated); no pin
that redactions stay in the generated populations; an overlap-session
shadow with no validity meta-test (demonstrated); a reordering acceptor
whose inversion no test shows firing, duplicated in the link conformance
suite; disruption cut ranges with no pin; and a poll-budget verdict no
test demonstrates. The invariant restored: a harness fails on the failure
it exists to catch, and every generated population is pinned live in each
dimension the suite claims to sample.

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
  `<scratchpad>/p1-harness-tests/`, polled with short foreground checks
  (the foreground command cap is ten minutes). Keep every working file
  under that directory. The disruption binary spawns child processes and
  is slow; run single tests by filter during iteration.
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

## Members

### tests-disruption-handshake-7 (high): ruling T26

Resolution: Hold the JoinSet outside the accept task (or have the task return it), and after the children are reaped and before reclaiming the Peers drain it: `while let Some(result) = sessions.join_next().await { result.expect("serving session task"); }`. Every serving task has completed by then (every child has exited), so the drain is immediate. End the accept loop by closing the listener rather than by abort, or check `JoinError::is_panic` on the aborted task's result. Acceptance: with `panic!("probe")` inserted as the first statement of the serving task body in a scratch copy, inter_process_disruption_upholds_party_invariants and each reconstructed_* test fail; today they pass.

The witness pass corrected the construction (`evidence/witness.md`,
section `tests-disruption-handshake-7`): a panic as the task's first
statement fails for an unrelated reason; the demonstrating probe is a
panic on the serving path after the port swap. Use that variant as the
negative control and record its output.

### tests-lifecycle-15 (medium): ruling T26

Resolution: Add deterministic-runner population tests mirroring `membership_population_contains_churn`: sample `arb_local_actions()` N times and assert some sample contains a `Redact` positioned after at least one `Insert`; sample `arb_schedule(any::<u64>(), N_PEERS, MAX_EVENTS)` and assert some emitted schedule contains an `Event::Redact`. Place them beside the strategies' consumers (pairwise.rs and multi_peer.rs) or in one `tests/population_liveness.rs`. Acceptance: setting the `Redact` weight to 0 in `arb_actions` or in `arb_choice` fails the new pin while the property suites otherwise still pass (the negative control demonstrates the pin fires).

### tests-observation-28 (medium) and tests-common-12 (low): ruling T13, resolutions amended

tests-observation-28, Resolution: (1) Expose `arb_overlap_schedule_with_shadow` returning the final `Knowledge` and add the overlap twin of `shadow_predicts_live_state` to this file. (2) Decide, with that test in hand, whether the shadow should snapshot at the first `Step` (the real fork point) rather than at `Open`, and fix the `Open` doc to say when the fork happens. (3) Only after the fork point matches the code, change the executor's guard to an assertion so a shadow mismatch fails instead of skipping. Acceptance: a committed test compares the overlap shadow's per-peer `observed_log`/`live` to the executor's; the executor asserts rather than skips; the `Open` doc states the fork point accurately.

tests-common-12, Resolution: (1) restate at 17-25, 178-179, and 562-564 that the shadow models the fork at `Open` while the live session forks at its first poll, and that the guard exists because the two differ; replace the guard comment's borrowed justification (there is no gossip filter here) with that one. (2) Return the skipped-`Redact` count from `execute_overlap_and_quiesce` and pin a ceiling in tests/session_overlap.rs (a fraction of emitted `Redact`s over the run), with a committed demonstration that a broken `merge_session` (skipping the `Close` merge) exceeds it. Acceptance: the docs describe the model as a model; the suite fails when the skip ceiling is exceeded and still passes at HEAD.

Ruled (T13): only part (1) of each. The meta-test lands; the docs are
restated as tests-common-12 (1) says; the guard stays a guard, the fork
point stays at `Open`, no skip count, no ceiling. Amended acceptance: a
committed test compares the overlap shadow's per-peer `observed_log` and
`live` to the executor's; the docs describe the Open-time fork as the
model and the guard as its consequence. Negative control: the witness
pass's no-op `Close` merge (`evidence/witness.md`, section
`tests-observation-28`) must fail the meta-test; record the output. Note
that the meta-test compares the shadow to the live state at the end of the
schedule, and the shadow's Open-time fork is a known imprecision: if the
meta-test fails on HEAD because of that imprecision rather than a defect,
that is a stop with the failing case, not a reason to loosen the
comparison.

### testing-infra-12 (medium): ruling T21

Resolution: (a) Add a unit test in transport.rs's tests: build a `memory()` pair, wrap one acceptor with `reorder_accepts(link, 2, counter)`, and under `run_to_quiescence` `join!` a task that connects two streams with a task that accepts twice; assert the release order is reversed and the counter reads 1 (the second connect lands during the acceptor's `yield_once`, so the patience loop is exercised, not merely present). Rewrite proxy/tests.rs:513-515 and transport.rs:667-669 to cite that witness and to describe the two acceptors as deliberately different. (b) Alternatively dissolve `ReorderingAcceptor`, `reorder_accepts`, `yield_once`, `REORDER_PATIENCE`, and `reconcile_symmetric_accepts_reordered`, leaving the conformance suite's link-level inversion proof. Recommendation: (a). Acceptance: a committed test asserts `reordered > 0` for `ReorderingAcceptor` with a batch of two released newest-first, or the decorator and its consumer are gone; no doc claims the conformance tests prove this implementation.

Ruled (T21): option (a); the proxy-tier `== 0` assertion stays. The
"describe the two acceptors as deliberately different" clause is
superseded by `conformance-18` below, which unifies them.

### conformance-18 (low): ruling T21

Resolution: Delete `ReversingAcceptor` and `reversing`; in `reordered_accepts_conform` and `reordering_acceptor_passes_independence` call `crate::testing::reorder_accepts(a, 3, counter.clone())`, confirming `reordered > 0` still holds and the negative controls keep their verdicts under the patient wait. Then restate the transport.rs sibling comment without the reference (outside this partition). If the owner keeps two, the comment at transport.rs:667-669 should say why they differ (Ready-only drain for concurrently connected probes, patient wait for the proxy topology). Acceptance: `grep -rn ReversingAcceptor src` is empty and the two tests pass with nonzero `reordered`, or the rationale for two fixtures is stated where the duplicate is declared.

Ruled (T21): unify, provided the conformance verdicts hold under the
patient wait. If they do not, that is a finding: stop with the failing
verdict, and do not keep the duplicate silently.

### tests-disruption-handshake-3 (low): ruling T28

Resolution: Meter one clean child cycle (bootstrap, session, final gossip, retire) against the parent through `fault::metered` over TCP or the in-memory link, and pin a named inter-process bound from both sides as max_cut_spans_the_envelope_session does; draw arb_child_plan's cuts from that bound. Acceptance: a named constant and a two-sided pin exist for the inter-process family, and arb_child_plan draws from it.

The metered number is one you produce; record the run in the commit
message.

### tests-disruption-handshake-17 (low): ruling T28

Resolution: Name the range as a constant and pin it against a metered clean run of the same `pair()` fixture from both sides (as max_cut_spans_the_envelope_session does), or draw the cut as a fraction of the metered extent; add read cuts to the family (`read_cut: Some(..)` on either side) so the certification branch's asymmetric case is reachable, and let the doc claim only what the family can produce. Acceptance: a named constant with a committed two-sided pin replaces the literal 400; a generated or deterministic case yields Ok on one driver and Err on the other and passes the certification assertions.

### testing-infra-4 (low): ruling T28

Resolution: Add to the poller's tests: `assert_eq!(run_to_quiescence(std::future::poll_fn(|cx: &mut Context<'_>| { cx.waker().wake_by_ref(); Poll::<()>::Pending })), Err(Quiescence::PollBudget));` (a million trivial polls completes in milliseconds). While there, give `MAX_POLLS` a one-line sizing statement (the largest closed-world session the suites run, with headroom), since a legitimate long session exceeding it would be misreported. Acceptance: a test asserts `Err(Quiescence::PollBudget)` for a perpetually self-waking future.

## Hazards and stops

- Widening the disruption families (read cuts, a new inter-process
  range) changes what the proptests generate; existing committed seeds
  must still replay (`tests/seed_liveness.rs` holds them to their paths)
  and any new seed is committed.
- The tests-harness lane and the tests lane in P5 both want the shared
  drivers generalized (`tests-bookmark-3`, `tests-lifecycle-3`,
  `tests-common-30`); none of that is in this lane. Do not consolidate
  drivers here.
- `src/testing/transport.rs` and `src/conformance/link/tests.rs` are the
  only `src/` files this lane opens; anything else under `src/` is a stop.
