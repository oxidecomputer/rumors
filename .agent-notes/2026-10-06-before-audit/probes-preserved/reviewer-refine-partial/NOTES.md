# reviewer-refine-partial: resumption record

Branch simplify/span-refine-partial @ 051cf07e in /Users/oxide/src/rumors-slot-22, base 70610c97.
Verified: HEAD = 051cf07e, tree was clean at start, both commits G-signed.

## Slot state (MUST RESTORE before return)
- Uncommitted experiment: query.rs + polarity.rs toggles (backups *.orig here, sha
  query d1c5b8c4, polarity b69b1056) and untracked causally/query_reviewer_probe.rs.
- Restore: cp *.orig back, rm probe file, then `git diff` empty and `git status` clean.

## Established by reading
- refine_partial's only caller is Query::coverage, after filter::coverage(lo,hi,self.demands())
  == Partial; demands() maps floor->After, ceiling->Before 1:1. Floor/Ceiling::coverage
  route through Query::coverage. Board ops call Query::coverage.
- Walk: required sides never drop; far pair never settled; live>0; exhaustion reads final interval.
- Every Span ctor enforces lo<=hi (new, at, owned callers, serde, borsh).

## Runs (cap 3)
- run1 (background): run1.log: debug build, debug probe, committed causally tests unmutated +
  mutants lt / holes_skip / up_top / up_unclamped (RV_MUT env toggle), release probe.
- run2 planned: checkout 95e96c7e, grid test alone (debug). Then restore branch.

## Done (all three runs used; slot restored, git diff empty, status clean, HEAD 051cf07e)
- run1.log: debug+release probe clean (no precondition break, no divergence); mutants lt, up_top,
  up_unclamped caught by grid+random; holes_skip caught by random only.
- run2.log: 95e96c7e passes causally suite (10/10).
- run3.log: grid-repair.diff passes; holes_skip/lt/up_top/up_unclamped all fail the extended grid;
  off-grid version Version(0b001010101011) demonstrated in [bottom, a1].
- Board: query_coverage_many scan exponents rose (+0.02..0.03, 6 rows) unreported by builder.
- Next: report only.
