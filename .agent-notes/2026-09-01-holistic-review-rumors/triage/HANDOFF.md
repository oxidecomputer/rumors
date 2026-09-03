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
   `7a31b673` holds in full (`coordinator/verify-link-3/`); fresh-eyes
   round 1 found no bugs and one accounting defect, sent as repairs
   (decrement at header arrival; two tests; prose); landed at
   `db616f70` (unsigned from `f3105224` on); final verification (runner
   `ad7cb42de7506727b`) and fresh-eyes round 2 (`a8b49e3a5d4f5745c`)
   done; round-2 repairs with the lane, then T167's re-scope on top
   (adapter-owned per-link pooling; `Dial` is `dial` alone; evidence in
   `triage/notes/pooling-shape-d.md`). When that sha lands: launch the
   commissioned sush patch agent (a draft diff against
   `oxidecomputer/sush` compiled on the box against the branch, with
   the reasoning versus qorb and tests; delivered beside the packet,
   never applied to sush), then the final verification and the packet.
   One stop left: the bound's justification wording.
2. **p2-commit-path** (`/Users/oxide/src/rumors-p2-commit-path`,
   base `main` at `030e1b5c`): T34, T36, T38, T39, T42; tip
   packet ready: `14d0d5fb` on the branch, code tip `5a4559e1` (T166
   landed on top of the first packet build), rebased
   onto main `1336daa0` (lane diff identical by patch-id), compiled and
   spot-tested on the box (`coordinator/verify-commit-path/41-*`); three
   fresh-eyes rounds, every item landed and verified (runner
   `a33b8f771988b88df`); the ceiling stop is ruled (T166) and landed.
   Merge at Finch's word: rebase-and-sign, compile the tip, fast-forward, ledger shas
   for T34, T36, T38, T39, T42's entries.
3. **p2-vanish-liveness** (`/Users/oxide/src/rumors-p2-vanish-liveness`,
   base `9c8ce16c`, rebase onto main is history-only): T145, T154;
   tip `28ef4341` verified (`coordinator/verify-vanish/`); round 1
   landed at `e3c5f986` (three of its five commits unsigned by the lock
   protocol), verified in full; fresh-eyes round 2 found one behavioral
   regression (the departure race dropped a delivered stream mid-label)
   and gaps, sent as the round-2 repairs (lane `a29e1e87156b5a036`); a
   repair sha, a third verification pass, and a light round-3 read are
   expected; packet after. The precedence rule is the stop that
   matters (meta stop 7). Six stops in the meta, including a
   `link.rs` sentence drafted for Finch's words.
4. **p1-collision-mode** (`/Users/oxide/src/rumors-p1-collision-mode`,
   base `d0dcb9f5`): T23, ruled by T162 and re-shaped by T163 (28-byte
   prefixes); round 1 landed at `e951b81e` (commits unsigned: the
   signing agent was locked; the merge rebase re-signs); second
   verification (runner `a5dbbd85bcd91a070`) and fresh-eyes round 2
   (`ab9be4c4467ec3094`) running; the second pass verified in full
   (`coordinator/verify-collision/r2/`). T164 item 2 (a hand-run sweep
   recipe with a 31-byte seed) is with the lane; a new stall
   (`duplicated_reply_is_rejected_as_unasked`) joins the deep-geometry
   lane's roster (that lane was told).
5. **p1-proptest-ci** (`/Users/oxide/src/rumors-p1-proptest-ci`, base
   `a07827ed`): T148, T151, T157; tip `a34859ed`; not yet verified by a
   runner. The CI count is ruled (T164 item 3): 256 until `p1-generators`
   merges, then 4000 as a one-line follow-up.
6. **p1-generators** (`/Users/oxide/src/rumors-p1-generators`, stacked
   on `a34859ed`): T161; running at handoff (agent
   `a15e73d787b3d0171`, scratchpad `p1-generators/`).
8. **p1-envelope** (`/Users/oxide/src/rumors-p1-envelope`, from main
   `0fad870c`): T10 then T43; running at handoff (launched at Finch's
   word; agent `ad8747469f2495256`; scratchpad `p1-envelope/`).
7. **p2-deep-geometry** (`/Users/oxide/src/rumors-p2-deep-geometry`,
   stacked on the collision tip `7858b35e`): T162 items 8 and 9;
   round 1 landed at `005bd86e` (agent `a2af8c0d37b66646f`; both
   anomalies closed with mechanisms; every decode error surfaces; two
   same-wave tests; the bias orders argued); verification (runner
   `a6483291f13d44eb4`, `coordinator/verify-deep/`) and fresh-eyes
   round 2 (`af2c60311410ae869`) running; packet meta drafted.
   Its `execute` edit and the vanish lane's T165 predicate meet at merge
   in `remote/proxy/work.rs`; integrate toward one predicate (only
   supply-caused symptoms are outranked). Its watcher (local pid 74988)
   runs `just test-collision` then `just gate` under the mutex when the
   box clears, writing `scratchpad/p2-deep-geometry/finalverify.status`
   (`ALLDONE` when both finish). The duplicated-reply stall at 28 bytes
   is the next investigation (`new-findings.md`).

## Running agents at handoff (their reports reach the successor)

p2-link successor (`a6b0b486905b78c33`, repairs), commit-path lane
(`aee05d3d797a0b84f`, idle after round 2), commit-path runner
(`a33b8f771988b88df`, re-verifying `71797970`), commit-path round-3
reader (`a7e6027188e6fd76c`), collision lane (`a47adb39f65478064`,
repairs), vanish lane (`a29e1e87156b5a036`, repairs), generators,
deep-geometry. Agent liveness is judged by transcript line count; a
parked agent is woken by SendMessage after its awaited log lands.

## Not launched

`p2-peer` (after
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

## Resume protocol (for a successor with no memory of this session)

The coordinator runs unattended overnight and auto-compacts; this file
and `STATUS.md` are the memory. On resume, in order:

1. Read `WORKFLOW.md`, `rulings.md` from T157, `../../STATUS.md`, this
   file, `new-findings.md`. Then `git -C /Users/oxide/src/rumors log
   --oneline -15` and `git worktree list`: a worktree that exists is a
   lane in flight or awaiting review; never create a second for the
   same lane, and never launch a lane whose brief the "Not launched"
   list holds without Finch's word.
2. For each agent named above, judge liveness by the line count of
   `/private/tmp/claude-506/-Users-oxide-src-rumors/3b30c9cc-2928-4cc5-ba62-14f8147fa4c3/tasks/<id>.output`
   across two samples a minute apart, never by mtime; a report that
   has already landed is in the transcript's task notifications and,
   once captured, in `STATUS.md`. An agent parked on its own watcher is
   woken by `SendMessage` after the log it awaits has landed (its logs
   are under the scratchpad directory its brief names).
3. Capture before acting: every report goes into `STATUS.md` (the
   lane's row), `new-findings.md` (findings without an entry), the
   packet meta (acceptance rows only from a runner's own output), and
   this file's lane entry, in one pathspec commit, before any
   follow-up is issued. Follow-ups allowed unattended: a verification
   runner, a fresh-eyes round (at most three per lane), a repair round
   to the lane agent. Never unattended: a merge, a rewrite of `main`, a
   new lane, a ruling, a snapshot or pin re-accept, a public-surface
   change no ruling names, a shared root-file edit outside a lane's
   last commit.
4. The scratchpad directory is session-local and may be gone; every
   packet meta is reconstructible from the brief, the lane's report in
   the transcript, and the runner logs; every verdict that matters is
   also in the commit messages on the lane branches.
5. The `before` session runs the same way overnight; its address is
   `uds:/tmp/cc-socks/90998.sock` (or its successor's, in
   `merge-queue.md`'s header); message it on any root-file edit or
   shared-instrument change, and read `STATUS.md`'s before section
   before touching a shared file.

## Open for Finch, in one list

The p2-link merge after its packet; the CI case count; the stops
named per lane above; the `link.rs` sentence; whether a starved pool's
liveness is a contract claim (`new-findings.md`); the overnight scope.
