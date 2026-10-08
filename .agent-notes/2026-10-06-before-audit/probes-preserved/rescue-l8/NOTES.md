# rescue-l8 resumption record

Task: catalogue every instrument lane L8 (adequacy) built, per
`.agent-notes/2026-10-06-before-audit/coordinator-briefs/instrument-rescue.md`.
Output: `.agent-notes/2026-10-06-before-audit/instrument-rescue/08-adequacy.md`.
Do not commit. No box runs unless census/runtime is missing (max 3, scratch
worktree /Users/oxide/src/rumors-rescue-l8). Read explore branch with git show.

## Source
- explore/l8-adequacy tip 0acb87bdc, base 58285ca51 (verified). 9 signed commits.
- 111 files: 41 vendored num-bigint (nbprobe), 37 l8/records copies, census
  module + ~25 scripts/probes/witnesses.
- Lane records: lanes/l8-adequacy/round-{1,2}, addendum-bits-party.

## Read so far
- briefs common/auditor/instrument-rescue; README; baseline; instruments.md;
  survey intro + 1.9-end; QUESTIONS notices 96/98/88; ready #38 #57 #66 #78 #82 #83
- lane: round-1 report/NOTES/findings (all 4)/census-baseline/coverage NOTES;
  round-2 report/NOTES/coordinator-leads; addendum survivors INDEX

## Already carried (one line each)
#57 nextest timeouts; #66 normalize exhaustive; #78 rank decode reader;
#82 range_minima near boundaries; #83 trait laws + hole_subtracts; #38 value pool (O2)
(+ disjoint-families brief: not built? check; padding brief: not built by decision)

## Next
- auditor-l8 scratch inventory
- read every non-vendored file on the branch; verify vendored diff = 1 line
- 00-baseline.md existence check; ranker handoff-l8 check
- write entries

## Established (2026-10-08, all by reading; no box runs)
- vendored num-bigint = crates.io 0.4.8 except FAST_DIV_WIDE=false (diff -r)
- 15 wasm32 injections apply at base and main (scripts/inj_applies.py);
  #43 retires forks-count-narrow; #63 retires rank-arith-route + rank-accumulate-sign
- pin filter names exist on main, #58, #92, #31 tips
- generators/optrace/diff_ops identical base vs main; #74 only adds files
- branch survivor records == lane addendum (shasum); 282 diffs; raw lists
  (scratch only) sum 3041/282/596
- probe insertions (L8_SHRINK, L8_SPLIT_NARROW_HIT, l8_probe! sites, env probe,
  L8_MUT window switch) preserved nowhere; probe copy restored clean
- trap "control" (<2^20) passed too: probes don't prove site reached
- census deterministic (census2 == baseline; family logs identical)
- census trace model re-implements optrace indexing (matches apply() on main)
- ddfabe4cb on main = TARGET_DEPENDENT marker from L8 D1
- #82 carries rm witnesses' values in public form; #66 subsumes normalize probe
- ranker attributes #38 value pool to L8; #38 entry says L7 O2 (correction)
- no mutation/coverage recipe committed anywhere

## Entry order (value)
census; wasm32 injection table+driver; survivor baseline+campaign tooling;
window witness; padding witnesses; probe tree+recorder; bisect tooling;
usize census; coverage procedure; fold-pin calibration; surface calibration;
nbprobe; writer hunt; env probe. One-liners: #57 #66 #78 #82 #83 ddfabe4cb.

## DONE (2026-10-08)
Wrote instrument-rescue/08-adequacy.md (15 full entries + 6 carried lines),
coordinated with 00-baseline.md (cites its sections; no duplication).
No box runs, no worktree created, no commits. Next: hand back.
