# `before` review checklist

Source IDs refer to the
[2026-09-01 review](../2026-09-01-holistic-review-before/README.md). The old
triage's dispositions and branches are leads only.

## 01. Current baseline and inherited work

- [x] Run the current gate and its `before`-specific legs once; record genuine
      baseline failures without normalizing pins or changing code.
      Sources: `gate-legs-*`, `deps-*`, `suite-economics-*`; later fuzz-fit
      failures indexed in the old triage's `new-findings.md`.

- [ ] Audit the meter-harness and proptest changes already landed from the
      aborted triage; retain, simplify, consolidate, or revert each coherent
      mechanism by current evidence.
      Sources: `envelopes-a-*`, `envelopes-b-*`, `meter-adequacy-*`,
      `tests-other-*`; aborted-triage changes.

- [ ] Correct crate guideposts that already point to missing modules or stale
      verification architecture.
      Sources: `crate-root-*`, `module-graph-*`, `prose-hygiene-*`;
      `crates/before/AGENTS.md` and the current crate docs.

## 02. Width and platform correctness

- [x] Compute suanpan digit positions without intermediate `usize` overflow,
      and test the shared landing boundary through native witnesses and the
      existing shifted-operation properties, plus direct wasm32 execution.
      Sources: `suanpan-24`, related accumulator and wasm witness evidence.

- [x] Make the fork iterator contract correct on 32-bit targets and at the
      maximum public count. Any public signature change requires prior approval
      and a matching Rumors update.
      Sources: `clock-3`, `clock-17`, `party-13`, `party-14`,
      `api-audit-6`, `api-audit-10`, `tests-other-17`.

- [x] Remove fixed `u32` caps from fill, query, and fold bookkeeping where the
      public contract is bounded only by memory.
      Sources: `inventory-1`, `skyline-fill-grow-23`, `recursion-5`,
      `skyline-query-24`, `party-23`.

- [x] Remove the backend-capacity boundary from wide gamma values and keep the
      wasm guest's own arithmetic from becoming the tested failure.
      Sources: `codec-bits-23`, `fuzz-guests-pins-26`,
      `fuzz-guests-pins-27`.

## 03. Canonical encoding and serialization

- [x] Make serde deserialize the same data model it serializes and preserve
      strict canonical decoding across supported formats.
      Sources: `crate-root-34`, `crate-root-35`, `fresh-eyes-2`.

- [ ] Cover every canonicality condition through the public decoders,
      including the span admission walk's collapsible-pair boundary.
      Sources: `skyline-coding-6`, `testing-oracles-4`, codec rejection
      findings and witness evidence.

- [ ] Reconcile marker padding, truncation, trailing input, and error
      precedence across the id, version, rank, clock, span, serde, and borsh
      entries.
      Sources: `codec-bits-8`, `codec-base-text-tree-*`, `rank-10`,
      `skyline-coding-31`, `fresh-eyes-2`, `api-audit-8`.

- [ ] Exercise serde and borsh through their actual buffering and ownership
      paths on wide-leaf and deep-topology inputs rather than delegating their
      resource claims to slice-backed codecs.
      Source: independent resource-surface audit, 2026-09-17.

## 04. Core semantic verification

- [ ] Consolidate the Party and Clock properties around disjointness,
      linearity, fork/join conservation, and stale-state hazards.
      Sources: `party-*`, `clock-*`, `oracle-laws-*`, `testing-diff-gen-*`.

- [ ] Consolidate Version, Span, Rank, projection, and causal-query properties
      around their algebra and independent oracles.
      Sources: `version-core-*`, `rank-*`, `span-causally-*`,
      `oracle-laws-*`, `testing-oracles-*`.

- [ ] Exercise iterative traversal over the meaningful deep shapes and branches,
      including right descents and both-present frames, without retaining a
      decorative stack metric.
      Sources: `clock-22`, `recursion-*`, `meter-adequacy-1`,
      `skyline-query-29`.

## 05. Time and auxiliary-space contracts

- [x] Restore `Version::join`'s bound on wide-leaf/deep-spine combinations.
      Sources: `skyline-coding-9`; Rumors dependence ledger.

- [x] Stop projected and masked comparisons from repeatedly scanning a parked
      cursor's trailing run.
      Sources: `skyline-sweep-place-masked-5`, `codec-bits-29`.

- [ ] Give `Ranked::cmp`, rank folds, and `sum_ranks` contracts their actual
      worst-case implementations and useful properties.
      Sources: `rank-20`, `rank-33`, `skyline-query-9`.

- [ ] Decode `Rank` directly from `Read` without retaining a redundant copy of
      the complete input. Preserve truncation, padding, trailing-input, and I/O
      errors, then reassess the fraction and numeric-assembly temporaries.
      Source: owner review, 2026-09-17.

- [x] Make multi-hole query refinement scale with the declared inputs. Cover
      concurrent `since` holes against one wide probe for transient space, and
      many surviving `until` holes against a dense probe for total work, while
      varying both tree size and hole count.
      Sources: `span-causally-24`, `span-causally-36`,
      `skyline-sweep-place-masked-21`.

- [x] Make full drains of `Party::forks`, `Clock::forks`, consuming array
      splits, and `Clock::sync_all` honor their aggregate bounds. Exercise
      arity scaling where scattered input parties join to an alternating-leaf
      union, so producing every child cannot hide repeated scans or copies
      behind a one-child iterator probe.
      Source: independent resource-surface audit, 2026-09-17.

- [ ] Price Query conjunction over independently growing hole sets, and make
      clone and ownership-conversion claims account for unbounded hole count.
      Reuse concurrent singleton bounds so retained holes, rather than early
      elimination, drive the measurement.
      Source: independent resource-surface audit, 2026-09-17.

- [x] Remove paper-notation text I/O and literal construction instead of
      maintaining their parsers, renderers, and complexity instruments.
      Sources: `party-11`, `clock-14`, `version-core-16`,
      `codec-base-text-tree-13`, `skyline-coding-20`, `skyline-coding-29`.

- [x] Bound tick's memo and suspended-level storage by a small constant multiple
      of input size without introducing a second representation solely for a
      meter.
      Sources: `skyline-fill-grow-2` and its witness.

- [x] Apply the global transient-heap ceiling to `min_ticks` by storing its
      open boundaries and reign records compactly, with exact fallbacks.
      Sources: `skyline-watermark` open question 4; owner review, 2026-09-17.

- [x] Make each arbitrary-count Party and Clock fork step linear in the stored
      values and count representation, with compact state per open ancestor.
      Bring iterator construction, one step, and partial drop under direct heap
      judgment.
      Source: owner resource-surface audit, 2026-09-17.

- [ ] Recheck every remaining public complexity and allocation claim against
      its implementation; meet it, correct it, or bring an unattainable bound
      to the owner.
      Sources: all remaining entries in `claims.md` and `dependence.md`.

## 06. Production structure and local simplification

- [ ] Simplify Party indexing, splitting, sum, difference, and fold paths while
      preserving their direct properties and costs.
      Sources: `party-*`, `performance.md` Party entries,
      `simplification.md` Party entries.

- [ ] Simplify skyline coding, fill, grow, comparison, query, and watermark
      layers so their invariants live in one place and their control flow is
      reviewable.
      Sources: all `skyline-*` partitions; `module-graph-*`.

- [ ] Consolidate codec buffers, cursors, builders, and parse paths; delete
      redundant validation passes and representations.
      Sources: `codec-bits-*`, `codec-base-text-tree-*`, codec performance
      entries.

- [x] Simplify suanpan's representation and verification surface around the
      operations `before` actually needs, preserving generality only where it
      carries a clear contract.
      Sources: `suanpan-*`, `suanpan-tests-*`.

- [x] Eliminate `dashu` if the remaining rank and accumulator arithmetic can
      be expressed more simply without it. The intended outcome is one
      arbitrary-width representation, with no backend-capacity boundary or
      small/wide `Num` split, while retaining input-proportionate memory use.
      Sources: `rank-*`, `suanpan-*`, `deps-*`; owner handoff, 2026-09-16.

## 07. Semantic instruments and generators

Broad test-suite cleanup follows the behavioral and API work; do not mix it
into otherwise small feature increments.

- [ ] Give the recursive oracle, function-space oracle, algebraic laws, and
      exhaustive enumeration distinct jobs; consolidate duplicated operation
      descriptors, populations, and drivers.
      Sources: `oracle-laws-*`, `testing-oracles-*`,
      `testing-diff-gen-*`, `paper-fidelity-*`.

- [ ] Replace hand-maintained surface rosters and source scanners with the
      smallest reliable public-surface check, or retire them where compiler and
      ordinary tests already provide the signal.
      Sources: `surface-roster-*`, `api-audit-*`, `tools-*`.

- [ ] Generate constrained proptest inputs directly, share useful generators,
      and remove rejection-heavy or decorative case-count machinery.
      Sources: generator findings in `testing-diff-gen-*`, `tests-other-*`,
      and the old triage's later findings.

## 08. Resource instrumentation

- [ ] Make the global heap judgment total over every allocation-bearing public
      method and trait family. Directly cover shape walks, compound Span and
      causal-query operations, Rank text parsing and precision formatting, and
      wide numeric operations. Include arity-dependent `shape::combine`
      bookkeeping; reserve not-applicable dispositions for paths that cannot
      amplify.
      Source: owner resource-surface audit, 2026-09-17.

- [ ] Audit every counter hook for a distinct, live observation. Remove dead
      currencies and production hooks whose only consumer is decorative.
      Sources: `meter-core-*`, `inventory-*`, `module-graph-*`;
      stack-segment findings across `board-*`, `recursion-1`, and
      `envelopes-a-2`.

- [ ] Make the board's headline bounds match what it proves. Reconcile the
      flat heap allowance and row-specific denominators with any universal
      amplification claim, and ensure plain formatting, parsing, and integer
      work cannot read green merely because scan and touch do not observe it.
      Source: independent resource-surface audit, 2026-09-17.

- [ ] Consolidate resource amplification into the board. Model each family as
      valid operands, derive every bundle those operands support, and apply it
      to every compatible public operation rather than only the operation that
      motivated the family. Promote useful envelope-only families, and preserve
      the correlated pairs, masks, and populations that make them difficult
      instead of replacing those relationships with generic adapters. Retain a
      separate instrument only for a distinct claim the board cannot express,
      and remove the duplicated harness, pins, and prose otherwise.
      Sources: `envelopes-a-*`, `envelopes-b-*`, `board-frame-*`,
      `board-families-floors-judge-*`, `board-ops-render-*`,
      `meter-adequacy-*`.

- [x] Remove the bench judge; use deterministic WASM fuel measurements for
      general time and complexity verification. Reassess worst-case rankings,
      asymptotic liveness pins, and superlinear tripwires separately.
      Sources: `benches-examples-*`, `tests-other-*`, `suite-economics-*`,
      `tools-*`.

- [ ] Reassess fuzz-fit and fuelscape separately. Retain fuelscape's rustdoc
      panels as explanatory views of cost distributions and edge families;
      simplify their refresh pipeline where possible. Retain fuzz-fit
      enforcement that catches unchosen worst-case shapes, while removing
      copied vocabulary and generated artifacts that add no distinct signal.
      Sources: `fuzzfit-strategies-*`, `fuzzfit-bands-*`,
      `fuelscape-pipeline-*`, `fuelscape-render-*`.

## 09. Fuzzing, platform pins, gate, and dependencies

- [ ] Give fuzz targets and seed replay a clear cadence and unique purpose;
      preserve all committed seeds and remove duplicate framing and roster
      machinery.
      Sources: `fuzz-guests-pins-*`, `tests-other-*`, `gate-legs-*`.

- [ ] Keep a direct 32-bit boundary suite if it catches behavior ordinary host
      tests cannot; simplify its guest, harness, terminal vocabulary, and
      dependency footprint.
      Sources: `fuzz-guests-pins-*`, `deps-*`, width findings in section 02.

- [ ] Make gate and CI recipes derive their inputs, run at their documented
      cadence, and stay green on an unchanged tree. Retire mutation, coverage,
      and citation machinery that only checks rosters rather than behavior.
      Sources: `gate-legs-*`, `deps-*`, `tools-*`, `surface-roster-*`.

- [ ] Prune unused dependencies and generated assets after instrument
      consolidation; verify each supported feature combination.
      Sources: `deps-*`, `inventory-*`, `module-graph-*`.

## 10. Public API and contracts

- [x] Render and parse `Rank` as canonical binary-point text, with formatter
      width and alignment behaving like other textual values. Human-readable
      serde uses this text; binary serde retains canonical encoded bytes. Test
      the mathematical value, accepted language, public formatting, and linear
      scaling without pinning integer-backend implementation details.
      Sources: prior rulings 85--86, critically reviewed against the current
      code; owner handoff, 2026-09-16.

- [ ] Review `#[must_use]`, fork iterator naming and count type, error source
      chains, error extensibility, missing trait symmetry, and human-readable
      forms as one coherent API proposal before implementation.
      Sources: all entries in `api.md`; approval required for each external
      change.

- [ ] Put the value, size, and cost guarantees Rumors relies on at the public
      methods that own them.
      Sources: `rumors-dependence-1`, `rumors-dependence-2`,
      `rumors-dependence-5`, and the dependence ledger.

- [ ] Update and test Rumors in the same batch as every approved external
      `before` API change.
      Source: owner instruction, 2026-09-16.

## 11. Documentation and module layout

- [ ] Rewrite the crate page into a concise model, safety contract, type tour,
      and correct examples; ground or remove every quantitative headline.
      Sources: `crate-root-*`, `paper-fidelity-*`, `fresh-eyes-*`.

- [ ] Rewrite public item docs around caller-visible contracts, errors, panics,
      and attainable bounds; remove internal vocabulary and copied mechanism.
      Sources: `documentation.md`, `claims.md`, `api.md`.

- [ ] Simplify maintainer docs and comments across touched modules, removing
      stale references, invented jargon, rosters, history, and contorted
      phrasing.
      Sources: `documentation.md`, `simplification.md`,
      `prose-hygiene-*`, `module-graph-*`.

- [ ] Simplify module boundaries after behavioral work settles, and keep the
      guideposts accurate.
      Sources: `module-graph-*`, structural entries in `simplification.md`.

## 12. Finish and reconcile

- [ ] Re-read every original finding against the integrated branch; assign any
      surviving issue to an open outcome or record an explicit disposition.

- [ ] Verify the complete branch with `just gate`, the justified slower checks,
      feature combinations, doctests, generated-document checks, and Rumors.

- [ ] Rebase onto current `main`, repeat affected verification, and present the
      final branch for approval and merge.
