# Current triage state

Work proceeds directly on `main`. The production-structure and exact-
accumulation work is integrated through `fdd1bf47`.

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

## Ready for review

Resource verification is consolidated around the amplification board. Every
adversarial family supplies valid operands to every compatible public operation;
correlated pairs, masks, and populations remain intact. The separate resource
suite now contains only independent arguments, paired marginals, exact early
exits, and the densification observation that the board cannot express. Dead
stack and branch-traffic counters and the duplicated harness are gone.

The board preserves independent operand-count and operand-size axes for
staggered folds, a fixed-work bound for masked comparison through an unowned
region, and a one-pass ceiling for read-only Party comparisons. A focused heap
check retains the distinct guarantee that joining separately stored equal
versions does not copy either operand.

## Next outcome

Consolidate semantic verification. Distinguish the recursive oracle,
function-space oracle, algebraic laws, property tests, and exhaustive small
scope by the failure class each uniquely detects; remove overlapping rosters,
copied helpers, and decorative case-count machinery without losing behavioral
coverage. Re-evaluate priorities after that increment before entering fuzz-fit,
fuelscape, or public-surface work.
