# Current triage state

Work proceeds directly on `main`. The completed foundation, approved API,
contract cleanup, and verification consolidation run through `0e5ec518`.
The production-documentation increment is active and uncommitted.

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

1. **Owner crate-page rewrite.** The public items, guideposts, and maintainer
   prose are triaged; the owner has reserved the crate page for a later rewrite.
2. **Reconcile and verify.** Re-read every original finding against the final
   tree, run the justified full verification set, and present the integrated
   result for approval.

Re-evaluate this order after every approved commit.
