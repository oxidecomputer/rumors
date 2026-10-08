# fixer-zero-shift: resumption record

Worktree /Users/oxide/src/rumors-slot-15, branch fix/suanpan-zero-shift.
Scratch S = this directory.

## Established

- Brief base verified: HEAD was 1bf133ef (signed). Rebased onto local main d6d22b4c
  with `git rebase main`: no conflicts. Test commit is now 86182875 (signed).
  Function-level integration checked: main's limb-index fix adds case 4 inside
  `suanpan_landing` and a u128 counter in `apply_limbs`; the branch appends a
  separate `suanpan_zero_shift` check and harness test. No semantic overlap.
- Mechanism: `shift_left` (operators.rs) takes `self` and re-deposits through
  `add_shifted` onto a fresh receiver, so the single fix point is
  `Accumulator::apply_accumulator`'s digit branch (operand.rs).
- Design chosen: read-only zero scan `Digits::value_is_zero` (sign.rs, same
  |partial| >= 3 rule), run only when the deposit would extend the receiver's
  retained buffer (`digit_shift + operand width > digits.len()`). Unconditional
  scan rejected: before folds borrowed accumulators in many hot loops
  (`+= &`), so it would move before's touch pins broadly.
- Predicted pin moves: metered.rs held_width `<<= 32` 2d -> 2d+1, `<<= 32_000`
  128 -> 129; accumulator_operand_rows add/sub_shifted at 32000/64000 4 -> 6.

## Test loosening plan (test commit, via fixup + autosquash)

- shifted zero: stored <= unshifted stored, retained <= unshifted retained + 2
  (for every shift; drop the `== 1` pin).
- receivers: retained <= before + 2; stored <= max(stored_before, retained_before) + 2.
- add a Reset receiver (wide value then reset: buffer retained, value small)
  so the in-buffer path is exercised with slack.

## Progress

- Branch now: test ccd34e4a (loosened assertions + Reset receiver, reworded via
  amend!), fix cfbf396f. Both signed? (check with git log --show-signature).
- Base failures verbatim: base-suanpan.log (orig form, left 1048578 right 1),
  test-commit-suanpan.log (loosened: "1048578 stored digits, up from 2"),
  base-wasm.log (Trapped(UnreachableCodeReached) vs Passed).
- Fix: suanpan 65/65 + zero property at PROPTEST_CASES=20000 pass (fix-suanpan.log).
- Pins moved (suanpan metered.rs), parent -> fix: held_width `<<= 32` 2d -> 2d+1
  (64: 128->129; 128: 256->257), `<<= 32_000` 128->129, add/sub_shifted of
  [1,1] at 32000/64000: 4->6. Shift-0 rows unchanged (inside buffer).
- Explore suite copied UNCOMMITTED: crates/suanpan/src/accumulator/tests/explore_l7.rs
  + `mod explore_l7;` in tests.rs. Remove before final check. Run log: explore-run.log.

- Explore run (explore-run.log): probe shows stored 1 for redundant zero under
  <<=, <<, add_shifted; pool model 20000 ok; touch property 5000 ok. Explore
  files removed; tree clean.
- Entry points: entry-points.log (native: borrowed/owned +=, -=, +, -, Sum<&>
  all stored 1 retained 0) and entry-points-wasm.log (6 combos Passed). Scratch
  reverted, git diff clean.
- before suite at fix: 693 passed (fix-before.log).
- Board render diff parent ccd34e4a vs fix (board-parent.log, board-fix.log,
  board.diff): 0 verdict changes, 0 scan moves, 45 touch moves (no exponent
  rises; constants +0.1/B in query_contains dominated-undercut, jump-comb,
  query_coverage cliff), 4 heap falls (version_tick/version_ticks/clock_tick
  mirror-wide heap e 0.02->0.01; ranked_cmp ascend-plateau 1.2->1.1/B).
- Tip after amend (verb-first helper doc): 0d343f33, test ccd34e4a. Both signed.

- Landing check at 0d343f33 (landing.log, landing-logs/): docs, surface,
  lints, wasm ok (pins 9/9, fuzzfit 25/25, fuelscape 43/43); board 5311/0 +
  the two count_display x heap drift lines only; tests: before+suanpan 758/758,
  rumors snapshot 141/142 with bounded_corpus_manifest_snapshot TIMEOUT at
  180.5s under load 380-515; rerun alone passed in 84.4s (timeout-rerun.log).

## Next

Report delivered; nothing pending.

## Round 2 (coordinator message: reviewer blocks on test adequacy + prose)

- Rebuilt via amend! + autosquash onto main 7e169418 (notes-only drift):
  test 87f32e37, fix 0f5316fd, both signed.
- Applied reviewer repair-test.patch (NearLimit receiver, accumulation
  property); ZERO_RETAINED_GROWTH 2 -> 1 with carry derivation.
- Base (87f32e37): r2-test-commit.log, both properties fail.
- Tip: r2-tip.log 66/66; both properties 20000 cases pass.
- Slack mutants (slack.py, reverted, diff clean): r2-slack2.log (both
  properties fail), r2-slack1.log (only accumulation property fails).
- Landing check running: r2-landing.log.
- Landing check at 0f5316fd (r2-landing.log, r2-landing-logs/): matches
  baseline; tests 759/759 + rumors 142/142, docs 193/3, surface ok, wasm
  9/25/43, board 5311/0 + the two drift lines. Round 2 report delivered.

## Round 3 (coordinator: two prose repairs)

- "With a valid count, shifting zero never panics." at ShlAssign and Shl
  (fix commit); value-bound sentence on ZERO_RETAINED_GROWTH (test commit,
  where the file belongs). Autosquash rebased onto main d20b4172, which
  gained board worst-case code (ddfabe4c) and the owner's working-width
  ruling (leave it). New SHAs: test a1526100, fix fc4743aa, both signed.
- fmt --check, just clippy, just clippy-default, just docs-internal: all 0
  (r3-*.log). Landing check left to the coordinator. Report delivered.
