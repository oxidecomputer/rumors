<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as the before triage's record of findings that have no ledger row (found during lane reviews, by the rumors triage, or by the coordinator), each routed to the brief of the lane that owns the file; not authored, audited, or endorsed by Finch. A finding leaves this file when its lane lands it and the ledger gains its row. -->

# Findings without a ledger row

Each entry: where it came from, what it is, which lane's brief carries it
(the brief holds the resolution; this file is the index).

- **The meter suite's pins are dev-profile numbers.** The rumors CI
  campaign (release profile, any `PROPTEST_CASES`) fails seven envelope
  rows on the limb column's improvement tripwire: `join_cliff`,
  `skyline_join_cliff`, `skyline_project_comb_scatter`,
  `skyline_render_cliff`, `skyline_render_hugeleaf`, `tick_expand_cross`,
  `tick_expand_spine` (for example `tick_expand_cross: limb counter reads
  2, below the 3 improvement tripwire`). Debug-assertion comparisons are
  metered limb work that vanishes under release, so the readings hold
  only under the dev profile. Route: `p2-rows` (the suite states and
  checks its profile at entry); the rumors release job excludes the
  meter binary with the reason stated. Recorded from the rumors session's
  logs (`p1-proptest-ci/run-{1,256,1000,4000}.log` in its scratchpad).
- **Two `version` proptests stop scaling at proptest's global-reject
  cap.** At `PROPTEST_CASES` of 1000 and above,
  `version::tests::grow_matches_brute_force` and `grow_minimal` abort
  with "Too many global rejects" at the `prop_assume!` on
  `crates/before/src/version/tests.rs:560` (`ov.fill_for_test(&op) ==
  ov`), which rejects most draws (322 and 343 successes before 1024
  rejects). The generator should draw the constrained value directly.
  Route: `p6-core` (the version tests). Same source as above.
- **`JOIN_WIDE_TOOTH`'s unattributed limb rise; deep-input rows for
  project, distance, lag, and the masked comparisons.** From the harness
  lane's review. Route: `p2-rows` (its brief's handoff section).
- **The `id_walk_scan_cost` band's floor premise against the envelope's
  `LiveBits` floor.** From the harness lane's review. Route: `p1-suites`.
- **Three line scanners remain in `crates/before/tests`** after the
  censuses moved to `surface-scan`'s parser. From the surface lane's
  review. Route: `p5-scanners`.
- **A re-stamped fuelscape dump is indistinguishable from a plan change**
  under compaction's measure check. From the surface lane's review.
  Route: `p6-fuelscape`.
- **The fuzz crate's unit tests run under no recipe; the wasm32-pins
  harness gained a panic-message channel the fuzz lane's discriminator
  should fold in.** From the widths lane. Route: `p1-fuzz`.
- **The `fuzzfit` recipe comment says "48 fuzzed programs"**, stale
  under ruling 109. From the proptest-cases lane. Route: `p1-gate`
  (the justfile's owner this wave).
- **The `ff_party_decode` fuel band is too tight for deep fork chains
  with tick runs.** At 4000 cases with zero reject budgets, the fuzzfit
  sentry breaches that band on main's own tree (bytes 1.12.1): 136 and
  144 bits at about 1.9x the pinned law, on programs the 48-case
  calibration never drew. The gate lane's `bytes` downgrade (ruling 114
  reverses it) only shifted the constant so the default count and the
  committed seed hit the same wall. The shrunk seeds are saved at the
  generators lane's scratchpad (`fuzzfit-enforce-seed-at-base.txt`,
  `fuzzfit-enforce-seed-after.txt`, seven `cc` lines) and not committed,
  since committing them reads `just fuzzfit` red until the band is
  refit. Route: `p1-fuzz` (ruling 14's calibration: the family joins the
  vocabulary, the law is refit, the seeds land with the refit in one
  series). The gate lane's replay settled the dependency question: one fuel
  unit between bytes 1.11.1 and 1.12.1 on the shrunk program; the
  gate lane committed that seed (its wasm stream reads red until the
  refit); the generators lane's two seed files join it in `p1-fuzz`.
- **Two `before` proptests exceed nextest's 180 s limit at 4000 cases**
  (`version_triple_laws`, `ranked_composite_bit_flip_rejects_or_decodes_canonically`);
  not rejections. Route: `p1-suites` (a per-case cost question: the
  suite's cost per draw, or the CI job's case count for those two, is
  the lane's to weigh and report).
