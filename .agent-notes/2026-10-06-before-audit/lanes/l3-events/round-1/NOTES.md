# auditor-l3 resumption record (lane L3, events)

Scratch: `/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/auditor-l3/` (call it `$S`).
Worktree: `/Users/oxide/src/rumors-audit-l3-events`, branch `explore/l3-events`, base 58285ca5 (verified at start); probes committed at 30379f8d; later commits 01a24058, 1a8601b3 (heap probes), bab0a381 (reach, counter search), 873f3999 (tick heap search). All signed (G).
Box mirror: `ox-east-1-agent:~/src/rumors-audit-l3-events` (builds in its own `target/`).

## Remote command prefix (coordinator, 2026-10-07)

Begin EVERY remote command with `unset CARGO_TARGET_DIR; export NEXTEST_TEST_THREADS=24;`.
If a running test times out or reports something impossible, suspect load first; rerun once under the cap.

## Read before acting

common.md, auditor.md, lane-l3-events.md (all under
`/Users/oxide/src/rumors/.agent-notes/2026-10-06-before-audit/briefs/`), this file, and
`$S/coverage-map.md`.

## Tree state RIGHT NOW

- Clean at 30379f8d (probes committed, signed). Both mutation batches reverted; `git status`
  empty (verified). Patches kept in `$S/mutations-batch{1,2}.patch`, script `$S/apply-batch2.py`.

## Background jobs

Running (started ~15:15 local, 2026-10-07), all on the clean tree 30379f8d:
- release heap probes: `$S/run-heap-release-1.log` (nextest --release, filter l3_heap, --no-capture).
- long run A: `$S/long-A.log`: STOPPED by me at 45 min (exit=255) after these passed 100,000
  cases each: l3_min_ticks_tight, l3_min_ticks_floor_over_histories, l3_cogen_bushy,
  l3_min_ticks_tight_spine, l3_family_bushy. Spines restarted detached (next bullet).
- DETACHED spine run on the box: pid 5713 (cargo; test binary 25758), started ~16:05;
  l3_cogen_spine PASSED 30,000 cases (verified in the box log at ~16:50); l3_family_spine running; log ON THE BOX at
  `~/src/rumors-audit-l3-events/target/l3-long/spines.log` (read with ssh); PROPTEST_CASES=30000,
  l3_cogen_spine + l3_family_spine, 2 threads, nice 19, tree 873f3999. Kill with `kill 5713` and its
  child test binary.
- long run B: `$S/long-B.log` (libtest, PROPTEST_CASES=5000, L3_DEEP_CASES=300, l3_family_wide,
  l3_family_multi_wide, l3_deep_spine, 4 threads).
- release diagnostics (reach of wide generators, counter worst-case search): `$S/run-diag-release-1.log`
  (commit bab0a381; tests l3_generator_reach, l3_counter_search).
Done: calib-batch1b/1c/1d.log (batch 1), calib-batch2.log (batch 2), heap runs
run-heap-release-{1,2,3}.log.
Killing local ssh does NOT stop a remote job; find remote roots via pwdx on `bash -c`.

## Established so far (verified by runs on ox-east-1)

- Unmutated, all probes pass at 2000 cases (`$S/run-probe-1.log`, exit 0) and at 1000 cases in
  the calibration M0 runs; the committed suites pass (72/72).
- Generator histogram (`$S/run-hist-1.log`): committed independent arbitrary pairs have no
  lookahead site in 81% of draws, nesting <= 2, depth >= 4 in 0.1%, never two sites in one
  range. My co-generated spine reaches nesting 7+ in 42% and multi-site ranges in 25%.
- Calibration so far (probes / committed): M0 clean/clean; M1 route tie-left 4 fail / 10 fail;
  M2 expansion tie-left 4 fail / 5 fail; M3 memo tie diverges 6+wide fail / 15 fail;
  M4 resolve_inner sign 4+wide fail / 2 fail; M5 latest_from_first skip 2+2wide fail / 2 fail;
  M6 Min-arm tie: probes cogen_spine, family_spine, family_wide fail / committed 72 of 72 PASS
  (verified in calib-batch1d.log). Shrunk witness (flag tripped but oracle fill is identity):
  version Node(0, Leaf(3), Node(0, Node(0, Leaf(3), Node(0, Leaf(3), Leaf(0))), Node(0, Leaf(0),
  Node(0, Leaf(3), Node(0, Leaf(3), Leaf(0)))))), party (1, ((1, (1, 0)), (1, (1, 0)))): an outer
  lookahead whose range holds two sibling sites, each nesting a site, minima tied;
  M4/M5 committed catch only via two hand-built-family tests (family_pairs_tick_and_flag_identically,
  undercut_under_a_live_relation_family_ticks_identically); probes 4-5 tests each.
  M7 raise successor repair: 7+2 / 21 fail. M8 pop_lookahead skips resolve_deferred: probes
  cogen_spine, family_spine, family_wide fail (deep_spine timed out) / committed 72 of 72 PASS
  (second escape). Witness (flag): version Node(0, Leaf(2), Node(0, Node(0, Leaf(2), Node(0, Node(0,
  Leaf(2), Node(0, Leaf(2), Leaf(0))), Leaf(3))), Node(2, Leaf(1), Leaf(0)))), party (1, ((1, ((1,
  0), 0)), (1, (0, 1)))). M9 negative control: all pass everywhere (no false alarm).
  M10 memo index: 2+2 / 2 fail. M11 min_ticks Close::Equal skip: 7 probes fail, but ALL via the
  debug assertion in HeightPrefixes::settle, not the floor/tightness assertions; 7 committed fail.
  Floor/tightness calibration therefore moved to batch 2 (M22 = +1, M23 = -1 on the result).
  M12 trailing repair dropped 9 / 27; M13 ticks n not n-1: 6 / 6; M14 seed_relation offset: 6 / 4.
- Batch 2 (calib-batch2.log): M0 clean everywhere (multi_wide passes). M15 begin_scan clears one
  block: ONLY l3_family_multi_wide fails ("a memo slot is written once"), committed 72/72 PASS.
  M16 reference_level decrement: probes 5 + wide 2 / committed 6. M19 block 1 allocated late:
  ONLY family_wide + multi_wide fail (index out of bounds), committed 72/72 PASS. M21 cursor not
  reset: 6 / 9. M22 (+1): floor fails ("min_ticks 3 exceeds the 2 ticks in the causal past after
  Send(0, 0)") and both tightness probes; committed 7. M23 (-1): exactly the two tightness probes;
  committed 6.
- Escapes from the committed suites, caught only by co-generated probes: M6, M8, M15, M19.
- Timeouts: l3_family_wide at 1000 debug cases and l3_deep_spine at 32 cases exceed nextest's
  180 s; they run at low counts (40 and 4 or 8) or should run via libtest without a timeout.
- Clock wrappers are one-line compositions (clock.rs:117-600) and laws already state them.
- Party grammar cannot encode an empty party (party/io/validate.rs:37-52), so tick's
  "party owns a region" precondition always holds.
- Oracle (testing/oracles/tree/version.rs fill/grow/event) checked by reading against
  itc2008.md 5.3.4: faithful; lexicographic (expansions, depth) equals the paper's N-cost.

## Open hypotheses

1. CONFIRMED (release, run-heap-release-1.log): min_ticks on the rising right spine `(0,(1,(2,…)))`
   peaks at 19.70 B/B near powers of two and 26.27 B/B at 1.5 x 2^k (n = 1536 .. 393216); board
   per-sample ceiling `1024 + 20 * n` is exceeded (n = 98304: ceiling 1,229,844 vs 1,613,872).
   Mechanism: `Boundaries::payloads: Vec<StoredContribution>` (8 B per positive boundary,
   range_minima/boundaries.rs) at ~5 input bits per boundary; PeakAlloc's realloc counts old+new
   (peaks 12,656 = 8*(512+1024)+368 at n=1024; 25,264 = 8*(1024+2048)+688 at n=1536). Only
   min_ticks uses a non-ZST RangeMinima payload (measure/min_ticks/minima.rs:115). Board cell
   `version_min_ticks` (testing/meter/board/ops.rs) uses the default model. Fine sweep around
   powers of two: run-heap-release-2.log (probe l3_heap_min_ticks_rising_spine_fine, 01a24058).
2. CONFIRMED (run-heap-release-3.log): jump-entered rising spine `(0, (J+1, (J+2, …)))`: J = 2^31,
   2^40 read ~122 B/B (163 at 1.5*2^16); J = 2^200 up to 197 B/B; J = 2^16 control 19.7. Fine
   sweep (run-heap-release-2.log): plain spine 39.4 B/B at n = 2^k + 2 for k = 8..18, RED vs board.
   Written up as D1: defect-D1-min-ticks-heap.md, test-brief-D1.md, fix-note-D1.md.
3. Tick memo heap: measured linear, <= 11.7 B/B (chain), 6.7 (siblings), 0.9 (zero links).
   Counter search (run-diag-release-1.log): tick max 13.5 touch/B, 47 scan bits/B; min_ticks 8.8, 8.0.
   Reach: wide >64 slots in 66% (max 301); multi-wide two >64-slot scans in 25%; committed max 2.
   Tick heap random search (run-heap-release-4.log, 873f3999): max 43% of board ceiling; no tick heap finding.
4. min_ticks floor over histories: probe passes; no proof; keep as a property.

## Round 1 ended (coordinator request, ~16:50)

Report returned with D1, MB-1, MB-2, S1, inventory, coverage record. Round 2 starts by reading:
long-B.log (local; fragile: streams through my local ssh, may die when my session ends) and the box
log target/l3-long/spines.log. Prefer release builds for future long property runs (coordinator).

## Next steps

0. auditor.md now requires an inventory entry per probe in the report: draft is
   `$S/instruments-inventory.md` (calibration column computed from the logs). Still to measure:
   memo slots per pre-scan reached by arb_wide / arb_multi_wide (after long runs; do not rebuild
   the test binary while long runs execute).

1. Wait for batch 1 to finish; record which mutations each instrument catches.
2. Kill nothing else; revert batch 1 (procedure above); confirm `git diff`; commit probes.
3. Apply batch 2, save patch, run (multi_wide + wide + committed), revert, verify diff.
4. Release heap probes; then long probe runs (libtest, no timeout, nice -n 10).
5. Fill calibration tables into `$S/machinery-brief-cogen.md` and
   `$S/machinery-brief-min-ticks-histories.md`; finish `$S/observations.md`; report.

## Drafts in $S

coverage-map.md, observations.md (O-1..O-4, line numbers verified at HEAD),
machinery-brief-cogen.md, machinery-brief-min-ticks-histories.md, STATUS.md.
