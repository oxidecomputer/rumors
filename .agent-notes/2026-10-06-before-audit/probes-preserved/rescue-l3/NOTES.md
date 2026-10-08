# rescue-l3 resumption record

Task: catalogue every instrument the events lane (L3) built, per
`.agent-notes/2026-10-06-before-audit/coordinator-briefs/instrument-rescue.md`.
Output: `.agent-notes/2026-10-06-before-audit/instrument-rescue/03-events.md` (do not commit).
Box runs: at most 3, census/runtime only, scratch worktree `/Users/oxide/src/rumors-rescue-l3`
(create detached at explore/l3-events; remove with plain `git worktree remove` at the end).
nextest limit on main is now 300 s (common.md changed; main commit 205199319).

## Read (all in full)
common.md (re-read after change), auditor.md, rescue brief, AGENTS.md x2, README, baseline,
instruments.md, survey 2.1 + last sections, QUESTIONS #74 + notices 88/96/98, ranker handoff-COMMON,
all lane records round-1, l3_probe.rs + l3_heap.rs at 873f39991 (copies in this dir),
#74 co_generated.rs + census tests + tick/tests.rs diff, builder/reviewer-events-cogen NOTES,
mutation patches + apply-batch2.py in auditor-l3 scratch, D1 demo commit 08573e159 message.

## Established (verified)
- explore/l3-events tip 873f39991, base 58285ca51, 5 signed commits; adds l3_probe.rs (1156 lines),
  l3_heap.rs (284), tick.rs registers both as cfg(test) children.
- #74 audit/events-cogen-and-tidy tip 183a094a4 carries: spine, wide, multi-scan strategies,
  check = assert_tick + ticks(0), ticks(1..4) vs iterated+oracle, wide composition; ONE late perturbation;
  palette 2..=5; census floors; two witnesses. #74 DROPS: bushy strategy as a property; check_one's
  strict domination, output envelope, region locality, min_ticks step <= 1 and <= count; the second
  late perturbation (from_end+1, height+1); palette len 1 (correctly: collapses).
- Committed laws: tick_strictly_advances, tick_only_inflates_the_region (law generators only);
  tick_output_is_input_bounded (independent arbitrary pairs only). No min_ticks step law.
- COMMITTED min_ticks_floors_every_history exists (version/tests.rs:682 on main): bound = TOTAL ticks of
  the whole history, checked on final clocks only, world_strategy (<30 ops, ticks 0..=6, no absorb).
  Lane records said "NONE found" -- a record error. L3 floor is per-clock causal past, every step.
- Calibration "committed" filter re-admits l3 tests matching /min_ticks/ (4 at 30379f8d):
  true committed = 68 (matches #74 builder). M22 committed 7 = 4 true + 3 l3; M23 6 = 4 true + 2 l3.
  M16 6 = 5 true + 1 l3. Escapes M6, M8, M15, M19 unaffected (72/72 pass implies 68/68 pass).
- Histogram/reach/counter/heap numbers in inventory match logs (run-hist-1, run-diag-release-1,
  run-heap-release-1..4).
- jump-rising-spine board family exists only on stopped fix/before-min-ticks-heap demo 08573e159;
  it fails acceptance on main by design (D1 unfixed), so it can land only with a fix.
- Board memo families (memo-chain/comb/fanout/oscillating/churn) cover L3 memo heap families in kind;
  memo-comb is version_tick's pinned heap worst.
- Adjacent downstream instruments (not L3's): #74 builder mutate.py; #74 reviewer memo proptest
  consecutive_scans_read_back_only_their_own_differences (experiment2.patch); D1 reviewer probes (survey 1.1).

- 00-baseline.md (integrator) exists; its 4.1 repeats the lane error ("min_ticks as a floor over
  histories: events lane found no instrument") -> flag to integrator. Its 2.4: organic heights < 64 bits.
- JR readings verified from run-heap-release-3.log; J=2^16 n=98304 reads 26.27 B/B RED (plain sawtooth).
- Ruling 68 (e7e107a2f): descend! removed after #74 lands; helpers recurse on construction-bounded input,
  deep tests on explicit-stack threads.

## Box runs (worktree /Users/oxide/src/rumors-rescue-l3 @ 873f39991 + uncommitted census module
## crates/before/src/version/tick/l3_rescue_census.rs registered in tick.rs)
- run 1: --no-run debug build, exit 0, 3m28s (run1-build.log).
- run 2 (background, bvjypbwi1): serial nextest, default cases, uncarried probes + census
  -> run2-tests.log.

## Next
1. Read run2-tests.log; fill numbers into draft 03-events.draft.md (scratch); copy to output path.
2. Revert census edits? No: worktree is scratch; remove with plain `git worktree remove` needs clean tree
   -> first `git -C ... checkout`? NO (rule). Use: delete the untracked census file and reverse the
   tick.rs append by string swap, confirm `git status` empty, then `git worktree remove`.
3. Report.

## DONE (2026-10-08)
- run 2: 12/12 passed, exit 0, 61.3 s (run2-tests.log); census and runtimes filled in.
- Published .agent-notes/2026-10-06-before-audit/instrument-rescue/03-events.md (cmp-identical to draft).
- Scratch worktree reverted (census file deleted, tick.rs swap reversed, status empty) and removed with
  plain `git worktree remove`. Box dir ~/src/rumors-rescue-l3 (with target/) left for the coordinator.
- Census source kept here: l3_rescue_census.rs. Report handed back.
