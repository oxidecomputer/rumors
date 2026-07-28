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

+ *Every operation is a local statement about the skyline.*
  Comparison is pointwise $<=$; join is pointwise max; meet is
  pointwise min (no operation of the paper's — @measures shows what
  the lattice buys); rank (a measure we will meet in @measures) is
  the
  area under it; a tick asks range questions — maxima and minima —
  over the caller's own region (@tick).
  None of them cares how a tree spelled the function.

+ *The tree's interior numbers are redundant.* Given the tree's
  _shape_ and the _absolute_ heights of its leaves, the function is
  fully determined. The paper's per-node bases are an artifact of
  spelling the same information top-down — the artifact that forced
  every walk in @naive to maintain path sums. The distinction bites
  as soon as a base is nonzero: the paper's tree $(1, 0, 2)$ has a
  leaf _written_ $0$ that _stands_ at absolute height $1$ (the base
  above it), and denotes the plateau heights $1, 3$ — which is all
  the skyline stores.

So we store the shape, and the leaf heights, and nothing else. This
is the sense in which the title says _compiled_: the paper's tree is
the source spelling, and the skyline is the object form the
operations execute directly.

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
proper prefix reaches zero early. (Payloads interleave with the
flags, so _finding_ a subtree's end still costs a scan of that
subtree's own bits — the price every splice in @operations pays.) Bits pack most-significant-first
within each byte — a convention that matters only in @machine, where
it lets whole codes settle under one count-leading-zeros instruction.

*Payloads.* Each leaf's flag is followed immediately, in-stream, by
its plateau's height. The first leaf stores its height _absolutely_;
every later leaf stores the _difference_ from the previous leaf in
stream order. Neighboring plateaus tend to sit close in height even
when both stand very tall. No spelling rule makes it so — by
observation 2 above the leaf heights are function-determined, so no
normalization can alter their differences. The operations'
_dynamics_ do: `fill`
flattens each owned region to one plateau and, where a filled
sibling's minimum stands higher, raises it to that minimum (@tick);
joins move whole regions at once. Real histories therefore keep
adjacent plateaus close (measured across
organic corpora — @ctf-caveat). Differences are
therefore usually small where absolutes are usually not. A difference can be negative, so it is
folded onto the naturals first by the _zigzag_ map

$ +k arrow.r.bar 2k quad (k >= 0), quad quad -k arrow.r.bar 2k - 1 quad (k >= 1) $

(a bijection, with no spelling for "negative zero"), and the result
$v$ is written as the _Elias gamma_ code of $v + 1$ — the first
leaf's absolute height, already a natural, skips the zigzag and takes
the same gamma code directly. The code: with
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
child follow?" and "does a right child follow?". The reading rule:
an absent child's half is unowned. That leaves the childless tag
`00` with only one possible reading — a wholly-unowned subtree is
never stored at all, since its parent's tag already said no child
follows, and an unowned root dissolves below — so `00` is spent on
the one childless case that does need spelling: a _terminal_, a
wholly owned region. Ownership is carried by presence, and the
paper's `0` has no spelling anywhere. Note the consequence the walks must repair later: unlike a
version stream, a party stream does not spell every plateau, so a
walk over it synthesizes each absent child's unowned plateau from
its parent's tag (@id-ops).

The seed, owning everything, is one terminal: two bits. The id
$(1, 0)$ — own the left half — is a left-only node followed by a
terminal, four bits in all:

#align(center, bitrow((("10", "t", [node: left child only]), ("00", "t", [terminal: owned]))))

At a hundred participants with stable membership, a depth-seven
share is sixteen bits — two bytes, derived. Measured, the mean sits
nearer three bytes, ownership fragmentation claiming the excess, and
sustained fork-and-retire churn raises it to a few tens (both
figures from the paper's two space scenarios — static membership
and data churn respectively — reproduced
on our implementation). The
id side of the system is, by design, nearly free.

One boundary case rounds out the coding. The paper's _anonymous_
stamp $(0, e)$ — causal information with no identity, used for
messages — owns nothing, and a root that owns nothing has no parent
tag to absorb it. The implementation dissolves the case instead of
spelling it: an anonymous stamp is modeled as a bare version, so a
party, wherever one exists at all, is nonempty by construction.

A clock — a party and its version — concatenates the two encoded
streams, party first, each padded to its own byte boundary
(@canonical's exactness rule). Both
codings are prefix-free (self-delimiting trees, self-delimiting
integer codes), so the pair needs no length prefix, and clock values
compose byte-aligned into larger messages without framing.

== Canonical form: one string per value <canonical>

The coding so far maps every tree to a stream. To get its strongest
property we need the converse discipline: every _function_ must have
exactly one accepted stream. Three rules, enforced strictly at every
decode boundary:

+ *Minimal topology* (the _sibling-merge rule_). No internal node
  whose two children are both leaves of equal height — such a pair is
  one plateau spelled as two, and merges. In stream terms: a
  right-sibling leaf may never carry a zero delta when its brother is
  a leaf — a _local_ test, because whenever the brother is a leaf it
  is exactly the previous leaf in stream order, so one pass checks
  the rule as it reads. (A zero delta between consecutive leaves that are _not_
  siblings is a real, canonical shape: two equal plateaus separated
  by a subtree boundary. The minimal shape is three plateaus — the
  paper's tree $(1, 0, (0, 0, 1))$, heights $1, 1, 2$, is one: its
  constant run at
  height $1$ spans $[0, 3\/4)$, which no dyadic interval covers.)
+ *Nonnegative heights.* The payload stream is signed; nothing else
  stops a delta from driving the running height below zero, so the
  decoder must.
+ *Exactness.* One complete tree and nothing after it (in a clock,
  the component that follows starts at the next byte boundary).
  Every
  encoded stream ends on a byte boundary with the final partial
  byte zero-filled, and the decoder requires exactly that padding.
  Every size in this document is a bit count before the padding.

Parties get the same discipline, with rules matched to their coding:
no stored node whose two children are both terminals (a wholly-owned
pair is one terminal spelled as two — the id-side sibling merge), and
the same exactness; unowned-as-absence needs no third rule, since
an unowned subtree has no spelling to forbid. The two components are
validated independently — a party and a version are independent
functions on the interval, and no cross-component rule exists to
check. Both codings' rules are
enforced at every decode boundary, so clock bytes — the two streams
concatenated — inherit the uniqueness below.

Note what is _absent_: the paper's other normalization — lifting a
common minimum into the parent — has no analogue here, because there
are no interior numbers to lift into. Storing absolute heights at the
leaves quietly discharged one of the two normal-form obligations.

The rules do two jobs, worth splitting: minimal topology and
exactness remove redundant _spellings_, while nonnegativity removes
streams whose function dips below $NN$ and so denotes no version —
injectivity and domain,
respectively. Together they leave one accepted spelling per value,
and the argument is short
enough to give. For a dyadic step function $h$ — constant on each
piece of some partition of $[0, 1)$ into $2^r$ equal dyadic
intervals; the inductions below run on $r$ — define $T(h)$: a
single leaf if $h$ is constant, else the node over
$T(h|_"left")$ and $T(h|_"right")$. First, $T(h)$ satisfies the
sibling-merge rule: its two children are equal-height leaves only if
$h$ is constant on both halves — but then $h$ was constant and
$T(h)$ was a leaf; and since the children are $T(h|_"left")$ and
$T(h|_"right")$, the same holds at every node by induction. Second, any rule-satisfying tree for $h$ equals
$T(h)$, by induction from the root: a tree spelling a non-constant
$h$ cannot be a leaf, so it is a node whose children spell the two
restrictions (uniquely, by induction); and a tree spelling a
_constant_ $h$ must be a leaf, since an internal node's children
would spell two constant halves — by induction two equal leaves,
which the rule forbids. Heights are function-determined; gamma has
one spelling per natural and zigzag has none to spare. Uniqueness
is not an aesthetic: it is a load-bearing feature bought
deliberately. @compactness prices what it costs in coding room —
the
sibling-merge rule alone carries an asymptotic $4.3%$ worst-case
tax under this payload code (@tax shows the tax is code-dependent),
and the
other rules cost little or nothing. What uniqueness buys:

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

Minimal topology and exactness need two bits of state per open
ancestor — "is my left child complete?" and "was that child a
leaf?" — kept on a packed bit stack. That is roughly two bits of
transient memory per
level, against the roughly three bits a level costs on the
depth-maximizing spine shape. The anatomy of those three: per
level, an internal flag plus the
off-spine leaf's flag and a 1-bit zero-delta payload, with the
deepest sibling pair obliged to differ (one wider code at the
bottom keeps the shape canonical) — $3d + O(1)$ in all. @lengths
named the shape; it is the one that matters, since it is the one an
adversary sends. Depth is paid for
in bits, honoring @naive-recursion's budget.

Nonnegativity is the interesting one. It needs the running absolute
height, updated by every delta, sign-checked at every leaf — exactly
the "running value" @ladder showed to be dangerous. The boundary
comb aims straight at it: $plus.minus 1$ deltas, three-bit codes,
astride $2^k$, so a normalized running height pays a $k$-bit carry
per three-bit code. That is $Theta(n^2)$ work in an $n$-bit
stream — _in the
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
