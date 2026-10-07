<!-- CAVEAT LECTOR: maintained by the coordinator (Claude Opus 5.5) during the audit; entries summarize auditors' reports and are unverified unless marked. -->

# Exploratory instruments on the explore branches

Auditors build probes, models, and generators on their private `explore/`
branches. Most of them never reach a reviewable branch, because the auditor
brief admits a new instrument only when it catches a failure no committed
instrument catches. Some of these instruments still go beyond the committed
suite in ways that standard is too narrow to see:

- They reach input regimes the committed generators never produce. The
  adequacy lane's census found that committed arbitrary versions stop near
  21 nodes and depth 4, and that independent pairs are never equal.
- They check against an independent model, rather than one derived from the
  code.
- Their mutants were hand-chosen by the auditor, so "the committed suite also
  caught every mutant" says less than it seems.

This file keeps every such instrument findable until the owner triages it.
No `explore/` branch is deleted before that triage.

## How the triage will work

When every lane has reported, one agent will evaluate the inventory below on
three questions:

1. **Mutation survivors.** Run the adequacy lane's cargo-mutants survivors
   for the instrument's files against it. A survivor it kills is a failure
   only it catches.
2. **Regime reach.** Compare the instrument's generator against the committed
   generators on depth, width, height, sharing, and relation mix, using the
   adequacy lane's census as the baseline.
3. **Where it belongs.** Prefer folding the stronger part into an existing
   shared instrument (`testing::generators`, `testing::laws`,
   `testing::diff_ops`, `testing::exhaustive`) over landing a parallel
   harness.

That agent writes one machinery brief per instrument worth folding, with the
cost under nextest's time limit stated. The owner then rules on each brief.

## Inventory

| Lane | Branch @ commit | Instrument | What it adds beyond the committed suite (auditor's claim) | Auditor's disposition |
|---|---|---|---|---|
| L1 identity | `explore/l1-identity` @ `26cbae22` | `crates/before/tests/audit_l1/` (interval-set model, generators, histories, deep probe, overlay, wide counts), `audit_l1_retention.rs`, cost and statistics probes | Model shares no code with production or the tree oracle. Party generators reach depth about 120 and 128 bytes (committed: depth 4, 4 bytes). Of 24 mutants, the committed suite also caught 21 and missed M19, M20, and M26. Full entries: `lanes/l1-identity/round-1/inventory.md`. | Three machinery briefs (MB1 to MB3) rest on its misses. The generators are not briefed, because the committed suite caught every walk mutant. |
| L2 algebra | `explore/l2-algebra` @ `d78c6129` | `crates/before/tests/l2_probe/` (leaf-list model, tape generators), `l2_cost/` | Independent step-function model with its own canonical encoders. Random topology to depth 300 (committed: depth 4). Heights on the 63-bit boundaries and past the word size. Correlated pairs forcing wide collapse cascades. Killed 20 of 20 mutants. | No brief: the committed suite also killed all 20. It suggests running the adequacy lane's survivors against the probe. |
| L3 events | `explore/l3-events` @ `30379f8d` | `src/version/tick/l3_probe.rs`, `l3_heap.rs` | Co-generated tick pairs, `min_ticks` floor over histories, `min_ticks` tightness | Pending its round report. Two machinery briefs drafted (co-generation; `min_ticks` histories). |
| L4 measures | `explore/l4-measures` @ `2c82c7ba` | `src/version/measure/explore_l4.rs`, `explore_l4_rank.rs` | Integrator harness: 70,000 cases plus 600 with spines to depth 3,000 and heights to 6,000 bits. `Rank` value harness: 40,000 cases. Integrator value checked against always-defer, always-freeze, and never-freeze schedules. | No brief: the committed suite caught all 12 integrator and 6 `Rank` mutants. Its two wasm32 instruments were briefed and are being built. |
| L5 spans | `explore/l5-spans` @ `19080507` | `src/testing/audit_l5.rs` | Grid-model harness, calibrated against 19 mutants | Pending its round report |
| L6 codecs | `explore/l6-codecs` @ `34e6cf2e` | `src/testing/l6_spec.rs` (an independent specification codec for all six wire types), `l6_probes.rs`, `tests/l6_resource.rs` | Differential testing of every decode entry point against an independent codec; reader and writer probes | Pending its round report |
| L7 suanpan | `explore/l7-suanpan` @ `880aa203` | `crates/suanpan/src/accumulator/tests/explore_l7.rs` (pool model, adversarial touch search), `crates/before/tests/explore_l7.rs`, wasm fuel instrument | Pool model: 20,000 programs over three accumulators with arbitrary histories, checked against an exact oracle after every step. Hill-climbing touch adversary. Wasm-fuel cost composition. | Pool model not proposed: the existing suites catch every value mutant it catches. Touch property briefed (MB1, being built). The fuel instrument backs finding F1. |
| L8 adequacy | `explore/l8-adequacy` @ `696d115c` | `src/testing/l8_census.rs` | Generator census: histograms of what the committed generators produce | The census is the baseline for question 2 of the triage |
