<!-- CAVEAT LECTOR: the round-1 report of the L8 (adequacy) auditor, Claude Opus 5.5, condensed by the coordinator; results are the auditor's own claims unless marked. -->

# Lane L8, round 1: report

The explore branch is `explore/l8-adequacy` at `5e0edfcf`, and all its
commits are signed. The sibling directories hold the auditor's findings,
briefs, survivor classifications, coverage summary, and census baseline.

## Verdict

- No defect in `before` or `suanpan` production code.
- Two instrument defects.
- Four machinery briefs and one simplification brief.
- One question for the owner, plus two questions routed to other lanes.
- The lane is *not* at diminishing returns. The mutation campaign is only
  partly done, and it continues unattended on the box.

## Instrument defects

- **D1 (medium):** [`findings/count-display-heap.md`](findings/count-display-heap.md).
  - **Behavior:** the board's `count_display × heap` worst-case pin depends
    on the architecture.
  - **Cause:** `num-bigint` chooses its decimal conversion base by
    `FAST_DIV_WIDE`, which is true only on x86.
  - **Evidence:** a probe reproduces the box's readings exactly. On non-x86,
    the pinned family wins by about 0.1%. This explains the drift recorded in
    the baseline.
  - **Route:** a design choice, question 11 in `QUESTIONS.md`.
- **D2 (low to medium):**
  [`briefs/machinery-detached-nextest-timeouts.md`](briefs/machinery-detached-nextest-timeouts.md).
  - **Behavior:** the detached workspaces have no nextest config, so a hung
    test is flagged but never terminated. The wasm32 harness's engine also
    has no fuel or epoch limit, so a guest loop gone infinite hangs the gate.
  - **Evidence:** verified with a 200-second probe test, which was flagged
    slow and never killed.

## Briefs

- [`briefs/machinery-rank-decode-reader.md`](briefs/machinery-rank-decode-reader.md):
  `Rank::decode`'s rejection paths never run past its 64-byte read chunk.
- [`briefs/machinery-suanpan-normalize-bound.md`](briefs/machinery-suanpan-normalize-bound.md):
  `normalize`'s width-shrinking and top-carry-of-2 paths are reachable but
  never tested. A surviving mutant gives a wrong value on the brief's witness.
- [`briefs/machinery-trait-impl-coherence.md`](briefs/machinery-trait-impl-coherence.md):
  a law group for the public trait impls: `Hash` from canonical bytes, `Debug`
  formats, agreement of operator spellings, and sound `size_hint`.
- [`briefs/simplification-hole-subtracts.md`](briefs/simplification-hole-subtracts.md):
  delete `Sealed::hole_subtracts`, which has no callers.

## Questions routed to lanes

- **To the algebra and spans lanes:** the mutant `>` to `>=` in
  `overlay.rs:142` (`advance_set`) survives. It is equivalent only if every
  `CursorSet` folds tied crossings regardless of order.
- **To the coordinator's `Rank` brief:** the auditor verified that forcing
  the contiguous `BigUint` route at a gap of 2^32 bits on wasm32 stays
  correct. That corroborates
  [`../../../coordinator-briefs/simplify-rank-single-alignment.md`](../../../coordinator-briefs/simplify-rank-single-alignment.md).

## Observations

These are in [`findings/`](findings/):

- **wasm32 pins:** they catch 7 of 11 injected narrowings. Three pins' docs
  claim more than they check.
- **Fold log-factor pins:** they check only a floor. A quadratic-fold mutant
  passes all five; the board would likely catch it, which is inferred.
- **The surface check:** it caught 2 of 2 injections.
- **`usize` in public signatures:** only `size_hint` and `Count`'s
  conversions use it. The auditor proposes pinning that list in
  `surfacecheck`.
- **Generator census:** see [`census-baseline.txt`](census-baseline.txt).
- **For the suanpan lane:** cost-only survivors in `zero_ranges`.

## Campaign and coverage

- **Mutation results so far:**

  | Group | Caught | Missed | Unviable |
  |---|---:|---:|---:|
  | `count.rs` | 41 | 3 | 13 |
  | `suanpan` | 355 | 30 | 26 |
  | version (partial) | 181 | 7 | 67 |
  | rest (partial) | 7 | 21 | 14 |

  Survivors are classified per module under [`survivors/`](survivors/).
- **Branch coverage:** one run on the Mac, under the owner's exception.
  - 10,102 of 10,324 lines and 1,576 of 1,652 branch outcomes. The summary
    is under [`coverage/`](coverage/).
  - The Mac build directories the run created were deleted by the
    coordinator.
