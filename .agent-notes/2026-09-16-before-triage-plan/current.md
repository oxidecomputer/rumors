# Current triage state

Work proceeds directly on `main`. The completed foundation, fuzz-fit
consolidation, and approved API run through `e5fb65c1`.

Four uncommitted consolidation diffs are isolated for separate owner review:

- `codex/before-fuzz-consolidation` owns libFuzzer and seed replay.
- `codex/before-wasm32-consolidation` owns the direct 32-bit suite.
- `codex/before-gate-ci-consolidation` owns shared gate and CI orchestration.
- `codex/before-deps-assets-consolidation` owns dependency and generated-asset
  pruning after the other three settle.

No consolidation branch commits before explicit owner approval. Documentation
and final finding reconciliation continue independently on `main`.

The uncommitted `main` increment closes the remaining public contract wording
and clarifies the `Span` implementation and tests. Join/meet encoding-size
coverage lives in the main lattice differential, where oracle and adversarial
families check the exact bit-length bound alongside value and canonicality.

## Completed foundation

Party and Version algorithms now operate through their domain readers, writers,
and stateful walks rather than raw bit storage. Public operations remain in
their owning type's rustdoc order, while substantial traversal state lives in
focused private modules. The codec primitives have one standard reader/writer
vocabulary, and the Party, Version, Span, and minima implementations use
domain operations rather than representation-level trampolines.

Suanpan now exposes standard arithmetic traits and a small set of explicit
accumulation operations. Its internals are split by responsibility, and one
surface-complete differential proptest checks every public operation against a
big-integer oracle across representation and arithmetic boundaries. Before
moves accumulator values directly between algorithms and normalizes them only
at true arbitrary-precision output or retention boundaries.

The public complexity and allocation audit is closed. The method-level false
claims, direct amplification-board gaps, numeric fuel gaps, balanced-fold
premise, and crate-headline disposition have each been resolved or assigned to
the owner's deferred crate-page rewrite.

Semantic verification now has one shared operation vocabulary across the
production implementation, recursive oracle, and function-space oracle.
Sampled and exhaustive schedules use the same differential driver, while
algebraic laws and focused deep-traversal tests retain their distinct roles.
The compiler-derived surface check directly holds the resource tables against
the public API, replacing copied source rosters and scanners. Shared generators
now construct nonempty and nonzero inputs directly, reuse the conditional
byte-boundary strategies, and exercise fold carry boundaries without parallel
per-test population builders.

Party and Version decoding and canonical-byte adoption live on their domain
types through one `from_canonical` boundary, and writers finish directly into
those types. The query filter bounds retained height differences by their query
bounds; targeted differential properties cover the accumulator postcondition,
alternating cancellation, exact order, and the per-bound storage condition.

The approved public surface is recorded in
[`api-proposal.md`](api-proposal.md). Its core-type, text, and serialization
increments are complete.

## Remaining work

1. **Settle the remaining verification infrastructure.** Simplify fuzz replay,
   the direct 32-bit suite, gate and CI derivation, dependencies, and generated
   assets while preserving each distinct signal.
2. **Finish the documentation.** Rewrite the crate page and public item docs at
   caller altitude; correct stale guideposts; finish the maintainer-prose sweep.
3. **Reconcile and verify.** Re-read every original finding against the final
   tree, run the justified full verification set, and present the integrated
   result for approval.

Re-evaluate this order after every approved commit.
