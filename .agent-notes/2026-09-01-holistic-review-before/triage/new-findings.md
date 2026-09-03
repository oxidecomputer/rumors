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
  series). A rumors gate (`triage/p2-link` at `575bcc63`) drew the same class at 144 bits (14460 fuel against about 10^3.882); its seed line is saved beside the others (`fuzzfit-enforce-seed-from-rumors-p2-link.txt` in the generators lane's scratchpad). The gate lane's replay settled the dependency question: one fuel
  unit between bytes 1.11.1 and 1.12.1 on the shrunk program; the
  gate lane committed that seed (its wasm stream reads red until the
  refit); the generators lane's two seed files join it in `p1-fuzz`.
- **Two `before` proptests exceed nextest's 180 s limit at 4000 cases**
  (`version_triple_laws`, `ranked_composite_bit_flip_rejects_or_decodes_canonically`);
  not rejections. Route: `p1-suites` (a per-case cost question: the
  suite's cost per draw, or the CI job's case count for those two, is
  the lane's to weigh and report).
- **Corrected: the decode kernel does not run `IdReader`.** `Party::decode` parses through `DsiCursor::read_bit` (two per node) and an `IdFrame` stack, on both sides of `5d167a63`; the tag-pair change below moved `ff_party_decode` by zero. What inside `5d167a63` moved the decode kernel (the bisect stands: in band before, 13633 at that commit) is still unnamed; candidates are the frame stack, the buffered reader's construction, and the frame `Vec`'s growth on deep chains. Route: `p8-performance` (measure `DsiCursor::read_bits(2)` per node and the frame allocation on the seeded program).
- **`BitsView::bit` on the id walk costs about twice the fuel per bit
  of the bit slice it replaced.** (Landed as `p8-tagwalk`: a real win on fork, covers, disjoint, and join; not the decode cause.) The gate lane's bisect over the
  fuzzfit guest names `5d167a63` (the crate-owned `BitsView`): each id
  tag bit is read through an assert, a bounds-checked byte index, and
  a shift in `u64`, which a wasm32 guest lowers to several
  instructions each; `ff_party_decode` reads about 101 fuel per bit
  against its law's 53 on deep fork chains. The band refit is
  `p1-fuzz`'s (above); the cost itself is a measured trade for
  `p8-performance` (a `usize` byte-and-offset cursor for `IdReader::tag`: both tag
  bits read with one shift and mask per node, one bounds check per
  node against `live`, the same access pattern bitvec's slice had on
  the owned representation; measured at the parent, ceilings tightened
  with attribution; ruling 88; the version walk's tag reads checked for
  the same shape).
- **The fuel bands were pinned on 2026-08-04 and never refit; the
  staleness leg's 0.7 dex tolerance hides 36 kernels moving by more
  than 0.05 dex.** Route: `p1-fuzz` (its one calibration run under
  ruling 14 refits every band with the movement attributed; the
  staleness tolerance is a question for that lane's report).
- **`PackedBuilder::read_bits`'s committed-byte chunk arm is
  unreachable through `extract_code`**, which always reads to the
  output's end, so the mutation campaign's surviving operator swap is
  equivalent on every input a caller can produce; the survivors lane
  killed it with a documented internal-entry differential over
  `read_bits(pos, n)`. The alternative is to narrow `read_bits` to
  "read to the end", dissolving the arm and the internal-entry test.
  Route: `p6-codec`.
- **The tag-pair read has siblings the tagwalk lane did not convert**:
  the same two-bit tag is read as two `bit` calls at
  `party/ops/index.rs:81, 101, 201`, `party/ops/diff.rs:313, 353`,
  `party/ops/split.rs:48`, `version/skyline/grow.rs:296`, and the skip
  probe closure has verbatim siblings at `diff.rs:364` and
  `grow.rs:303`; single-bit sequential loops at `codec/buf.rs:354,
  366`, `codec/build.rs:185`, `version/skyline/encode.rs:34`,
  `version/skyline/fill.rs:1090`. Each is a fixed-sign candidate for
  the same measure-at-parent discipline. Route: `p8-performance`.
- **suanpan's crate doc misprices the sign fold's certified skip.**
  The zero-run ledger section says the skip costs one touch instead of
  one per digit; `fold_and_collapse`'s skip through `consume_run_at`
  records no touch (only `settle_top`'s does) and three digits are
  zeroed with a touch each, which the existing pin
  `sign_fold_skips_certified_runs` (6) already reflects. Route:
  `p6-suanpan` (the doc restated from the code; the wholesale exact-pin
  conversion under ruling 88 lands beside it).
