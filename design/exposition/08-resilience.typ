#import "fig.typ": *

= Resilience, as a property of the whole <resilience>

The introduction claimed resilience as the property the other three
virtues — asymptotic optimality, small constants, machine affinity —
do not imply and cannot substitute for. With every piece now
derived, the property can be stated precisely — and one discipline
turns out to have produced all four.

== The property <property>

#block(inset: (x: 1.5em), [
  _For every operation and every input a caller can present — any
  value magnitude, any tree depth, any shape; well-formed or
  malformed; crafted or organic — time and transient memory are
  proportional to the bits the operation reads plus the bits its
  answer mandatorily occupies, plus — for the exact area measures
  alone — the cost of the integer multiplications their answers
  provably embed._
])

Derived throughout, on both sides where both sides exist: the area
measures' excess over linear carries a floor proven by reduction
and a worst case within one logarithmic factor of it, the factor's
tightness derived with its witness past committed sizes
(@measures). The scope of "amortized", fixed in
@accum-contract, strengthens the claim rather than weakening it:
the amortization is internal to one call, so each individual API
call is worst-case bounded on its own input plus mandatory output,
not merely cheap on average across a sequence. Two bounds whose
full derivations outgrew this document live in our work, their
shapes given here (@join's exact constant, @tick-output's
inequalities); one composition is stated without proof (the
minimum-tick floor's induction over forked histories, @measures).
Everything else is argued here.

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
cross trust boundaries at decode, and are computed with, inside
servers whose availability is the product. In that position, every
disproportion between input size and computational cost is a
denial-of-service primitive for an adversary and an unexplained outage for
an operator — the two audiences differ only in intent. The
authenticated setting this library actually ships in makes hostile
peers unlikely; the bar is held anyway, because "unlikely" is not an
argument availability can rest on, and because — as this work
repeatedly found — every amplification an adversary could exploit
is also a tax some honest workload eventually pays.

== One discipline, every genre <genres>

The same move cured every cost defect this document met: _find the
quantity nothing was paying to maintain, and re-coordinate
it so that every touch has a payer._ One discipline, twelve seams:

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
    [a growing decode buffer re-touched per bit],
    [word-windowed reads; work charged to the code's own width],
    [recursion (@naive-recursion)],
    [a native frame per level],
    [iterative walks; ~2 bits of explicit state per level (two
      priced exceptions: @tick-web, @tick-fusion)],
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
    [never-written runs† (@redundant)],
    [a settling scan stepping a gap no code paid to walk],
    [zero-run certificates consumed whole; scaled reads priced by
      the write watermark],
    [position-dense settles† (@measures)],
    [evicted drift multiplied by absolute positions topology alone
      can densify],
    [anchored segments; a promotion ledger settled once through a
      mass-balanced product tree],
    [answer-embedded products (@measures)],
    [an exact answer as hard as an arbitrary integer product],
    [no cure exists — delegate to sub-quadratic multiplication and
      prove the floor: the one funded superlinearity],
    [output-dominated ops (@projection)],
    [output the input's size cannot bound],
    [denominate against mandatory output; compare through the lazy
      view; the materializing sweep held I/O-linear],
    [tick's emissions (@tick-output)],
    [work priced by output with no output bound],
    [the output inequality:
      emitted $<= 2 dot "size"(e) + 4 dot "size"(i) + 32$],
  ),
  caption: [The amplifier genres and their cures. Every cure is the
    funding discipline of @funding instantiated at one seam; the
    seven
    rows marked † bottom out in the accumulator's contract, the
    emissions row is what lets that contract's output-funded clause
    telescope back to input, the output-dominated row is the one
    place the funding source is the output rather than the input,
    and the answer-embedded row is the one place the "cure" is a
    proof that no cure can exist.],
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
holds it by an adversarial method that produced this document's own
contents:

- *Constructions, not samples.* Every genre in @fig-genres is
  witnessed by a committed input-family generator — `bigroot`, the
  boundary comb, the wide-tooth comb, the descending staircase, the
  reveal comb; `hugeleaf` for wide decode, `deep spine` for
  recursion, the scattered-party comb for the output-dominated row,
  the duplicated-wide-code instance for tick's emissions; the
  cancelled spike for exact-top maintenance, the weight comb and
  the freeze parade for the never-written runs, the freeze
  staircase, the re-arming spine, the punctured tail, and the jump
  pair for the position-dense settles, the puncture product for
  the floor itself — each
  built _to break a candidate
  design_, and kept because each still reads red — on the pinned
  counters — against a re-introduction of the design it refuted.
  The families sit in one committed roster crossed against the
  whole operation surface as a standing dashboard: at this
  writing, a forty-six-family roster, thirty-two of them board
  columns priced on every operation their operand bundles reach —
  1,846 cells, every one green. Green is earned, not
  granted: red is reserved for _untriaged_ contradictions, a
  persistent red is a process failure rather than a status, and
  there is no "accepted red" list — the triage buffer is asserted
  empty, and a cell whose behavior is intended carries a dated
  declared model instead.
- *Deterministic meters, floored as well as ceilinged.* Cost is
  measured in machine-independent counters — bits scanned, digit
  touches, peak transient bytes — with enforced ceilings per input
  byte _and enforced floors_: a meter reading zero where work is
  mandatory means the meter came unhooked, and a ceiling over a dead
  meter proves nothing. Instrumentation is treated as one more
  component that can fail silently.
- *Instruments before cures.* A defect is first pinned _red_ — the
  quadratic measured and committed as a failing threshold — and the
  cure's commit turns exactly that pin green and tightens it. Every
  claim of improvement moves a committed number.
- *White-box worst-case construction, as the closing audit.* The
  roster is not grown by sampling but by reading: for each
  operation, read the implementation, construct the input family
  that maximizes its work — construct, not argue — and diff the
  constructed shapes against the committed roster. The negative
  space is where the blindspots live: the audit's largest catches
  (unfired promotions, answer-embedded products,
  non-shrinking fold accumulators) were all inputs whose essence no
  instrument had generated. Each operation pair also demands its
  _dual_ family — join against meet, forward against reverse — since
  a shape that wedges one walk can read benign on its mirror. The
  final round of that audit constructed nothing that read above its
  committed band: the clean verdict the board's all-green state
  summarizes.
- *The designs in this document are survivors.* The two-zone
  accumulator, the uncompressed watermark stack, fold-on-close, the
  additive output bound, the composed pair measures, freeze
  accounting against absolute positions, the sequential n-ary
  folds, the high-water top, the digit-stepped settlement scan, the
  settle tree balanced by entry count rather than mass: each was a
  plausible design refuted by a
  constructed family — for the tick walk's close rule and for the
  pair measures, only after
  an earlier design had already been built
  (@tick-web, @measures) — and each refuting
  family is a permanent regression test. The clean derivations of
  @accum, @measures, and @tick were reached by iterated attack.

== Closing <closing>

This document presented the skyline representation and its
accumulator as an
efficiency story, and they are one. Within $4.3%$ of the counting
floor asymptotically and $6.7%$ at hundred-byte sizes, against
the family the coding reaches (the framing @ctf-caveat keeps
honest). Linear sweeps for every operation — and for the one
family of answers provably as hard as integer multiplication,
sweeps around products priced at the multiplication bound, floor
and worst case matched to within a logarithm. Constants within an
order of magnitude of decoding cost, decoding itself a bounded
factor above
raw byte movement. All on the access pattern the machine likes
best. But the deeper claim is about _worst cases as a design
material_. Every
structure here — the delta coding, the balanced digits, the
difference-coded watermarks, the mass-balanced settle, the output
inequality — was shaped by
asking what the most hostile input could extract, and the finished
design's answer is: nothing beyond the bits it brought and the bits
it is owed back — and, where the answer itself is a product,
nothing beyond the multiplication it provably contains. The
complete list of concessions, so the sentence
above cannot be quoted without them: the area measures' extreme
tier — the worst case one logarithmic factor above the proven
multiplication floor, the factor's tightness derived but its
witness family too large to sit in a committed test
(@measures); the probabilistic step in the counting bound's
asymptotic rate (@nonneg); the framing every compactness claim must
carry (@ctf-caveat); the bounded branch-prediction cost the
linear bound absorbs rather than eliminates (@words); the
composition stated without proof in the minimum-tick floor — its
induction over forked histories, join subadditivity inside it
(@measures); and the two bounds whose full derivations live in our
work rather than here, their shapes given (@join's exact constant,
@tick-output's inequalities).

That is what it means for the implementation of a paper's elegant
recursive equations to be not only correct, and not only fast, but
resilient to arbitrary adverse inputs: the equations' meaning is
preserved exactly — a recursive, paper-faithful transcription, kept
apart in-tree,
remains the permanent oracle against which every operation is
tested —
while the costs are rebuilt on a conservation law. Correctness by
transcription, performance by funding; the skyline is where the two
meet in one bit string.

#v(1em)
#line(length: 30%, stroke: 0.5pt + gray-line)

*References.* The subject, and the baseline every cost in @naive is
priced against (its appendix's coding included):

- P. S. Almeida, C. Baquero, V. Fonte,
  "Interval Tree Clocks: A Logical Clock for Dynamic Systems,"
  _Principles of Distributed Systems_ (OPODIS 2008), LNCS 5401,
  Springer, pp. 259–274; its evaluation section hosts the
  space-consumption scenarios reproduced in @id-coding and
  @ctf-caveat.

Results this document leans on:

- *signed-digit redundant arithmetic* — A. Avizienis, "Signed-Digit
  Number Representations for Fast Parallel Arithmetic," _IRE Trans.
  Electronic Computers_ EC-10(3), 1961, pp. 389–400 (the carry-save
  adder is the same idea in hardware dress);
- *redundant representations amortizing structural work* —
  C. Okasaki, _Purely Functional Data Structures_, Cambridge
  University Press, 1998, ch. 9;
- *exact long accumulation* — U. Kulisch, _Advanced Arithmetic for
  the Digital Computer: Design of Arithmetic Units_, Springer,
  2002;
- *amortization and the potential method* — R. E. Tarjan,
  "Amortized Computational Complexity,"
  _SIAM J. Algebraic Discrete Methods_ 6(2), 1985, pp. 306–318;
- *the read that rewrites, as amortization made visible* —
  D. D. Sleator, R. E. Tarjan, "Self-Adjusting Binary Search
  Trees," _J. ACM_ 32(3), 1985, pp. 652–686;
- *integer codes, and the competitive framing of universal
  coding* — P. Elias, "Universal
  Codeword Sets and Representations of the Integers," _IEEE Trans.
  Information Theory_ IT-21(2), 1975, pp. 194–203;
- *the zigzag fold of @coding* — folklore, popularized by Protocol
  Buffers' signed varints;
- *Kraft completeness* — T. M. Cover, J. A. Thomas, _Elements of
  Information Theory_, 2nd ed., Wiley, 2006, §5.2;
- *parameterized run-length codes* —
  S. W. Golomb, "Run-Length Encodings," _IEEE Trans. Information
  Theory_ IT-12(3), 1966, pp. 399–401; R. F. Rice, "Some Practical
  Universal Noiseless Coding Techniques," JPL Publication 79-22,
  1979 (weighed in @ctf-caveat);
- *succinct tree encodings* — G. Jacobson, "Space-Efficient
  Static Trees and Graphs," _Foundations of Computer Science_
  (FOCS 1989), pp. 549–554; J. I. Munro,
  V. Raman, "Succinct Representation of Balanced Parentheses and
  Static Trees," _SIAM J. Computing_ 31(3), 2001, pp. 762–776 (the
  preorder-flag spelling of @coding belongs to this literature);
- *singularity analysis and the square-root-branch transfer* —
  P. Flajolet, R. Sedgewick, _Analytic Combinatorics_, Cambridge
  University Press, 2009, ch. VI (the standard function scale and
  its transfer, Thms VI.1 and VI.3) and §VII.6 (irreducible
  context-free structures);
- *nonnegative-walk exponent* — E. Sparre Andersen, "On the
  Fluctuations of Sums of Random Variables" I–II, _Math. Scand._ 1,
  1953, pp. 263–285, and 2, 1954, pp. 195–223.

The composed contract of @accum — the lazy balanced form with a
collapsing sign fold and domination floors, as one interface — is,
to our knowledge, this design's own; so are the minimum-tick
measure and its identity (@measures), the join size inequality
(@join), and the count of the _canonical_ grammar under these rules
and this payload code (@compactness — the preorder-flag encoding
itself is the classical succinct representation above). A
literature search stands behind that sentence. Its nearest
findings, and where each differs:

- *compressed causality metadata* — D. Malkhi, D. B. Terry,
  "Concise Version Vectors in WinFS," _Distributed Computing_
  20(3), 2007, pp. 209–219; P. S. Almeida, A. Shoker, C. Baquero,
  "Delta State Replicated Data Types," _J. Parallel and Distributed
  Computing_ 111, 2018, pp. 162–173. Both compress _which events a
  peer has seen_ — per-replica counter sets, gaps allowed — over
  fixed replica identifiers. Neither is a bit-level coding of a
  tree-structured clock, and neither measures itself against a
  counting floor or prices adversarial inputs.
- *tree-shaped clocks* — U. Mathur, A. Pavlogiannis, H. C. Tunç,
  M. Viswanathan, "A Tree Clock Data Structure for Causal Orderings
  in Concurrent Executions," ASPLOS 2022, pp. 710–725: a tree over
  per-thread counters that makes in-memory join and copy sublinear
  for dynamic race prediction. An operation-cost result on a
  different clock, with no wire form, no canonical bytes, and no
  adversarial model.
- *the sign query* — sign and zero detection for signed-digit
  operands is treated in depth as a worst-case circuit problem in
  B. Parhami, "Generalized Signed-Digit Number Systems: A Unifying
  Framework for Redundant Number Representations," _IEEE Trans.
  Computers_ 39(1), 1990, pp. 89–98. The search found no amortized,
  collapse-on-read treatment, and none of the domination-floor
  comparisons the collapse makes possible.

The ITC implementations the same search surfaced — the paper's
authors' reference implementations and later independent ones —
keep the paper's appendix coding, the baseline @naive prices.
Ownership here still means only that a genuine search found no
prior statement of these four — not that none exists.
