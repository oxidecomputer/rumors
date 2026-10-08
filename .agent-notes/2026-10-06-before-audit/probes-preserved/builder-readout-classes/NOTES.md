# builder-readout-classes: resumption notes

Task: MB3 brief (lanes/l7-suanpan/round-3/MB3-readout-high-part-classes.md).
Worktree /Users/oxide/src/rumors-slot-25, branch audit/suanpan-readout-class-table,
base 5c0d1c78 (verified HEAD, clean tree).

## Established
- Final carry c and low part M are fixed by (value, d) via Euclidean division by B^d.
- Domain: c in [-3,2] x {M=0, M!=0}; (-3, M=0) impossible; 11 reachable cells.
  Brief's table lumps c=0 and c=1 over M; I exhaust all 11 (adds c=0,M=0 redundant
  zero and c=1,M=0 = B^d).
- Expected touches derived from oracle value: d + d*[v<0 and M!=0] + [|v| >= B^d].
- Baseline landing check at 5c0d1c78 (QUESTIONS.md #27): before+suanpan 758;
  snapshots 142/142; board 5311 green with only the two known count_display x heap
  drift lines (branch predates the board-pin fix).

## Plan
1. Replace nonzero_high_parts_cost_one_touch_in_each_sign in metered.rs with
   readout_high_part_costs_one_touch_in_every_carry_class.
2. Build --no-run on box (cold slot), run the test.
3. Calibrate: env-var-selected mutant schema in read.rs (touch:c:z / value:c:z),
   run new test + suanpan suite minus new test per class; plus brief's two literal swaps.
4. Commit, landing check (check uptime first), report.

## Background jobs
(none yet)

## Progress (update)
- New test spliced into metered.rs (replaces nonzero_high_parts_cost_one_touch_in_each_sign);
  suanpan suite on box: 65/65 pass (base-suite.log).
- Calibration: swap.py schema applied to read.rs (REVERT with `swap.py schema revert`
  and confirm read.rs git diff empty). Running remote-calibrate.sh -> schema-calibration.log.
- Still to do: brief's two literal swaps (negative-minus-one, push-two-twice), commit,
  landing check (uptime first, load <= 300).
- Calibration done: schema (22 mutants) all fail new test (schema-table.txt);
  literal swaps negative-minus-one.log, push-two-twice.log both fail new test. read.rs reverted, diff empty.
- Committed 8fc5f716 (signed G).
- Landing check running: background task bymxdre3t -> landing-check.log. Then report.
- Landing check done at 8fc5f716: matches #27 baseline (758; 142/142; docs 193+3; surface 14, 211 items;
  wasm 8/25/43; board 5311 green with only the two count_display x heap drift lines). Logs in audit-logs/.
- Remaining: hand back report. Nothing else pending.
- Amended to a56e688c (G) per reviewer repair; landing check running -> landing-check-2.log
