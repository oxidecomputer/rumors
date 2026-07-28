#import "fig.typ": *

= Resilience, as a property of the whole <resilience>

The introduction claimed resilience as the property the other three
virtues — asymptotic optimality, small constants, machine affinity —
do not imply and cannot substitute for. The pieces are now all on the
table; this section states the property precisely, and shows that one
discipline produced all four.

== The property <property>

#block(inset: (x: 1.5em), [
  _For every operation and every input a caller can present — any
  value magnitude, any tree depth, any shape; well-formed or
  malformed; crafted or organic — time and transient memory are
  proportional to the bits the operation reads plus the bits its
  answer mandatorily occupies._
])

The word "amortized" in earlier sections needs its scope fixed here,
because it strengthens the claim rather than weakening it: every
accumulator is created and destroyed within a single operation, so
the amortization is internal to one call — each individual API call
is worst-case $O(n + m)$, not merely cheap on average across a
sequence. And one derivational boundary carries over: rank's
freeze-position funding has the uncertified input shape @measures
states, where the linear behavior is enforced by a pinned measured
ceiling rather than derived. Everything else is derived — in this
document, or, for two bounds whose full derivations outgrew it, in
our work with the shapes given here (@join's exact constant,
@tick-output's inequalities) — with one clause stated without proof
(join's subadditivity in the minimum-tick floor, @measures).

Note what the statement does _not_ say. It does not say "fast on
realistic inputs" — that is @machine's separate, additional claim.
It does not bound inputs: no maximum depth, no maximum magnitude, no
"values are expected to be small." It does not exempt failure:
rejecting a malformed stream is an outcome with a cost, bounded like
any other — a defect may sit at a stream's final bit, so no validator
can reject sooner than reading to it; the guarantee is that
rejection never costs more than that one reading. And it does not
average over inputs: the adversary picks the input after reading the
algorithm.

Why hold a clock library to this bar? Because a causal clock is
infrastructure that _meets bytes_: values arrive from other machines,
cross trust boundaries at decode, and are computed with in servers
whose availability is the product. In that position, every
disproportion between input size and computational cost is a
denial-of-service primitive for an adversary and an unexplained outage for
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
    [path sums† (@path-sums)],
    [absolute heights carried down every walk],
    [sweeps over differences the stream itself supplies, held on
      the accumulator],
    [wide decode (@naive-decode)],
    [a growing accumulator re-touched per bit],
    [word-windowed reads; work charged to the code's own width],
    [recursion (@naive-recursion)],
    [a native frame per level],
    [iterative walks; ~2 bits of explicit state per level],
    [carry cliffs† (@ladder)],
    [normalized digits crossed by cheap deltas],
    [the accumulator: no normalized region anywhere (@redundant)],
    [cancelling prefixes† (@sign)],
    [re-scanned dead digits under repeated sign reads],
    [the collapsing fold: each lane scanned once per write that
      raised the top],
    [watermark webs† (@tick-web)],
    [absolute range minima, one per open range],
    [difference-coded stack, zero runs compressed, undercuts funded],
    [close/reopen cycles† (@tick-web)],
    [a boundary difference re-folded per cycle],
    [moves, not folds: wide content shuttles at $O(1)$],
    [output-dominated ops (@projection)],
    [output the input's size cannot bound],
    [denominate against mandatory output; sweep held I/O-linear],
    [tick's emissions (@tick-output)],
    [work priced by output with no output bound],
    [the output inequality: emitted $<= 2 dot$ input $+ O("id")$],
  ),
  caption: [The amplifier genres and their cures. Every cure is the
    funding discipline of @funding instantiated at one seam; the five
    rows marked † bottom out in the accumulator's contract, the
    emissions row is what lets that contract's output-funded clause
    telescope back to input, and the output-dominated row is the one
    place the funding source is the output rather than the input.],
) <fig-genres>

Two structural facts stand out. First, _the accumulator is the
keystone_: the five marked rows bottom out in its contract — amortized
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
  constructed family — for the tick walk's close rule, only after
  an earlier design of the walk had already been built
  (@tick-web) — and each refuting
  family is a permanent regression test. What @accum and @tick
  present as clean derivations were reached by iterated attack.

== Closing <closing>

The skyline representation and its accumulator were presented as an
efficiency story, and they are one: within $4.3%$ of the counting
floor asymptotically, $6.7%$ at hundred-byte sizes (against
the family the coding reaches — the framing @ctf-caveat keeps
honest), linear sweeps for every operation, constants a small
multiple of reading cost, on the access pattern the machine likes
best. But the deeper claim, and the one this document was written to
make legible, is about _worst cases as a design material_. Every
structure here — the delta coding, the balanced digits, the
difference-coded watermarks, the output inequality — was shaped by
asking what the most hostile input could extract, and the finished
design's answer is: nothing beyond the bits it brought and the bits
it is owed back. The complete list of concessions, so the sentence
above cannot be quoted without them: the derivational gap in rank's
funding argument, held by a pinned measurement instead of a proof
(@measures); the probabilistic step in the counting bound's
asymptotic rate (@nonneg); the framing every compactness claim must
carry (@ctf-caveat); the bounded branch-prediction cost the
linear bound absorbs rather than eliminates (@words); the clause
stated without proof in the minimum-tick floor (join subadditivity,
@measures); and the two bounds whose full derivations live in our
work rather than here, their shapes given (@join's exact constant,
@tick-output's inequalities).

That is what it means for the implementation of a paper's elegant
recursive equations to be not only correct, and not only fast, but
resilient to arbitrary adverse inputs: the equations' meaning is
preserved exactly — a boxed, recursive, paper-faithful transcription
remains the permanent oracle its every operation is tested against —
while the costs are rebuilt on a conservation law. Correctness by
transcription, performance by funding; the skyline is where the two
meet in one bit string.

#v(1em)
#line(length: 30%, stroke: 0.5pt + gray-line)

*References.* The subject: P. S. Almeida, C. Baquero, V. Fonte,
"Interval Tree Clocks: A Logical Clock for Dynamic Systems,"
_Principles of Distributed Systems_ (OPODIS 2008), LNCS 5401,
Springer, pp. 259–274; its evaluation section hosts the
space-consumption scenarios reproduced in @id-coding and
@ctf-caveat. Results this document leans on, with their homes:
*signed-digit redundant arithmetic* — A. Avizienis, "Signed-Digit
Number Representations for Fast Parallel Arithmetic," _IRE Trans.
Electronic Computers_ EC-10(3), 1961, pp. 389–400 (the carry-save
adder is the same idea in hardware
dress); *redundant representations amortizing structural work* —
C. Okasaki, _Purely Functional Data Structures_, Cambridge
University Press, 1998, ch. 9;
*exact long accumulation* — U. Kulisch, _Advanced Arithmetic for the
Digital Computer_, Springer, 2002; *the integer codes, and the
competitive framing of universal coding* — P. Elias, "Universal
Codeword Sets and Representations of the Integers," _IEEE Trans.
Information Theory_ IT-21(2),
1975, pp. 194–203; *Kraft completeness* — T. M. Cover, J. A.
Thomas, _Elements of Information Theory_, 2nd ed., Wiley, 2006,
§5.2; *singularity analysis and the square-root-branch transfer* —
P. Flajolet, R. Sedgewick, _Analytic Combinatorics_, Cambridge
University Press, 2009, ch. VI–VII;
*the nonnegative-walk exponent* — E. Sparre Andersen, "On the
Fluctuations of Sums of Random Variables" I–II, _Math. Scand._ 1
(1953), pp. 263–285, and 2 (1954), pp. 195–223. The
composed contract of @accum — the lazy balanced form with a
collapsing sign fold and domination floors, as one interface — is,
to our knowledge, this design's own.
