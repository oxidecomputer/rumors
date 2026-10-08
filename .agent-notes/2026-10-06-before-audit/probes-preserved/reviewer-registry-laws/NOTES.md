# reviewer-registry-laws NOTES
Task: review audit/registry-multiplicity-laws, worktree /Users/oxide/src/rumors-slot-37, tip e1782c17 (verified HEAD, clean), base 72d534bf.
No commits. Scratch worktree name must end -rv-registry under /Users/oxide/src/.
Run cap ~8 focused test runs.
## Established
- HEAD verified e1782c17, tree clean.
## Hypotheses
- owner_layers order-dependent if an empty Party exists (empty carry pushes empty top layer).
## Next
- read Party API, laws mod doc, tree oracle same_multiplicity.
## Established (runs 1-5)
- run1 tip: 8/8 pass (and_list_laws, organic, conservation, same_multiplicity).
- run2 M19 (party.rs:425): party_and_list_laws + organic FAIL naming party_join_all_err_conserves_multiplicity. restored, diff empty.
- run3 M20 (clock.rs:307): clock_and_list_laws + organic FAIL naming clock law. restored, diff empty.
- run4 comparator mutants vs agreement proptest: union/count/top/union_count_top caught; measure_top PASSES (256 and 4096 cases) -> blind spot; counterexample A,B,C (1,2,3) vs (2,1,3).
- run5 exhaustive repair test: real pass 0.42s; all mutant comparators fail.
- repair diff: repair-exhaustive-agreement.diff (applied in slot-37 worktree currently! must remove before finishing)
## Next
- run6: clippy + new test on repair tree; then revert worktree to clean.
- run7/8: scratch merge with 48875686 (-rv-registry worktree), law at tip of merge, then M20 in absorb_groups.
## Scope narrowed by coordinator after run 6 (stop comparator search, ~4 runs).
- run6: repair passes testdoc, clippy -D warnings, 3/3 conservation tests. Repair reverted from slot-37; diff empty, HEAD e1782c17.
- run7: scratch merge e1782c17+48875686 (no commit), M20 at clock.rs:336 absorb_groups -> clock_and_list_laws + organic FAIL naming clock law. Scratch worktree removed cleanly. Box dir ~/src/rumors-37-rv-registry left for coordinator.
- DONE; report handed back.
