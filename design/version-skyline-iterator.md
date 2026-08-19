# A public skyline walk for `Version` and `Party`

Status: proposal, pre-implementation. Owner: Finch. Origin: design
conversation, 2026-08-19.

## Goal

Expose the semantic content of a `Version` — its skyline, the step function
over the unit id interval — as a public, allocation-honest iterator, so that
renderers (before-viz, external tooling, debuggers) can draw or analyze a
version without private access to the stream coding, and so the crate gains
an independent public witness of its own comparison and join semantics.

## Grounding

The skyline module's own documentation states the semantic model this API
promotes: a `Version` is a step function over the unit id interval, and each
maximal constant run is a *plateau* — one leaf of the tree, spanning a dyadic
interval of width `2^-depth`. The stored form is already exactly this
content: preorder topology bits (widths) interleaved with zigzag-gamma leaf
height deltas (rises). The iterator is therefore a *transliteration of the
canonical stream*, walkable in place off `&Version` with the existing cursor
machinery: no materialization, no allocation beyond what the item type
itself carries.

## Item shape

One item per leaf, delta-encoded (absolute heights would cost the consumer
nothing to reconstruct and would cost the iterator its asymptotic
optimality — the walk must stay linear in the encoded size, not in the
magnitudes):

```rust
/// One plateau of the skyline: the height change entering it, and the
/// dyadic interval it spans.
pub struct Plateau {
    /// Height change entering this plateau; `None` continues level.
    ///
    /// The first plateau's rise is the skyline's initial absolute height
    /// (`None` if it starts at 0): the walk begins at height 0 on the
    /// interval's left edge.
    pub rise: Option<Rise>,
    /// The plateau spans a dyadic interval of width `2^-depth`.
    pub depth: u64,
}

/// A nonzero vertical move, sign in the variant, magnitude in the payload.
pub enum Rise {
    Up(Ticks),
    Down(Ticks),
}
```

Design forces behind each field:

- **`rise: Option<Rise>` rather than `(direction, ticks)`.** Canonical
  skylines contain consecutive equal-height plateaus (a zero delta between
  non-sibling leaves is a real, canonical shape — the skyline module doc
  says so explicitly), so level steps occur. A flat `(direction, ticks)`
  pair spells the level step two ways (`Up(0)`/`Down(0)`), a canonicality
  leak every downstream comparison would have to normalize forever. The
  sum type gives one spelling per step and states the "magnitude nonzero
  inside `Some`" invariant once, at `Rise`, instead of policing it at
  every consumer.
- **`Ticks`, not `u64`, as the magnitude.** Heights are event counts and
  `Ticks` is arbitrary-magnitude (its own rustdoc exercises a value past
  2^128); a `u64` here would be a structural cap of exactly the kind the
  32-bit correctness program exists to delete. `Ticks` keeps single-word
  magnitudes inline, so the common case allocates nothing and the wide
  case pays a cost that scales with the input's own extravagance.
- **`depth` is explicit and cannot be derived.** An earlier sketch derived
  the horizontal granularity from a running total of the vertical moves.
  That conflates the two independent streams of the coding: rises are
  height-denominated (payload stream), widths are depth-denominated
  (topology stream). Counterexample: a version whose left half sits at
  height `h` and right half at 0 has a first plateau of width `1/2`
  (depth 1) while the running rise total is `h`; deriving the exponent
  from the total yields granularity `2^-h`, which for `h = 2^40` demands
  on the order of `2^(2^40)` horizontal steps. Widths travel explicitly.
  A related overflow rules out "steps at the current finest granularity"
  vocabularies: expressing a width-`1/2` plateau at a deep cousin's
  `2^-100` granularity needs `2^99` steps. Per-leaf explicit `depth`
  keeps every number small and honest.
- **The name `depth`.** The crate's public prose already speaks it
  ("spanning a dyadic interval of width `2^-depth`"); any other name
  mints a synonym for an established term. Runner-up candidates recorded
  for the decision: `scale` (the fixed-point/dyadic term), `halvings`.

## Escaping the backend: `Ticks` as u64 limbs

Consumers must not need big-integer arithmetic hardcoded to this crate's
backend. `Ticks` gains a limb iterator — a base-2^64 spelling:

- Little-endian limb order (least significant first): the order
  reconstruction consumes, and the streaming-friendly one.
- Canonical: no trailing zero limbs; zero yields the empty iterator
  (unreachable through `Rise`, whose magnitudes are nonzero, but the
  method is general `Ticks` API).
- `ExactSizeIterator + FusedIterator`: the limb count is known up front,
  so consumers can preallocate.
- Paired with `TryFrom<&Ticks> for u64` so the overwhelmingly common
  small case never touches limbs.

No backend type reaches the public surface.

## Contract decisions

Settled by this design:

- One item per leaf; delta-encoded rises; explicit per-item `depth`
  (named `depth`).
- `rise: Option<Rise>` — one spelling per step, nonzero invariant at
  `Rise`.
- The walk starts at height 0 at the interval's left edge; the first
  plateau's rise carries its absolute height.
- `Up(0)`/`Down(0)` are unrepresentable (nonzero invariant on `Rise`).
- No closing move after the last plateau: the items remain a pure leaf
  transliteration, and the roundtrip property stays exact.
- No level-run merging in the primitive iterator (merged runs generally
  have no single dyadic width); a rendering adapter may merge, and the
  `Party` case (0/1-valued, long runs) is where that adapter earns its
  keep.
- The iterator borrows `&Version`, walks the canonical stream in place,
  and implements `FusedIterator`. (Not `ExactSizeIterator`: the leaf
  count is not known without a scan.)

Open, wanting a ruling before implementation:

- The method names (`Version::skyline()`, `Party::skyline()`), and the
  item type names (`Plateau`/`Rise`/`Owned`/`Cell` are placeholders).
- The combiner's arity form: const-generic array, runtime `Vec`, or both
  (see "The N-way combiner").
- Whether a projected-stream variant (the `masked` machinery's overlays)
  should be anticipated in the naming now, even if it lands later.

## `Party`

A `Party` is the 0/1-valued skyline over the same interval: identical item
type, magnitudes always 1. The item vocabulary is designed once and shared.

## `Clock`: the combined overlay walk

A `Clock` walks as one iterator rather than a compose-it-yourself pair —
writing the composition as a consumer is nontrivial, and the overlay is
what before-viz actually draws. The item is the version's plateau plus an
ownership flag:

```rust
pub struct Owned {
    pub plateau: Plateau,
    /// Whether the clock's party owns this plateau's interval.
    pub owned: bool,
}
```

(the name is a placeholder; a named struct rather than a bare tuple, so
the flag's meaning has a documentation home). When the party subdivides a
version plateau, the iterator splits it internally.

The splitting is well-formed for free, because **dyadic intervals form a
laminar family**: two of them either nest or are disjoint, never partially
overlapping. Every cell of the common refinement is therefore itself a
dyadic interval — `depth` on a split fragment is simply the deeper of the
two sides, and no new width vocabulary is needed. Conventions: the first
fragment of a split carries the version plateau's rise; subsequent
fragments continue level (`rise: None`). The overlay stream is a
*refinement* of the version's stream, not a transliteration — the
roundtrip property lives on the primitive iterators, never on overlays.

## The N-way combiner

The generalization (recorded as a companion option, and the natural
implementation substrate for the `Clock` walk): a combiner over an
arbitrary number of plateau iterators — parties or versions — yielding the
coarsest common refinement. Since every entry in a cell shares the cell's
interval, per-entry depths are redundant; the item is the cell plus the
rises entering it:

```rust
pub struct Cell<M> {
    pub depth: u64,
    /// One entry per input, in argument order: the rise entering this
    /// cell from that input (`None` continues level, including on every
    /// fragment a split produced after its first).
    pub rises: /* [Option<Rise<M>>; N] or Vec<Option<Rise<M>>> */,
}
```

This stays in the delta domain end to end (no height is materialized),
and the `Clock` walk is its two-input instance with the party's entry
folded into a running `owned` flag. Open: the arity form — const-generic
arrays (allocation-free, `N` static) versus runtime `Vec` (one allocation
per cell, or an internal-buffer visitor variant); possibly both, with the
array form primitive.

This combiner is the public, rendering-grade counterpart of the `overlay`
machinery's tiling-and-advance law — the same subdivision the internal
extremum walks perform. It is deliberately *not* a replacement for them:
as the assessment below records, the internal merges keep byte- and
slice-oriented fast paths (equal-span skipping in sub-leaf time) that a
per-cell iterator inherently forfeits.

## Testing plan

In the house differential style, all through the public door:

- **Roundtrip**: canonical stream ↔ item sequence, bijective (proptest
  against the recursive oracle).
- **Totality**: widths sum to exactly 1 (`Σ 2^-depth_i = 1`).
- **Naturality**: the running height never goes negative.
- **Join homomorphism**: merge-join two versions' item streams by dyadic
  interval, take the pointwise max, and compare against `join`'s items —
  an independent public-door differential witness of the `emit` kernel.
- **Comparison concordance**: `a ≤ b` iff pointwise `≤` over the merged
  streams — the same witness for `sweep`.
- Limb iterator: roundtrip against `Ticks` parsing/display; canonicality
  (no trailing zeros); `TryFrom` agreement on the small range.
- Overlay/combiner: refinement totality (cell widths sum to exactly 1);
  coarsest-common (every cell boundary is some input's plateau boundary —
  no cell is splittable without crossing one); the `Clock` walk agrees
  with independently walking its party and version and projecting the
  flag.

## API stability notes

- `Rise` is closed by construction (a step function steps up or down);
  by the crate's non-exhaustiveness criterion it stays exhaustive, and
  renderers match it exhaustively.
- *Skyline* and *plateau* are today maintainer-facing terms; the public
  rustdoc defines them at the method, promoting the skyline module doc's
  existing definitions.

## Could a generic iterator become the internal substrate?

The question (owner, 2026-08-19): the only thing stopping the internals'
walks and folds from being re-expressed over this iterator is that it
yields `Ticks` — which forces normalization and exposes the pathological
normalization cliffs the accumulator currency exists to sidestep. If the
item were generic over its magnitude type, constrained only by what the
iterator needs to construct one, could the crate's internals radically
simplify?

**Assessment: only partially — but the partial is worth having, and the
genericization itself is cheap and should ship with the feature.**

### The contract

The iterator constructs magnitudes from exactly one thing: a sign and a
little-endian limb window decoded from the zigzag-gamma payload. So the
generic bound is one small trait — constructible from a signed limb
window, nothing more. `Ticks` satisfies it by normalized construction;
the accumulator currency satisfies it by deferred carries and never pays
a cliff. Sealed, with only the `Ticks` instantiation public: the public
surface stays exactly this document's API while internal consumers
instantiate the accumulator. Crucially, the items are *already*
delta-encoded, so a generic consumer folds in the delta domain and never
materializes an absolute height unless its own semantics demand one.

### What re-expresses cleanly (fixed sign, simplification likely)

- **`text` and validation**: pure consumers; the iterator is a decoder,
  so canonicality checking rides the walk it already does.
- **The linear functionals in `query`** (rank, distance, lag): a weighted
  sum over plateaus reassociates (Abel summation) into a pure
  delta-domain fold — each rise multiplied by the width remaining to its
  right — which is exactly the shifted-limb streaming the accumulator
  entries provide. No absolute height is ever materialized.
- **New consumers** (before-viz, external tooling, the differential
  witnesses in the testing plan): the substrate from day one.

### What re-expresses only with care (sign workload-dependent)

- **The verdict merges** (`sweep`, `place`, `masked`): expressible in
  principle as a merge combinator over two item streams plus a
  difference-sign fold in the accumulator currency (projection becomes a
  lazy adapter zeroing masked tiles). Two stream-level optimizations must
  survive the translation, and the second is the hard one:
  word-parallel unary descent reads (an iterator can keep these
  internally), and **equal-span skipping** — the identity fast paths,
  pinned with exact zero-walk counts, prove the merges today skip
  byte-identical spans in sub-leaf time, and the mostly-equal case is
  reconciliation's hot path. A plateau iterator is inherently per-leaf;
  giving it span-identity skip hints re-grows the overlay machinery under
  a new name, at which point the simplification has dissolved itself.
  Verdict: candidate-by-candidate, construct-and-measure, never a
  campaign on anticipated simplicity.
- **`emit`** (join/meet): follows the merges if they go, plus the
  collapsing-builder coupling on the output side.

### What resists

- **`fill`/`grow`** (event registration): a localized mutation — the
  fused walk decides, then a splice rebuilds one root-to-leaf path at
  known bit positions. The semantic iterator deliberately abstracts away
  the positions the splice needs; re-expressing fill over it forfeits the
  surgical splice for a whole-stream rebuild. Stays on the stream.
- **The metering estate**: every kernel re-expressed re-homes its meters,
  envelopes, and pins — a re-measurement and attribution campaign of
  program scale. Not a correctness objection (the instruments make the
  rewrite safe), but a cost any per-kernel decision must price in.

### Recommendation

Genericize the item now (the sealed one-trait bound above), because it is
nearly free and it is the door everything else walks through. Then treat
each existing kernel as its own dissolution candidate under the standing
governor — adopt where a construction measures neutral-or-better and
reads simpler; the likely first adoptions are `text`, validation, and the
`query` functionals; the likely never is `fill`'s splice and any merge
whose equal-span skipping would be forfeit.

## Sequencing

After the wave-2 branches merge: the iterator lives on the same
`sweep`/`query`/`walk` surfaces the 32-bit program is finishing, and it
inherits the u64-clean depth denomination from that work for free.

## Decision record

- 2026-08-19 (Finch): delta-encoded plateau items, `Ticks` magnitudes,
  and a backend-free u64-limb spelling of `Ticks` — ruled in
  conversation.
- 2026-08-19 (Finch): `rise: Option<Rise>` over a flat direction/magnitude
  pair; the field name `depth`; no closing move; no merging in the
  primitive iterator — recommendations accepted as ruled.
- 2026-08-19: granularity-from-rise-totals rejected (independence of the
  topology and payload streams; counterexample above).
- 2026-08-19 (Finch + Claude): the item genericizes over its magnitude
  currency behind a sealed trait (assessment above); internal kernel
  re-expression proceeds candidate-by-candidate under construct-and-
  measure, never as a campaign.
- 2026-08-19 (Finch): `Clock` gets the combined overlay walk — items are
  the version's plateaus with a party-ownership flag, splitting performed
  internally. The N-way combiner is recorded as a companion option
  (and/or); its arity form is open.
