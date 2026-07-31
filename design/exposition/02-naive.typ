#import "fig.typ": *

= What the direct transcription costs <naive>

Transcribe the paper into a program in the most natural way: an
algebraic data type with one heap node per tree node, an
arbitrary-precision integer at each event node, the operations as the
recursive functions printed in the paper, and the appendix's
variable-length coding for the wire. Call this the _direct
transcription_. It is the correct starting point: our own
implementation keeps exactly such a transcription in-tree forever,
as the semantic oracle against which its optimized kernels are
differentially tested. And it is worth costing honestly, because its
defects — and the failure of the obvious repairs — dictate the shape
of everything built later.

== Two lengths to fear <lengths>

Fix an encoded input of $n$ bits. Two quantities inside it can each
be $Theta(n)$ on their own:

- *Depth* $d$: a chain of nodes costs a constant number of bits per
  level — about three, in the paper's coding and in the coding of
  @skyline alike — so a few tens of kilobytes of input encode a
  tree a hundred thousand levels deep. (The paper spends a 3-bit
  node tag per spine level; the skyline prices the same shape at
  $3d + O(1)$, with the anatomy made exact in @validation.)
- *Magnitude width* $W$: a single stored integer can occupy a
  constant fraction of the input — $W = Theta(n)$, a value near
  $2^(n\/2)$ in one leaf, since a self-delimiting code spends about
  two bits per magnitude bit. (Nothing downstream needs more: the
  quadratics below already fire at $W = Theta(n)$.)

Every cost of the form "per node, work proportional to a magnitude" is
therefore a latent quadratic: the input can buy $Theta(n)$ nodes and
$Theta(n)$-bit values *in the same bytes* and make the implementation
multiply them. Validity, not provenance, bounds cost at a byte
boundary. @families collects the adversarial constructions this
document builds — each an ordinary, canonical, normal-form value that
decodes cleanly — with a forward pointer for the ones whose
construction needs machinery we do not have yet. Each is named once,
here, and used consistently through @resilience. The table is the
document's core set, not the whole committed roster: as each later
mechanism is built, its section constructs the further families
aimed at it, each presented as an _attack card_ — the input shape,
the work it extracts from the design it refutes, and the landed
mechanism that defeats it — in one uniform format from here through
@resilience.

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
      spine, driving every enclosing range minimum to the running
      height (attacks @tick-web's undercut cascade)],
    [the tick walk's watermark stack, at every level at once],
    [_reveal comb_$(t, k)$],
    [$t$ sibling regions sharing one $2^k$-scale minimum over a low
      floor, closed and reopened in sequence (attacks @tick-web's
      close rule)],
    [any design that re-touches a wide boundary per region],
    [_scattered-party comb_$(t, k)$],
    [a $t$-tooth version riding a $k$-bit base, against a party
      owning every other tooth (built in @projection)],
    [any masking that must re-spell absolutes per transition —
      mandatory output],
    [_duplicated wide code_$(W)$],
    [one $W$-bit code that a raise re-codes across a fill boundary
      (built in @tick-output)],
    [any additive output bound — one code can be half the stream],
  ),
  caption: [The adversarial families, all in one place; the last six
    are constructed where the machinery they attack is built. Each is
    a legal, canonical value the decoder must accept; each is named
    here in the paper's tree vocabulary, and @coding's rules
    determine each one's stream form mechanically (`bigroot`'s
    $W$-bit root becomes a $W$-bit leading
    absolute over small deltas; the combs, one absolute followed by
    $plus.minus$-steps). `hugeleaf` and
    `bigroot` are unreachable by any honest history at the scales
    that hurt (a height near $2^W$ needs on the order of $2^W$
    ticks); the others are merely unlikely.],
) <families>

== Defect 1: path sums in comparison and join <path-sums>

The paper's comparison lifts subtrees as it descends:

$ "leq"((n_1, l_1, r_1), (n_2, l_2, r_2)) = n_1 <= n_2 and
  "leq"(l_1 arrow.t n_1, l_2 arrow.t n_2) and
  "leq"(r_1 arrow.t n_1, r_2 arrow.t n_2) $

with $e arrow.t m$ the lift of @model. Transcribed literally, the
equation makes each recursive call materialize lifted children: an
arbitrary-precision addition _and a fresh integer_ per node per level.
After descending a path, the lifted value at the cursor is the sum of
bases along the path — a _path sum_ — and on `bigroot`$(d, W)$ every
one of the $d$ frames on the way down owns a live $W$-bit sum.

Time is $Theta(d dot W)$ bit-operations and transient memory is
$Theta(d dot W)$ bits, on an input of $Theta(d + W)$ bits. Choosing
$d approx W approx n\/2$ makes both quadratic: the amplification
ratio scales as $d W \/ (d + W)$, growing without bound as the
operand grows. This is not a corner case that needs contriving. Before
the cure, the committed
`bigroot` instance — a $40,000$-bit root value under a
$10,000$-level spine, a fifteen-kilobyte operand — drove a single
comparison's transient memory to fifty-six megabytes, more than
$3,700 times$ the operand's bytes (the committed movement record
on the comparison cell). The figure reconciles
with the formula: one live $W$-bit path sum per level of the
descent is $d dot W = 4 dot 10^8$ bits — fifty megabytes — and
the remainder is the walk's own overhead. The ratio grows with
the operand, as the formula says it must.

#figure(
  attack(
    [bigroot$(d, W)$],
    [every walk that carries absolute values along paths],
    stack(dir: ttb, spacing: 6pt,
      codestrip((
        ([flag], 18pt, "t"),
        ([$W$-bit absolute (root value)], 128pt, "w"),
        ([flag], 18pt, "t"), ([$plus.minus 1$], 20pt, "p"),
        ([flag], 18pt, "t"), ([$plus.minus 1$], 20pt, "p"),
        ([$dots.c$], 16pt, "x"),
        ([flag], 18pt, "t"), ([$plus.minus 1$], 20pt, "p"),
      )),
      text(size: 7.5pt, fill: gray-line.darken(40%),
        [one wide leading code, then $d$ spine levels of cheap
         codes: $Theta(d + W)$ bits buying depth _and_ width]),
    ),
    [$d$ live $W$-bit path sums on one descent —
     $Theta(d dot W)$ time and transient memory from a
     $Theta(d + W)$-bit operand pair (measured: $3,700 times$ the
     input's bytes).],
    cure: [sweeps that never materialize an absolute: the running
      quantity is a difference the stream's own deltas update
      (@cmp), so the width is held once, not per level.],
  ),
  kind: image,
  caption: [The `bigroot` attack card: depth times width bought in
    the same bytes, aimed at path sums.],
) <fig-attack-bigroot>

Join has the same skeleton: at every paired node it lifts both of
one side's children by the base difference, $l_2 arrow.t (n_2 - n_1)$
and $r_2 arrow.t (n_2 - n_1)$, with the operands ordered so the
difference is nonnegative. It then adds
the normalization pass: `norm` computes the minimum over each node's
children and lifts it into the parent, subtracting it from both
children — more per-level arbitrary-precision work of exactly the
path-sum shape. Every two-operand walk in the transcription shares
the defect, because the paper's equations all reason in _absolute_
values, and an absolute value exists at a node only as the sum of
everything above it. And
`fill` and `grow` inherit it too: the shortcut arms recompute
subtree maxima and minima at every level — absolute, $W$-bit
quantities on `bigroot` — so the transcription's tick is another
$Theta(d dot W)$ walk, the baseline against which @tick-output's
cure is measured.

== Defect 2: decoding a wide value <naive-decode>

The appendix's integer coding is a chain of grow-by-one-bit stages,
and the natural decoder accumulates one bit at a time into a heap
integer: shift, add, repeat. (_Natural_ means the shape the stage
recursion invites, not the only transcription of it; the other
defects need no such invitation.) Appending a bit to a $j$-bit accumulator
rewrites all of its machine words in a normalized representation, so
a single $W$-bit value decodes in

$ sum_(j = 1)^(W) Theta(j) = Theta(W^2) "bit-work — on 64-bit words,"
  W^2 \/ 128 + O(W) "machine-word writes." $

On `hugeleaf` this is the whole input, and the arithmetic reconciles
with the committed counter. The instrument holds
$W = 125,000$ bits — a sixteen-kilobyte value spelled as one
thirty-one-kilobyte code — where the formula predicts
$W^2\/128 approx 1.22 dot 10^8$ machine-word writes; the
bit-at-a-time decoder counted $122$ million, the
formula to within a percent. The cured
decoder — accumulate machine words, splice them once — landed the
same cell at $1,954$ word writes: the value's own words, exactly
linear, a sixty-thousandfold drop (measured). And the quadratic
outgrows any patience: a value just thirty times wider — half a
megabyte — pays the formula a further thousandfold. The defect looks trivial once named —
of course you buffer words — but it is worth its own entry for two
reasons. First, it is a _decode-time_ quadratic: it runs on arbitrary
bytes before any validity judgment, which is the worst possible place
to be slow. Second, its cure is the first instance of the document's
central discipline: the cost of touching a wide value must be charged
to the bits that spell it — $W$ bits of code license $O(W)$ work, paid
once, not $W$ payments of growing size.

#figure(
  attack(
    [hugeleaf$(W)$],
    [decoding and re-encoding one wide integer],
    stack(dir: ttb, spacing: 6pt,
      codestrip((
        ([flag], 18pt, "t"),
        ([one payload of $approx 2W$ code bits (value $2^W - 1$)],
         240pt, "w"),
      )),
      text(size: 7.5pt, fill: gray-line.darken(40%),
        [the whole input is a single self-delimiting code]),
    ),
    [a bit-at-a-time decoder rewrites its growing buffer per
     appended bit: $Theta(W^2)$ — $122$ million machine-word
     writes counted on the committed sixteen-kilobyte instance.],
    cure: [word-windowed reads: accumulate machine words, splice
      once — $1,954$ writes on the same instance, the value's own
      word count (@words runs the instruction-level version).],
  ),
  kind: image,
  caption: [The `hugeleaf` attack card: the whole budget spent on
    one code, aimed at any per-bit touch of a growing value.],
) <fig-attack-hugeleaf>

== Defect 3: recursion is a representation choice <naive-recursion>

Every operation in the paper is a structural recursion, and a
transcription runs it on the call stack: one native frame per tree
level. A bare frame — return address, saved registers — is tens of
bytes against the roughly *three bits* the level cost on the wire:
already a hundredfold amplification. With each frame's spilled
locals and temporaries, the heaviest recursive frame measures
roughly half a kilobyte per level (per-level stack-pointer deltas,
release build) — over a thousandfold the wire. And there is a
harder edge behind the constant: at half a kilobyte per level, a
default
thread stack of a few megabytes overflows at $d$ around $10^4$ —
`deep spine` at an input of a few _kilobytes_ — crashing the
process, or forcing a guard-page fault handler. A library that can be
crashed by a short message it correctly parses is not merely slow; it
has delegated its availability to its callers' inputs.

The cure is as unglamorous as Defect 2's: walks must be iterative,
with explicit state whose size the implementer chooses. What is worth
carrying forward is the _budget_. Explicit state can be a few
*bits* per level — we will see walks that need exactly two — so
depth, which the input buys at bits per level, must never be paid
for in words per level. The system holds that budget everywhere
but at two
bounded, priced exceptions, stated where they live (@tick-web,
@tick-fusion).

#figure(
  attack(
    [deep spine$(d)$],
    [recursion itself: frames and stack depth],
    stack(dir: ttb, spacing: 6pt,
      codestrip((
        ([flag], 18pt, "t"), ([leaf], 20pt, "t"), ([$0$], 14pt, "p"),
        ([flag], 18pt, "t"), ([leaf], 20pt, "t"), ([$1$], 14pt, "p"),
        ([$dots.c$], 16pt, "x"),
        ([flag], 18pt, "t"), ([leaf], 20pt, "t"), ([$0$], 14pt, "p"),
      )),
      text(size: 7.5pt, fill: gray-line.darken(40%),
        [an alternating chain: about three bits per level,
         $d approx 10^5$ levels in a few tens of kilobytes]),
    ),
    [one native frame per three-bit level — measured near half a
     kilobyte, a thousandfold amplification — and a crashed process
     at $d approx 10^4$: a few kilobytes of valid input overflow a
     default thread stack.],
    cure: [iterative walks with explicit packed state, about two
      _bits_ per suspended level (@depth-machine): the state is
      smaller than the input's own spelling of the depth.],
  ),
  kind: image,
  caption: [The `deep spine` attack card: depth at three bits per
    level, aimed at the call stack.],
) <fig-attack-deepspine>

== Defect 4: the constant-factor anatomy <naive-constants>

Suppose the quadratics were fixed and the transcription made merely
linear. It would remain slow by constant factors that compound:

- *Pointer chasing.* One heap node per tree node makes the walk a
  linked-structure traversal: each step a
  dependent load, likely a cache miss, at hundreds of cycles apiece
  when the tree is cold. The information content of a node — around
  three bits — travels in a 64-byte (512-bit) cache line.
- *Allocation.* Every lift, every `norm`, every join builds nodes;
  every node is an allocator round trip. The paper's operations are
  algebraically pure, and purity transcribed naively allocates once
  per equation applied.
- *Dispatch.* Leaf-or-node branching happens at every step, on data
  the predictor has never seen, between loads it must wait for.
- *Serialization.* At rest the value is a pointer graph; to
  store or send it, a full encode pass; to receive, a full decode and
  rebuild. Every process boundary pays a serialization tax
  proportional to the whole value even when the consumer needed only
  an equality check — which @canonical will make a byte comparison.
  (Strictly this is an architectural cost rather than a constant
  factor; the representation dissolves it rather than shrinks it.)

Hold the shape of this list. The representation of @skyline makes
each item structurally impossible rather
than carefully avoided: no nodes, no pointers, no per-node
allocation, and no distinction between the resting form and the wire
form.

== The repair ladder, and where it breaks <ladder>

The defects above invite a ladder of local repairs, and the ladder's
top rung fails in an instructive way.

Replace per-frame lifted clones with a single running offset
(cures Defect 1's memory, and its time on `bigroot` — one wide add
at the root instead of $d$ of them); decode words at a time (cures
Defect 2);
make walks iterative (cures Defect 3). During any walk, the natural
next design — the one this section talks you out of — maintains a
_running absolute height_ in one normalized big integer: add each
node's base on the way down, subtract on the way up, compare
absolutes where needed.

Consider now a value whose plateaus oscillate between $2^k - 1$ and
$2^k$ — in binary, between $k$ ones and a one followed by $k$
zeros. Call the boundary between them a _carry
cliff_, and the value the *boundary comb* (@families): stepping
across the cliff rewrites $k + 1$ bits, so each crossing costs
$Theta(k)$ work in any normalized representation, no matter how small
the step. A comb with $t$ teeth astride one cliff extracts
$Theta(t dot k)$ bit-work from its walk. (This amplifier is not a
fifth defect of the transcription — there it hides inside Defect 1's
path sums; it is the defect of every _repair_ of Defect 1 that keeps
a normalized running value, which is why it closes the ladder
instead of joining the numbered four.)

And the comb is _cheap to spell_, under either coding. The paper's
normal form lifts the shared $2^k - 1$ into the root, leaving every
tooth a small relative value; the delta coding we are about to adopt
(@skyline) spells one absolute and then $plus.minus 1$ steps. Both
codings store the comb in $Theta(t + k)$ bits, though writing every
plateau's absolute height out would take $Theta(t dot k)$. So a
walk that
maintains a normalized running absolute does $Theta(t dot k)$ work
on a $Theta(t + k)$-bit input: a genuine amplifier over either
spelling, and not one the obvious next repair escapes. Normalize
everything except a small pending window, and the cliff merely
moves to the window's edge. @two-zone builds the input that defeats
any fixed window, weighs the adaptive one, and removes the
settled/pending split entirely.

#figure(
  attack(
    [boundary comb$(t, k)$],
    [any normalized running quantity],
    stack(dir: ttb, spacing: 4pt,
      skyline(
        ((0.125, 2), (0.125, 1), (0.125, 2), (0.125, 1),
         (0.125, 2), (0.125, 1), (0.125, 2), (0.125, 1)),
        w: 200pt, unit: 16pt, show-heights: false,
      ),
      text(size: 7.5pt, fill: gray-line.darken(40%),
        [$t$ teeth oscillating $2^k - 1 arrow.l.r 2^k$ (drawn as
         1 and 2): every crossing rewrites $k + 1$ bits of any
         normalized value, and each tooth costs three stored
         bits]),
    ),
    [$Theta(t dot k)$ bit-work from a $Theta(t + k)$-bit input —
     in the validator itself, on arbitrary bytes, before any
     validity judgment.],
    cure: [the accumulator's lazy zone (@redundant): $2^32$ at
      digit 0 is a legal spelling of the carried form, so the
      $k$-bit carry is dissolved, not deferred — $t$ touches
      total.],
  ),
  kind: image,
  caption: [The boundary comb attack card: three-bit codes astride
    a carry cliff. It closes the repair ladder, and @accum is built
    against it.],
) <fig-attack-comb>

A sibling construction aims the same cliff at a different seam. In
the paper's tree form an absolute exists at a node only as the sum
of bases above it, so a walk that maintains a running absolute adds
each base on entry and subtracts it on exit. The *cliff fan* makes
that entry/exit itself the amplifier: one $2^k - 1$ base at the
root, and under it a fan of $t$ cheap teeth whose leaf values never
cross anything — but the walk's running sum crosses the $2^k$
boundary _twice per tooth_, funded once by the root's single wide
code. The excursions are siblings, not nested, so no balancing
argument caps them.

#figure(
  attack(
    [cliff fan$(t, k)$],
    [running path sums at subtree entry and exit],
    stack(dir: ttb, spacing: 4pt,
      codestrip((
        ([root: $2^k - 1$], 70pt, "w"),
        ([tooth $+1$], 34pt, "p"), ([tooth $+1$], 34pt, "p"),
        ([$dots.c$], 16pt, "x"),
        ([tooth $+1$], 34pt, "p"),
        ([leaf $0$], 30pt, "p"),
      )),
      text(size: 7.5pt, fill: gray-line.darken(40%),
        [one wide base lifted over $t$ twelve-bit teeth: the
         running sum sits at $2^k - 1$ and every tooth's entry and
         exit crosses $2^k$]),
    ),
    [$Theta(t dot k)$ carry work from a $Theta(t + k)$-bit input:
     $2t$ cliff crossings funded by one wide code paid once.],
    cure: [the same zone (@redundant), reached by never
      maintaining the absolute at all: the skyline stores leaf
      heights, and its walks fold only differences (@skyline,
      @cmp).],
  ),
  kind: image,
  caption: [The cliff fan attack card: entry/exit accumulation
    priced separately from leaf deltas — the two constructions pin
    a running-value design from both sides.],
) <fig-attack-clifffan>

The ladder's lesson, then, is a constraint, not a fix: the efficient
representation needs arithmetic whose per-update cost is
bounded by the update's _own_ coded size — never by where the running
value happens to sit. Holding that constraint, we can now build the
representation; the arithmetic that honors it is @accum.
