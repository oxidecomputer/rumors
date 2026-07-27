# `before`: the skyline encoding — an exposition of the Version representation redesign

Status: expository companion to
`design/before-adversarial-resource-amplification.md`, whose §10
("Tier 2") specifies this design normatively and whose §12 holds the
open decision on adopting it. Written 2026-07-23, while that decision
is pending: nothing here is a decision record. The epistemic key is
the same — **[measured]** = observed by an instrument committed on
the `before-hardening` branch (the ratio meter, the Euler-tour probe,
the amplification board); **[derived]** = argument from code or
arithmetic; **[open]** = pending. This document optimizes for
intuition; where it and §10 disagree, §10 is the specification.

## 1. What a Version *is*: a skyline

Forget trees for a moment. Semantically, a Version is a **step
function over the id space** — the unit interval [0, 1). Every point
of the interval has seen some number of events; a Version maps each
point to that count. Picture a city skyline: flat plateaus at
different heights, with jumps only at dyadic boundaries (halves,
quarters, eighths, …).

The event tree is one way to *write down* that skyline. An internal
node `(n, l, r)` says "add `n` everywhere in my region, then let `l`
and `r` refine the two halves"; a leaf `n` says "exactly `n` more,
uniformly, here." So the height of the skyline at any point is the
**sum of the bases along the root-to-leaf path** — the stored numbers
are relative increments; the *meaning* is the path sum. This one fact
generates everything that follows.

## 2. What the current format stores, and why it fights back

Today a Version at rest is the tree in preorder — per node, a flag
bit plus an Elias-gamma code of its base — kept in a normal form that
does two things:

- **min-lift**: each node's base absorbs the minimum of its
  children's, so magnitude migrates toward the root and children keep
  small residues;
- **collapse**: a node whose children are equal-valued leaves becomes
  a single leaf.

Min-lift is a *factoring*: a large magnitude shared by an entire
region is stored once, at the highest node that dominates the region.
It is a genuinely good compression scheme for the at-rest bytes, and
nothing in this document disputes that.

The trouble begins when you compute. Every algorithm — compare, join,
tick — needs the skyline's *absolute heights*, but the format stores
*increments hung on a tree*. The current code obtains heights in one
of two ways, and each is one of the audited amplifier classes:

- thread path sums down a recursive walk, each frame holding its own
  owned copy of an arbitrary-precision sum (amplifier V1 — quadratic,
  because d live frames × B-bit sums is d·B bytes for a 2B + 4d-bit
  input) [measured: ×6,668 at 29 KiB, doubling the input quadruples
  the peak];
- or materialize the **working form**: a fixed-width node array at
  ~24 bytes per node that cost ~2 bits packed, so the tree can be
  *edited* (amplifier V4, ×~100 per materialization) [measured].

Why does computation need to edit at all? Because normal form is
discovered **bottom-up** while the format is written **top-down**.
Only after both children of a node are finished does the algorithm
learn the min `m` that min-lift wants raised: the parent's base gains
`m` — and its gamma code *widens* — while each child's root gives up
`m` and its code *narrows*. In a preorder bit stream the parent's
code was emitted before its children's. Widening an already-written
variable-width code means shifting every bit after it: the one
genuinely **back-referential edit** in the system. The working form
exists to make that edit O(1); the entire node-array economy is rent
paid on it. (`fill`'s deferred-leaf machinery is the same phenomenon
in another corner.)

That is the shape of the problem: the format's *maintenance
invariant* — parent codes widen on close — is incompatible with
streaming emission, so every write path detours through a
fixed-width intermediate two orders of magnitude larger than the
data.

## 3. The two observations that dissolve the problem

**Observation A: the internal bases are redundant.** The skyline is
fully determined by its plateau heights — the absolute values at the
leaves — plus the topology saying where the boundaries sit. The
internal bases are merely one particular factoring of those heights
(the min-lift factoring); they can be recomputed from the leaves in
one pass whenever wanted. The at-rest format is therefore storing a
*derived compression artifact*, and the algorithms pay V1 to
reconstitute the underlying values and V4 to maintain the artifact.
The move: **store the skyline, not the scaffolding.**

**Observation B: adjacent plateaus have provably close heights.**
Consecutive leaves in preorder are *neighbors on the interval*. The
difference between leaf `i` and leaf `i+1` telescopes over exactly
the two path segments to their lowest common ancestor — every base at
or above the LCA cancels:

    v(i+1) − v(i) = (sum of bases, LCA-exclusive, down to leaf i+1)
                  − (sum of bases, LCA-exclusive, down to leaf i)

So delta-coding consecutive leaf heights recovers the sharing that
min-lift achieved by factoring, through a different mechanism: a big
magnitude is paid once where the skyline *steps up* and once where it
*steps down*, instead of once at the dominating node.

That "at most twice" is a theorem-shaped claim, and it has a clean
charging picture. Walk the tree's Euler tour. Each stored base `b` at
node `v` lies on the *exit* path segment of exactly one
consecutive-leaf pair (the pair whose LCA is above `v` on the way
out of `v`'s subtree) and on the *entry* path segment of exactly one
other (the pair entering the subtree). Those are the only two deltas
whose telescoped sums include `b`. Since
`gamma(x + y) ≤ gamma(x) + gamma(y) + O(1)`, the total delta coding
is at most twice the total stored-base coding plus O(1) per leaf
[derived — §10.4 of the amplification doc; validated executably by
the committed Euler-tour probe]. Hence the compactness envelope:

> Skyline coded size ≤ ~2× today's coded size, and sometimes smaller.

The measurement came back stronger than the argument requires
[measured, ratio meter, ~13k samples across arbitrary, organic,
adversarial, comb, and gossip-shaped families]: the ratio stayed
≤ 2× *outright* — the O(1)-per-node allowance was never needed — with
the deliberately tight alternating comb reaching 1.92, and many
organic shapes coming out *smaller* than today's encoding. Smaller is
possible because deltas share magnitude across *any* adjacent
boundary, while the tree factoring can only share within a subtree:
on a staircase of heights `M, M+1, M+2, …` spanning several subtrees,
min-lift stores growing residues while the skyline stores a constant
`+1` per step.

## 4. The format

A Version becomes two interleaved streams in one bit string:

- **Topology**: preorder flag bits — one bit per node, `0` internal,
  `1` leaf. Internal nodes carry *no numbers*. (Amendment 2026-07-27,
  #67: the flag polarity inverted inside the same unshipped revision
  so a descent reads as one word-parallel unary run; a bijection on
  encodings, every size and cost figure in this document unchanged.)
- **Leaf payloads, in-stream at each leaf position**: the first
  leaf's absolute height as `gamma(v₁)`; every later leaf as
  `zigzag-gamma(vᵢ − vᵢ₋₁)`, deltas taken over consecutive leaves in
  preorder. Zigzag maps signed to unsigned with one canonical sign
  convention (`k ≥ 0 → 2k`, `k < 0 → 2|k| − 1` — no negative zero),
  and the existing gamma machinery codes the result; the 64-bit
  window fast path applies unchanged, since the code shape (`2k+1`
  bits) is identical.

Two facts worth pausing on:

- **This is the id-side format plus a payload stream.** A `Party` is
  already pure topology (2-bit presence tags) with no integers — and
  the id side has never needed a working form, an unpack, or a
  quadratic anything, because nothing it writes ever changes width
  after being written. The skyline encoding is precisely the change
  that gives the event side the same property, after which one
  packed-tree-builder abstraction serves both sides, parameterized by
  the per-node payload (none for ids, a leaf delta for events).
- **Canonical form stays byte-unique** [derived]: heights are
  function-determined; minimal topology (no uniform sibling pair,
  enforced exactly as today's collapse) makes the tree unique; deltas
  are determined by heights and the sign convention is fixed. So
  `Eq`/`Hash` remain byte-equality, exactly as today.

A subtlety that matters for validation (§7): a *zero* delta is legal
between non-sibling consecutive leaves — two plateaus of equal height
separated by a subtree boundary are a real, canonical shape. Zero is
prohibited only where the two leaves are *siblings*, because equal
sibling leaves are exactly what collapse removes.

## 5. Reading: every binary operation is a merge of run-length encodings

Two Versions are two step functions over the same interval. Every
binary operation the crate performs is pointwise:

- comparison asks for the sign of `a − b` on every elementary
  interval (`≤` everywhere, `≥` everywhere, mixed = concurrent);
- join is pointwise `max`; meet is pointwise `min`.

On skylines these are all the same walk: a **merge over two sorted
run-length encodings**. Each cursor tracks its current leaf's
interval — a depth stack driven by the topology bits gives the
interval's width (2^−depth) and end — and the merge repeatedly
advances whichever cursor's interval ends first, crossing one
boundary at a time. The overlay of the two partitions is the set of
elementary intervals.

State per step: one running signed difference `D = height_a −
height_b`, updated by adding `a`'s delta or subtracting `b`'s as
their boundaries pass. Comparison folds `sign(D)` per elementary
interval and can exit early on a strict mix. Join and meet emit
`max`/`min` per interval — which is `height_a` or `height_b`
selected by `sign(D)`, both recoverable from `D` and one tracked
absolute anchor — and re-delta the output on the fly.

Note what is *absent*: recursion (depth costs ~2 bits of topology
stack per level, not ~0.5 KiB of stack frame — amplifier V2's
substrate), per-frame owned path sums (V1's substrate — there is
exactly one accumulator in the whole walk), and the `Zero`-broadcast
machinery that today re-walks a deep subtree against a synthetic zero
when the other side bottoms out early (a leaf on one side is just a
long plateau; the merge consumes the deep side's boundaries against
it without any pretend-tree).

Why the accumulator is cheap [derived; pinned empirically by the
limb-work meter]: adding a small delta into a big `D` is amortized
O(1) limb work by the standard carry-run potential argument, and any
delta large enough to force long carry runs repeatedly is a delta
whose gamma code the *input paid for* at matching length. Total
arithmetic over a sweep is O(n + m) amortized in the packed input
bits. (The pathological case — oscillation across a `2^64k` carry
cliff — requires the input to keep re-buying comparably-coded
magnitudes; the meter's cliff-straddling generators exist to keep
this claim pinned rather than assumed.)

## 6. Writing: append, and occasionally truncate

Here is the punchline of the whole design. Recall §2: the old
format's normalization *widens the parent* after the children close —
back-referential, hence the working form. In the skyline format
**there is no parent base to widen**. The only normalization left is
collapse, and collapse has become *local and subtractive*:

- The builder emits topology and leaf codes in preorder, appending.
- When a node closes with both children being leaves, canonicality
  asks: are they equal? But "equal sibling leaves" is literally **"the
  right sibling's delta is zero"** — a fact about the last code the
  builder just emitted, no arithmetic, no values.
- If zero: the pair is a uniform region. Repair = **truncate** the
  output back to the recorded start of the left leaf's code and
  re-emit a single leaf (whose delta against its new predecessor is
  computed from quantities already at hand). The collapse can cascade
  upward — the merged leaf may now equal *its* sibling — and each
  cascade step is another truncation.

Truncation-only repair is what makes the builder streaming: an
emitted bit is truncated at most once (it is never re-emitted wider),
so emission is amortized O(1) per output bit, with transient state
one recorded position per open ancestor — bits per level, not bytes
per node [derived]. This is exactly the property `IdBuilder` has
today — its normalization is a fixed-width 2-bit tag patch plus
truncation-only collapses — and exactly why the id side never grew a
working form. The event side inherits the property the moment its
per-node payload stops being a widenable prefix code and becomes a
leaf-position delta.

What gets deleted, once every read is a merge and every write is
append-and-truncate [derived; the §12 decision table]:
`WorkingVersion`, the event `Builder`'s node array, `EvReader`'s
packed/working form split and its `Zero` broadcast, `fill`'s deferred
leaf, decode's value-carrying parse frames (§7), and — since nothing
recurses on input depth any longer — most likely the `stacker`
dependency and `recurse::descend!` themselves ([open] until the last
walk is converted).

## 7. Validation without arithmetic

Today `decode` re-derives normal form with 56-byte frames holding two
`Base` values per unfinished ancestor (amplifier V5, ×118 on the
dense spine) [measured]. Under the skyline format the canonical-form
conditions are:

- topology minimal: no internal node whose children are two leaves
  with a zero right delta — per §4, zero deltas elsewhere are legal;
- codes canonical: gamma/zigzag well-formed (the existing decoder's
  per-bit arbiter already enforces this shape);
- first leaf absolute, all others deltas — positional, free.

So the validator needs, per open ancestor, roughly: one bit for
"is my left child a completed leaf," and one bit for "was the last
completed code a zero delta." **About 2 bits per level and zero
arithmetic** — the validator never materializes a single height.
Decode's transient state drops from ~n/4 × 56 bytes to ~n/4 × 2 bits:
the V5 row goes green by construction rather than by tuning
[derived; to be pinned by the meter when built].

## 8. Ticks as splices, queries as folds

**Tick** (`fill` + `grow`) changes the skyline on exactly one
root-to-leaf path — the region the ticking `Party` owns rises;
everything else is untouched. On a packed skyline that is the id
side's `split` pattern (today's one recursion-free, splice-based id
operation): copy every off-path subtree *verbatim as a raw bit
range*, re-emit only the path nodes, and repair exactly one boundary
delta at each splice edge (the first leaf of a copied range has a new
predecessor; one locally-sized re-code). Time O(n), transient state
O(path) [derived]. Today the same operation materializes the full
node array (×198) [measured].

**Queries** (`rank`, `distance`, `lag`, `min_ticks`, `project`,
`causally::contains`) are folds of `value × interval-width` (or
min/max of value) down the leaf sweep — one pass, one accumulator.
The board caught `project` running V1's owned-path-sum pattern on an
unlisted path [measured, P0 landed record]; under the skyline format
that entire family shares the one sweep skeleton and the amplifier
class has no substrate left.

**Display and parsing** keep the paper notation unchanged: rendering
derives internal bases from leaf heights in one bottom-up pass
(min-lift is a fold); parsing accumulates path sums in one top-down
pass. Both must use the same sweep discipline as everything else so
the notation paths do not become the last amplifier standing — the
board's `FromStr`/`Display` rows enforce this with numbers rather
than intentions.

## 9. Two worked examples

**Encoding.** Take the paper-notation version `(2, (1, 0, 3), 0)` —
root base 2, left child `(1, 0, 3)`, right leaf 0. Its skyline,
left to right over [0,1): height `2+1+0 = 3` on [0, ¼), height
`2+1+3 = 6` on [¼, ½), height `2+0 = 2` on [½, 1).

- The min-lifted node coding: topology `1 1 0 0 0` (that coding's
  own polarity) interleaved with five gamma codes for the increments
  `2, 1, 0, 3, 0`.
- Skyline: five topology bits `0 0 1 1 1` (`0` internal, `1` leaf),
  then three payloads at the leaf positions:
  `gamma(3), zigzag(+3), zigzag(−4)` — *start at 3, step up 3, step
  down 4*. The skyline itself, run-length encoded.

**A join sweep.** Join it with the version that is uniformly 4 (a
single leaf, skyline `gamma(4)`).

    elementary interval   a    b    max
    [0, ¼)                3    4    4
    [¼, ½)                6    4    6
    [½, 1)                2    4    4

The merge emits heights `4, 6, 4` over the overlay partition —
payloads `gamma(4), zigzag(+2), zigzag(−2)` on the refined topology —
then collapse tidies: the right half of the output is uniformly 4
across what would be a subtree boundary, and wherever a closing node
finds equal sibling leaves (zero right delta) the builder truncates
the pair to one leaf. No tree was materialized at any point: two
cursors, one `D`, an append-truncate output, and a few bits of stack.

## 10. What it costs, honestly

- **The wire breaks.** Every stored `Version` (and therefore `Clock`)
  byte changes. `before`'s codec pin tests and `rumors`'
  `gossip_snapshot`/`bootstrap_snapshot`/`retire_snapshot` suites
  re-pin as a deliberate protocol change; the P2 negative-space
  review sweeps the workspace for any other consumer of the byte
  layout. Cross-version wire compatibility is not attempted — the
  library's model has always been "one universe, one code version."
- **The 2× envelope is real.** [measured] The comb shapes sit at
  1.92×; an adversary (or an unlucky honest history) can make a
  Version's at-rest bytes up to twice today's. The trade purchased:
  the deletion list in §6, transient memory at ~1× packed + bit
  stacks on every operation, and validation with no arithmetic. Many
  organic shapes get *smaller* [measured]; the honest-regime median
  is the number to watch in the ratio meter's statistics of record.
- **Two representation-bridging passes remain** — display and parse
  (§8) — and they are sweep-shaped, not exempt.
- **Risk is concentrated in the new codec's canonical form**: a
  validator bug is a byte-equality bug. The mitigation is the one the
  crate already lives by: the alternating-protocol oracle
  differential, the exhaustive small-scope suite, and the algebraic
  laws all transfer unchanged (they speak semantics, not bytes), and
  the snapshot suite re-pins the new bytes deliberately.

## 11. The correspondence, side by side

| concern | today (min-lift factoring) | skyline |
|---|---|---|
| stored numbers | relative increments on tree nodes | absolute first leaf + neighbor deltas |
| meaning of a number | path-sum context required | height step, self-contained |
| normalization on write | parent widens + children narrow (back-referential) | truncate-and-re-emit (append-only) |
| working form | required by the widening edit | none — the id side's property, inherited |
| compare/join/meet | tree recursion, per-frame sums, Zero broadcast | one merge sweep, one `D` accumulator |
| decode validation | 56 B/level with values | ~2 bits/level, no arithmetic |
| tick | full node-array rebuild | single-path splice |
| queries | recursive walks | leaf-sweep folds |
| at-rest size | 1× (the baseline) | ≤ 2× measured, often < 1× organic |
| wire | unchanged | breaking, snapshots re-pin |

The shortest honest summary: **today's format stores a clever
factoring of the skyline and pays rent on it in every algorithm; the
skyline format stores the function itself, which turns every
algorithm into a merge of run-length encodings — and the write path,
which was the whole obstruction, degenerates into
append-and-occasionally-truncate.** The alternative endgame (Tier
1.5, §9 of the amplification doc) keeps the factoring and routes
around the back-referential edit with a two-pass mirrored scratch: it
reaches the same resource envelope, but by *adding* machinery to
preserve a representation whose maintenance cost is the problem,
where the skyline *removes the problem's substrate*.

## 12. Cross-references

- Normative specification and decision record:
  `design/before-adversarial-resource-amplification.md` §10 (the
  encoding), §10.4 (compactness claim), §12 (the open decision and
  its evidence table), §13 (the meters and board that will pin every
  claim here), §8.1 (the difference accumulator this design's sweep
  absorbs).
- Instruments on the `before-hardening` branch: the ratio meter and
  Euler-tour probe (`tests/`, meter feature), the Tier 2 size
  function (`before::meter::tier2`), the amplification board
  (`just amp-board`).
- The existence proof in today's code: `party/ops/build.rs`
  (`IdBuilder`, truncation-only normalization) and
  `party/ops/split.rs` (splice-based single-path edit).
