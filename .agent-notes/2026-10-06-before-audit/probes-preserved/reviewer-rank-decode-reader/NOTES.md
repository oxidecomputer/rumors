# reviewer-rank-decode-reader: resumption notes

Reviewing audit/rank-decode-reader at 1dcd2790 in /Users/oxide/src/rumors-slot-35 (base 38b4d9a5).
HEAD verified 1dcd2790, signed G, tree clean at start.

Originals saved: rank.rs.orig, tests.rs.orig (restore by cp, then git diff must be empty).

## Plan (cap: three box runs)
1. Run 1: mutant schema in rank.rs (env RANK_MUTANT), original test: baseline + 11 survivors + constructed mutants 50-56, fresh seeds, persistence off.
2. Run 2: proposed repair in tests.rs (exact failure oracle, deterministic interrupt arm) + schema.
3. Run 3: repair alone: clippy, test, doc.

## Established
- Run 1 (run1.log): schema mutants vs the branch's property, persistence off, no shrink.
  m=0 pass x3 and at 4096 cases. All 11 survivors (11,2,20,21,22,30,31,32,40,41,42) FAIL x3.
  Constructed swallow-before-decided_by mutants 50-54 FAIL x3. 55 (swallow after settled) PASSES x3 and
  at 4096: permissive window is real. 56 (correct early-stopping decoder) passes x3 and 4096.
  Single-case kill rates: m21 67/1500, m2 29/500, m52 20/500.
- Run 2 (run2.log): repair applied (repair.diff) + schema. Pending.
- Repair in worktree tests.rs now; rank.rs carries the schema. Restore both from *.orig at the end.
- Run 2 done (run2.log): deterministic test D kills 11 survivors + 50-54 once each; passes 0, 55, 56.
  Repaired property P kills 11 survivors + 50-55 x3; passes 0, 56 x3 and at 4096. m55 single-case 68/500.
- Run 3 (run3.log): repair + prose candidates (full-repair.diff), no schema: clippy, doc pub/priv, before nextest.
- After run 3: cp rank.rs.orig and tests.rs.orig back; git diff must be empty.
- Restored; git diff empty, status clean, HEAD 1dcd2790.
