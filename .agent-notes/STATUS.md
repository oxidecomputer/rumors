<!-- CAVEAT LECTOR: maintained by the two triage coordinator sessions (Claude) for Finch as the current-state dashboard of the before and rumors triages; not authored, audited, or endorsed by Finch. Rewritten in place at every state change; it carries no history (the merge queue and git do). -->

# Triage landing status

Updated: 2026-09-03 13:15 UTC by the rumors session.

Ordered merge record and cross-plan rules: `merge-queue.md`. Lane states: not started, running, in review (fresh-eyes rounds or repairs), packet ready, merged, held (with the reason).

Note: every rumors subagent died at 07:30 UTC on an account-level credits 429; all were resumed at their last step at 13:00 UTC after Finch's login. Judge any report against its log, not its timestamp.

## At a glance

| Plan | Merged | Packet ready | In review | Running | Not started | Ledger rows pending |
|---|---|---|---|---|---|---|
| before | 2 of 39 lanes | 6 | 1 | 1 | 29 | 1185 of 1204 |
| rumors | 10 lanes | 0 | 3 | 2 | 25 briefed (P1 envelope, P2 peer, seven P3, fifteen P4, publication prep) | 874 of 995 |

## Waiting on Finch

- Review: the `p2-commit-path` packet (branch `triage/p2-commit-path`, packet `14d0d5fb`; no open stop) and the `p2-deep-geometry` packet (branch `triage/p2-deep-geometry`, packet `f74324a7`; stacked on collision-mode, so it merges after that lane; four stops in its packet).
- Open stops from before: the gate lane's syn 2/3 holdout roster (in its packet); the surface lane's fuelscape.js caption (in its packet); the board lane's ruling-11 residual fit (its journal's morning brief, stop 3). Rumors: T158, T159, T162, T163, T164 rule the launch questions, the collision mode's stops, and the CI count (256 until generators, then 4000); the commit-path packet will carry three judgment calls from its fresh-eyes round (the ceiling on an all-skipped key, the unwind-path wording, one pin of a non-contract); the collision-mode lane's ten stops (variable name, one seed, `ci` tier, cluster weighting, the mark's name, the unreachable production-cfg test, three findings, and whether `test-collision` joins `ci` while red on the findings) are in the coordinator's message of 2026-09-03 and its packet.

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
| p1-gate | packet ready | `75e0bc91` (37 commits; one stop inside: the syn 2/3 holdout roster; its gate red on audit and wasm by construction until the stop is ruled and p1-fuzz refits) |
| p2-widths | packet ready | `ba36e970` (51 code commits; two items for Finch's eye inside, no stop) |
| p2-surface | packet ready | `cad0b75d` (44 code commits; one stop inside: the fuelscape.js caption) |
| p1-fuzz | in review | `8073fada` (child of p8-tagwalk): twelve sentry seeds (the twelfth from the rumors vanish-liveness gate replays green inside the refit band); coordinator acceptance green at the tip; packet meta drafted; fresh-eyes round 1 running; stops: `REFIT_TOLERANCE`, the bimodal `ff_party_without` arm, the `ff_version_eq` routing, skyline-query-9 |
| p1-survivors | packet ready | `f9ec31d7` (child of p1-gate; rulings 18, 50, 88) |
| p1-board | in review | `00948f7b` (37 commits on main): coordinator acceptance at the tip green but for the span_dominance worst-case pin drift (a stop, deliberately not re-pinned); fresh-eyes round 3 running; packet after; stops: ruling 11's residual fit, the dominance re-pin, the shard tag bump v5 to v6 |
| p1-suites | not started | after p1-board |
| p2-rows | not started | after p1-suites; carries two handoffs from the harness review |
| p2-cures | not started | after p2-rows and p2-widths |
| p2-surface (registry) | not started | the held step 3 of p2-surface, after p1-suites |
| p2-generators | packet ready | `e84a74a3`; no rejecting strategy remains in before (rulings 111, 113) |
| p3-vocabulary | not started | last of the sweeps |
| p4-ghosts, p4-structure, p4-rosters | not started | after the P1 and P2 lanes they follow |
| p5-scanners, p5-judge, p5-buffers | not started | after p1-gate, p1-fuzz, p2-surface |
| p7-api | not started | after p3 and p4 |
| p8-tagwalk | packet ready | `3316c4ed` (child of p1-gate; fork kernels about 18% less fuel per bit, no reading moved) |
| p8-performance | not started | after p7 and the P2 lanes |
| p6 (16 module lanes) | not started | last; p6-harness first among them |

## rumors

Maintained by the rumors session.

| Lane | State | Note |
|---|---|---|
| p1-swarm, p1-conformance, p1-memwatch, p1-gate, p2-codec, p1-renderer, p1-harness-tests, p1-harness-crate, p2-walk, p1-causality | merged | see the queue's Merged section for shas and root files |
| p2-link | in review | shape (d) at `398b77f2` verified in full; round 4 (the pool probe under tokio's spent budget, coverage, prose) with the lane; the sush patch is drafted and probe-verified (`coordinator/sush-eval/`), with a premise finding for Finch: sush's whole crate cannot compile against current rumors for reasons outside the link (the payload API's serde move, 610 commits of drift) |
| p2-commit-path | packet ready | packet `14d0d5fb` on `triage/p2-commit-path` (code tip `5a4559e1`, base `1336daa0`, lane diff identical across the rebase); every acceptance and control run by the verifier; three fresh-eyes rounds; T166 landed; no open stop; commits unsigned by the lock protocol, re-signed at merge |
| p1-proptest-ci | in review | landed at `a34859ed` (sweep, caselint, release recipe and nextest profile, meter excluded, T157 rewrite); gate clean; the CI number is a stop (recommendation 4000 after p1-generators); verification and packet after p1-generators lands, since the number is set on that tree |
| p2-vanish-liveness | in review | round 2 at `86cde61c` (accept-only race, O(1) lookahead probe, T165's shared precedence shape, T166's `link.rs` paragraph); verification (62 lane tests pass so far; controls running) and the round-3 read running; packet after |
| p1-collision-mode | in review | `71d7a498` verified in a third pass (all ten items hold; the recipe fails exactly the lane's three); the final legibility round is with the lane; packet after; its `wasm` stream red on before's fuel band until before's refit merges |
| p2-deep-geometry | packet ready | packet `f74324a7` on `triage/p2-deep-geometry` (code tip `7e7bae71`, base `7858b35e`, stacked on the collision branch; merges after it); three fresh-eyes rounds, every item verified; its gate runs under the mutex and the verdict is appended when it lands; stops: the shared predicate at integration with vanish-liveness, the conformance floor under the schedule, the residual precedence window, the duplicated-reply stall queued |
| p1-generators | in review | `9903be30` verified so far (552 passed under zero budgets; checks clean); fresh-eyes round 1 found committed seeds decoupled from their counterexamples and a lint-roots regression, sent as repairs; packet after; its last commit merges after `before/p2-generators` |
| p1-envelope | in review | round 1 at `efbf5338` verified in full (six exact cells 2^8..2^16; the two-message line crosses between 2^15 and 2^16; both constant controls and the doc-figure control fail by name); the round-2 read running; packet after; the `src/lib.rs` regime sentence is Finch's to restate |
| p2-peer | not started | after p2-commit-path |
| p3-dashes, p3-imports, p3-lints, p3-modules, p3-prose-pass, p3-seeds, p3-vocabulary | not started | after every P1 and P2 lane merges; seven open questions for Finch |
| fifteen P4 lanes (`briefs/p4-*.md`) | not started | briefs committed at `46fb2cb6`; after P3; twelve open items for Finch |
| publication prep (T79) | not started | joint, last, after both triages' code lanes close |
