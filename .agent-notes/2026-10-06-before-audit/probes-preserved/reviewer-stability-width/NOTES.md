# reviewer-stability-width NOTES (round 1)

Branch fix/suanpan-stability-width, worktree /Users/oxide/src/rumors-slot-18, tip 62519bb6 (base 94365fa5).
Verified: worktree clean, HEAD=62519bb6, both commits signed.

## Established
- Fix diff read: cmp_zero_stable_above(u64), index widened to u64 via try_from().expect.

## Open
- items 1-6 per task prompt

## Background jobs
(none yet)

## Item 4 (verified by reading)
- before callers: boundary.rs:104,114; anchor.rs:149,154,165; filter.rs:225,240. Widths = stored_bits() or 64.
- Digits buffer is Vec<i64>; on wasm32 len < 2^28, so stored_bits < 2^33 << 32*2^32. before cannot reach the defect. 64-bit: usize==u64, old/new identical.
- From the public entry, adjustment_high <= 2^59-1 (u64::MAX.div_ceil(32)-1), so saturating_add(2) never saturates via Accumulator; only direct Digits calls.

## Item 1 hypothesis (experiment in progress)
- Pin decides at index 0 (value 5 at digit 0) -> answer check vacuous for any threshold >= 1.
- Mutant: `index >= (adjustment_high as usize as u64).saturating_add(2)` (truncating widening). Expect: committed pin PASSES on wasm32.
- Strengthened: guest uses b as digit of the 5 (b=2): mutant -> Failed(WrongValue) at FIRST_UNINDEXABLE.
- Edits are temporary in sign.rs, guest checks.rs, harness pins.rs; restore and confirm git diff empty.

## Item 1 RESULT (verified on box)
- mutant.log: truncating mutant -> committed pin PASS; probe low=2 width=137438953473 Failed(WrongValue). suanpan recompiled into guest.
- control.log: tip, no mutant -> probe low=2 all Passed.
- Worktree restored, git diff empty (verified).
- Finding B1: pin's answer check vacuous (decides at index 0); fix: put the 5 at digit 2 (expected stored count 3).

## Items 2,3,5,6 (by reading/analysis)
- Item 3: all 14 moved ops reach the query: tick/send/recv/version_tick(s) via version/tick.rs RangeMinima (anchor.rs, boundary.rs);
  min_ticks via measure/min_ticks/minima.rs RangeMinima; query_* via causally/query.rs -> place::filter::compare_heights.
  fuelcmp.py: tick paths deltas multiples of 2; min_ticks multiples of 4; query_* one overlay sample, multiples of 17 (-1088=64*17).
  Codegen inference sound. Commit msg WRONG: says -17 on "range-minima anchor's path" (it's place filter / query_*), omits min_ticks step 4.
- Item 2: family oracle checks answer==cmp_zero sign; truncation mutant returns the TRUE sign -> family would bless it. 70s runtime.
  Recommend: do not commit family; commit strengthened pin (value at digit 2).
- Item 5: dispositions confirmed by own grep of suanpan non-test source.
- Item 4 record: title + mechanism para still say ">= 32*2^32"; failure family says ">". Record lacks before-unreachability statement.
- Prose: harness/native/commit say "5 above a cancelling top" -- 5 is BELOW (digit 0 vs 9-10). Fn doc "a bound ... is never established" mirrors pre-existing comment usage; suggestion only.
- Max adjustment_high via public API = 2^59-1, so saturating_add(2) never saturates from Accumulator.

## Background jobs
- landing check: $S/landing.log (bg task blcmt8ju3). After it: re-run pin at base by `git diff 17e0c5b6 62519bb6 -- crates/suanpan | git apply -R`, then restore.

## DONE
- landing.log + landing-logs/: matches pre-board-pin baseline (tests 758+1 skip, 142 snapshots, docs 193+3, surface 14 & 211=134+77, wasm 9/25/43, board 5311/0 + exactly two count_display x heap drift lines).
- base-pin.log: fix reverse-applied -> pin fails `SuanpanStabilityWidth failed for (137438953473, 0)` left Failed(WrongLength). Restored, git diff empty.
- Verdict: changes required (B1 pin vacuous on answer; B2 commit msg fuel attribution; B3 "above" wording). Report handed back.

## Round 2 (tip cf99aef9, commits 2b1eeba1 + cf99aef9, signed)
- r2-mutant.log: truncating mutant -> new pin Failed(WrongValue) at (137438953473, 0). Restored clean.
- r2-control.log: tip -> pin passes; native witnesses pass.
- B1/B2/B3 resolved. Declined is_ok_and suggestion: argument sound enough. Verdict: approve.
