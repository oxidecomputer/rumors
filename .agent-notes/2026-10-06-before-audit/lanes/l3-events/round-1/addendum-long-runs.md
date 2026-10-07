<!-- CAVEAT LECTOR: written by Claude (Opus 5.5) as auditor-l3, a round-1 addendum read from finished box runs; no new investigation. -->

# Addendum: the round-1 long property runs

All three runs passed every case. None changes D1, MB-1, MB-2, or S1.

Each run used a debug build of the lib test binary, with debug assertions on, executed through
libtest without nextest's timeout. The tree in each was the base `58285ca5` plus the explore
probes, with no production change. Every probe compares production against the recursive tree
oracle unless noted.

## What each run tested and found

| Run | Probes and case counts | Explore revision | Result (verbatim) |
|---|---|---|---|
| A (stopped by me once its spine families became the long pole) | `l3_min_ticks_tight`, `l3_min_ticks_floor_over_histories`, `l3_cogen_bushy`, `l3_min_ticks_tight_spine`, `l3_family_bushy`, at 100,000 cases each | `30379f8d` | each `... ok` before the stop (`exit=255` is the stop, not a failure) |
| B | `l3_family_wide` and `l3_family_multi_wide` at 5,000 cases, `l3_deep_spine` (oracle-free claims, spines up to 3,000 levels) at 300 cases | `30379f8d` | `test result: ok. 3 passed; 0 failed; ... finished in 5463.91s` |
| Spines (detached on the box) | `l3_cogen_spine` and `l3_family_spine` at 30,000 cases | `873f3999` | `test result: ok. 2 passed; 0 failed; ... finished in 2134.71s` |

The per-pair claims are those listed under `check_one` and `check_family` in
`instruments-inventory.md`. The `min_ticks` probes check the floor over linear histories and
attainment by a constructive single-seed history.

Logs:

- Run A: `long-A.log` in the auditor-l3 scratch directory.
- Run B: `long-B.log` in the same directory.
- Spines: `~/src/rumors-audit-l3-events/target/l3-long/spines.log` in the auditor's worktree on
  ox-east-1.

## Effect on the round-1 deliverables

- **D1 (`min_ticks` heap).** No change. These runs measure values, not heap. They ran on the
  unfixed tree, so they say nothing about the chosen repair (deferred re-anchoring plus a chunked
  payload stack).
- **MB-1 (co-generated pairs).** The unmutated record grows from about 120 wide-family cases to
  5,000 each for the single-pre-scan and multi-scan strategies, and to 100,000 or 30,000 for the
  bushy and spine families. No false alarm appeared, which strengthens the brief's claim that the
  strategies flag only real defects. The calibration table is unchanged.
- **MB-2 (`min_ticks` over histories).** The floor and both tightness properties held over
  100,000 cases each. Calibration is unchanged.
- **S1 (four tidy-ups).** Unaffected; they are prose and test-layout changes.

One use for the fixer and reviewer: the D1 repair changes how `min_ticks` stores contributions,
so `l3_min_ticks_floor_over_histories`, `l3_min_ticks_tight`, and `l3_min_ticks_tight_spine`
(explore branch, `crates/before/src/version/tick/l3_probe.rs`) check its values beyond the
committed differentials. A release build runs them much faster. A debug build keeps the internal
assertion in `HeightPrefixes::settle` active, which caught the round-1 close-count mutant.
