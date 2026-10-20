# fixer-range-minima-wide: resumption record

Worktree `/Users/oxide/src/rumors-slot-58`, branch `fix/range-minima-wide-boundaries`,
base `main` = `6a8dab7cc` (verified HEAD at start).

Execution: ox-east-1 is DOWN; build locally under `nice -n 10`,
`CARGO_BUILD_JOBS=5`, nextest `--test-threads 5`, `AMP_BOARD_SHARDS=5`.
Long commands: harness `run_in_background`, never `&`. Commit `--no-gpg-sign`.
Stay out of suanpan `Digits::reset` / rank integrator (slot 57) and slot 17.

## Established (by reading, at 6a8dab7cc)

- `range_minima/boundaries.rs`: word boundaries (stored_bits <= 64) are packed
  in `compact: PackedU64Stack` (2w bits each); wider ones go whole into
  `wide_values: Vec<Accumulator>`. `Accumulator` = RefCell<Value>, Value =
  Option<i128> + Digits; estimated 128 bytes. suanpan small form holds
  |v| <= 2^96 inline (stored_bits <= 96); 32-bit digits in i64 cells.
- Stepped spine at e=65: ~2e+4 = 134 input bits per level, ~17 B; the slot
  is 128 B, up to 3x at a Vec reallocation peak (peak_alloc counts old+new).
- Lesson 5.2 (2026-10-08 notes): any representation that reads a wide
  boundary on push/pop is quadratic on reveal-comb (one wide value circulates
  boundary -> deferred -> boundary at O(1) input per cycle). So wide moves
  must stay O(1); only bounded-width values may be packed on push.

## Plan

1. Probe (uncommitted) min_ticks heap/B on stepped spines across e and depth
   just past powers of two; also reference-dense e=65.
2. Demo commit: family `wide-step-spine` (or similar), e just over 64.
3. Fix: candidate = pack boundaries with stored_bits <= 128 as u128 (O(1)
   read), keep wider ones as accumulators; measure e=128..160 for the
   residual struct-slot regime; consider boxing if it stays high.

## Background jobs

(none yet)

## Progress (update)

- Base probe (probe-base.log): Accumulator = 128 B; WS e=64/65 ~26 B/B at
  depth 2^k+2, 13 at 2^k; e=97 17.5; e=128 13.6; e=256 8; tick unaffected.
  WP e=65 13.4 at d=2000, 25.5 at d=2100 (shares cause).
- Owner ruling 2026-10-20 (coordinator msg): NO known-bad variants; no tests
  justified only by imagined regressions.
- Demo commit e9ab04ba2 (--no-gpg-sign): family wide-step-spine (e=64,
  base depth 260). Board at e9ab04ba2 (scratch wt, demo-commit-board.log):
  1 red: version_min_ticks x wide-step-spine heap 25.9 (4307->8630),
  25.8 (17275->34565); small 15.3 green; drift only version_min_ticks x heap
  (pinned propagate-seam 12.7) at both scales.
- Fix design (in tree, uncommitted): Boundaries copies wide boundaries with
  stored_bits <= INLINE_BOUNDARY_BITS (256) as limbs+count into `compact`,
  `whole` BitStack tag, `whole_values` Vec for wider. Mirrors min_ticks
  ChunkStack. Must lower WIDE_STEP_PAIRS_HEAP_CEILINGS in
  tests/meter/version_scaling/minimum_resources.rs if readings fall.
- Scratch worktree: $S/wt (detached), build dir /Volumes/forge/build/78/e6d6d6dba63b57.
  Use it for the base capture at 6a8dab7cc next; remove it at the end
  (git worktree remove, then delete its forge build dir).

## STOPPED (proposal) -- state at stop

- Branch fix/range-minima-wide-boundaries = e9ab04ba2 (demo only), slot clean.
- Copy variant (candidate-copy.patch): heap proportional; wide-step-spine
  25.9->1.7, propagate-seam 12.7->1.8; but touch rises: seam_plunge large
  21064->25160 > DESCENDING_TOUCH_CEILING 23099; surplus band -253 < 1514;
  board touch +0.23/B propagate-seam, +0.15/B wide-step-spine, uneven on
  dominated-undercut/cancelling-chain. => ceiling would rise: STOP.
- Box variant (candidate-box.patch): touch-neutral, green, wide-step-spine
  25.9->12.2, propagate-seam 12.7->10.9, -352/-480 constant gains, but heap
  rises on promo-rearm/dense-suffix (+0.065/B), wide-step-pairs (+0.098),
  arming-train (+0.005); not proportional.
- Logs: copy-board.log/copy-compare.log, box-*.log, fix2-minimum.log,
  base-minimum.log, probe-{base,fix}.log, demo-commit-board.log.
- Remaining: base capture + compare (base-vs-demo-compare.log), then remove
  $S/wt (forge dir /Volumes/forge/build/78/e6d6d6dba63b57), then report.
