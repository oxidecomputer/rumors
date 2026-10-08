# reviewer-fork-hint resumption record

Task: review `fix/before-fork-size-hint` (round 1) in /Users/oxide/src/rumors-slot-12.
Tip a6215b4f (fix), 96d69408 (test), base 99922ef5. Verified: HEAD = a6215b4f,
clean, both commits signed (G). Reviewer commits nothing; experiments reversible.

## Established (by reading)
- Share sequence unchanged: current_forks_again/current_share/index+second update
  untouched; only exhaustion (remaining == 0) and hint source changed.
- Item 3 verified: From<Party> for [Party; N] uses into_shares (Shares, usize);
  From<Clock> delegates to it. Neither uses Plan.
- No ghost refs (Remaining/Distant/has_saturated/classif) in crates/before.
- wasm32 pin forks_accept_the_first_count_past_usize: k=2^32, asserts
  (MAX,None) then after one real next (MAX,Some(MAX)) -> covers the u32::MAX+1 -> u32::MAX
  transition through skip_one/advance/try_from on wasm32.
- KEY CONCERN: post-fix, position_at_remaining writes self.remaining directly, so the
  regression test checks only usize::try_from conversion; never runs new/advance/is_empty
  on a wide plan. Fixer's scratch drains (uncommitted) use the same helper.
  Doctrine: "Any quantity computable two ways gets a committed test comparing them"
  (remaining is derivable from index/second/extra).

## Plan
1. Mutant experiment: env FORK_MUTANT switch in forks.rs (reversible; original saved
   as forks.rs.orig here), graft L1 tests/audit_l1 (untracked) into slot; run
   `-p before -E 'test(/fork/)'` per mutant. Mutants:
   1 advance off-by-one at MAX+1; 2 hint from count; 3 Err->(0,None);
   4 no decrement while remaining.bits()>w+1 (is_empty also index); 5 new: +1 when k.bits()>w+1;
   6 off-by-one at remaining==MAX.
2. Proposed stronger test (skip helper moving index+remaining by the same amount from
   construction, drain tails) -- run against mutants.
3. Restore, git diff empty, remove graft; base re-run of regression test at 96d69408? (fixer log
   exists; rerun myself at 99922ef5-tree+test i.e. detached 96d69408).
4. Landing check at tip in background.

## Background jobs
(none yet)

## Results (update 1)
- regression-at-96d69408.log: exit 100, 16 mismatches, claimed reason. Verified.
- Mutant runs (mutants-run-0-1.log, mutants-run-2-6.log; FORK_MUTANT env switch; 38 fork tests):
  M0 all pass (incl. L1 wide + reviewer test). M1 caught by adjacent_wide (committed).
  M2, M3 caught by committed. M4, M5, M6 SURVIVE every committed test; reviewer test
  (tests.rs.with-reviewer-test) kills all; L1 wide kills M6 only.
- Worktree restored (git status empty) at a6215b4f; graft removed.
- Background: landing check -> audit-check.log (local bg task bl3w4n2fj). Do not edit slot.
## Next: prose review, re-pin stability (fixer before/after-forkrows.txt), report.

## Results (update 2)
- Re-pin: default party_forks heap: before ascend 10.1 > copy-hole 9.4; after copy-hole 10.8 > ascend 10.6
  (margin 1.019). Acceptance scale: ascend 10.6 > copy-hole 10.0 (unchanged pin). Deterministic, but
  any ~2% fixed-heap shift flips it; cross-machine heap drift precedent (count_display open item).
- worst.rs comment "a fixed cost" inaccurate: board's wide_fork_count makes count width = party bytes,
  so every party_forks/clock_forks row rose ~+0.5 B/B; copy-hole default +1.4 (tiny-input overhead).
- Fix commit msg "every other fork row ... unchanged" false: clock_forks heap +0.5 B/B on every family shown.
- Rustdoc "While more remain, it reports (usize::MAX, None)" ambiguous (more than usize::MAX).
- wasm32: existing pin covers 2^32 -> 2^32-1 via real step; no new pin needed.
- Landing check running: bg task bl3w4n2fj, monitor b8a7y3az8 waits for "exit N" in audit-check.log.
