# Signed versions and saturating subtraction

Finch's proposal: think of the pairwise measures as measures of a
saturating difference `a.saturating_sub(b)`, and introduce a signed version,
tentatively `Diff`, for `a − b`. This section works out what the two objects
are and how they reorganize [02-measures.md](02-measures.md). Part of the
[presentation note](README.md). Claims marked *proved* carry their argument
here; nothing in this section is probed or tested yet.

## The objects

- **Saturating subtraction.** `a ∸ b` (`a.saturating_sub(b)`) is the
  pointwise `max(a − b, 0)`: the history `a` records beyond `b`, measured
  point by point. It is a nonnegative dyadic step function, so it is a
  `Version`.
- **Signed difference.** `a − b` is a step function from `[0, 1)` to the
  integers: a `Diff`.
- **Positive and negative parts.** `(a − b)⁺ = a ∸ b` and
  `(a − b)⁻ = b ∸ a`. Their meet is the empty version (where one is
  positive the other is zero), and `a − b = (a ∸ b) − (b ∸ a)`. This is the
  "maximally cancelled pair": the decomposition every lattice-ordered group
  has, and the Jordan decomposition of a signed measure.

Pointwise identities (proved, each a one-line case analysis):

```
a | b  =  a + (b ∸ a)  =  b + (a ∸ b)
a & b  =  a − (a ∸ b)  =  b − (b ∸ a)
|a − b|  =  (a ∸ b) | (b ∸ a)  =  (a | b) − (a & b)
a + b  =  (a | b) + (a & b)
```

The third uses a join where a sum would also do, because the two parts never
overlap. The fourth is why `rank` is modular.

**Structure.** Diffs under pointwise addition and pointwise maximum and
minimum form a lattice-ordered abelian group, and versions are its positive
cone: closed under `+`, `|`, `&`, and `∸`, but not `−`. This is the group
completion from the
[monoid-to-group reading](02-measures.md#grounding-the-signed-brick-distance),
made concrete.

**Output size is linear (proved).** `project` can grow multiplicatively, so
a pointwise operation is not automatically safe. For `a ∸ b`:

- A node of its canonical tree is one where it is non-constant, so `a` or
  `b` is non-constant there too. The output's nodes are among the inputs'.
- Each output delta is bounded by the input deltas. Inside a positive
  region it is `δa − δb`. Entering one from zero, the new difference `D'`
  satisfies `D ≤ 0 < D'`, so `D' ≤ D' − D`; leaving is symmetric. This is the
  sign-crossing bound `Integrator::jump` and the lattice kernel already rely
  on.

Projection blows up because entering ownership emits an absolute height,
which no input delta bounds. Nothing in `∸` does that, so it belongs with join
and meet: `O(|a| + |b|)` output.

## What it simplifies

### Pairwise measures are single-version measures of a part

| Measure | of `b ∸ a` | of `\|a − b\|` | signed, of `a − b` |
|---|---|---|---|
| area (`rank`) | `a.lag(&b)` | `a.distance(&b)` | `rank(a) − rank(b)` |
| peak | lag-height | distance-height | — |
| width | lag-width | distance-width | — |
| bricks (`min_ticks`) | lag-bricks | brick count of `\|a − b\|` | — |
| signed brick norm | — | — | brick distance |

The pairwise vocabulary of 02 collapses into one vocabulary of
single-version measures (`rank` and `min_ticks` exist; peak and width are
proposed) applied to three views of one object. Under the rulings below, the
candidate names `lag_height` and `lag_width` become measures of a `Lag`:
`a.lag(&b).peak()` and `a.lag(&b).width()`.

### Difference measures and level measures

Every measure of a `Diff` is unchanged when the same version is added to both
operands: it sees only the disagreement. The skews split along that line.

- **`rank(a) − rank(b)` is a difference measure.** `rank` is linear under
  pointwise addition, so it extends to Diffs as a group homomorphism. The
  signed integral that `rank_cmp` computes is the `Diff`'s rank.
- **Peak skew and `min_ticks` skew are level measures.** The peak and
  `min_ticks` are not additive, so their differences depend on where the
  versions sit, not only on how they disagree.

**Causal bounds are level measures (proved by example).** Take the `Diff`
that is 10 over one quarter and 0 elsewhere. It arises from the fill example
in 02 with one missing tick, and from `empty` against 10 ticks on that
quarter with at least ten. A function of the `Diff` alone that bounded
missing ticks from below could therefore promise at most one. The one causal
bound found so far, `missing(a, b) ≥ max(b) − max(a)`, uses the levels.
Difference measures describe disagreement; causal content needs the levels
too.

### Order is the sign pattern of the difference

`a <= b` exactly when `a ∸ b` is empty. The versions are concurrent exactly
when both parts are nonempty, and equal when the `Diff` is zero.
`OrderState` tracks the emptiness of the two parts, and each early exit stops
at the first nonzero point of a part. Span placement is the sign pattern of
two differences, `v − lo` and `hi − v`, and a span's thickness `hi ∸ lo` is
the version `hi − lo`, since `lo <= hi`.

### Drivers and folds become views and folds

The pair driver's orientation closure (positive part, absolute value, or
signed) chooses a view of the `Diff`. So the pair driver streams a
`Diff` view, and [the drivers-and-folds table](02-measures.md#shared-drivers-separate-folds)
becomes two kinds of stream (a version, or a view of a difference) crossed
with the folds. Emission is a fold too: materializing `a ∸ b` runs a
`VersionWriter` over the positive-part stream, the lattice kernel's
machinery emitting the oriented difference instead of the maximum or
minimum.

### The causal diff display is the `Diff`'s `Debug`

The assertion display designed in the README renders `a − b`. With zero
ranges elided (a version of the base-plus-bumps rule, with base 0) and
explicit signs, so that each arm says who leads:

```
Diff { 0.1..0.101 => +1, 0.11..1 => -1 }    // a − b: a leads on 0.1..0.101, b on 0.11..1
```

Eliding zeros loses nothing: unlisted ranges are 0. The assertion message
shows both versions and their `Diff`, and `Crossed`, now carrying its
versions, can render the same way. The name `Diff`, proposed earlier for the
display type, passes to the algebraic object, and the display becomes its
`Debug`.

### Brick distance, restated

The signed brick distance is the brick norm of the `Diff`. The canonical pair
`(a ∸ b, b ∸ a)` is the decomposition that minimizes area, which is total
variation. A decomposition minimizing brick count may use overlapping parts:
`flat(1)` less a hole is cheapest as one brick added and one subtracted, not
as its positive part's five pieces. So the oddity of counting bricks is
exactly that the `Diff`'s canonical pair is not brick-optimal.

## Rulings

- **`Lag` replaces the bare number** (ruled in direction): `a.lag(&b)`
  returns a lazy view `Lag` denoting `b ∸ a`, the history `b` records beyond
  `a`, and the number is `a.lag(&b).rank()`. Peak, width, and brick count
  become the view's other measures. `saturating_sub` does not appear on
  `Version`: the measurement lives in its own type, apart from states.
- **`Diff`** (ruled): `&a - &b` produces it, with the named form
  `a.diff(&b)`.
- **A signed rank** (ruled in direction): either a separate signed type, or a
  type parameter on `Rank` marking signedness. The crate already has the
  parameterized idiom in `Query<'_, P: Polarity>`. A signed rank needs its
  own order-preserving encoding (for example, negative values with their
  magnitude bits complemented) and a `Display` with a sign.
- **Addition belongs to `Lag` and `Diff`** (ruled): pointwise sums of lags or
  diffs mean something (quantities of disagreement add), while sums of
  versions do not. A version lifts to a lag or diff with the empty version as
  its low end: `Lag` from `v` is `v ∸ empty`, `Diff` from `v` is `v − empty`.
  `Lag + Lag` is a `Lag` (nonnegative functions are closed under addition),
  `Diff + Diff` a `Diff`, and a `Lag` lifts to a `Diff`. `Diff::positive()`
  and `Diff::negative()` return `Lag`s, and `|a − b|` is the `Lag`
  `a.lag(&b) + b.lag(&a)`, so `distance` is that sum's rank.

## Representation

The ruling on addition settles most of it. Hold both operands **by value**:
`Lag { low, high }` denotes `high ∸ low`, and `Diff { plus, minus }` denotes
`plus − minus`. A `Version` clone is a refcount increment, so neither type
needs a lifetime, as with spans by value.

- **Lazy by default.** `a.lag(&b)` is `Lag { low: a, high: b }`;
  `&a - &b` is `Diff { plus: a, minus: b }`. Measures and `Debug` stream
  through the pair driver; nothing is materialized.
- **Materialized through the lift.** A materialized lag is
  `Lag { low: empty, high: b ∸ a }`, the same struct. The canonical form of a
  `Diff` is the maximally cancelled pair `(a ∸ b, b ∸ a)`, again the same
  struct. One representation covers both, and the only difference is
  whether the pair is canonical.
- **Addition materializes.** `Diff + Diff` can add componentwise,
  `(a₁ + a₂) − (b₁ + b₂)`, using an internal pointwise-sum kernel on
  versions that is never exposed. Saturation does not distribute, so
  `Lag + Lag` must canonicalize each term first: `(empty, (b₁ ∸ a₁) + (b₂ ∸ a₂))`.
  The pointwise sum is linear in output size, by the same argument as `∸`:
  its nodes are among the inputs' and its deltas are sums of input deltas.
- **Equality and hashing** compare canonical forms, so two `Diff`s built
  from different operands but denoting the same function are equal. Equality
  walks; hashing must canonicalize, or `Diff` should not be `Hash`.
- A dedicated signed encoding remains possible later, if a `Diff` must be
  stored or sent and the canonical pair's overhead is measured to matter.

## Which walks the `Diff` view absorbs

The question: how much of the crate's custom walking (inventoried in
[the lockstep walk audit](../2026-10-10-before-lockstep-walk-audit/README.md))
could be rewritten as traversals of the lazy `Diff` view, with no loss of
performance? The constraint behind every verdict below: a traversal must not
hand out absolute heights or differences per region. Copying a wide height
at every narrow boundary is the quadratic pattern the crate avoids, so the
traversal folds over deltas and borrowed accumulator state, as
`advance_diff` already does. A public iterator over a `Diff` would yield
rises, as `shape` does for versions.

**Absorbed as is.** These already traverse `a − b` in all but name, through
`OpenedPair` and `advance_diff`, so naming the abstraction costs nothing:

| Walk | As a `Diff` traversal |
|---|---|
| order comparison (`order.rs`) | fold each region's sign into the emptiness of the two parts, stopping early |
| join and meet (`lattice.rs`) | fold the sign together with each side's own step, emitting through `VersionWriter` |
| `lag`, `distance`, rank comparison (`measure/integral/pair.rs`) | fold an orientation of the difference (which part or view) into `Integrator` |

**Absorbed through views of views, if the masking moves into the stream.**
Lazy projected comparison (`projection.rs`'s `Comparison`) is the `Diff` of
two masked operands, `(a / p) − (b / q)`. Its speed comes from `block_skip`:
inside an unowned region, it folds a version's deeper regions in bulk
instead of stepping each through the four-slot overlay. If a masked operand
is itself a stream whose unowned regions arrive as one coarse region (the
version's subregions consumed inside the stream, as materialization already
does), the comparison becomes an ordinary `Diff` traversal over two such
streams. The bulk skip is preserved, only relocated. Parity is plausible
but not shown: today the skip interleaves with the slot priority loop, and
moving it changes the walk's structure, so this is one for the fuelscape
boards. Projection materialization is not a `Diff`. It is the writer fold
over one masked stream, and the same relocation applies.

**Not a single `Diff`: one probe against many bounds.** Span placement
(`place.rs`) wants the signs of `v − lo` and `hi − v`, and the query filter
(`place/filter.rs`, `MemberCursors` and `SpanCursors`) the signs of `k`
differences against one probe. Running independent `Diff` traversals would
decode the probe once per bound, which is exactly what both were built to
avoid. What they share is a *fan-out* traversal: one probe, many bounds, the
probe's deltas folded into every live difference, and bounds dropped once
decided. The filter adds a memory policy that keeps absolute heights and
discards a bound's private difference when the probe grows much wider. Place
and filter could share one fan-out driver; folding them into the single-pair
`Diff` would cost a decode per bound.

**Not differences at all.** The shape iterators refine `N` streams into one
partition of rises (`shape::combine`, `Clock::shape`); they are many-way
overlays, not differences. `tick` is a tree walk shaped by `fill` and
`grow`. Both stay as they are.

**Party operations are `Lag` and `Diff` operations on 0/1 functions, but
their walks should stay.** A party is the indicator function of its region,
so:

- `p.is_disjoint(&q)` holds exactly when `p & q` is empty;
- `p.covers(&q)` holds exactly when `q ∸ p` is empty;
- `p.without(q)` is `p ∸ q`;
- `p.join(q)` is `p + q`, defined exactly when the parts are disjoint (where
  the sum equals the join).

That correspondence belongs in their docs. Their implementations should
stay structural. They settle whole subtrees at once without per-region work,
and [the audit's party finding](../2026-10-10-before-lockstep-walk-audit/README.md)
shows a region walk would make `join`'s constants worse. The correspondence
is semantic, not a reason to share the walk.

**Tally.** Of the overlay walks, three consumers (order, the lattice, the
pair integrals) are `Diff` traversals today. Projected comparison and
materialization can become views of views, with parity to be measured.
Placement and the two filter walks share a fan-out driver instead. The shape
iterators, `tick`, and the party operations are not differences. So the
`Diff` view absorbs the hottest paths at no cost. It absorbs projection
probably at no cost. And it clarifies the rest without absorbing them.

## Open questions

- **A separate signed type, or a parameter on `Rank`?**
- **Hashing a `Diff`.** Equality compares the denoted functions, so hashing
  must canonicalize first, or `Diff` should not implement `Hash`.
- **`distance`.** It could stay as a fused method, or become the rank of
  `a.lag(&b) + b.lag(&a)` (or of `(&a - &b).abs()`).
- **The fan-out driver.** Whether place and filter consolidate onto one is a
  question for a follow-up audit.
- The universe rule applies unchanged: a `Lag` or `Diff` of versions from
  different seeds is meaningless.
