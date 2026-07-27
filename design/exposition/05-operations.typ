#import "fig.typ": *

= The operations, each as a funded sweep <operations>

Notation for the rest of the document: $n$ and $m$ are the packed bit
lengths of an operation's operands, and "linear" with no further
qualifier means $O(n + m)$ total work — scan, arithmetic, and
transient allocation all included — with the arithmetic bounds
amortized in the sense of @funding. Where an operation's _mandatory
output_ can exceed any constant multiple of its input, we say so and
denominate against input plus output; a bound that ignored mandatory
output would be unsatisfiable by construction, and quietly exempting
it is how cost claims rot.

One idea carries the whole section. The skyline's leaves are plateaus
in left-to-right order, so any two operands can be walked _together_,
plateau by plateau, and every operation the API asks is some fold over
that shared walk. The walk is @sweep; everything after it is a payload
bolted onto it.

And one sentence fixes what "asymptotically optimal" means
throughout. Every operation here answers a whole-value question whose
verdict can still change at its operands' final codes (the coding is
self-delimiting, so nothing short of parsing reveals even where the
value ends), so no algorithm can beat one full pass in the worst
case; a bounded number of linear passes is therefore optimal up to
constants, and the early-exit walks — comparison, the predicates —
beat the floor on favorable inputs without owing anything on
unfavorable ones.

== The overlay walk <sweep>

Two skylines partition $[0, 1)$ differently. Overlaying the two
partitions yields the _elementary intervals_: the maximal spans
crossing no plateau boundary of either operand. On each elementary
interval both operands are constant, so every pointwise question —
$<=$, max, min, masking — is answered interval by interval.

#figure(
  overlay(
    ((0.5, 1), (0.5, 0)),
    ((0.75, 0), (0.25, 2)),
    ticks: ((0.0, [0]), (0.5, [½]), (0.75, [¾]), (1.0, [1])),
    label-a: [$a = (0,1,0)$],
    label-b: [$b = (0,0,(0,0,2))$],
  ),
  caption: [Two skylines and their overlay. Plateau boundaries at ½
    (from both operands — $b$'s is a zero step between two equal
    plateaus, invisible in the drawing) and at ¾ (from $b$) cut
    $[0,1)$ into three elementary intervals; on each, both operands
    are constant. The join $a or b$ takes the pointwise max per
    interval: heights $1, 0, 2$ — the skyline of @fig-skyline.],
) <fig-overlay>

The walk holds one leaf _cursor_ per operand — the current plateau,
with the path of ancestor turns that locates it, kept as one bit per
level on a packed stack — and advances whichever cursor's plateau ends
first. The delicate point is deciding "ends first" without arithmetic:
an interval's endpoint is a dyadic rational $d$ bits wide, and
comparing endpoints numerically at every boundary would be quadratic
on deep operands. Three facts about dyadic intervals eliminate the
numbers entirely:

+ *Overlapping dyadic intervals nest.* Both cursors' plateaus contain
  the current sweep point, so they overlap — hence the deeper one is
  contained in the shallower, and the deeper one ends first or ties.
  _Rule: advance the deeper cursor; at equal depths the intervals
  coincide, so advance both._
+ *Ties are visible on the path.* (Depth increases downward from the
  root; "shallower" means a smaller depth.) Advancing a cursor pops
  the trailing "right child" levels off its path and flips the
  deepest "left child" level to right. The deeper interval's end
  coincides with the shallower's exactly when the flipped level's
  depth is at most the shallower cursor's depth — a stack operation,
  not a comparison of positions.
+ *The all-right path is the exhausted stream.* A leaf whose path is
  all right turns is the last plateau, ending at 1. Canonical
  operands therefore exhaust exactly together, which is both the loop
  condition and a free structural check.

Each topology bit of either stream is read at most once, each path bit
is pushed and popped at most once, and each payload is decoded exactly
once. The walk is linear in $n + m$ unconditionally — before any
payload arithmetic is even mentioned.

== Comparison <cmp>

Causal comparison must decide $a <= b$, $b <= a$, both (equality), or
neither (concurrency) — pointwise questions over the overlay. The
sweep maintains a single running signed difference on an accumulator:

$ D = h_a - h_b, $

updated at each boundary by folding the advancing side's _delta
codes_ — the numbers the stream itself supplies, at the width the
stream itself paid for — and consulted once per elementary interval
for its sign. No absolute height is ever reconstructed. A sign
$D > 0$ somewhere refutes $a <= b$; $D < 0$ somewhere refutes
$b <= a$; the four verdicts are the four subsets of refutations
accumulated by the end, and each entry point stops early the moment
its own question is settled (equality dies at the first nonzero
sign; an order dies at the first sign against it; full
`compare` runs until both directions have spoken or the streams
end).

Cost: the walk is linear; each word-scale delta folds in amortized
$O(1)$ digit touches, each wide delta in $O$(its own limbs); each
per-interval sign read is amortized $O(1)$ by collapse (@sign). The
boundary comb — the family that broke every normalized design —
funds each of its three-bit teeth with $O(1)$ touches. Comparison is
linear on every input, and its transient state is two path-bit stacks
and one accumulator: comparing against a deep operand costs its
_bits_, not frames.

Contrast this with the direct transcription's `leq` (@naive): same
verdict, same recursion scheme even — but there the per-node quantity
was an absolute path sum, and here it is one shared difference whose
updates are the input's own deltas. The entire cure is _which quantity
the walk maintains_.

== Join and meet <join>

Join is pointwise max, meet pointwise min; both ride the identical
walk and differ in one branch. What emission adds to comparison is an
output: one plateau per elementary interval, at the depth of the
deeper cursor (nesting makes that the interval's exact width), with a
payload delta the walk must produce _without knowing any absolute
height_.

Per elementary interval the output equals one operand — the _side_:
$a$ where $D > 0$, $b$ where $D < 0$, unchanged (both agree) where
$D = 0$, for max; the mirror for min. Two cases at each boundary:

- *Same side.* The output moves with its side, so the output delta
  _is_ that side's own step delta — zero if the boundary belonged to
  the other stream alone. Cost: the code already read.
- *Switch.* The output jumps from the old side's plateau to the new
  side's. With $D'$ the difference _after_ this boundary's folds and
  $delta$ the old side's step at this boundary (zero if it did not
  step), the jump is $+D' + delta$ when switching to $a$, and
  $-D' + delta$ when switching to $b$: one sign-and-magnitude read of
  the accumulator plus one signed add. (Check, switching to $a$:
  the jump is $h_a^+ - h_b^-$, and
  $D' + delta = (h_a^+ - h_b^+) + (h_b^+ - h_b^-)$ telescopes to
  exactly that.) A switch means $D$ crossed or left zero at this
  boundary, so $|D'|$ is bounded by the deltas just folded — the read
  is priced by the codes that carried them, and the accumulator's
  collapse (@sign) has already flattened any cancelling prefix by the
  time the magnitude is taken.

Here is the whole machine run once, on @fig-overlay's operands —
$a = (0, 1, 0)$, a 9-bit stream, and $b = (0, 0, (0, 0, 2))$, a
12-bit stream:

#figure(
  table(
    columns: (auto, auto, auto, auto, 1fr),
    align: (left, center, center, center, left),
    stroke: 0.4pt + gray-line,
    inset: 5.5pt,
    table.header([*interval*], [*folds*], [$D$ *after*], [*side*],
      [*emitted plateau*]),
    [$[0, 1\/2)$], [$h_a = 1$, $h_b = 0$ (the absolutes)], [$+1$],
    [$a$],
    [depth 1, absolute $1$ (code `010`)],
    [$[1\/2, 3\/4)$], [$delta_a = -1$, $delta_b = 0$ (tie: both
      advanced)], [$0$], [$a$ (sticky)],
    [depth 2, same side: $delta = delta_a = -1$ (code `010`)],
    [$[3\/4, 1)$], [$delta_b = +2$ ($b$ deeper: $b$ alone)], [$-2$],
    [switch to $b$],
    [depth 2, jump $= -D' + delta_a = +2$ (code `00101`)],
  ),
  kind: image,
  caption: [The join of @fig-overlay's operands, boundary by
    boundary. The emitted plateaus — depths $1, 2, 2$, payloads
    $1, -1, +2$ — assemble into exactly @fig-stream's sixteen bits:
    $(0, 1, (0, 0, 2))$, within the size bound
    $16 <= 9 + 12 - 2$.],
) <fig-worked-join>

The output must itself be canonical, and it is made canonical _while
streaming_: emitted plateaus feed a _collapsing builder_ that derives
the output topology from the depth sequence and merges equal sibling
leaves the moment the second of a pair completes. A merge can cascade
upward, but only along the ancestors still open on the output's right
edge, so the builder's state is a couple of bit stacks bounded by
depth, and every merge removes bits that were already paid for by
their emission. The result is born in normal form — there is no
normalization pass anywhere in the system. To see a cascade, run the
_meet_ of the same operands: pointwise min gives height $0$ on all
three intervals, so the builder receives equal leaves at depths
$1, 2, 2$; the two depth-2 siblings merge into a depth-1 leaf, which
now equals _its_ pending sibling and merges again — the output is the
single leaf $0$, two bits, born canonical.

Cost: the walk is linear; folds are funded by input codes; switch
reads are funded by the boundary's own codes; each output code is
written once and truncated at most once. Join and meet are
$O(n + m)$, and the output is, too: a canonicality dividend worth
recording is that for canonical operands,

$ "size"(a or b) <= "size"(a) + "size"(b) - 2 "bits" $

(similarly for meet). The topology half of the reason is immediate —
the output's plateau boundaries are a subset of the operands'
together, and merging only shrinks the tree. The payload half needs
the boundary algebra above plus one code-length fact: gamma is nearly
subadditive,

$ "len"(gamma("of" v_1 + v_2)) <= "len"(gamma("of" v_1)) + "len"(gamma("of" v_2)) + 1 "bit," $

because $(v_1 + 1)(v_2 + 1) >= v_1 + v_2 + 1$ makes the sum's
logarithm at most the summands' logarithms together. A same-side
output delta _is_ an input delta at its own length; a switch's jump
is a signed sum of the deltas folded at its boundary, so its code
costs at most theirs plus a constant — and each boundary that emits
one output code _consumed_ its folded input codes, which never fund
another emission. Summed across the sweep the output's payload bits
are covered by the input's, constant slack per boundary absorbed by
the topology half's savings. (The sketch gives the shape; the exact
constant, tight at the empty pair, is verified mechanically in our
implementation as a structural bound over the coding.) Joins never blow up — the inequality is what lets a
system fold thousands of versions together with a predictable memory
ceiling.

== The party operations <id-ops>

The party side runs the same machinery with one-bit heights and no
accumulator at all; it is worth a moment, both because forks and
retirements are the dynamic half of the ITC story and because the
operations degenerate pleasantly.

*Fork* is the paper's `split`. In the pruned id coding, a node with
one present child names it directly, so the split walks the _spine_ —
the chain of one-child nodes — to the first node with both children
(or to a terminal), then builds both halves by splicing: each half is
the spine prefix, verbatim; a retag of the branch node keeping one
side; and that side's subtree, verbatim. A terminal splits as
$1 -> ((1,0), (0,1))$. One linear pass, at most one fresh node per
half, no descent into the kept subtrees at all — forking is cheap at
_any_ id shape, which a system that forks per request depends on.

*Party join* merges two disjoint id landscapes by union: a lockstep
walk; where one side is absent, the other's subtree is spliced
verbatim; where both descend, the walk descends with them; a node
whose two sides become wholly owned collapses to a terminal (the
coding's $(1,1) -> 1$). Linear, output subadditive.

*Party difference and the predicates.* The difference (given parties
$p$ and $q$, the regions of $p$ not owned by $q$ — how a share is
carved when something less than a full retirement moves) is the same
boolean-skyline sweep, emitting $1$ where $p$ owns and $q$ does not.
The predicates — _covers_ ($q$'s owned region $subset.eq$ $p$'s) and
_disjoint_ (no owned region shared, the safety condition every join
checks) — are lockstep verdict walks: no emission, no per-level
state at all, early exit at the first refuting position.

== Projection, and pricing by mandatory output <projection>

The projection $v \/ p$ masks a version's skyline to a party's owned
region: owned intervals keep their plateaus, unowned intervals emit
zero. The sweep is the overlay walk of $v$'s stream against $p$'s,
emitting through the collapsing builder; heights stay relative except
at ownership transitions, where the output re-enters the skyline at a
plateau's absolute height — and the emitted code at that transition
_is_ that absolute, so the work of materializing it is priced by the
output written.

Projection is the section's honest exception on denomination, and the
reason the preamble defined one. Take a comb version whose $t$ teeth
ride a $k$-bit base — cheap deltas, $Theta(t + k)$ bits — and a
scattered party owning every other tooth. Masking breaks every chain
of cheap deltas with an interleaved zero, so each kept tooth must
re-spell its full $k$-bit height: the _output_ is
$Theta(t dot k)$ bits from a $Theta(t + k)$-bit input — mandatory
given that the result must itself be a canonical stream (a lazy
"masked view" would dodge it, at the price of every downstream
consumer carrying the mask forever). No algorithm can beat its own
output; the honest claim, and the one our implementation is held to,
is that the sweep is linear in _input plus output_ — and that
nothing else in this section has an _unbounded_ output ratio: tick's
output can exceed its input by at most a constant factor
(@tick-output), join's not at all.

== The measures <measures>

*Rank.* The area under the skyline,

$ "rank"(v) = integral_0^1 h_v = sum_("leaves" i) h_i dot 2^(-d_i), $

is a dyadic rational, kept exact at any magnitude. Exactness buys a
guarantee sorting can rely on: if $v < w$ causally then
$h_v <= h_w$ pointwise with the two functions unequal, so they
differ on some plateau — which has positive width — and the exact
integrals differ strictly: $"rank"(v) < "rank"(w)$, with ties
possible only between concurrent versions. Ranks give any store a
total order extending causality (tiebreak concurrent values however
you like — say, by canonical bytes, which uniqueness makes a
legitimate tiebreak).

Computing it is a one-cursor sweep with a weighted fold: add
$h_i dot 2^(S - d_i)$ per leaf (numerator units, $S$ the maximum
depth, found by one topology-only pre-pass — rank is one of the
two-pass operations the introduction owned up to). The naive fold
materializes $h_i$ per leaf — the boundary comb makes that
$Theta(n dot k)$ again. The cure splits the running height into
_frozen + live_, $h = F + L$: $L$, an accumulator holding the drift
since the last _freeze_; $F$, the rest, touched only when a freeze
evicts $L$ into it. Per leaf the fold adds only $L$'s digits —
bounded by the codes folded since the freeze, hence funded — using
the scaled add of @accum-contract (requirement 2) to land them at
the leaf's weight in one funded pass. $F$'s contribution enters by
summation by parts:

$ sum_i F_i dot "mass"_i = F_"final" dot 2^S - sum_("freezes" j) Delta F_j dot ("prefix mass before" j), $

one wide shifted add at the end, plus one correction per freeze
priced by the drift $Delta F_j$ being evicted — which is exactly what
the codes since the previous freeze paid for. The trigger is
concrete: a freeze fires when a folded delta finds $L$ more than a
fixed allowance of digits wider than that delta's own code — the
signature of stale wide drift about to ride under cheap codes. The
two properties claimed follow directly: a firing freeze evicts drift
the earlier, wider codes funded (they made $L$ wide); and bounded
oscillation at _any_ width keeps $L$ within its own codes' width
plus the allowance, so it never freezes at all.

One uncertified case, stated rather than smoothed — the first of the
three boundaries the introduction announced. Each freeze correction
also multiplies by its freeze _position_ (the prefix mass), and the
funding argument certifies the product only where the position's
signed-digit form compacts to $O(1)$ digits — which it does on every
adversarial family we construct or have seen (comb positions are
ones-runs, two signed digits). A stream engineered to re-arm wide
drift under cheap codes at maximally dense positions sits outside the
certified argument. Measured behavior there is still held by an
enforced ceiling — accumulator digit touches per packed input byte,
pinned flat across size doublings on the committed families — but the
derivation above does not cover the shape, and we say so.

*Distance and lag* need no machinery of their own — the lattice
already paid for them:

$ "distance"(a, b) &= integral |h_a - h_b| &&= "rank"(a or b) - "rank"(a and b), \
  "lag"(a, b) &= integral (h_b - h_a)^+ &&= "rank"(a or b) - "rank"(a), $

each a difference of linear pieces (the pointwise identities
$max - min = |a - b|$ and $max - a = (b - a)^+$, integrated).
Distance is a metric on versions — a genuine one, not a
pseudometric: symmetry and the triangle inequality are inherited
pointwise from $|dot|$, and distance zero forces equality because
distinct versions denote distinct functions (@canonical's uniqueness
argument), which then differ over some plateau of positive width. Lag is the one-sided
"how much of $b$ have I not seen", the natural backpressure signal
for anti-entropy protocols. Note the pass count these identities buy
the composites: two emissions and two rank folds, every piece linear.

*Minimum ticks.* The fewest tick operations that could have produced
$v$, over all fork/tick/join histories, equals the sum of the
normalized event tree's node values. The sum has a pleasant skyline
form: writing $mu(x)$ for the _absolute_ minimum of the skyline over
node $x$'s interval — so $mu("leaf") = h$ — the normalized base at
$x$ is $mu(x) - mu("parent"(x))$, and the sum telescopes:

$ sum_x "base"(x) = sum_("leaves") h - sum_("internal") mu =: M(v). $

Both directions of the identity deserve their sketch, since an API's
meaning rests on it. _Floor_: $M$ is a measure on versions that
forks preserve (both halves keep the event component), that joins
are subadditive in, and that a single tick raises by at most one.
The last is where to look closely. `grow`'s increment raises one
leaf term by one, and enclosing minima can only rise, which
subtracts: at most $+1$. A `fill` collapse replaces a subtree's
contribution to $M$ — itself at least the subtree's maximum, since
building any function costs at least its tallest point — by exactly
that maximum: never an increase. And a `fill` raise lifts an owned
plateau _exactly to the adjacent filled minimum_, so the leaf term
and the enclosing node's minimum term rise by the same amount and
cancel. No tick raises $M$ by more than one, so any history reaching
$v$ spends at least $M(v)$ ticks. _Achieved_, by mirroring the
telescoping rather than the leaves: at each node from the root down,
tick the whole currently-owned region $mu(x) - mu("parent"(x))$
times, then fork into the two children — the counts spent are
exactly the normalized bases, summing to $M(v)$. (Derived in our
work; the join-subadditivity step is the one to check if you check
one.)

The sweep folds subtree minima with one machine word per open
ancestor, saturating: the operation's contract is _exact below
$2^64$, clamped at $2^64 - 1$ above_ — semantically comfortable
because a count beyond $2^64$ is beyond any history a system will
ever run, and callers use the clamp as a magnitude. Saturation is
also why words suffice: any single height beyond word range forces
the clamped answer immediately (the count dominates every leaf
height), so the fold exits early rather than escalating to wide
arithmetic. This is the one walk in the system that pays a machine
word rather than bits per open ancestor — eight bytes against the
level's roughly three input bits, a constant near twenty, linear and
priced, kept because the fold's values are words by construction. Not every fold needs the accumulator; the funding
question is asked per fold, and here the honest answer is "a word
suffices."

== Tick: `fill`, `grow`, and the watermark web <tick>

The tick is the paper's `event`: run `fill`, keep its result if it
changed anything, else apply `grow`'s cheapest strict increment.
It is the one operation whose walk is genuinely intricate, because
`fill`'s equations ask _range_ questions — maxima and minima over
subtrees on both sides of the cursor — while everything before this
point asked only pointwise ones. This subsection derives the walk in
five stages — the semantics, the walk and its lookahead, the
watermark web that carries its range minima, the fused decision
between `fill` and `grow`, and the output bound that closes the
funding ledger — and it is the payoff of the whole apparatus.

=== The semantics, restated on the skyline <tick-semantics>

The paper's `fill` is worth having at hand verbatim; it is short, and
@tick-walk maps each arm to a stream action:

$ "fill"(0, e) &= e \
  "fill"(1, e) &= max(e) \
  "fill"(i, n) &= n \
  "fill"((1, i_r), (n, e_l, e_r)) &= "norm"((n, max(max(e_l), min(e'_r)), e'_r)), quad e'_r = "fill"(i_r, e_r) \
  "fill"((i_l, 1), (n, e_l, e_r)) &= "norm"((n, e'_l, max(max(e_r), min(e'_l)))), quad e'_l = "fill"(i_l, e_l) \
  "fill"((i_l, i_r), (n, e_l, e_r)) &= "norm"((n, "fill"(i_l, e_l), "fill"(i_r, e_r))) $

In skyline terms: `fill` flattens what the caller owns — every event
subtree the id wholly owns collapses to one plateau at that subtree's
maximum (the second arm) — and the two _shortcut arms_ (fourth and
fifth) additionally let a wholly-owned, freshly collapsed child rise
to the minimum of its _filled_ sibling, if that is higher. The rise
is the profitable move: it merges plateaus across the sibling
boundary and lets ancestors collapse in turn. The cap at the
sibling's minimum is _not_ a safety constraint — over its own region
a participant may inflate as far as it likes (@model: any inflation
over the id is a legal successor) — it is parsimony, the paper's own
"does not dominate more events than needed" desideratum, with a
size rationale behind it: raising exactly to the sibling's minimum
is the largest raise that still merges plateaus; raising past it
would claim events no observation forced _and_ re-split what it just
merged.

=== The walk, and its one lookahead <tick-walk>

Pair the id cursor against the version cursor, and emit through the
same collapsing builder as join:

#algo(title: [fill, as a sweep])[
  at each paired position: \
  #h(1.2em) *unowned* (id absent — arm 1): copy the event subtree's
    bits verbatim, after re-coding its first payload against the last
    plateau emitted (one boundary code; everything after it is
    delta-coded against neighbors inside the copy, which the mask
    does not disturb). \
  #h(1.2em) *wholly owned* (id terminal — arm 2): consume the event
    subtree, folding its running max; emit one plateau at that max. \
  #h(1.2em) *event already flat* (id node over an event leaf —
    arm 3): emit the plateau unchanged; the id's finer structure
    below has nothing left to flatten. This is the steady-state arm:
    a region a participant keeps ticking is flat after the first
    fill. \
  #h(1.2em) *mixed* (both sides descend — arms 4–6): descend in
    stream order, iteratively, pushing two bits of suspended state
    per level; on a shortcut arm (4, 5), raise the owned child's
    plateau to the sibling's filled minimum where that exceeds its
    max. \
]

Two of the three range quantities are easy. A wholly-owned range's
max folds as it is consumed (word-or-wide adds, funded by the codes
read). Verbatim copies cost their own bits. The delicate quantity is
the shortcut arms' _sibling minimum_ — and the two arms differ in a
way the stream order makes vivid:

- *Right-full arm* ($(i_l, 1)$): the raised leaf is the _right_
  child's output, emitted after the left child's range has already
  been walked. Its minimum is a fact about output the walk has
  already produced — a _watermark_. The walk keeps it as it goes:
  no lookahead, no second pass.
- *Left-full arm* ($(1, i_r)$): the raised leaf is emitted _before_
  the range its minimum comes from. The walk must look ahead: a
  _pre-scan_ of the right sibling's range computes the minimum
  `fill` will emit there. Done naively per arm, nested left-full
  sites re-scan shared suffixes quadratically; the pre-scan is
  therefore _memoized_ — one fresh scan per uncovered range, with
  every interior left-full site's minimum recorded on the way, so no
  stream position is ever pre-scanned twice. The walk's total read
  budget is at most two passes per position, flat — tick is the other
  two-pass operation the introduction owned up to. The memo's
  _memory_ obeys the same ledger as its reads: at most one recorded
  entry per left-full site (so no more entries than id bits), each
  held as a bounded difference against a reference the walk still
  holds when the entry is consumed — never an absolute — so $k$
  nested sites sharing one wide minimum store its width once, not
  $k$ times.

=== The watermark web <tick-web>

Open ranges nest, so the walk needs a LIFO stack of range minima
over one running height — and the naive spelling, one absolute
minimum per open range, is the path-sum defect reborn: wide value,
deep nesting, $Theta(d dot W)$. The cure is the same move that cured
comparison, applied one structure up: _store differences, not
absolutes_.

#figure(
  {
    let w = 250pt
    let unit = 17pt
    let h = 6.4 * unit
    box(width: w + 120pt, height: h + 30pt, {
      // staircase trace: running height across the walked prefix
      let pts = ((0.00, 5), (0.10, 5), (0.10, 3), (0.24, 3), (0.24, 4),
                 (0.38, 4), (0.38, 1.6), (0.54, 1.6), (0.54, 2.6),
                 (0.70, 2.6), (0.70, 3.4), (0.86, 3.4))
      for k in range(pts.len() - 1) {
        let (x1, y1) = pts.at(k)
        let (x2, y2) = pts.at(k + 1)
        place(top + left, dx: 12pt + x1 * w, dy: h - y1 * unit,
          line(start: (0pt, 0pt), end: ((x2 - x1) * w, (y1 - y2) * unit),
            stroke: 1.4pt + ink))
      }
      // current height marker
      place(top + left, dx: 12pt + 0.86 * w - 2.4pt, dy: h - 3.4 * unit - 2.4pt,
        circle(radius: 2.4pt, fill: ink))
      place(top + left, dx: 18pt + 0.86 * w, dy: h - 3.4 * unit - 5pt,
        text(size: 8.5pt, fill: ink, [$h$ (running height)]))
      // watermark dashed lines: m2 (innermost) at 1.6, m1 at 1.6 (zero diff), m0 at 1.0
      let wm(y, x0, lab, col) = {
        place(top + left, dx: 12pt + x0 * w, dy: h - y * unit,
          line(length: (1.0 - x0) * w + 42pt,
            stroke: (paint: col, thickness: 0.8pt, dash: "dashed")))
        place(top + left, dx: 12pt + w + 46pt, dy: h - y * unit - 5pt,
          text(size: 8pt, fill: col, lab))
      }
      wm(1.6, 0.30, [$m_1 = m_2$ (zero-run pair)], accent)
      wm(1.0, 0.06, [$m_0$], accent.darken(25%))
      // brace-like range markers under the axis
      let brk(x0, x1, lab, dyy) = {
        place(top + left, dx: 12pt + x0 * w, dy: h + dyy,
          line(length: (x1 - x0) * w, stroke: 0.8pt + gray-line.darken(30%)))
        place(top + left, dx: 12pt + x0 * w, dy: h + dyy - 2.5pt,
          line(angle: 90deg, length: 2.5pt, stroke: 0.8pt + gray-line.darken(30%)))
        place(top + left, dx: 12pt + x1 * w, dy: h + dyy - 2.5pt,
          line(angle: 90deg, length: 2.5pt, stroke: 0.8pt + gray-line.darken(30%)))
        place(top + left, dx: 12pt + x0 * w, dy: h + dyy + 3pt,
          box(width: (x1 - x0) * w, align(center,
            text(size: 7.5pt, fill: gray-line.darken(40%), lab))))
      }
      brk(0.06, 0.998, [range 0 open], 8pt)
      brk(0.30, 0.998, [range 1 open], 17.5pt)
      brk(0.44, 0.998, [range 2 open (innermost)], 27pt)
    })
  },
  caption: [The watermark web, schematically. Three ranges are open;
    each needs the minimum of the output emitted since it opened.
    Minima nest ($m_0 <= m_1 <= m_2$), so the walk stores the running
    height $h$, the innermost gap $t = h - m_2$, and the chain of
    differences outward — with runs of _zero_ differences (here
    $m_1 = m_2$) held as one counted entry. Nothing wide is stored
    twice.],
) <fig-watermark>

The walk stores: the running height $h$ (one accumulator); the
innermost watermark as the nonnegative gap $t = h - m$; and each
enclosing watermark as a nonnegative difference outward — with runs
of zero differences compressed to one counted entry. Now walk the
costs through @funding's three funding sources:

- *A consumed delta* folds into $h$ and $t$ — two accumulators, a
  constant — at the delta's own width. The outer differences do not
  move at all: differences between minima are invariant under a
  shift of the current height. This invariance is the reason
  differences are the right coordinates.
- *An emitted plateau at or above the innermost minimum* is one
  amortized sign read of $t$.
- *An emitted plateau _below_ the innermost minimum* — an
  _undercut_ — must lower some suffix of the watermark chain. It
  walks outward: each nonzero difference it fully penetrates _dies_
  (folded once into the running residue — the $Phi$ drop pays);
  each zero _run_ passes in $O(1)$, wholesale, because every frame
  in the run shares the inner minimum and is updated implicitly;
  and at the first difference it cannot penetrate, one surviving
  fold, priced by the undercut's own delta — whose code the input
  just paid. Without run compression this cascade is
  $Theta("open depth")$ per undercut — the *descending staircase*
  (@families): unit steps, one full-penetration undercut per level,
  $Theta(d^2)$ total; with it, each undercut costs its dying
  differences plus $O(1)$. (Both halves measured: the uncompressed
  form's quadratic on the staircase is reproducible, and the
  compressed form holds every family flat.)
- *A range close* pops the innermost difference and _moves_ it
  aside into a one-slot register, rather than folding it into the
  revealed watermark. The walk's invariant weakens by exactly that
  slot: with $r$ the parked difference, the revealed range's true
  gap is $t + r$, resolved lazily — the next opener recycles $r$
  into its own boundary difference (a move, then a narrow
  anchor-relative adjustment); a second close before an open folds
  the narrower of the two parked differences into the wider (a
  death, funded, which is why one slot suffices); an undercut deep
  enough to reach $r$ annihilates it (a death again); and
  comparisons against the composite gap go through the domination
  floor, deciding from top digits without touching $r$'s width.
  The distinction between move and fold is not pedantry:
  fold-on-close re-folds a wide boundary difference on every
  close/reopen cycle, and a comb of $k$ sibling sites sharing one
  $2^b$-scale minimum — the *reveal comb*, the last of @families'
  constructions — then circulates that width $k$ times with no
  input or output code funding any crossing: a genuine
  $Theta(k dot b)$ amplifier on $Theta(k + b)$ input, found by
  adversarial construction against an earlier design of this very
  walk, and cured by making the close a move. Moves are free;
  deaths pay; nothing is read twice at width.

The same difference discipline runs inside the memoized pre-scan, as
@tick-walk already noted for its memory; the pre-scan is a second
instance of the web, not a second mechanism.

=== The changed flag, and the fused `grow` <tick-fusion>

`tick` needs to know whether `fill` changed anything, and the walk
decides it in-pass: the flag trips at the first emitted plateau that
differs from the input plateau it replaces. Until then the output
would be a bit-identical prefix of the input (nothing earlier has
changed, so no copy needs its boundary payload re-coded) — and
nothing is built; the first divergence copies the matched prefix
wholesale and emission continues from there. A walk that never
diverges has _built nothing at all_, and hands its second product to
`grow`: riding along the same walk, at every id branch node, a fold
has recorded which child hosts the cheaper inflation — fewest node
expansions, then shallowest depth, which is the paper's `grow` cost
function read lexicographically (its "large constant $N$" per
expansion is exactly a lexicographic order, as the paper itself
notes), with ties to the right child as in the paper's final
equation — one direction bit per id branch. `grow` then _splices_:
copy everything up to the inflation point verbatim; re-code the
grown leaf's payload ($+1$); repair the one successor delta that the
change can reach; emit an expansion chain's fresh leaves as
$0 slash plus.minus 1$ codes; copy the suffix verbatim. One walk
plus at most one splice, both linear.

Can the increment break canonicality — raise a leaf into equality
with its sibling, demanding a merge the splice does not perform?
Belt and braces: the spliced output still flows through the
collapsing builder, so any merge the increment makes reachable _is_
performed; and on this branch the case is vacuous anyway — a
wholly-owned leaf sitting one below an equal-able sibling is exactly
what `fill`'s shortcut arm would have raised, so the flag would have
tripped and `grow` would never have run.

The hottest pattern in any real deployment — a participant ticking
its own version again and again — takes exactly this branch: its
own region is already flat, `fill` changes nothing, the flag never
trips, and the tick is two skims plus a splice that re-codes exactly
two payloads — the grown leaf's and its successor's. The
adversarial machinery and the common case land on the same fast
path; that coincidence is the fusion's whole justification.

=== Two ticks, worked <tick-worked>

The machinery deserves the same concrete treatment join got
(@fig-worked-join). Two tiny traces, one per branch, both against
the party $p = (1, 0)$ — owner of the left half, the four-bit stream
of @id-coding.

*The grow branch* — the hot path. Tick @fig-stream's value,
$v = (0, 1, (0, 0, 2))$. The walk pairs the streams: the left child
is wholly owned and already one plateau (height 1, its max); the arm
asks whether raising it to the right sibling's filled minimum would
merge anything, and the pre-scan answers: the right range's minimum
is 0, the owned max 1 already dominates it, no raise. Nothing else
is owned; every emitted plateau equals its input; the flag never
trips, so nothing was built. The route fold recorded one direction
bit (the cheapest inflation lives down the id's left branch:
$"grow"(1, 1) = 2$, a bare increment, no expansion). The splice then
copies the topology verbatim and re-codes exactly two payloads: the
grown leaf's absolute, $1 -> 2$ (`010` $->$ `011`), and its
successor's delta, $-1 -> -2$ (`010` $->$ `00100`); the final code
(`00101`) copies untouched. Result:
$"tick"(v) = (0, 2, (0, 0, 2))$, eighteen bits, built by two skims
and two re-coded codes — and the reader can check every bit against
@coding's rules.

*The fill branch* — a raise, a cascade, and the flag. Tick
$v = (0, 0, 2)$ instead (plateaus $0, 2$; nine bits: flags `011`,
payloads `1` and `00101`). The owned left plateau's max is 0; the
pre-scan reports the right range's minimum as 2; the arm raises the
left plateau to $max(0, 2) = 2$ — the raise-to-minimum move,
recording the event by flattening. The first emitted plateau (height
2) differs from the input's (height 0): the flag trips, and emission
becomes real. The builder receives equal plateaus at depths $1, 1$,
merges the sibling pair, and the whole version collapses to the
single leaf $2$ — four bits (`1` then $gamma(3) = mono("011")$).
One tick took a nine-bit version to a four-bit one: `fill` is why
skylines shrink under use, and the flag is how the walk knew the
flattening itself had recorded the event.

=== The output side of the ledger <tick-output>

@funding lets emitted codes fund work, so tick's cost bound is only
as good as a bound on what tick emits. Two facts close the loop,
both derived and then pinned by enforced tests in our
implementation. First, the one-step bound:

$ "size"("tick"(e, i)) <= 2 dot "size"(e) + 4 dot "size"(i) + 32 "bits." $

Each term has a mechanism. The factor 2 on the event side is real
and tight in kind: a raise can re-code one delta against a wide
neighbor, duplicating one input code's width once — and because a
single code can be nearly the entire stream, "one code duplicated"
_is_ a doubling. That is exactly how an earlier, stronger conjecture
died: we first believed the bound was additive
($"size"(e) + O(dots)$), and a constructed counterexample — one wide
code, duplicated — refuted it, leaving the multiplicative form as
the honest survivor. The id term covers `grow`'s expansion chain, which
descends along the id adding a constant number of flag and
$0 slash plus.minus 1$ payload bits per id level; the additive 32
absorbs the first leaf's absolute code and byte-boundary slack.

Second, the growth does not compound. In closed form, for $k$
iterated ticks against the same party:

$ "size"("tick"^k (e, i)) <= "size"("tick"(e, i)) + 4 dot "size"(i) + 4 ceil(log_2 (k + 1)) + 8 "bits" $

— after the first tick's possible doubling, everything further is
logarithmic in $k$: the doubling is a one-step transient, not a
ratchet a peer could crank.

Together: every code tick emits is priced by codes tick read, up to
constants, and the funded-sweep bound $O(n + m)$ — scan bits, digit
touches, and transient heap alike — holds unconditionally. This is
the strongest single cost statement in the system, and the one the
direct transcription misses by a full polynomial degree — on more
currencies at once (scan, arithmetic, and transient memory) than any
other operation.
