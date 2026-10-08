# reviewer-rank-sum-pin: resumption record

Task: review branch audit/wasm32-rank-sum-pin (25c2c02d on f062e744), worktree
/Users/oxide/src/rumors-slot-10. Commit nothing; restore any experiment, check git diff empty.
Brief: .agent-notes/2026-10-06-before-audit/lanes/l4-measures/round-1/machinery-wasm32-rank-sum-pin.md

## Established
- HEAD = 25c2c02d, base f062e744 ancestor, tree clean (verified 2026-10-07).

## Next
- read pins.rs, checks.rs, rank.rs sum_iter, builder evidence
- mutant run (shift as usize as u64), case 5 pass / case 6 trap
- rebase check vs audit/wasm32-trap-diagnosis (4dcc0caa), fix/suanpan-limb-index-wrap
- reserved gate: unset CARGO_TARGET_DIR; export NEXTEST_TEST_THREADS=24; ~/bin/audit-reserved just gate

## Established (update 1)
- nextest 180 s terminate is NOT configured for detached wasm32-pins workspace
  (no crates/before/wasm32-pins/.config/; builder run1.log: rank_arithmetic PASS at 186.719s).
  => commit msg "nextest's 180-second budget" inaccurate (it's audit policy). prose finding.
- Small-first order lands both summands at offset 0 (first via add_shifted_limbs(0), second
  at exp-rank.exp=0 after shift) => landing-offset narrowing mutant unreachable by cases 5/6.
  Idea: case [one, deep, one]: after shift acc digits (Vec<i64>) already reach index 2^27,
  third lands at bit 2^32 without growth. Judge: sum.checked_sub(one+one)==Some(deep).
- merge-tree: branch merges clean onto audit/wasm32-trap-diagnosis (4dcc0caa) and
  fix/suanpan-limb-index-wrap (385e9acd). Those two CONFLICT with each other (checks.rs, pins.rs).
  After trap-diagnosis lands, the doc's memory hedge could use panic:None diagnosis.
- Doc numbers: 58399 pages*64KiB = 3.564 GiB; headroom 7137 pages = 446 MiB < 512 MiB (deep value 2^32 bits). Builder probe: c4 & c5 58399.

## Experiments (worktree edits; restore after!)
- M1: shift mutant in rank.rs (line after `let shift = gap.max...`), log m1-shift-mutant.log (bg task b91ku0n6e)
- E1 (planned): add case 7 + probe test (untracked harness/tests/zz_reviewer_probe.rs)
- E2 (planned): E1 + landing mutant `(exp - rank.exp) as usize as u64`

## Update 2
- M1 DONE (m1-shift-mutant.log): rank_sum FAIL "RankArithmetic failed for (6, 0)" left Trapped(UnreachableCodeReached)
  (so case 5 passed); rank_arithmetic PASS 146.3s under mutant. rank.rs reverted (diff empty).
- E1 RUNNING (bg bwu646qjz, log e1-case7-probe.log): worktree has checks.rs case-7 edit + untracked
  harness/tests/zz_reviewer_probe.rs. MUST restore both (reverse case7 edit; rm probe) before gate.
- E2 STARTED: rank.rs landing mutant applied (restore: '(exp - rank.exp) as usize as u64' -> 'exp - rank.exp'), log e2-landing-mutant.log
- E2 DONE: rank_sum PASS under landing mutant (57.7s); probe case7 Ok(8)=WrongValue pages 42014. Tree RESTORED, git diff empty, HEAD 25c2c02d.
- GATE started: log gate.log
- Coordinator: verification of record is now ~/bin/audit-check (not just gate). Killed my queued (never-started) gate wrapper pid 5400 on box. audit-check started -> check.log; per-leg logs on box ~/src/rumors-slot-10/target/audit-check-logs/
- audit-check DONE (check.log): surface/docs/lints/wasm/tests ok; board FAILED with exactly 2 baseline drift lines, 5311 green/0 red. wasm32-pins 9/9: rank_sum 46.453s, rank_arithmetic 80.284s. Tree clean at 25c2c02d. NEXT: hand back report.

## Round 2 (tip 0c22bd66)
- Running probe (untracked harness/tests/zz_reviewer_probe.rs, REMOVE after) -> r2-probe.log
- r2 probe on tip: case4 58399, case5 58399, case6 50207, case7 50207 (20.6s, load ~200). Probe removed, tree CLEAN at 0c22bd66. Verdict: APPROVE. Handed back.
