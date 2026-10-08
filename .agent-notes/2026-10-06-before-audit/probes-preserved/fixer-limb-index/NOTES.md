# fixer-limb-index resumption notes

Task: fix D1 (limb index wrap in suanpan `Digits::apply_limbs`) on branch
`fix/suanpan-limb-index-wrap`, worktree /Users/oxide/src/rumors-slot-04.
Test commit d464ea55 (verified signed, HEAD at start, tree clean).

## Established
- Briefs read (common.md, fixer.md, README, baseline.md, D1 record).
- Position-arithmetic survey (suanpan/src, non-test): only
  `Digits::apply_limbs` (digits/add.rs:89) takes a position from an
  iterator counter. Others: deposit_magnitude (u128 position, <=4 steps),
  add_digits (slice enumerate, offset < isize::MAX, exact), apply_shifted_word
  (u128 from u64 digit_shift), add_at carry (checked_add), shift_left ->
  add_shifted -> add_digits/deposit_value; sum impls iterate operands, no index.
  Accumulator::apply_limbs (operand.rs) peeks two limbs then chains into
  Digits::apply_limbs, so both add_/sub_shifted_limbs share it.

## Next
- Read auditor's case 6 classifier; apply fix; run regression + discovery.

## Progress (fix applied, uncommitted)
- Fix in digits/add.rs: `(0_u128..).zip(limbs)` + comment; saved as
  scratch fix.patch. Scratch classifier probe (panic hook -> PANIC_CLASS
  export; harness run_classified; tests/zz_scratch.rs; guest cases 5,6,9-13)
  saved as probe.patch -- MUST be reverted before commit (git apply -R
  probe.patch; rm harness/tests/zz_scratch.rs; git diff shows only add.rs).
- Background: run-fixed.log (fixed tree: committed pin + ZZ_CASES=13,4,6,9,10,11,12,5).
- Next: base run (fix swapped out) for cases 4,6 + committed pin; then
  revert probe, commit, gate in background.
- DONE run-fixed.log (exit 0): pin PASS 26.992s; classes: 13->0 (alloc abort),
  4,6,9,10,12,5 -> Trapped class 1 (landing panic); 11 -> Passed.
- NOW: fix swapped OUT (git apply -R fix.patch) for base run, probe has extra
  case 14 (other panic). run-base.log in progress. AFTER: git apply fix.patch,
  revert probe (git apply -R probe.patch after removing case 14 line; rm zz_scratch.rs).
- DONE run-base.log: pin FAIL 24.546s (case 4 Failed(WrongValue)); 14->class 2,
  4->Failed(WrongValue) class 0, 6->Failed(WrongBytes) class 0.
- Probe reverted, fix committed: 385e9acd (signed; amended msg of 0eec83b3, same tree 9fb8960a). Tree clean.
- NOW: gate.log (just gate on box, background). Compare to baseline.md:
  fuzz leg fails (libfuzzer illumos), board worst-cases-pin drift
  count_display x heap (pinned hugeleaf; live pure-comb default, memo-comb acceptance).
- DONE gate (gate.log, exit 1 = baseline): FAILED board fuzz only; board 5311/0 +
  same two drift lines; fuzz libfuzzer #error; workspace 1863/1863 (2 skipped);
  wasm: fuzzfit 25/25, fuelscape 43/43, wasm32-pins 8/8 (pin 30.105s).
- Report handed back. Remaining: review rounds if dispatched.
