# builder-deep-identity: resumption record

Task: MB2 (lanes/l1-identity/round-1/MB2-deep-identity-probe.md), worktree
/Users/oxide/src/rumors-slot-32, branch audit/deep-identity-probe, base 20da73ad
(verified HEAD, clean, at start). Base lacks board-pin fix ddfabe4c, so the
landing check expects board leg failing with exactly the two count_display x heap
drift lines, and before+suanpan count = 758 (#34) + my new tests.

Mutant swap: $S/mutate.py <wt> M26 apply|revert (strings from auditor-l1/muts4.py).

## Steps
1. [running] M26 applied at base; build log m26-base-build.log (bg). Then run
   whole `before` suite (expect all pass), revert, git diff empty.
2. Part A + Part B commits; M26 again -> deep_identity_stack_safety aborts.
3. Part C commit + own mutant.
4. Landing check (uptime < 300 first).

## Established (step 1 done)
- M26 at base, whole `cargo nextest run -p before --all-features` (m26-base-run.log):
  694 run, 689 passed, 5 failed, all amp_board_smoke (stack overflow, SIGABRT):
  retaining_results_does_not_allocate_measured_heap, merge_refuses_a_silently_shrunk_grid_for_every_family,
  shard_protocol_round_trips, board_runs_to_completion, worst_map_covers_every_operation_row.
  => BRIEF PREMISE FALSIFIED: committed suite catches M26 via board smoke (auditor ran only
  --lib, --test meter, --test forks_count). Report as disagreement.
- Reverted; git diff + status empty.
- Plan: add M26-lean (recursion with &mut iterator, black_box after call, descend inline(never))
  to show Part B catches a small-frame recursion that board smoke passes.
- Disagreements to report: Ranked::decode/precedence/contains have no walks of their own
  (compose driven walks / share place::walk with Span::place); Version::ticks walk also driven by Part B.
  Debug comment "recursive Debug pretty-printer" wrong (renders bits) -> fix.
- Design: one const STACK_SAFETY_DEPTH in testing/generators.rs holding the frame bound.

## Established (steps 2-3 done)
- Commits (signed G): A 7ebea7ee, B 3dd90095, C aad3577b (tip). Tree clean.
- Tip deep run (tip-deep-run.log, load ~84): A tests 0.07-1.47 s, #34 1.91 s, B 2.66 s, C 5.18 s; all pass.
- M26 at tip (m26-tip-run.log): deep_identity + 5 board smoke abort.
- M26-lean (&mut iter, black_box, descend inline(never)) (m26lean-tip-run.log): same 6 abort.
- M26-stored (recurse per stored level, loop below owned region) (m26stored-tip-run.log):
  ONLY deep_identity_stack_safety aborts; 695 pass. => Part B's distinct catch, demonstrated.
- MC-place-drop (place::walk post-drop continuation recursive) (mc-tip-run.log):
  ONLY deep_tree_remaining_surfaces_stack_safety aborts; 695 pass.
- All mutant reverts verified (per-file diff empty).
## Next
- clippy + deep tests at commit B (detached), back to branch.
- Landing check at tip (uptime < 300). Expect tests 758+2=760 (#34 base lacks board-pin fix ->
  board leg fails with exactly two count_display x heap drift lines).
