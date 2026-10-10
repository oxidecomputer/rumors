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
proposed) applied to three views of one object. The candidate names
`lag_height` and `lag_width` become compositions: `b.saturating_sub(&a)`
followed by the peak or the width. `lag` and `distance` remain as fused,
allocation-free paths whose docs can say what they compose.

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

## Representations

Three options, which are interconvertible in linear time:

1. **A lazy view over `(a, b)`**, as `OwnVersion` is for projections.
   Construction is `O(1)`; measures and `Debug` stream through the pair
   driver without materializing anything; equality walks. It serves every
   use in these notes and needs no new codec. By the views ruling, its
   `Debug` renders the signed function it denotes.
2. **The canonical pair `(a ∸ b, b ∸ a)`**, with the invariant that the two
   parts' meet is empty. It reuses `Version`'s codec, `Debug`, and measures,
   at the linear materialization cost shown above. It is the natural form for
   a `Diff` that must outlive its operands, or be stored or sent.
3. **A signed stream.** The skyline encoding already stores signed deltas;
   only its opening height is unsigned. A `Diff` encoding would be the
   `Version` encoding with a signed opening payload and its own validation:
   the smallest single representation, and a new format.

Recommendation: start with the lazy view, materialize to the canonical pair
when a `Diff` must outlive its operands, and consider the signed stream only
if the pair's overhead is measured to matter.

## Open questions

- **Should `saturating_sub` live on `Version`?** `a ∸ b` is a measurement,
  not a state any replica holds. As a `Version` it can be compared, joined,
  or ticked, which type-checks and means nothing. Keeping it behind the
  `Diff` (`(&a - &b).positive()`, returning a view type) keeps measurements
  and states in separate types, at some cost in convenience.
- **The operator.** `&a - &b` producing a `Diff`, with a named form such as
  `a.diff(&b)`.
- **The signed rank's type.** `Rank` is nonnegative; a `Diff`'s rank needs a
  sign, either as an `(Ordering, Rank)` pair or a signed type.
- **Pointwise addition of versions** appears in the identities but has no
  causal meaning (a join, not a sum, combines histories). The identities can
  stay in docs and tests without exposing it.
- The universe rule applies unchanged: a `Diff` of versions from different
  seeds is meaningless.
