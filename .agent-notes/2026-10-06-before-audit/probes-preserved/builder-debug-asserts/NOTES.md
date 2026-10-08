# builder-debug-asserts: resumption record

Task: brief at /Users/oxide/src/rumors/.agent-notes/2026-10-06-before-audit/coordinator-briefs/simplify-debug-assert-scans.md
Worktree /Users/oxide/src/rumors-slot-08, branch simplify/debug-assert-scans, base d67e1dcb (verified HEAD == base, clean).
Scratch: this directory (S below).

## Established (by reading)
- Site 1 (read.rs read_digits prefix scan) invariant = "every digit below lowest_written is zero";
  held by Digits::assert_invariants ("nonzero digit below watermark") in digits/tests.rs, called after every
  step of representation.rs's exhaustive + proptest suites. assert_value also checks scaled readout vs oracle.
- Site 2 (digits.rs activate all-zero scan) invariant = "retained digits are zero while small is active";
  assert_invariants idle branch checks it, BUT representation.rs has no reset op, so idle only sees never-activated
  storage. Candidate catchers: value suites that reset then re-spill (differential.rs:351, surface.rs:471, metered).
- Site 3 (boundary.rs from_positive clone+cmp_zero): clone().cmp_zero() calls touch() -> debug meter readings in
  crates/before/tests/meter (dev profile, ceilings measured x1.25, plus two-sided bands) may FALL. Board is release: unaffected.
  Sites 1,2 do no touch(); heap meters (tests/meter.rs global allocator) might see site 3's clone allocation.

## Plan
1. Base run: before+suanpan tests with stderr, collect MEASURED lines -> S/base-measured.log
2. Delete three asserts; rerun; diff MEASURED; adjust ceilings/bands that move (lower only).
3. Mutations (assert already deleted), each reversible swap, verify git diff restored:
   M1 site1: add_at not lowering lowest_written / start too high
   M2 site2: Digits::reset skips zeroing
   M3 site3: lower_word Less arm skips negation; zero boundary variant
4. Docs: state invariants + naming tests where a reader looks.
5. Commit; just gate in background; compare to baseline.md (fuzz leg fails; board worst-cases-pin count_display x heap drift only).

## Background jobs
(none yet)

## Progress (update)
- Base run (S/base-run.log, base d67e1dcb + nothing): 757 passed, 1 skipped, exit 0. Readings: S/base-measured.txt (152 MEASURED, via S/extract.sh).
- Deletions applied locally (uncommitted); saved as S/deletion.diff. Prose NOT yet written (write after mutation results).
- Post-deletion run: S/del-run.log (background). Diff its MEASURED vs base.
- Mutation harness: python3 -I S/mutate.py apply|revert <name>; names m1a,m1b,m2a,m2b,m3a,m3b,m3d,m3c (see file).
  After each revert: git diff | diff - S/deletion.diff must be empty.
- Reading: Digits::reset is the ONLY digits->small transition (all other small=Some sites start small).
- Mutation results so far (logs S/m*.log):
  m1a: 33/64 suanpan fail; representation exhaustive "scaled value left 2^64 right 2^64+4" (assert_value)
  m1b: representation exhaustive "nonzero digit below watermark 3 after [4, 0]" (digits/tests.rs:42)
  m2a: pooled_reuse "unscaled value left 18446744075857035264 right 18446744073709551616"; surface "a scalar value leaves every retained digit zero after [1, 6]"
  m2b: pooled_reuse 2^65 vs 2^64; surface idle check after [9, 6]; also metered held_width_rows
  m3a: 12 before fail; arbitrary_trees_agree "min_ticks kernel disagrees with the tree-fold oracle ... left 2 right 3"; many others via anchor.rs:148 debug assert
  m3b: 9 fail, ALL via anchor.rs:148 "the deferred distance is positive" (left Equal). With anchor assert also off (x-anchor-assert-off): 693 pass -> no test oracle sees zero wide boundary.
- Meter diffs computed (S/base-measured.txt vs S/del-measured.txt): only site 3 moves touches; ceilings to lower listed in report draft below.
- m3c, m3d: caught only via anchor.rs:148 assert (m3c also ascend_cliff_plateau ceiling); with anchor off m3d passes all, m3c only the ceiling.
- Re-pinned 11 meter constants (table in commit 01d7d34b body). Focused run: clippy clean, 757 pass (1 leaky), readings == deletion run.
- COMMITS: 6e4f2dcb (suanpan, signed), 01d7d34b (before + re-pins, signed).
- Coordinator: every remote cmd starts `unset CARGO_TARGET_DIR; export NEXTEST_TEST_THREADS=24;`
- Gate: S/gate.log (background). Compare to baseline.md: fuzz leg fails (libfuzzer); board worst-cases-pin count_display x heap drift only.
- NEXT: on gate finish, compare, then report via SubagentHandback.
- GATE DONE: matches baseline (fuzz+board fail as recorded; workspace 1863/1863/2 skipped). Report next.

## Round 2 (reviewer changes on 01d7d34b)
- New branch: 6e4f2dcb, 4372e81a (generator), 0f6b9859 (compare_height doc), 33a60d4c (deletion, rewritten doc+msg). All G. Tree == pre-rebase tip tree.
- Calibration logs: S/r2-m3c-arm-zero.log, r2-m3d-word-zero.log, r2-m3b-wide-zero.log (all fail property); r2-clean.log (0.259s default, 20000 cases 21.2s pass); r2-runtime-before.log (0.141s); r2-at-T.log (pass at T, 20000 cases 15.3s).
- Landing check: S/r2-check.log (background). Compare to baseline.md "Landing check". Also compare MEASURED to del-measured.txt.
- R2 DONE: landing check matches baseline (S/r2-check.log); readings identical (S/r2-measured.txt). Reported.
- R3: terminology fix applied; tip 919c752f; fmt ok; doc-internal ok (S/r3-doc.log). Reported.
