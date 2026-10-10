# Measures of versions

What the numeric measures of a version, and of a pair of versions, mean;
how they relate; and which of their properties are proved, checked, or
conjectured. Part of the [presentation note](README.md); the diff's summary
line is the first consumer.

Each claim carries its status:

- **proved**: the argument is given here (as a sketch, not kernel-checked);
- **probed**: the scratch [probe](probe/) found no counterexample over
  randomized histories, and nothing is committed as a test;
- **conjectured**: neither.

Notation follows the README: positions are binary fractions, heights decimal,
and `Version { 0..0.1 => 3, … }` lists maximal runs.

## Bricks and floors

The ITC paper (§5) reads an event tree as nested *base values*: each node
contributes "a base value, common for the whole interval", and the paper's
graphical notation draws the function "by 'stacking' the corresponding
parts". This note names the unit of that stacking. A *brick* is a block of
height 1 over one node's dyadic interval, so a node whose base is `n` stacks
`n` bricks. (The term is this note's, not the paper's.) In the canonical tree,
writing `m(N)` for the minimum height over node `N`'s interval,
`base(N) = m(N) − m(parent(N))`, with the root's parent at 0, and the version
is the sum of its bricks:

```
v = Σ_N base(N) · 1_N
```

A *floor* is the set of ids at or above a height: floor `k` is `{v ≥ k}`. The
floors determine the version (`v = Σ_k 1_{floor k}`, the layer-cake formula),
and the README's floors notation lists them.

**Bricks are the floors' maximal dyadic pieces (proved).** A dyadic interval
`J` inside floor `k` is either a tree node or lies inside a leaf (any interval
containing a node is a node). If it lies inside a leaf, that leaf is constant
at height at least `k`, so `J` is maximal only if it is the leaf. So the
maximal dyadic intervals of floor `k` are exactly the nodes `N` with
`m(parent(N)) < k ≤ m(N)`, and node `N` is such a piece on exactly `base(N)`
floors. Counting floor by floor and counting bricks agree.

## Single-version measures

| Measure | In floors and bricks | Weighting | Lattice behavior |
|---|---|---|---|
| maximum height | number of floors | none (vertical) | preserves joins |
| minimum height | number of full floors; the root's base | none (vertical) | preserves meets |
| width | measure of floor 1 (the *support*) | by measure | modular (probed) |
| support pieces | maximal dyadic pieces of floor 1 | by count | not examined |
| `rank` | total width of all bricks | by measure | modular (probed) |
| `min_ticks` | number of bricks | by count | submodular, not monotone |

The vertical measures need no weighting. Each horizontal or volume measure
comes in two forms: one weights pieces by their width (and so depends on tree
depth), and one counts them (and does not). **`min_ticks` is `rank` with the
widths forgotten:** `rank = Σ_N base(N) · |N|` and `min_ticks = Σ_N base(N)`.
`min_ticks` equal to the floors' total dyadic piece count is probed over
15,000 versions and follows from the proof above, since `min_ticks`'s
documented formula, `Σ leaf heights − Σ internal-subtree minima`, telescopes
to `Σ_N base(N)`.

**Inequalities (proved):**

- `width ≤ rank ≤ max × width`: every point of the support has height at
  least 1 and at most the maximum.
- `min ≤ rank ≤ max`: the interval has width 1.
- `max ≤ min_ticks`: every floor holds at least one brick.
- `rank ≤ min_ticks`: every brick is at most 1 wide.

The first and third are also probed.

### Scalar time, from above and below

The flat versions `flat(k) = Version { 0..1 => k }`, which are `k` ticks of
the seed, form a chain: scalar, single-writer time. Then (proved):

```
v <= flat(k)   exactly when   max(v) <= k
flat(k) <= v   exactly when   k <= min(v)
```

So the maximum is the left adjoint of the embedding `flat` and the minimum its
right adjoint: they are the best scalar approximations of `v` from above and
below. A left adjoint preserves joins and a right adjoint preserves meets, so
`max(a | b) = max(max a, max b)` and `min(a & b) = min(min a, min b)`. Both
identities are also probed.

### The maximum is causal depth

Every history that produces `v` contains a causal chain of at least `max(v)`
ticks, and some history has no longer chain. So `max(v)` is the least possible
length of the longest causal chain over all histories of `v` (proved, given
the tick law below):

- **Lower bound.** A tick raises a version's global maximum by at most one,
  and a join takes maxima, so along any history the maximum grows by at most
  the length of the longest chain of ticks behind it.
- **Achieved.** Fork the seed into one party per plateau of `v`, tick each
  party up to its plateau's height (all concurrently), and join the results.
  Each party's version is its plateau's height over its own interval and 0
  elsewhere, so the join is `v`, and the longest chain is `max(v)`.

`min_ticks` is the companion bound: the least possible *number* of ticks over
all histories of `v`. The poset-theoretic dual of "least possible longest
chain", the least possible largest antichain, carries no information: any
version can be produced by a totally ordered history. Pass one version around
the parties of `v`'s plateaus in rounds, each round raising every unfinished
plateau by one. A party's tick either grows its plateau by one or fills it to
the same height from a sibling already at that height (conjectured; reasoned
from `fill` and `grow`, not probed).

### Width is breadth

The support is the set of ids that carry any history; width is its measure.
Width says how much of identity space has seen an event, not how many events
there were: one seed tick has width 1. Under static membership with `N`
equal-share parties, width × `N` counts the parties that have ticked. The
support preserves both joins and meets, as union and intersection (proved),
and is the right adjoint of the embedding that sends a set of ids `S` to the
version at height 1 over `S` and 0 elsewhere. Each floor `{v ≥ k}` is likewise
the right adjoint of the embedding at height `k`, which is why the floors
determine the version.

A support or floor is a set of ids with no identity attached. If either is
ever exposed, it needs its own type: returning a `Party` would create
identity.

### `min_ticks` is submodular and not monotone

**Submodular (proved; probed over 60,000 pairs, strict in 8,395, never
violated):**

```
min_ticks(a | b) + min_ticks(a & b) ≤ min_ticks(a) + min_ticks(b)
```

Floor `k` of `a | b` is the union of the floors `k` of `a` and `b`, and of
`a & b` their intersection, so it suffices that `P(S)`, the number of maximal
dyadic pieces of a set `S`, is submodular. Fix a depth `D` finer than every
boundary. A maximal piece at depth `d` contains `2^(D−d)` depth-`D` cells and
`2^(D−d) − 1` coarser dyadic intervals, so `P(S) = A(S) − I(S)`, where `A`
counts depth-`D` cells inside `S` and `I` counts coarser dyadic intervals
inside `S`. `A` is modular. `I` is supermodular, since an interval inside
`S ∪ T` need not lie inside either, so `P` is submodular.

**Not monotone (probed: 479 of 21,194 ordered pairs).**
`Version { 0..0.01 => 1, 0.01..0.1 => 0, 0.1..1 => 1 }` (one tick by a
quarter-share party, one by a half-share party) lies below `flat(1)` (one seed
tick), yet needs two ticks to the flat version's one. A union can merge
sibling pieces into one brick, which a single tick of the party owning both
lays down. So learning more history can make the cheapest explanation
shorter.

This is why `min_ticks` looked like the odd one out. The other single-version
measures are sizes, monotone in the causal order. `min_ticks` is a minimum
description length, in bricks. It also induces no lattice metric:
`min_ticks(a | b) − min_ticks(a & b)` is negative for the pair above.

## Pairwise measures

Write `g = (b − a)⁺`, the part of `b` above `a`. It is a nonnegative dyadic
step function, so it has the same floors and bricks as a version, and every
single-version measure applies to it:

| Measure of `g` | Name | Meaning |
|---|---|---|
| area | `a.lag(&b)` (exists) | width-weighted gap |
| maximum | `a.lag_height(&b)` (candidate) | the largest height difference at any id |
| width of support | `a.lag_width(&b)` (candidate) | how much id space `b` leads on |
| brick count | unnamed | the gap's description length, depth-independent |

`width ≤ lag ≤ height × width` holds again. All of these are zero exactly when
`b <= a`, shrink as `a` absorbs history, and grow as `b` gains it. Height,
width, and area satisfy the triangle inequality (proved, pointwise).

**Depth dependence.** Height and the brick count do not depend on tree
depth. Width and area do: one missed tick on a 2⁻²⁰-wide fragment has width
and area 2⁻²⁰.

**Symmetric forms.** Pointwise, `|a − b| = (a | b) − (a & b)`, so each
symmetric measure is the matching lag measure from the meet to the join.
`distance` is already defined this way. Distance-height is `max |a − b|`, the
Chebyshev distance, equal to the larger of the two lag-heights.
Distance-width is the width of `{a ≠ b}`, equal to the sum of the two
lag-widths, because the sets where each side leads are disjoint. Heights
combine by maximum and widths and areas by sum. Distance-height,
distance-width, and `distance` are metrics. `distance` is the lattice metric
that `rank` induces as a strictly monotone valuation. Width's induced
`width(a | b) − width(a & b)` is only a pseudometric: it vanishes between
distinct versions with the same support. None of the symmetric forms says who
leads; the asymmetric pair does.

## Causal content

Single-version maxima bound causal depth; pairwise gaps measure disagreement
in heights, not events.

**Pairwise height bounds nothing causal (probed example).** A `fill` can copy
a height that `a` already holds elsewhere. Party P forks into quarters, the
right quarter ticks ten times, and P rejoins. A replica `a` that saw those
ticks lacks only P's next tick, which fills the left quarter to 10:

```
a = Version { 0..0.01 => 0, 0.01..0.1 => 10, 0.1..1 => 0 }
b = Version { 0..0.1 => 10, 0.1..1 => 0 }
```

`a.lag_height(&b)` is 10 and the gap holds 10 bricks, yet `b` has one tick
that `a` lacks. Height does count events under static membership, where each
party owns one dyadic interval and only its owner ticks there: height over a
party's region is then that writer's version-vector difference.

**A lower bound on missing ticks (proved, given the tick law).** Let
`missing(a, b)` be the number of ticks in `b`'s history that are not in `a`'s.
It is unbounded above, but lower bounds are finite: from the empty version,
the fewest ticks any history of `v` holds is exactly `min_ticks(v)`. Then:

```
missing(a, b) ≥ max(b) − max(a)
```

By induction over the execution, every version `u` satisfies
`max(u) ≤ max(a) + missing(a, u)`. A tick that `a` has seen produces a version
below `a`. A tick that `a` has not seen raises `missing` by one and the
maximum by at most one. A join takes maxima.

`min_ticks(a | b) − min_ticks(a)` is *not* a bound, because `a`'s real history
may hold more than `min_ticks(a)` ticks. `max(a | b) − max(a)` is at most both
`a.lag_height(&b)` and `missing(a, b)`, and those two are incomparable: the
example above has height 10 and one missing tick, and ten single ticks on ten
separate regions have height 1 and ten missing ticks. Whether the greatest
lower bound on `missing` is computable is open.

## Ticks and the global maximum

- **A tick raises a version's global maximum by at most one** (Finch
  confirms; probed over 16,216 ticks, 7,560 raising it by 0 and 8,656 by 1).
  `fill` raises points only to heights already present, and `grow` adds one.
- **So `ticks(k)` raises it by at most `k`.**
- **`ticks(k)` raises it by at least `k − 1` when the party is a single
  dyadic interval containing a point of maximal height.** The first tick may
  fill that interval up to the maximum, and every later tick grows it by one.
  The seed is such a party (probed: `ticks(3)` on a non-flat version raised it
  by 2). The general condition is reasoned, not probed.
- **For any other party the lower bound is 0** (probed counterexample). A
  party owning `0..0.1` ticks four times from
  `Version { 0..0.1 => 0, 0.1..0.11 => 5, 0.11..1 => 14 }`: the first tick
  fills its half to 5, the minimum of the other half, and the rest grow it
  underneath 14, which never moves.

## Names and uses

- **Names.** The candidates are `lag_height` and `lag_width` beside `lag`.
  They show the pairing and the inequality, and inherit `lag`'s direction:
  `a.lag_height(&b)` is how far `a` sits below `b` at worst. Rejected:
  `deficit` and `extent`, which hide the pairing; `depth`, which collides with
  tree depth; `span`, which collides with the type.
- **Return types.** Height returns `Count`. Width returns `Rank`, following
  `lag` and `distance`, which return `Rank` for areas that are not the rank of
  any version; a dedicated width type is the stricter alternative.
- **Uses.**
  - Progress measures in convergence tests. Height is depth-independent, must
    reach 0, and cannot increase while peers only absorb.
  - Property tests: the inequalities, triangle inequalities, monotonicity,
    and modularity above.
  - The diff's summary line.
  - Bounded-staleness predicates, such as `a.lag_height(&b) <= k`, in the
    spirit of TACT's error bounds (Yu and Vahdat) and Cosmos DB's "K
    versions" bounded staleness. Under static membership this means "no
    writer's ticks are more than `k` ahead of what I have seen". It is an
    application policy with no current consumer in rumors.
  - Choosing an anti-entropy partner by gap width or height. Also an
    application policy with no current consumer.
- **Proposal.** Implement both privately for the diff, and expose
  `lag_height` when a consumer appears. Height is computable today through
  `shape::combine` by summing rises, with the documented quadratic hazard for
  running `Count` sums. Width cannot be built exactly as a `Rank` from the
  public API: besides `ZERO`, the only way to obtain a `Rank` is parsing text.

## Open

- Is the greatest lower bound on `missing(a, b)` computable?
- Is the brick count of `|a − b|` a metric? It is zero only at equality and
  symmetric, but the brick count is not monotone pointwise, so the triangle
  inequality does not follow from the pointwise one.
- Does the support's piece count, the depth-independent breadth measure,
  serve any consumer?

## The probe

[`probe/`](probe/) is a throwaway crate that depends on `before` by path and
reads versions only through the public `shape()` iterator. Heights are
running sums of each plateau's rise in a `u64`, and positions are `u128`
numerators over 2^64, so it asserts depth at most 64. Its renderer prints the
README's notation, merging equal adjacent plateaus. It is not the design:
production rendering carries `Count` heights at any depth and avoids the
quadratic running-sum hazard. Run it from its directory:

- `cargo run`: the `ticks(k)` counterexample and the seed case;
- `cargo run --bin laws`: the tick law, join and meet preservation, the
  width/rank/max inequality, and `min_ticks ≥ max`;
- `cargo run --bin fillgap`: one missing tick with a lag-height of 10;
- `cargo run --bin bricks`: `min_ticks` as the floors' piece count,
  modularity of `rank` and width, submodularity and non-monotonicity of
  `min_ticks`.

Its histories fork up to 8 parties from the seed and mix ticks with joins
from a pool of versions, from a fixed seed.
