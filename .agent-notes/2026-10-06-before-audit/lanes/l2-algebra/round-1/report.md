<!-- CAVEAT LECTOR: the round-1 report of the L2 (version algebra) auditor, Claude Opus 5.5, transcribed by the coordinator. The coordinator verified the explore branch's commits and signatures and read every brief; the auditor's run results are its own claims. -->

# Lane L2, round 1: report

The explore branch is `explore/l2-algebra` at `d78c6129`, with production code
unchanged from `58285ca5`. The sibling files in this directory are the
auditor's deliverables, copied from its scratch directory.

## Verdict

- No confirmed defects in lane L2 (version algebra), and no questions about
  intended semantics.
- The auditor judges the lane to be at diminishing returns.
- The probe checks the algebra against an independent leaf-list step-function
  model, with its own canonical byte encoders for versions and parties.
- Its generators reach what the committed ones do not:
  - random topology to depth 300, where the committed generators stop at
    depth 4
  - heights on the 63-bit code boundaries and past the machine word
  - correlated pairs built to force wide collapse cascades in the writer
- It ran more than 54,000 property cases on ox-east-1, all passing.
- Of 20 injected mutants, the probe killed all 20. The committed suite also
  killed all 20, so the probe adds no first-order detection; the auditor
  proposes no new semantic instrument.
- All five candidate leads hold as stated:
  - the lattice is distributive and canonical
  - the encoding-size bound holds
  - all comparison paths agree
  - folds equal sequential application
  - projection is exactly restriction
- All four closed fixes it re-attacked hold.

## Briefs

- [`machinery-deep-surfaces.md`](machinery-deep-surfaces.md): a depth-100k
  stress test covering what the committed deep tests never drive at depth:
  - the shape walks
  - the concurrent-pair hull
  - `Sum` and `FromIterator`
  - join and meet on a concurrent pair

  The test is calibrated against a one-frame-per-level recursion, which
  aborts with a stack overflow on a 2 MiB thread.
- [`simplification-lattice-entry-delegation.md`](simplification-lattice-entry-delegation.md):
  `join` and `meet` delegate to the operators instead of repeating their
  short-circuit ladders.
- [`simplification-lattice-one-sweep.md`](simplification-lattice-one-sweep.md):
  one const-generic sweep replaces the two copies in `hull_bits` and
  `Extreme::emit`.

## Observations

These are in [`observations.md`](observations.md):

- `Sum` and `FromIterator` state their cost in terms of a receiver they lack.
- The lattice module doc describes picking closures that do not exist.
- `Version`'s order docs conflate `<` with `<=`.
- The "more efficient" advice on `join_all` holds only in the worst case.
- The board's depth disposition overstates what its cited test drives.
- The concurrent-pair hull walks its operands twice, a constant factor; the
  auditor recommends no action.

## Coverage and resumption

See [`coverage.md`](coverage.md) and [`NOTES.md`](NOTES.md).

The auditor names these blind spots:

- 32-bit is covered only by the committed pins.
- Random cases stop near 900 splits and depth 300.
- Builds were debug-only.
- The mutants were hand-chosen, at 20 sites.

What it would do next:

- run the cargo-mutants survivors from the adequacy lane for its files against
  the probe
- second-order mutants on the writer's split-stream paths
- larger random sizes with a faster model
- a release-mode run
