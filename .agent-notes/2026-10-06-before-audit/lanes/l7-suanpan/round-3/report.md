<!-- CAVEAT LECTOR: the round-3 report of the L7 (suanpan) auditor, Claude Opus 5.5, filed by the coordinator because the harness refuses report files from subagents. The coordinator condensed it and has not yet corroborated its measurements; they are the auditor's claims. -->

# Lane L7, round 3: report

The explore branch is `explore/l7-suanpan` at `5d5e3347`. The sibling files
in this directory are the auditor's briefs and records. Logs are in the
coordinator's session scratchpad under `auditor-l7/r3/`.

## Verdict

- No new defect. U1-a, carried over from round 2, is the only open one, and
  its repair is verified.
- Both F1 items are briefed with measured numbers, in the order the owner
  ruled: a committed counter first, then the W fix that flattens it.
- All four coordinator leads are evaluated. The auditor judges the lane at
  diminishing returns.

## Defect

[`U1a-stability-width-test-brief.md`](U1a-stability-width-test-brief.md)
holds the defect record, the test brief, and a verified fix note.
- **Behavior.** On 32-bit targets, `cmp_zero_stable_under` returns early for
  widths of `32 · 2^32` bits or more and skips the compaction its doc
  promises.
- **Evidence.** It still fails after merging `main`, at `5358b7f8`:
  `left: Failed(WrongLength), right: Passed`.
- **Repair.** Keeping the threshold in `u64` makes the wasm32 case pass at
  `5d5e3347`. The landing pins still pass, and the native suanpan suite
  passes 72/72.

## Item 1: a committed fuel ladder

[`F1c-machinery-zero-range-fuel-ladder.md`](F1c-machinery-zero-range-fuel-ladder.md)
is a builder brief.
- **Where it lives.** It extends the fuzz-fit harness, for four reasons:
  - The board's counters and the touch meter never see map work.
  - The board's slope limit of 1.15 would pass F1's fitted exponent of
    about 1.03.
  - A self-counted meter could be flattened by under-counting.
  - The fuelscape enforces nothing.
- **What it measures.** Six host-built families (F1, ±S, OS, each with a
  control that records no ranges), measured through decode, `<=`, and
  `min_ticks` at r = 64 to 4,096.
- **What it commits.**
  - exact base readings for 72 cells;
  - a ±2% band on each, with a ceiling and an improvement tripwire;
  - a liveness floor of fuel at least the input bytes;
  - value checks against the native build;
  - a flatness check on the controls.
- **Measured at `b9f688a3`.**
  - Every sparse row grows except F1's `min_ticks`: F1 decode +12% and `<=`
    +11%; ±S +46% to +60%; OS +17% to +56%. Every control is flat.
  - A known-bad variant, K1, does the map lookup twice: the sparse cells rise
    6.6% to 29.9%, and the controls are unchanged.
  - Under W, every sparse row is flat. The controls are 7.2% to 20.5% dearer,
    except F1's `min_ticks`, which is 6.5% cheaper.
- **Three traps the brief designs out.**
  - `ff_version_min_ticks` renders the count in decimal, so even the controls
    grew 45% to 94% through it.
  - `Version::new() <= v` can be answered in constant time, so pinning its
    linear walk would lock in removable work.
  - A comparison operand that keeps F1's opening never puts it in the running
    difference.

## Item 2: the W fix

[`F1d-fix-design-W.md`](F1d-fix-design-W.md) is a fixer brief, stacked on
F1c.
- **Design.** The written-position bitset, with each `ZeroRanges` call site
  mapped to its replacement.
- **Tuning.** It targets the nine control rows, with the owner's spare-bits
  idea among the candidates. The auditor expects the most from skipping the
  mark when the written digit was already nonzero.
- **Model test.** A `BTreeSet`-oracle test with three calibration swaps.
- **Stops.** Any touch pin moving, or a control row still over its ceiling
  after tuning.

## Leads

- **Lead 1, no defect.**
  [`L1-offset-comparison-composition.md`](L1-offset-comparison-composition.md).
  `compare_offset_to` lives in `version/range_minima/anchor.rs`. Across ten
  board families that reach it, it costs 1.0 to 2.5 touches per operand digit
  at every size. On MirrorWide the gap grows from 16 to 125 stored digits,
  yet every call costs exactly 3 touches. F1's map work on this path is
  inferred, not measured; W removes it.
- **Lead 2, a test gap.**
  [`MB3-readout-high-part-classes.md`](MB3-readout-high-part-classes.md).
  Every extreme readout class is reachable through public updates. Carry −2
  over a zero low part is reached by no committed test, and no extreme class
  has a touch pin. MB3 is a table-driven test over all eight reachable
  classes, stacked on `simplify/suanpan-read-high-part`.
- **Lead 3, holds.**
  [`MB2-add-shifted-wasm32-landings.md`](MB2-add-shifted-wasm32-landings.md).
  `deposit_value` at a huge shift is unpinned on wasm32 through two routes:
  `add_shifted` with a small operand, and `<<=` on a small value. Each
  route's mutant fails only its own new case and passes every committed
  case.
- **Lead 4.** The U1-a brief above.

## Decisions for the owner

1. The F1c band width. The auditor recommends ±2%; a band wider than about
   6.6% would let K1's smallest cell pass. The strict alternative is exact
   pins, re-pinned by `just fuzzfit-calibrate`.
2. Whether to keep F1's `min_ticks` row. The auditor recommends keeping it:
   it shows no F1 growth, but it is the one row where W is cheaper.
3. Stronger suanpan bounds under W. F1d tells the fixer to propose the text,
   not to publish it.

## Observations

- Fuzz-fit's `min_ticks` bands also price decimal rendering, which is
  superlinear. This matters for the instrument survey.
- Comparing a leaf against a tree walks the whole tree. In normal form the
  answer is the root base, so `Version::new() <= v` could be constant time;
  on F1 it costs about 1,650 to 1,900 fuel per byte. This is a performance
  observation, not a defect.

## Gaps not closed

- The readout census did not cover `before`'s suite, because box load was
  about 990.
- No fuel was measured on tick's memo path.
- MB3's calibration swaps are left to its builder.
- F1c's base readings come from the explore tree. Its production code
  matches `main`'s outside `cfg`-gated code, and the builder re-measures
  anyway.

## Inventory

See [`inventory-additions.md`](inventory-additions.md).
