<!-- CAVEAT LECTOR: maintained by the two triage coordinator sessions (Claude) for Finch as the current-state dashboard of the before and rumors triages; not authored, audited, or endorsed by Finch. Rewritten in place at every state change; it carries no history (the merge queue and git do). -->

# Triage landing status

Updated: 2026-09-03 02:26 UTC by the rumors session.

Ordered merge record and cross-plan rules: `merge-queue.md`. Lane states: not started, running, in review (fresh-eyes rounds or repairs), packet ready, merged, held (with the reason).

## At a glance

| Plan | Merged | Packet ready | In review | Running | Not started | Ledger rows pending |
|---|---|---|---|---|---|---|
| before | 2 of 37 lanes | 0 | 2 | 1 | 32 | 1186 of 1204 |
| rumors | 10 lanes | 1 | 1 | 3 | 25 briefed (P1 envelope, P2 peer, seven P3, fifteen P4, publication prep) | 874 of 995 |

## Waiting on Finch

- Review: the `p2-link` packet (branch `triage/p2-link`, packet commit `4a7baddb`, rendered HTML opened in the browser); merge on your word.
- Open stops: none from before. Rumors: seven P3 questions and twelve P4 items (`triage/briefs/README.md`, both "Open before launch" sections; each with a recommendation in the coordinator's message of 2026-09-03); the collision mode's four questions (T23) and the CI `PROPTEST_CASES` number arrive with their lanes' reports.

## Open items on main

- The default-features `just check` failure (proxy tests, bookmark_causality) is repaired at `9a7e898e`; `just check` reads green workspace-wide.
- The rumors CI job that runs the suites under the release profile with a large `PROPTEST_CASES` (T148) finds `before::meter tick_expand_cross_envelope` failing under release at any case count: the meter suite's pins are measured under the dev profile (debug assertions and overflow checks are part of the observer), so that job must not run the meter binary, or the suite must state and check the profile its pins hold under. Rumors owns the job's filter; before owns the suite's statement of its profile (carried to `p2-rows`). Failure text pending from the rumors lane.

## before

Lanes run in the order `triage/briefs/README.md` gives; every lane builds and gates on the illumos box; packets are reviewed in the browser and answered in the Markdown.

| Lane | State | Note |
|---|---|---|
| p1-proptest-cases | merged | `e27ba5e6` (ruling 109) |
| p1-harness | merged | `5b0a17d4` (rulings 4, 38, 50, 51, 108); the meter suite's one harness |
| p1-gate | running | ten commits landed on the merged rumors gate tip (retirements, lockfile audit and convergence, wasm32 leg in CI, writer-door law); final verification batch running, then it drops its moot bench-judge commit and gates |
| p2-widths | in review | two repair rounds landing (rulings 33, 34, 39, 41, 110); next: stack on main for its two memo-row re-pins, second fresh-eyes round, packet |
| p2-surface | in review | repair round landing (rulings 40, 42, 44, 46, 47, 48, 49, 98, 110); registry step held for a second launch after p1-suites; next: second fresh-eyes round, packet |
| p1-fuzz | not started | stacks on p1-gate when it lands |
| p1-survivors | not started | stacks on p1-gate after the roster retirement |
| p1-board | not started | after p1-harness (merged); next launch on the harness stack |
| p1-suites | not started | after p1-board |
| p2-rows | not started | after p1-suites; carries two handoffs from the harness review |
| p2-cures | not started | after p2-rows and p2-widths |
| p2-surface (registry) | not started | the held step 3 of p2-surface, after p1-suites |
| p3-vocabulary | not started | last of the sweeps |
| p4-ghosts, p4-structure, p4-rosters | not started | after the P1 and P2 lanes they follow |
| p5-scanners, p5-judge, p5-buffers | not started | after p1-gate, p1-fuzz, p2-surface |
| p7-api | not started | after p3 and p4 |
| p8-performance | not started | after p7 and the P2 lanes |
| p6 (16 module lanes) | not started | last; p6-harness first among them |

## rumors

Maintained by the rumors session.

| Lane | State | Note |
|---|---|---|
| p1-swarm, p1-conformance, p1-memwatch, p1-gate, p2-codec, p1-renderer, p1-harness-tests, p1-harness-crate, p2-walk, p1-causality | merged | see the queue's Merged section for shas and root files |
| p2-link | packet ready | rebased onto main (`7aa2b9a1`), tip compiled and its suites green on the box; packet `4a7baddb`; both stops ruled (T152, T156) |
| p2-commit-path | in review | all six entries landed, lane reported; rebased onto main (`3447206f`); acceptance verification and fresh-eyes round 1 running; p2-peer launches after it merges |
| p1-proptest-ci | running | no `cases` anywhere (T151), check workspace-wide; CI release-profile job with a measured `PROPTEST_CASES` (the number is a stop for Finch) |
| p2-vanish-liveness | running | T145 as widened by T154 |
| p1-collision-mode | running | T23; four open questions are stops for Finch |
| p1-envelope | not started | launchable from main now |
| p2-peer | not started | after p2-commit-path |
| p3-dashes, p3-imports, p3-lints, p3-modules, p3-prose-pass, p3-seeds, p3-vocabulary | not started | after every P1 and P2 lane merges; seven open questions for Finch |
| fifteen P4 lanes (`briefs/p4-*.md`) | not started | briefs committed at `46fb2cb6`; after P3; twelve open items for Finch |
| publication prep (T79) | not started | joint, last, after both triages' code lanes close |
