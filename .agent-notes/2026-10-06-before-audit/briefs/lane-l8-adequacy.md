<!-- CAVEAT LECTOR: written by Claude (Opus 5.5) as the lane section for the L8 auditor, modeled on the L6 codec lane; under review by Finch before launch. -->

# Lane L8: adequacy of the instruments

This lane audits the evidence rather than the code. Its findings are mostly
routed to other lanes; its own deliverables are machinery briefs and
instrument defects.

## Scope

This lane's theme is *the evidence itself*: whether the crate's tests, oracles,
generators, and meters can detect the failures they claim to. The other lanes
ask "is the code right?"; this lane asks "would we know if it weren't?"

Some things to consider, as starting points rather than a checklist. They
are prompts for your own exploration: pursue whatever else the code
suggests, and expect the most valuable questions to be ones not listed
here.

- Which mutants survive, and at which step does detection break down? A
  mutant can go unreached, infect no state, fail to propagate to an output,
  or reach an output no assertion checks.
- Which regimes do the generators reach only with negligible probability?
- Can each instrument fail on the input family it claims to cover?
- Are the meters deterministic across platforms, as the board claims? The
  `count_display` ranking difference in `baseline.md` is a lead.

Mutation testing and branch coverage give a mechanical first map. Measuring
what the generators actually produce shows which inputs the tests ever see.
The validation index gives each instrument a failure class it must be able to
catch. Most of what this lane finds is routed to other lanes, through the
coordinator.

The identifiers below are a starting map, not a boundary. Follow the theme
wherever the code takes it, including into code this list does not name.

- **Mutation testing:** cargo-mutants over `crates/before/src` (excluding
  `src/testing/`) and `crates/suanpan/src`.
- **Branch coverage:** llvm-cov on the pinned nightly. You may install the
  `llvm-tools` component into the `agent` account's rustup on the box.
- **Shared test support:** `src/testing/generators.rs`, `optrace.rs`,
  `exhaustive.rs`, `diff_ops.rs`, the oracles, the bridge.
- **The amplification board and meters:** `src/testing/meter/`,
  `examples/amp_board.rs`, `tests/meter/`.
- **The 32-bit pins:** `crates/before/wasm32-pins`.
- **The surface check:** `crates/before/surfacecheck`.

## Contracts to read first

- The validation index (`src/testing/validation_index.rs`): each
  instrument's claim of the failure class only it catches.
- The board's claims in `src/testing/meter/board.rs`: deterministic counters,
  liveness floors, and a known-bad artifact convicted for each judgment.

## Operating rules

These protect the box and the other lanes.

- **Estimate before mutating.** Count the mutants first
  (`cargo mutants --list`) and give the coordinator an estimate of a full
  run's duration before starting it.
- **Share the box.** Run under `nice` with a capped job count; other agents
  share it.
- **Report as you go.** Write survivors and uncovered branches incrementally,
  to a file per module, so the coordinator can route them while the run
  continues.

## Things to consider

These are starting points rather than a checklist: pursue whatever else the
evidence suggests.

- **Where to start mutating.** Which modules are likely to yield most? Core
  kernels are a natural place to begin.
- **Branch coverage per module,** as a first map of what nothing reaches.
- **What the shared generators actually produce.** Histograms of depth,
  width, height, party fragmentation, and operation mix. Which regimes do they
  reach only with negligible probability?
- **The `count_display × heap` ranking difference** between ox-east-1 and the
  pin (see `baseline.md`). Does the heap reading depend on the platform, or
  does the ranking rest on a near-tie? Either answer bears on the board's
  claim to be deterministic.
- **Can each instrument fail on the input family it claims to cover?**
