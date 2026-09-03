<!-- CAVEAT LECTOR: maintained by the two triage coordinator sessions (Claude) for Finch as the current-state dashboard of the before and rumors triages; not authored, audited, or endorsed by Finch. Rewritten in place at every state change; it carries no history (the merge queue and git do). -->

# Triage landing status

Updated: 2026-09-04 04:15 UTC by the before session.

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
| p2-widths | in review (reopened) | ruling 118 (Finch, 2026-09-04): the narrow-or-wide fold table is dissolved into one `Vec<u64>` and the log-factor liveness floor re-derived under 64-bit probes; the lane is audited for code shaped to keep a pin from moving; a lane agent is on it; the packet `ba36e970` is superseded and rebuilt after |
| p2-surface | in review (reopened) | ruling 119 (Finch, 2026-09-04): every size measure becomes the unit label `input bytes` (roster, datasets, x-axis caption, island summary); the sampling prose moves to rustdoc; a lane agent is on it; the packet `cad0b75d` is superseded and rebuilt after |
| p1-fuzz | packet ready | `0df4aa4b` (code `1b6df6fd`, child of p8-tagwalk; three fresh-eyes rounds; coordinator acceptance green at the tip on both machines; twelve sentry seeds); six stops inside; merges after p1-gate and p8-tagwalk, before the rumors p1-collision-mode |
| p1-survivors | packet ready | `f9ec31d7` (child of p1-gate; rulings 18, 50, 88) |
| p1-board | packet ready | `cc0e365d` (code `67a454cf`, 60 commits on main; six review rounds; the board 2096 green / 0 red at both scales and the worst-case pin clean at the tip; coordinator's gate clean but for the fuzz port); two stops inside (ruling 11's fit; the shard tag bump). The before session is PAUSED at Finch's word until the packets are reviewed interactively |
| p1-suites | not started | after p1-board |
| p2-rows | not started | after p1-suites; carries two handoffs from the harness review |
| p2-cures | not started | after p2-rows and p2-widths |
| p2-surface (registry) | not started | the held step 3 of p2-surface, after p1-suites |
| p2-generators | packet ready | `e84a74a3`; no rejecting strategy remains in before (rulings 111, 113) |
| p3-vocabulary | not started | last of the sweeps |
| p4-ghosts, p4-structure, p4-rosters | not started | after the P1 and P2 lanes they follow |
| p2-census | running | new lane (ruling 120, Finch 2026-09-04): the surface census, the hand roster, the `syn` extractor, and the coverage suite dissolve into a committed `cargo public-api` snapshot diffed in the gate; the two consumers (fuelscape's coverage table, the fuzzfit exemption tiling) derive the surface from the snapshot; stacked on `p1-fuzz`'s code tip; merges after `p1-fuzz` and `p2-surface` |
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
| p2-link | waiting on Finch | packet `95c429e2` (code `a956bdb3`, base `7aa2b9a1`), five verification passes, five rounds; three stops; root file `Cargo.toml` (`tokio/rt`); the sush patch beside it |
| p2-commit-path | packet ready | packet `14d0d5fb` on `triage/p2-commit-path` (code tip `5a4559e1`, base `1336daa0`, lane diff identical across the rebase); every acceptance and control run by the verifier; three fresh-eyes rounds; T166 landed; no open stop; commits unsigned by the lock protocol, re-signed at merge |
| p1-proptest-ci | in review | landed at `a34859ed` (sweep, caselint, release recipe and nextest profile, meter excluded, T157 rewrite); gate clean; the CI number is a stop (recommendation 4000 after p1-generators); verification and packet after p1-generators lands, since the number is set on that tree |
| p2-vanish-liveness | waiting on Finch | packet `c9e7465b` (code `31324e42`, base `9c8ce16c`), five verification passes, four rounds; seven stops; merges after `p2-deep-geometry` (the terminal-tail conflict resolves toward this lane) |
| p1-collision-mode | waiting on Finch | packet `f6d9a0af` (code `35d134fd`, base `d0dcb9f5`), four verification passes, three rounds; three stops; merges after before's `p1-fuzz` refit |
| p2-deep-geometry | packet ready | packet `f74324a7` on `triage/p2-deep-geometry` (code tip `7e7bae71`, base `7858b35e`, stacked on the collision branch; merges after it); three fresh-eyes rounds, every item verified; its gate runs under the mutex and the verdict is appended when it lands; stops: the shared predicate at integration with vanish-liveness, the conformance floor under the schedule, the residual precedence window, the duplicated-reply stall queued |
| p1-generators | waiting on Finch | packet `f21541da` (code `8adc330a`, base `a34859ed`), four verification passes, three rounds; six stops; merges whole after before's `p2-generators`; root files `.cargo/config.toml`, `Cargo.toml`, `justfile`, `AGENTS.md` |
| p1-envelope | waiting on Finch | packet `10f4bb6f` (code `e02d0aee`, base `0fad870c`), five verification passes, four rounds plus the grid's two; five stops; root files `Cargo.toml` (T169), `AGENTS.md`, `README.md`, `.config/nextest.toml`, `.gitignore` |
| p2-peer | not started | after p2-commit-path |
| p3-dashes, p3-imports, p3-lints, p3-modules, p3-prose-pass, p3-seeds, p3-vocabulary | not started | after every P1 and P2 lane merges; seven open questions for Finch |
| fifteen P4 lanes (`briefs/p4-*.md`) | not started | briefs committed at `46fb2cb6`; after P3; twelve open items for Finch |
| publication prep (T79) | not started | joint, last, after both triages' code lanes close |
