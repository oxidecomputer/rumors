# builder-deep-surfaces: resumption notes

Brief: .agent-notes/2026-10-06-before-audit/lanes/l2-algebra/round-1/machinery-deep-surfaces.md
Worktree: /Users/oxide/src/rumors-slot-01, branch audit/deep-surfaces, base d5e80103 (verified HEAD == base at start).
Remote: on-illumos.sh /Users/oxide/src/rumors-slot-01 'unset CARGO_TARGET_DIR; ...' ; -j 24, nice -n 10 for long jobs.

## Established
- Only concurrent pairs reach Version::hull_bits (Version::hull, version.rs ~940): equal/empty/comparable short-circuit.
- Sum/FromIterator fold through Version::join (Extreme::emit), same sweep as `|`.
- VersionRegionReader is shared with join/meet/projection/place: mutating it is caught by existing deep tests, so calibration mutants go on surfaces only the new test reaches: (M1) hull_bits boundary loop -> recursion; (M2) Plateaus rebuilt on a recursive tree descent via VersionTreeReader::node().
- `before` builds at opt-level 2 in dev/test: mutants must be non-tail recursions.
- No RUST_MIN_STACK anywhere; tests run on 2 MiB threads.
- Depth choice: min non-tail recursive frame is 16 B (x86_64/aarch64); 2 MiB/100k = 21 B, so 100k misses 16-20 B frames. Plan DEPTH = 1 << 18 (2 MiB / 2^18 = 8 B). Report as deviation.
- Coverage row: name the deep_tree_* family; do NOT claim every public walk (Party::without/covers, Clock::absorb, Span::intersect etc. are not driven at depth) -> report as follow-up.

## Next
1. Write test deep_tree_shape_hull_and_fold_stack_safety in crates/before/src/clock/tests.rs after the query test.
2. Run it on base (time it). 3. M1 and M2 calibrations (+ existing deep_tree tests pass under each). 4. Commit. 5. just gate in background.

## Progress (update)
- Test written + fmt'd (uncommitted) in clock/tests.rs; also deep_tree_stack_safety doc reworded. Base run: PASS 0.892s (base-run.log).
- Mutants reverted; git diff on shape.rs and lattice.rs empty. Calibration done: M1+M2 full suite 693/694 pass, only new test SIGABRT; M1 alone and M2 alone each abort only the new test.
  Revert with: python3 -I mut/swap.py revert m1.json ; ... revert m2.json ; then `git diff --stat` must show only clock/tests.rs.
- Background: full before suite under M1+M2 -> mut-m1m2-full.log (task b4088dbe1).
- Next: revert M1 only, rebuild, run new test alone -> expect abort in shape (M2 alone). Then revert M2, git diff check.
- Committed 3d14c9fe (signed). Gate running -> gate.log
- DONE: gate matches baseline (board 5311/0 + 2 drift lines; fuzz libfuzzer; workspace 1864/1864, 2 skipped; wasm 25/43/8). Report sent.

## ROUND 2 (coordinator message; tip 3d14c9fe)
B1: new test doc + commit msg: "non-tail recursive call ... overflows" -> state in compiled frames; optimizer-loops (tail, 1+f(rest)) pass.
B2: sibling doc (:546-552) + coverage row overclaim; say "checks at depth for the operations they drive"; row cites deep-input stress
    tests >= 100,000 levels, chiefly deep_tree_*; no enumerations/counts; reflow line 552.
S3: add right-spine coverage to new test; right-recursive mutant (loop left via descend_left, recurse right) must PASS tip test and
    ABORT strengthened test; git diff empty after. Report runtime before/after.
Verify: bg on-illumos 'unset CARGO_TARGET_DIR; export NEXTEST_TEST_THREADS=24; ~/bin/audit-reserved ~/bin/audit-check' > check.log; baseline = baseline.md "Landing check" (757/757 +1 per test added, 142/142, board 5311/0 + 2 drift lines).
Use `ulimit -c 0` in mutant runs (abort core dumps).
- M3 reverted (DIFF-EMPTY). M3 at tip: 694/694 pass (escapes).
- M3 on strengthened tree: only new test SIGABRT (693/694); reverted, shape.rs diff empty.
- Round 2 committed (amend) 20da73ad signed. Landing check -> r2-check.log
- Round 2 DONE: landing check matches baseline (758/758+1 skip, 142/142, doctests 193+3, surface 14 / 211=134+77, wasm 8/25/43, board 5311/0 + 2 drift). Report sent.
