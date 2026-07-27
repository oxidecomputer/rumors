#import "fig.typ": *

= The skyline <skyline>

== A version is a step function

Start from the semantics, not the syntax. The paper interprets an
event tree as a function from $[0, 1)$ to the naturals; the tree is
merely one spelling of that function. Plot the function and you get
plateaus of differing heights over subintervals — a city _skyline_.
Here is the event tree $(0, 1, (0, 0, 2))$ and the function it
denotes:

#figure(
  grid(columns: (auto, auto), column-gutter: 28pt, align: horizon,
    tree(([0], ([1], none, none), ([0], ([0], none, none), ([2], none, none)))),
    skyline(
      ((0.5, 1), (0.25, 0), (0.25, 2)),
      ticks: ((0.0, [0]), (0.5, [½]), (0.75, [¾]), (1.0, [1])),
    ),
  ),
  caption: [One value, two spellings: the paper's tree
    $(0, 1, (0, 0, 2))$ and the skyline it denotes. Every leaf of the
    tree is a plateau of the skyline; the leaf's interval is dyadic —
    width $2^(-d)$ at depth $d$ — and its absolute height is the sum
    of bases from the root down.],
) <fig-skyline>

Call each of the skyline's pieces a _plateau_: a dyadic interval
together with the constant height the function takes on it — a leaf
of the tree, in tree language. A plateau is deliberately _not_ "a
maximal constant run": two adjacent plateaus of equal height are a
real shape (they arise exactly when a constant run's extent is not a
dyadic interval, so no single leaf can span it), and the canonical
form of @canonical will keep them. Two observations make the skyline
the right thing to store:

+ *Every operation is a pointwise statement about the skyline.*
  Comparison is pointwise $<=$; join is pointwise max; meet is
  pointwise min; a tick raises the skyline somewhere over the caller's
  id; rank (a measure we will meet in @measures) is the area under it.
  None of them care how a tree spelled the function.

+ *The tree's interior numbers are redundant.* Given the tree's
  _shape_ and the _absolute_ heights of its leaves, the function is
  fully determined. The paper's per-node bases are an artifact of
  spelling the same information top-down — the artifact that forced
  every walk in @naive to maintain path sums. The distinction bites
  as soon as a base is nonzero: the paper's tree $(1, 0, (0, 0, 1))$
  has a leaf _written_ $0$ that _stands_ at absolute height $1$ (the
  base above it), and denotes the plateau heights $1, 1, 2$ — which
  is all the skyline stores.

So we store the shape, and the leaf heights, and nothing else.

== The coding <coding>

A version is stored as one contiguous, bit-packed stream with two
interleaved kinds of content. There is no node graph behind it — at
rest and on the wire the value _is_ this stream, and every operation
in @operations runs on it directly.

*Topology.* Event trees are full binary trees — a node has two
children or none — so the shape is one flag bit per node, written in
preorder: `0` for an internal node, `1` for a leaf. Preorder means a
node's flag precedes everything in its subtree, and — the property
every sweep leans on — *the leaves appear in left-to-right order*:
read in stream order, the leaves are exactly the plateaus of the
skyline, west to east. No bit says where a subtree ends; none is
needed, because a full binary tree is self-delimiting: start a count
of outstanding obligations at one; an internal node consumes its
obligation and creates two (net $+1$), a leaf consumes one (net
$-1$); the tree ends exactly when the count reaches zero, and no
proper prefix reaches zero early. Bits pack most-significant-first
within each byte — a convention that matters only in @machine, where
it lets whole codes settle under one count-leading-zeros instruction.

*Payloads.* Each leaf's flag is followed immediately, in-stream, by
its plateau's height. The first leaf stores its height _absolutely_;
every later leaf stores the _difference_ from the previous leaf in
stream order. Neighboring plateaus tend to sit close in height even
when both stand very tall — that is precisely what the paper's
normalization arranges — so differences are usually small where
absolutes are usually not. A difference can be negative, so it is
folded onto the naturals first by the _zigzag_ map

$ +k arrow.r.bar 2k, quad -k arrow.r.bar 2k - 1 $

(a bijection, with no spelling for "negative zero"), and the result
$v$ is written as the _Elias gamma_ code of $v + 1$: with
$b = floor(log_2 (v+1))$, write $b$ zeros, then the $b + 1$ bits of
$v + 1$ in binary (whose leading bit is necessarily `1` — that is
how the decoder knows the zeros have ended). The code costs

$ 2 floor(log_2 (v + 1)) + 1 "bits:" $

one bit for a zero, three bits for $plus.minus 1$, and about
$2 log_2 v$ bits for a large $v$ — self-delimiting (the zero run
announces the length), with exactly one spelling per natural.

Here is the whole stream for our example. The tree
$(0, 1, (0, 0, 2))$ has preorder flags (internal, leaf, internal,
leaf, leaf) and leaf heights $1, 0, 2$, hence payloads: absolute $1$,
then $delta = -1$ (zigzag $1$), then $delta = +2$ (zigzag $4$):

#figure(
  align(center, bitrow((
    ("0", "t", [internal]),
    ("1", "t", [leaf]),
    ("010", "p", [$h = 1$]),
    ("0", "t", [internal]),
    ("1", "t", [leaf]),
    ("010", "p", [$delta = -1$]),
    ("1", "t", [leaf]),
    ("00101", "p", [$delta = +2$]),
  ))),
  kind: image,
  caption: [The packed stream of $(0, 1, (0, 0, 2))$: sixteen bits.
    Topology flags shaded blue, payload codes orange. The payload
    $delta = -1$ zigzags to $1$ and codes as $gamma(2) = mono("010")$;
    $delta = +2$ zigzags to $4$ and codes as
    $gamma(5) = mono("00101")$.],
) <fig-stream>

The empty version — the constant-zero function — is one leaf of height
zero: two bits.

== Ids are boolean skylines <id-coding>

A party — the id component — has the same reading one step simpler: a
$0$-or-$1$ landscape over the same interval, $1$ where the party owns
the id space, $0$ where it does not. No payloads are needed at all.
The paper's id trees are full binary trees like its event trees, but
here the coding _prunes_: an unowned subtree — a whole `0` of the
paper's syntax — stores nothing, because its parent's tag already
said no child follows. The pruned shape is 0-, 1-, or 2-ary, so the
coding spends _two_ flag bits per stored node, answering "does a left
child follow?" and "does a right child follow?", with the childless
tag `00` as a terminal: a wholly owned region.

The seed, owning everything, is one terminal: two bits. The id
$(1, 0)$ — own the left half — is a left-only node followed by a
terminal, four bits in all:

#align(center, bitrow((("10", "t", [node: left child only]), ("00", "t", [terminal: owned]))))

At a hundred participants with stable membership, a party measures
about three bytes; sustained fork-and-retire churn fragments
ownership and raises it to a few tens (measured: the paper's own
space-consumption scenarios, reproduced on our implementation). The
id side of the system is, by design, nearly free.

One boundary case rounds out the coding. The paper's _anonymous_
stamp $(0, e)$ — causal information with no identity, used for
messages — owns nothing, and a root that owns nothing has no parent
tag to absorb it. The implementation dissolves the case instead of
spelling it: an anonymous stamp is modeled as a bare version, so a
party, wherever one exists at all, is nonempty by construction.

A clock — a party and its version — concatenates the two streams. Both
codings are prefix-free (self-delimiting trees, self-delimiting
integer codes), so the pair needs no length prefix, and clock values
compose into larger messages without framing.

== Canonical form: one string per value <canonical>

The coding so far maps every tree to a stream. To get its strongest
property we need the converse discipline: every _function_ must have
exactly one accepted stream. Three rules, enforced strictly at every
decode boundary:

+ *Minimal topology* (the _sibling-merge rule_). No internal node
  whose two children are both leaves of equal height — such a pair is
  one plateau spelled as two, and merges. In stream terms: a
  right-sibling leaf may never carry a zero delta when its brother is
  a leaf. (A zero delta between consecutive leaves that are _not_
  siblings is a real, canonical shape: two equal plateaus separated
  by a subtree boundary — the non-dyadic constant run met under
  @fig-skyline.)
+ *Nonnegative heights.* The payload stream is signed; nothing else
  stops a delta from driving the running height below zero, so the
  decoder must.
+ *Exactness.* One complete tree, no trailing bits, and (at the byte
  boundary) zero padding only.

Note what is _absent_: the paper's other normalization — lifting a
common minimum into the parent — has no analogue here, because there
are no interior numbers to lift into. Storing absolute heights at the
leaves quietly discharged one of the two normal-form obligations.

The three rules make the spelling unique, and the argument is short
enough to give. For a dyadic step function $h$, define $T(h)$: a
single leaf if $h$ is constant, else the node over
$T(h|_"left")$ and $T(h|_"right")$. First, $T(h)$ satisfies the
sibling-merge rule: its two children are equal-height leaves only if
$h$ is constant on both halves — but then $h$ was constant and
$T(h)$ was a leaf. Second, any rule-satisfying tree for $h$ equals
$T(h)$, by induction from the root: a tree spelling a non-constant
$h$ cannot be a leaf, so it is a node whose children spell the two
restrictions (uniquely, by induction); and a tree spelling a
_constant_ $h$ must be a leaf, since an internal node's children
would spell two constant halves — by induction two equal leaves,
which the rule forbids. Heights are function-determined; gamma has
one spelling per natural and zigzag has none to spare. Uniqueness
is not an aesthetic: it is a load-bearing feature bought deliberately,
and @compactness prices exactly what it costs in coding room
($4.3%$, as it turns out). What it buys:

- *Byte equality is semantic equality.* Equality and hashing are raw
  byte operations — no walk, no decode. Any system that deduplicates,
  content-addresses, or gossips values compares them constantly;
  those comparisons are `memcmp`.
- *Decode can reject rather than repair.* Every valid value has
  exactly one acceptable spelling, so anything else is refused. There
  is no normalization pass, no "fix up and continue" path whose cost
  an adversary controls, and no way for two replicas to disagree
  about the bytes of a value they agree about.
- *Encode is a copy.* The resting form is the wire form; encoding
  clones a buffer, decoding is one validating pass that then adopts
  the bytes as the value's own storage. A value's memory footprint is
  its wire footprint — recall @naive-constants's closing list.

== Validation, and a bill coming due <validation>

Strict decode must check the three rules in one pass over untrusted
bits, and @naive taught us what that pass may cost: nothing
per-node beyond the node's own bits, nothing per-level beyond bits.

Minimal topology and exactness need, per open ancestor, two bits of
state — "is my left child complete?" and "was that child a leaf?" —
kept on a packed bit stack: roughly two bits of transient memory per
level against the level's roughly three bits of input. Depth is paid
for in bits, honoring @naive-recursion's budget.

Nonnegativity is the interesting one: it needs the running absolute
height, updated by every delta, sign-checked at every leaf — exactly
the "running value" that @ladder proved dangerous, and the boundary
comb aims straight at it: $plus.minus 1$ deltas, three-bit codes,
astride $2^k$, so a normalized running height pays a $k$-bit carry
per three-bit code — $Theta(W^2)$ work in a $W$-bit stream, _in the
validator_, on arbitrary bytes. (Measured, on a deliberately plain big-integer
sweep kept as a tripwire: the quadratic is real and reproducible.)
The skyline's compactness has written a check the representation
alone cannot cash. The accumulator of @accum cashes it: with the
running height held as balanced signed digits, validation is linear
per wire bit on every input, and the whole decode boundary — the
most exposed surface in the library — costs one sequential funded
pass.

== The trade made explicit <trade>

The packed stream surrenders random access: there is no $O(1)$ "height
at position $x$", no jumping into the middle. Every question is
answered from the front. This is the right trade _for this API_ —
comparison, join, meet, tick, and the measures are whole-value
questions, and a linear scan of a few dozen to a few thousand
contiguous bytes is the cheapest thing a modern memory system does
(@machine quantifies this) — but it is a trade, and a different API
(point queries against enormous single values) would want a different
structure. The operations of @operations are the demonstration that
nothing the clock actually asks for ever misses the random access.
