# fixer-stability-width

Worktree /Users/oxide/src/rumors-slot-18, branch fix/suanpan-stability-width.
Base 17e0c5b6 (signed G, demonstration commit on 94365fa5), verified clean at start.
Record: .agent-notes/2026-10-06-before-audit/lanes/l7-suanpan/round-3/U1a-stability-width-test-brief.md

## Plan
1. Fix: Digits::cmp_zero_stable_above takes adjustment_high: u64; index widened by
   u64::try_from(index).expect (precedent read.rs:44); caller drops usize::try_from(..).ok()?.
2. Remote: wasm32-pins at fix (new pin must pass); native suanpan suite with touch-meter.
3. Fuel: fuelscape --dump with small plan on ops reaching the stability query, at parent
   (17e0c5b6) and fix; diff. Native touch: exact pins in metered.rs + board in landing check.
4. fmt --check root + wasm32-pins; clippy; commit signed; landing check in background.

## usize narrowing dispositions (from reading, 17e0c5b6)
- accumulator.rs cmp_zero_stable_under: usize::try_from(adjustment_digits-1).ok()? -> THE DEFECT.
- same fn, small branch: u32::try_from(u64) -> width-independent, fine.
- sign.rs cmp_zero / compact_until_order_known: index is a held buffer index; no narrowing.
- read.rs normalized_limbs_with_shift: u64::try_from(start) widening a held index; fine.
- operators.rs shifts: u64::try_from(shift) value-based on every width; fine.
- digit_index(u128)->usize: memory-holding exception (round-2 sweep).
- reserve_digits(usize): owner's question 6; not this branch.
- before rank.rs reservation hint: moves with question 6; not this branch.

## Progress
- parent.log: base pin fails verbatim `SuanpanStabilityWidth failed for (137438953473, 0)` left Failed(WrongLength) right Passed.
  parent fuzzfit guest sha256 f37d449b399f610e9f0a8a3c58c50654f656001cecdc1fcf8d6dcffd40fd61ef;
  parent fuelscape dump (--samples 20 --max-bytes 128, default seed) at box ~/src/rumors-slot-18/target/fuel-parent.
- Scratch family check applied UNCOMMITTED (protocol Check 10/11, guest fns, harness zz_scratch_stability_family);
  MUST be removed before commit (string swaps; source in this dir family_*.rs).
  Base calibration (family-base.log): 166/600 compactable; 498 failures = 166*3, all at unindexable widths.
- Fix applied in worktree (sign.rs u64 adjustment_high; accumulator.rs drops usize narrowing).
- fix.log: family 0/4800 failures (coverage 166/600); pins 9/9 (+1 skipped scratch); suanpan 65/65 all-features (14 metered);
  clippy -p suanpan clean. Fix fuzzfit sha bd16840b... (parent f37d449b...).
- Fuel: 14 ops moved (fuel-cmp.log, fuel/ dir). Quantized: +2 steps tick/send/recv/min_ticks, -17 steps query_* (only the
  concurrent_pair x hugeleaf size-80 overlay). Flat ratio across sizes. My first hypothesis (behavioral compaction) was WRONG:
  before's widths are stored_bits (<= 32*(2^32-1) on wasm32) or 64, never past the boundary; so codegen.
- Scratch removed (git diff on crates/before empty). Committed tip 62519bb6 (signed G).
- Landing check launched -> landing.log (background).
- Landing check at 62519bb6 (landing.log, landing-logs/): matches baseline. tests 758/758 +1 skip (757+1 witness), snapshots 142/142;
  docs 193 + 3; surface 14, 211=134+77; board 5311 green 0 red + exactly the two count_display x heap drift lines;
  wasm32-pins 9/9 (8+1 pin), fuzzfit 25/25, fuelscape 43/43; lints ok. DONE; report via handback.

## Round 2 (review findings B1-B3)
- Round-1 commits: test 17e0c5b6, fix 62519bb6 (patch saved fix-r1.patch). Rebuilding: reset --hard 17e0c5b6 (tree clean),
  amend test commit (5 at digit 2 via add_shifted_limbs(64,[5]); expect 3 digits; i128 == 5<<64; prose: 5 BELOW the 2-digit top),
  re-apply fix + wording "always yields None"; commit fix with corrected fuel attribution
  (tick family + min_ticks via RangeMinima: +2 steps tick, +4 min_ticks; query_* -17 via causal query place filter compare_heights).
- Declined reviewer's usize::try_from(threshold).is_ok_and: brief wants u64 comparison.
- Then runs: base (reverse fix) pin fails; mutant (reviewer swap.py MUTANT line) pin fails; tip just wasm32-pins; landing check.
- R2 commits: test 2b1eeba1 (G), fix cf99aef9 (G). r2-base.log: pin fails WrongLength at (137438953473,0); witnesses pass.
  r2-mutant.log: truncating mutant -> WrongValue at (137438953473,0); restored, git diff empty.
- Landing check at cf99aef9 -> landing-r2.log (background).
- landing-r2.log at cf99aef9: matches baseline (tests 758+1 skip, 142; docs 193+3; surface 14, 211=134+77; board 5311/0 + exactly
  the two drift lines; wasm 9/25/43; lints ok). One nextest LEAK marker on before own::tests::from_impl_is_to_version (passed;
  absent in round 1; unrelated code). DONE round 2.
