# rescue-l5 resumption record

Task: catalogue every instrument on `explore/l5-spans` (tip 2874e0c3d, base
58285ca51) per `coordinator-briefs/instrument-rescue.md`. Output (do not
commit): `.agent-notes/2026-10-06-before-audit/instrument-rescue/05-spans.md`.
Box runs: at most 3, scratch worktree `/Users/oxide/src/rumors-rescue-l5`
(created detached at 2874e0c3d; remove with plain `git worktree remove` at the
end, after confirming `git status --short` shows only my uncommitted probe).

## Read (done)
common.md, auditor.md, rescue brief, README, baseline, lane brief, all round-1
lane records, the whole harness (copy: `audit_l5.rs` here), instrument survey,
QUESTIONS #61/#85/#94/notices, ranker handoff-COMMON and handoff-l5 (thin),
L8 census baseline, L8 survivor INDEX, committed lane tests (causally/tests.rs,
place/tests.rs, laws version_triple/version_list/version_party, verdict_matrix
header and pool), function oracle module doc, exhaustive corpus.

## Established
- Harness = one file, 2172 lines, 4 commits; lane production code unchanged on
  main since base (only board files moved) -> harness should compile on main.
- Mutant calibration (mut-run.log): 8 L5 properties at 5cec22e4 (before shaped,
  organic, cube, carry existed). 17/19 caught by both L5 and committed; M11
  both survive; M18 committed-only (Debug hole count). Per-property attribution
  extracted (queries+antichain: M1-5,8-10,15,17; verdicts+deep: M6,7,13,16,20;
  algebra: M12; own: M14; span_query_agree: M3,8,9,13).
- Committed per-mutant kill counts min 2 (M12), 3 (M5,M10,M14).
- L8 survivors in lane files: none value-changing that this harness could kill.
- Committed span/query oracles use production partial_cmp / <=, which shares
  overlay advance, OrderState, RegionReader, suanpan Accumulator with walks.
- Committed exact coverage: two-party 3x3 grid only (+ #61 extension, ready).
- arb_layout depth cap 40 (spines <=40, recursive <=10); record's "to 64" is a
  bucket label.
- #61 builder grafted the harness and ran query props (timings in
  builder-refine-partial/tip-run.log) but its mutant runs excluded L5 tests.
- witness_coverage is dead; finer-universe test doc overclaims "witness".
- body_span_algebra doc says "set intersection over generated sublattice"; code
  checks endpoint formula only.

## Box runs
- Run 1 (planned): build; nextest default L5 properties (runtime); ignored
  cube/carry/refine (runtime); census probe `rescue_l5_refine_census`
  (refinement-decided coverage checks per population, vs committed grid
  emulations). Log: `run1.log` here.

## Next
Write 05-spans.md: summary + one entry per instrument, ordered by value.

## Run 1 (done, 2026-10-08 16:56-17:02 UTC, load avg 26 -> 51)
- Log: run1.log; census extracted to census.txt. All phases passed; no STATUS lines.
- Build cold 4m09s. 17 default L5 props pass in 26.1 s wall (per-test times in log;
  shaped_antichains 26.08 s, span_verdicts 13.97 s, others <= 5.9 s).
- Ignored: cube 7.24 s (tally identical), carry 0.063 s, refine 0.055 s (readings
  byte-identical to records).
- Census probe (uncommitted, in rescue worktree): 857,820 checks, 0 mismatches.
  committed-grid-main up refine = 0 (matches #61's stated gap); #61 grid -> 232.
- Remaining: fill placeholders in 05-spans.md from entries.md + census.txt;
  then `git -C /Users/oxide/src/rumors-rescue-l5 diff --stat` (only audit_l5.rs),
  save the probe diff to scratch, `git worktree remove` (plain; will need the probe
  edit gone first: save diff, then `git -C ... restore`? No: brief says outputs only
  under that worktree and plain remove; plain remove refuses with modified files.
  Save diff to scratch, then revert by reverse-applying the diff, verify git diff empty,
  then plain remove.)

## Done
- 05-spans.md written (not committed): 21 entries, census section, ranking.
- Probe patch saved (rescue-census-probe.diff); reverse-applied; worktree clean;
  `git worktree remove` done. Box mirror ~/src/rumors-rescue-l5 (778M) left for
  the coordinator.
- No commits made.
