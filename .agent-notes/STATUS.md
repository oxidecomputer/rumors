<!-- CAVEAT LECTOR: maintained by the two triage coordinator sessions (Claude) for Finch as the current-state dashboard of the before and rumors triages; not authored, audited, or endorsed by Finch. Rewritten in place at every state change; it carries no history (the merge queue and git do). -->

# Triage landing status

Updated: 2026-09-03 02:05 UTC by the before session (rumors-74).

Ordered merge record and cross-plan rules: `merge-queue.md`. Lane states: not started, running, in review (fresh-eyes rounds or repairs), packet ready, merged, held (with the reason).

## At a glance

| Plan | Merged | Packet ready | In review | Running | Not started | Ledger rows pending |
|---|---|---|---|---|---|---|
| before | 2 of 37 lanes | 0 | 2 | 1 | 32 | 1186 of 1204 |
| rumors | 10 lanes | 0 | 2 | 4 | 10 briefed (P1 envelope, P2 peer, seven P3, publication prep) plus the P4 lanes being drafted | 874 of 995 |

## Waiting on Finch

- Nothing to review right now.
- Open stops: none from before. Rumors: the collision mode's four open questions (T23); the CI `PROPTEST_CASES` number (p1-proptest-ci); seven P3 questions (see the rumors handoff).

## Open items on main

- `cargo check --locked --workspace --all-targets` (`just check`, default features) fails on main in two places: `src/tree/mirror/streaming/remote/proxy/tests.rs` (the walk lane's rebase over the harness-crate rewrite dropped its imports, its `failing_root` helper, and its failure aliases) and `tests/bookmark_causality.rs` (calls `FaultPlan::is_clean`, deleted by the harness-tests lane before the causality lane rebased). Rumors owns the repair, verifying on the box now; the sha is announced when it lands. Both plans' merge steps now compile the rebased tip (default and all features) before any fast-forward. Workspace-wide `just check` and `just test` read red until the repair lands; before's lanes verify with before-only invocations meanwhile.

## before

Lanes run in the order `triage/briefs/README.md` gives; every lane builds and gates on the illumos box; packets are reviewed in the browser and answered in the Markdown.

| Lane | State | Note |
|---|---|---|
| p1-proptest-cases | merged | `e27ba5e6` (ruling 109) |
| p1-harness | merged | `5b0a17d4` (rulings 4, 38, 50, 51, 108); the meter suite's one harness |
| p1-gate | running | stacked on the merged rumors gate lane; coverage reproduction, mutants and workflowlint retirement, lockfile audit, wasm32 leg in CI |
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

Drafted by the before session from the rumors HANDOFF.md at `cb79d712`; the rumors session corrects and maintains this section.

| Lane | State | Note |
|---|---|---|
| p1-swarm, p1-conformance, p1-memwatch, p1-gate, p2-codec, p1-renderer, p1-harness-tests, p1-harness-crate, p2-walk, p1-causality | merged | see the queue's Merged section for shas and root files |
| p2-link | in review | final verification running; both stops ruled (T152, T156); packet next |
| p2-commit-path | in review | three entries landed, T42's negative control in progress; p2-peer launches after it merges |
| p1-proptest-ci | running | no `cases` anywhere (T151), check workspace-wide; CI release-profile job with a measured `PROPTEST_CASES` (the number is a stop for Finch) |
| p2-vanish-liveness | running | T145 as widened by T154 |
| p1-collision-mode | running | T23; four open questions are stops for Finch |
| P4 briefs | running | a drafter is writing them (359 rows) |
| p1-envelope | not started | launchable from main now |
| p2-peer | not started | after p2-commit-path |
| p3-dashes, p3-imports, p3-lints, p3-modules, p3-prose-pass, p3-seeds, p3-vocabulary | not started | after every P1 and P2 lane merges; seven open questions for Finch |
| publication prep (T79) | not started | joint, last, after both triages' code lanes close |
