# Calibration of the L1 discovery suite (explore/l1-identity @ 03213585)

Driver: `calibrate.py` with `muts1.py`; each mutant is an exact string swap in
production code, run as
`on-illumos.sh <wt> 'unset CARGO_TARGET_DIR; nice -n 10 cargo nextest run -p before --all-features --locked --build-jobs 24 -j 24 --no-fail-fast --test audit_l1'`,
then reverted. `git diff --stat` after the run: empty. Logs: `calib/<name>.log`.

| Mutant | Site | Caught by (audit_l1) |
|---|---|---|
| M01 join never collapses two owned children | party/join.rs | 8 properties |
| M02 compare queues right-pair presence bits swapped | party/compare.rs | set_algebra_{arbitrary,disjoint,nested}, histories |
| M03 sync's separable path forgets b's right child | party/sync.rs | sync_{arbitrary,disjoint}, histories |
| M05 forks' extra-share selection inverted | party/forks.rs | forks_partial_drain, clock_forks, histories |
| M06 removal forgets deferred right sibling | party/fork/remove.rs | forks_partial_drain, clock_forks, histories |
| M07 difference disposition swapped | party/without/difference.rs | set_algebra_{arbitrary,nested} |
| M08 fold drops the rejected operand | fold.rs | join_all_failure_conserves_multiplicity, clock_join_all_conserves |
| M10 sync_all plans one share too many | clock.rs | sync_all_matches_model, histories |
| M11 join_all drops untested groups on final-loop failure | party.rs | join_all_failure_conserves_multiplicity |
| M12 without's disjoint fast path inverted | party.rs | set_algebra_{arbitrary,nested} |
| M13 region walk reports absent right child owned | party/io/regions.rs | set_algebra_* (shape) |
| M15 fork of an owned region swaps halves | party/fork.rs | fork, array_split, sync_*, histories |
| M16 select_path skips on the wrong choice | party/fork.rs | forks_partial_drain, clock_forks, histories |
| M17 sync prefix always tagged left | party/sync.rs | sync_*, histories |
| M18 Clock::join error drops the version | clock.rs | clock_join_all_conserves |
