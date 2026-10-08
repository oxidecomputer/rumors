# fixer-fork-hint resumption record

Task: fix D1 (fork plan size hint depends on usize width) on branch
`fix/before-fork-size-hint` in /Users/oxide/src/rumors-slot-12.
Test commit 1002799a over base 99922ef5 (verified: clean, signed G).

## Established
- Plan / Remaining in crates/before/src/party/forks.rs; ClockForks delegates
  size_hint to PartyForks -> Plan (shared path). `From<Party> for [Party; N]`
  uses `into_shares` (Shares, usize count, default size_hint), NOT Plan; it
  does not share the hint mechanism. sync_all also uses into_shares.
- Board ceilings are shared ratios (ceilings.rs); fork rows: party_forks,
  party_forks_full, clock_forks, clock_forks_full; party_forks count = 2^(8*input_bytes).

## Background jobs
- before-board.log: `just amp-board-acceptance` at 1002799a (no remote PID recorded;
  wrapper runs in foreground of a local bg task). DO NOT sync worktree until done.

## Next
- Implement exact BigUint remainder in Plan; reimplement position_at_remaining.
- Update rustdoc in party/forks.rs and clock/forks.rs; existing tests that pin
  (usize::MAX, None) for 2^128 stay valid; distant_size_hint_stays_sound_near_exhaustion
  must change (owner-approved).
- After board finishes: run regression test, then after-board, then wide.rs from scratch copy, then gate.

## Progress (update 1)
- Fix applied locally (uncommitted) in slot-12: Remaining deleted; Plan.remaining: BigUint;
  is_empty = remaining == 0; size_hint = try_from(remaining) or (MAX, None); extra = k.0.clone().
  Rustdoc restated in party/forks.rs and clock/forks.rs.
- tests.rs: dropped `distant_size_hint_stays_sound_near_exhaustion` (owner-superseded; subsumed by
  regression test) and `first_distant_count_handles_word_width_depth` (named deleted classification;
  r = MAX+1 case covers its hint). Helper now: precondition extra==0 only; sets index, second, remaining.
- Scratch family: $S/scratch_unit_tests.rs (append to tests.rs uncommitted, run, then remove).
- Explore probe at 1a5f5f8b sets only `index` -> not portable to new representation (expected to fail);
  the helper is its port.
- Array split `From<Party> for [Party; N]` uses into_shares, not Plan: unaffected (no size_hint override).
- Commit message draft: $S/commit-msg.txt
- COORDINATOR RULE: every remote command begins `unset CARGO_TARGET_DIR; export NEXTEST_TEST_THREADS=24;`
- COORDINATOR RULE: gate = `unset CARGO_TARGET_DIR; export NEXTEST_TEST_THREADS=24; pset-run -n 24 -- just gate` (I also set AMP_BOARD_SHARDS=24). before-board.log done: 5311 green / 0 red at 1002799a.
## Progress (update 2)
- regression-before.log: test fails on 1002799a tree (exit 100, 16 mismatches) - verified myself.
- focused.log: with fix + scratch, 24/24 pass (regression, scratch family x3, forks_count, audit_l1 wide x2 strengthened, L1_CASES=2048).
- Scratch removed; tree == fix.patch. tests-with-scratch.rs saved in $S.
- after-board.log running (bg). Then: diff fork rows, commit with commit-msg.txt, pset-run gate.
## Progress (update 3)
- Fix committed 7ba8d622 (signed G). after-board: 5311 green / 0 red. Fork rows: scan unchanged
  everywhere; heap/B +0.5..+1.7 on single-step party_forks/clock_forks rows (extra count-sized BigUint),
  per-op max heap/B unchanged (14.7 / 12.7); exps <= 1.00 (clock_forks benign 0.59->0.64). full/array rows identical.
- gate.log running: pset-run -n 24 just gate at 7ba8d622. Compare against baseline.md (fuzz fails libfuzzer; board worst-cases-pin count_display x heap 2 drift lines only).
- COORDINATOR RULE (supersedes pset-run): gate via `~/bin/audit-reserved just gate` (never pset-run directly). Direct pset-run attempt failed exit 125 (gate-psetfail-direct.log); gate.log now = audit-reserved run at 7ba8d622.
- gate queued in audit-reserved (my waiter pid 19490). Observed orphan pset-run pid 11125 (ppid 1, -n 32 just gate) at ~16:50 box time: flag to coordinator.
## STOPPED (final)
- Gate at 7ba8d622 (audit-reserved, 32 cores): ok audit/internal-docs/surface/docsrs/doctest/workspace(1862 run, 2 skipped)/wasm(25,43,8);
  fuzz FAILED = baseline (FuzzerPlatform.h:72); board FAILED: acceptance 5311/0 but worst-cases-pin has a NEW line:
  party_forks x heap default scale: pinned ascend-cliff,ascend-plateau, live copy-hole (9.4->10.8 /B vs 10.1->10.6 /B).
- Stop condition (ranking flip from regression). Branch reset --keep to 1002799a; fix commit 7ba8d622 kept as object + $S/fix-commit/*.patch.
- Report sent to coordinator with proposal: owner rules on re-pinning worst.rs default party_forks heap -> copy-hole.
## Round 2 (owner accepted option 1)
- Test commit rebuilt: 96d69408 (doc restated; fails at base, regression-base-96d69408.log exit 100, 16 mismatches).
- Fix commit: a6215b4f = 7ba8d622 + worst.rs re-pin (default party_forks heap -> copy-hole, comment) + message movement.
- Landing check: audit-check.log (audit-reserved ~/bin/audit-check) at a6215b4f; expect board 5311/0, only 2 count_display drift lines; tests 757+1-2.
- audit-check at a6215b4f: surface/docs/lints/tests(756 passed,1 skipped; snapshots 142)/wasm ok; board FAILED only on the 2 baseline count_display lines; 5311/0. DONE, reported.
## Round 3 (reviewer B1, P1-P3, item 5)
- Test commit 80d4072b (skip_shares helper, real steps; base fail verified regression-base-80d4072b.log exit 100).
- Fix commit 0b05eeb2 (helper lowers remaining; remaining_from_position + stored_remainder_matches_the_plan_position;
  prose P1/P2/P3; skip_one debug_assert removed). Mutants: mutants.log (M0 pass, M1) + mutants-1-6.log: all killed. forks.rs restored, diff empty.
- Landing check: audit-check3.log at 0b05eeb2. Expect tests 756+1 = 757 (one new test), board 2 count_display lines.
- audit-check3: board only 2 count_display lines; see summaries. DONE round 3.
