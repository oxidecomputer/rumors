#import "fig.typ": *

= Introduction

A causal clock answers the question wall-clock time cannot: given two
updates made on different machines, did one _know about_ the other, or
did they happen concurrently? Interval Tree Clocks (ITCs) answer it in
a setting where participants come and go freely — any participant can
_fork_ a new identity out of its own, and a departing participant can
_join_ its identity and history back into a survivor — with no global
coordination and no registry of who exists. The 2008 paper by Almeida,
Baquero, and Fonte defines the mechanism as a pair of small recursive
tree structures and a handful of recursive equations over them. We
assume you have read that paper, or have it at hand; @model
re-establishes only the notation this document leans on.

This document is about what happens _after_ the paper: the distance
between equations that are correct and an implementation that is
efficient — and, more demandingly, an implementation whose efficiency
cannot be revoked by its inputs. The gap is wider than it first
appears. A faithful, node-for-node transcription of the paper's
equations inherits four independent cost defects, two of them
quadratic, and the quadratics are not exotic: inputs of a few
kilobytes trigger them at observable scale (@naive). A clock library
sits at a boundary where bytes arrive from other machines. It must
price its work against what actually arrives, not against what a
well-behaved peer would send.

The construction we develop answers with one representation and one
piece of arithmetic:

- *The skyline* (@skyline): a clock's event component denotes a step
  function over the unit interval. Store _that_ — the sequence of
  plateau heights, delta-coded, bit-packed, in one contiguous buffer —
  rather than the tree that spells it. The representation is canonical
  (one bit string per value, so byte equality _is_ semantic equality),
  compact (within $4.3%$ of the information-theoretic floor for the
  family it covers; @compactness), and sweepable: every operation the
  clock API asks for is computable in a bounded number of
  left-to-right passes — one for most operations, two where a
  lookahead or a measure's pre-pass is inherent.

- *The accumulator* (@accum): every sweep maintains a running signed
  integer — a running height, a running difference of heights, a
  running area. Delta coding makes those integers enormous while the
  deltas stay cheap, and ordinary big-integer arithmetic then leaks a
  quadratic through carry propagation. A redundant signed-digit
  accumulator with no normalized region anywhere makes every
  word-sized update and every sign query amortized constant-time, and
  every wide update linear in its own width, _on every input
  sequence_ — the load-bearing component that lets each sweep's cost
  argument close.

On top of those two, @operations derives each operation — comparison;
join and meet; fork, party join, and party difference; projection;
the measures rank, distance, lag, and minimum tick count; and last,
because it needs everything before it, tick (the paper's `event`,
with `fill` and `grow`) — as a sweep over the packed form, each with
an informal argument for linearity and a statement of what "linear"
is denominated in. @machine turns to constant factors: why a packed
sequential scan is the access pattern the machine rewards, and where
the measured costs of our implementation sit relative to the floor of
simply reading the input. @resilience closes the arc by stating the
property the whole design serves.

*The thesis.* The design is not merely asymptotically optimal, and
not merely efficient in its constants and friendly to the machine.
It is _resilient to arbitrary adverse inputs_: for every operation and
every well-formed input — any value magnitude, any tree depth, any
shape, crafted by an adversary or produced by an unlucky workload —
time and transient memory are proportional to the bits the operation
reads plus the bits it must write, with no caveats and no bounds on
the input. Malformed inputs are rejected, and rejection obeys the
same proportionality. Each section's cost argument is one clause of
that claim; the accumulator is the clause the others lean on.

*Provenance of claims.* Every cost statement in this document is one
of two kinds, and says which: _derived_ — an argument carried out here,
from the representation and the algorithm, which the reader can check;
or _measured_ — an observation from our implementation's instrumented
test and benchmark apparatus, quoted at the level of mechanism. The
setup behind every measured figure: one commodity 64-bit
workstation, release builds, deterministic committed input
generators, medians over repeated samples for wall-clock numbers;
the resource counters (bits scanned, accumulator digit touches, peak
transient bytes) are deterministic and machine-independent, so
nanosecond bands indicate a class while counter readings are exact.
Three
arguments have known boundaries, each stated where it lives rather
than smoothed over: one uncertified input shape in rank's funding
argument (@measures), one probabilistic step in the counting bound
(@nonneg), and one framing choice in what the compactness floor is
measured against (@ctf-caveat).

*What this document does not cover.* The library around this design
has concerns this exposition deliberately omits: the API and its
safety rules (identity must be handled linearly — forked, never
duplicated), wire-format versioning and evolution, concurrency, and
the embedding protocol a clock library serves. Here there are only
values, operations, and costs.

== The paper's model, briefly <model>

An ITC _stamp_ is a pair $(i, e)$: an *id tree* $i$ naming the slice
of identity this participant owns, and an *event tree* $e$ recording
the events it knows about. Both are interpreted as functions on the
real interval $[0, 1)$ — the *id space* — by recursive halving:

$ i &::= 0 | 1 | (i_1, i_2) \
  e &::= n | (n, e_1, e_2) quad n in NN $

An id tree denotes a $\{0, 1\}$-valued function: $0$ owns nothing on
its interval, $1$ owns all of it, and $(i_1, i_2)$ splits the interval
in half, $i_1$ governing the left half and $i_2$ the right. An event
tree denotes an $NN$-valued function: a leaf $n$ is the constant $n$,
and $(n, e_1, e_2)$ is $n$ plus the two halves' functions — the node's
value _lifts_ its whole subtree.

The three operations:

- *fork* splits a stamp's id into two disjoint ids covering the same
  intervals, both keeping the event tree: two participants where there
  was one.
- *event* (which we will call *tick*) inflates the event function
  somewhere over the caller's own id — anywhere there will do, since a
  successor timestamp only has to dominate its predecessor, and
  disjointness guarantees nobody else writes that region. The paper
  exploits the freedom: `fill` raises owned regions to flatten the
  tree where it can, and `grow` performs the cheapest strict increment
  where it cannot.
- *join* merges two stamps: ids by pointwise sum (disjointness makes
  that a union), event trees by pointwise maximum.

Comparison is pointwise: $e_1 <= e_2$ iff the function of $e_1$ is
nowhere above the function of $e_2$. Two event trees with no
containing order are _concurrent_. The paper's operations lean on one
piece of notation this document reuses: the _lift_ $e arrow.t m$,
which adds $m$ to the root value of $e$ — so $n arrow.t m = n + m$
and $(n, e_1, e_2) arrow.t m = (n + m, e_1, e_2)$.

Finally, the paper keeps trees in a *normal form* — $(0,0)$ and
$(1,1)$ collapse in ids; in event trees, equal-leaf siblings collapse
and a common minimum is lifted into the parent — so that a function
has one preferred spelling and the operations can stay simple. Normal
forms will matter enormously in what follows: the entire canonicality
story of @skyline, and a measurable fraction of the coding's size
(@compactness), descend from this choice.

*Vocabulary.* From @skyline onward this document uses the working
names of the implementation rather than the paper's, because the
renaming carries a point of view: we write *version* for the paper's
event component (it is a causal timestamp — a value in its own right,
freely copied), *party* for the id component (the participant's
share of the id space), *clock* for the stamp $(i, e)$ (a party
paired with its current version), and *tick* for the paper's `event`
operation. "Tree" is reserved for the paper's spelling of these
values; the whole burden of @skyline is that the tree is not the
value.

*Symbols.* Recurring symbols, fixed here (a few letters do local
double duty; each such use is flagged where it occurs):

#figure(
  table(
    columns: (auto, 1fr),
    align: (left, left),
    stroke: 0.4pt + rgb("#999999"),
    inset: 5.5pt,
    [$n, m$], [packed bit lengths of an operation's operands (in
      @compactness, $n$ is a stream-length budget in bits)],
    [$d$, $d_i$], [tree depth; a leaf's depth],
    [$W$], [the bit width of a stored magnitude],
    [$h$, $h_i$], [an absolute plateau height; the running height],
    [$delta$], [a height difference between consecutive plateaus],
    [$D$], [a two-operand sweep's running difference $h_a - h_b$],
    [$a_i$], [the accumulator's signed digits (@accum)],
    [$ell$], [the word length of a wide operand],
    [$k$], [a construction's scale parameter (a cliff's width, a
      gamma bucket, an iteration count — local to each use)],
    [$t$], [double duty, flagged in place: a construction's tooth
      count; the watermark gap $h - m$ in @tick],
    [$S$], [a stream's maximum leaf depth],
  ),
  kind: table,
  caption: [Notation. Lengths are in bits unless bytes are named.],
) <fig-notation>

Everything in the paper is correct and complete as semantics. The rest
of this document treats it as a specification and asks: what does it
cost to run, and what _should_ it cost?
