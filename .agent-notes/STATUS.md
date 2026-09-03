<!-- CAVEAT LECTOR: maintained by the two triage coordinator sessions (Claude) for Finch as the current-state dashboard of the before and rumors triages; not authored, audited, or endorsed by Finch. Rewritten in place at every state change; it carries no history (the merge queue and git do). -->

# Triage landing status

Updated: 2026-09-03 03:39 UTC by the rumors session.

Ordered merge record and cross-plan rules: `merge-queue.md`. Lane states: not started, running, in review (fresh-eyes rounds or repairs), packet ready, merged, held (with the reason).

## At a glance

| Plan | Merged | Packet ready | In review | Running | Not started | Ledger rows pending |
|---|---|---|---|---|---|---|
| before | 2 of 38 lanes | 2 | 1 | 1 | 32 | 1185 of 1204 |
| rumors | 10 lanes | 0 | 3 | 2 | 25 briefed (P1 envelope, P2 peer, seven P3, fifteen P4, publication prep) | 874 of 995 |

## Waiting on Finch

- Nothing to review right now: the `p2-link` packet was withdrawn for re-scoping under T160 (per-link admission of recovered connections, no configurable pool bound).
- Open stops: none from before. Rumors: the P3 and P4 launch questions are ruled (T158, T159); the collision mode's four questions (T23) and the CI `PROPTEST_CASES` number arrive with their lanes' reports; the commit-path packet will carry three judgment calls from its fresh-eyes round (the ceiling on an all-skipped key, the unwind-path wording, one pin of a non-contract); the collision-mode lane's ten stops (variable name, one seed, `ci` tier, cluster weighting, the mark's name, the unreachable production-cfg test, three findings, and whether `test-collision` joins `ci` while red on the findings) are in the coordinator's message of 2026-09-03 and its packet.

## Open items on main

- Resolved (rumors): the reported party overlap was a torn sequential read by the dissolved harness's prober, not a library window; the exact plan is disjoint at every replica-observed instant (200 in-process runs) and the report's message is reproduced by construction. Record and follow-up: `triage/new-findings.md`.
- The default-features `just check` failure (proxy tests, bookmark_causality) is repaired at `9a7e898e`; `just check` reads green workspace-wide.
- The rumors CI job that runs the suites under the release profile with a large `PROPTEST_CASES` (T148) finds `before::meter tick_expand_cross_envelope` failing under release at any case count: the meter suite's pins are measured under the dev profile (debug assertions and overflow checks are part of the observer), so that job must not run the meter binary, or the suite must state and check the profile its pins hold under. Rumors' job excludes the meter binary with the reason stated; before's `p2-rows` makes the suite state and check its profile. Seven rows fail there, all on the limb tripwire (debug assertions are metered limb work); details in before's `triage/new-findings.md`.

- Unsigned commits on `main` ahead of `origin/main` (22: note-only commits made while the 1Password signing agent was refusing, e.g. `a21b7930`, `027d822e`, `bfa06203`, `1184006c`, `680c3e9a`, `f05de7ae`, ...). Lane commits are re-signed at their merge rebase; these need a rewrite of `main` before any push, which is Finch's (rule 5: announced in the queue first, the other session paused).

## before

Lanes run in the order `triage/briefs/README.md` gives; every lane builds and gates on the illumos box; packets are reviewed in the browser and answered in the Markdown.

| Lane | State | Note |
|---|---|---|
| p1-proptest-cases | merged | `e27ba5e6` (ruling 109) |
| p1-harness | merged | `5b0a17d4` (rulings 4, 38, 50, 51, 108); the meter suite's one harness |
| p1-gate | in review | repairs landed; the fuzzfit breach root-caused to the sweep lowering the guest's `bytes` (ruling 114: the convergence goes upward, root brought to 1.12.1 and the other swept crates likewise); the lane is bisecting to confirm, upgrading, and explaining the mechanism; then its gate and packet |
| p2-widths | packet ready | `ba36e970` (51 code commits; two items for Finch's eye inside, no stop) |
| p2-surface | packet ready | `cad0b75d` (44 code commits; one stop inside: the fuelscape.js caption) |
| p1-fuzz | not started | stacks on p1-gate when it lands |
| p1-survivors | not started | stacks on p1-gate after the roster retirement |
| p1-board | not started | after p1-harness (merged); next launch on the harness stack |
| p1-suites | not started | after p1-board |
| p2-rows | not started | after p1-suites; carries two handoffs from the harness review |
| p2-cures | not started | after p2-rows and p2-widths |
| p2-surface (registry) | not started | the held step 3 of p2-surface, after p1-suites |
| p2-generators | running | scope widened by ruling 113 (no strategy rejects: seven prop_filter sites); its packet is rebuilt afterward |
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
| p2-link | in review | re-scoped under T160: a successor agent replaces the endpoint-wide pool bound with per-link admission (one session complement per link); T44, T45, T156 commits stand; packet rebuilt after |
| p2-commit-path | in review | `9ca5e959` verified (every acceptance and control); fresh-eyes round 2 found platform-dependent allocation pins and test gaps, repair round 2 with the lane; packet after; p2-peer launches after it merges |
| p1-proptest-ci | in review | landed at `a34859ed` (sweep, caselint, release recipe and nextest profile, meter excluded, T157 rewrite); gate clean; the CI number is a stop (recommendation 4000 after p1-generators); verification and packet after p1-generators lands, since the number is set on that tree |
| p2-vanish-liveness | in review | landed at `28ef4341` (departure watch; vanish draw at every point; zero parks asserted), gate clean; six stops for Finch (`PeerDeparted` shape, a `link.rs` sentence, the dissolved park count, the deferred-watch deviation, two files outside its list, the point test's fixture); verification and fresh-eyes round 1 running |
| p1-collision-mode | in review | `7858b35e` verified; stops ruled (T162) and the schedule re-shaped to 28-byte prefixes (T163); fresh-eyes round 1 repairs with the lane |
| p2-deep-geometry | running | T162 items 8 and 9: the masked typed error and the non-reproducible poll count under deep geometry; stacked on the collision branch |
| p1-generators | running | T161: no rejecting strategies, zero reject budgets in the gate; stacked on p1-proptest-ci at `a34859ed` |
| p1-envelope | not started | launchable from main now |
| p2-peer | not started | after p2-commit-path |
| p3-dashes, p3-imports, p3-lints, p3-modules, p3-prose-pass, p3-seeds, p3-vocabulary | not started | after every P1 and P2 lane merges; seven open questions for Finch |
| fifteen P4 lanes (`briefs/p4-*.md`) | not started | briefs committed at `46fb2cb6`; after P3; twelve open items for Finch |
| publication prep (T79) | not started | joint, last, after both triages' code lanes close |
