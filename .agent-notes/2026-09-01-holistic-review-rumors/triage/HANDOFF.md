<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch and for the next coordinating session at the end of the 2026-09-02 session, as the state of the rumors triage's landing; not authored, audited, or endorsed by Finch. Read with the ground rules in ../../README.md. Everything here is also in git history, the ledger, the rulings, and the merge queue; this file is the map. -->

# Handoff: where the rumors triage stands

Read `WORKFLOW.md` first (the procedure), then `rulings.md` T135 and
later (this session's rulings), then this file.

## Merged to main (in order), each with ledger shas written

swarm (T27, T135), conformance (T6, T18, T139, T142), memwatch (T136),
gate (T7, T15, T16, T20, T26, T28, T29, T30, T147), codec (T33, T41),
renderer (T5, T9, T138, T140), harness-tests (T13, T21, T26, T28, T143,
T144, T146), harness-crate (T19, T22, T24, T26, T127), walk (T40),
causality (T8, T149). Ledger: 874 rows pending, 0 defects. The
`before` triage merged its proptest-cases and other lanes to the same
main; `.agent-notes/merge-queue.md` records every merge from both
sides with root files.

One check was in flight at handoff: a streaming-suite run on merged
main after the walk merge (four lanes rewrote the same files under
it; every added line of the walk's edits is present on main, verified;
the run confirms the merged tree passes). Its log:
`<scratchpad>/coordinator/verify-main-after-walk.log`; if it did not
complete, rerun `pset-run -n 40 -- cargo nextest run -p rumors
--all-features --locked -E 'test(tree::mirror::streaming) |
test(testing::) | binary(seed_liveness)'` on the box from a detached
scratch worktree at main.

## Lanes in flight (worktrees under /Users/oxide/src/, branches triage/*)

- `p2-link` (tip `81e7d822` on `32a1fa24`): final verification running
  in `rumors-verify-p2-link-2` (logs `coordinator/verify-link-2/`);
  both stops ruled (T152, T156); packet meta at
  `<scratchpad>/coordinator/p2-link-meta.md`, acceptance table filled,
  rounds filled. Next: fill the final-pass rows from the verifier's
  report, build the packet (`review.py packet --base 32a1fa24 --head
  81e7d822`), render, hand to Finch, merge on his word.
- `p2-commit-path`: three entries landed (T38, T39, api-core-10),
  T42's negative control in progress; status requested; verify,
  fresh-eyes, packet as WORKFLOW.md says. `p2-peer` launches after it
  merges (brief exists).
- `p1-proptest-ci` (on main after the gate merge): T148 as amended by
  T151 (no `cases` anywhere; check keys on `cases`/`with_cases` only,
  workspace-wide from the start since `before`'s sites are gone; CI
  release-profile job with a measured `PROPTEST_CASES`, the number a
  stop for Finch).
- `p2-vanish-liveness` (on the harness-tests tip `9c8ce16c`, now
  merged): T145 as widened by T154; the harness's `parked` count is
  its oracle. After its report, rebase its branch onto main
  (history-only; the trees match).
- `p1-collision-mode` (on main `d0dcb9f5`): T23; four open questions
  are stops for Finch.
- A drafter is writing the P4 lane briefs (359 rows) into `briefs/`
  and the README; commit them with explicit pathspecs when it reports.

## Not yet launched

`p1-envelope` (brief exists; after gate, which merged: launchable from
main now), `p2-peer` (after commit-path), the seven P3 lanes (briefs
committed; after every P1 and P2 lane merges; seven open questions in
`briefs/README.md`'s P3 section need Finch's rulings first), P4 through
P9 (P4 briefs in progress; P9 runs last and jointly with the `before`
triage, since it touches `crates/before`).

## Machine rules in force (also in WORKFLOW.md)

Gate of record on ox-east-1 under `pset-run -n 40 -- just gate`, at
most two psets per session; `fuzz` red alone is clean; no `just all`,
no `just ci`, no Mac gate; `before` carries an illumos-scoped clippy
allow; memwatch is gone; the box clock is stepped (chrony
`makestep 1.0 3`); verification runs in detached scratch worktrees,
never the lane's; annotation rows are checked at `-U3`; findings
without an entry go to `new-findings.md`. Liveness of an agent is
judged by transcript line count, never file mtime. Signing: the
1Password agent refuses while Finch is on the other account; commit
`--no-gpg-sign` then, and the merge rebase re-signs.

## Cross-session

The `before` triage's coordinator is reachable at
`uds:/tmp/cc-socks/90998.sock` (name rumors-74); rules in
`.agent-notes/merge-queue.md`'s header. Message it on every merge
(sha and root files) and on any ruling touching a shared instrument.
Its `before/p1-gate` stacks on the merged rumors gate.

## Defect on main, unrepaired at handoff (report first; Finch said no repair before compaction)

`cargo check --locked --workspace --all-targets` (default features, the
`just check` recipe) fails on main at
`src/tree/mirror/streaming/remote/proxy/tests.rs`: `LeftFailure`,
`RightFailure` (line 251), `PayloadCodec`, `PayloadDepthLimit` (265,
271) are used without imports. Found by the `before` coordinator on a
tree rebased onto main; confirmed by reading main: the walk lane's tip
`eb97720b` had `use crate::message::{PayloadCodec, PayloadDepthLimit};`
at line 3 of that file, and the merged file does not, so the walk's
rebase over the harness-crate rewrite (which replaced the file's `use`
block) dropped the walk's imports while keeping its code. The gate's
all-features legs passed on the merged tree because those names arrive
under a feature there. Repair: restore the two `use` lines the walk
tip carried (and the failure-type imports the harness-crate tip used
for `LeftFailure`/`RightFailure`, from its `harness` module), run `just
check` on the box, then the proxy suite under a pset, commit as a
merge-integration fix naming both lanes. Then add `cargo check
--workspace --all-targets` at default features to the gate's lint tier
if it is not there, so a default-feature import loss fails the gate
(`just check` is not a gate leg today). Record it in
`new-findings.md` with T-number when ruled.

## Open items for Finch

None in the decision queue at handoff. Stops will come from
`p1-collision-mode` (four questions), `p1-proptest-ci` (the CI case
count), and `p2-vanish-liveness` (the `PeerDeparted` error variant's
shape).
