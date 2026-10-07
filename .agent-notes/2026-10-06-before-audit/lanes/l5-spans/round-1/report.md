<!-- CAVEAT LECTOR: the round-1 report of the L5 (spans) auditor, Claude Opus 5.5, condensed by the coordinator, who verified the explore branch's signatures; results are the auditor's own claims. -->

# Lane L5, round 1: report

The explore branch is `explore/l5-spans` at `2874e0c3`, and all its commits
are signed. The sibling files in this directory are the auditor's
deliverables.

## Verdict

- No behavioral defects. Three low-severity documentation defects. No
  questions about intended semantics. The lane is at diminishing returns.
- The oracle represents versions as height vectors over one shared leaf
  partition, ordered pointwise, with no code shared with production. It
  checked:
  - every verdict, operator, and coverage result, at up to 4,000 cases per
    property
  - an exhaustive four-cell cube, with 921,456 coverage verdicts all agreeing
- Calibration: the harness caught 17 of 19 injected mutants. The two
  survivors are equivalent, and the committed suite also caught all 17.

## Findings

- **Documentation defects:** [`doc-defects.md`](doc-defects.md). All three go
  to the docs branch.
  1. The law doc of `coverage_bounds_membership` claims `Empty` coverage
     cannot be complete. It can, and `refine_partial` makes it exact.
  2. `toward`'s rustdoc names parameters `e` and `p`, which don't exist.
  3. `Polarity`'s docs promise holes that conjunction can prune away.
- **Brief:** [`brief-refine-partial.md`](brief-refine-partial.md). Deciding
  the crossed clamp by `floor <= ceiling` would cut a neutral `Partial` call
  by about 37%, but `refine_partial` would then depend on guarantees the walk
  establishes. Whether to accept that coupling is question 13.
- **Tie order:** [`tie-order.md`](tie-order.md). The `>` to `>=` survivor in
  `overlay.rs:142` is equivalent for `place.rs` and `filter.rs`; the argument
  and an empirical check are there. The docs at `place.rs:407-409` and
  `filter.rs:414-417` and `:711-715` overstate why probe-first stepping
  matters: it makes accumulator work reproducible and has nothing to do with
  correctness. This goes to the docs branch.
  `projection.rs:420` is unchecked; that is lane L2's.

## Observations and coverage

- Observations: [`observations.md`](observations.md). They include no
  `usize` defect in the lane, `OwnSpan::place` re-transcribing the nine-way
  table, the re-attacked closed fixes, and a linear carry-boundary cost
  family.
- Coverage: [`coverage-record.md`](coverage-record.md).
- Inventory: [`inventory.md`](inventory.md).
- Resumption notes: [`NOTES.md`](NOTES.md).
