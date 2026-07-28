#import "fig.typ": *

= Introduction

A causal clock answers the question wall-clock time cannot: given two
updates made on different machines, did one _know about_ the other, or
did they happen concurrently? Interval Tree Clocks (ITCs) answer it
in a setting where participants come and go freely, with no global
coordination and no registry of who exists: any participant can
_fork_ a new identity out of its own, and a departing participant
can _join_ its identity and history back into a survivor. The 2008
paper by Almeida, Baquero, and Fonte defines the mechanism as a pair
of small tree structures and a handful of recursive equations over
them. We
assume you have read that paper, or have it at hand; @model
re-establishes only the notation this document leans on.

This document is about what happens _after_ the paper: the distance
between equations that are correct and an implementation that is
efficient — and, more demandingly, an implementation whose efficiency
cannot be revoked by its inputs. The gap is wider than it first
appears. A faithful, node-for-node transcription of the paper's
equations inherits four independent cost defects, two of them
quadratic; a fifth amplifier surfaces only once those are repaired
(@ladder). The quadratics are not exotic. Inputs of tens of
kilobytes trigger them at observable scale, and a few kilobytes
already crash the transcription's stack (@naive). A clock library
sits at a boundary where bytes arrive from other machines. It must
price its work against what actually arrives, not against what a
well-behaved peer would send.

Our construction answers that demand with one representation and one
piece of arithmetic:

- *The skyline* (@skyline): a clock's event component denotes a step
  function over the unit interval. Store _that_, rather than the
  tree's interior numbers: the tree's shape as one flag bit per
  node, and the sequence of plateau heights — delta-coded,
  bit-packed, in one contiguous buffer. (The id component gets a
  related but distinct coding: the same step-function reading, one
  bit deep, with ownership carried by presence rather than by a
  stored value. See @id-coding.) The representation is canonical,
  compact, and sweepable. _Canonical_: one bit string per value, so
  byte equality _is_ semantic equality. _Compact_: worst case
  within $4.3%$ of the information-theoretic
  floor asymptotically and $6.7%$ at hundred-byte sizes, for the
  family it covers (@ctf fixes the framing, @ctf-caveat states its
  exposure). _Sweepable_: every operation the
  clock API asks for is computable in a bounded number of
  left-to-right passes — one for most operations, two where a
  lookahead is information-forced (tick) or where the one funded
  form we know needs a pre-pass (rank), and the sum of their parts
  for the composite measures.

- *The accumulator* (@accum): every sweep maintains a running signed
  integer — a running height, a running difference of heights, a
  running area. The values run enormous while delta coding keeps
  their codes cheap, and ordinary big-integer arithmetic then leaks a
  quadratic through carry propagation. A redundant signed-digit
  accumulator, with no normalized region anywhere, makes every
  word-sized update and every sign query amortized constant-time, and
  every wide update linear in its own width — _on every input
  sequence_. One restriction, stated and used in @accum: an
  accumulator that receives writes at power-of-two scales is
  never asked for its sign — it is written, then read out once at
  the end. The accumulator is load-bearing: every sweep's cost
  argument closes on it.

On top of those two, @operations derives every operation as a sweep
over the packed form, each with an informal argument for linearity
and a statement of what "linear" is denominated in. The operations,
in order: comparison; join and meet; fork, party join, party
difference, and the party predicates `covers` and `disjoint`;
projection; the measures rank, distance, lag, and minimum tick
count; and last, because it needs everything before it, tick (the
paper's `event`, with `fill` and `grow`). @machine turns to constant
factors: why a packed sequential scan is the access pattern the
machine rewards, and where the measured costs of our implementation
sit relative to the floor of simply reading the input. @resilience
states the property the whole design serves.

*The thesis.* The design is not merely asymptotically optimal, nor
merely efficient in its constants and friendly to the machine.
It is _resilient to arbitrary adverse inputs_: for every operation and
every well-formed input — any value magnitude, any tree depth, any
shape, crafted by an adversary or produced by an unlucky workload —
time and transient memory are proportional to the bits the operation
reads plus the bits it must write. Every known boundary of the
argument is stated where it lives; the *Concessions* paragraph below
collects them. Malformed inputs are rejected,
and rejection obeys the same proportionality. Each section's cost argument is one clause of
that claim; the accumulator is the clause the others lean on.

*Provenance of claims.* Every cost statement in this document is one
of three kinds, and says which: _derived_ — an argument carried out
here,
from the representation and the algorithm, which the reader can
check; _derived in our work_ — a bound whose full derivation outgrew
this exposition, its shape given here;
or _measured_ — an observation from our implementation's instrumented
test and benchmark apparatus, quoted at the level of mechanism. The
setup behind every measured figure: one commodity 64-bit
workstation, release builds, deterministic committed input
generators, medians over repeated samples for wall-clock numbers;
the resource counters (bits scanned, accumulator digit touches, peak
transient bytes) are deterministic and machine-independent, so
nanosecond bands indicate a class while counter readings are exact.

*Concessions.* Seven, each stated where it lives rather than
smoothed over: two arguments with stated boundaries — one
uncertified input shape in rank's funding argument (@measures), one
probabilistic step in the counting bound (@nonneg); one framing
every compactness claim must carry (@ctf-caveat); one machine
effect the linear bound absorbs rather than eliminates (@words);
one composition stated without proof — the minimum-tick floor's
induction over forked histories, join subadditivity inside it
(@measures); and two derivations whose full forms live in our work
with their shapes given here (the join size constant, @join; tick's
output bounds, @tick-output). @closing re-collects all seven.

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

- *fork* splits a stamp's id into two disjoint ids whose union is
  the original's region, both keeping the event tree: two
  participants where there was one.
- *event* (which we will call *tick*) inflates the event function
  somewhere over the caller's own id — anywhere there will do, since a
  successor timestamp only has to dominate its predecessor, and
  disjointness guarantees nobody else writes that region. The paper
  exploits the freedom: `fill` raises owned regions to flatten the
  tree where it can, and `grow` performs the cheapest strict
  increment where it cannot. Flattening includes lifting a
  wholly-owned region up to a filled sibling's minimum.
- *join* merges two stamps: ids by pointwise sum (disjointness makes
  that a union), event trees by pointwise maximum.

A system starts from the _seed_ stamp $(1, 0)$: one participant
owning the whole id space, no events yet.

Comparison is pointwise: $e_1 <= e_2$ iff the function of $e_1$ is
nowhere above the function of $e_2$. Two event trees are
_concurrent_ when neither is $<=$ the other. The paper's operations
lean on three pieces of notation this document reuses. The _lift_
$e arrow.t m$ adds $m$ to the root value of $e$:
$n arrow.t m = n + m$, and
$(n, e_1, e_2) arrow.t m = (n + m, e_1, e_2)$. $min(e)$ and
$max(e)$ are the extrema of $e$'s function over $e$'s own interval —
range quantities, which is what @tick must carry. And `norm` is
the paper's normalizing constructor.

Finally, the paper keeps trees in a *normal form*, so that a
function has one preferred spelling and the operations can stay
simple: $(0,0)$ and $(1,1)$ collapse in ids; in event trees,
equal-leaf siblings collapse and a common minimum is lifted into the
parent. Normal forms matter in what follows: the entire canonicality
story of @skyline, and a measurable fraction of the coding's size
(@compactness), descend from this choice.

*Vocabulary.* From @skyline onward this document uses the working
names of the implementation rather than the paper's, because the
renaming carries a point of view: we write *version* for the paper's
event component (it is a causal timestamp — a value in its own right,
freely copied), *party* for the id component (the participant's
share of the id space), *clock* for the stamp $(i, e)$ — the
value: a party paired with its current version — and
*tick* for the paper's `event`
operation. "ITC" names the scheme. "Tree" is reserved for the paper's spelling of these
values; the whole burden of @skyline is that the tree is not the
value.

*Symbols.* Recurring symbols, fixed here. The table covers symbols
whose scope crosses a subsection; strictly local ones (a carry $c$,
gamma's bucket index $b$) are defined
at use. A few letters do
double duty; each such use is flagged where it occurs:

#figure(
  table(
    columns: (auto, 1fr),
    align: (left, left),
    stroke: 0.4pt + rgb("#999999"),
    inset: 5.5pt,
    [$n, m$], [packed bit lengths of an operation's operands (in
      @lengths, the bit length of one encoded input; in
      @compactness, a stream-length budget in bits)],
    [$d$, $d_i$], [tree depth; a leaf's depth],
    [$W$], [the bit width of a stored magnitude],
    [$h$, $h_i$], [an absolute plateau height; the running height],
    [$delta$], [a height difference between consecutive plateaus],
    [$D$], [a two-operand sweep's running difference $h_a - h_b$
      (@cmp)],
    [$a_i$], [the accumulator's signed digits (@accum)],
    [$ell$], [the word length of a wide operand],
    [$k$], [a construction's scale parameter (a cliff's width, a
      gamma bucket, an iteration count — local to each use)],
    [$w$], [a width in magnitude bits: a construction's tooth width
      (@two-zone), a delta's magnitude-bit count (@sign), a family's
      payload width (@ctf-caveat)],
    [$v$], [a coded payload value (@coding); also a version, as in
      $"rank"(v)$ — context separates them],
    [$a$, $b$], [two operand versions (@operations; distinct from
      the accumulator's digits $a_i$)],
    [$p$, $q$ (again)], [two parties (@id-ops; $q$ is also a
      digit's would-be value, local to @redundant)],
    [$s$], [a scaled write's power-of-two exponent
      (@accum-contract)],
    [$t$], [double duty, flagged in place: a construction's tooth
      count; the watermark gap $h - m$ in @tick],
    [$ell$ (again)], [in @compactness only: a walk's step count and
      a family's plateau count],
    [$S$], [a stream's maximum leaf depth],
    [$F$, $L$], [the frozen/live split of a running height,
      $h = F + L$; $Delta F_j$ is a freeze's evicted drift
      (@measures)],
    [$Phi$], [the funding potential: held lanes across live
      accumulators (@funding)],
    [$mu(x)$, $M(v)$], [a subtree's skyline minimum; the minimum-tick
      measure (@measures)],
    [$f$], [a domination floor's digit index (@sign)],
    [$r$], [the watermark stack's parked boundary difference
      (@tick-web)],
    [$alpha$], [the canonical grammar's growth exponent
      (@compactness)],
    [$n$ (again)], [in @model and @naive only, the paper's own
      grammar uses $n, n_1, n_2$ for event-node values],
    [$m$ (again)], [in @model, a lift amount; in @tick, a range
      minimum ($m_0, m_1, dots$)],
  ),
  kind: table,
  caption: [Notation. Lengths are in bits unless bytes are named.],
) <fig-notation>

Everything in the paper is correct and complete as semantics. The rest
of this document treats it as a specification and asks: what does it
cost to run, and what _should_ it cost?
