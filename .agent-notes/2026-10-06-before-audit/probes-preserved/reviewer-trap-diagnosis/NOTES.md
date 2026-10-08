# reviewer-trap-diagnosis: resumption record

Task: review branch audit/wasm32-trap-diagnosis (machinery), commit 4dcc0caa, base f062e744.
Worktree /Users/oxide/src/rumors-slot-09 (verified HEAD == 4dcc0caa, clean, signed good).
Brief: .agent-notes/2026-10-06-before-audit/lanes/l4-measures/round-1/machinery-wasm32-trap-diagnosis.md
Commit nothing; experiments reversible; `git diff` empty before finishing.

## Plan
1. Gate on clean tip (background): gate.log in this dir.
2. Q1: construct panic-under-memory-exhaustion -> expect panic: None (std formats
   FormatStringPayload String before hook; Option::expect goes via panic_display "{}").
3. Q2: harness memory config: Engine::default, no limiter, no pooling -> 4 GiB max.
4. Q3: grep protocol numbering users.
5. Q4: validation_index rules.
6. Mutant A rerun (builder: digit_index expect -> handle_alloc_error).

## State
- gate attempts 1,2: pset-run exit 125 "psrset: cannot assign processor N: Device busy" (environmental). Logs gate-attempt{1,2}-psetfail.log.
- Q1 CONFIRMED by construction (probe.patch, probe.log; reverted, diff empty): fill memory via try_reserve_exact,
  then None.expect(suanpan msg) -> panic: None; literal panic! -> Some; formatted panic! -> None;
  mode 5 verifies fill (1-byte reserve fails). => Outcome::Trapped rustdoc "None when aborted without panicking" too strong;
  panic_record module doc "nothing recorded identifies an abort that never panicked" false for expect-based panics. Direction: spurious pin failure only, never false pass.
- Q2: guest memory min 17 pages, no max, 32-bit; Engine::default + Store no limiter -> 65536 pages cap; control guaranteed.
- Q3: in-branch consistent. CROSS-BRANCH: fix/suanpan-reserve-digits-hint (5bd37ff3) adds SuanpanReserve = 9 -> collision (loud: E0081).
  fix/suanpan-limb-index-wrap (385e9acd) edits the same suanpan pin loop (1..=4) -> textual conflict.
- Mutant A rerun (mutantA.log): suanpan pin FAILS with panic: None; restored, diff empty.
- gate attempt 3 started (gate.log).
- gate attempt 3 also pset-run 125. Coordinator: use `~/bin/audit-reserved just gate` (serialized), never pset-run directly.
- probe2 (probe2.patch/probe2.log; reverted, status clean): recursion -> Trapped{StackOverflow, None};
  process::abort -> Trapped{Unreachable, None}. => triage bullet in validation_index ("no panic message -> ran out of memory
  before reaching the behavior under test") overclaims: ignores trap kind and non-memory aborts, and Q1 shows behavior may have been reached.
- gate attempt 4 via audit-reserved started: gate.log. Remaining: read gate result, write report.
- gate via audit-reserved (wrapper attempt 3 got cores): gate: FAILED after 326s: board fuzz; all other legs ok;
  workspace 1863/1863 (2 skipped); wasm fuzzfit 25/25, fuelscape 43/43, wasm32-pins 8/8; board 5311/0 + the two baseline
  count_display drift lines; fuzz = libfuzzer platform error. == baseline. Leg logs copied to gate-logs/.
- Final: worktree clean at 4dcc0caa. Report delivered (verdict: changes required, prose accuracy only).
## Round 2 (tip 8fe3e5b8 on main 0bdeb588)
- Resolution: checks.rs = main + branch additions; suanpan_landing identical to main; pin loop 1..=4 assert_panics. OK.
- probe3 (probe3.patch/log, reverted): full memory (65536 pages): Option::unwrap -> Some("called `Option::unwrap()` on a `None` value");
  Result::unwrap -> None. Landing cases 1-4 trap at 18 pages each.
- => new prose "every expect and unwrap message" has arguments is false for Option::unwrap (pessimistic overclaim). Changes required (one-phrase fix).
