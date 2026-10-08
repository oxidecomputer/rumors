# reviewer-docs-branch resumption notes

Task: review docs/audit-corrections in /Users/oxide/src/rumors-slot-38, tip 5fcfb95c, base 176d3d43.
Brief: .agent-notes/2026-10-06-before-audit/coordinator-briefs/docs-branch.md. No commits; repairs as diffs here.

## Established
- Base: HEAD 5fcfb95c, clean, 23 commits all signed (G).
- Item 1 (7f2d9ad4f rank sum_iter): argument SOUND. Verified:
  - panic: digit_index filters index < usize::MAX (suanpan accumulator.rs:413-418).
  - exp sources: decode_stream groups Vec<u8> (exp<=8*isize::MAX<2^34), FromStr len, version max_depth (preorder tags => depth<=stored bits), pair_rank max of depths; all arithmetic via from_raw lowest terms => exp<=max operand.
  - W: Digits is dense Vec<i64>, add_at resizes; shift_left wide = add_shifted into fresh acc. W <= 2^33.
  - Bullet 1 & 2 algebra checked. Nit: carry chain can add a few digits above contributions; "every position under 3*2^35" omits it (margin unaffected). Non-blocking.
  - Argument generalizes to any usize width (ratio 4), comment only says 32-bit.

## Next
items 2..8, deferred-item merge-tree checks.

## Milestone 2 (all commits read)
- Run 1 (run1-sketch.log, exit 0): 3/3 sketches pass. from_parts after recv: y = s1|m, unissued, y < s2. Held s1: y unissued, y < s2. Off-grid example verified; grid closed under join/meet. Sketch file removed; tree CLEAN.
- Item 4 key: tests/stale_state.rs from_parts_over_an_earlier_version_reproduces_its_successor says "Valid by the model"; L1 report "model accepts this". Builder's "forbids" contradicts; crate rule 2 literal reading supports builder. Base incoherence -> owner. "exactly one hole: bytes" also false via dangerously_alias (party.rs module doc names it).
- Item 2: argument correct, but omits "witness lies in the segment"; summary is noun phrase; "nine" count.
- Item 3: bound true at all include sites; two notations incl. space line in same section. Recommend unify to n.
- Item 5 OK; M2 caught via panic not assertion. Items 6,7 OK. Deferrals verified via merge-tree (565b57893 conflicts lib.rs/README with wasm32 fix; read-high-part rewrites read.rs; fork-size-hint rewrites size_hint text). Tip merges cleanly with all audit branches except 3 pre-audit rumors-only ones.
- Minor: a2d64a3ef "all three verdicts are exact" (grid only); Clock::forks sentence mixes Clock sizes vs party shares.
## Next: apply repairs (reversible), save diff to scratch, run doclint + docs + docs-internal (run 2), reverse-apply, confirm clean, report.
- Run 2 (run2-docs.log, exit 0): doclint, readme-check, docs, docs-internal all OK with repairs.diff applied. Reverse-applied; tree CLEAN at 5fcfb95c. Crate-page proposal in crate-page-linearity-proposal.md. NEXT: hand back report.
