# reviewer-min-ticks-final: resumption record

Task: review fix/before-min-ticks-heap in /Users/oxide/src/rumors-slot-16 (demo 08573e15, fix 1a31f8d1
on main 5065caeb). Base verified: HEAP 1a31f8d1, clean at start. No commits. Cap: 4 remote runs.

## Established (by reading)
- Touch meter counts only suanpan Accumulator digit work (crates/suanpan/src/accumulator.rs:44);
  num-bigint arithmetic in SuspendedMinima::push/pop is invisible to every board time meter.
- Hypothesis H1 (blocking): a narrow record pushed/popped repeatedly over an outer neighbor with a WIDE
  own offset costs O(W) per push/pop on the branch (push computes top - outer; pop computes top - diff),
  O(1) on main (StoredContribution word moved in RangeMinima payload column; encoded once at store).
  Shapes: cross_prefix (A=J at prefix 0, comb records at prefix 1 offset ~0) and same_prefix
  (A offset 1-2^b, comb at same prefix offset ~0). Contradicts minima.rs:49-51 doc and O(n) contract.
  Arm-below path (finish_arming Less: suspend then retire) does a push+pop round trip: same mechanism.
- suspended.rs:75-76 comment false when outer.offset == 0: num-bigint (_, NoSign) arm returns self.clone().
- PackedU64Stack stores widths in unary: a full limb costs 128 bits, and pop of width>=62 takes the
  bit-at-a-time loop (packed_u64.rs:63-76).
- wide-arming rise: predicted = retained BitStack capacity of two W-bit diffs (value + unary widths,
  doubled to next pow2): s=1174 -> 32 KiB = 1.13 B/B; s=587 1.1; s=4696 1.1. Matches measured.
- ChunkedStack came from archived 7222d49b (re-anchor round, payloads + wide_values); not required by
  this design; fixer's own probe shows mixed off-board moves (step-2^32 up, step-2^64 down).
- Band test duplicates the existing seam_plunge measurement at the same k and checks a constant
  inequality (ceiling - k*r < floor). Recommend drop.
- Ghost: tests/meter/version_scaling/minimum_combs.rs:32 "one contribution".
- BigUint::new copies only on 64-bit digit targets (num-bigint 0.4.8 biguint.rs:569-581).

## Temporary edits in the worktree (MUST restore)
- Backups in backup/: meter.rs, min_ticks.rs, suspended.rs. Untracked probe:
  crates/before/src/testing/meter/reviewer_probe.rs. Restore: cp backups back, rm probe, git diff empty.

## Runs
1. run1-branch.log: probe on branch (circulation + wide-arming marks).
2. (planned) parent: reverse-apply git diff 08573e15 1a31f8d1 -- crates/before/src, hooks into
   contributions.rs store spill path + min_ticks mark; run probe. Then git restore + rm untracked.
3. (planned) repair diffs: band-test drop, minimum_combs ghost, ChunkedStack removal; build + focused tests.
