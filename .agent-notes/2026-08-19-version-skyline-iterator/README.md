# The shape walk

[`version-skyline-iterator.md`](./version-skyline-iterator.md) is the design
document and decision record for `before`'s public shape API:
`Version::shape()`, `Party::shape()`, and `Clock::shape()`, which expose each
type's step function over the unit id interval as allocation-honest iterators
walked in place off the canonical stream. It settles the item vocabulary —
`Plateau` with `rise: Option<Rise>` and an explicit per-leaf `depth` for
versions; the absolute `Region { owned, depth }` for parties, making the
magnitude-1 invariant unrepresentable rather than policed; `(Plateau, bool)`
overlay items for clocks, well-formed because dyadic intervals form a laminar
family — plus the base-2^64 limb spelling of `Ticks`, the version-only
const-generic N-way combiner taking `[&Version; N]` directly, and the
public-door differential testing plan (roundtrip, totality, join
homomorphism, comparison concordance). The title's *skyline* survives only in
this filename: one of the document's own rulings keeps that term
maintainer-facing, with *shape* and *plateau* as the public vocabulary.

Retired as implemented: the API shipped in the shape-walk merge (`Plateau`,
`Rise`, `Region`, and the three `shape()` methods live in
`crates/before/src/shape.rs` and its callers). The one deliberately
unimplemented section remains so by its own ruling — the magnitude
genericization behind a sealed trait is deferred until a kernel
re-expression is actually constructed, candidate-by-candidate under
construct-and-measure, and the document's assessment of which kernels would
resist (the merges' equal-span skipping, `fill`'s surgical splice) is the
standing reference for that day. The body is byte-identical to the
document's last revision at `design/version-skyline-iterator.md`.
