# rescue-l2 resumption record

Task: catalogue every instrument lane L2 (algebra) built, per
`.agent-notes/2026-10-06-before-audit/coordinator-briefs/instrument-rescue.md`.
Output: `.agent-notes/2026-10-06-before-audit/instrument-rescue/02-algebra.md`
(do not commit). Box runs <= 3, only census/runtime the records lack, scratch
worktree `/Users/oxide/src/rumors-rescue-l2` (detached; plain `git worktree
remove` at end; restore any edits first so removal needs no --force).
common.md changed mid-task: nextest limit is now 300 s (commit ce67ab083).

## Source (verified)
- explore/l2-algebra tip 6748bcd41; merge-base with main 1745d3779 (the branch
  merged it at 5c53b5d26). Round-1 base 58285ca5.
- Files added: tests/l2_probe/{main,gen,model}.rs, tests/l2_cost/main.rs,
  tests/l2_probe/mutants/{00..19}.patch, run.sh, run_on_tree.sh, round-2 report.
- Probe source identical d78c6129..6748bcd41; crates/before/src identical
  1745d377..main except testing/exhaustive/tests.rs (1 line). So lane records
  apply to main's production code.
- Scratch-only (auditor-l2/, NOT on branch, lost on reboot): mutate.py,
  mutants1/2/all.json (20 hand-chosen string swaps M01..M38), extract.py,
  selected.txt, span_check.rs.txt (draft), writer_mut.rs / 18-*.rs / 19-*.rs
  (mutated source copies used to make patches), logs.

## Established (verified unless marked)
- Round-1 mutants: probe kills 20/20, but only 9 by its own model assertions
  (M01 M02 M08 M09 M10 M11 M12 M13 M28 -> l2_probe/main.rs lines); 11 by
  production debug_asserts/panics (M03..M07, M14, M15, M17, M18, M27, M38).
  Committed suite kills 20/20 (fail-fast, counts are lower bounds).
- Round-2: 18 L8 survivors: classes T1 U2 C9 E4 ?1 G1 -> only 00 (T), 06 (?),
  16 (G) could be value-caught at all; each outside probe predicates/inputs.
- Committed bridge from_oracle_version goes through production VersionWriter
  (testing/version.rs from_tree_stream) -> committed lattice oracle shares the
  writer; probe's encode_version is independent bit-by-bit.
- Committed reach for L2 ops: arb_oracle_version ARB_DEPTH 4 (census: max
  depth 4, max 21 nodes, pairs never equal); exhaustive EV depth 2 (691);
  organic depth <= 6; lattice family_pool fixed shapes (Dense 64, AltSpine 64,
  Dense 512 in flat_over_deep); wide grids 32 cells; deep shape_version only
  in tick tests.
- Deep tests carried: deep_fork_halves -> #34 (deep_tree_shape_hull_and_fold);
  deep_surfaces view-vs-view -> #75 Part C. Not restated: to_version on a
  disjoint/concurrent region at depth (minor).
- Board overlap: CliffComb (k=n, alternating 2^k-1/2^k teeth), MaskDrift
  (comb under scattered mask vs 2^k plateau) cover most of l2_cost regimes.
- Runtime (logs): algebra_matches_model 48.2 s at 256 cases (run9); deep
  0.28/0.48 s; calibration aborts 0.154 s; census 1.19 s / 2000; cost
  families 0.19-0.71 s each.

## Hypothesis to measure (box run 1)
Late draws (parties p,q, perturb c, fold arity) often read an exhausted tape in
the 300/900-split tiers -> seed party, c == a, empty fold. Census records only
pair + p. Plan: extended census in scratch worktree (Tape pos accessor +
rescue_census test), same draw order as run_case.

## Next
- create worktree, write census, run once in background, read, restore files,
  remove worktree.
- write 02-algebra.md.

## Box run 1 (started 2026-10-08 ~16:55 box time)
- Worktree /Users/oxide/src/rumors-rescue-l2 at 6748bcd41, UNCOMMITTED edits:
  gen.rs (+pos accessor), main.rs (+rescue_census). Copies + census.diff in
  this dir. MUST restore both files (git show HEAD:path > path; git diff
  empty) before `git worktree remove`.
- Local log: run1-census.log (background task bnhqnnupq).

## Box run 1 DONE (exit 0); worktree restored (diff empty) and removed (plain).
- Box copy ~/src/rumors-rescue-l2 (with target/) left for the coordinator.
- Log: run1-census.log (+ .keep copy). Cross-check: first-2000 relation and
  family counts equal the lane's census rerun at the tip exactly.
- census1.log (09:43:46) predates 10ec351cf (09:44:11) and the depth-300 tier
  117dcb1eb (09:52:12): lane's recorded census describes the 4-tier generator.
- Key tip numbers (10,000 tapes): a depth p50 12 p90 100 max 300; leaves p50 28
  p90 278 max 1034; both a,b depth>=5 64.7%; relation None 34.2 Eq 27.2 Gt 16.3
  Lt 22.4; 63-bit code in a 22.9%, 65-bit 41.1%; cascade with 63-bit left code
  join 1.76% meet 1.93%; p nonseed depth>=5 48.6%, q 43.3%; p party depth p50 4
  p90 68 max 300, regions max 834; fold arity 0: 38.0%, 18-40: 35.9%;
  exhausted before fold 36.5% overall, 97.7% in tier 900; before p 20.6%
  overall, 64.0% in tier 900; c == a 22.0%.
## Next: write 02-algebra.md.

## DONE: 02-algebra.md written (uncommitted, untracked), reviewed twice.
16 full entries; 3 carried one-liners; 7 intermediate files listed. Next: hand back.
