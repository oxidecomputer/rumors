#import "fig.typ": *

= What the direct transcription costs <naive>

Transcribe the paper into a program in the most natural way: an
algebraic data type with one heap node per tree node, an
arbitrary-precision integer at each event node, the operations as the
recursive functions printed in the paper, and the appendix's
variable-length coding for the wire. Call this the _direct
transcription_. It is the correct starting point — our own
implementation keeps exactly such a transcription in-tree forever as
the semantic oracle its optimized kernels are differentially tested
against — and it is worth costing honestly, because its defects, and
the failure of the obvious repairs, dictate the shape of everything
built later.

== Two lengths to fear <lengths>

Fix an encoded input of $n$ bits. Two quantities inside it can each
be $Theta(n)$ on their own:

- *Depth* $d$: a chain of nodes costs a constant number of bits per
  level — about three, in the paper's coding and in the coding of
  @skyline alike (the paper spends a 3-bit node tag per spine level;
  the skyline an internal flag plus the off-spine leaf's flag and a
  1-bit payload) — so a few tens of kilobytes of input encode a tree
  a hundred thousand levels deep.
- *Magnitude width* $W$: a single stored integer can occupy nearly the
  whole input — a value near $2^n$ in one leaf.

Every cost of the form "per node, work proportional to a magnitude" is
therefore a latent quadratic: the input can buy $Theta(n)$ nodes and
$Theta(n)$-bit values *in the same bytes* and make the implementation
multiply them. @families collects the adversarial constructions this
document builds — each an ordinary, canonical, normal-form value that
decodes cleanly — with a forward pointer for the ones whose
construction needs machinery we do not have yet. Each is named once,
here, and the names are used consistently through @resilience.

#figure(
  table(
    columns: (auto, 1fr, 1fr),
    align: (left, left, left),
    stroke: 0.4pt + gray-line,
    inset: 6pt,
    table.header([*family*], [*shape*], [*what it stresses*]),
    [_bigroot_$(d, W)$],
    [a $W$-bit value at the root of a spine $d$ levels deep],
    [every walk that carries values along paths],
    [_hugeleaf_$(W)$],
    [one leaf holding $2^W - 1$],
    [decoding and re-encoding a single wide integer],
    [_deep spine_$(d)$],
    [an alternating chain, $d$ levels, small values],
    [recursion itself: frames, stack depth],
    [_boundary comb_$(t, k)$],
    [$t$ teeth of $plus.minus 1$ steps astride the value $2^k$
      (built in @ladder)],
    [any normalized running quantity — the carry cliff],
    [_wide-tooth comb_$(t, k)$],
    [teeth of width-$Theta(k)$ steps astride a far larger cliff
      (built in @two-zone)],
    [partially-normalized ("two-zone") running quantities],
  ),
  caption: [The first five adversarial families; two more (the
    _descending staircase_ and the _reveal comb_) arrive with the tick
    walk in @tick. Each is a legal, canonical value the decoder must
    accept; several are unreachable by any honest history at the
    scales that hurt — which is exactly the point. Validity, not
    provenance, is what bounds cost at a byte boundary.],
) <families>

== Defect 1: path sums in comparison and join <path-sums>

The paper's comparison lifts subtrees as it descends:

$ "leq"((n_1, l_1, r_1), (n_2, l_2, r_2)) = n_1 <= n_2 and
  "leq"(l_1 arrow.t n_1, l_2 arrow.t n_2) and
  "leq"(r_1 arrow.t n_1, r_2 arrow.t n_2) $

where $e arrow.t m$ adds $m$ to the root of $e$. Transcribed
literally, each recursive call materializes lifted children: an
arbitrary-precision addition _and a fresh integer_ per node per level.
After descending a path, the lifted value at the cursor is the sum of
bases along the path — a _path sum_ — and on `bigroot`$(d, W)$ every
one of the $d$ frames on the way down owns a live $W$-bit sum.

Time is $Theta(d dot W)$ bit-operations and transient memory is
$Theta(d dot W)$ bits, on an input of $Theta(d + W)$ bits. Choosing
$d approx W approx n\/2$ makes both quadratic: the amplification
ratio scales as $d W \/ (d + W)$, growing without bound as the
operand grows. This is not a corner case that needs contriving:
measured on our direct transcription before any cure, a `bigroot`
operand pair of 29 kilobytes drove transient memory to roughly
$6,700 times$ its input — approaching two hundred megabytes — inside
a single comparison, and the ratio kept growing with the operand,
as the formula says it must.

Join has the same skeleton (`join` lifts one side by the base
difference, $r_2 arrow.t (n_2 - n_1)$, at every paired node) and adds
the normalization pass: `norm` computes each node's subtree minimum
and sinks it, more per-level arbitrary-precision work of exactly the
path-sum shape. Every two-operand walk in the transcription shares the
defect, because the paper's equations all reason in _absolute_ values
which only exist, at a node, as the sum of everything above it.

== Defect 2: decoding a wide value <naive-decode>

The appendix's integer coding is a chain of grow-by-one-bit stages,
and the natural decoder accumulates one bit at a time into a heap
integer: shift, add, repeat. Appending a bit to a $t$-bit accumulator
rewrites all $Theta(t \/ 64)$ of its machine words in a normalized
representation, so a single $W$-bit value decodes in

$ sum_(t = 1)^(W) Theta(t / 64) = Theta(W^2 \/ 64) "word operations." $

On `hugeleaf` this is the whole input, and the arithmetic reconciles
with the wall clock: at $W = 4 dot 10^6$ bits, each append rewrites a
half-megabyte buffer, four million times — two terabytes of memory
traffic, which at memory-bandwidth speed is right where the
measurement landed: over fourteen seconds for one value. The cured
decoder (accumulate machine words, splice them once) does the same
work in milliseconds, linearly. The defect looks trivial once named —
of course you buffer words — but it is worth its own entry for two
reasons. First, it is a _decode-time_ quadratic: it runs on arbitrary
bytes before any validity judgment, which is the worst possible place
to be slow. Second, its cure is the first instance of the document's
central discipline: the cost of touching a wide value must be charged
to the bits that spell it — $W$ bits of code license $O(W)$ work, paid
once, not $W$ payments of growing size.

== Defect 3: recursion is a representation choice <naive-recursion>

Every operation in the paper is a structural recursion, and a
transcription runs it on the call stack: one native frame per tree
level. A bare frame — return address, saved registers — is tens of
bytes against the roughly *three bits* the level cost on the wire,
already a hundredfold amplification; with each frame's spilled
locals and temporaries the stack cost measured near $300$ bytes per
level in our transcription, $800 times$ the wire. And there is a
harder edge behind the constant: default thread stacks are a few megabytes, so
`deep spine`$(d)$ overflows the stack — crashes the process, or forces
a guard-page fault handler — at $d$ around $10^5$, an input of some
tens of kilobytes. A library that can be crashed by a short message it
correctly parses is not merely slow; it has delegated its availability
to its callers' inputs.

The cure is as unglamorous as Defect 2's: walks must be iterative,
with explicit state whose size the implementer chooses. What is worth
carrying forward is the _budget_: the explicit state can be a few
*bits* per level (we will see walks that need exactly two), so depth —
which the input buys at bits per level — must never be paid for in
words per level.

== Defect 4: the constant-factor anatomy <naive-constants>

Suppose the quadratics were fixed and the transcription made merely
linear. It would remain slow by constant factors that compound:

- *Pointer chasing.* One heap node per tree node means the walk's
  memory access pattern is a linked-structure traversal: each step a
  dependent load, likely a cache miss, at hundreds of cycles apiece
  when the tree is cold. The information content of a node — around
  three bits — travels in a 64-byte (512-bit) cache line.
- *Allocation.* Every lift, every `norm`, every join builds nodes;
  every node is an allocator round trip. The paper's operations are
  algebraically pure, and purity transcribed naively is an allocation
  per equation application.
- *Dispatch.* Leaf-or-node branching at every step, on data the
  predictor has never seen, between loads it must wait for.
- *The wire is elsewhere.* At rest the value is a pointer graph; to
  store or send it, a full encode pass; to receive, a full decode and
  rebuild. Every process boundary pays a serialization tax
  proportional to the whole value even when the consumer needed one
  comparison.

Hold the shape of this list. The representation of @skyline is
designed so that each item becomes structurally impossible rather
than carefully avoided: no nodes, no pointers, no per-node
allocation, and no distinction between the resting form and the wire
form.

== The repair ladder, and where it breaks <ladder>

The defects above invite a ladder of local repairs, and the ladder's
top rung fails in an instructive way.

Replace per-frame lifted clones with a single running offset
(cures Defect 1's memory); decode words at a time (cures Defect 2);
make walks iterative (cures Defect 3). The natural next design — and
the one to be talked out of — maintains, during any walk, a _running
absolute height_ in one normalized big integer: add each node's base
on the way down, subtract on the way up, compare absolutes where
needed.

Consider now a value whose plateaus oscillate between $2^k - 1$ and
$2^k$ — in binary, between $0underbrace(11 dots 1, k)$ and
$1underbrace(00 dots 0, k)$. Call the boundary between them a _carry
cliff_, and the value the *boundary comb* (@families): stepping
across the cliff rewrites $k + 1$ bits, so each crossing costs
$Theta(k)$ work in any normalized representation, no matter how small
the step. A comb with $t$ teeth astride one cliff extracts
$Theta(t dot k)$ bit-work from its walk.

In the paper's own coding this shape happens to pay its way — each
tooth spells a fresh $k$-bit base, so the input is $Theta(t dot k)$
bits and the work is linear in it. But we are about to compress
consecutive plateaus by _delta coding_ (@skyline), after which the
same teeth cost a few bits each: $Theta(t + k)$ bits of input carrying
$Theta(t dot k)$ bits of implied absolute value. The compression is
the entire point — and it turns the carry cliff into a genuine
amplifier for any running-absolute design, with no repair available by
tuning: pick any boundary a normalized representation owns, and an
input exists that oscillates across exactly that boundary at unit cost
per crossing.

So the ladder's lesson is a constraint, not a fix: the efficient
representation must be paired with arithmetic whose per-update cost is
bounded by the update's _own_ coded size — never by where the running
value happens to sit. Holding that constraint, we can now build the
representation; the arithmetic that honors it is @accum.
