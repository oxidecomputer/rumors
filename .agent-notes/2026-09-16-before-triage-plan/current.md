# Current triage state

Work proceeds directly on `main`. The completed foundation runs through
`2bce9edb`.

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

Party and Version decoding and canonical-byte adoption live on their domain
types through one `from_canonical` boundary, and writers finish directly into
those types. The query filter bounds retained height differences by their query
bounds; targeted differential properties cover the accumulator postcondition,
alternating cancellation, exact order, and the per-bound storage condition.

## Remaining work

1. **Consolidate semantic verification.** Give the recursive oracle,
   function-space oracle, algebraic laws, sampled differentials, exhaustive
   small scope, and deep traversal tests distinct jobs. Consolidate the Party,
   Clock, Version, Span, Rank, projection, and query properties without losing
   a failure class. Then retire redundant operation registries, surface scans,
   copied drivers, and decorative generator controls.
2. **Settle the remaining verification infrastructure.** Reassess fuzz-fit and
   fuelscape independently, then simplify fuzz replay, the direct 32-bit suite,
   gate and CI derivation, dependencies, and generated assets while preserving
   each distinct signal.
3. **Review the public surface and contracts.** Present the API changes as one
   coherent owner-reviewed proposal. Apply every approved external change to
   Before and Rumors together, and put the surviving value, size, and cost
   guarantees on the methods that own them.
4. **Finish the documentation.** Rewrite the crate page and public item docs at
   caller altitude; correct stale guideposts; finish the maintainer-prose sweep.
5. **Reconcile and verify.** Re-read every original finding against the final
   tree, run the justified full verification set, and present the integrated
   result for approval.

The next implementation outcome is the first category. Re-evaluate this order
after every approved commit.
