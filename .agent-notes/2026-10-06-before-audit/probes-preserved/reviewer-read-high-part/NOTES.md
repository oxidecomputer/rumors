# reviewer-read-high-part resumption record

Task: round-1 review of two stacked branches in /Users/oxide/src/rumors-slot-07.
- audit/suanpan-readout-high-touch = d4fd5e9f (pin, machinery) on base 719f9e57
- simplify/suanpan-read-high-part = 12231140 (simplification) on d4fd5e9f
- Leave slot on simplify/suanpan-read-high-part, clean. Commit nothing.
- Brief: .agent-notes/2026-10-06-before-audit/lanes/l7-suanpan/round-1/S1-read-digits-high-part.md
- Builder evidence: scratchpad/builder-read-high-part/

## Established
- Base verification: HEAD 12231140, clean; 719f9e57..455e97de differs only in notes.
- Carry-bound proof (read.rs:5-9) checked by hand: |digit| < 2B = DIGIT_LIMIT (strict, digits.rs:49-50),
  so digit in [-2B+1, 2B-1]; + carry in [-3,2] -> [-2B-2, 2B+1]; floor/B -> [-3, 2]. Correct, tight.
- Pin arithmetic by hand: positive digits 2B-2: d0 -> low B-2 carry 1; then 2B-1 -> low B-1 carry 1; final 1.
  Negated: d0 -2B+2 -> low 2 carry -2; then -2B -> low 0 carry -2; final -2, M=2 nonzero; complement d; high 1.
  Touches d+1 and 2d+1. Correct.
- Builder calibration: base drop_touch survives 64/64 without pin; with pin each fails only pin (calibrate_base.out).

- arith.py (scratch): closure exhaustive at B=2,4,8,16, endpoints at 2^32; old loop == new push for all
  carry x low_nonzero; pin readout simulated: (1, d+1) and (-2, 2d+1). All pass.
- Premise analysis: count==d plus value 2B^d-2 >= B^d forces high part exactly 1 in both signs and M!=0
  negative; so assertion suffices for touches, but message "each doubled digit stays in place" overclaims.
  Non-blocking suggestion.
- Readout is &self (accumulator.rs:376-410): no mutation between the two readouts.
- Pin commit msg claims skip-or-repeat "passed every test"; builder only checked drop, suanpan only.
  Running mutants.sh (both-site drop / repeat; workspace nextest + amp-board-acceptance) on audit branch,
  slot switched to audit/suanpan-readout-high-touch. Logs: mutant-drop.log, mutant-repeat.log,
  mutants-driver.log. Script reverts and checks git diff empty. DO NOT edit worktree while running.

- RESULT mutants (workspace nextest, audit branch): drop-both 1863 passed / 1 failed (only the pin,
  metered.rs:463); repeat-both 1863/1 (only the pin). Board step skipped: on-illumos.sh runs `set -e`.
  Tree restored clean. Now boards.sh (board-drop.log, board-repeat.log, boards-driver.log) running.

- RESULT boards: drop and repeat both 5311 green / 0 red (board-drop.log, board-repeat.log). Pin is sole detector.
- Gate on pin tip d4fd5e9f (reserved) running: gate-audit-d4fd5e9f.log. Slot on audit branch; DO NOT switch
  until it finishes. Then switch to simplify, gate -> gate-simplify-12231140.log.

- RESULT gate pin tip d4fd5e9f: matches baseline (workspace 1864/1864, 2 skipped; wasm 25/43/8; board 5311
  green + the two count_display x heap drift lines; fuzz libfuzzer). Logs gate-audit-d4fd5e9f.log, gate-logs-d4fd5e9f/.
- Slot switched to simplify 12231140 (clean). Gate running -> gate-simplify-12231140.log. Then report.

- RESULT gate simplify tip 12231140: matches baseline (same tallies, same two drift lines, fuzz libfuzzer).
  Logs gate-simplify-12231140.log, gate-logs-12231140/. Coordinator switched verification to audit-check;
  both full gates had already run (superset), no rerun needed.
- Builder's simplified-code mutant logs spot-checked: drop_touch fails only the pin.
- Slot on simplify/suanpan-read-high-part at 12231140, git status clean.
- DONE: verdicts Approve / Approve, non-blocking suggestions only. Report sent.

## Next
1. Read Accumulator::sign_biguint path; check readout does not mutate digits between the two readouts.
2. On audit branch: mutants touch(2) (repeat) at each site + drop_touch; run suanpan tests.
3. Gate on audit tip (background), then switch to simplify, gate on simplify tip.
