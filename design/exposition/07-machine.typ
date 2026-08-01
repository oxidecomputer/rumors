#import "fig.typ": *
#import "viz.typ": *

= Constants, and the machine <machine>

Asymptotics chose the representation; constants are where it earns
its keep on real hardware. This section states
where the measured costs of our implementation sit and — more
durably — _why_ the skyline is the shape the machine wants. Numbers
here are measured (release builds, deterministic inputs, medians).
The body of the chapter quotes them as mechanism and magnitude,
because magnitudes drift with hardware and mechanisms do not;
@measured then closes the chapter with the corpora of record behind
them — what was measured, on what, and the headline readings.

== The floor is reading <floor>

The packed form has no random access: positions, extents, and values
are implicit in self-delimiting codes, so every operation pays one
sequential parse of its operands _by construction_. That sets a floor
— you cannot answer a whole-value question below the cost of reading
the value — and makes the honest question about constants: _how close
does each operation sit to its own reading cost?_

The measured picture, per packed input byte. Raw byte movement is
the floor, and each band is quoted against the one below it:

- *Encode, hash, equality* sit at memory-copy speed (a tenth of a
  nanosecond per byte and below): the resting form is the wire form
  (@canonical), so these operations do not process the value at all —
  they move or compare bytes.
- *Validating decode, comparison, the party predicates* sit between
  a few and a few tens of nanoseconds per byte — comparison at the
  low end, within a small multiple of simply visiting every bit
  (it exits early on most organic pairs and re-checks no
  canonicality: decode already did);
  validating decode toward the high end. This is
  the price of decoding every code once,
  plus the canonicality checks.
- *The arithmetic sweeps* — join, meet, tick, projection, rank —
  sit roughly an order of magnitude above _validating decode_: tens
  to low
  hundreds of nanoseconds per byte, the cost of decoding every
  payload, folding accumulators, and re-coding an output stream.
- *Composites* (receive $=$ decode $+$ join $+$ tick; the mutual
  exchange) pay their parts' passes, a few hundred nanoseconds per
  byte at worst.

Two properties of this picture matter more than its values. The
_ordering_ is exactly the ordering of how much each operation must
do per bit: nothing pays for machinery it does not use. (Each
operation's band sits above the previous at matched operand sizes;
the bands' edges overlap only because each aggregates several
operations.) And the values
are _flat across input shapes_: the adversarial families of the
previous sections land in the same bands as organic values of the
same size, with the two priced exceptions already stated — the
area measures' settle products, which run at the multiplication
bound their floor proves mandatory (@measures), and the prediction
cost of @words. All
per-byte figures are taken over the bench corpus's organic sizes,
tens to hundreds of bytes, where per-call fixed costs share the
denominator at the small end. At the scales where the adversarial
families actually bite — tens of kilobytes and up — flatness is
carried by @method's deterministic counters, pinned flat across
size doublings, and by the fuel survey's population panels
(@measured), not by these wall-clock bands. End to end, against
the direct transcription of @naive held in memory as its pointer
tree, the corpus of record measures a trade rather than a rout,
tabulated in @fig-trade: the identity and membership operations — fork, sync, and the
party join — run multiples to decades cheaper on the packed form,
the mutating sweeps a small bounded multiple dearer than pointer
surgery on an already-materialized tree — a comparison that never
charges the tree for arriving or leaving. On the adversarial
shapes of @families the ratio is
unbounded by construction, a class apart rather than a multiple.
Flat constants are the resilience thesis made visible at
the nanosecond scale; a shape-sensitive constant is a small
amplifier waiting for a bigger denominator.

== What the cache sees <cache>

A sweep touches memory forward, densely, and once. Every operand is
one contiguous buffer read left to right; the transient state — two
path-bit stacks, a handful of accumulators, the output builder's bit
stacks — is a few dozen bytes that stay resident in L1 for the whole
operation.

Now price a single load, and contrast the pointer-tree walk of
@naive-constants. A 64-byte cache line holds _512 bits_ of packed
skyline — dozens of plateaus, likely more than the entire operand —
versus one heap node carrying roughly three bits of information. The
tree walk's next node is a dependent load at an unpredictable
address: a cache miss costs on the order of a hundred nanoseconds,
and the walk cannot start the next load until this one lands. The
sweep's next bits are in the same line or the next sequential one;
hardware prefetchers recognize the pattern and run ahead of it, so
the stream arrives before it is asked for. That is not a
constant factor on the same curve. It is the gap between
latency-bound on pointer chases and bandwidth-bound on a
prefetched stream — and it is most of why the per-byte constants stay
flat across every shape, and why the trade of @fig-trade keeps its
shape at every size. No input's _layout_ can degrade
the access pattern, because the access pattern does not depend on
the input.

== Words, not bits <words>

The per-bit description of @coding is the specification, not the
inner loop. Gamma codes are read a word at a time: load one 64-bit
window, count leading zeros — one instruction on every modern
ISA, plus the byte-reversing load a little-endian machine pairs
with it — and the count names the whole code's length and position, so a
payload decodes in a handful of instructions with no per-bit loop.
(Codes wider than a window take a genuinely wide path, priced by
their own width, per @funding.) The topology polarity of @coding
exploits the same instruction: with internal $=$ `0`, a descent
chain of
internal nodes is a zero run, and the find-first-one that terminates
it is the same primitive that parses a gamma prefix — one instruction
family serving both stream vocabularies.

Value arithmetic is priced the same way. A decoded payload lives
inline in machine words until it outgrows two of them, and only then
spills to a heap integer; organic values essentially never spill, so
the accumulator's digit lanes and the sweep's folds run entirely in
registers and L1. Wide values — when an input pays for them — run
the wide paths at their own width, exactly once each, per the
funding discipline. Nothing about supporting arbitrary precision
taxes the inputs that do not use it.

Branches, last. The sweep's inner dispatch (advance left, advance
right, tie) is data-dependent, and an adversarial interleaving can
degrade prediction — that is a bounded constant, not a class change,
and it is the one machine effect the linear bound absorbs
rather than eliminates. On organic inputs the advance pattern is
strongly biased and predicts well.

== Depth without frames <depth-machine>

Every walk is iterative. Suspended ancestors cost about two _bits_
each on packed stacks: the validator keeps two topology-state bits
per open ancestor, the overlay walk one path bit per level per
cursor — two bits per level either way, by two different routes.
Two stated exceptions run wider, both still linear and both
priced: the
watermark stack's bounded differences (@tick-web), and the route
fold's pending cost pairs (@tick-fusion), a log-width pair per open
id _branch_ stacked at twice its own width in packed bits against
the id's own two bits per level — a bounded
transient multiplier on the id operand alone. A cursor walking a
tree $10^5$ levels deep — a thirty-seven-kilobyte
message — pays one path bit per level: some twelve
kilobytes of packed stack state against that operand's own
thirty-seven kilobytes, and no native stack at all —
no overflow, no guard pages, no frame setup and teardown in the hot
loop. The direct transcription's
thousandfold frame amplification (@naive-recursion) is
replaced by a constant near one-third for a single-cursor walk (the
validator's two state bits per level make it two-thirds) — either
way the state is _smaller_ than
the input's own bits.

The mechanism story compresses to one sentence: the skyline turns
every clock operation into the one workload fifty years of
memory-hierarchy engineering has been built to reward — a forward
scan of
dense bytes with register-resident state.

== The measured record <measured>

Every measured claim in this chapter traces to one of two committed
corpora. They divide by what each can honestly assert, and the
division mirrors the introduction's provenance rule: counters are
exact, nanoseconds indicate a class.

*The fuel survey* is deterministic. For each public operation of the
implementation, inputs are drawn uniformly at random from the whole
canonical input space of each exact packed size — counting-guided
generation over the stream grammar, seeded per (operation, size,
sample), no entropy from time or the machine — and the operation
runs under instruction metering, so each sample's cost is an exact
instruction count that no cache, scheduler, or host can perturb.
The survey of record spans sixty-three public operations, an
8 KiB size span at 2,000 samples per size column, and every
committed adversarial family measured as marked points on the same
axes. It happens to have run on a 192-vCPU x86-64 host; being
counters, the readings would be identical anywhere, and the survey
is committed as data beside this document
(`data/fuelscape-8k-2000`), from which every panel replays
byte-identically. @fig-atlas is one panel.

#let atlas = json("data/fuelscape-8k-2000/version_join.json")
#let agrid = atlas.grid
#let acols = agrid.columns
#let nbins = agrid.fuel_bins
#let x-edges = range(acols.len() + 1).map(i => agrid.x_lo + (agrid.x_hi - agrid.x_lo) * i / acols.len())
#let y-edges = range(nbins + 1).map(j => agrid.y_lo + (agrid.y_hi - agrid.y_lo) * j / nbins)
// Per-column-normalized density — p(fuel | size) is what the eye
// compares — with empty bins masked to the page.
#let z = range(nbins).map(j => acols.map(c => {
  let m = calc.max(..c.counts)
  let v = c.counts.at(j)
  if v == 0 { float.nan } else { v / m }
}))
#let med-x = acols.map(c => calc.log(c.size, base: 2))
#let med-y = acols.map(c => calc.log(c.median, base: 2))
#let over = atlas.op.overlay
#let over-x = over.map(o => calc.log(o.size, base: 2))
#let over-y = over.map(o => calc.log(o.fuel, base: 2))
// Reference slopes, anchored at the smallest column's median.
#let x0 = med-x.first()
#let m0 = med-y.first()
#let slope1-x = (agrid.x_lo, agrid.x_hi)
#let slope1-y = slope1-x.map(x => m0 + (x - x0))
#let slope2-top = x0 + (agrid.y_hi - m0) / 2
#let slope2-x = (agrid.x_lo, slope2-top)
#let slope2-y = slope2-x.map(x => m0 + 2 * (x - x0))
// The document's heatmap colormap, reversed so density is ink weight
// on the page, with rare cells faded toward the paper; lightness
// stays monotone in density, so the panel survives grayscale.
#let vir = color.map.viridis
#let heat-paper = range(vir.len()).map(i => {
  let t = i / (vir.len() - 1)
  vir.at(vir.len() - 1 - i).transparentize((1 - t) * 60%)
})
#let byte-ticks = (
  (1, [2]), (3, [8]), (5, [32]), (7, [128]), (9, [512]),
  (11, [2 Ki]), (13, [8 Ki]),
)
#let fuel-ticks = range(3, 8).map(k => (k * calc.log(10, base: 2), [$10^#k$]))

#figure(
  chart(lq.diagram(
    width: 300pt, height: 180pt,
    xlim: (agrid.x_lo, agrid.x_hi),
    ylim: (agrid.y_lo, agrid.y_hi),
    xlabel: [total packed input (bytes)],
    ylabel: [fuel (instructions)],
    xaxis: (ticks: byte-ticks, subticks: none, mirror: false),
    yaxis: (ticks: fuel-ticks, subticks: none, mirror: false),
    lq.colormesh(
      x-edges, y-edges, z,
      map: heat-paper,
      interpolation: "pixelated",
    ),
    lq.plot(slope1-x, slope1-y, mark: none,
      stroke: (paint: gray-line, thickness: 0.5pt, dash: "dashed"),
      label: none),
    lq.plot(slope2-x, slope2-y, mark: none,
      stroke: (paint: gray-line, thickness: 0.5pt, dash: "dashed"),
      label: none),
    lq.plot(med-x, med-y, mark: none,
      stroke: (paint: ink, thickness: 1.1pt), label: none),
    lq.plot(over-x, over-y, stroke: none, mark: "x", mark-size: 4.5pt,
      color: accent, label: none),
    lq.place(12.9, m0 + (12.9 - x0) - 0.9)[#text(size: fig-annot-size,
      fill: gray-line.darken(30%))[slope 1]],
    lq.place(slope2-top - 0.7, agrid.y_hi - 1.2)[#text(size: fig-annot-size,
      fill: gray-line.darken(30%))[slope 2]],
    lq.place(9.2, med-y.at(9) + 2.6)[#text(size: fig-annot-size,
      fill: ink)[column median]],
    lq.place(6.2, 21.5)[#text(size: fig-annot-size,
      fill: accent)[adversarial families]],
  )),
  caption: [The fuel survey's join panel. Each column is the
    conditional distribution of instruction counts over 2,000
    canonical operand pairs drawn uniformly at that exact total
    size, normalized within the column and darkening with density;
    the ink line joins column medians, and the accent crosses are
    the committed adversarial families measured on the same axes.
    Both reference slopes anchor at the smallest column's median.
    The bulk rides slope 1 — the top decade of medians fits slope
    $1.003$ — and every family lands on or below the bulk's own
    band: the dearest, a concurrent pair near the span's top, runs
    at about three quarters of the organic per-byte median. The
    strata below the main band are the operation's cheap exits,
    present in the population at every size.],
) <fig-atlas>

What the panel asserts, no wall clock could: work per packed byte is
a constant of the operation, not of the input's shape, magnitude, or
luck of the draw — the join sweep's per-byte median settles near
5,700 instructions and moves by about a fifth from eight bytes
to eight kibibytes, and the shapes engineered to maximize its work
land on or below the band the uniform population already occupies.
Sixty-two further panels cover the rest of the surface, and their
pictures differ where they should. The sweep panels share the join
panel's shape — a bulk riding slope 1, the families tracking the
same slope at constants of their own. The comparison-family panels
split in two: a bulk that exits
almost immediately — a uniform random pair disagrees within its
first codes — under engineered equal-and-dominated families riding
the reading bound above it, the early exit being the population's
luck and the linear ceiling the family's price (@cmp). The
output-denominated operations carry their output term (@projection).
Uniform sampling audits the bulk; the overlaid families audit the
corners no uniform sample would ever hit; and the verdicts stay with
the pinned envelopes and the board — the atlas is the audit view
that lets a reader _see_ the population those instruments bracket
(@method).

*The wall-clock corpus* is statistical: one hundred timed samples
per point on the library sweeps (the slowest composition points
collect fewer, never below ten), medians quoted, release builds, inputs prebuilt outside
the timed region, on one quiet aarch64 workstation
(an Apple M4 Max, 128 GiB). Its like-for-like comparison holds the
implementation against the direct transcription of @naive — the
in-tree semantic oracle — on structurally identical values built
from one generation plan: universes of 8 to 32,768 forked members,
randomly preserved and joined back down to operand trees whose
packed size grows roughly linearly with member count. Both sides are held
in memory; the transcription is a pointer tree that pays nothing to
arrive or leave, while the packed form's resting state _is_ its
wire state — so the table below prices the skyline's decode-sweep-recode
discipline against pointer surgery under the most
tree-flattering framing the corpus can construct.

#figure(
  table(
    columns: (auto, auto, auto, auto),
    align: (left, right, right, left),
    stroke: 0.4pt + gray-line,
    inset: 6pt,
    table.header([*operation*], [*transcription*], [*skyline*],
      [*skyline : transcription, across the sweep*]),
    [fork], [2.36 ms], [38.4 µs],
    [$4.7 times$ cheaper at 8 members, $61 times$ at the top],
    [sync], [8.34 ms], [5.02 ms],
    [parity at 8 members; $1.4$–$1.8 times$ cheaper beyond],
    [party join], [1.24 ms], [0.89 ms],
    [$1.4$–$1.8 times$ cheaper past the smallest size],
    [clock join], [4.24 ms], [4.99 ms],
    [$1.7 times$ dearer at 8 members; $1.0$–$1.2 times$ beyond],
    [version join], [2.35 ms], [4.05 ms],
    [$1.4$–$2.1 times$ dearer],
    [comparison (ordered)], [0.50 ms], [1.09 ms],
    [$1.4$–$2.2 times$ dearer],
    [tick], [1.84 ms], [7.70 ms],
    [near parity at 8 members; $4.2$–$4.7 times$ dearer beyond],
    [send], [3.06 ms], [7.65 ms],
    [$2.0$–$4.1 times$ dearer],
    [receive], [4.07 ms], [11.3 ms],
    [$2.6$–$3.5 times$ dearer],
  ),
  caption: [The representation trade, at matched in-memory operands.
    Medians at the sweep's top size (32,768 members); the last
    column gives each ratio's range over the whole sweep. Fork,
    sync, and the party join are identity-and-reconciliation work,
    where the tree must clone or rebuild structure the packed form
    moves as bytes; the mutating sweeps below them pay the packed
    form's re-coding against surgery on nodes the tree already
    holds materialized. Comparison is quoted on its ordered
    outcome, the full-length walk; the concurrent outcome exits
    early on both representations and measures in nanoseconds on
    both, two orders below the ordered walk.],
) <fig-trade>

Read the table as the price sheet of one architectural decision. The
transcription wins exactly where it mutates a materialized graph in
place and the sweep must re-emit its stream; it loses wherever
structure must be duplicated or reconciled — an order of magnitude
and growing at fork, steady margins at sync and the party join —
because there its every node is a clone and a pointer chase while
the skyline's is a byte in a `memcpy`. Three
readings sharpen the trade. First, the composites inherit their
parts: send and receive sit at the sum of their sweeps (@floor), so
a composite's multiple is a weighted mean of its parts' — never a
compounding. Second, the
priced multiple buys the resilience the rest of this document
derives: the transcription's rows hold only on organic inputs — on
the families of @families its cost is a polynomial degree worse,
unbounded as a ratio — while the skyline's rows are its cost on
_every_ input of the size. Third, the framing charges the packed
form for canonicality and never credits it: equality on canonical
bytes is a byte comparison — the corpus's self-comparison answers in
$1.4$ nanoseconds flat across the sweep, where the transcription
walks its whole structure to say yes, a millisecond at the top
size — and a hash, a dedup, or a wire frame is the same story, with
no serialization pass anywhere on the skyline's side of the ledger.

Repeated mutation earns a series of its own, because it is a
pinned inequality made visible. At a fixed 64-member tree, one tick
costs $13.2$ µs on the skyline against $3.0$ µs on the
transcription. Sixty-four ticks, applied one at a time, cost
$577$ µs against $781$ µs — the loop repays the per-call
unpack-repack before the transcription's per-tick renormalization
does — but the $k$-fold tick of @tick-output turns the same
sixty-four into one call: $22.6$ µs, _flat_ from $k = 4$ to
$k = 64$, byte-identical output. Two walks and a splice at any
count, on the wall clock as in the bound.

A final set of measurements looks past the library, at composition
scale — the gossip protocol this clock library was built to serve,
whose design is otherwise outside this document's subject. Reconciling two
replicas over a full session — handshake, framing, round trip, the
clock work of this document riding inside — prices a differing
message in microseconds and a shared one in nanoseconds. With
nothing shared, about $6$ µs per differing message, held across
three decades of divergence: a hundred differing messages reconcile
in $633$ µs, a hundred thousand in $625$ ms. A session over one
hundred thousand shared-and-identical messages confirms agreement
in $619$ µs — about $6$ ns per shared message, so a hundred
differing messages cost the session what a hundred thousand agreed
ones do. Divergence pays extra for the shared bulk it must navigate
— a thousand differing messages cost $5.5$ ms against no shared
state and $28$ ms astride a hundred-thousand-message shared history
— but the microseconds-to-nanoseconds ordering between differing
and shared holds at every measured combination of shared and
differing state. Cost rides the delta,
not the database: the same proportionality discipline, one layer
up.
