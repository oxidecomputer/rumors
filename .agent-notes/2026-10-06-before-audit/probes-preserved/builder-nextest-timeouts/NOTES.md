# builder-nextest-timeouts: resumption record

Task: L8 D2 brief `lanes/l8-adequacy/round-1/briefs/machinery-detached-nextest-timeouts.md`.
Worktree /Users/oxide/src/rumors-slot-19, branch audit/detached-nextest-timeouts, base 70610c97 (verified HEAD at start).

## Established
- Timing data (no new timing runs): `tab.py` over (a) box landing-check/gate logs
  (`box-wasm-logs.txt`, `box-surface-logs.txt`, harvested from ~/src/*/target/{audit-check,gate}-logs)
  and (b) scratchpad agent logs. Slowest PASS of base tests:
  wasm32-pins rank_arithmetic_straddles_the_usize_alignment_limit 576.9 s (scratch; box logs 415.8 s);
  version_compare 416, rank_decode_accepts 388, version_join 349, version_decode 200, suanpan_rejects 112.
  fuelscape count::tests::parallel_build_matches_sequential_reference 247.1 s (box).
  fuzzfit enforce fuel_stays_in_the_pinned_bands 112.5 s (box). surfacecheck 1.7 s.
- 845.8 s for rank_decode_streams_fraction_groups_at_the_doubling_limit is from the demonstrator's pre-fix branch (fix/before-wasm32-buffer-growth), not base.
- Rule: limit = first 60 s multiple >= 2x slowest pass, floored at root 180 s.
  wasm32-pins 1200 (20), fuelscape 540 (9), fuzzfit 240 (4), surfacecheck 180 (3).
- Chose one limit per workspace, not per-test overrides (dispute; see report).
- Root .config/nextest.toml: added a pointer paragraph.

## Progress
- Committed 02899c17 (signed G).
- Hang demos done (hang-demo.log): TIMEOUT surfacecheck 180.014s, fuzzfit 240.018s, fuelscape 540.177s, wasm32-pins (guest spin in liveness) 1200.064s. Swaps reverted, git diff empty.
- Landing check running: check-1.log (background, local wrapper process).

## Next (original plan)
1. Commit configs.
2. Hang demos (uncommitted swaps): wasm32 guest liveness() -> spin loop (hangs harness_outcomes_are_live);
   zz_hang_probe park-loop tests appended to fuzzfit harness/tests/sanity.rs, fuelscape src/plan/tests.rs,
   surfacecheck src/check/tests.rs. Build --no-run, then one remote command running 4 nextest filters in parallel.
3. Revert swaps, git diff empty, landing check in background.

## Observations for report
- Root config claims "slowest ... well under 60 seconds"; under audit load root tests hit 85-180 s and
  table_corpus_has_similar_protocol_overhead TIMEOUT 180.4 s (builder-trap-diagnosis NOTES).
- Non-nextest legs have no termination: doctests (cargo test --doc), fuzz `cargo test --lib`, board `cargo run`, surface `cargo run`.
- Landing check clean at 02899c17 (check-1.log, check-1-legs/). Done; report delivered.
- Amended with reviewer repair: 1b9ba048 (G); diff vs 02899c17 == proposed-comments.patch; TOMLs parse.
