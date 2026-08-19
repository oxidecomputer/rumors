#import "fig.typ": *

= The accumulator <accum>

Every sweep in this document maintains at least one _running signed
integer_. Validation carries the running height (@validation).
Comparison carries the running difference between two heights
(@cmp). The area measures carry running weighted sums (@measures).
The tick walk carries a whole small web of running range minima
(@tick). Each is updated by a stream of deltas — mostly tiny,
occasionally enormous — and each is consulted mostly for its _sign_.

The representation of these running integers is not an implementation
detail. Every cost argument in @operations bottoms out in it, and
the resilience thesis is won or lost there.

== The contract <accum-contract>

From the sweeps' side, the accumulator must support four value
operations and four housekeeping ones — the latter for the
bookkeeping across a sweep's several live accumulators (@tick) and
the one width introspection the weighted folds need (@measures):

+ *apply a signed machine-word delta*, in amortized $O(1)$ work;
+ *apply a wide delta* of $ell$ machine words in amortized $O(ell)$
  work, independent of the magnitude already held — and optionally
  _scaled_ by $2^s$ for arbitrary $s$ (the scaled variant is a
  distinct _mode_, unpacked below the list);
+ *read the sign* of the held value, in amortized $O(1)$ — the
  amortization charged, where the read must descend, to the
  unscaled writes that raised the held top (@sign derives the
  charge);
+ *materialize* the held value as an ordinary integer, in work
  proportional to that value's own width. The operation opens with
  the same collapsing fold the sign query runs (@sign), which
  brings the held spelling within two digits of the value's width.
  A scaled-mode accumulator is instead read out through its _write
  watermark_: the value returns as a magnitude and a power-of-two
  scale, priced by the span from the lowest position written since
  the accumulator last emptied up to its top — the untouched scale
  prefix beneath the writes is never spelled and never walked;
+ *move* a held value between slots, $O(1)$ — a buffer swap (a
  walk parking a boundary quantity aside rather than folding it);
+ *fold* one accumulator into another, at the cost of the _dying_
  operand's held lanes, never the survivor's (two running minima
  meeting when a range closes);
+ *compare* two held values through their domination floors
  (@sign): $O(1)$ where a floor decides, the fold's price where
  none does (a running gap tested against a parked quantity,
  neither materialized);
+ *report the held top's index*, $O(1)$ — the width test behind the
  weighted folds' freeze trigger (@measures).

Requirement 2's scaled mode, unpacked. The scale routes the words
to higher lanes at the same $O(ell)$; its one further cost is a
one-time zero-fill of the lanes below $s\/32$, charged once per
rise of the allocation high-water mark. The caller must therefore
hold a code that pays for $s$, as the folds of @measures do — the
depth bits that certified the scale. And the mode trades away the
sign read (@sign derives why). Exactly one family of folds uses it:
the weighted folds of @measures, where a plateau's height is added
at a position weight, and where the independence from $s$ is what
makes the folds linear.

The bounds must hold on _every_ interleaving of these operations —
not in expectation, not for typical streams — with the single
stated exception that the scaled mode forgoes sign reads. One word
on "amortized",
binding for the whole document: every accumulator is created and
destroyed inside a single API operation, so amortization is always
internal to one call — each operation is worst-case linear on its
own, not merely cheap on average across a sequence. (Deltas arrive
by two routes: a payload lives inline in machine words until
it outgrows two of them — @words — so the inline paths, requirement
1 and requirement 2 at $ell <= 2$, are the hot ones and the
heap-limb route the rarity.)

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

Now add one parameter to the comb — a tooth width — and the repair
fails: the *wide-tooth comb* of
@families. Give the teeth any width past the window: deltas of
$plus.minus 2^192$, say, oscillating across a cliff at $2^k$ with $k$
much larger still. Each tooth's code
costs about 387 bits (the zigzag fold adds one to the magnitude's
exponent); each application punches through the
word-sized window into the normalized prefix and ripples the full
$k$-bit carry: work per tooth proportional to $k$, unbounded relative
to the tooth's own code. Widen the window and the teeth widen past it
again.

#figure(
  attack(
    [wide-tooth comb$(t, w, k)$],
    [any settled/pending split — the two-zone repair],
    stack(dir: ttb, spacing: 4pt,
      skyline(
        ((0.125, 2), (0.125, 1), (0.125, 2), (0.125, 1),
         (0.125, 2), (0.125, 1), (0.125, 2), (0.125, 1)),
        w: 200pt, unit: 16pt, show-heights: false,
      ),
      text(size: 7.5pt, fill: gray-line.darken(40%),
        [the boundary comb with a width dial: teeth of
         $plus.minus 2^w$ astride the $2^k$ cliff, $w$ chosen past
         whatever window the design fixed]),
    ),
    [every tooth punches through the pending window into the
     normalized prefix: a full $k$-bit carry per
     $approx 2w$-bit code, at any fixed window width.],
    cure: [no normalized region anywhere (@redundant): with every
      digit lazy there is no boundary to widen past, and a
      $plus.minus 2^w$ tooth costs $O(w\/32)$ touches — its own
      limbs.],
  ),
  kind: image,
  caption: [The wide-tooth comb attack card: the parameter that
    kills every windowed repair, forcing the zone to be
    everything.],
) <fig-attack-widetooth>

The lesson generalizes and is worth stating as a principle:

#block(inset: (x: 1.5em), [
  _Any representation with a normalized region has a boundary between
  "settled" and "pending" digits, and there exists an input stream
  that oscillates the value across that boundary at unit cost per
  crossing to the input and full-carry cost per crossing to the
  representation._
])

The principle is a design heuristic, not a theorem, and a
motivation rather than a load-bearing step: we do not prove
that no cleverer boundary exists; the natural candidate, a window
whose width adapts to the widest delta yet seen, has no funded
form that we know. And no claim below rests on the principle,
which is why it does not join @closing's concessions. The design
takes the one escape that
needs no such proof — remove the boundary. No digit is ever
"settled": *no normalized region
anywhere*.

== Balanced redundant digits <redundant>

Hold the value as digits in base $2^32$, little-endian, each digit a
_signed_ 64-bit integer kept in the _lazy zone_ $|a_i| < 2^33$. The
base's width is half the lane's on purpose: the spare bits are what
the scheme spends. With digits under $2^33$ over a $2^32$ base, an
unscaled word
delta's two halves, a carry, and the recentering all fit the lane's
own 64-bit arithmetic; a scale-shifted landing (below) runs in the
machine's double-width arithmetic before its remainder recenters
back into the lane.

$ "value" = sum_i a_i dot 2^(32 i), quad a_i in (-2^33, 2^33). $

_Balanced_ here means only that the digit set is symmetric about
zero. The classical minimally-redundant balanced sets stop near the
base; ours runs to twice it, and that extra slack is the mechanism.

Two deliberate redundancies. The digits are signed and may exceed the
base, so a given value has many spellings — that freedom is the
storage the design absorbs oscillation into. And the zone is _twice_
the base, so a digit can wander a long way before anything must be
done. (If an image helps: an abacus rod allowed to hold a surplus of
beads, and owing-beads too, so a carry need not propagate the moment
a column fills.) The idea's machine-arithmetic ancestry is
Avizienis's signed-digit number systems and the carry-save adder;
its software ancestry is the redundant number representations of the
purely functional data-structure tradition, and Kulisch's long
accumulators for exact summation.

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
again. And the upward repeat is a trickle, not a cascade: past the
digits a landing itself spans, a zone-bounded digit plus an incoming
carry stays within $2^34$, so the carry passed onward obeys
$|c| <= 4$ — four units of drift against the $3 dot 2^31$ the next
digit must absorb before carrying in turn. Every carry is
funded: a digit that carries cannot carry again until deltas
totalling $3 dot 2^31$ in net movement have landed on it (carries
included: a carry is just more drift on the same ratchet), so the
carries out of a digit are at most its arriving drift
divided by $3 dot 2^31$, and a word delta costs amortized $O(1)$
digit touches on every
stream. And because _every_ write recenters what it touches, no digit
anywhere is ever "settled": the two-zone counterexample has no
boundary to aim at.

The zone's width is itself a dial between this section's two
mechanisms, worth turning once. Write it $2^z$. The ratchet demands
$2^z - 2^31$ of drift per carry: wider is stronger. @sign's
unscanned-tail bound grows as $(2^z - 1)\/(2^32 - 1)$: wider is
weaker, inflating the stop threshold and the domination gap with it.
$z = 33$ keeps the tail a hair over 2, so the stop test is one
comparison against 3, while the ratchet already stands at
$3 dot 2^31$.

The carry cliff itself, walked through the digits, since the whole
section exists to defeat it. Hold $2^k - 1$: every 32-bit digit at
$2^32 - 1$, each comfortably inside the zone. Add $1$: it lands at
digit 0, which becomes $2^32$ — still inside the zone, so nothing
carries; one touch. Subtract $1$: one touch back. The boundary
comb's $t$ teeth cost $t$ touches where every normalized
representation paid $t dot k$ — the $k$-bit carry is not deferred;
it is dissolved, because $2^32$ at digit 0 is simply another
spelling of the carried form, and this representation is allowed to
hold it.

One storage remark, so the accounting has no hidden pocket. Two
quantities must not be confused: _allocated lanes_ — the dense
little-endian vector, which only grows; and _held lanes_ — the
lanes up to a tracked index of the highest nonzero digit, an index
raised by writes and lowered by collapses and cancellations. A delta
landing
at a new highest lane zero-fills the gap below it once, because the
allocation high-water mark only rises; the total zero-fill over a
sweep is bounded by the final lane count, and the largest scale any
sweep uses is bounded by the operand's depth, whose topology bits
the sweep already read — funded, once. Every fold and every
materialization starts at the tracked top index and is denominated
in _held_ lanes, so lanes above the held value, zeroed or never
touched, are never scanned again.

Keeping that top index _exact_ has a cost of its own, and it is the
one place a scan with no funding source could hide. When a write
cancels the highest nonzero digit, the new top is the next nonzero
digit below — and between a scaled write's landing site and the
digits beneath it lies a run of never-written zeros that no code
ever paid to walk. An alternating pair of far-apart scaled writes
would make a naive settling scan walk that run again and again,
forever, at a price that grows with the scale. The accumulator
instead keeps a ledger of _zero-run certificates_: a write that
lands above the current top records the run it jumped as one entry,
$O(1)$ whatever the run's width; a scan that reaches a certified
run consumes the certificate and crosses the run whole, one touch;
a write whose carries land inside a certified run splits the entry
around the digits actually written. Each certificate is created
once, by the write that jumped the run, and consumed at most once,
so top maintenance never exceeds the metered work that funded it —
amortized $O(1)$ per write beyond the write's own deposits, at any
scale, on any schedule. The alternative — tracking the top as a
high-water mark that only rises — silently re-prices every later
read at the highest lane ever touched, long after a cancellation
emptied it; @sign constructs the input pair that punishes exactly
that substitution.

#figure(
  {
    lanes((
      ("+9", [$a_4$]),
      ("−5 000 000 000", [$a_3$]),
      ("+6", [$a_2$]),
      ("0", [$a_1$]),
      ("−1", [$a_0$]),
    ))
    v(4pt)
    align(center, text(size: 8.5pt, fill: gray-line.darken(35%),
      [value $= 9 dot 2^128 - 5 space 000 space 000 space 000 dot 2^96 + 6 dot 2^64 - 1$,
       every digit inside $(-2^33, 2^33)$]))
  },
  kind: image,
  caption: [Digit lanes of the accumulator, drawn most-significant
    lane leftmost for readability (storage is little-endian). Digits
    are signed and may exceed the base $2^32$ — $a_3$ does, standing
    past the base yet inside the zone; each write recenters
    only the digits it touches, so a small delta lands in $a_0$ and
    stops. Nothing here is normalized, deliberately.],
) <fig-lanes>

A wide delta arrives as a sign and an $ell$-word magnitude, and
routes each 64-bit word into its two 32-bit digit positions
independently — positions $2i$ and $2i + 1$ for word $i$. A scale
splits into whole lanes and a residual shift under 32: each half is
shifted by the residual as it lands and routed at the whole-lane
offset, so no separate pre-shift pass
exists. A shifted half spans up to 63 bits — which is why landings
run double-width: the half lands whole at its position, and the
recentering carry (the rule here, not the exception, and $O(1)$
either way) moves the overflow into the next lane, a bounded few
carries per half.
Cost: $O(ell)$ touches regardless of what the accumulator already
holds and regardless of the scale, which is requirement 2 exactly.
Negation flips digit signs in place (a balanced digit's negation is a
balanced digit); subtraction is negated addition.

== Reading the sign: domination and collapse <sign>

The sign query looks troublesome: redundancy means the top digit's
sign can simply be wrong ($a_1 = +1$, $a_0 = -2^33 + 1$ has a
positive top digit and a decisively negative value), so no fixed
number of high digits settles the sign in general. Fold digits from
the top, maintaining a running partial $sigma$ by

$ sigma <- sigma dot 2^32 + a_i quad ("digit index" i "descending"), $

which after scanning down to index $i$ equals, in closed form,
$sum_(j = i)^("top") a_j dot 2^(32(j - i))$: the _exact_ value of the
scanned suffix in units of $2^(32 i)$. (The fold only continues while
$|sigma| <= 2$, so after a step $|sigma| < 2 dot 2^32 + 2^33 = 2^34$ — the
partial itself always fits comfortably in fixed-width arithmetic.) The digits
not yet scanned contribute, in the same units, at most

$ sum_(j < i) (2^33 - 1) dot 2^(32(j - i)) < (2^33 - 1) / (2^32 - 1) approx 2.0000000002, $

a hair over $2$. So the moment $|sigma| >= 3$, the suffix _dominates_
everything below: the sign is decided, stop — at the top digit
itself for most reads (measured, across the instrumented corpora).

An adversary can prevent that: write $+2^k$, then $-(2^k - 1)$, and
the top digits cancel to a whisper — the fold must descend toward
digit 0 to find the surviving $+1$. Descending is honest work, but it
must not be repeatable for free: a stream of cheap sign queries
against one expensive cancelling prefix would re-scan it each time.
The fix makes the read pay forward: when the fold descends, it
_collapses_ what it scanned — zeroes the scanned digits and deposits
their exact partial $sigma$ at the scan's floor (a bounded write: the
fold's invariant keeps $sigma$ within two digits' range). The value is
unchanged; the spelling is now shallow; the next sign query re-reads
none of it — the fold starts at the tracked top-of-held index, which
the collapse just lowered. Concretely, at $k = 96$: after $+2^96$
the lanes $(a_3, a_2, a_1, a_0)$ read $(1, 0, 0, 0)$; after
$-(2^96 - 1)$, whose magnitude lands lane by lane, they read
$(1, -(2^32 - 1), -(2^32 - 1), -(2^32 - 1))$ — every digit in the
zone, the value a whisper. The sign fold descends: $sigma = 1$,
then $1 dot 2^32 - (2^32 - 1) = 1$ at each further lane, reaching
digit 0 with $sigma = 1$ — the value, exactly. The collapse zeroes
the four scanned lanes, deposits $1$ at lane 0, and lowers the
held top to 0: the vector now reads $(0, 0, 0, 1)$, and the next
sign query costs one touch. So *each held lane is scanned at most
once per write that raised the held top above it* — and a run of
never-written lanes is not scanned even once: the fold with a
nonzero partial decides within a step of entering one, and a fold
or settling scan carrying a zero partial consumes the run's
certificate (@redundant's storage remark) and crosses it whole.
The exactness of the top matters as much as the collapse. Consider
two long unit-step streams whose second codes both spike $2^(32 g)$:
the comparison sweep folds the two spikes into its running
difference at one boundary — they cancel, leaving a value of one
digit under a buffer $g$ digits tall — and then reads the sign once
per remaining boundary, thousands of reads with no intervening
write. With the top settled at the surviving digit, each read is
one touch; with a high-water top, each read re-walks the spike's
$g$ dead digits — $Theta(m dot g)$ on a $Theta(m + g)$-bit pair,
the cost the spike's own code paid exactly once.

#figure(
  attack(
    [cancelled spike$(g, m)$],
    [exact-top maintenance under repeated sign reads],
    stack(dir: ttb, spacing: 5pt,
      oprow([operand $a$], codestrip((
        ([1], 16pt, "p"), ([spike $2^(32g) + 1$], 76pt, "w"),
        ([1], 16pt, "p"), ([1], 16pt, "p"), ([$dots.c$], 14pt, "x"),
        ([1], 16pt, "p"), ([0], 16pt, "p"),
      ))),
      oprow([operand $b$], codestrip((
        ([2], 16pt, "p"), ([spike $2^(32g) + 2$], 76pt, "w"),
        ([2], 16pt, "p"), ([2], 16pt, "p"), ([$dots.c$], 14pt, "x"),
        ([2], 16pt, "p"), ([0], 16pt, "p"),
      ))),
      text(size: 7.5pt, fill: gray-line.darken(40%),
        [same shape, same boundaries: the spikes cancel inside
         $D$ at one boundary, then $Theta(m)$ boundaries of sign
         reads follow with no intervening write]),
    ),
    [under a high-water top, every one of the $Theta(m)$ sign reads
     re-walks the spike's $g$ dead digits: $Theta(m dot g)$ on a
     $Theta(m + g)$-bit pair.],
    cure: [the settled top: the cancelling write's own collapse
      lowers the top to the surviving digit, so each remaining
      read is one touch — the spike's width is paid exactly once,
      by its own code.],
  ),
  kind: image,
  caption: [The cancelled spike attack card: a pair whose one
    cancellation leaves a tall dead buffer over thousands of
    reads.],
) <fig-attack-spike>

The charge is honest exactly when the write can pay it, and an
unscaled write can: a delta of $w$ magnitude bits places its own
digits no higher than lane $w\/32 + 1$, a span its application
already touched and its code already funded — and it raises the top
beyond that span only through carries, which @redundant's ratchet
already amortizes against the writes that provoke them. The
top-raising a sweep can buy is therefore $O$(its input bits), sign
queries amortize against
the same writes, and requirement 3 holds on every
interleaving of reads with unscaled writes.

#figure(
  attack(
    [cancelling chain$(t, k)$],
    [the sign read, through cancelling prefixes],
    stack(dir: ttb, spacing: 4pt,
      skyline(
        ((0.125, 2), (0.125, 0), (0.125, 2), (0.125, 0),
         (0.125, 2), (0.125, 0), (0.125, 2), (0.125, 0)),
        w: 200pt, unit: 16pt, show-heights: false,
      ),
      text(size: 7.5pt, fill: gray-line.darken(40%),
        [$t$ drops from a $2^k$ peak to $1$ (drawn as 2 and 0):
         after each wide drop the held value is tiny but spelled
         by a high digit cancelled by a trail of negatives]),
    ),
    [every sign read after a drop must descend the whole cancelling
     prefix — no fixed number of top digits decides — and a design
     that re-reads it per query pays the prefix once per read.],
    cure: [the collapsing fold (@sign): the descent zeroes what it
      scanned and deposits the exact partial at the floor, so each
      lane is read once per write that raised the top — here, once
      per drop, funded by the drop's own wide code.],
  ),
  kind: image,
  caption: [The cancelling chain attack card: wide writes that
    leave whispers, aimed at repeated sign reads.],
) <fig-attack-cancelling>

The _scaled_ write of requirement 2 is the stated exception, and it
carries a discipline. A delta scaled by $2^s$ lands its $ell$ words
at lane $s\/32$ without spelling the lanes beneath: an $O(ell)$ code
opens a span no code paid to scan, and a stream alternating cheap
scaled writes with sign reads would march the fold across that span
once per round — a quadratic with no payer. (The depth bits that
certified the scale fund the once-per-high-water zero-fill of
@accum-contract — one traversal, not one per read; the per-read
traversal is exactly what a scaled write cannot pay for.) The discipline, part of
the contract: *an accumulator that receives scaled writes is never
sign-read*. It is write-only until read out through its watermark,
and each read-out must be separately funded — the weighted folds of
@measures, the only scaled writers in the system, price every
read-out against the topology bits that certified the scales and
the written span the watermark reports.

Two consequences deserve a pause. First, _reads mutate_: the sign
query rewrites the representation (value-preservingly). That is
unusual as an interface but it is exactly the amortization made
visible — the data structure's version of a splay. Second, the
decision bound generalizes: if the fold decided at digit index
$i >= f + 2$, then no quantity living entirely in digits
$0 dots f$ — anything of bounded scale — could overturn either the
sign or a magnitude comparison. The arithmetic behind the "$+2$":
deciding at index $i$ means $|sigma| >= 3$ against under $2.01$ of
unscanned tail, certifying $|"value"| > 0.99 dot 2^(32 i)$; a
quantity confined to digits $0 dots f$ is below
$2.01 dot 2^(32 (f + 1))$; with $i >= f + 2$ the first exceeds the
second by a factor near $2^31$ — a certified _floor_ under the
decided value against a certified _ceiling_ over the confined one,
which settles a magnitude comparison between them, not
merely a sign. Sweeps use this _domination floor_
constantly: a watermark whose fold decided at digit index 5 stands
at least $0.99 dot 2^160$; no adjustment confined to digits 0 and 1 —
any machine-word quantity — can bring it near zero, and the comparison
ends after those one or two top digits, in $O(1)$, without touching
the watermark's width at all (@tick).

Materializing the held value for output — the one place normalization
happens — is a single low-to-high pass with a signed carry, costing
the held lane count. After a collapse, the held lane count exceeds
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
  input code being consumed (together with the carries its landing
  provokes, which @redundant's ratchet charges back to earlier
  arrivals on the same digit); an output code being
  emitted, which licenses reads
  up to its own width; or held lanes dying that some earlier code
  already paid for — each lane dies at most once per write that
  opened it._
])

Equivalently, put the potential $Phi$ at the number of _held_ lanes
across every live _unscaled_ accumulator, each accumulator counting
the lanes up to its tracked top. (The scaled mode stands outside
the potential from the start, under @sign's discipline, its span
priced at materialization.) A lane _dies_ when a collapse or a
cancelling write lowers the top past it; allocated storage above
the top (@redundant's storage remark) counts for nothing. $Phi$
grows only when input codes are consumed, and then by at most one
lane per code, plus one per 32 bits of the code's width, in each
accumulator the code folds into — the span
bound of @sign, whose $+1$ is the carry, itself already amortized
by @redundant's ratchet. Each code folds into $O(1)$ live
accumulators: a rule every sweep below obeys, and the reason
@tick-web codes its watermarks as shift-invariant differences. Over
a sweep, $Phi$'s growth is $O("input bits")$ — an unscaled code
funding every lane up to the top it can set. A collapse zeroes its
scanned span,
deposits at most two digits at the scan's floor, and lowers the top
to the deposit, so its touches equal its $Phi$ drop plus a constant
and it never increases $Phi$. Every touch not covered by a
consumed or emitted code is covered by a drop in $Phi$, so total
work over the sweep is $O("input bits" + "output bits")$.

The discipline has teeth. As rules of craft:

- wide values are _moved_, never copied — a move is a buffer swap,
  $O(1)$, $Phi$-neutral;
- when two accumulators must combine, the narrower is folded into
  the wider and dies; the fold costs the dying side's digits — the
  $Phi$ drop — never the survivor's;
- a comparison folds nothing until domination floors have failed
  to decide it. Where scales are comparable, one of two payers
  steps in: a boundary whose emitted code prices the read (join's
  switch, @join), or the narrower operand folding into the wider
  and dying, the $Phi$ drop paying (the parked differences of
  @tick).
"Fold in, read the sign, fold back out" is forbidden — restoring
resurrects digits, and a repeated resurrection is exactly a quadratic
(we will meet the input family that punishes it in @tick).

With the representation and its discipline in hand, we can derive
the operations — each a sweep whose every touch names its
funding source.
