#import "fig.typ": *

= The operations, each as a funded sweep <operations>

One idea carries the whole section. The skyline's leaves are plateaus
in left-to-right order, so any two operands can be walked _together_,
plateau by plateau, and every operation the API asks for is some fold
over that shared walk. The walk is @sweep; everything after it is a
payload folded along it.

Notation for the rest of the document: $n$ and $m$ are the packed bit
lengths of an operation's operands, and "linear" with no further
qualifier means $O(n + m)$ total work — scan, arithmetic, and
transient allocation all included — with the arithmetic bounds
amortized in the sense of @funding. Where an operation's _mandatory
output_ can exceed any constant multiple of its input, we say so and
denominate against input plus output; a bound that ignored mandatory
output would be unsatisfiable by construction, and quietly exempting
it is how cost claims rot.

What "asymptotically optimal" means here: every operation answers a
whole-value question whose verdict can still change at its operands'
final codes — the coding is self-delimiting, so nothing short of
parsing reveals even where the value ends. No algorithm can beat one
full pass in the worst case, so a bounded number of linear passes is
optimal up to constants. Each second pass is argued where it occurs:
tick's lookahead is information-forced, rank's pre-pass is argued
against its alternative, and @projection argues the one
output-denominated operation. The early-exit walks — comparison, the
predicates — beat the floor on favorable inputs without owing
anything on unfavorable ones. One family of operations answers to a
_higher_ floor: the exact area measures (@measures), whose answer
can embed an arbitrary integer product, so the honest yardstick
there is the cost of one multiplication — proven mandatory by
construction — and the walk's own traffic stays linear around it.

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

The walk holds one leaf _cursor_ per operand and advances whichever
cursor's plateau ends first. A cursor is the current plateau plus
the path of ancestor turns that locates it, kept as one bit per
level on a packed stack.
The delicate point is deciding "ends first" without arithmetic:
an interval's endpoint is a dyadic rational $d$ bits wide, and
comparing endpoints numerically at every boundary would be quadratic
on deep operands. Throughout, depth increases downward from the
root; _shallower_ means a smaller depth. Three facts about dyadic
intervals eliminate the
numbers entirely:

+ *Overlapping dyadic intervals nest.* Both cursors' plateaus contain
  the current sweep point, so they overlap — hence the deeper one is
  contained in the shallower, and the deeper one ends first or ties.
  _Rule: advance the deeper cursor; at equal depths the intervals
  coincide, so advance both._
+ *Ties are visible on the path.* Advancing a cursor pops
  the trailing "right child" levels off its path, flips the
  deepest "left child" level to right, then descends leftmost to
  the next leaf, pushing one path bit per level of descent. The deeper interval's end
  coincides with the shallower's exactly when the flipped level's
  depth is at most the shallower cursor's depth — a stack operation,
  not a comparison of positions — and when it fires, the shallower
  cursor advances too: its plateau ends at the same point. The test
  works because the deeper interval ends at the right endpoint of
  its highest all-right ancestor — the flipped level. If that level
  sits at or above the shallower cursor's depth, the ancestor
  contains the shallower plateau, so its right endpoint is at least
  the shallower's, while nesting forces at most: hence equal. The
  flip can sit well _above_ the shallower cursor — two plateaus
  sharing an end at a coarse boundary — which is why the test is
  $<=$ and not equality; and fact 1's equal-depth advance is the
  same rule's degenerate case.
+ *An all-right path means the stream is exhausted.* A leaf reached
  by right turns only is the last plateau, ending at 1. Complete
  operands therefore exhaust exactly together (completeness, not
  canonicality, is what this needs — every well-formed stream
  denotes a total function on $[0, 1)$), which is both the loop
  condition and a free structural check.

Each topology bit of either stream is read at most once, each
descent pushes one path bit and each advance pops it — one push per
internal node of either stream — and each payload is decoded exactly
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
for its sign. No absolute height is ever materialized: the two
leading absolutes fold straight into $D$, and nothing above word
width is ever reassembled. A sign
$D > 0$ somewhere refutes $a <= b$; $D < 0$ somewhere refutes
$b <= a$. The four verdicts are the four subsets of refutations
accumulated by the end. Each entry point stops early the moment
its own question is settled: an order dies at the first sign
against it, the _equal_ verdict inside a full compare dies at the
first nonzero sign, and full
`compare` runs until both directions have spoken or the streams
end. (A standalone equality test never runs this walk at all —
@canonical made it a byte comparison.)

Cost: the walk is linear; each word-scale delta folds in amortized
$O(1)$ digit touches, each wide delta in $O$(its own limbs); each
per-interval sign read is amortized $O(1)$ by collapse — every delta
the walk folds is unscaled, so @sign's charge lands on the codes
just consumed. The
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

Join is pointwise max, meet pointwise min; both ride the same
walk and differ in one branch. Join adds one thing to comparison: it
emits. One plateau per elementary interval, at the depth of the
deeper cursor (nesting makes that the interval's exact width), with a
payload delta the walk must produce _without knowing any absolute
height_. The leading plateau is the one seeding step:
both leading codes are read, the side the operation selects is
emitted (max's larger, min's smaller), $D$ starts from
their difference, and no absolute survives past it.

Per elementary interval the output equals one operand; call that
operand the _side_. For max, the side is $a$ where $D > 0$ and $b$
where $D < 0$; for min, the mirror.
At $D = 0$ the side is _sticky_ — the walk keeps whatever it already
held, an arbitrary choice that costs nothing,
since at $D = 0$ the operands agree and either side's step yields
the same output (@fig-worked-join's middle row shows the case). Two
cases at each boundary:

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
    [$[3\/4, 1)$], [$delta_b = +2$ ($b$ deeper: $b$ alone;
      $delta_a = 0$)], [$-2$],
    [switch to $b$],
    [depth 2, jump $= -D' + delta_a = -(-2) + 0 = +2$ (code
      `00101`)],
  ),
  kind: image,
  caption: [The join of @fig-overlay's operands, boundary by
    boundary. The emitted plateaus — depths $1, 2, 2$, payloads
    $1, -1, +2$ — assemble into exactly @fig-stream's sixteen bits:
    $(0, 1, (0, 0, 2))$, within the size bound
    $16 <= 9 + 12 - 2$.],
) <fig-worked-join>

The output must itself be canonical, and the _collapsing builder_
makes it so _while
streaming_: emitted plateaus feed it, and it derives
the output topology from the depth sequence and merges equal sibling
leaves the moment the second of a pair completes. A merge can cascade
upward, but only along the ancestors still open on the output's right
edge, so the builder's state is a couple of bit stacks bounded by
depth, and every merge removes bits that were already paid for by
their emission. (The output buffer itself obeys the ledger: it grows
geometrically, so appends are amortized $O(1)$ per emitted bit and
the transient peak stays within a constant factor of the output's
own size.) The result is born in
normal form — there is no
normalization pass anywhere in the system. To see a cascade, run the
_meet_ of the same operands: pointwise min gives height $0$ on all
three intervals, so the builder receives equal leaves at depths
$1, 2, 2$; the two depth-2 siblings merge into a depth-1 leaf, which
now equals _its_ pending sibling and merges again — the output is the
single leaf $0$, two bits, born canonical.

#figure(
  attack(
    [alternating pair$(t)$],
    [the emission walk's side switches],
    stack(dir: ttb, spacing: 4pt,
      overlay(
        ((0.125, 3), (0.125, 1), (0.125, 4), (0.125, 2),
         (0.125, 3), (0.125, 1), (0.125, 4), (0.125, 2)),
        ((0.125, 1), (0.125, 3), (0.125, 2), (0.125, 4),
         (0.125, 1), (0.125, 3), (0.125, 2), (0.125, 4)),
        w: 210pt, unit: 10pt, label-a: $a$, label-b: $b$,
      ),
      text(size: 7.5pt, fill: gray-line.darken(40%),
        [two organically forked-and-ticked versions whose dominance
         alternates at every one of the $t - 1$ overlay boundaries
         — join and meet alike, with no boundary ever collapsing]),
    ),
    [a side switch at every boundary: the maximum possible density
     of the one emission step that must read the running
     difference's sign and magnitude.],
    cure: [each switch's read is priced by the boundary's own
      folded codes ($|D'|$ bounded by the deltas just folded, the
      collapse having flattened any cancelling prefix), so maximal
      density buys no amplification — the walk measures flat.],
  ),
  kind: image,
  caption: [The alternating pair attack card: switch density at its
    ceiling, aimed at the jump read.],
) <fig-attack-alternating>

Cost: the walk is linear; folds are funded by input codes; switch
reads are funded by the boundary's own codes; each emitted code is
written once, and a merge cascade deletes at least two codes for
each one it writes, so the builder's total writes stay within a
constant factor of the emissions. Join and meet are
$O(n + m)$, and so is the output. Uniqueness pays a dividend here —
for canonical operands,

$ "size"(a or b) <= "size"(a) + "size"(b) - 2 "bits" $

(similarly for meet). The topology half of the reason is immediate —
the output's plateau boundaries are a subset of the operands'
together, and merging only shrinks the tree; the guaranteed deficit
also rides this half, visible at the seam where the overlay spells
the two operands' leading plateaus as one — a leaf flag saved, and
the shorter leading absolute's code absorbed by the longer's. The payload half rests
on the boundary algebra above plus one code-length fact: gamma is
nearly subadditive,

$ "len"(gamma(v_1 + v_2 + 1)) <= "len"(gamma(v_1 + 1)) + "len"(gamma(v_2 + 1)) + 1 "bit" $

for naturals, because
$(v_1 + 1)(v_2 + 1) >= v_1 + v_2 + 1$ makes the sum's
logarithm at most the summands' logarithms together — the signed
deltas are what add at a boundary, and their zigzags obey the same
bound with a constant's further slack. A same-side
output delta _is_ an input delta at its own length. A switch's jump
is a signed sum of the deltas folded at its own boundary — codes the
switch consumed, at least one of them nonzero and hence at least
three bits, and none of them funding another
emission. So the jump's code costs at most theirs plus a few bits'
slack, the zigzag re-fold contributing a constant of its own. The
topology half absorbs the per-boundary slack: boundaries
both operands paid for are spelled once, and merges delete flags —
savings the full count shows outweigh the slack. What we
offer here is that shape, not a per-boundary ledger closing to the
exact constant. The inequality's status, plainly: sketched here;
_derived_ in full
in our work — the derivation is longer than this section wants —
with the $-2$ additionally _attained_ in property tests across
roughly
1.5 million generated operand pairs. Joins never blow up — the inequality is what lets a
system fold thousands of versions together with a predictable memory
ceiling.

One more design point closes the lattice story: the _n-ary_ folds.
Joining or meeting a whole population is not a loop over the binary
operation in arrival order; it is a *balanced reduction*, pairing
operands of similar accumulated weight so that every input passes
through $O(log k)$ two-operand steps — $O(D log k)$ time and $O(D)$
space over $k$ operands of $D$ total packed bits. The sequential
left-to-right fold it replaces has a genuine quadratic in each
direction, and both refuting populations are ordinary values. For
the meet: an accumulator's _value_ shrinks with every step, but its
_packed size_ need not — one deep operand among $k$ shallow operands
that all dominate it leaves the running meet byte-identical to the
deep operand at every step, and a sequential fold re-walks it once
per operand, $Theta(k dot d)$ on a $Theta(k + d)$-bit population.
For the join, the mirror: single-plateau operands ordered so that no
two consecutive arrivals are adjacent in the id space keep the
accumulator's plateaus from ever coalescing, and the sequential fold
re-walks the accumulated result per arrival. The balanced tree walks
any such stubborn operand once per _level_ instead of once per
operand; its intermediate results can legitimately swell toward the
sum of their inputs' sizes at every level (a population of mutually
interleaved teeth achieves exactly that), which is the $log k$
factor's honest content, not an accident to engineer away. The party
side runs the same reduction with one further obligation: the n-ary
party union is _fallible_ — an operand no disjoint placement can
absorb is rejected and handed back, dropping nothing. Rejection has
two doors, each priced without re-probing the accumulated group: an
operand overlapping the host fails an up-front test that walks the
operand once against an index of the host built once per call (one
logarithmic table search per node both sides own), and an operand
aliasing its fellows surfaces later, as one failed combine inside
the counter — priced at the combine the reduction was running
anyway, its overlap witness at the bottom of the operands' shared
path.

#figure(
  attack(
    [shaded carrier$(d, k)$],
    [the n-ary meet's accumulator],
    stack(dir: ttb, spacing: 5pt,
      oprow([carrier], codestrip((
        ([a $d$-level dense version, $Theta(d)$ bits], 150pt, "t"),
      ))),
      oprow([$k - 1$ shades], codestrip((
        ([flat 3], 34pt, "p"), ([flat 3], 34pt, "p"),
        ([$dots.c$], 14pt, "x"), ([flat 3], 34pt, "p"),
      ))),
      text(size: 7.5pt, fill: gray-line.darken(40%),
        [every shade dominates the carrier everywhere, so the
         running meet stays byte-identical to the carrier at every
         step — never equal to a shade, never empty]),
    ),
    [a sequential reduce re-walks the whole carrier per shade:
     $Theta(k dot d)$ on a $Theta(k + d)$-bit population, with no
     short-circuit available.],
    cure: [the balanced reduction: the carrier is walked once per
      counter level, $O(d log k + k)$ — the committed sequential
      form survives as the laws' value oracle and as the adequacy
      tripwire that keeps this cell's verdict live.],
  ),
  kind: image,
  caption: [The shaded carrier attack card: a value that never
    shrinks, aimed at fold orders that re-walk their
    accumulator.],
) <fig-attack-shade>

#figure(
  attack(
    [staggered teeth$(k, m)$],
    [the balanced reduction's intermediate results],
    stack(dir: ttb, spacing: 5pt,
      oprow([operand 1], skyline(((0.06, 1), (0.19, 0), (0.06, 1), (0.19, 0), (0.06, 1), (0.19, 0), (0.06, 1), (0.19, 0)), w: 170pt, unit: 9pt, show-heights: false)),
      oprow([operand 2], skyline(((0.125, 0), (0.06, 1), (0.19, 0), (0.06, 1), (0.19, 0), (0.06, 1), (0.19, 0), (0.06, 1), (0.065, 0)), w: 170pt, unit: 9pt, show-heights: false)),
      text(size: 7.5pt, fill: gray-line.darken(40%),
        [$k$ operands of $m$ unit teeth each, every operand's teeth
         in the gaps of every other's, fed in an order that pairs
         the least-overlapping groups first at every level]),
    ),
    [every internal merge, at every level, swells to near the sum
     of its inputs' sizes — no coalescing until the last level: the
     declared $O(D log k)$ model's worst case, realized in full.],
    cure: [nothing to defeat — the family gauges the model itself:
      the measured per-level cost holds flat across joint doublings
      of $k$ and $m$, so the log factor is the reduction's honest
      price, not a leak. Kin gauge the single axes: single-tick
      operands ordered never to coalesce (arity), and interleaved
      region sets under one shared skeleton (operand size).],
  ),
  kind: image,
  caption: [The staggered teeth attack card: the joint
    arity-times-size loading of the balanced fold, with its
    single-axis kin in the cure line.],
) <fig-attack-stagger>

== The party operations <id-ops>

The party side runs the same machinery with one-bit heights and no
accumulator at all; it is worth a moment, both because forks and
retirements are the dynamic half of the ITC story and because the
operations degenerate pleasantly.

One restatement first, because the pruned coding stores no leaf
where a party owns nothing, and @sweep was derived for streams that
spell every plateau. A party cursor _synthesizes_ the absent child
its parent's tag declares: an unowned plateau spanning exactly that
child's half, costing no bits and funded by the tag that declared
it. The cursor thus presents a total $0$-or-$1$ function on
$[0, 1)$, plateau by dyadic plateau, and @sweep's three facts apply
unchanged; even exhaustion agrees — a party stream ends when
its obligation count dies, the same point the synthesized plateaus'
all-right path reaches. Every walk
below, and every version-against-party overlay later (@projection,
@tick), pairs cursors on this footing.

*Fork* is the paper's `split`. In the pruned id coding, a node with
one present child names it directly, so the split walks the _spine_ —
the chain of one-child nodes — to the first node with both children
(or to a terminal), then builds both halves by splicing: each half is
the spine prefix verbatim, then the branch node retagged to keep one
side, then that side's subtree verbatim. A terminal splits into the
pair $((1,0), (0,1))$. One linear pass, at most one fresh node per
half, and no _re-coding_ of the kept subtrees: each is delimited by
the obligation counter (the predicates below) and copied at the
cost of its bits.
Forking is therefore cheap at
_any_ id shape — which is what a system that forks per request
depends on.

*Party join* merges two disjoint id landscapes by union, in one
lockstep
walk: where one side is absent, the other's subtree is spliced
verbatim; where both descend, the walk descends with them; and a
node
whose two sides become wholly owned collapses to a terminal (the
coding's $(1,1) -> 1$). Linear, output subadditive.

*Party difference and the predicates.* The difference of parties
$p$ and $q$ — the regions of $p$ not owned by $q$ — is the same
boolean-skyline sweep, emitting $1$ where $p$ owns and $q$ does
not. It is how a share is carved when something less than a full
retirement moves.
When $"covers"(q, p)$ holds — $q$ owning all of $p$ — the result is
empty. The API reports that as
_no party_ rather than spelling it, since a party is nonempty by
construction (@id-coding); `covers` is therefore the caller's test
before
carving.
Two predicates ride the same walk: $"covers"(p, q)$ asks whether
$q$'s owned region $subset.eq$ $p$'s, and
_disjoint_ asks whether no owned region is shared — the safety
condition every join
checks. Both are lockstep verdict walks: no emission, and two bits
of suspended state per _queued right pair_ — the presence tags the
walk parks while it finishes the two left children — on one packed
stack; a right pair neither side stores queues nothing at all, so
a lockstep chain of unary nodes keeps the stack empty at any
depth. Where one side is absent, the other's whole
subtree is skipped by its own stream's counter (@coding's
device transposed: each stored node contributes its child count
minus one) rather than by
anything stacked per level. Early exit at the first refuting
position.

#figure(
  attack(
    [lockstep pair$(d)$],
    [the party predicates' per-level state],
    stack(dir: ttb, spacing: 5pt,
      oprow([party $p$], codestrip((
        ([node], 26pt, "t"), ([node], 26pt, "t"), ([$dots.c$], 14pt, "x"),
        ([node], 26pt, "t"), ([owned], 34pt, "t"),
      ))),
      oprow([party $q$], codestrip((
        ([node], 26pt, "t"), ([node], 26pt, "t"), ([$dots.c$], 14pt, "x"),
        ([node], 26pt, "t"), ([owned], 34pt, "t"),
      ))),
      text(size: 7.5pt, fill: gray-line.darken(40%),
        [two spines descending in full lockstep for $d$ levels,
         diverging only at the bottom: every level is a paired
         descent both walks must suspend]),
    ),
    [any predicate that suspends a word per paired level pays
     $Theta(d)$ words of transient state against $Theta(d)$ _bits_
     of operand.],
    cure: [two presence bits per queued right pair on a packed
      stack — and a unary lockstep chain queues nothing, riding an
      empty stack at any depth — with subtree skips priced by the
      skipped stream's own counter.],
  ),
  kind: image,
  caption: [The lockstep pair attack card: maximal paired depth,
    aimed at per-level suspension in the verdict walks.],
) <fig-attack-lockstep>

#figure(
  attack(
    [aliased shares$(k, d)$],
    [the n-ary party union's rejection path],
    stack(dir: ttb, spacing: 5pt,
      oprow([host], codestrip((
        ([owns everything but one depth-$d$ fragment], 170pt, "t"),
      ))),
      oprow([$k$ operands], codestrip((
        ([fragment], 48pt, "t"), ([fragment], 48pt, "t"),
        ([$dots.c$], 14pt, "x"), ([fragment], 48pt, "t"),
      ))),
      text(size: 7.5pt, fill: gray-line.darken(40%),
        [$k$ byte-identical spellings of one deep fragment against
         a host owning the rest: exactly one completes the region,
         and every other must be rejected and handed back]),
    ),
    [a rejection path that re-probes the accumulated group per
     rejected operand pays the group's size $k$ times; each
     operand's overlap witness sits at the bottom of its own
     $d$-level path, so even the honest walk is forced to
     $Theta(k dot d)$ — the floor the meter's liveness check
     pins.],
    cure: [each rejection pays its own path plus one failed
      lowest-weight combine, never a re-probe: flat per population
      byte across an arity doubling, and within the reduction's
      declared log factor across a depth doubling.],
  ),
  kind: image,
  caption: [The aliased shares attack card: mass rejection, aimed
    at hand-back paths that touch the accumulated group.],
) <fig-attack-alias>

== Projection: pricing by mandatory output <projection>

The projection $v \/ p$ masks a version's skyline to a party's owned
region: owned intervals keep their plateaus, unowned intervals emit
zero. Callers use it to ask what a share itself has witnessed: the
slice of history a participant's own region vouches for — the
question behind auditing a share's contribution, and behind
splitting responsibility when shares move.
The sweep is the overlay walk of $v$'s stream against $p$'s,
emitting through the collapsing builder. Heights stay relative
except
at ownership transitions, where the output re-enters the skyline at a
plateau's absolute height — and the emitted code at that transition
_is_ that absolute. So the sweep's running height materializes once
per
transition (requirement 4, @accum-contract): a wide read priced
bit-for-bit by the wide code being written.

Projection is the one operation whose output term is not slack — the
reason the preamble denominated against output at all. Take a comb version whose $t$ teeth
ride a $k$-bit base — cheap deltas, $Theta(t + k)$ bits — and a
scattered party owning every other tooth. Masking breaks every chain
of cheap deltas with an interleaved zero, so each kept tooth must
re-spell its full $k$-bit height: the _output_ is
$Theta(t dot k)$ bits from a $Theta(t + k)$-bit input — mandatory
given that the result must itself be a canonical stream. No
algorithm can beat its own output — so the design makes sure a
caller pays it only when a stored value is actually wanted. The
projection ships in two forms. The primary form is a _view_: a
borrowed pair (version, party), built in $O(1)$, that never spells
the masked skyline at all. Comparing a view — against a version, or
against another view — runs as one fused co-walk over the three or
four operand streams, the masks folded into the comparison sweep's
running difference on the fly, priced by the operands' packed sizes
and never by the projection's; equality on views is semantic, not
byte-level, precisely because no canonical bytes exist to compare.
Materializing the projection as a value is a second, explicit
operation — the emitting sweep above — and the only one that pays
the output term. The correlated family that stresses the fused walk
interleaves the amplifiers of everything before: a comb whose teeth
oscillate across a carry cliff, under a mask owning every other
tooth, compared against a wide flat plateau — every ownership
toggle alternates the walk between a sign read of a near-cancelled
wide difference and a zero test of a masked height, each operand
harmless alone. The accumulator's collapse and domination floors
answer every such read in amortized $O(1)$; nothing is
materialized. So we claim only what holds, and hold the
implementation to it: the materializing sweep is linear in _input
plus output_, the view's comparisons are linear in input alone, and
nothing else in this section has an _unbounded_ output ratio.
Fork's halves each stay within their operand's size plus two bits
(the fresh node), and the pair within twice that plus four; the
seed's two bits forking to two four-bit halves is where the
additive term binds. Party difference stays within its operands'
sum; tick's
output can exceed its input by at most a constant factor
(@tick-output); join's cannot exceed it at all.

#figure(
  attack(
    [scattered-party comb$(t, k)$],
    [projection — through its mandatory output],
    stack(dir: ttb, spacing: 5pt,
      oprow([version], skyline(
        ((0.125, 2), (0.125, 1), (0.125, 2), (0.125, 1),
         (0.125, 2), (0.125, 1), (0.125, 2), (0.125, 1)),
        w: 170pt, unit: 10pt, show-heights: false)),
      oprow([party], codestrip((
        ([own], 21pt, "t"), ([—], 21pt, "x"), ([own], 21pt, "t"),
        ([—], 21pt, "x"), ([own], 21pt, "t"), ([—], 21pt, "x"),
        ([own], 21pt, "t"), ([—], 21pt, "x"),
      ))),
      text(size: 7.5pt, fill: gray-line.darken(40%),
        [a comb of $t$ teeth riding a $k$-bit base — cheap deltas —
         under a party owning every other tooth: masking breaks
         every chain of cheap deltas with a zero]),
    ),
    [every kept tooth re-enters the skyline at its full $k$-bit
     absolute: $Theta(t dot k)$ bits of _mandatory_ output from a
     $Theta(t + k)$-bit input.],
    cure: [no algorithm beats its own output, so the design splits
      the operation: comparisons run through the lazy view at the
      operands' size, and the materializing sweep is held linear in
      input _plus_ output — the honest denominator.],
  ),
  kind: image,
  caption: [The scattered-party comb attack card: the one unbounded
    output ratio in the system, aimed at any claim that forgot to
    denominate output.],
) <fig-attack-scattered>

#figure(
  attack(
    [masked drift$(t, k)$],
    [the view's fused masked comparison],
    stack(dir: ttb, spacing: 5pt,
      oprow([version], skyline(
        ((0.125, 2), (0.125, 1), (0.125, 2), (0.125, 1),
         (0.125, 2), (0.125, 1), (0.125, 2), (0.125, 1)),
        w: 170pt, unit: 10pt, show-heights: false)),
      oprow([mask], codestrip((
        ([own], 21pt, "t"), ([—], 21pt, "x"), ([own], 21pt, "t"),
        ([—], 21pt, "x"), ([own], 21pt, "t"), ([—], 21pt, "x"),
        ([own], 21pt, "t"), ([—], 21pt, "x"),
      ))),
      oprow([against], skyline(((1.0, 2),), w: 170pt, unit: 10pt,
        show-heights: false)),
      text(size: 7.5pt, fill: gray-line.darken(40%),
        [the comb's teeth oscillate across a $2^k$ carry cliff
         behind 3-bit codes; the flat operand stands at $2^k$; the
         mask toggles at every tooth]),
    ),
    [every toggle alternates the walk between the sign of a
     near-cancelled wide difference and a zero test of a masked
     height — each operand harmless alone, the composition aiming
     the carry cliff at both reads at once.],
    cure: [the collapse and the domination floors answer both reads
      in amortized $O(1)$ (@sign); nothing is materialized, and the
      verdict walk stays linear in the three operands. A
      four-stream variant — two view-against-view masks with
      interleaved parities — lands on the same machinery.],
  ),
  kind: image,
  caption: [The masked drift attack card: correlated operands
    aiming an old amplifier at the fused three-stream walk.],
) <fig-attack-maskdrift>

== The measures <measures>

Four measures ride the same skyline reading: rank is its area,
distance and lag are areas of a difference, and the minimum tick
count is a sum of leaf heights less subtree minima. The area
measures carry this section's deepest machinery — and its one
honestly superlinear operation, priced against a floor proven by
construction.

=== Rank

The area under the skyline,

$ "rank"(v) = integral_0^1 h_v = sum_("leaves" i) h_i dot 2^(-d_i), $

is a dyadic rational, kept exact at any magnitude. Exactness buys a
guarantee sorting can rely on: if $v < w$ causally then
$h_v <= h_w$ pointwise with the two functions unequal, so they
differ on some plateau — which has positive width — and the exact
integrals differ strictly: $"rank"(v) < "rank"(w)$, with ties
possible only between concurrent versions. Ranks give any store a
total order extending causality (break ties among concurrent values
however you like — say, by canonical bytes, which uniqueness makes
legitimate).

Computing it is a one-cursor sweep with a weighted fold: add
$h_i dot 2^(S - d_i)$ per leaf, in numerator units, with $S$ the
maximum depth. One pre-pass finds $S$, reading flags and hopping
over payloads by their coded lengths — rank is one of the two-pass
operations the introduction owned up to. The pre-pass earns its
keep: the
alternative anchors the total at the running maximum depth and
rescales the held digits whenever it rises, a held-width rewrite
per rise that a stream carrying width down every level turns
quadratic — a one-leaf-per-depth staircase makes the running
numerator as wide as the depth already walked at _every_ level.
Knowing $S$ up front makes every landing final. The naive fold
materializes $h_i$ per leaf — the boundary comb makes that
$Theta(n dot k)$ again. The cure splits the integrand into three
_anchored_ components, $h = B + P + L$, and the split's governing
rule is worth stating before its parts: *no correction, at any
point of the sweep or its close, multiplies by an absolute
position.* Positions grow arbitrarily dense while the codes at hand
stay cheap — a spine that plants isolated position bits a full
digit apart buys, with topology alone, absolute positions whose
signed-digit spelling no code ever funded — so a design that
settles evicted drift against "the mass before this point" has a
quadratic waiting in it. The components:

- $L$ (_live_): the drift since the last _freeze_. Each elementary
  interval adds $L$'s digits at the interval's weight — the scaled
  add of requirement 2 (@accum-contract), into a running total that
  stays write-only until the sweep's single closing materialization
  (@sign) — and the add is bounded by the width of the delta folded
  at the previous boundary plus a fixed allowance. The freeze
  trigger keeps that true: a freeze fires when a folded delta finds
  $L$ wider than that delta's own code by more than the allowance —
  the signature of stale wide drift about to ride under cheap
  codes. Bounded oscillation at _any_ width stays within its own
  codes' width plus the allowance and never freezes.
- $B$ (_base_): the opening plateau, anchored at position zero,
  closing in one shifted add $B dot 2^S$.
- $P$ (_parked_): drift a freeze moved out of $L$, anchored _at
  that freeze_. A segment-mass accumulator sums the interval masses
  since $P$'s anchor, and the next freeze (or the stream's end)
  settles $P dot "segment"$ in one product and re-anchors. The
  anchoring is the point: the segment mass's nonzero digits span
  only the _depth variation inside the segment_ — the dyadic
  positions' shared prefix never appears in it — so a parked crest
  settled a boundary later costs $P$'s width times $O(1)$ digits
  however dense the absolute position is. The segment lives in the
  scaled write-only mode and is read out once, at its own written
  span (@accum-contract's watermark read), never at its scale.

Two further pieces complete the machinery. First, a gate: the
segment feed _opens at the first freeze_. Segment mass exists only
to settle parked drift, and no drift precedes the first freeze, so
a sweep that never freezes — word-scale heights, the practical
regime — deposits no interval mass at all and pays nothing for the
settle machinery's existence. Second, a ledger: when incoming
drift runs more than the allowance _narrower_ than $P$, settling
$P$ per freeze would re-read its full width against every later
narrow-drift freeze, so $P$ is instead _promoted_ — recorded once,
with the window of interval mass banked since the previous
promotion, as one ledger entry; two funded-width reads, no
product. An entry's debt is $P dot (2^S - "position")$, which the
window decomposition turns into cross terms: entry $i$ owes
$P_i dot w_j$ against every later window $w_j$. The ledger settles
once, at the sweep's close, as one product tree over the entry
sequence, balanced by _mass_ (parked digits plus window density,
not entry count): each node contributes exactly one aggregate
product — the left half's summed parked components times the right
half's summed windows, parked sums folded digit-wise so opposing
armings cancel before any product reads a width, window sums held
as sparse balanced signed digits so a long climb's consumed mass
compacts to $O(1)$ terms — and each product is delegated,
cluster-wise, to sub-quadratic integer multiplication. Every cross
term rides exactly one aggregate product; no width or density is
re-read more times than its node's depth, which the mass balance
keeps logarithmic; and the geometric shrink of node masses down the
tree telescopes the products' costs into the root's.

What may all this cost? Three statements, each with its status. The
walk's own traffic — scan, decode, folds, the per-freeze and
per-promotion reads — is linear in the packed input, derived
through @funding, and streams whose parked drifts stay a bounded
number of digits wide (every committed adversarial family) run in
$O(n log n)$ total. The settle products are the honest exception,
and they are _mandatory_: a version of $Theta("bits"(x) +
"bits"(y))$ stored bits can embed the product of two arbitrary
integers in its exact rank. Put the plateau $x$ over a spine whose
right turns spell the mass $2y$ bit by bit — each turn's interval
mass is one set bit of $2y$ — and the exact numerator is
$2 x y + 1$: any fold that answers exactly has multiplied two
input-funded factors at linear overhead, so $Omega(M(n))$ digit
work is mandatory, with $M$ the integer-multiplication bound of
the arithmetic backend. The matching upper bound: the settle runs
in $O(M(n))$ whenever the products land in a power-law tier of the
backend's multiplication — cluster splitting keeps every densified
span funded, and the mass balance telescopes the tree — which
covers every input whose packed size is under roughly 64 kilobytes
(no product's smaller side clears the backend's quasilinear
threshold, near 32 kilobytes per side, before that) and every
input of any size that arms the ledger $O(1)$ times. Past that
tier the per-level products stop telescoping and the settle pays
at most one extra tree-depth factor, $O(M(n) dot log n)$ — and the
log factor is tight there (derived; the witness family needs
operands past 65 kilobytes, too large to sit in a committed test).
The gap between $O(M(n) dot log n)$ and $Omega(M(n))$ is not
contractual: a deeper mechanism change may close the tree-depth
factor, and nothing may beat one multiplication on the
answer-embedding inputs. Stating the bound in terms of $M$ is
deliberate: it is stable under backend swaps, and it names the one
primitive doing the superlinear work.

The machinery above was not designed in one sitting; each piece is
the survivor of a construction aimed at its predecessor, and the
cards below collect the family per piece — in the order the pieces
appeared.

#figure(
  attack(
    [one-leaf ladder$(d)$],
    [the weighted fold's scale anchor],
    stack(dir: ttb, spacing: 4pt,
      skyline(
        ((0.03125, 0), (0.03125, 1), (0.0625, 1), (0.125, 1),
         (0.25, 1), (0.5, 1)),
        w: 210pt, unit: 14pt, show-heights: false,
        ticks: ((0.0, [0]), (0.5, [½]), (1.0, [1])),
      ),
      text(size: 7.5pt, fill: gray-line.darken(40%),
        [one unit leaf at every depth: level $i$ contributes area
         $2^(-i)$, so the running numerator is the all-ones
         integer as wide as the depth already walked — at every
         level]),
    ),
    [a fold that anchors at the running maximum depth rescales its
     held numerator at every deepening: $Theta(d^2)$ limb work
     against $Theta(d)$ input bits.],
    cure: [the topology-only pre-pass pins $S$ up front; every
      landing is final, and this family reads the same linear
      signature as a one-bit-numerator control.],
  ),
  kind: image,
  caption: [The one-leaf ladder attack card: width bought by depth
    alone, aimed at per-level rescaling.],
) <fig-attack-ladder-fam>

#figure(
  attack(
    [jump comb$(t, k)$],
    [the live component's width discipline],
    stack(dir: ttb, spacing: 4pt,
      skyline(
        ((0.125, 1), (0.125, 2), (0.125, 5), (0.125, 6),
         (0.125, 5), (0.125, 6), (0.125, 5), (0.125, 6)),
        w: 210pt, unit: 11pt, show-heights: false,
      ),
      text(size: 7.5pt, fill: gray-line.darken(40%),
        [a low start, one wide mid-stream jump to a $2^k$-scale
         band, then 3-bit oscillation: the jump is the one wide
         code, arriving with only cheap codes behind and ahead of
         it]),
    ),
    [a sweep that keeps the jump in its running state re-reads its
     $k$-bit width under every later 3-bit code — stale drift
     riding a cheap-delta path.],
    cure: [the relative freeze trigger: the first cheap code that
      finds the live drift over-wide evicts it, once, paid by the
      jump's own code — while bounded oscillation at any width
      (the wide-tooth comb) never trips it.],
  ),
  kind: image,
  caption: [The jump comb attack card: eviction priced against
    residence — the freeze trigger's two-sided witness.],
) <fig-attack-jumpcomb>

#figure(
  attack(
    [lone freeze$("pre", "post")$],
    [the settle machinery's fixed costs],
    stack(dir: ttb, spacing: 4pt,
      skyline(
        ((0.1, 5), (0.1, 6), (0.1, 5), (0.1, 6),
         (0.15, 2), (0.1125, 1), (0.1125, 2), (0.1125, 1), (0.1125, 2)),
        w: 210pt, unit: 11pt, show-heights: false,
      ),
      text(size: 7.5pt, fill: gray-line.darken(40%),
        [a long wide-plateau oscillation (no freeze ever fires),
         one allowance-clearing drop (the sweep's single freeze),
         then a long cheap tail with the parked drift live]),
    ),
    [a per-interval deposit made _before_ any drift exists to
     settle scales with the prefix and is never read; a tail feed
     or close read not amortized $O(1)$ per interval scales with
     the tail against $O(1)$ funded wide codes.],
    cure: [the first-freeze gate — the segment feed opens only when
      parked drift exists — and the watermark segment read: the one
      settle is priced at the segment's written span. The family
      dials both axes independently.],
  ),
  kind: image,
  caption: [The lone freeze attack card: the smallest nonempty
    settle, aimed at costs that should be zero on never-freezing
    sweeps.],
) <fig-attack-lonefreeze>

#figure(
  attack(
    [freeze staircase$(s)$],
    [freeze accounting against absolute positions],
    stack(dir: ttb, spacing: 4pt,
      skyline(
        ((0.125, 6), (0.125, 5), (0.125, 4), (0.125, 3),
         (0.125, 2), (0.125, 1), (0.125, 1), (0.125, 0)),
        w: 210pt, unit: 11pt, show-heights: false,
      ),
      text(size: 7.5pt, fill: gray-line.darken(40%),
        [$2s$ descending leaves alternating a ten-digit drop and a
         unit drop down a spine: every pair re-arms wide drift and
         the unit fires a freeze — $Theta(s)$ freezes at ever
         deeper stream positions]),
    ),
    [an accounting that multiplies evicted drift by its absolute
     position — or re-reads any whole-history state per freeze —
     goes quadratic while every comb fires $O(1)$ freezes and this
     family's positions compact to $O(1)$ digits.],
    cure: [anchored segments: each settle multiplies the parked
      drift by the interval mass _since its own anchor_, so no
      absolute position is ever a factor; the committed
      absolute-position kernel keeps reading superlinear beside
      the flat pin.],
  ),
  kind: image,
  caption: [The freeze staircase attack card: many freezes, deep
    positions — the family that forced the anchoring rule.],
) <fig-attack-freezepos>

#figure(
  attack(
    [re-arming spine$(p)$],
    [the parked component's re-settle width],
    stack(dir: ttb, spacing: 5pt,
      codestrip((
        ([span-building prefix: $32p$ cheap levels], 118pt, "t"),
        ([climb $2^608$], 52pt, "w"), ([1], 12pt, "p"),
        ([climb $2^288$], 46pt, "w"), ([1], 12pt, "p"),
        ([$dots.c times p$], 30pt, "x"),
      )),
      text(size: 7.5pt, fill: gray-line.darken(40%),
        [each block parks a 20-digit drift, then freezes again at a
         10-digit one: the parked component is over-wide at every
         second freeze, $Theta(p)$ times, at $O(1)$ stored codes
         each — over a consumed mass whose written span keeps
         growing]),
    ),
    [a design that re-settles the full parked width at every
     narrower freeze — or re-reads the whole banked mass per
     arming — goes quadratic at $O(1)$ codes per block.],
    cure: [promotion: an over-wide parked component is recorded
      once, with its banked window, as one ledger entry — two
      funded-width reads, no product — and settles once, at the
      close.],
  ),
  kind: image,
  caption: [The re-arming spine attack card: many armings, the
    family that forced the promotion ledger.],
) <fig-attack-rearm>

#figure(
  attack(
    [punctured tail$(p, d)$],
    [the ledger settle's re-read discipline],
    stack(dir: ttb, spacing: 5pt,
      codestrip((
        ([gap spine: $d$ turns, one digit apart], 100pt, "t"),
        ([arming], 34pt, "w"), ([arming], 34pt, "w"),
        ([$dots.c times p$], 30pt, "x"),
        ([trailing mass: ones-run punctured $d$ times], 110pt, "p"),
      )),
      text(size: 7.5pt, fill: gray-line.darken(40%),
        [$p$ promotions all owing their debt across one trailing
         interval mass whose punctures sit a full digit apart —
         $Theta(d)$ balanced digits no compaction can merge]),
    ),
    [a settle that walks the suffix once per arming — or re-reads a
     promoted prefix once per window — pays $Theta(p dot d)$
     against a $Theta(p + d)$-bit input; the two directions are
     duals, and each defeats a naive associativity.],
    cure: [the mass-balanced product tree: every arming-window
      cross term rides exactly one aggregate product, and no width
      or density is re-read more times than its node's depth —
      logarithmic by the mass balance.],
  ),
  kind: image,
  caption: [The punctured tail attack card (with its cheap mate for
    the pair measures): the shared-suffix loading of the ledger
    settle.],
) <fig-attack-densesuffix>

#figure(
  attack(
    [arming train$(t, w, g)$],
    [the product tree's aggregation],
    stack(dir: ttb, spacing: 5pt,
      codestrip((
        ([window 1], 40pt, "t"), ([$+2^(32 w)$], 40pt, "w"),
        ([window 2], 40pt, "t"), ([$-2^(32 w)$], 40pt, "w"),
        ([window 3], 40pt, "t"), ([$+2^(32 w)$], 40pt, "w"),
        ([$dots.c$], 16pt, "x"),
      )),
      text(size: 7.5pt, fill: gray-line.darken(40%),
        [$t$ wide armings, each behind its own dense
         topology-funded window; a sign dial makes consecutive
         armings alternate or align]),
    ),
    [an aggregation that cannot cancel opposing armings before
     multiplying pays every full width against every window to its
     right; one that re-reads windows per level without a mass
     balance stacks a polylog on the multiplication bound.],
    cure: [aggregate nodes sum parked components digit-wise
      (opposing armings cancel before any product reads a width)
      and hold windows as sparse balanced digits; the mass split
      keeps the per-level products telescoping.],
  ),
  kind: image,
  caption: [The arming train attack card: many wide armings with
    interleaved windows, aimed at the settle's aggregation
    algebra.],
) <fig-attack-armingtrain>

#figure(
  attack(
    [wide arming$(w, d)$],
    [the settle product itself],
    stack(dir: ttb, spacing: 5pt,
      codestrip((
        ([gap spine: $d$ turns], 70pt, "t"),
        ([one arming, $32 w$ bits — as wide as the input], 130pt, "w"),
        ([trailing mass: $d$ dense digits], 90pt, "p"),
      )),
      text(size: 7.5pt, fill: gray-line.darken(40%),
        [one promotion whose parked mass is as wide as the input,
         owing its debt across a trailing mass as dense as the
         input — the wide $times$ dense product at its purest]),
    ),
    [a per-digit schoolbook charge pays $Theta(w dot d)$ digit work
     against a $Theta(w + d)$-bit operand — quadratic at $w = d$;
     the committed schoolbook kernel keeps failing exactly here.],
    cure: [delegation: the product rides one sub-quadratic backend
      multiplication per dense cluster, so the settle costs the
      multiplication bound $M$ over its funded factors — which the
      floor below proves is not overhead but the answer's own
      price.],
  ),
  kind: image,
  caption: [The wide arming attack card: the product no charge
    scheme can split, priced at the multiplication bound.],
) <fig-attack-widearming>

#figure(
  attack(
    [puncture product$(x, y)$],
    [every exact area fold — the floor],
    stack(dir: ttb, spacing: 4pt,
      skyline(
        ((0.5, 3), (0.25, 0), (0.125, 3), (0.0625, 3), (0.0625, 0)),
        w: 210pt, unit: 13pt, show-heights: false,
        ticks: ((0.0, [0]), (0.5, [½]), (1.0, [1])),
      ),
      text(size: 7.5pt, fill: gray-line.darken(40%),
        [a plateau at the arbitrary integer $x$ (drawn as 3) over
         dyadic turns whose interval masses spell the arbitrary
         mass $2y$ bit by bit: the exact rank numerator is
         $2 x y + 1$, and the stored version is
         $Theta("bits"(x) + "bits"(y))$ bits]),
    ),
    [nothing — this family is not defeated. Any fold that answers
     exactly has computed $x dot y$ from input-funded factors at
     linear overhead: $Omega(M(n))$ digit work is mandatory, and no
     future mechanism can go below one multiplication here.],
  ),
  kind: image,
  caption: [The puncture product attack card: the answer-embedding
    reduction. The committed instance draws both factors from an
    incompressible digit stream, so neither side compacts and the
    floor's denominator cannot drift.],
) <fig-attack-puncture>

#figure(
  attack(
    [weight comb$(t)$],
    [the accumulator's top settlement, through rank's deposits],
    stack(dir: ttb, spacing: 4pt,
      skyline(
        ((0.02, 1), (0.02, 0), (0.12, 0), (0.105, 0), (0.105, 2),
         (0.105, 0), (0.105, 2), (0.105, 0), (0.105, 2),
         (0.105, 0), (0.105, 2)),
        w: 210pt, unit: 13pt, show-heights: false,
      ),
      text(size: 7.5pt, fill: gray-line.darken(40%),
        [one unit parked at depth $32t$ (the sliver at the left,
         landing at digit 0), then $2t$ shallow leaves oscillating
         0 and 2, whose deposits land $Theta(t)$ digits above it —
         position weight is topology, so no code funds the gap]),
    ),
    [every cancellation makes the accumulator's top settle back
     across the never-written run: a scan that steps it digit by
     digit pays $Theta(t)$ unfunded touches per $O(1)$-bit event.],
    cure: [the zero-run certificates of @redundant's storage
      remark: the write that jumps a run records it once, and every
      settling scan crosses it whole — one touch however wide.],
  ),
  kind: image,
  caption: [The weight comb attack card: topology-funded gaps
    between deposits, aimed at digit-stepped top scans.],
) <fig-attack-weightcomb>

#figure(
  attack(
    [freeze parade$(t)$],
    [the scaled segment read, through rank's freezes],
    stack(dir: ttb, spacing: 4pt,
      skyline(
        ((0.02, 1), (0.02, 0), (0.12, 0), (0.14, 6), (0.14, 4),
         (0.14, 4), (0.14, 2), (0.14, 2), (0.14, 0)),
        w: 210pt, unit: 9pt, show-heights: false,
      ),
      text(size: 7.5pt, fill: gray-line.darken(40%),
        [the same parked unit at depth $64t$, then $t$ shallow
         wide-drop pairs, each firing one freeze whose segment
         masses all sit $Theta(t)$ digits above digit 0]),
    ),
    [a segment read-out that starts at digit 0 walks the
     never-written prefix once per freeze — $Theta(t^2)$ touches on
     linear input, dragging the wide-operand traffic with it.],
    cure: [the write watermark (@accum-contract's materialize
      clause): each read is priced at the segment's written span
      and returns the scale unspelled — the deep parked unit
      forecloses every emptiness shortcut that would fake it.],
  ),
  kind: image,
  caption: [The freeze parade attack card: deep-scale segments,
    aimed at read-outs that walk their scale.],
) <fig-attack-parade>

=== Distance and lag

Distance and lag take their _meaning_ from the lattice:

$ "distance"(a, b) &= integral |h_a - h_b| &&= "rank"(a or b) - "rank"(a and b), \
  "lag"(a, b) &= integral (h_b - h_a)^+ &&= "rank"(a or b) - "rank"(a), $

the pointwise identities $max - min = |a - b|$ and
$max - a = (b - a)^+$, integrated.
Distance is a metric on versions — a genuine one, not a
pseudometric: symmetry and the triangle inequality are inherited
pointwise from $|dot|$, and distance zero forces equality because
distinct versions denote distinct functions (@canonical's uniqueness
argument), which then differ over some plateau of positive width. Lag is the one-sided
"how much of $b$ have I not seen", the natural backpressure signal
for anti-entropy protocols.

The identities are the semantics and the differential oracle; they
are deliberately _not_ the implementation. The composite route —
emit the join and the meet, rank them, subtract — was refuted by a
two-operand construction: one operand's wide difference crests
riding over the other's cheap codes, deep under a spine that makes
every absolute position dense. There the meet's emission re-codes
one operand's width into switch jumps, and the ranks' integrals
then evict that drift at boundaries the _other_ operand's cheap
codes set, at a position density neither operand funded — each
operand certified linear alone, the composition superlinear. The
landed form is a single fused co-sweep: one merge walk over both
streams maintaining the running difference $D = h_a - h_b$ exactly
as comparison does, integrating $h^* = sigma dot D$ where
$sigma in {-1, 0, +1}$ is the measure's _orientation_ at
$"sign"(D)$ (distance: the sign itself; lag: $-1$ where $D < 0$,
else $0$), so $h^*$ is the measure's nonnegative integrand by
construction. Per boundary, with $sigma -> sigma'$ and net folded
difference $d D$, the integrand moves by
$(sigma' - sigma) dot D' + sigma dot d D$: the second term re-folds
the boundary's own codes, and the first materializes $D$ only at
orientation changes — which require $D$ to have crossed, left, or
entered zero at this boundary, so $|D'| <= |d D|$ and the read is
priced by the codes just folded, the same argument as join's
switch. The integrand runs on the anchored-segment split of the
rank fold unchanged — rank _is_ this integral's single-stream
instance, its orientation constantly $+1$ — with one difference of
funding arity: the potential of @funding splits into one ledger
per operand, and every charge names the ledger of the operand
whose codes funded it. A cheap code from one operand can _fire_ a
freeze, but the work the freeze performs is bounded by deposits
from the codes that built the state being moved — never by a
position the firing operand chose. Costs are exactly rank's, in
the pair denomination $n + m$, floor included: the composed forms
survive as committed differential pins (digit-exact against the
co-sweep on every family), not as the shipped path.

#figure(
  attack(
    [jump pair$(k, t, d)$],
    [the pair measures' freeze accounting],
    stack(dir: ttb, spacing: 4pt,
      overlay(
        ((0.0625, 0), (0.0625, 5), (0.0625, 0), (0.0625, 0),
         (0.0625, 5), (0.0625, 0), (0.0625, 0), (0.0625, 5),
         (0.0625, 0), (0.0625, 0), (0.0625, 5), (0.0625, 0),
         (0.0625, 0), (0.0625, 5), (0.0625, 0), (0.0625, 0)),
        ((1.0, 4),),
        w: 210pt, unit: 11pt,
        label-a: [teeth], label-b: [band],
      ),
      text(size: 7.5pt, fill: gray-line.darken(40%),
        [one operand's $2^k$-scale teeth against the other's
         near-flat band, deep under a shared spine that makes every
         absolute position $d$ incompressible digits: $|D|$ crests
         wide at every level, and the crest is parked at the
         _band's_ cheap boundaries]),
    ),
    [each operand is certified linear alone; composed, an
     absolute-position accounting pays crests $times$ positions
     $times$ width — $Theta(t dot d dot k)$ limb work on a
     $Theta(t k + d)$-bit pair — and the freezes are fired by the
     operand that never funded the drift.],
    cure: [the two-ledger potential plus anchored segments: the
      work each freeze moves is bounded by the deposits of the
      codes that built the drift, and each crest settles against
      its own segment — flat, with the composed emit-then-rank
      route left behind as the design this family refuted.],
  ),
  kind: image,
  caption: [The jump pair attack card: the two-operand composition
    that exists in neither operand, aimed at cross-funded freeze
    work.],
) <fig-attack-jumppair>

=== Minimum ticks

The fewest tick operations that could have produced
$v$, over all fork/tick/join histories from the seed, equals the sum
of the normalized event tree's node values. (The measure, the
identity, and the argument below are this design's own — the paper
does not define a tick count. One step of the floor's argument is a
stated gap, flagged below and catalogued at @closing.) The sum has a pleasant skyline
form: writing $mu(x)$ for the _absolute_ minimum of the skyline over
node $x$'s interval — so $mu("leaf") = h$ — the normalized base at
$x$ is $mu(x) - mu("parent"(x))$, and the sum telescopes:

$ sum_x "base"(x) = sum_("leaves") h - sum_("internal") mu =: M(v). $

Both directions of the identity deserve their sketch, since an API's
meaning rests on it. _Floor_: $M$ is a functional on versions —
forks preserve it (both halves keep the event component), joins
are subadditive in it, and a single tick raises it by at most one.
One honest flag before the clauses are used. These three do not by
themselves compose over an arbitrary history: after a fork, two
lineages share their prefix ticks, and adding per-lineage bounds at
a join counts the shared ticks twice. The floor therefore needs an
induction over the whole system of live stamps, with the tick
clause applied to joined state. That composition, with join
subadditivity inside it, is the one step this document states
rather than proves; it is catalogued at @closing.
The last is where to look closely. `grow`'s increment raises one
leaf term by one, and enclosing minima can only rise — and rising
minima subtract. The net is at most $+1$. A `fill` collapse replaces a subtree's
contribution to $M$ by exactly the subtree's maximum, and the
contribution was at least that already — over any subtree,
$M = M_"left" + M_"right" - mu >= max_"left" + max_"right" -
min(max_"left", max_"right") = max$, by induction from the leaves —
so a collapse never increases $M$. And a `fill` raise lifts an owned
plateau _exactly to the adjacent filled minimum_, so the leaf term
and the enclosing node's minimum term rise by the same amount and
cancel. No tick raises $M$ by more than one — the per-version
clauses all hold; the composition across forks is the flagged gap.
_Achieved_: mirror the
telescoping rather than the leaves. At each node from the root down,
tick the whole currently-owned region $mu(x) - mu("parent"(x))$
times, then fork into the two children, joining the halves back at
the end (joins spend no ticks) — the counts spent are
exactly the normalized bases, summing to $M(v)$. (No tick here gets
a free raise: at each node one child's normalized base is zero, so
the region being ticked never sits below a sibling's minimum and
every tick is a bare increment.)

The count is an unbounded natural — heights are unbounded, so the
sum is too — and at first sight it is the one measure the funding
discipline cannot reach: the formula adds an _absolute_ height at
every leaf, exactly the naive fold the rank paragraph just
rejected. Two accounting moves dissolve the absolutes, one per sum.
The heights side splits $h = F + L$ — live drift on an accumulator,
under the same relative freeze trigger as rank — but $F$ never
materializes anywhere: an _epoch ledger_ holds one signed drift per
freeze (epoch 0's "drift" is the first leaf's absolute), each leaf
folds only its narrow epoch-relative offset into the total and
counts $+1$ against its epoch, and the frozen component reaches the
total once, at the end, by summation by parts — one
$"drift" times "suffix-count"$ product per freeze, priced by the
drift's own width times the count's $O(1)$ compacted digits. The
minima side rides a range-minimum web (@tick-web builds the same
nesting structure for tick, with the boundaries between adjacent
open minima held as differences, zero runs compressed): the
innermost minimum's _value_ is always some leaf the sweep already
paid for, recorded once as a narrow epoch-relative offset when it
becomes the minimum, and every node that closes while that value
reigns just _counts_ — $O(1)$, no width read. The record settles
into the total exactly once, at its death (a lower leaf dethrones
it, or the stream ends), as one $"offset" times "count"$ product
priced by the offset's width; an interruption — an inner range
arming above it — moves the record aside and returns it at the pop
with its count intact, never re-reading the offset. No event is
ever re-based across a freeze: an offset keeps its epoch, and the
ledger's closing settle carries the frozen differences for it.

Note what this measure does _not_ inherit: rank's multiplication
floor. Both of the fold's product forms multiply a width by a
_count_ — closes at a reigning value, leaf references per epoch —
and a count's digits are logarithmic in the input's node count,
never input-widened; position never multiplies a value, because the
count weighs every leaf the same. No answer-embedding construction
exists here, and the sweep is one funded linear pass, exact at any
magnitude, on every input.

== Tick: `fill`, `grow`, and the watermark web <tick>

The tick is the paper's `event`: run `fill`, keep its result if it
changed anything, else apply `grow`'s cheapest strict increment.
It is the one operation whose walk is genuinely intricate, because
`fill`'s equations ask _range_ questions — maxima and minima over
subtrees on both sides of the cursor — while everything before this
point asked only pointwise ones. This subsection is the payoff of the whole apparatus. It derives
the walk in
five stages — the semantics, the walk and its lookahead, the
watermark web that carries its range minima, the fused decision
between `fill` and `grow`, and the output bound that closes the
funding ledger — with a pair of worked traces between the last two.

=== The semantics, restated on the skyline <tick-semantics>

The paper's `fill` is short, and worth having at hand verbatim —
@tick-walk maps each arm to a stream action. The numbering
(1)–(6) is ours:

$ "fill"(0, e) &= e &&quad (1) \
  "fill"(1, e) &= max(e) &&quad (2) \
  "fill"(i, n) &= n &&quad (3) \
  "fill"((1, i_r), (n, e_l, e_r)) &= "norm"((n, max(max(e_l), min(e'_r)), e'_r)), quad e'_r = "fill"(i_r, e_r) &&quad (4) \
  "fill"((i_l, 1), (n, e_l, e_r)) &= "norm"((n, e'_l, max(max(e_r), min(e'_l)))), quad e'_l = "fill"(i_l, e_l) &&quad (5) \
  "fill"((i_l, i_r), (n, e_l, e_r)) &= "norm"((n, "fill"(i_l, e_l), "fill"(i_r, e_r))) &&quad (6) $

In skyline terms: `fill` flattens what the caller owns — every event
subtree the id wholly owns collapses to one plateau at that subtree's
maximum (arm 2) — and the two _shortcut arms_ (4 and
5) additionally let a wholly-owned, freshly collapsed child rise
to the minimum of its _filled_ sibling, if that is higher. The rise
is the profitable move: it merges plateaus across the sibling
boundary and lets ancestors collapse in turn. The cap at the
sibling's minimum is _not_ a safety constraint: over its own region
a participant may inflate as far as it likes (@model: any inflation
over the id is a legal successor). It is parsimony — the paper's
own
"does not dominate more events than needed" desideratum — and the
minimum-tick measure of @measures characterizes it exactly.
Up to the sibling's minimum, a raise costs nothing, the leaf term
and the enclosing minimum rising together; past it, the version
would
claim events no observation forced. When the filled sibling is
itself flat, the capped raise moreover merges the pair outright —
the case the worked trace below exhibits.

=== The walk, and its one lookahead <tick-walk>

Pair the id cursor against the version cursor, and emit through the
same collapsing builder as join:

#algo(title: [fill, as a sweep])[
  at each paired position: \
  #h(1.2em) *unowned* (id absent — arm 1): copy the event subtree's
    bits verbatim, after re-coding its first payload against the last
    plateau emitted (one boundary code; everything after it is
    delta-coded against neighbors inside the copy, which the copy
    leaves untouched). \
  #h(1.2em) *wholly owned* (id terminal — arm 2): consume the event
    subtree, folding its running max; emit one plateau at that max. \
  #h(1.2em) *event already flat* (id node over an event leaf —
    arm 3): emit the plateau unchanged; the id's finer structure
    below has nothing left to flatten. This is the steady-state arm:
    a region a participant keeps ticking is flat after the first
    fill. \
  #h(1.2em) *mixed* (both sides descend — arms 4–6): descend in
    stream order, iteratively, pushing two bits of suspended state
    per level (plus the two priced wider states this walk builds —
    @tick-web, @tick-fusion); on a shortcut arm (4, 5), raise the
    owned child's
    plateau to the sibling's filled minimum where that exceeds its
    max. \
]

Three range quantities ride the walk — a wholly-owned range's max,
a verbatim copy's extent, and the shortcut arms' sibling minimum —
and the first two are easy. A wholly-owned range's
max folds as it is consumed (word-or-wide adds, funded by the codes
read), and the structure is why it is easy: arm 2 consumes a
wholly-owned subtree in one contiguous scan, so owned-range scans
never nest, and the single live max rides as a bounded gap against
the running height, dead at its range's close. Open ranges' minima
nest; consumed ranges' maxima do not. Verbatim copies cost their own bits. The delicate quantity is
the shortcut arms' _sibling minimum_ — and the two arms differ in a
way the stream order makes vivid:

- *Right-full arm* ($(i_l, 1)$): the raised leaf is the _right_
  child's output, emitted after the left child's range has already
  been walked. Its minimum is a fact about output the walk has
  already produced — a _watermark_. The walk keeps it as it goes:
  no lookahead, no second pass.
- *Left-full arm* ($(1, i_r)$): the raised leaf is emitted _before_
  the range its minimum comes from. The walk must look ahead — and
  for the right quantity: the arm's equation asks for
  $min("fill"(i_r, e_r))$, the minimum of what `fill` _will emit_
  over the range, so the _pre-scan_ simulates the fill there rather
  than merely reading heights. That sounds circular — the simulated
  fill has shortcut arms of its own, each wanting a further
  lookahead — but one lemma flattens it: on either shortcut arm the
  raised child takes the maximum of its own collapsed max and its
  sibling's filled minimum, so it never undercuts that sibling. A
  shortcut arm's output minimum is thus exactly its
  not-wholly-owned sibling's
  filled minimum, raise or no raise; a fully-mixed arm's is the
  minimum of its two filled halves, and a wholly-owned range's is
  its
  collapsed max. Each
  quantity the pre-scan needs is therefore a range quantity that
  settles when its range closes, and one left-to-right pass keeping
  a pending minimum per open range computes them all — no nested
  scans. Naive repetition would still
  re-scan shared subranges once per enclosing arm,
  quadratically; the pre-scan is therefore _memoized_ — one fresh
  scan per uncovered range, with
  every interior left-full site's minimum recorded on the way, so no
  stream position is ever pre-scanned twice. The walk's total read
  budget is flat: at most two passes over each position of either
  stream, id and version alike. Tick is the other
  two-pass operation the introduction owned up to (the grow branch
  adds a bounded third, the splice). The memo's
  _memory_ obeys the same ledger as its reads: at most one recorded
  entry per left-full site (so no more entries than id bits), each
  held as a bounded difference against a reference the walk still
  holds when the entry is consumed — never an absolute — so $k$
  nested sites sharing one wide minimum store its width once, not
  $k$ times.

#figure(
  attack(
    [nested raises$(d, W)$],
    [the right-full arm's deferred bookkeeping],
    stack(dir: ttb, spacing: 5pt,
      oprow([id], codestrip((
        ([site], 26pt, "t"), ([site], 26pt, "t"), ([$dots.c$], 14pt, "x"),
        ([site], 26pt, "t"), ([full], 26pt, "t"),
      ))),
      oprow([version], codestrip((
        ([$W$-bit root], 64pt, "w"), ([level], 24pt, "t"),
        ([level], 24pt, "t"), ([$dots.c$], 14pt, "x"),
        ([level], 24pt, "t"),
      ))),
      text(size: 7.5pt, fill: gray-line.darken(40%),
        [a right-full shortcut site at every one of $d$ id levels —
         the deepest stacking of deferred raise decisions — with a
         wide variant putting the root's $W$ bits into every
         level's net movement]),
    ),
    [bookkeeping that carries an absolute quantity per open site
     pays width $times$ depth through the right-full arm — the
     path-sum defect reborn in the walk's suspended state.],
    cure: [watermarks as differences (@tick-web) and raises funded
      at the range's close: the narrow variant reads flat, and the
      wide variant's one wide quantity rides a single accumulator,
      never the stack.],
  ),
  kind: image,
  caption: [The nested raises attack card: every level a shortcut
    site, aimed at the walk's suspended state — narrow and wide
    variants.],
) <fig-attack-nested>

#figure(
  attack(
    [mirrored nesting$(d, W)$],
    [the memoized pre-scan],
    stack(dir: ttb, spacing: 5pt,
      oprow([id], codestrip((
        ([left-full site], 48pt, "t"), ([left-full site], 48pt, "t"),
        ([$dots.c$], 14pt, "x"), ([left-full site], 48pt, "t"),
      ))),
      oprow([version], codestrip((
        ([level], 24pt, "t"), ([level], 24pt, "t"), ([$dots.c$], 14pt, "x"),
        ([level], 24pt, "t"), ([$W$-bit tail], 60pt, "w"),
      ))),
      text(size: 7.5pt, fill: gray-line.darken(40%),
        [the mirror image: a left-full site at every level, so
         every raise needs a _future_ range's minimum; a comb
         variant makes consumption order run $Theta(d)$ apart from
         recording order, and a wide tail puts $W$ bits into every
         net movement]),
    ),
    [an unmemoized pre-scan re-reads shared subranges once per
     enclosing site — quadratic; records resolved by walking the
     recorded differences between consecutively consumed sites
     re-read $Theta(d)$ of them per consume.],
    cure: [one fresh scan per uncovered range, with per-site
      records held as bounded differences against the walk's own
      live state — consumed in $O(1)$ each, wherever the
      consumption order lands. A raise-ordering variant, whose
      every consume moves the tracked minimum, pins the
      decide-then-emit ordering the records must survive.],
  ),
  kind: image,
  caption: [The mirrored nesting attack card: the left-full memo
    family — chains, combs, fanouts, and churn variants all land
    on the same two disciplines.],
) <fig-attack-memo>

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
- *A range open* is a move and a zero: the new innermost gap starts
  at $t = 0$ — the opening plateau is its own minimum so far — and
  the previous gap parks as the new boundary difference, recycling
  the one-slot register $r$ where a close just filled it; a zero
  difference extends the counted run. $O(1)$, nothing wide touched:
  the opener _creates_ the state that emissions, undercuts, and
  closes then maintain.
- *An emitted plateau at or above the innermost minimum* is one
  amortized sign read of $t$.
- *An emitted plateau _below_ the innermost minimum* — an
  _undercut_ — must lower some suffix of the watermark chain. It
  walks outward: each nonzero difference it fully penetrates _dies_
  (folded once into the running residue — the $Phi$ drop pays);
  each zero _run_ passes in $O(1)$, wholesale, because every frame
  in the run shares the inner minimum and is updated implicitly —
  and an undercut can never halt inside a run, since frames sharing
  one minimum are penetrated all or none;
  and at the first difference it cannot penetrate, one surviving
  fold, priced by the undercut's own delta — whose code the input
  just paid. Without run compression this cascade is
  $Theta("open depth")$ per undercut, and the *descending
  staircase*
  (@families) shows why the zero runs are where the danger lives.
  Its first descent drags every open minimum down together, zeroing
  every enclosing difference; each further unit step then
  penetrates
  the full stack of dead frames with no deaths left to fund the
  walk — $Theta(d^2)$ total. With run compression, each undercut
  costs its dying
  differences plus $O(1)$. (Both halves measured: the uncompressed
  form's quadratic on the staircase is reproducible, and the
  compressed form holds every family flat.)
- *A range close* pops the innermost difference and _moves_ it
  aside into a one-slot register, rather than folding it into the
  revealed watermark. The walk's invariant weakens by exactly that
  slot: with $r$ the parked difference, the revealed range's true
  gap is $t + r$, resolved lazily. Four cases resolve it. The next
  opener recycles $r$
  into its own boundary difference — a move, then a narrow
  adjustment against a reference the walk already holds. A second
  close before an open folds
  the narrower of the two parked differences into the wider: a
  death, funded, which is why one slot suffices. An undercut deep
  enough to reach $r$ annihilates it — a death again. And
  comparisons against the composite gap go through the domination
  floor, deciding from top digits without touching $r$'s width;
  where the floors fail to decide — the scales being comparable —
  the
  narrower of the two folds into the wider and dies, a $Phi$ drop
  funded once by the codes that deposited it. Closes cannot re-arm
  it: they move $r$, never re-spell it.
  The distinction between move and fold is not pedantry.
  Fold-on-close re-folds a wide boundary difference on every
  close/reopen cycle, and a comb of sibling sites sharing one
  $2^k$-scale minimum — the *reveal comb* of @families, the last of
  its constructions — then circulates that width once per site
  with no input or output code funding any crossing: work
  proportional to sites times scale, on input proportional to sites
  plus scale. That amplifier is genuine; we found it by
  adversarial construction against an earlier design of this very
  walk, and making the close a move cures it. Moves are free;
  deaths pay; nothing is read twice at width.

The same difference discipline runs inside the memoized pre-scan, as
@tick-walk already noted for its memory; the pre-scan is a second
instance of the web, not a second mechanism.

#figure(
  attack(
    [descending staircase$(d)$],
    [the watermark stack's undercut cascade],
    stack(dir: ttb, spacing: 4pt,
      skyline(
        ((0.125, 7), (0.125, 6), (0.125, 5), (0.125, 4),
         (0.125, 3), (0.125, 2), (0.125, 1), (0.125, 0)),
        w: 210pt, unit: 9pt, show-heights: false,
      ),
      text(size: 7.5pt, fill: gray-line.darken(40%),
        [unit steps descending a $d$-level spine: the first descent
         drags every open minimum down together, zeroing every
         enclosing difference — and each further step undercuts the
         whole stack again]),
    ),
    [with one frame per open range, each unit step penetrates the
     full stack of dead frames with no deaths left to fund the
     walk: $Theta(d^2)$ on unit-scale values.],
    cure: [zero-run compression: frames sharing one minimum are one
      counted entry, penetrated all or none, so each undercut costs
      its _dying_ differences plus $O(1)$ — both halves measured,
      the uncompressed quadratic reproducible.],
  ),
  kind: image,
  caption: [The descending staircase attack card: every level
    undercut at once, aimed at per-frame cascade work.],
) <fig-attack-staircase>

#figure(
  attack(
    [ascending cliff$(s)$],
    [the undercut's fold direction],
    stack(dir: ttb, spacing: 4pt,
      skyline(
        ((0.125, 1), (0.125, 2), (0.125, 3), (0.125, 4),
         (0.125, 5), (0.125, 6), (0.125, 7), (0.125, 0)),
        w: 210pt, unit: 9pt, show-heights: false,
      ),
      text(size: 7.5pt, fill: gray-line.darken(40%),
        [$s$ ascending wide leaves arm $s - 1$ nonzero unit
         boundary differences, then a terminal cliff to zero drives
         one $s$-scale residue outward through all of them]),
    ),
    [per hop, the residue meets a narrow difference: a fold that
     always subtracts the residue _into_ the survivor re-writes the
     residue's width at every hop — $Theta(s^2)$ — where the dying
     side's width is $O(1)$.],
    cure: [domination decides each hop's direction before any fold
      (@sign's floors), so the dying side always funds the fold
      that consumes it; a leveled control with the same hop
      schedule passes the whole stack as one zero run, isolating
      the genre.],
  ),
  kind: image,
  caption: [The ascending cliff attack card: a wide residue driven
    through narrow boundaries, aimed at fold-direction mistakes.],
) <fig-attack-ascend>

#figure(
  attack(
    [reveal comb$(t, k)$],
    [the close rule — move versus fold],
    stack(dir: ttb, spacing: 4pt,
      skyline(
        ((0.11, 0), (0.127, 5), (0.127, 6), (0.127, 5), (0.127, 6),
         (0.127, 5), (0.127, 6), (0.128, 5)),
        w: 210pt, unit: 9pt, show-heights: false,
      ),
      text(size: 7.5pt, fill: gray-line.darken(40%),
        [$t$ sibling sites sharing one $2^k$-scale minimum over a
         zero floor (the sliver at the left), the walk closing each
         site's range back into the floor's between consecutive
         visits: a $k$-wide boundary difference is minted and
         popped once per site]),
    ),
    [fold-on-close re-folds the wide boundary difference on every
     close/reopen cycle — sites $times$ scale work on sites $+$
     scale input, with no code funding any crossing.],
    cure: [the close _moves_ the difference into the one-slot
      register, $O(1)$; only deaths fold, and each is funded once.
      A bare variant with no shortcut site isolates the stack's own
      arm/close cycle, and a raised-floor control shrinks the
      circulated width to $O(1)$ — the gap control.],
  ),
  kind: image,
  caption: [The reveal comb attack card (with its bare and
    raised-floor kin): the close/reopen cycle that refuted an
    earlier design of this walk.],
) <fig-attack-reveal>

=== The changed flag, and the fused `grow` <tick-fusion>

`tick` needs to know whether `fill` changed anything, and the walk
decides it in-pass: the flag trips at the first emitted plateau that
differs — in extent or in height — from the input plateau it
replaces. Until then the output
would be a bit-identical prefix of the input — nothing earlier has
changed, so no copy needs its boundary payload re-coded — so
nothing is built. The first divergence copies the matched prefix
wholesale, and emission continues from there. A walk that never
diverges has _built nothing at all_, and hands its second product to
`grow`. Riding along the same walk, a fold has recorded at every id
branch node which child hosts the cheaper inflation — one direction
bit per id branch. Cheaper means fewest node
expansions, then shallowest depth: the paper's `grow` cost
function read lexicographically (its "large constant $N$" per
expansion is exactly a lexicographic order, as the paper itself
notes), with ties to the right child as in the paper's final
equation. The fold's working set
is honest about its width: one pending cost pair per open id
branch, two counts each at most the id's node count — log-width
entries, held on a pop-able stack that prices each at twice its
own width in packed bits, never a machine-word frame. Beside the
watermark stack it is the walk's other suspended state wider than
the two-bit norm, and it is priced the same way: bounded by the
id's own
depth, alive only while the flag is clear. `grow` then _splices_:
copy everything up to the inflation point verbatim; re-code the
grown leaf's payload ($+1$); repair the one successor delta that the
change can reach; emit an expansion chain's fresh leaves as
$0 slash plus.minus 1$ codes; copy the suffix verbatim. One walk
plus at most one splice, both linear.

Can the increment break canonicality — raise a leaf into equality
with its sibling, demanding a merge the splice does not perform?
Belt and braces. The spliced output still flows through the
collapsing builder, so any merge the increment makes reachable _is_
performed. And on this branch the case is vacuous anyway: a
wholly-owned leaf sitting one below a sibling it could equal is
exactly
what `fill`'s shortcut arm would have raised, so the flag would have
tripped and `grow` would never have run. On arm 3's unchanged
emission the increment does not fire either — there `grow` expands
the leaf into fresh nodes rather than raising it, creating no equal
pair.

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
raises the owned leaf to the maximum of its own max and the
sibling's filled minimum, and the pre-scan reports that minimum as
0, which the owned max 1 already dominates — no change. Nothing else
is owned; every emitted plateau equals its input; the flag never
trips, so nothing is built. The route fold had nothing to choose —
the id's single branch forces the way, and the cheapest inflation
is the owned leaf's bare increment, $1$ to $2$, no expansion. The splice then
copies the topology verbatim and re-codes exactly two payloads: the
grown leaf's absolute, $1 -> 2$ (`010` $->$ `011`), and its
successor's delta, $-1 -> -2$ (`010` $->$ `00100`); the final code
(`00101`) copies untouched. Result:
$"tick"(v) = (0, 2, (0, 0, 2))$, eighteen bits, built by two skims
and two re-codings — and the reader can check every bit against
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

$ "size"("tick"(i, e)) <= 2 dot "size"(e) + 4 dot "size"(i) + 32 "bits." $

Each term has a mechanism. The factor 2 on the event side is real
and tight in kind, and it survives a shape that should worry a
reader fresh from @projection: every unowned copy re-codes its first
payload, and projection just showed per-region re-codings turning
$Theta(t + k)$ into $Theta(t dot k)$. The cases differ in what the
re-coding must spell. Projection zeroes unowned regions, so each
re-entry re-spells a _global_ absolute — codes from the whole
history, reused per transition. `fill` copies unowned regions
verbatim above an output that only _collapsed_, so each boundary
re-coding is a signed sum of deltas consumed inside the one owned
region that just collapsed — and the collapsed regions _partition_
the owned area, so the boundary re-codings sum to at most the
input's own codes: a doubling at worst, not a per-region blowup.
The tightness witness is the single unavoidable duplication: a
raise can
re-code one delta against a wide
neighbor, duplicating one input code's width once — and because a
single code can be nearly the entire stream, "one code duplicated"
_is_ a doubling. That is exactly how an earlier, stronger conjecture
died: we first believed the bound was additive
($"size"(e) + O(dots)$), and a constructed counterexample — one wide
code, duplicated — refuted it, leaving the multiplicative form as
the honest survivor. The id term covers `grow`'s expansion chain, which
descends along the id adding a constant number of flag and
$0 slash plus.minus 1$ payload bits per id level; the additive 32
absorbs the _widening_ of the first leaf's absolute code and the
splice's constant overheads.

Second, the growth does not compound. In closed form, for $k$
iterated ticks against the same party:

$ "size"("tick"^k (i, e)) <= "size"("tick"(i, e)) + 4 dot "size"(i) + 4 (floor(log_2 (k + 1)) + 1) + 8 "bits" $

— after the first tick's possible doubling, everything further is
logarithmic in $k$: the doubling is a one-step transient, not a
ratchet a peer could crank. This is the bound a tick-cranking peer
tests, and each of its terms has a mechanism too. Iterated ticks
cannot add nodes beyond the id's own resolution: an expansion
chain fires only where an event leaf still sits above id structure,
and once split, the region stays split. (A `fill` collapse cannot
undo a `grow` expansion under the same party. A
wholly-owned event _node_ always changes under `fill`, so `grow`
only ever sees leaves, and the shapes an expansion leaves behind —
sibling values a bare increment apart under split ownership — are
exactly the ones the next same-party `fill` does not disturb; the
full case
analysis lives with the derivation in our work.) So the node
budget is
spent at most once, the explicit $4 dot "size"(i)$ being slack that
covers whichever tick spends it. And $k$ ticks raise
values by at most $k$, so the two re-coded payloads widen by at
most $2 (floor(log_2 (k + 1)) + 1)$ code bits apiece — gamma's two
bits per magnitude bit, at the width of the count — the logarithmic
term. The closed form is
moreover _constructive_: the implementation offers the $k$-fold
tick as one operation, byte-identical to $k$ sequential ticks and
computed in a bounded number of passes — two fused walks and one
splice at any count, $O(n + m + log k)$ — so a caller skipping a
history forward pays what one tick costs plus the width of the
count, never $k$ walks.

Together: every code tick emits is priced by codes tick read, up to
constants, and the funded-sweep bound $O(n + m)$ holds
unconditionally for scan bits, digit touches, and transient heap
alike. It is
the strongest single cost statement in the system, and on more
currencies at once than any other operation — the direct
transcription misses it by a full polynomial degree.
