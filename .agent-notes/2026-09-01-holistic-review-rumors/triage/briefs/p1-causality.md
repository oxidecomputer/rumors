<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P1 lane: the bookmark causality harness

## Goal

`tests/bookmark_causality.rs` is the suite that shows a bookmarked peer
never recycles a version and never loses durable content across crashes
and rejoins. At the reviewed commit its recycle oracle flags only
`later <= earlier`, while the recycle the bookmark exists to prevent
produces an emission that compares `Greater` or incomparable, so a
`reclaim` mutant that re-admits every stored region on reboot deletes a
durable message fleet-wide and the suite passes (demonstrated). Its
harness folds every session error into a benign plan outcome, so 452
injected failures passed (demonstrated); its spawned sessions have no
stall bound, so a stall is a 180-second kill that loses the seed; and its
"fully deterministic" claim is false because network ids come from
`OsRng`. The invariant this lane restores: a harness that reads a failure
as success is itself a defect, and every claim the suite's docs make is
one its body checks.

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
  `<scratchpad>/p1-causality/`, polled with short foreground checks (the
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

1. `tests-bookmark-8` first (the seeded RNG): every later negative control
   wants a reproducible plan.
2. `tests-bookmark-11` next (the `join!` experiment): its outcome decides
   the shape of the session drivers that `-12`'s assertions attach to.
3. `tests-bookmark-12` before `tests-bookmark-9`: the oracle repair is
   observable only once the harness stops swallowing errors (TRIAGE.md,
   Dependencies).

All four are governed by ruling T8, which adopts each resolution as
stated.

## Members

### tests-bookmark-8 (medium): ruling T8

Resolution: thread a deterministic RNG through `World` (a `SmallRng` seeded per world, or a counter mapped through `seed_from_u64`) and replace the three `Peer::<Msg>::seed()` calls with `Peer::seed_rng(&mut rng)`; assert generated networks are pairwise distinct. Then the two reconstructed plans pin one path each and can assert which node re-bootstrapped. Calibrate the doc sentence for the `select!` bound ("deterministic up to tokio's `select!` branch order in the session internals") or add a cheap replay-identity check on a fixed plan using `common::fault::metered`'s `ByteMeter`. Acceptance: two runs of `reconstructed_cut_gossip_then_retire_under_bookmark_faults` resolve the mismatch with the same, asserted winner, and the doc claim at L46-56 is true as written.

`Peer::seed_rng` is `#[doc(hidden)] pub` today and usable from the suite; its gating is a P6 question (owner decision 19) and not this lane's. Report whether the schedule is then deterministic across two runs; that answer is one the review left unsettled.

### tests-bookmark-11 (medium): ruling T8

Resolution: first, one experiment: rewrite `World::gossip` as `block_on(join!(async move { let mut link = fault::faulty(side_a, fault_a); ra.gossip(&mut link).await }, async move { ... }))` under `common::wire::block_on` and run the two reconstructed tests plus a faulted proptest case. If the faulted side's EOF still reaches the survivor, drop `tokio::spawn` and `tokio_block_on` from causality (L506-523, L580-602, L672-694, L786-801) and transmit (the gated session becomes `join!(ga, gb, async { bm_a.entered().await; a.send(M1).unwrap(); bm_a.release() })`; `Notify` needs no runtime), which restores the stall detector without adding a timeout. If it does not, wrap each spawned pair in `tokio::time::timeout(SESSION_BUDGET, join!(...))` (the runtime is built with `enable_all()`) and panic naming the step. Move attach's `bootstrap_unbookmarked` onto the shared driver either way (finding 3). Acceptance: a planted never-completing gossip future in `bookmarking_never_recycles_a_version` fails the case inside the budget and writes a seed to proptest-regressions/bookmark_causality.txt; tests/bookmark_attach.rs imports `common::wire::block_on`, not `tokio_block_on`.

The last clause ("Move attach's `bootstrap_unbookmarked` onto the shared driver (finding 3)") depends on `tests-bookmark-3`, a P5 harness-lane entry that generalizes the shared drivers over `B: Bookmark`. Do not do it here; make `bookmark_attach.rs` import `common::wire::block_on` if that is possible without the generalized driver, and otherwise report that clause as open. Report the experiment's outcome either way, with the run that decided it.

### tests-bookmark-12 (high): ruling T8

Resolution: give `World` knowledge of the fault regime (a `reliable: bool` set by `single_network` and by `arb_plan`'s `faults` flag, or `FaultPlan::is_clean` plus an emptiness accessor on `FaultFeed`) and assert `Ok` for gossip, bootstrap-serve, bootstrap-join, and attach whenever the step cannot legitimately fail. Where faults are possible, extract the classifier from `retire` (L706-716) into one `fn assert_not_codec_bug(&Error<FlakyInMemoryBookmark>)` applied to every session result on both sides, extended to `Error::Bookmark(BookmarkIo::Format(_))`, `Error::PartyOverlap`, and `Error::Io` with `InvalidData`; apply it to `Retire::Recovered { error }` and `Retire::Uncertain { error }` too. Bind `serve_out.expect("bootstrap serve task")` so a server panic fails the test. Rewrite L585-587 to name the actual sources on a clean wire. Acceptance: a planted decode failure in the bootstrap serve path fails `bookmarking_prevents_party_leakage` at the offending step with the classified error, not at heal and not never; a deliberately injected `panic!` in a serve-side path fails `bookmarking_never_recycles_a_version` instead of leaving the node dormant; the two proptests and the reconstructed tests still pass unchanged.

Negative control: the second witness pass's injection (`evidence/witness.md`, section `tests-bookmark-12`: every third `World::gossip` returning `Err(Error::PartyOverlap)` under `run_reliable_plan`). Record the observed failure of the repaired suite under that injection in the commit message.

### tests-bookmark-9 (high): ruling T8

Resolution: (1) Track redactions: `World::redact` knows the version it redacts; record it in a per-network ledger (the shape tests/disruption.rs already uses). After `heal`, assert that every message live at some live peer of the winning network at heal start, and never redacted, is live at every peer. Scope the ledger to the winning network: `heal` collapses other universes by re-bootstrapping their members, discarding their content. Under a correct implementation a frontier dominates an emission only by having merged it or a redacter's frontier, so the check is sound; crash-lost content is excluded because it is not live at heal start. (2) Optionally strengthen `promote`: record the emitter's party alias (`dangerously_alias_party()`) on each `Emission` and flag `later.version / later.party <= earlier.version / earlier.party` whenever the two parties overlap; then drop the version-only argument at L26-28 and L96-98. (3) Commit the demonstration: the mutant below passes today's suite and must fail the strengthened one. Acceptance: with the mutant applied to `Bookmarked::reclaim`, `bookmarking_never_recycles_a_version` fails on a plan of the constructed shape; on HEAD both variants still pass across the full case count.

The mutant is in the entry's Construction line and in `evidence/witness.md`, section `tests-bookmark-9` (the reclaim alias pushed at `Version::new()` in `src/bookmark.rs`). Apply it as a reversible string swap, record the observed failure in the commit message, restore, and verify `git diff` on `src/` is empty. Part (2) is optional per the resolution; do it if part (1) is landed and the run count allows, and say which you did.

## Hazards and stops

- `src/bookmark.rs` is production code; the mutant is applied only in
  your worktree, only to demonstrate, and never committed. A commit
  touching `src/bookmark.rs` is a stop.
- If the `join!` experiment shows the faulted side's EOF does not reach the
  survivor, that is a finding about the fault injector, not only a reason
  to add a timeout; report it as such.
- If the seeded schedule is still nondeterministic across two runs after
  `-8`, report the two divergent outcomes verbatim; do not weaken the doc
  claim beyond what the resolution's calibration sentence says.
