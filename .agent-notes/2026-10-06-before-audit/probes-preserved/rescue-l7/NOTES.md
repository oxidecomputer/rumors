# rescue-l7 NOTES (resumption record)

Task: catalogue every instrument on explore/l7-suanpan (tip 5d5e33471, base 0bdeb588d)
per coordinator-briefs/instrument-rescue.md. Output:
/Users/oxide/src/rumors/.agent-notes/2026-10-06-before-audit/instrument-rescue/07-suanpan.md
Do not commit. <=3 box runs on scratch worktree /Users/oxide/src/rumors-rescue-l7 (detached),
remove with plain `git worktree remove` at end. Already carried: touch-bound #37, fuel ladder #52,
wasm32 landings #54 -> one line each; full entries for anything they left behind.
Report: count, top three, anything not assessable.

## Read so far
- common.md, auditor.md, instrument-rescue.md, README, baseline, instruments.md,
  survey (intro, 2.1, 4, 5, rulings), QUESTIONS header/notices 96/98/88,
  ready entries 27, 28, 37, 48, 50, 52, 54, 64, 66, 86, 89
- lanes/l7 round-2/inventory.md, round-3/inventory-additions.md
- 00-baseline.md does not exist (instrument-rescue/ empty at 12:43)

## Established

## Next
- rest of lane records; git log/diff of branch; read each added file

## Milestone 1 (read whole diff 0bdeb588d...5d5e33471, 22 files, +2402)
Instruments found on branch:
- suanpan explore_l7.rs: pool model (Op enum 19 kinds, swarm weights, GapRound, Park, near_threshold,
  arb_shift boundary set, check_stable, check_conversions, observe); metered::work + meter_program;
  l7_touches_are_linear_in_table_work (8w+64) [#37]; l7_replay_touch_repro1; l7_probe_repeated_comparison_cost [#89 pin];
  l7_adversarial_touch_search (ignored); l7_check_readout + l7_probe_extreme_high_parts [#64];
  probes: zero_shift_history [#48], stability_huge_width_compacts [#50?], reserve_digits_extreme [#28]
- suanpan prototypes: l7-proto (T tags + tag invariant check), l7-proto-w (written.rs + assert_invariants) [#86],
  l7-o1 [#89]; l7-high-log readout census (read.rs)
- before: l7_compare.rs counters + anchor.rs hook + board/l7_compare_probe.rs (left_chain)
- before/tests/explore_l7.rs size probe; fuzzfit/harness/tests/explore_l7.rs ladder [#52]
- wasm32-pins: l7_builders.rs (L7Bits, L7Wide, families 0-5, lowered), guest l7_fuel modes 0-11,
  l7_suanpan workloads, run_fueled, Check::L7Fuel=9, zz_l7_fuel_matrix (ignored),
  landing cases 5,6 (D1), 7,8 (Q1) [#48?], 9 (U1-a) [#50], 10,11,12 [#54 has 10,12]
- scratch (not on branch): S/mutate.py etc. at scratchpad/auditor-l7
Next: read main suanpan tests, ready branches #37 #48 #50 #52 #54 #64 #86 #89 #28 for what was carried.

## Milestone 2 (carried items and logs, verified)
- main now ce67ab083 (briefs: nextest limit 300 s; .config/nextest.toml terminate-after 5 x 60s).
- #37 touch_bound.rs ports the pool model's GENERATOR (pool of 3, swarm, arb_word, near_digit_multiple,
  arb_shift incl. 3200..64000, arb_bits, Park(neg only), GapRound, + CancelledChain); no value oracle, no Convert.
- surface.rs (main): one accumulator, fresh operands, shifts <=1024, cmp_zero after every step (compacts),
  stored-width check bits<=stored_bits+2 (weaker than rustdoc accumulator.rs:352 "< 2.01*2^stored_bits");
  no Park. Pool model observe() states the 2.01 bound + exact max (implied in digit form by private
  assert_invariants digit bound).
- #48 carries cases 7/8 (SuanpanZeroShift) + representation property; #50 carries case 9 (stronger, digit 2)
  + witness maximum_adjustment_width_still_compacts; #28 reserve tests; #64 table test; #89 pin
  comparison_skips_only_a_write_back_that_restores_its_digits; #86 written_positions_match_an_ordered_set.
- #52 ladder ops: Decode, LessOrEqual, MinTicks only; fuzzfit guest exports join/meet/tick/project/cmp already.
- main landing cases 1-4; explore case 5 (limb index 2^31, shift 0 -> digit 2^32) not on main or #54.
- Kill matrix from logs (S=auditor-l7): pool model fails M1 (timeout while shrinking, 567 stable-sign panics),
  M2, M3, M4, M5, M7 (None-width clause line 169), M9, N1 (timeout while shrinking, normalize width line 364).
  M6: pool PASSES in calibrate.out and calibrate2.out; calibrate2's touch FAIL = owned-sub pricing bug
  (minimal input OwnedClone negative), fixed c100e028. Records' "M6 caught" claim unsupported.
  M8 = production debug_assert. Committed suite (calibrate_existing.out) catches M1-M7, M9, M12, M16; misses M14, M15.

## Milestone 3: box run 1 of 3 DONE (rescue-l7/run1.log, exit 0), worktree /Users/oxide/src/rumors-rescue-l7 @5d5e33471 + census edits
Runtime (default 256 cases, debug, serial, nice 10, load ~21): pool 3.72 s; explore touch 1.43 s; surface 2.06 s.
Census, 1000 programs each, TestRunner::deterministic (pool | surface):
 steps 58249 | 40775; redundancy>=2 after step 61 (0.10%) | 58 (0.14%); >=8: 16 | 2; >=64: 5 | 0; max 1969 | 33
 programs with redundancy>=2: 32 | 51; queries on redundancy>=2: 17/9249 | 58/40775
 operand uses 12500 | 23153; operand redundancy>=2: 13 | 0; operand >=2 zero ranges: 4165 (33%) | 0; self-aliased 1047 | 0
 receiver >=2 ranges: 19043 (32.7%) | 14788 (36.3%); max ranges 66 | 11; programs 728 | 876
 extreme digit after step: 9302 (16%) | 657 (1.6%); programs 590 | 202
 normalize on extreme: 372/2936 (12.7%) | 36/1621 (2.2%); shift on extreme: 410/3013 (13.6%) | 54/3183 (1.7%)
 redundant zero states 5 | 3; max digits 5112 | 116; >64 digits 13503 (23%) | 313 (0.8%); >512: 6576 (11%) | 0
surface.rs identical explore tip vs main (verified).
Follow-up "surface never parks at bound": surface does reach |d|=2^33-1 organically in 202/1000 programs.
Next: run 2 = census of #37's generator (apply #37 diff into scratch worktree), same metrics via readout.

## Milestone 4: box run 2 of 3 DONE (rescue-l7/run2.log, exit 0). Census code saved: rescue-l7/census.diff
#37 generator census (1000 programs, deterministic, counted per pattern PART): steps 95187;
 redundancy>=2 5302 (5.6%), >=8 4086, >=64 2655, max 16364; programs 553; queries on red>=2 2637/35766 (7.4%)
 operand uses 11964; red>=2 10; >=2 ranges 3649 (30.5%); aliased 1086; receiver >=2 ranges 35622; max ranges 160
 extreme after 17676 (18.6%), programs 588; normalize on extreme 474/3263 (14.5%); shift 514/3184 (16.1%)
 redundant zero 1346; max digits 16382; >512 16829 (17.7%). census secs 5.6
Verified F1b ±S decode r=64: 6664878/32292 = 206.4 (m3/current-f2.log).
Decision: no run 3. Next: write 07-suanpan.md; then clean & remove scratch worktree (plain remove).

## Milestone 5: writing 07-suanpan.md
Plan: 11 full entries (pool oracle; fuel-matrix ops; mutant schema; adversary; landing cases 5/6; offset accounting;
readout census; T prototype; size probe; replay probe; scratch drafts) + 15 carried one-liners = 26.
Mutant schema: 27 mutants (M1-9, M12-16, Z1-5, N1-3, K1, L3S/D/W/H); 12 target zero_ranges.rs.
M1 pool panics 834, N1 1854 (both nextest timeouts while shrinking). Cases 4+5+6 = 70.2 s (wasm3-fix.log).
After writing: restore scratch worktree (census.diff saved), git clean its untracked dir, plain worktree remove.

## DONE
- Wrote instrument-rescue/07-suanpan.md (uncommitted, as briefed): 26 instruments, 11 full + 15 carried.
- Scratch worktree removed (plain `git worktree remove`, clean). Box dir ~/src/rumors-rescue-l7 remains (for coordinator).
- Box runs used: 2 of 3.
