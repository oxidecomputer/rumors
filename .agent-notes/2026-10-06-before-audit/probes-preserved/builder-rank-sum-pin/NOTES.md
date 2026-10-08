# builder-rank-sum-pin: resumption record

Task: machinery brief `lanes/l4-measures/round-1/machinery-wasm32-rank-sum-pin.md`
(under /Users/oxide/src/rumors/.agent-notes/2026-10-06-before-audit/).
Worktree /Users/oxide/src/rumors-slot-10, branch audit/wasm32-rank-sum-pin,
base f062e744 (verified HEAD == base, clean, at start).

## Established
- Files: crates/before/wasm32-pins/guest/src/checks.rs (`rank_arithmetic`),
  harness/tests/pins.rs (`rank_arithmetic_straddles_the_usize_alignment_limit`).
- wasm32-pins is a detached workspace with no .config/nextest.toml, so the
  root 180 s terminate does not apply there; still keep each test < 180 s.
- Mutant for calibration: after `let shift = ...` in `sum_iter`
  (crates/before/src/rank.rs ~line 1061) insert
  `let shift = shift as usize as u64;`. Expect case 6 trap, case 5 pass.

## Plan / next
1. Edit guest + pins (Route enum, cases 5/6, loop 1..=6).
2. Temporary untracked probe test harness/tests/zz_probe.rs to read memory
   pages per case; delete before commit.
3. Build + run on box; record pages and per-test time.
4. Calibrate mutant; revert; git diff empty.
5. Commit; nice'd `just gate` in background; compare with baseline.md.

## Background jobs
(none yet)

## Progress (update 1)
- Edits done (uncommitted): RankRoute enum + cases 5/6 in guest; NEW test
  `rank_sum_straddles_the_usize_alignment_limit` (cases 5..=6) instead of the
  brief's loop 1..=6, because the combined loop took 186.7 s (run1.log) > 180 s.
- Probe (run1.log): pages case1 50207, c2 50207, c3 42014, c4 58399, c5 58399,
  c6 50207. secs 24.8/23.8/21.6/30.3/26.0/34.7 (box load ~200).
- Deep-first verified (deepfirst.log): both small summands abort in
  handle_alloc_error <- Digits::add_at <- Rank::sum_iter, pages 42014,
  panic hook not called (control panics read 1). Temp edits restored by cmp
  against *.saved; probe source kept at scratch zz_scratch_probe.rs.
- Next: mutant calibration (mutant.log), commit, nice'd just gate (gate.log).

## Progress (update 2)
- Mutant calibration done (mutant.log): case 6 Trapped(UnreachableCodeReached),
  case 5 passed, rank_arithmetic test passed under mutant. Reverted; rank.rs diff empty.
- Committed 25c2c02d (signed).
- Gate running in background: local log gate.log (bash task bxxx). Compare to baseline.md:
  only fuzz (libfuzzer illumos) and board (count_display x heap drift) may fail.

## Progress (update 3)
- Gate run 1 (gate-run1-uncapped.log, load ~585): audit/internal-docs/surface/
  docsrs/doctest/wasm ok; fuzz + board match baseline exactly; workspace FAILED
  only by TIMEOUT of rumors::dispute_wire table_corpus_has_similar_protocol_overhead
  (181 s) -- untouched by diff (diff is only wasm32-pins). wasm: rank_sum 69.7 s,
  existing rank_arithmetic 177.7 s (pre-existing near-budget).
- Coordinator: prefix NEXTEST_TEST_THREADS=24; rerun once under cap.
- Gate run 2 in background -> gate.log (task bc...). After: write report.
- Gate run 2 (capped, gate-run2-capped.log): same as run 1 but workspace had TWO
  load timeouts (bounded_corpus_manifest_snapshot, table_corpus_...); board/fuzz
  match baseline; wasm 9/9 (rank_sum 82.4 s, rank_arithmetic 156.8 s).
- Coordinator now requires `pset-run -n 24 -- just gate`; run 3 -> gate.log.
- Runs 3 and 4 with `pset-run -n 24`: psrset "Device busy" (procs 13/8/34; then 114),
  exit 125, gate never ran; no stray pset left (psrset -i shows only set 1).
  Stopped retrying; reporting to coordinator. Work committed at 25c2c02d.

## Round 2 (review changes required)
- Reviewer notes: scratchpad/reviewer-rank-sum-pin/NOTES.md. Do: amend message (180 s is audit
  rule, not nextest), pins.rs "rather than a panic in the arithmetic", "512 MiB numerator",
  add case 7 [one,deep,one] judged by checked_sub(one+one)==deep. Branch stays one commit.
- Edits made (uncommitted): RankRoute::SumShift/SumLanding, case 7, loop 5..=7, new doc.
- Verify: r2-pins.log (just wasm32-pins), r2-landing.log (landing mutant), r2-shift.log
  (shift mutant + case-7 probe), then amend, then ~/bin/audit-check -> r2-check.log.
- r2 results: r2-pins.log 9/9 pass (load 626: rank_sum 339.7 s, rank_arith 374.7 s,
  version_decode 128 s). Landing mutant: case 7 Failed(WrongValue) (r2-landing.log).
  Shift mutant: case 6 trap; case 7 alone trap (r2-shift.log). Reverted; diff empty.
- Amended -> 0c22bd66 (signed). audit-check running -> r2-check.log.
- r2-check: board only (baseline drift), rest ok. Reporting.
