# builder-events-cogen resumption notes

Task: build MB-1 (cogen, machinery) and S1 (four tidy-ups) from lanes/l3-events/round-1, one commit each,
in /Users/oxide/src/rumors-slot-29 on audit/events-cogen-and-tidy. Do NOT build min-ticks-histories brief.

## Established
- Base verified: HEAD d528d816, clean, branch audit/events-cogen-and-tidy.
- Overlap: redesign (slot-16, fix/before-min-ticks-heap, uncommitted live edits) touches tick.rs only at
  minima.close() call sites (lines ~225-430) and prescan.rs; S1 touches none of these, nor min_ticks. Nothing dropped.

## Next
- read tick code, generators, tick/tests.rs, validation_index
- Plan: commit 1 = S1 (all four items, one commit per task prompt; S1 brief said one per item -> report),
  commit 2 = MB-1 (new module testing/generators/co_generated.rs + tests.rs census; proptests in version/tick/tests.rs;
  validation_index paragraph). Commits touch disjoint files.
- Explore probe copy: scratch l3_probe.rs (arb_pair, arb_spine, arb_wide, arb_multi_wide, check_family, census helpers).
- Site model verified against tick.rs:319-366 and prescan.rs run(): site = party Node(Leaf(true), nonempty) over version Node;
  outermost site's scan reserves 1 + sites in its right range.
- S1 edits done (uncommitted): party/clock/version ticks docs; memo tests -> memo/tests.rs; tick.rs let _ = matched x2; Panics prose x2.
  Focused S1 run in background -> s1-focused.log (release clippy, debug clippy, memo test, private doc).
- MB-1 written (uncommitted): testing/generators/co_generated.rs (+ /tests.rs census, FLOORS ARE PLACEHOLDER 1 -> measure!),
  tick/tests.rs section "co-generated pairs", validation_index paragraph, generators.rs mod decl.
- Mutations planned: M6 tick.rs compare_above_vs == Less -> != Greater; M8 delete resolve_deferred in pop_lookahead;
  M15 memo begin_scan .min(1); M19 memo reserve slot.saturating_sub(1)/BLOCK_SLOTS.
- COMMITTED: S1 = 1ff2ffb1 (G); MB-1 = ed75af84 (G). S1 focused checks passed (s1-focused.log).
- MB-1 first run (mb1-first.log): census counts bushy-nested 19, spine-nested 263, spine-shared 129, wide 252, multi 121 (of 400);
  floors set 5/65/32/63/30. Times debug: bushy 1.9s, spine 16.5s, wide 26.6s, multi 41.1s (load ~118).
- Calibration loop running: calibrate.sh -> calib-<M>.log, seeds-<M>/ (fetched box seed dirs), calib-done marker.
  Mutants auto-revert; check "restored" lines. DO NOT edit worktree until calib-done.
- Then: decide on mutant seeds (box-only), landing check at tip.
- Calibration DONE (calib-*.log): M6 -> spines, wide, multi fail; M8 -> spines, wide, multi; M15 -> multi only ("a memo slot is written once");
  M19 -> wide, multi (index OOB memo.rs:146); 68 pre-existing tick-filter tests pass under every mutant. TAKE -> memo test fails (slot 0 Some(-2) vs Some(1)).
  CENSUS (WIDE 1..40) -> census fails "wide ... fell to 0 of 400". All restored (git diff empty).
- Seeds: M6, M8 spine seeds appended to tick/tests.txt; MB-1 amended -> b0149aea (G). S1 = 1ff2ffb1 (G).
- Next: landing check at tip b0149aea -> landing.log
- landing run 1 at b0149aea: lints FAILED (doclint 232/227 chars), all else ok, 764 tests. Amended -> 82d2b425; gate-lints ok; landing run 2 -> landing2.log
- DONE: landing run 2 at 82d2b425 clean (landing2.log, landing2-logs/). Final: S1 1ff2ffb1 G, MB-1 82d2b425 G. Report handed back.
## Round 2 (coordinator message): amend MB-1 only. Edits uncommitted in worktree (HEAD 82d2b425).
- B4 reviewer text missing from reviewer NOTES -> wrote own; also fixed M8 misstatement (resolve_deferred is tracker's, not memo entry).
- K=6 (K=8 infeasible: multi needs 146, measures 123). Census new: 341/172/322/123; floors 10/10/56/110. r2-first.log.
- Running calibrate2.sh -> r2-calib-*.log, r2-calib-done. Then amend commit (msg fix tie + deferred, drop bushy), landing check.
- r2 calibration done (r2-calib-*.log): all caught, witnesses fail under own mutant, HALVE 61<110. landing -> r2-landing.log at 948b6f21
- r2 landing clean at 948b6f21 (r2-landing-logs). Reported.
