# reviewer-events-cogen resumption notes

Task: round-1 review of audit/events-cogen-and-tidy in /Users/oxide/src/rumors-slot-29
(S1 = 1ff2ffb1, MB-1 = 82d2b425, base d528d816). No commits. Cap: 3 box runs, nice -n 10, no landing check.

## Established
- HEAD 82d2b425, clean, on branch (verified at start).
- S1: statically verified. Party has no empty encoding (validate.rs grammar; borsh validates; writer finish asserts;
  join/without return Option; only cfg(test) from_test_bits bypasses). Count: From u8..u128, usize. let _ = matched
  removal safe (debug_assert! names var in release); builder's release clippy OK in s1-focused.log.
- Census site predicate matches tick.rs walk (owned left via peek, right present, version Node).
- BUSHY_REGIONS doc wrong: prop_recursive desired_size = soft target on total child slots (proptest recursive.rs).
- Census is deterministic (fixed seed); floors are ~quarter of measured; halving passes.
- Persisted seeds replay before the case loop regardless of cases (proptest 1.11 runner.rs:601).
- seed_liveness checks file locations only; seeds were appended to an existing live file.
- M15 release analysis: stale bits + take's Small(0) residue; value corruption needs a stale slot written after a
  higher slot in the same block (deferred first entries do that).

## Experiment (run 1, IN FLIGHT -> run1.log; do not edit worktree until it finishes)
- apply.py applied; diff saved to experiment.patch. Revert: git apply -R experiment.patch; confirm git diff empty.
- Mutants by REVIEW_MUT: 6, 8 (builder's), 15 (builder's), 21 (mine: clear only last used block),
  22 (prescan leaf arm skips only left party child when both present; bushy probe).
- Candidate memo proptest `consecutive_scans_read_back_only_their_own_differences` in memo/tests.rs.

## Next
- Run 2: release build (--cargo-profile release): multi + memo proptest under M15/M21 for release visibility.
- Then revert, confirm git diff empty, report.

## Run 1 results (run1.log, wrapper exit 0)
- Correct code: 73/73 tick set incl. candidate memo proptest; census counts 19/263/129/252/121 (same as builder).
- Seeds-only (PROPTEST_CASES=0) spine under M6 and M8: both FAIL (new seeds replay first, runner .rev()).
- M6: spine, wide, bushy, multi fail; M8: spine, multi, wide; M15/M21: multi + memo proptest (debug: write-once assert);
  M22: family_pairs (pre-existing) + all cogen -> bushy not unique.
- Halved (PROPTEST_CASES=128 -> 22 multi cases): M15 3/3 caught (~52 s incl shrink), M21 3/3 (~25 s).
- seed_liveness 3/3 pass.
## Run 2 IN FLIGHT (run2.log): release, M15/M21 value visibility; census palette 2..=5. experiment2.patch = full diff.

## Round 2 (HEAD 948b6f21)
- Read r2 diff. K arithmetic verified: floor(1)=10, floor(6)=56, floor(12)=110. Witness narratives hand-traced.
- M8 builder correction right: resolve_deferred is RangeMinima anchor (range_minima.rs:331), not memo deferred first entry.
- r2 run IN FLIGHT: r2run.log. apply_r2.py applied; revert with git apply -R r2-experiment.patch, confirm git diff empty.
