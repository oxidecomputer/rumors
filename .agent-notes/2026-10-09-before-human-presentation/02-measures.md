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

A function `f` on a lattice is *modular* when `f(a | b) + f(a & b) =
f(a) + f(b)`, the inclusion–exclusion identity `|A ∪ B| + |A ∩ B| =
|A| + |B|` of set sizes, and *submodular* when the left side is at most the
right. Read for `min_ticks`: the cheapest explanation of the merged history,
plus the cheapest explanation of the shared history, never costs more than
explaining each version separately. The saving comes from bricks fusing. With
`a = Version { 0..0.1 => 1, 0.1..1 => 0 }` (one tick by the left half) and
`b = Version { 0..0.1 => 0, 0.1..1 => 1 }` (one by the right half), the join
is one seed tick and the meet is empty, so `1 + 0 < 1 + 1`. Joint entropy is
the familiar submodular function: for sets `A` and `B` of random variables,
`H(A ∪ B) + H(A ∩ B) ≤ H(A) + H(B)`. `min_ticks`, a description length,
behaves the same way.

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
description length, in bricks. It also induces no lattice metric. `distance`
is `rank(a | b) − rank(a & b)`, a metric because `rank` is a strictly
monotone modular function; the same construction from `min_ticks` is
negative for the pair above, and so is the entropy-style
`2 f(a | b) − f(a) − f(b)` (variation of information), which needs a monotone
`f`.

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

### Counted pairwise measures

The brick count of the gap, `lag_bricks(a, b) = min_ticks((b − a)⁺)`, is the
counted counterpart of `lag`: the fewest ticks that would build the gap on top
of nothing. It is zero exactly when `b <= a` and does not depend on depth, but
it lacks the properties that make `lag_height` and `lag_width` useful:

- **It can grow as `a` learns.** Take `b = flat(1)`. Against the empty
  version the gap is one brick; after `a` learns
  `Version { 0..0.01 => 0, 0.01..0.1 => 1, 0.1..1 => 0 }`, a middle quarter
  that `b`'s history can contain, the gap
  `Version { 0..0.01 => 1, 0.01..0.1 => 0, 0.1..1 => 1 }` is two bricks.
- **It breaks the triangle inequality (probed).** With `c` equal to
  `flat(1)` less a depth-5 hole, `lag_bricks(empty, c)` is 5, while
  `lag_bricks(empty, flat(1))` is 1 and `lag_bricks(flat(1), c)` is 0. The
  same three versions break it for the symmetric brick count of `|a − b|`:
  5 against 1 + 1.
- **It is not causal**, as `lag_height` is not: the one-missing-tick example
  below has a 10-brick gap.

**A depth-independent distance does exist, from signed bricks.** Let
`‖f‖_B` be the fewest unit bricks, each added or subtracted, that sum to a
signed step function `f`. Concatenating decompositions gives
`‖f + g‖_B ≤ ‖f‖_B + ‖g‖_B`, and `‖−f‖_B = ‖f‖_B`, so
`brick_distance(a, b) = ‖a − b‖_B` is a metric (proved): `a − c` is exactly
`(a − b) + (b − c)`. It is the counted counterpart of `distance` in a precise
sense. `distance` is the least total *width* of a signed decomposition of
`a − b`, because width is additive and the leaf decomposition attains
`∫ |a − b|`; `brick_distance` is the least *count*. Signs matter: `c` above
has 5 positive bricks but `‖c‖_B = 2` (`flat(1)` less the hole).

**What a signed brick means.** A positive brick raises one dyadic interval by
one, and a negative brick lowers it by one. A signed decomposition of
`a − b` is then the shortest recipe of such strokes turning `b`'s skyline
into `a`'s. Its advantage over positive bricks is exceptions. "Raise
everything by one, except this hole" takes two strokes, while listing only
raises takes one per maximal piece around the hole: 5 for a depth-5 hole.
Floor by floor, positive bricks list a floor's maximal pieces, and signed
bricks may write it as a coarse piece minus holes. Routing tables work the
same way. A default route plus one more-specific override is two entries;
the same table without overrides needs every aggregate around the
override's prefix.

So `brick_distance` is the length of the shortest description of the
difference in a language with overrides. Because no operation lowers
heights, a negative brick never corresponds to a single event; the readings
in [Grounding the signed brick distance](#grounding-the-signed-brick-distance)
recover meaning anyway. Signed quantities are not meaningless: a signed difference compares two states, as clock skew
compares two clocks that never run backward. `a − b` is positive where `a`
leads and negative where `b` leads, and `brick_distance` measures how
complex that pattern of leads is to describe. Individual bricks still do not
mean "who leads here": in "+1 everywhere, −1 in the hole", the hole is where
the two versions are level. The plausible consumer is sizing a compact delta
encoding between versions, which nothing needs today.

### Grounding the signed brick distance

The running example has eight cells and a hole in the first:

```
c = Version { 0..0.001 => 0, 0.001..1 => 1 }      // height 1 everywhere except the first eighth
e = Version { 0..1 => 0 }                          // the empty version
```

Built from added bricks only, `c` takes three: `0.001..0.01`, `0.01..0.1`,
and `0.1..1`. With subtraction allowed it takes two: all of `0..1`, minus the
hole `0..0.001`. So `brick_distance(c, e) = 2`.

**What the sign means: a negative brick is the other replica's raise.**
Call a *raise* an event that lifts one dyadic block by one; a brick is one
raise. Suppose replicas holding `c` and `e` had to reach the same version by
raising alone, never merging. Two plans:

| Plan | `c`'s replica raises | `e`'s replica raises | Common result | Raises |
|---|---|---|---|---|
| reach the join | nothing | `0.001..0.01`, `0.01..0.1`, `0.1..1` | `c` | 3 |
| aim higher | the hole `0..0.001` | everything, `0..1` | `Version { 0..1 => 1 }` | 2 |

If they finish at `y`, with `c`'s replica raising by `A` and `e`'s by `B`,
then `y = c + A = e + B`, so `c − e = B − A`. Written as a single difference,
one replica's raises appear with a plus sign and the other's with a minus
sign. That is the whole meaning of "signed". No brick ever lowers anything;
a negative brick is a raise performed on the other side. The second plan is
the decomposition `c = (0..1) − (0..0.001)`.

So `brick_distance(a, b)` is the fewest raises, split between the two
replicas, that bring them to a common version without merging (proved: the
plans correspond exactly to the ways of writing `a − b` as one sum of
bricks minus another). Running time backwards gives the mirror image: the
fewest raises, on two branches since a common ancestor, that explain how `a`
and `b` diverged (parsimony, in the phylogenetic sense). For `c` and `e`,
the ancestor equals `e` except one level below it over the hole: `c`'s
branch raised everything, and `e`'s branch raised the hole. Adding a seed
tick of shared prehistory to all three makes that ancestor a genuine
version.

This is a hypothetical cost model. Real ITC replicas converge for free by
joining, and a real tick is not always one raise (`fill` can raise many
blocks in one tick). It measures how much independent work the disagreement
would demand, not what convergence costs.

**The area analog shows what counting adds.** If cost is the area raised
rather than the number of raises, the cheapest common result is exactly the
join, and the cost is `distance`. With counted cost the cheapest common
result can lie above the join, as `Version { 0..1 => 1 }` does here: one
broad raise beats several narrow ones. (Submodularity, in another form.)

**Algebraically**, versions under pointwise addition form a monoid generated
by bricks, `min_ticks` is word length in it, and `brick_distance` is word
length in its group completion, as integers complete the naturals. This
restates the sign reading; it adds no domain meaning.

**Prefix intervals and signed digits (probed exhaustively at depth 6).**
Writing a number as a sum of powers of two takes one term per one bit:
`7 = 4 + 2 + 1`. Allowing subtraction can take fewer: `7 = 8 − 1`. The
fewest terms, with subtraction allowed, is the weight of the number's
*non-adjacent form* (NAF), a representation elliptic-curve cryptography uses
because subtracting a point costs no more than adding one. The id range
`0..0.111` (`[0, 7/8)`) behaves the same way: its added bricks are
`0..0.1`, `0.1..0.11`, and `0.11..0.111`, three, one per one bit; with
subtraction it is `0..1` minus `0.111..1`, two. For any range starting at 0,
the brick counts are exactly these digit counts. The crate's rank integrator
already uses the trick, storing `2^(32k) − 1` as two signed digits in
`SparseWidth`. In the domain this explains why subtraction helps (runs of
ones collapse) rather than adding a capability.

**Lower-bound certificates (proved; probed by brute force at depth 3).**
Finding a decomposition with few bricks shows an upper bound. A different
kind of evidence shows a lower bound:

1. Put an integer *charge* on each cell.
2. Require that every dyadic block (the whole range, each half, each
   quarter, …, each cell) holds a total charge of −1, 0, or +1.
3. Score the charges against `f = a − b`: sum, over cells, height times
   charge.

Any brick decomposition of `f` uses at least the score. Each brick is ±1
over one block, and that block's total charge is between −1 and +1, so each
brick contributes at most one point; the score is the bricks' total
contribution. For `c` (cells numbered 0 to 7, the hole at 0):

```
cell     0   1   2   3   4   5   6   7
c        0   1   1   1   1   1   1   1
charge  -1  +1   0   0  +1   0   0   0
```

Every block's total is in range: the whole range 1; halves 0 and 1;
quarters 0, 0, 1, 0; each cell −1, 0, or +1. The score is `1 + 1 = 2`, so `c`
needs at least two bricks, and `0..1` minus the hole shows two suffice. A
standard theorem (nested families of sets give totally unimodular
constraint matrices, so the linear program has no integrality gap) makes
the best score always equal the minimum. A certificate is easy to check, so
it could serve a test, but it is a proof technique, not a domain concept.

**Summary.** The sign reading carries the meaning: the fewest independent
raises, split between two replicas, that reconcile them without merging, or
equivalently explain their divergence. The algebra, the digits, and the
certificates are supporting mathematics. If it is ever built, *parsimony
distance* names what it measures.

### Signed comparison: skew

Finch's question: is there a signed, integer-valued *skew* between two
versions, in the sense of clock skew?

**A skew is a potential difference (proved).** Ask, as clock skew satisfies,
that skews add along chains: `skew(a, c) = skew(a, b) + skew(b, c)`. Fixing a
reference version `v₀` and setting `φ(a) = skew(a, v₀)` gives
`skew(a, b) = φ(a) − φ(b)`. So choosing a skew means choosing one scalar per
version, and the single-version measures are the candidates.

**Lead minus lead.** The area form is already a potential difference:
`b.lag(&a) − a.lag(&b) = ∫ (a − b) = rank(a) − rank(b)`, which `rank_cmp`
integrates. The counted form, `lag_bricks(b, a) − lag_bricks(a, b)`, does not
add along chains, because bricks are not modular. Modularity of `rank` is
exactly what makes "my lead minus your lead" a potential difference.

**No integer skew sees every advance (proved).** Below `flat(1)` there are
strict chains of every length:
`0 < Version { 0..0.1 => 1, 0.1..1 => 0 } < Version { 0..0.11 => 1, 0.11..1 => 0 } < … < flat(1)`.
An integer-valued `φ` that rises on every strict advance would need
`φ(flat(1)) − φ(0)` to exceed every chain length. So an integer skew is at
best weakly monotone, and `rank` is a dyadic rational because it is strictly
monotone. Strict sensitivity and integer values cannot coexist.

**Candidates:**

| Skew | Integer | Depth-independent | Respects order | Causal reading |
|---|---|---|---|---|
| `rank(a) − rank(b)` | no (dyadic) | no | strictly | net area of history |
| `max(a) − max(b)` | yes | yes | weakly | `s > 0` means `a` has seen at least `s` ticks `b` has not |
| `min(a) − min(b)` | yes | yes | weakly | difference in full layers every id has seen |
| `min_ticks(a) − min_ticks(b)` | yes | yes | no | none |
| net count of a minimal signed decomposition of `a − b` | ill-defined | | | |

The causal reading of the maximum skew is the missing-ticks bound from
[Causal content](#causal-content), read in both directions. It is one-sided:
concurrent versions can have skew 0 while each lacks many ticks of the
other's, and activity below the peak never registers. The true tick skew,
the difference in the number of ticks behind `a` and behind `b`, is
unidentifiable from the versions, since each count is unbounded above.

The net signed count is ill-defined because minimal decompositions differ.
`Version { 0..0.01 => 1, 0.01..0.1 => 0, 0.1..1 => 1 }` is `flat(1)` less a
quarter (one brick added, one subtracted) or two added pieces. Both have two
bricks, with nets 0 and 2.

**Naming.** "Tick skew" overclaims, since the maximum skew bounds ticks
rather than counting them. "Depth" collides with tree depth. A candidate is
to call the maximum height a version's *peak*, and the minimum its *base*,
the paper's term for the root's base value, giving `peak_skew` and
`base_skew`. These are working names, not rulings.

Computing it means choosing a base for every node of the two versions' common
refinement, minimizing `Σ |base|` subject to each leaf's path sum: a tree L1
problem, solvable by dynamic programming over convex piecewise-linear costs.
That is a third algorithm, unlike either `rank`'s or `min_ticks`'s. In the
positive-only problem the optimal bases are the canonical minima; in the
signed problem they are median-like. `brick_distance` is a structural edit
distance with no causal reading, since no operation lowers heights.

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

## Computing the measures

Read against `fix/suanpan-uniform-amortization` (for `rank`, `lag`, and
`distance`) and `fix/min-ticks-transient-heap` (for `min_ticks`).

### One formula, two degenerate corners

Give each dyadic interval a weight `w` that depends only on its depth, and
define the brick measure `μ_w(v) = Σ_N base(N) · w(N)`. Expanding `base`
gives (proved)

```
μ_w(v) = Σ_leaves h(L) · w(L)  +  Σ_internal m(I) · δ(I),    δ(I) = w(I) − w(left) − w(right)
```

where `δ` is the weight's *defect*, how far it is from additive. The formula
holds on any refinement of the canonical tree: splitting a constant leaf adds
and removes equal amounts. So it also holds on the common refinement of two
versions.

- `rank` has `w = 2^−depth`, which is additive: `δ = 0`. The internal term
  vanishes, and `rank` is a weighted sum over leaves, with no tree structure
  beyond leaf widths.
- `min_ticks` has `w = 1`: `δ = −1`. The leaf term is a plain sum of leaf
  heights, and the internal term subtracts every internal minimum, matching
  its documented formula.

Each corner removes the other's hard part. `rank`'s `Integrator` exists to
multiply wide heights by wide sparse widths (`live`, `parked`, and
`deferred` components, sparse base-2^32 widths, a balanced deferred
reduction), for `O(M(n))`. The multiplication cost is necessary, since exact
integration can encode an integer product. `min_ticks` needs no width
products. Its hard part is tracking nested subtree minima of wide heights
(`RangeMinima`, a record stack with close counts) and summing leaf heights
without copying them (frozen references with small integer coefficients,
merges that read only the narrower chunk), for `O(n)`.

A single kernel for general `w` would need both machineries at once. Minimum
records would settle against weighted close sums, which for width-like
weights are the same sparse wide integers `rank` handles. No meaningful
member of the family needs both, so such a kernel would either charge
`min_ticks` the `M(n)` class or specialize back into the two existing
implementations. The kernels should stay separate.

### Shared drivers, separate folds

The clean generalization lives one layer up. Each measure has two parts:

1. **A driver** turns input into a stream of leaves: depth, the turn depth
   to the next leaf, the height change, and the input bits that funded it.
   There are two: one version (`VersionRegionReader`), or two versions'
   common refinement with an orientation over the sign of their difference
   (`pair.rs`'s overlay walk).
2. **A fold** consumes the stream: `Integrator` (width-weighted), the
   `min_ticks` fold (counted, with minima), and potentially a running
   maximum, or the integral of a 0/1 indicator.

Today `pair.rs` couples the pair driver to `Integrator` (giving `lag`,
`distance`, and rank comparison through the orientation closure), and
`min_ticks` couples the single driver to its fold. Separating driver from fold
yields the pairwise measures without new kernels:

| Fold | Single driver | Pair driver, orientation `(b − a)⁺` | Pair driver, `|a − b|` |
|---|---|---|---|
| width-weighted integral | `rank` | `lag` | `distance` |
| brick count | `min_ticks` | `lag_bricks` | brick count of `|a − b|` |
| running maximum | max height | `lag_height` | distance-height |
| indicator integral | width | `lag_width` | distance-width |

Floor-truncated measures need no new code either: the pair driver with
`flat(K)` as one operand gives the part above height `K`.

### What could interfere

- **Funding.** Both folds freeze a wide live change when it outgrows the
  delta that reached the leaf. With an orientation, the derived delta is
  bounded by the input deltas: the sign-change correction obeys
  `|D'| ≤ |D' − D|`, the argument `Integrator::jump` relies on. But a fold's
  triggers should compare against the *input* bits, as `pair.rs` does, not
  the derived delta's width. The `min_ticks` fold today reads
  `digit_len(height_change.magnitude())` from the delta it is handed, which
  under the single driver is the input delta.
- **Turn depths.** The `min_ticks` fold opens and closes subtrees from turn
  depths. The common refinement has them: a boundary between consecutive
  refinement leaves is a boundary of whichever version advanced, and the
  lowest common ancestor of two adjacent dyadic intervals depends only on
  their shared boundary, so both versions agree when both advance. The pair
  driver does not currently expose them.
- **Running maxima are not free.** A wide maximum compared against every
  later narrow height is the quadratic pattern the crate avoids. The global
  maximum is the root case of what `RangeMinima` tracks (negated), which
  `tick` and `min_ticks` already share.
- **Cost of a measure** follows its fold: counted and maximum folds over the
  pair driver should stay linear, and width-weighted ones `O(M(n))`. Each is
  a claim for the fuelscape boards to confirm, not an inference to rely on.

Only the folds with consumers should be built: today, the running maximum and
indicator integral over the pair driver, for the diff's summary line.

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
- Does the support's piece count, the depth-independent breadth measure,
  serve any consumer?
- Does `brick_distance` serve any consumer worth a third algorithm?
- Not a question about measures, but noted because the probe first assumed
  otherwise: `Party::forks(k)` creates `k` new parties and leaves the
  receiver a share, as `fork` does. From the seed, `forks(32)` makes 33
  parties, two of width 2⁻⁶ and the rest 2⁻⁵. As far as I can tell this is
  the intended design.

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
  `min_ticks`;
- `cargo run --bin triangle`: brick counts of absolute differences breaking
  the triangle inequality;
- `python3 py/norm.py`: the signed brick norm by dynamic programming over
  shallow dyadic trees, checked against popcount and NAF weight on every
  prefix indicator at depth 6;
- `python3 py/dual.py` (run from `py/`): the charge-certificate duality by
  brute force at depth 3.

Its histories fork up to 8 parties from the seed and mix ticks with joins
from a pool of versions, from a fixed seed.
