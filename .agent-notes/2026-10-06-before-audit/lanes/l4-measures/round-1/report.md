<!-- CAVEAT LECTOR: the round-1 report of the L4 (measures) auditor, Claude Opus 5.5, condensed by the coordinator, who read every brief; results are the auditor's own claims. -->

# Lane L4, round 1: report

The explore branch is `explore/l4-measures` at `2c82c7ba`, and all its
commits are signed. The sibling files in this directory are the auditor's
deliverables.

## Verdict

- No contract defect in `Rank`, `Ranked`, `Count`, or the version measures.
  The auditor judges the lane to be at diminishing returns.
- The independent oracles passed:
  - an integrator harness, at 70,000 cases plus 600 deep cases
  - a `Rank` value harness, at 40,000 cases
  - an exhaustive `Count` boundary sweep
- Both harnesses were calibrated by injected defects before their silence
  counted as evidence. The committed suite also caught every mutant the
  auditor built (observation O3), so no new semantic instrument is proposed.
- Every closed fix the lane re-attacked still holds:
  - `Sum` cost stays flat across adversarial summand orders.
  - The cost of `Ranked::cmp` ties grows ever more slowly per doubling.
  - Fold cost stays flat.
  - `Rank` text honors formatter flags.

## Findings and briefs

- **Prose defect (low):**
  [`simplification-rank-normalization-prose.md`](simplification-rank-normalization-prose.md).
  The maintainer docs say `Rank`'s numerator is odd unless the value is zero.
  The true rule, `exp == 0 || num.bit(0)`, keeps even integers with an even
  numerator. This goes to the docs branch.
- **Machinery:**
  [`machinery-wasm32-trap-diagnosis.md`](machinery-wasm32-trap-diagnosis.md).
  `wasm32-pins` reports a panic and an allocation abort identically, so a pin
  that expects a documented panic also passes for an implementation that
  aborts on memory. The brief records the panic message through a panic hook
  and adds a validation-index entry for `wasm32-pins`.
- **Machinery:**
  [`machinery-wasm32-rank-sum-pin.md`](machinery-wasm32-rank-sum-pin.md).
  No 32-bit check calls `Rank`'s `Sum`, which has its own code path. The
  brief adds two summands that force a shift past 2^32, calibrated against a
  `usize`-narrowing mutant.
- **Question:** on wasm32, `[&deep, &half].sum()` aborts on allocation
  failure while `[&half, &deep].sum()` and `&deep + &half` succeed. The cause
  is `Vec` doubling of a 1 GiB digit buffer. This is question 7 in
  `QUESTIONS.md`.
- **Cross-lane lead for L6:**
  [`cross-lane-l6-rank-decode-wasm32.md`](cross-lane-l6-rank-decode-wasm32.md).
  The auditor infers, without demonstrating it, that past about 1.13 GiB of
  input on wasm32, `Rank::decode` may panic with "capacity overflow" instead
  of returning an error.

## Observations

These are in [`observations.md`](observations.md):

- **O1:** a stale exponent bound in a comment.
- **O2:** duplicated alignment dispatch in `+` and `checked_sub`.
- **O4:** a cheap generalization of the tie tests.
- **O5:** the touch meter is blind to `BigUint` work.
- **O7:** `wasm32-pins` has no validation-index entry.

## Coverage

See [`coverage.md`](coverage.md) and [`NOTES.md`](NOTES.md).
