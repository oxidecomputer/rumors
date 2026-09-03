<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) as the p1-harness-tests lane agent, recording a stop for Finch to rule on; not authored or endorsed by Finch. -->

# Stop: tests-observation-28 (ruling T13), the overlap shadow's meta-test

T13 lands the overlap twin of `shadow_predicts_live_state` with the
shadow's fork point kept at `Open`. Written as ruled, the meta-test
fails on the lane's base by the imprecision the ruling keeps, not by a
defect, and the lane brief marks that outcome a stop.

## The shrunk case

Four peers, converged preamble, then `Open { slot: 0, a: 2, b: 3 }` at
event 20, `Insert { peer: 3 }` at events 21 and 32 before that session
is polled, and `Close { slot: 0 }` at event 33. The live session forks at
the `Close`, when peer 3 holds 21 and 32, so peer 2 learns both; the
shadow's `Open`-time snapshot never carries them, so it predicts peer 2
observed neither (`peer 2 observation set disagrees with the overlap
shadow`). Any insert at either endpoint between an `Open` and the
session's first poll produces the same divergence, so the failure lands
within the first few cases (two successes before it here).

## Contents

- `tests-observation-28-meta-test.patch`: `Knowledge` made public,
  `arb_overlap_schedule_with_shadow`, `execute_overlap` returning the
  pre-quiescence fleet and `resolved_versions`, and the meta-test in
  `tests/shadow_validity.rs`; applies to the lane tip with `git apply`.
- `tests-observation-28-seed.diff`: the seed proptest persisted for the
  failing case (not committed: no landed test owns it).
- `tests-observation-28-run.log`: the failing run, with the full shrunk
  schedule and shadow.

## The choice

Either the shadow's fork moves to the session's real fork point (the
entry's original part 2, which T13 declined), or `open()` polls the
session through its preamble so the model's `Open`-time fork is exact
(at the cost of no longer sampling interleavings during the preamble).
Under either, the patched meta-test lands unchanged. Keeping the guard
without a meta-test leaves the overlap shadow unverified.
