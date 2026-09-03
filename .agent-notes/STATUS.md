<!-- CAVEAT LECTOR: maintained by the two triage coordinator sessions (Claude) for Finch as the current-state dashboard of the before and rumors triages; not authored, audited, or endorsed by Finch. Rewritten in place at every state change; it carries no history (the merge queue and git do). -->

# Triage landing status

Updated: 2026-09-03 05:37 UTC by the before session (rumors-74), running overnight.

Ordered merge record and cross-plan rules: `merge-queue.md`. Lane states: not started, running, in review (fresh-eyes rounds or repairs), packet ready, merged, held (with the reason).

## At a glance

| Plan | Merged | Packet ready | In review | Running | Not started | Ledger rows pending |
|---|---|---|---|---|---|---|
| before | 2 of 39 lanes | 6 | 0 | 2 | 29 | 1185 of 1204 |
| rumors | 10 lanes | 0 | 3 | 2 | 25 briefed (P1 envelope, P2 peer, seven P3, fifteen P4, publication prep) | 874 of 995 |

## Waiting on Finch

- Review: the `p2-commit-path` packet (branch `triage/p2-commit-path`, packet commit `14d0d5fb`); no open stop; merge on your word.
- Open stops from before: the gate lane's syn 2/3 holdout roster (in its packet); the surface lane's fuelscape.js caption (in its packet). Rumors: T158, T159, T162, T163, T164 rule the launch questions, the collision mode's stops, and the CI count (256 until generators, then 4000); the commit-path packet will carry three judgment calls from its fresh-eyes round (the ceiling on an all-skipped key, the unwind-path wording, one pin of a non-contract); the collision-mode lane's ten stops (variable name, one seed, `ci` tier, cluster weighting, the mark's name, the unreachable production-cfg test, three findings, and whether `test-collision` joins `ci` while red on the findings) are in the coordinator's message of 2026-09-03 and its packet.

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
| p1-fuzz | running | overnight, stacked on p1-gate's tip; carries the band refit |
| p1-survivors | packet ready | `f9ec31d7` (child of p1-gate; rulings 18, 50, 88) |
| p1-board | running | overnight, from main `2fe750ac` (rulings 9, 11, 12, 13, 38, 50) |
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
| p2-link | in review | round-2 repairs with the lane, then T167's re-scope (adapter-owned per-link pooling, `Dial` is `dial` alone) on top; a sush patch against the new interface is commissioned as a draft once that sha lands; packet after; one stop left (the bound's wording) |
| p2-commit-path | packet ready | packet `14d0d5fb` on `triage/p2-commit-path` (code tip `5a4559e1`, base `1336daa0`, lane diff identical across the rebase); every acceptance and control run by the verifier; three fresh-eyes rounds; T166 landed; no open stop; commits unsigned by the lock protocol, re-signed at merge |
| p1-proptest-ci | in review | landed at `a34859ed` (sweep, caselint, release recipe and nextest profile, meter excluded, T157 rewrite); gate clean; the CI number is a stop (recommendation 4000 after p1-generators); verification and packet after p1-generators lands, since the number is set on that tree |
| p2-vanish-liveness | in review | round 1 landed at `e3c5f986` (the watch keeps every control byte and reads on; deadline aborts; the floor measured against the donor too), gate clean; `e3c5f986` verified; fresh-eyes round 2 found one behavioral regression (a delivered stream dropped mid-label under the departure race) and gaps, sent as repairs; packet after; seven stops in the meta, the precedence rule the one that matters |
| p1-collision-mode | in review | round 1 landed at `e951b81e` (T163 width, recipe liveness, fixtures routed through the crate's derivation, marks re-justified at 28 bytes), gate clean; `e951b81e` verified in full; fresh-eyes round 2 found one bug (a marked test bypassing the harness check) and instrument gaps, sent as repairs with T164's sweep recipe; packet after; two design stops (`ci` membership, a wider root fan) |
| p2-deep-geometry | in review | `4c3051ed` closes both anomalies (biased in-session selects; a decode error returned as the pump's failure and exempt from `SupplyClosed`'s precedence); fresh-eyes round 1 found two decode variants on the wrong side of the exemption (every decode error is a violation) and sent repairs with tests; its full-suite runs wait on the gate mutex; the duplicated-reply stall at 28 bytes is a distinct liveness item, queued |
| p1-generators | running | T161: no rejecting strategies, zero reject budgets in the gate; stacked on p1-proptest-ci at `a34859ed` |
| p1-envelope | running | T10's certificate proptest first, then T43's deletion of the envelope simulation, the crate-doc figures derived, `results/` cleaned; from main `0fad870c`, launched at Finch's word for the overnight run |
| p2-peer | not started | after p2-commit-path |
| p3-dashes, p3-imports, p3-lints, p3-modules, p3-prose-pass, p3-seeds, p3-vocabulary | not started | after every P1 and P2 lane merges; seven open questions for Finch |
| fifteen P4 lanes (`briefs/p4-*.md`) | not started | briefs committed at `46fb2cb6`; after P3; twelve open items for Finch |
| publication prep (T79) | not started | joint, last, after both triages' code lanes close |
