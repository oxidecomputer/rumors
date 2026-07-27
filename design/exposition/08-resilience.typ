#import "fig.typ": *

= Resilience, as a property of the whole <resilience>

The introduction promised that the four virtues claimed for this
design — asymptotic optimality, small constants, machine affinity,
and resilience — are one property wearing four faces. The pieces are
now all on the table; this section assembles the claim and says
precisely what it means.

== The property <property>

#block(inset: (x: 1.5em), [
  _For every operation and every input a caller can present — any
  value magnitude, any tree depth, any shape; well-formed or
  malformed; crafted or organic — time and transient memory are
  amortized proportional to the bits the operation reads plus the
  bits its answer mandatorily occupies. There is no input family, at
  any scale, whose cost grows faster than that._
])

Note what the statement does _not_ say. It does not say "fast on
realistic inputs" — that is @machine's separate, additional claim.
It does not bound inputs: no maximum depth, no maximum magnitude, no
"values are expected to be small." It does not exempt failure:
rejecting a malformed stream is an outcome with a cost, bounded like
any other, even though a stream's defect may be discoverable only at
its final bit (self-delimiting codes guarantee an honest validator
that much work; the guarantee is that it is never more). And it does
not average over inputs: the adversary picks the input after reading
the algorithm.

Why hold a clock library to this bar? Because a causal clock is
infrastructure that _meets bytes_: values arrive from other machines,
cross trust boundaries at decode, and are computed with in servers
whose availability is the product. In that position, every
disproportion between input size and computational cost is a denial-
of-service primitive for an adversary and an unexplained outage for
an operator — the two audiences differ only in intent. The
authenticated setting this library actually ships in makes hostile
peers unlikely; the bar is held anyway, because "unlikely" is not an
argument availability can rest on, and because — the campaign's
repeated experience — every amplification an adversary could exploit
is also a tax some honest workload eventually pays.

== One discipline, every genre <genres>

Each cost defect this document met was cured by the same move:
_identify the quantity whose maintenance was unfunded, and re-coordinate
it so that every touch has a payer._ The table is worth reading as a
unity:

#figure(
  table(
    columns: (auto, 1fr, 1fr),
    align: (left, left, left),
    stroke: 0.4pt + gray-line,
    inset: 6pt,
    table.header([*genre*], [*the unfunded quantity*], [*the cure*]),
    [path sums (@naive)],
    [absolute heights carried down every walk],
    [sweeps over differences the stream itself supplies],
    [wide decode (@naive-decode)],
    [a growing accumulator re-touched per bit],
    [word-windowed reads; work charged to the code's own width],
    [recursion (@naive-recursion)],
    [a native frame per level],
    [iterative walks; ~2 bits of explicit state per level],
    [carry cliffs (@ladder)],
    [normalized digits crossed by cheap deltas],
    [the accumulator: no normalized region anywhere (@redundant)],
    [cancelling prefixes (@sign)],
    [re-scanned dead digits under repeated sign reads],
    [the collapsing fold: each digit scanned once per write],
    [watermark webs (@tick)],
    [absolute range minima, one per open range],
    [difference-coded stack, zero runs compressed, undercuts funded],
    [close/reopen cycles (@tick)],
    [a boundary difference re-folded per cycle],
    [moves, not folds: wide content shuttles at $O(1)$],
    [output-dominated ops (@projection)],
    [output the input's size cannot bound],
    [denominate against mandatory output; sweep held I/O-linear],
    [tick's emissions (@tick)],
    [work priced by output with no output bound],
    [the output inequality: emitted $<= 2 dot$ input $+ O("id")$],
  ),
  caption: [The amplifier genres and their cures. Every cure is the
    funding discipline of @funding instantiated at one seam.],
) <fig-genres>

Two structural facts stand out. First, _the accumulator is the
keystone_: six of the nine rows bottom out in its contract — amortized
$O(1)$ word deltas at any held width, $O("limbs")$ wide deltas at any
scale, amortized $O(1)$ sign, funded materialization. It is why the
introduction called it half of the answer rather than an
optimization: remove it and every sweep's ledger reopens at the
carry cliff. Second, _the discipline composes_: each operation's
argument was local (this fold is funded by that code), and the
whole-system claim is just their sum — there was no global argument
to make, because conservation laws add.

== How the claim is held, not just made <method>

A claim of this shape cannot be established by testing what occurs to
the implementer — the quantifier is over _all_ inputs, and the
dangerous ones are precisely the unimagined ones. Our implementation
holds it by an adversarial method worth recording, briefly, because
the document's own contents were produced by it:

- *Constructions, not samples.* Every genre in @fig-genres is
  witnessed by a committed input-family generator — `bigroot`, the
  boundary comb, the wide-tooth comb, the descending staircase, the
  reveal comb, and their kin — each built _to break a candidate
  design_, and kept forever once it has.
- *Deterministic meters, floored as well as ceilinged.* Cost is
  measured in machine-independent counters — bits scanned, digit
  touches, peak transient bytes — with enforced ceilings per input
  byte _and enforced floors_: a meter reading zero where work is
  mandatory means the meter came unhooked, and a ceiling over a dead
  meter proves nothing. Instrumentation is treated as a thing that
  can itself fail silently.
- *Instruments before cures.* A defect is first pinned _red_ — the
  quadratic measured and committed as a failing threshold — and the
  cure's commit turns exactly that pin green and tightens it. Every
  claim of improvement moves a committed number.
- *The designs in this document are survivors.* The two-zone
  accumulator, the uncompressed watermark stack, fold-on-close, the
  additive output bound: each was a plausible design refuted by a
  constructed family before (or after) shipping, and each refuting
  family is a permanent regression test. What @accum and @tick
  present as clean derivations were reached by iterated attack.

== Closing <closing>

The skyline representation and its accumulator were presented as an
efficiency story, and they are one: within $4.3%$ of the counting
floor at rest, linear sweeps for every operation, constants a small
multiple of reading cost, on the access pattern the machine likes
best. But the deeper claim, and the one this document was written to
make legible, is about _worst cases as a design material_. Every
structure here — the delta coding, the balanced digits, the
difference-coded watermarks, the output inequality — was shaped by
asking what the most hostile input could extract, and the finished
design's answer is: _nothing_. Nothing beyond the bits it brought
and the bits it is owed back.

That is what it means for the implementation of a paper's elegant
recursive equations to be not only correct, and not only fast, but
resilient to arbitrary adverse inputs: the equations' meaning is
preserved exactly — a boxed, recursive, paper-faithful transcription
remains the permanent oracle its every operation is tested against —
while the costs are rebuilt on a conservation law. Correctness by
transcription, performance by funding; the skyline is where the two
meet in one bit string.
