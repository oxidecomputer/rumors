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
  1-bit payload, with the deepest sibling pair obliged to differ —
  one wider code at the bottom keeps the shape canonical at
  $3d + O(1)$ bits) — so a few tens of kilobytes of input encode a
  tree a hundred thousand levels deep.
- *Magnitude width* $W$: a single stored integer can occupy a
  constant fraction of the input — $W = Theta(n)$, a value near
  $2^(n\/2)$ in one leaf, since a self-delimiting code spends about
  two bits per magnitude bit. (Nothing downstream needs more: the
  quadratics below already fire at $W = Theta(n)$.)

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
    [_wide-tooth comb_$(t, w, k)$],
    [$t$ teeth of $plus.minus 2^w$ steps astride a cliff at $2^k$,
      $k gt.double w$ (built in @two-zone)],
    [partially-normalized ("two-zone") running quantities],
    [_descending staircase_$(d)$],
    [unit-step plateaus descending monotonically down a $d$-level
      spine (attacks @tick-web's undercut cascade)],
    [the tick walk's watermark stack, at every level at once],
    [_reveal comb_$(t, k)$],
    [$t$ sibling regions sharing one $2^k$-scale minimum over a low
      floor, closed and reopened in sequence (attacks @tick-web's
      close rule)],
    [any design that re-touches a wide boundary per region],
  ),
  caption: [The adversarial families, all in one place; the last four
    are constructed where the machinery they attack is built. Each is
    a legal, canonical value the decoder must accept. `hugeleaf` and
    `bigroot` are unreachable by any honest history at the scales
    that hurt (a height near $2^W$ needs on the order of $2^W$
    ticks); the others are merely unlikely. Either way the lesson is
    the same: validity, not provenance, is what bounds cost at a byte
    boundary.],
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
measured on our direct transcription before any cure, the committed
`bigroot` regression instance — a $40,000$-bit root value under a
$10,000$-level spine, a 29-kilobyte operand pair — drove transient
memory to roughly $6,700 times$ the pair's bytes, approaching two
hundred megabytes, inside a single comparison. The figure reconciles
with the formula: each paired node materializes _four_ lifted
children — both operands', both halves' — that live across the
descent, and $4 dot d dot W = 1.6 dot 10^9$ bits is exactly the two
hundred megabytes measured. The ratio kept growing with
the operand, as the formula says it must.

Join has the same skeleton (`join` lifts both of one side's
children by the base difference, $l_2, r_2 arrow.t (n_2 - n_1)$, at
every paired node) and adds
the normalization pass: `norm` computes each node's subtree minimum
and sinks it, more per-level arbitrary-precision work of exactly the
path-sum shape. Every two-operand walk in the transcription shares the
defect, because the paper's equations all reason in _absolute_ values
which only exist, at a node, as the sum of everything above it.

== Defect 2: decoding a wide value <naive-decode>

The appendix's integer coding is a chain of grow-by-one-bit stages,
and the natural decoder accumulates one bit at a time into a heap
integer: shift, add, repeat. Appending a bit to a $j$-bit accumulator
rewrites all of its machine words in a normalized representation, so
a single $W$-bit value decodes in

$ sum_(j = 1)^(W) Theta(j) = Theta(W^2) "bit-work — on 64-bit words,"
  W^2 \/ 128 + O(W) "machine-word writes." $

On `hugeleaf` this is the whole input, and the arithmetic reconciles
with the wall clock: at $W = 4 dot 10^6$ bits the buffer grows to
half a megabyte and each append rewrites it — a quarter-megabyte on
average, four million times, about a terabyte of memory traffic —
which at memory-bandwidth speed is the right order for what the
measurement showed: over fourteen seconds for one value. The cured
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
harder edge behind the constant: at 300 bytes per level, a default
thread stack of a few megabytes overflows — crashing the process, or
forcing a guard-page fault handler — at $d$ around $10^4$:
`deep spine` at an input of a few _kilobytes_. A library that can be
crashed by a short message it correctly parses is not merely slow; it
has delegated its availability to its callers' inputs.

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
$2^k$ — in binary, between $k$ ones and a one followed by $k$
zeros. Call the boundary between them a _carry
cliff_, and the value the *boundary comb* (@families): stepping
across the cliff rewrites $k + 1$ bits, so each crossing costs
$Theta(k)$ work in any normalized representation, no matter how small
the step. A comb with $t$ teeth astride one cliff extracts
$Theta(t dot k)$ bit-work from its walk.

And the comb is _cheap to spell_, under either coding. The paper's
normal form lifts the shared $2^k - 1$ into the root, leaving every
tooth a small relative value; the delta coding we are about to adopt
(@skyline) spells one absolute and then $plus.minus 1$ steps. Both
codings store the comb in $Theta(t + k)$ bits carrying
$Theta(t dot k)$ bits of implied absolute value — so a walk that
maintains a normalized running absolute does $Theta(t dot k)$ work
on a $Theta(t + k)$-bit input, a genuine amplifier over either
spelling, with no repair available by tuning: pick any boundary a
normalized representation owns, and an input exists that oscillates
across exactly that boundary at unit cost per crossing.

So the ladder's lesson is a constraint, not a fix: the efficient
representation must be paired with arithmetic whose per-update cost is
bounded by the update's _own_ coded size — never by where the running
value happens to sit. Holding that constraint, we can now build the
representation; the arithmetic that honors it is @accum.
