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
   done; round 2 and T167's shape (d) landed at `398b77f2` (adapter-owned
   per-link pooling; `Dial` is `dial` alone; `Config::pooling`; gate
   clean under the mutex; evidence `triage/notes/pooling-shape-d.md`).
   Final verification (runner `ad7cb42de7506727b`, `coordinator/verify-link-3/`)
   and the third fresh-eyes read (`af556f9cc68298f4f`) running; the
   commissioned sush patch agent (`a4e36f261a9b926d5`) drafts the diff
   and report under `coordinator/sush-eval/` (never applied to sush).
   Packet after; one stop left: the bound's justification wording.
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
   done; round 2 and T164's sweep recipe landed at `71d7a498` (unsigned;
   the gate's `wasm` stream red on before's fuel band with a committed
   seed, `75b17b36`, which merges after before's fuzz refit); third
   verification (runner `a5dbbd85bcd91a070`, `coordinator/verify-collision/`)
   and the final read (`a8a6207f6d25e4d01`) running; packet after. New
   findings: the pipelining hop budget fails under deep geometry (347
   hops against 24), a load-dependent residue seed (deep-geometry's fix),
   mutex fairness.
5. **p1-proptest-ci** (`/Users/oxide/src/rumors-p1-proptest-ci`, base
   `a07827ed`): T148, T151, T157; tip `a34859ed`; not yet verified by a
   runner. The CI count is ruled (T164 item 3): 256 until `p1-generators`
   merges, then 4000 as a one-line follow-up.
6. **p1-generators** (`/Users/oxide/src/rumors-p1-generators`, stacked
   on `a34859ed`): T161; landed at `9903be30` (six commits, unsigned;
   agent `a15e73d787b3d0171`, scratchpad `p1-generators/`); verification
   (runner `afb3290efc3f7875e`, `coordinator/verify-generators/`) and
   fresh-eyes round 1 (`a7cabd3f0b98e19ef`) done: repairs sent (per-rule
   lint roots, decoupled seeds promoted to unit regressions, the fold
   construction at minimum-1 sets, a tripwire liveness leg, three more
   spellings, prose); a repair sha and a second pass are expected;
   packet meta drafted.
   Merge order: its last commit (the zero budgets, `9903be30`) waits for
   `before/p2-generators`; the five sweep commits do not. Stops in the
   meta (fail-fast; the widened `partition.rs` population; the asserted
   chunk premise).
8. **p1-envelope** (`/Users/oxide/src/rumors-p1-envelope`, from main
   `0fad870c`): landed at `147993bd` (T10's exact-Chernoff differential
   tests, T43's deletion, T17's derivation tests; unsigned); verified in
   full (runner `adb7d46d8e13e197c`, `coordinator/verify-envelope/`);
   fresh-eyes round 1 (no bugs) sent repairs (a per-scale sweep of the
   8 KB figure, a string pin of the doc's figures, the lone-message cell,
   convergence asserted, prose); a repair sha and a second pass are
   expected; packet meta drafted. Stops: the `src/lib.rs` sentence
   (Finch's paragraph) and the figure's set size (`new-findings.md`).
7. **p2-deep-geometry** (`/Users/oxide/src/rumors-p2-deep-geometry`,
   stacked on the collision tip `7858b35e`): T162 items 8 and 9;
   packet ready: `f74324a7` on the branch, code tip `7e7bae71`, base
   `7858b35e` (the collision branch; at merge, rebase onto the merged
   collision tip and compile); three rounds, every item verified
   (runner `a6483291f13d44eb4`, `coordinator/verify-deep/`); its watcher
   runs `test-collision` and the gate under the mutex
   (`scratchpad/p2-deep-geometry/finalverify.status`, `ALLDONE`) and
   retires its box caches; append the gate verdict to the meta when it
   lands. Packet meta drafted with four stops (the
   shared predicate at integration; the duplicated-reply stall; the
   `materializing_backend_conforms` floor under the schedule; the
   residual precedence window, recommendation: close it).
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

## Resumed 2026-09-03 13:00 UTC after the credits outage

Every subagent died at 07:30 UTC on an account-level 429; all were resumed
at their last step. Since then: envelope round 1 landed (`efbf5338`,
gate 4 of record green but for the illumos fuzz leg) and vanish round 2
landed (`86cde61c`, gate green likewise); the p2-link round-3 read found
one design defect (the pool probe reads tokio's spent cooperative budget
as idle) and went to the lane as round 4 (`a6b0b486905b78c33`); the
collision final round is with its lane (`a47adb39f65478064`) with an
addendum (the fixture-window count is an exact pin, not a floor).
Running now: envelope verification (`abba4f7abb43f0d84`,
`coordinator/verify-envelope/r2/`) and read (`abf9c1f5e858939da`);
vanish verification (`aa22f906227c78b72`, `coordinator/verify-vanish/r3/`)
and read (`a62d194c21c91e85d`); the sush patch (`a4e36f261a9b926d5`);
generators' round-1 commit B and annotations (`a15e73d787b3d0171`). The
before session holds a third fuel-band seed (`cc a072497c…`).

## State at 2026-09-03 14:20 UTC

Packets: `p1-collision-mode` ready (`f6d9a0af`, three stops). Running:
p2-link round 5 (lane `a6b0b486905b78c33`; the budget test's inert
leg, a lost sentence, the `rt` comment; packet from its sha after the
runner re-checks the control); envelope round-2 verification
(`ad6bf77487cc23a1e`, `verify-envelope/r3/`) and the T168 grid on the
lane (`ad8747469f2495256`, cells built fresh); vanish round-3
verification (`a622615e162d51b98`, `verify-vanish/r4/`) and light read
(`acd87ebd67dc09435`), then its packet against stack parent `9c8ce16c`;
generators round 2 (`a15e73d787b3d0171`; budgets into `[env]`, the
tripwire asserting zero rejects, `crates/` under the full rules in the
budgets commit, 4000 folded in), then verification and a light read;
its packet against `a34859ed`. Done: p2-link round-4 verification
(`a35b83f0afbba6837`) and light read (`a81280fa719d14fe7`); the sush
patch at `locker` (`sush-eval/sush-shape-d-locker.patch`). Rulings
through T168.
