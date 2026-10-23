# experiment-no-written-set: resumption record

Task: remove suanpan's `WrittenPositions` set; descents step one position at a
time (bounded by `lowest_written` where sound). Measure both boards (heap,
fuel) and a constructed gap-walk adversary at base and branch. Judge whether
the set earns its place. Throwaway branch; coordinator deletes it.

- Worktree: /Users/oxide/src/rumors-slot-74, branch experiment/no-written-set,
  base 1d09cc795 (verified HEAD == base, clean, at start).
- Box dir: ~/src/rumors-slot-74. Captures under ~/src/rumors-slot-74/target/caps/
  (rsync excludes /target/).
- Fixed env for every board run: AMP_BOARD_SHARDS=24, AMP_BOARD_FUEL_THREADS=24.

## Commits

1. 6999695fb (signed) fuzzfit: temporary gap-walk probe (amended from
   fcaa544db: rank check now compares encodings; format precision > u16::MAX
   panics). Guest exports
   ff_xp_gap_walk(mode, gap, rounds), ff_xp_mixed_{build,apply,check};
   harness bin `xp_gap_walk` (crates/before/fuzzfit/harness/src/bin/). Cargo.lock
   hand-edited: fuzzfit-guest gains "suanpan". Not on the board.
2. 7302bab63 (signed) suanpan: remove the written set. Lines: 661 set+bitset,
   ~75 maintenance/prose net in digits/normalize/sign, ~630 tests, 8 seed.

## Established

- `last_below` production readers: `Digits::descend_to_nonzero` (digits.rs:361)
  and the zero-partial branch of `compact_until_order_known` (sign.rs:80-88).
  Other reads are debug-only (activate's debug_assert, debug_assert_valid's
  contains, WrittenPositions::clear's precondition) or test inspectors.
- G reading: cost proof on unmerged docs/suanpan-time-bounds
  (accumulator/digits/costs.rs) defines G as growth of the working width from
  the update's starting top (re-growth counts); Q113 ruling agrees. A descent
  lowers the top by what it walks (+O(1)), so walks ACROSS operations are paid
  by earlier updates' G (potential = top). Sign queries stay amortized O(1).
- BREAK predicted in suanpan's own documented bound: an accumulator operand at
  a nonzero bit offset with digits [2^32, -1]* then 3 (the cost proof's own
  mixed-sign example) raises and cancels the top once per pair INSIDE one
  update; without the set each cancellation walks to the receiver's top:
  Theta(A * shift) vs documented O(A log(W+1) + G). Probe: ff_xp_mixed_*.
- BREAK predicted through before: Integrator::interval does
  total.add_shifted(S - d, &live) per region. Comb (depth S) + 2^t teeth with
  heights 2,0 around opening 1 -> live +1,-1 -> total's top cancels at every
  second tooth, walking ~(S-d)/32 digits; input pays O(1) bits per tooth.
  Rank, distance, lag, rank comparison (pair_order) reach it. Documented
  O(M(n)); walk ~K^2/64 for K = S = teeth. NOTE: this also means before's
  rank bound already relies on suanpan doing better than its DOCUMENTED bound
  (documented G charge per interval is ~(S-d)/32): a composition gap at base.
- Direct modes 0/1 (alternating re-add at gap h) are within suanpan's
  documented amortized bound under the G reading above, but break the touch
  pins in metered.rs (alternating_shifted_writes..., sign_query_skips...,
  limb_streams_select..., magnitude_reads..., primitive_conversions...,
  accumulator_operands_cost...) and tests/amortized_sequences.rs (claims an
  "intended bound O(n + d/32)", stronger than the documented table).
- Rank::sum_iter: nonnegative only, never cancels a top; other before writers
  shift 0 (width growth paid by input) -> walks paid by input.

## Base results

- Boards at 1d09cc795 tree: fuel 5729 green / 0 red; heap 5729 green / 0 red,
  pin clean (272 rows). Log base-boards.log.
- Direct probe at base: mode 0 = 608.0 fuel/round flat for gap 2..65536;
  mode 1 = 1085 flat (1097 at gap 2).
- before already has this family: WC(n) weight-comb (testing/meter.rs
  weight_comb), board rows version_rank/distance/lag/ranked_cmp x weight-comb
  at 7041->14081 B; tests/meter/version_scaling/skip_adequacy.rs and
  tests/meter.rs skyline_flatness band. My comb-teeth == WC independently.

## Branch landing check (7302bab63), log branch-audit-check.log, legs in branch-check-logs/

- ok: surface, docs, lints, wasm (pins 12/12, fuzzfit 25/25, fuelscape 43/43).
- FAILED board: 9 red, all weight-comb x {version_rank, version_distance,
  version_lag, ranked_cmp, ranked_encode, ranked_encode_rank, ranked_decode,
  ranked_serde_deserialize, ranked_borsh_deserialize}: touch exponent 1.97-1.99,
  touch constant (rank 75.1/B default, 298.5/B top). Worst-case pin drifts:
  touch rankings -> weight-comb; heap version_rank/ranked_encode(_rank) at
  acceptance scale arming-train -> wide-arming.
- FAILED fuel: 16 red across ladder: same 9 weight-comb cells (fuel e 1.37-1.52;
  rank 2569.3/B base -> 4490.4/B default, 10298.9/B top) + 7 small-input
  constant reds (count_borsh_deserialize propagate-seam/promo-rearm/dense-suffix/
  arming-train, count_serde_deserialize arming-train/hoisted-window, rank_add
  hoisted-window). Unexplained yet: check compare-fuel.
- FAILED tests: 860 run, 852 pass, 8 fail, all meters: suanpan metered
  abs_diff_negates..., accumulator_operands_cost..., alternating_shifted_writes...,
  magnitude_reads..., primitive_conversions...; amortized_sequences sign_flip;
  before meter skip_adequacy::skyline_rank_weight_comb (266255 touches vs 6414),
  pair_crossings::skyline_distance_jump_pair (164021 vs 158194; flat per byte,
  constant-factor). Snapshot filter did not run (leg stops at first failure).

## Background jobs

- b7pcix905 (done): base captures (fuel then heap) at 1d09cc795 tree.
- btgs9fuer (done): base probe at 6999695fb, log probe-base.log.
- bgvblvg28 (done): branch landing check.
- b3px6d0yf: branch measure job (snapshots, fuel+heap captures, compares,
  probe, calibrate-fuel). Log branch-measure.log; box files target/caps/*.
  Log: <scratch>/base-boards.log. Captures: box target/caps/base-{fuel,heap}.cap.

## Branch measurements (done)

- Probe at branch (branch-measure.log): direct mode 0 per round 480 (gap 2) ->
  1,704,364 (gap 65536), linear in gap; mode 1 812 -> 5,571,202. Comb-teeth
  rank/B 2493, 3489, 7481, 23456 (K = 1024..65536; base flat 2239-2243);
  control flat 2522-2517. Mixed-sign fuel/operand byte 1730, 6721, 26691,
  106563 (base 96-98).
- Snapshots 142/142 pass. Captures box target/caps/branch-{fuel,heap}.cap;
  local copies of compares + calibrate in caps/.
- Fuel calibrate: 3341 cells moved, 81 up, 2626 down, median -1.0%; weight-comb
  integrals +156..+297%; count_*_deserialize / rank_add small-input +9..+34%
  (measured code untouched; inferred allocator state from setup).
- Heap compare: 540 heap rows moved, none up; query_*_many -15% (inferred:
  32-byte inline set per accumulator), min_ticks dense-suffix 4.9 -> 4.3 B/B.
  Touch: 732 rows up, all flat per byte except weight-comb (37 -> 298 /B).
  Scan unchanged.

## State

Complete. Branch committed and clean (6999695fb, 7302bab63, both signed). Box
tree has a calibrated fuel.rs left by calibrate-fuel (not committed; next sync
restores it). Handed back to coordinator.
