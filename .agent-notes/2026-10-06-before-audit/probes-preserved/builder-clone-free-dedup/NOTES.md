# builder-clone-free-dedup resumption record

Brief: /Users/oxide/src/rumors/.agent-notes/2026-10-06-before-audit/coordinator-briefs/heap-dedup-and-owned-operands.md
Worktree /Users/oxide/src/rumors-slot-23, branch simplify/clone-free-dedup-and-moves, parent 6efb4562 (verified HEAD, clean, G).
Note: 6efb4562 sits directly on d5e80103; #30 is NOT in its git ancestry (prompt said "itself on #30").

## Baseline for landing check (#36 at 6efb4562, coordinator/recheck/rumors-slot-02.log)
docs ok, lints ok, surface ok, wasm ok; tests: before+suanpan 758 run/758 passed/1 skipped; rumors snapshot 142
(one load timeout of bounded_corpus_manifest_snapshot, rerun alone passes 86 s); board 5311 green x3 + the two
count_display x heap drift lines (pre-board-pin base).

## Plan
1. Parent capture: build amp_board release at parent, copy to target/bed/amp_board-parent, capture cells x3 scales
   (capture.sh adapted from builder-entry-delegation, slot-23 path, N=24).
2. Change 1: shared dedup adapter (Peekable::next_if) in crate::fold; DedupRuns struct deleted; span fold_endpoints uses it.
3. Change 2: private trait for right operand (Version moves, &Version clones); extreme/assign_extreme take it;
   binop_matrix own/assign cells pass r directly. lattice_clones extended to all entry points.
4. Capture new; diff; re-pin flips; calibrate; commit (two commits, one purpose each); landing check.

## Established

## Background jobs

## Next

## Progress (part 1: dedup)
- Parent capture: box target/bed/parent-*.cells (local copies $S/cells/), binary amp_board-parent.
- Part 1 implemented (uncommitted at time of note): fold::dedup_runs (Peekable::next_if), DedupRuns deleted,
  span fold_endpoints uses dedup_runs; new test src/fold/tests.rs owned_inputs_at_a_freed_address_are_not_duplicates.
  Fixture: mirror pair had unequal lengths (218 vs 188) -> concurrent_twins (shared history + 1 tick on share 1 vs 3).
- Calibration (cal-stale-*.log): each stale-address injection fails ONLY the new test, at exactly its entry points;
  reverts verified by cmp with clean-part1.diff.
- Board parent->dedup (move-dedup-*.tsv via celldiff.py): heap only, every move a fall; span_*_all return to/below d5e80103 base.
  version_*_all non-24 falls: peak moment shifts. worst-cases-check: only the two baseline count_display drifts (no re-pin).
- clippy + private rustdoc clean (lint-part1.log).
Next: commit part 1 (signed), then part 2 (Cow right operand), lattice_clones rewrite, calibrate right-clones, capture, commit, landing check.


## Progress (commits)
- 1ebfc1af (G) part 1 dedup. ab548daf (G) part 2 Cow right operands + lattice_clones restated (7 entries).
- Part 2 calibration: right-clones-assign -> owned assignment + owned operator 24 vs 0; right-clones-extreme -> owned-right
  operator 24 vs 0; dedup-never-fires -> NOTHING fails (observation: Bits::eq ptr_eq fast path makes filter a constant saving).
- Parent prod + new check (cal-parent-prod.log): fails exactly at the 3 owned-right entries, 24 vs 0.
- Board dedup->moves: 0 cells moved x3. Leaky flag once on version::tick::tests::flag_reads_plateau_divergence_not_arm_firing (unrelated).
## Landing check
- running at ab548daf -> $S/landing.log
landing check done at ab548daf: board fails with only the 2 baseline drift lines; all else ok
T=02c6d3e04dfb330a0ff5ee48cf5b49d172f29e61

## Round 2 (owner ruling q69: keep lookahead; reviewer notes P1-P3 + scan meter)
- Rebuilt: dffa63fe (G) = 1ebfc1af + P1 trade doc, P3 same_buffer doc, tests/meter/duplicate_runs.rs; msg msg-part1-r2.txt.
  533f4622 (G) = cherry-pick ab548daf. tree(533f4622) == tree(temp 02c6d3e0). Branch moved.
- Meter calibration r2-cal-nofilter.log: fails all 11 entry points (Version::join_all 799@k2 vs 398).
- Probe (scratch zz_builder_probe.rs, removed): r2-tip-probe.log vs r2-parent-probe.log.
- Landing check at 533f4622 -> $S/landing-r2.log
T3=162c34c5ff2a2f035246ccbed8f2f5d33dd77778
Round 3: rebuilt 53776a9e (G) + a2868797 (G); landing -> landing-r3.log
