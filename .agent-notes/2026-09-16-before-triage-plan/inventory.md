# Evidence and inherited state

Checked on 2026-09-16 while creating `codex/before-triage` from `main` at
`437604d1a`. This is an orientation record, not a new correctness verdict.

## Sources read

The restart read the current crate-level documentation and skyline module
documentation, the current testing architecture, `crates/before/AGENTS.md`,
the review overview and orientation map, the eight class syntheses and their
highest-value findings, the evidence partition and sweep indexes, the witness
and dependence summaries relevant to the leading defects, and the aborted
triage's workflow, rulings index, handoff, briefs, later findings, and ledger
shape. It also read the Rumors restart README in full and sampled its checklist
and inventory for structure.

Primary evidence:

- [Review overview](../2026-09-01-holistic-review-before/README.md)
- [Correctness](../2026-09-01-holistic-review-before/correctness.md)
- [Claims](../2026-09-01-holistic-review-before/claims.md)
- [Verification](../2026-09-01-holistic-review-before/verification.md)
- [Simplification](../2026-09-01-holistic-review-before/simplification.md)
- [Documentation](../2026-09-01-holistic-review-before/documentation.md)
- [Performance](../2026-09-01-holistic-review-before/performance.md)
- [API](../2026-09-01-holistic-review-before/api.md)
- [Rumors dependence](../2026-09-01-holistic-review-before/dependence.md)
- [Orientation map](../2026-09-01-holistic-review-before/evidence/map.md)
- [Witness constructions](../2026-09-01-holistic-review-before/evidence/witness.md)

The class documents copy findings from 34 partition reports and 13 sweeps. Use
the detailed report for an entry only after confirming its anchor against the
current source. The review was performed at `9e5784fb`; line numbers and some
architecture no longer describe `main`.

The entire aborted `triage/` directory is non-normative. Its recorded owner
rulings reflect prior conversations but require current confirmation whenever
they would decide a change. Its ledger is useful as an ID index, not as scope,
status, acceptance, or proof that a fix exists.

## Current tree differs from the reviewed tree

The diff from `9e5784fb` to the restart baseline changes `before` source,
tests, documentation assets, and fuzz-fit seeds. Some changes arose from later
Rumors work; some are the small portion of the aborted `before` triage that
reached `main`.

The clearest inherited triage change is the resource-envelope harness:

- commits `5a6be5c0` through `4604ad1c` unified four harnesses, removed the
  envelope's segments column, pinned every remaining column, changed rows, and
  added several follow-up repairs;
- `crates/before/tests/meter.rs` is still about ten thousand lines;
- the board retains a separate segments currency throughout its types, rows,
  measurement, judgment, and rendering, so the dead-column question was only
  partly resolved;
- commits `7892c60c` through `f60734bb` changed proptest case-count behavior in
  semantic-oracle and fuzz-fit tests.

None of this is accepted merely because it landed. Section 01 of the checklist
reviews the current result as code and may simplify or reverse it.

Current guideposts also need correction: `crates/before/AGENTS.md` directs the
reader to a public `implementation` module that is not exported and describes
verification structure that has already changed. The crate page and skyline
module remain long and use the vocabulary and sentence structures this triage
is meant to remove.

## Leading correctness claims still present in current source

Read-only checks found current code matching several important review
mechanisms:

- The current fork increment removes the `ExactSizeIterator` contract and the
  fixed `u64` count. Both public methods accept the existing unbounded `Ticks`
  vocabulary, and a direct wasm32 pin covers the former `2^32` trap.
- The fill memo's links and query ledger's epochs use vector-native `usize`
  indices. The current Party fold increment removes its auxiliary position
  table and keeps only the balanced fold's logarithmic working set.
- Suanpan's shifted-digit landing bug is repaired in the current review
  increment: the full position is computed before its one checked conversion,
  with native boundary witnesses and direct wasm32 release coverage.

These observations establish priority, not the final repair. Each batch must
trace the complete current path and reproduce or prove the boundary before
editing it.

## Initial instrument inventory

The verification architecture currently includes:

- recursive and function-space oracles;
- algebraic laws, operation descriptors, proptest populations, and exhaustive
  enumeration;
- a public-surface roster, source scanners, rustdoc-surface checks, citation
  checks, and several coverage tables;
- a ten-thousand-line resource-envelope suite and a similarly large
  amplification board with heap, stack-segment, limb, scan, and touch
  currencies;
- asymptotic liveness pins, superlinear tripwires, worst-case rankings, a bench
  judge, and criterion benches;
- fuzz targets, seed derivation checks, fuzz-fit fuel bands, a wasm32 guest and
  harness, fuelscape sampling, committed datasets, and rustdoc charts;
- coverage pins, mutant-list pins, dependency audits, and generated-document
  checks.

Several initial dissolution candidates are strong enough to investigate
early, but not yet decided:

- **Suanpan's global touch meter:** its exact-count tests interfere under
  Cargo's default parallel test runner (15 false failures in one observed run)
  and pass serially. Determine whether to isolate the counter, serialize the
  tests explicitly, or consolidate the exact-count layer before treating an
  all-feature run as a reliable instrument.

- **Stack segments:** the review found the production readers disconnected from
  their only writer. The envelope column was deleted, but the board still
  carries the currency across hundreds of declarations and explanations.
- **Surface and citation rosters:** several layers derive or scan overlapping
  descriptions of the same public surface and named tests. Their main signal
  may be obtainable from compiler-visible APIs plus ordinary property drivers.
- **Asymptotic liveness versus resource enforcement:** some tests preserve the
  existence of a slow mechanism while envelopes, board fits, fuel bands, and
  benches separately claim to constrain it. The distinct useful signals need
  to be stated before retaining all layers.
- **Heap constants:** widening the fill ledger made the board's 16 B/B global
  ceiling reject three linear tick cells at 16.2–18.5 B/B. The shape-specific
  envelope usefully isolated the changed allocation. The board remained useful
  after recalibrating its one ordinary ceiling to 24 B/B; adding another
  family-specific model would have preserved the old number rather than a
  distinct contract. The later board/envelope audit must apply this same test
  to the existing declarations.
- **Fuelscape:** the population view may help exploration, but charts and
  committed datasets do not automatically enforce worst-case contracts. Its
  build hooks and generated-doc footprint must justify themselves separately
  from fuzz-fit enforcement.
- **Mutation roster (retired):** the gate only listed syntax and checked
  exclusion counts; it ran no mutation campaign and therefore provided no
  behavioral signal. A concise documentation edit broke an exclusion keyed to
  a source line, demonstrating its maintenance cost directly. The owner chose
  retirement: the exclusions, count checker and expected file, recipes, CI
  install, and supporting guide text are removed together. Tests discovered by
  ordinary runners remain the verification of record.

The audit may find that an elaborate instrument is uniquely valuable. In that
case the work is to make its claim, inputs, and failure leg obvious and remove
the surrounding ceremony.

## Prepared branches from the aborted attempt

These refs remain available as historical experiments:

- `before/p1-board`
- `before/p1-board-residual-fit`
- `before/p1-fuzz`
- `before/p1-gate`
- `before/p1-survivors`
- `before/p2-census`
- `before/p2-generators`
- `before/p2-surface`
- `before/p2-widths`
- `before/p8-tagwalk`

Their tips are review-packet or notes commits, and their stacks were built on
old bases. Do not rebase, merge, or revive them. A current batch may inspect a
specific code commit or counterexample, then implement the justified idea
afresh on `codex/before-triage`.

The untracked `.worktrees/` directory predates this restart and is left
untouched.

## Known baseline questions

- The Rumors checklist's enabled fuzz-fit failure reproduced on `437604d1`:
  `ff_party_decode` at 136 bits consumed 13,612 fuel, above the pinned law's
  allowance. The failing property printed the same counterexample repeatedly,
  created no new regression seed, and remains enabled. The old `before`
  handoff's additional `ff_clock_sync` error-arm seed did not fail this run.
  Diagnose the implementation and the band before changing either.
- The old review reported coverage-only heap movement on an unchanged tree.
  Establish whether that remains a supported deterministic claim before
  preserving the pin or its coverage integration.
- The crate page's “approximately 100x” space claim and several absolute size
  figures were not tied to committed measurements in the review. They remain
  claims to prove or remove.
- `just check` is intentionally a no-op because other recipes type-check the
  tree. The first baseline should therefore use the actual gate legs rather
  than treating `just check` as evidence.

## Baseline verification

`just gate` ran on the unchanged restart tree. The lint, workspace, doctest,
board, fuzz-build, public-surface, internal-docs, docs.rs, and supply-chain
streams passed. The gate failed only in the wasm stream at the fuzz-fit case
above. Because that recipe stopped before its later commands, the remaining
legs were run directly:

- `just wasm32-pins`: 48 passed, 0 skipped;
- `just fuelscape-test`: 42 passed, 0 skipped; nextest reported two leaky tests
  but returned success.

The direct commands initially could not open the configured `/Volumes/forge`
build locks inside the filesystem sandbox; rerunning the same recipes with
their configured build access succeeded. This was an execution-environment
restriction, not a repository failure.

No proptest seed, source file, pin, generated artifact, or lockfile changed.
The only untracked paths remain this plan and the pre-existing `.worktrees/`.
