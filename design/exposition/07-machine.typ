#import "fig.typ": *

= Constants, and the machine <machine>

Asymptotics chose the representation; constants are where a
representation earns its keep on real hardware. This section states
where the measured costs of our implementation sit and — more
durably — _why_ the skyline is the shape the machine wants. Numbers
here are measured (release builds, deterministic inputs, medians);
they are quoted as mechanism and magnitude, not as a benchmark table,
because the magnitudes will drift with hardware and the mechanisms
will not.

== The floor is reading <floor>

The packed form has no random access: positions, extents, and values
are implicit in self-delimiting codes, so every operation pays one
sequential parse of its operands _by construction_. That sets a floor
— you cannot answer a whole-value question below the cost of reading
the value — and makes the honest question about constants: _how close
does each operation sit to its own reading cost?_

The measured picture, per packed input byte — one floor, raw byte
movement, with each band quoted against the one below it:

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
  the price of actually decoding every code once,
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
_ordering_ — each operation's band sitting above the previous at
matched operand sizes, the bands' edges overlapping only because
each aggregates several operations — is exactly the ordering of how
much each operation must do
per bit: nothing pays for machinery it does not use. And the values
are _flat across input shapes_: the adversarial families of the
previous sections land in the same bands as organic values of the
same size, with the two bounded exceptions already stated —
@measures' pinned-counter shape and @words' prediction cost. (All
per-byte figures are taken over the bench corpus's organic sizes,
tens to hundreds of bytes, where per-call fixed costs share the
denominator at the small end. At the scales where the adversarial
families actually bite — tens of kilobytes and up — flatness is
carried by @method's deterministic counters, pinned flat across
size doublings, not by these wall-clock bands.) Flat constants are the resilience thesis made visible at
the nanosecond scale; a shape-sensitive constant is a small
amplifier waiting for a bigger denominator. End to end, against the
direct transcription running the bench corpus's _organic_ workloads
— values of tens to hundreds of bytes, where the transcription's
quadratics stay dormant — the sweeps measure between $2 times$ and
$20 times$ faster; on @families' adversarial shapes the ratio is
unbounded by construction, a class apart rather than a multiple.

== What the cache sees <cache>

A sweep touches memory forward, densely, and once. Every operand is
one contiguous buffer read left to right; the transient state — two
path-bit stacks, a handful of accumulators, the output builder's bit
stacks — is a few dozen bytes that stay resident in L1 for the whole
operation.

Contrast the pointer-tree walk from @naive-constants at the level of
what a load costs. A 64-byte cache line holds _512 bits_ of packed
skyline — dozens of plateaus, likely more than the entire operand —
versus one heap node carrying roughly three bits of information. The
tree walk's next node is a dependent load at an unpredictable
address: a cache miss costs on the order of a hundred nanoseconds,
and the walk cannot start the next load until this one lands. The
sweep's next bits are in the same line or the next sequential one;
hardware prefetchers recognize the pattern and run ahead of it, so
the stream arrives before it is asked for. The difference is not a
constant factor on the same curve — it is the difference between
being latency-bound on pointer chases and bandwidth-bound on a
prefetched stream, and it is most of why the per-byte constants stay
flat across every shape (and why the $2$–$20 times$ on organic
values never degrades): there is no input whose _layout_ degrades
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
serves the same trick: with internal $=$ `0`, a descent chain of
internal nodes is a zero run, and the find-first-one that terminates
it is the same primitive that parses a gamma prefix — one instruction
family serves both stream vocabularies.

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
and it is the one machine effect the linear bound simply absorbs
rather than eliminates. On organic inputs the advance pattern is
strongly biased and predicts well.

== Depth without frames <depth-machine>

Every walk is iterative. Suspended ancestors cost about two _bits_
each on packed stacks — the validator's two topology-state bits per
open ancestor, the overlay walk's one path bit per level per cursor
(same number, two derivations) — with two stated exceptions, the
watermark stack's bounded differences (@tick-web) and the route
fold's pending cost pairs (@tick-fusion: two machine words per open
id _branch_ against the id's own two bits per level, a bounded
transient multiplier on the id operand alone), both still linear,
both priced. A tree $10^5$ levels deep — a thirty-seven-kilobyte
message —
costs a cursor walking it one path bit per level: some twelve
kilobytes of packed stack state against that operand's own
thirty-seven kilobytes, and no native stack at all —
no overflow, no guard pages, no frame setup and teardown in the hot
loop. The direct transcription's
$approx 800 times$ frame amplification (@naive-recursion) is
replaced by a constant near one-third for a single-cursor walk (the
validator's two state bits per level make it two-thirds) — either
way the state is _smaller_ than
the input's own bits.

The whole section compresses to one sentence: the skyline turns
every clock operation into the one workload — a forward scan of
dense bytes with register-resident state — that fifty years of
memory-hierarchy engineering has been optimizing for.
