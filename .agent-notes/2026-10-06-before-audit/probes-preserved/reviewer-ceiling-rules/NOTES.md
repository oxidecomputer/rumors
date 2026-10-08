# reviewer-ceiling-rules notes
Task: review audit/board-ceiling-rules (a, 5323e4a7) and -keep (b, 60d75027), base 38b4d9a5, lowering 89e75384.
Scratch worktree: $S/rumors-review-ceil (detached), syncs to box ~/src/rumors-review-ceil. REMOVE when done.
## Method
COMB cell: amp_board --shard <op_pos>/<op_count> --scale-bits 4010000000000000 (top), 3ff0... (base).
Positions at 38b4d9a5: own_version_to_version 52, clock_own 103, of 135. At 345f5f4ee: 30, 57, of 74 (awk count; self-verified by output names).
## Results
- 345f5f4ee (v3): top s2 522560/258998 = 2.017622 (GOOD, matches "flat 2.0"). base s1 65736/32814.
- eff7de3ea (list pos 30): 772416/258998 = 2.982324 (BAD). Driver: step.sh <commit>; logs m-<commit>.log.
- Verified builder exact readings from its logs: DESER 16384/4501=3.640080, COMB 2.982324, QUERY small 5440/51 needs 86.59.
- audit.py grouping: base table unaffected (distinct values per currency); landing-a table mislabeled (COMB as DESER 4.0; DESER+RANK merged at 5.0).
- 96ef1c49f (pos 45, v4): 522560/258998 GOOD
- 929709cd8 (pos 38): GOOD
- 5c603aefc (pos 34): GOOD
- aed7d6b82 (pos 32): GOOD
- 63d01d903 (pos 31): BAD; parent aed7d6b82 GOOD -> first bad commit 63d01d903 "refactor Before around domain operations".
- Allocator-log diagnostic (exp.sh, patch_alloc.py; scratch-only, restored): base s1 own_version_to_version x comb-scatter
  aed7d6b82: R2048>4096 ... R16384>32768 A0>32104 D32768  (peak 65736)
  63d01d903: R4000>8000 ... R32000>64000 A0>32104 D64000   (peak 96992)
  Mechanism: split payload stream ladder seeded by exact reserve in BitsWriter::extend_bytes (unaligned arm) -> 250*2^k rungs; content ~32.1K just above 32000 rung.
  Final A0>32104 = SplitOutput::finish exact interleave copy. Not an algorithmic regression: phase artifact held constant by geometric board ladder.
- Touch 22 from c0b5d7018 (ladder basis 17.18), predates small-input samples (317f0f890).
- Intercept-adjusted rule (peak-1024)/units: QUERY exact max 86.588 -> 109; others reproduce 4->5,3->4,5,9.
- All 15933 board rows identical base vs landing-a (verdict, heap, scan, touch).
DONE; scratch worktree removed at end.
