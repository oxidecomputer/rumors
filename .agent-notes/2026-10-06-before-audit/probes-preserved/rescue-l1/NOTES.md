# rescue-l1 NOTES (resumption record)

Task: catalogue every instrument on explore/l1-identity (tip 346a82ac9, base d5e80103a)
per coordinator-briefs/instrument-rescue.md. Output:
/Users/oxide/src/rumors/.agent-notes/2026-10-06-before-audit/instrument-rescue/01-identity.md
Do not commit. Box runs <= 3 via scratch worktree /Users/oxide/src/rumors-rescue-l1.
Report via SubagentHandback: count, top three, unassessable items.

## Read
- common.md, auditor.md, instrument-rescue.md, README.md, baseline.md, ranker handoffs

## Next
- lane records l1-identity all rounds; survey last two sections; QUESTIONS 96 + ready entries
- full diff base..tip

## Established (2026-10-08)
- Read: all round-1 lane records, survey 2.1/2.3/4/5/rulings, QUESTIONS #34 #40 #43 #75 #76 #81 notices 88 96 98,
  instruments.md, auditor-l1 scratch (calibration.md, muts1-4.py, calibrate.py, stats-1, stats-committed,
  cost-1, retention-6, long-1/2/3).
- Full diff d5e80103a...346a82ac9 read: tests/audit_l1/{main,model,gen,history,deep,overlay,wide}.rs,
  audit_l1_cost.rs, audit_l1_retention.rs; src diff = #34 merge + MB2 drafts (carried by #75) +
  explore_committed_generator_stats + D1 probe (superseded by #43).
- None of #34 #40 #43 #75 #76 #81 landed on main (verified merge-base --is-ancestor).
- Identity code + generators + optrace unchanged 58285ca5..main (git diff --stat empty) -> stats-committed.log valid at main.
- nextest limit at main: 60 s period x5 = 300 s (common.md updated mid-task).
- cost-1.log: is_disjoint row reads 0.00 (inputs overlap at first leaf -> early exit; vacuous family).
  forks(2^d).next() scan = ~10,000 + 6d (coverage.md says 4,000 + 6d: arithmetic slip).
- M26 "only this probe" claim wrong: #75 reviewer found 5 amp_board_smoke tests catch it at base.
- Committed: optrace ops = Tick Ticks Fork Send Sync Join only. Board party_without = seed.without(b) and a.without(a) only.
  Committed clock_shape oracle check folds rises to absolute heights, depth<=4.
  wide_count_prefixes_conserve_the_party: taken<=4, conservation only (no share identity).

## Box plan (<=3 runs): scratch worktree /Users/oxide/src/rumors-rescue-l1 at 346a82ac9
- run 1: --no-run build of audit_l1, audit_l1_cost, audit_l1_retention
- run 2: default-count run + census (tests/audit_l1/census.rs: history op execution, overlay reach, join_all Err rate)

## Box runs done (2 of 3); worktree removed (clean), box dir ~/src/rumors-rescue-l1 left for coordinator
- run1-build.log: --no-run, exit 0, 39.86 s
- run2-default-and-census.log: 25/25 pass, exit 0, load ~20; every L1 property <1.5 s at default counts;
  generator_stats 2.79 s; cost/retention readings identical to cost-1.log / retention-6.log.
- census.rs saved in this dir. Results:
  history: 40,446 steps over 2000 histories; 51% of steps end at pop>=8; peak pop >8 in 68%;
    join exec 4054/5833, sync 3969/5702 (rest i==j); join_all 3885 (36% zero others, 1645 with >=2);
    sync_all 3817 (34% zero others, 1609 with >=2); forks full 2382 / partial 1837 / take0 604 / k0 697;
    split 2:931 3:867 5:918 8:896.
  overlay: empty version 27%; version depth >4 11.75%, >32 2.6%; party depth >4 78%, >32 47%;
    cells >16 65%, >64 30%; cell depth >4 81%.
  join_all failure property: Err 1991/2000 (Ok arm 9 draws, all mode 1); returned 1..10, >=3 in 60%.
- stats-1.log bucket correction: two-child branches >=17 in 42% (>=33 in 20.5%), depth >32 in 46.75%.

## DONE (2026-10-08)
- Wrote instrument-rescue/01-identity.md (14 full entries + 6 one-liners), not committed.
- Line refs verified; style pass done. Next: hand back via SubagentHandback.
