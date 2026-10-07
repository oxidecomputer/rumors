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

## The instrument survey: the audit's final phase

The owner asked for this survey once every lane is done and the rest of the
review is complete. It is the last phase of the audit, not an interleaved
task.

Most of these instruments have not caught a current defect or an injected
regression. Their value lies in going past the current suite's boundaries:
reaching inputs the generators never produce, checking against independent
models, and stating predicates nobody has written down. The survey aims to
turn that work into durable improvements. It covers seven areas:

- **Generator coverage.** Which regimes the committed generators reach only
  with negligible probability, or never, and which explore-branch generator
  constructions reach them cheaply. The adequacy lane's census is the
  baseline, and each candidate gets the same census.
- **Oracles.** Which independent models (the leaf-list model of versions, the
  interval-set model of parties, the specification codec, the accumulator
  pool model, the grid model of spans) would strengthen the committed
  oracles. Each oracle is judged either as a replacement for a committed
  oracle or as a second, independent one.
- **Predicates.** Which properties the auditors checked that no committed
  test states, such as multiplicity, history independence, and order
  independence of folds. For each, ask which plausible future defect it
  would catch.
- **Cost instruments.** Which input families the auditors built to maximize
  work would catch a cost regression the board's current families miss.
  Candidates include the sparse-height family behind the suanpan lane's cost
  finding, the carry-ripple families, fragmented masks, the hill-climbing
  touch adversary, and wasm fuel measurement. Each could become a board
  family or a metered property.
- **Target dependence.** Under the owner's `usize`-invariance rule, which
  pins, build configurations, and checks would make target-dependent
  behavior visible. The adequacy lane found several gaps:
  - The wasm32 pins catch 7 of 11 injected narrowings, and some pins' docs
    claim more than they check.
  - The pins cannot tell a panic from an allocation abort.
  - Nothing measures cost on a 32-bit target.
  - Stack-safety proofs are void at `opt-level = 0`.

  One candidate is pinning the list of `usize` uses in public signatures in
  `surfacecheck`.
- **Checks on the evidence itself.** Each candidate states the failure that
  would go unnoticed without it:
  - generator-reach floors pinned from the census, so a future generator
    change cannot silently narrow what the generators reach
  - a committed known-bad demonstration for each instrument, proving it can
    still fail
  - the mutation survivor list, as a baseline a later campaign is compared
    against
- **The verification map.** The validation index and the crates'
  `# Testing` sections, updated for everything folded in. The index has no
  entry for `wasm32-pins` or for any `suanpan` instrument today.

The survey's output is one ranked list across all seven areas, so the owner
reviews a single document.

### Sequence

1. **Survey.** One agent reads every lane's inventory entries and explore
   branch. It runs the adequacy lane's mutation survivors against each
   instrument, and runs the census on each generator. It writes a survey
   document with its findings in the seven areas, each ranked by the failure
   class it would guard against and its cost under nextest's time limit. The
   coordinator commits the survey here.
2. **Proposal branches.** For each of the best enhancements, a builder
   implements one polished branch, one enhancement per branch, extending the
   shared instruments (`testing::generators`, `testing::laws`,
   `testing::diff_ops`, `testing::exhaustive`, the oracles) rather than
   landing parallel harnesses. The branches are sequenced so that each
   builds on the shared instruments the earlier ones extend. Each branch gets
   the usual skeptical review.
3. **Owner review.** Nothing from the survey is folded in without the owner's
   review. The branches are proposals: like every audit branch, they wait
   for the owner and never land autonomously.

## Inventory

| Lane | Branch @ commit | Instrument | What it adds beyond the committed suite (auditor's claim) | Auditor's disposition |
|---|---|---|---|---|
| L1 identity | `explore/l1-identity` @ `26cbae22` | `crates/before/tests/audit_l1/` (interval-set model, generators, histories, deep probe, overlay, wide counts), `audit_l1_retention.rs`, cost and statistics probes | Model shares no code with production or the tree oracle. Party generators reach depth about 120 and 128 bytes (committed: depth 4, 4 bytes). Of 24 mutants, the committed suite also caught 21 and missed M19, M20, and M26. Full entries: `lanes/l1-identity/round-1/inventory.md`. | Three machinery briefs (MB1 to MB3) rest on its misses. The generators are not briefed, because the committed suite caught every walk mutant. |
| L2 algebra | `explore/l2-algebra` @ `d78c6129` | `crates/before/tests/l2_probe/` (leaf-list model, tape generators), `l2_cost/` | Independent step-function model with its own canonical encoders. Random topology to depth 300 (committed: depth 4). Heights on the 63-bit boundaries and past the word size. Correlated pairs forcing wide collapse cascades. Killed 20 of 20 mutants. | No brief: the committed suite also killed all 20. It suggests running the adequacy lane's survivors against the probe. |
| L3 events | `explore/l3-events` (5 signed commits) | `src/version/tick/l3_probe.rs` (co-generated (version, party) pairs: bushy, spine, wide, multi-scan; `min_ticks` floor over histories; tightness), `l3_heap.rs` (release heap probes), diagnostics (generator reach, counter search) | Reach: nesting 7 or deeper in 42% of draws, two or more lookahead sites per range in 25%, more than 64 memo slots per pre-scan in 66% (committed: at most 2 slots, no site in 81% of draws). Catches four mutants (M6, M8, M15, M19) that pass all 72 committed tick tests. Full entries: `lanes/l3-events/round-1/instruments-inventory.md`. | Two machinery briefs (co-generation; `min_ticks` histories). |
| L4 measures | `explore/l4-measures` @ `2c82c7ba` | `src/version/measure/explore_l4.rs`, `explore_l4_rank.rs` | Integrator harness: 70,000 cases plus 600 with spines to depth 3,000 and heights to 6,000 bits. `Rank` value harness: 40,000 cases. Integrator value checked against always-defer, always-freeze, and never-freeze schedules. | No brief: the committed suite caught all 12 integrator and 6 `Rank` mutants. Its two wasm32 instruments were briefed and are being built. |
| L5 spans | `explore/l5-spans` @ `2874e0c3` | `src/testing/audit_l5.rs` (grid model, shaped and organic generators, exhaustive 4-cell cube, coverage census, cost probes) | Oracle shares no code with production. Placement mix: `Before` and concurrent verdicts at 9.6 to 12.6% (committed: 1.5 to 2.4%). Depth 17 to 64 for 338 of 771 shaped versions (committed: 4). Two to five pairwise-concurrent holes in about a third of cases (committed: at most two). Exact coverage on organic shapes (committed: grid only). Full entries: `lanes/l5-spans/round-1/inventory.md`. | No machinery brief: the committed suite caught all 17 of its non-equivalent mutants. |
| L6 codecs | `explore/l6-codecs` (9 signed commits) | `src/testing/l6_spec.rs` (independent specification codec for all six wire types, differential against every entry point), `l6_probes.rs` (reader and writer misbehavior), `tests/l6_resource.rs` (hostile resource families), `l6_writer.rs` (copy paths), wasm32 growth probes | Spec codec: 300,000 cases per type, twice; version depth above 32 in 11.8% of inputs (committed: 4); planted negative heights 22.3%, collapsible pairs 15.5%, truncated trees 32.2%. Error-class agreement on every rejection. Full entries: `lanes/l6-codecs/round-1/coverage.md`. | No machinery brief: the committed suite caught all 17 of its mutations. The wasm32 probes back the growth defect. |
| L7 suanpan | `explore/l7-suanpan` @ `880aa203` | `crates/suanpan/src/accumulator/tests/explore_l7.rs` (pool model, adversarial touch search), `crates/before/tests/explore_l7.rs`, wasm fuel instrument | Pool model: 20,000 programs over three accumulators with arbitrary histories, checked against an exact oracle after every step. Hill-climbing touch adversary. Wasm-fuel cost composition. | Pool model not proposed: the existing suites catch every value mutant it catches. Touch property briefed (MB1, being built). The fuel instrument backs finding F1. |
| L8 adequacy | `explore/l8-adequacy` @ `696d115c` | `src/testing/l8_census.rs` | Generator census: histograms of what the committed generators produce | The census is the baseline for question 2 of the triage |
