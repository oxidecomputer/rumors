# Current triage state

Work proceeds directly on `main`. Resource verification is consolidated through
`193a1474`.

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

## Current increment

Party and Version decoding and canonical-byte adoption live on their domain
types through one `from_canonical` boundary, and writers finish directly into
those types. Adaptation façades are absent where callers can state the operation
directly; algorithm-specific helpers belong to the state that owns their
invariants. Genuine module laws remain detached.

The query filter retains an exact height difference only while its storage is
bounded by the corresponding query bound. Wider comparisons compact shared
accumulators without normalization; each refusal strictly reduces retained
width, so the work is amortized and the comparison loop terminates. Targeted
differential properties cover the accumulator postcondition, alternating
cancellation, exact order, and the per-bound storage condition. The discovered
alternating-cancellation witness is pinned as a proptest regression.

## Next outcome

Consolidate semantic verification. Distinguish the recursive oracle,
function-space oracle, algebraic laws, property tests, and exhaustive small
scope by the failure class each uniquely detects; remove overlapping rosters,
copied helpers, and decorative case-count machinery without losing behavioral
coverage. Re-evaluate priorities after that increment before entering fuzz-fit,
fuelscape, or public-surface work.
