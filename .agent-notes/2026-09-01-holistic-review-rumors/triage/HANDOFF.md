<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch and for the next coordinating session at the end of the 2026-09-03 session, as the state of the rumors triage's landing; not authored, audited, or endorsed by Finch. Read with the ground rules in ../../README.md. Current state lives in ../../STATUS.md; this file is the map of what to do next and where the evidence is. -->

# Handoff: where the rumors triage stands

Read `WORKFLOW.md` first (the procedure), then `rulings.md` T157 and
later (this session's rulings), then `.agent-notes/STATUS.md`, then
this file.

## Merged to main this session

The merge-integration repair `9a7e898e` (the walk's dropped helpers,
the causality suite's deleted `is_clean`), the prober-doc correction
`41129172`, and notes. No lane merged: Finch wound down before the
packets were ready. Ledger: 874 rows pending, 0 defects.

## Lanes with code landed, in the review order for tomorrow

Each has a packet meta under `<scratchpad>/coordinator/<lane>-meta.md`
(the scratchpad is session-local; if it is gone, the metas are
rebuilt from the briefs, the lane reports in the transcripts, and the
runner logs named below). Every tip compiled at both feature sets on
the box before it was offered. Build each packet with
`review.py packet --base <base> --head <tip>` from the lane worktree,
render with pandoc and `packet.css`, and open it for Finch; merge only
at his word, by rebase and sign, then compile the rebased tip before
the fast-forward (WORKFLOW.md's merge step).

1. **p2-link** (`/Users/oxide/src/rumors-p2-link`, `triage/p2-link`,
   base `7aa2b9a1`): T44, T45, T156, T160 (per-link admission of
   recovered connections, no configurable pool bound). Verification of
   `7a31b673` was running (`coordinator/verify-link-3/`); fresh-eyes
   round 1 found no bugs and one accounting defect, sent as repairs
   (decrement at header arrival; two tests; prose); a repair sha is
   expected. Stops for Finch: the release-on-link-end shape against
   per-peer pools (document; drop the release; or key pools by link),
   and the bound's justification wording.
2. **p2-commit-path** (`/Users/oxide/src/rumors-p2-commit-path`,
   base `main` at `030e1b5c`): T34, T36, T38, T39, T42; tip
   `71797970` after two repair rounds; re-verification and a light
   round-3 read were running (`coordinator/verify-commit-path/`,
   `fresh-eyes-commit-path-r3/`). Stop: the ceiling on an all-skipped
   key (recommendation: accept, pin whichever direction).
3. **p2-vanish-liveness** (`/Users/oxide/src/rumors-p2-vanish-liveness`,
   base `9c8ce16c`, rebase onto main is history-only): T145, T154;
   tip `28ef4341` verified (`coordinator/verify-vanish/`); round-1
   repairs sent (the watch kept alive after a control byte, abort on
   deadline, the floor's scope, prose); a repair sha and a second
   verification pass are expected. Six stops in the meta, including a
   `link.rs` sentence drafted for Finch's words.
4. **p1-collision-mode** (`/Users/oxide/src/rumors-p1-collision-mode`,
   base `d0dcb9f5`): T23, ruled by T162 and re-shaped by T163 (28-byte
   prefixes); tip `7858b35e` verified (`coordinator/verify-collision/`);
   round-1 repairs sent (T163, recipe liveness, `--no-fail-fast`,
   unmarking tests whose claims hold at any geometry, prose); a repair
   sha and a second verification pass are expected.
5. **p1-proptest-ci** (`/Users/oxide/src/rumors-p1-proptest-ci`, base
   `a07827ed`): T148, T151, T157; tip `a34859ed`; not yet verified by a
   runner. Stop: the CI case count (recommendation 4000 once
   `p1-generators` lands; 256 meanwhile).
6. **p1-generators** (`/Users/oxide/src/rumors-p1-generators`, stacked
   on `a34859ed`): T161; running at handoff (agent
   `a15e73d787b3d0171`, scratchpad `p1-generators/`).
7. **p2-deep-geometry** (`/Users/oxide/src/rumors-p2-deep-geometry`,
   stacked on the collision tip `7858b35e`): T162 items 8 and 9;
   running at handoff (agent `a2af8c0d37b66646f`). Its fix to the
   unbiased `select!` sites is production code; a change to error
   precedence or a contract is a stop.

## Running agents at handoff (their reports reach the successor)

p2-link successor (`a6b0b486905b78c33`, repairs), commit-path lane
(`aee05d3d797a0b84f`, idle after round 2), commit-path runner
(`a33b8f771988b88df`, re-verifying `71797970`), commit-path round-3
reader (`a7e6027188e6fd76c`), collision lane (`a47adb39f65478064`,
repairs), vanish lane (`a29e1e87156b5a036`, repairs), generators,
deep-geometry. Agent liveness is judged by transcript line count; a
parked agent is woken by SendMessage after its awaited log lands.

## Not launched

`p1-envelope` (T43; launchable from main; disjoint from every branch
in flight; asked of Finch for the overnight run), `p2-peer` (after
commit-path merges), the seven P3 lanes (after every P1 and P2 merge;
T158 rules their questions), the fifteen P4 lanes (T159), P5 on, P9
last with the `before` triage.

## Worktrees to retire (resolve the forge dir from inside first)

`rumors-verify-p2-commit-path`, `rumors-verify-collision`,
`rumors-verify-vanish`, `rumors-verify-link-3`,
`rumors-verify-overlap-main` (branch `triage/party-overlap`, evidence,
keep the branch), and `rumors-verify-overlap` (carries uncommitted
instrumentation edits saved as `coordinator/party-overlap/instrumentation-84c6b5c6.diff`;
commit or discard at Finch's word, never force-remove).

## Machine rules in force

Queue rule 6 as rewritten: every build and gate unbound with
`CARGO_BUILD_JOBS=32 NEXTEST_TEST_THREADS=32`; `pset-run` only for a
wall-time measurement, one at a time, announced; hold launches above
load 150. The box sync is a real checkout. `fuzz` red alone is clean.
The clean-file precondition for `STATUS.md` and `merge-queue.md`.
Signing: `--no-gpg-sign` when the agent refuses; the merge rebase
re-signs.

## Open for Finch, in one list

The p2-link merge after its packet; the CI case count; the stops
named per lane above; the `link.rs` sentence; whether a starved pool's
liveness is a contract claim (`new-findings.md`); the overnight scope.
