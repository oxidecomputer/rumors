#import "fig.typ": *

= The accumulator <accum>

Every sweep in this document maintains at least one _running signed
integer_. Validation carries the running height (@validation).
Comparison carries the running difference between two heights
(@sweep). The area measures carry running weighted sums (@measures).
The tick walk carries a whole small web of running range minima
(@tick). Each is updated by a stream of deltas — mostly tiny,
occasionally enormous — and each is consulted mostly for its _sign_.

The representation of these running integers is not an implementation
detail. It is the component every cost argument in @operations
bottoms out in, and the place where the resilience thesis is won or
lost. This section builds it from its requirements.

== The contract <accum-contract>

From the sweeps' side, the accumulator must support:

+ *apply a signed machine-word delta*, in amortized $O(1)$ work;
+ *apply a wide delta* of $ell$ machine words — optionally scaled by
  $2^s$ for arbitrary $s$ — in amortized $O(ell)$ work, _independent
  of the magnitude already held and of $s$_ (the one cost the scale
  can force, materializing storage lanes up to $s$, is paid once per
  high-water mark and funded by the input bits that certified the
  scale — the storage remark in @redundant);
+ *read the sign* of the held value, in amortized $O(1)$;
+ *materialize* the held value as an ordinary integer, in work
  proportional to that value's own width (the operation opens with
  the same collapsing fold the sign query runs — @sign — which
  brings the held spelling within two digits of the value's width);

and the bounds must hold on _every_ interleaving of these operations —
not in expectation, not for typical streams. One word on "amortized",
binding for the whole document: every accumulator is created and
destroyed inside a single API operation, so amortization is always
internal to one call — each operation is worst-case linear on its
own, not merely cheap on average across a sequence. Requirement 2's "scaled
by $2^s$" earns its keep in the weighted folds of @measures, where a
plateau's height is added at a position weight; the independence from
$s$ is what makes those folds linear.

The first two requirements express @ladder's constraint: a delta that
cost $c$ bits of code may fund only $O(c)$ work, wherever the running
value happens to sit. The carry cliff showed a normalized big integer
violating this — a 3-bit code extracting a $k$-bit carry — so
normalization itself is the suspect.

== Why every normalized region fails <two-zone>

One repair suggests itself immediately: keep the bulk of the value
normalized, plus a small unnormalized _window_ — a machine word of
pending drift — and pay the big carry only when the window overflows.
Small oscillations then live entirely in the window. The boundary
comb is absorbed.

It fails to a one-parameter generalization — the *wide-tooth comb* of
@families. Give the teeth width just beyond the window: deltas of
$plus.minus 2^192$, say, oscillating across a cliff at $2^k$ with $k$
much larger still. Each tooth's code
costs about 385 bits; each application punches through the
word-sized window into the normalized prefix and ripples the full
$k$-bit carry: work per tooth proportional to $k$, unbounded relative
to the tooth's own code. Widen the window and the teeth widen past it
again.

The lesson generalizes and is worth stating as a principle:

#block(inset: (x: 1.5em), [
  _Any representation with a normalized region has a boundary between
  "settled" and "pending" digits, and there exists an input stream
  that oscillates the value across that boundary at unit cost per
  crossing to the input and full-carry cost per crossing to the
  representation._
])

The only escape is for no digit to be "settled": *no normalized region
anywhere*.

== Balanced redundant digits <redundant>

Hold the value as digits in base $2^32$, little-endian, each digit a
_signed_ 64-bit integer kept in the _lazy zone_ $|a_i| < 2^33$:

$ "value" = sum_i a_i dot 2^(32 i), quad a_i in (-2^33, 2^33). $

Two deliberate redundancies. The digits are signed and may exceed the
base, so a given value has many spellings — that freedom is the
storage the design absorbs oscillation into. And the zone is _twice_
the base, so a digit can wander a long way before anything must be
done. (If an image helps: an abacus rod allowed to hold a surplus of
beads, and owing-beads too, so a carry need not propagate the moment a
column fills — the machine-arithmetic ancestry of the idea is
Avizienis's signed-digit number systems and the carry-save adder, and
its software ancestry the redundant number representations of the
purely functional data-structure tradition and Kulisch's long
accumulators for exact summation.)

A word-sized delta lands at the digit position its scale names —
digit 0 unscaled; a 64-bit magnitude spans two 32-bit positions. If
each touched digit stays in the zone, done: a touch or two. If a
digit leaves the zone, with $q$ its would-be value, carry

$ c = floor((q + 2^31) / 2^32) $

upward into the next digit and keep the _recentered_ remainder
$q - c dot 2^32 in [-2^31, 2^31)$; repeat upward while a digit
overflows. The bias in $c$ is the point: a digit that just carried is
left within $2^31$ of zero, so it must absorb at least
$2^33 - 2^31 = 3 dot 2^31$ of further _net_ drift before it can carry
again. Every carry is funded: a digit that carries cannot carry again
until deltas totalling $3 dot 2^31$ in net movement have landed on
it, so carries are strictly outnumbered by the deltas that provoke
them, and a word delta costs amortized $O(1)$ digit touches on every
stream. And because _every_ write recenters what it touches, no digit
anywhere is ever "settled": the two-zone counterexample has no
boundary to aim at.

(One storage remark, so the accounting has no hidden pocket: the
digits live in a dense little-endian vector, and a delta landing at a
new highest lane zero-fills the gap below it — once, because the
high-water mark only rises. The total zero-fill over a sweep is
bounded by the final lane count, and the largest scale any sweep uses
is bounded by the operand's depth, whose topology bits the sweep has
already read; the fill is funded, once, by those bits.)

#figure(
  {
    lanes((
      ("+9", [$a_4$]),
      ("−2 147 483 903", [$a_3$]),
      ("+6", [$a_2$]),
      ("0", [$a_1$]),
      ("−1", [$a_0$]),
    ))
    v(4pt)
    align(center, text(size: 8.5pt, fill: gray-line.darken(35%),
      [value $= 9 dot 2^128 - 2 147 483 903 dot 2^96 + 6 dot 2^64 - 1$,
       every digit inside $(-2^33, 2^33)$]))
  },
  kind: image,
  caption: [Digit lanes of the accumulator, drawn most-significant
    lane leftmost for readability (storage is little-endian). Digits
    are signed and may exceed the base $2^32$; each write recenters
    only the digits it touches, so a small delta lands in $a_0$ and
    stops. Nothing here is normalized, deliberately.],
) <fig-lanes>

A wide delta arrives as a sign and an $ell$-word magnitude, and
routes each 64-bit word into its two 32-bit digit positions
independently — positions $2i$ and $2i + 1$ for word $i$, offset by a
scale's digit shift — each half added or subtracted at its position,
carrying (rarely, and $O(1)$ each) where a digit leaves the zone.
Cost: $O(ell)$ touches regardless of what the accumulator already
holds and regardless of the scale, which is requirement 2 exactly.
Negation flips digit signs in place (a balanced digit's negation is a
balanced digit); subtraction is negated addition.

== Reading the sign: domination and collapse <sign>

The sign query looks troublesome: redundancy means the top digit's
sign can simply be wrong ($a_1 = +1$, $a_0 = -2^33 + 1$ has a
positive top digit and a decisively negative value), so no fixed
number of high digits settles the sign in general. Fold digits from
the top, maintaining a running partial $s$ by

$ s <- s dot 2^32 + a_i quad ("digit index" i "descending"), $

which after scanning down to index $i$ equals, in closed form,
$sum_(j = i)^("top") a_j dot 2^(32(j - i))$: the _exact_ value of the
scanned suffix in units of $2^(32 i)$. (The fold only continues while
$|s| <= 2$, so after a step $|s| < 2 dot 2^32 + 2^33 = 2^34$ — the
partial itself always fits comfortably in fixed-width arithmetic.) The digits
not yet scanned contribute, in the same units, at most

$ sum_(j < i) (2^33 - 1) dot 2^(32(j - i)) < (2^33 - 1) / (2^32 - 1) approx 2.0000000005, $

a hair over $2$. So the moment $|s| >= 3$, the suffix _dominates_
everything below: the sign is decided, stop. Most reads stop at the
top digit.

An adversary can prevent that: write $+2^k$, then $-(2^k - 1)$, and
the top digits cancel to a whisper — the fold must descend toward
digit 0 to find the surviving $+1$. Descending is honest work, but it
must not be repeatable for free: a stream of cheap sign queries
against one expensive cancelling prefix would re-scan it each time.
The fix makes the read pay forward: when the fold descends, it
_collapses_ what it scanned — zeroes the scanned digits and deposits
their exact partial $s$ at the scan's floor (a bounded write: the
fold's invariant keeps $s$ within two digits' range). The value is
unchanged; the spelling is now shallow; the next sign query re-reads
none of it. Since only writes make digits nonzero, and a collapse
zeroes every digit it scans, *each digit is scanned at most once per
write that made it nonzero*: sign queries amortize against the writes
that provoked them, and requirement 3 holds on every interleaving.

Two consequences deserve a pause. First, _reads mutate_: the sign
query rewrites the representation (value-preservingly). That is
unusual as an interface but it is exactly the amortization made
visible — the data structure's version of a splay. Second, the
decision bound generalizes: if the fold decided at digit index
$i >= f + 2$, then no quantity living entirely in digits
$0 dots f$ — anything of bounded scale — could overturn either the
sign or a magnitude comparison. The arithmetic behind the "$+2$":
deciding at index $i$ means $|s| >= 3$ against under $2.01$ of
unscanned tail, certifying $|"value"| > 0.99 dot 2^(32 i)$; a
quantity confined to digits $0 dots f$ is below
$2.01 dot 2^(32 (f + 1))$; with $i >= f + 2$ the first exceeds the
second by a factor near $2^31$. Sweeps use this _domination floor_
constantly: a watermark whose fold decided at digit index 5 stands
at least $0.99 dot 2^160$; no adjustment confined to digit 0 — any
machine-word quantity — can bring it near zero, and the comparison
ends after those one or two top digits, in $O(1)$, without touching
the watermark's width at all (@tick).

Materializing the held value for output — the one place normalization
happens — is a single low-to-high pass with a signed carry, costing
the held digit count. After a collapse, the held digit count exceeds
the value's true width by at most two digits (the fold decided at a
suffix whose value certifies width $32 i$ or more), so a
materialization costs $O$(the width of the value it produces) — which
is $O$(the code about to be written for it). Even the exit is funded.

== The funding discipline <funding>

The accumulator makes a general accounting scheme possible, and the
cost arguments of @operations are all instances of it. State it once,
here. During any sweep, every quantity is held as digits in
accumulators, and

#block(inset: (x: 1.5em), [
  _every digit touch is paid for by one of exactly three sources: an
  input code being consumed (which may also deposit new digits, at
  most its own width); an output code being emitted (which licenses
  reads up to its own width); or the death of digits already
  deposited (each digit dies at most once)._
])

Equivalently, with potential $Phi = $ the number of _nonzero_ digits
across all live accumulators (storage lanes are never freed; a digit
_dies_ when a write or a collapse sets it to zero): $Phi$ grows only
when input codes are consumed, and by at most their width (one
bookkeeping exception: a collapse zeroes every digit it scanned —
at least two, or it would not have descended — and deposits at most
two, so it never increases $Phi$); every touch not covered by a
consumed or emitted code is covered by a drop in $Phi$. Summing over
the sweep, total work is $O("input bits" + "output bits")$.

The discipline has teeth as rules of craft: wide values are _moved_,
never copied (a move is a buffer swap, $O(1)$, $Phi$-neutral); when
two accumulators must combine, the narrower is folded into the wider
and dies (the fold costs the dying side's digits — the $Phi$ drop —
never the survivor's); a comparison folds nothing until domination
floors have failed to decide it, and where scales are comparable the
near-cancellation itself is what a subsequent emitted code prices.
"Fold in, read the sign, fold back out" is forbidden — restoring
resurrects digits, and a repeated resurrection is exactly a quadratic
(we will meet the input family that punishes it in @tick).

With the representation and its discipline in hand, the operations
can now be derived — each as a sweep whose every touch names its
funding source.
