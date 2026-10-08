# builder-ceiling-rules resumption notes

Brief: .agent-notes/2026-10-06-before-audit/coordinator-briefs/board-sibling-ceilings.md
Worktree /Users/oxide/src/rumors-slot-26, branch audit/board-ceiling-rules, base 38b4d9a5 (verified clean).

## Established
- Reviewer evidence dir has ceiling-rules.txt, no NOTES.md (prompt said NOTES.md; absent).
- Judge (judge.rs): red iff reading > HEAP_INTERCEPT(heap only) + ceiling*constant_units, per sample.
  Rendered per_unit = max over window's two samples of raw reading/constant_units.
- Ceilings with a measured-derivation rule in their doc: MAX_TOUCHES (22, "largest release-board
  measurement +25% rounded up"), FOLD_SCAN (17, "largest fold measurement at any judged size +25%"),
  COMB_SCATTER (3.0, cites "2.0 B/B" reading in prose -> violates no-values rule), DESERIALIZE (4),
  RANK (5), RANKED (9), QUERY (152). No-rule: MAX_HEAP 20, MAX_SCAN 96, MAX_SCALING_EXPONENT.
- Box load at start: 32.

## Jobs
- base board run: log scratch/base-board.log (local redirect)

## Next
- parse base board, bracket per ceiling (floor = 2048 constant units), build table.

## Milestone: base readings (base-board.log, exit 0, 5311 green / 0 red), audit-base.txt
Exact single-cell reads (shard child mode, --shard i/135 = op column; i/7695 = one cell, CombScatter fp=5):
 exact-columns.log, exact-columns-2.log, exact-query-small.log; parsers exact.py exact2.py.
- QUERY floor rule exact 68.3169 (557056/8154) -> 86, BUT small sample query_coverage_many x scatter
  s1 51u 5440B needs C>=86.59 -> 86 RED. Decision: lower to ladder rule 78.2 -> 98 (flag).
- RANKED exact 7.1658 -> 9 ok. RANK 3.7 -> 5 ok.
- DESERIALIZE exact 3.640080 (16384/4501) -> 5 (rise). 10% headroom would give 5 too; keep-rule = ceil(r) no headroom.
- COMB version spellings exact 2.982324 -> 4 (rise); span spelling 1.995. Doc "flat 2.0" written in 345f5f4ee for
  own_version_to_version + clock_own; possible regression since -> report, unmeasured.
- TOUCH all samples 18.8 (version_ticks x memo-comb small) -> 24; ladder 17.2 -> 22.
- FOLD all 12.3 (party_join_all x stagger-arity small) -> 16 (lower from 17); ladder 11.4 -> 15.
Plan: commit1 lower FOLD 16, QUERY 98. Branch (a) = audit/board-ceiling-rules: raise DESER 5, COMB 4, TOUCH 24.
(b) = sibling branch audit/board-ceiling-rules-keep from commit1: keep values, state rules.

## Milestone: commits
- 89e75384 lowering (FOLD 17->16, QUERY 152->98), signed G. focused-c1.log ok
- 5323e4a7 option (a) on audit/board-ceiling-rules (DESER 4->5, COMB 3->4, TOUCH 22->24), signed G
- 60d75027 option (b) on audit/board-ceiling-rules-keep (values kept, rules stated), signed G, focused-b.log ok
- landing check at 5323e4a7: log scratch/landing-a.log (started next)
- landing check at 5323e4a7: clean (landing-a.log); readings identical to base. Report next.

## Round 2: owner ruling Q65 (one intercept-adjusted rule everywhere)
- New branch audit/board-ceiling-one-rule from 89e75384 (keep lowering). Siblings (a)/(b) left in place.
- onerule.py: COMB 4, DESER 5, RANK 5, RANKED 9, QUERY 109 (exact 86.5882), FOLD 16, TOUCH 24 -- all resolved.
- MAX_HEAP/MAX_SCAN/ops.rs inline: not moved (not in reviewed table); report.
- #94: neutral row max adjusted <= 50.72 (< 87.2 rule-move, < 109 red); existing query cells identical on #94.
- Repairs: P1 (no sibling refs; name 63d01d903 for COMB), P3 (skyline builder ghost), P4 (denominator units).
- reviewer report extracted: scratch/reviewer-report.txt
- one-rule commit b1e71248 (G) on audit/board-ceiling-one-rule; landing check -> scratch/landing-onerule.log
- landing at b1e71248 clean, counts = baseline, readings identical. Report.
- four-ceiling commit on audit/board-ceiling-one-rule; landing -> scratch/landing-four.log; raw sweep raw-sweep.log, exactrule.py
