# builder-touch-bound resumption notes

Task: MB1 (touch-bound property) from
/Users/oxide/src/rumors/.agent-notes/2026-10-06-before-audit/lanes/l7-suanpan/round-1/MB1-touch-bound-property.md
Worktree /Users/oxide/src/rumors-slot-06, branch audit/suanpan-touch-bound, base 719f9e57 (verified HEAD == base, clean).

## Established
- Read common.md, builder.md, README rulings, baseline.md (gate: fuzz leg fails on illumos; board leg drift
  `count_display x heap` two lines; everything else ok).
- Read all suanpan production code and metered.rs, prototype explore_l7.rs (explore/l7-suanpan).
- Isolation convention in before: rely on nextest process-per-test, name it in failure messages (ISOLATION_NOTE
  in crates/before/src/testing/meter/tests.rs). Gate runs suanpan lib tests only via nextest (test-all).

## Design decisions (so far)
- Prefix-checked bound: touches(prefix) <= K * work(prefix) + D after every step.
- Derived K = 15 (L-limb streams: 1 read + 2 deposits x 7 amortized per limb), D = 60 (3 pool activations x 20).
  Per-deposit amortized 7 = 2 chain positions + 2x2 later-visit credits + 1 loaded digit.
- Price clone inside OwnedClone and owned Sum (deviation from brief: clone duplicates prepaid credits).
- GapRound writes priced as one-limb streams (2 units each).
- Known-bad demo: test-only model of trim without upper remnant (no production hook).

## Next
- Write the property in metered.rs, measure ratios, calibrate M14/M15 by string swap.

## Progress (milestone 2)
- Committed-to-be file: crates/suanpan/src/accumulator/tests/metered/touch_bound.rs (+ `mod touch_bound;` in metered.rs).
- TEMPORARY (must remove before commit): touch_bound/probe.rs and the line `mod probe; // TEMPORARY-PROBE` at end of touch_bound.rs.
- Both new tests pass at 256 cases (1.6 s). Logs: build1.log, run1.log.
- families1.log: worst constructed family F2' (descending 6-limb streams at bit shift 31, then a scan) = 7.67/unit,
  util 0.51 of bound. F1 comparison fixed point 5.99. All others below 6.
- Sweep in background: sweep1.log (20000 random cases + adversary 8 seeds x 5000).
- Mutant script: mutants/mutate.py <M14|M15|M16|M12> apply|revert (dry-run verified, tree restored).

## Next
- Read sweep1.log; then calibrate M14/M15 (+M16, M12): run touch_bound tests and full suanpan suite under each.
- Fill "Measured" sentence in the doc; clean (top, inner) shadowing in the control test.

## Progress (milestone 3)
- Probe removed (saved as probe.rs.saved in scratch). Tree: metered.rs modified + metered/touch_bound.rs new.
- Measurements: families1.log (F2' 7.67/unit at L=6), long1.log (9.0/limb, 8.75/unit at L=64), sweep1.log (random max util 0.17;
  adversary best 5.58 (t-D)/w). Detection: M14 306/2000, M15 424/2000 (mutants/*.detect.out).
- Shrinking fix: prop_ind_flat_map (weights not shrunk); M15 now shrinks to 1-2 steps.
- Final calibration on final code: mutants/M14.final.out, M15.final.out (both: 64 existing pass, both new tests fail; control
  readings 4101 and 4104 equal model predictions exactly). M16 caught (also by existing); M12 not caught by property (constant +1).
- clippy (all-features all-targets; default lib tests) clean; full suanpan 66/66 (check3.log).

## Next
- Commit; then `just gate` on box in background (gate1.log), compare to baseline.md.

## Progress (milestone 4)
- Committed d14f683e (signed, "Add a touch-bound property to suanpan's metered suite"); commit message text in commit-msg.txt.
- First gate (gate1.log) was launched uncapped; I terminated its process group 12310 (its own ssh session) on the box.
- Gate relaunched capped: CARGO_BUILD_JOBS=3 NEXTEST_TEST_THREADS=12 AMP_BOARD_SHARDS=8, nice 10 -> gate2.log (background).
- Compare against baseline.md: expect fuzz leg failure (libfuzzer on illumos) and board drift for count_display x heap
  (pinned hugeleaf; live pure-comb default scale, memo-comb acceptance scale), everything else ok.

## Next
- When gate2.log has "gate exit=", compare to baseline; then write the final report.
- gate2 (capped, unreserved): workspace leg 1 TIMEOUT rumors::dispute_wire table_corpus_has_similar_protocol_overhead (load); board/fuzz match baseline; stopped during wasm. Rerun via audit-reserved -> gate3.log
- DONE: gate3 (audit-reserved) matches baseline: workspace 1865/1865 (+2 new), wasm 25/43/8, board 5311 green + same 2 drift lines, fuzz libfuzzer platform error. Report delivered.
## Round 2: tip 33c674ae (signed). Calibration at tip: mutants/*.tip.out. Landing check -> check-r2.log
- DONE round 2: landing check check-r2.log matches baseline (tests 759/1 skipped, snapshot 142, docs 193/3, surface 14 + 211=134+77, board 5311 green + 2 drift lines, wasm 8/25/43). Report delivered.
- Round 3: amended wording fix -> 7f777ce58acc4ba48ab3a3c95b10525473740432
