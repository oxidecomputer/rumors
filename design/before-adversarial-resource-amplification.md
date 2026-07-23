# `before`: adversarial resource amplification in Version and Party computation

Status: analysis complete (2026-07-22, measured against the working
tree at `dd2b1645`, branch `link-transport`, Apple M4 Max, release
builds). Both representations audited: the event side (`Version`) and
the id side (`Party`), every public operation. Fix space designed;
execution in progress on branch `before-hardening` (plan §14, amended
2026-07-22); decision open at §12 (Tier 1.5 vs Tier 2).

Scope: `rumors`' model of record is authenticated-honest-peer, so none
of this is a `rumors` security finding — an authorized peer holds
write authority and needs no memory tricks. The goal is to harden and
performance-optimize `before` *unconditionally*: as a standalone
library whose `decode` boundary may face untrusted bytes, and because
every amplification constant below is also a tax on honest deep or
large inputs (fork-heavy histories produce deep trees organically).
Native builds only; the wasm target is demo-only and out of scope.
The yardstick is resource proportionality — transient cost as a
function of input size, for whoever presents the input — never
adversary economics.

Epistemic key, following `design/streaming-wire-deadlock.md`:
**[measured]** = observed in the instrumented experiment (§5);
**[derived]** = argument from the code or arithmetic in this document;
**[open]** = known unknown / decision pending.

## 1. Problem statement

A `Version` at rest is a packed preorder bit stream — per node, a
flag bit plus an Elias-gamma-coded base (`codec/gamma.rs`). A `Party`
is a packed preorder id tree of 2-bit child-presence tags
(`idbits.rs`), with no integers. Computation converts between these
compact at-rest forms and working values: fixed-width arrays,
arbitrary-precision path sums, recursion frames.

The question: can an adversary craft canonical, normal-form inputs
whose *computation* costs memory or CPU grossly disproportionate to
their encoded size — and can the library be redesigned to compute
with cost proportional to the packed size, **without bounding
inputs** (no value, depth, or size caps) and **without losing the
compactness of the representation or the `O(n + m)` operation
costs**? Bounding inputs is the uninteresting answer and is not
pursued here; the representations and algorithms are on the table.

Answer, in brief: yes. There are six amplifier classes — five on the
event side (§3), one on the id side (§4) — two of them quadratic;
and all are removable (§7–§10) because every quantity the algorithms
need at a node is either one of two global accumulators or bounded by
that node's own coded size. The blowups come from the chosen
representation of *intermediate* state, not from anything the
algorithms inherently require. The id side, which has no integers, is
already most of the way there and serves as the existence proof for
the event-side redesign (§11.1).

## 2. Adversarial input constructions

All shapes pass strict normal-form validation (`codec/tree.rs`:
`parse_ev` — at least one zero-base child per node, no equal-valued
leaf pair; `parse_id` — no `(1,1)` node) and therefore
`Version::decode` / `Party::decode`. Each is reachable by an honest
history. Bit layouts are given so an independent implementer can
reproduce them. Event coding: `enc_ev(Leaf n) = 0 · gamma(n)`,
`enc_ev(Node n l r) = 1 · gamma(n) · l · r`, where `gamma(n)` codes
`m = n + 1` (so `gamma(0) = 1`, `gamma(1) = 010`,
`gamma(2^B − 1) = 0^B · 1 · 0^B`). Id coding: 2-bit presence tags,
`00` terminal, `10`/`01` unary, `11` both; absent children occupy no
bits.

**S(d), the dense event spine.** A left spine of `d` zero-base
internal nodes, each with a 0-leaf right sibling, bottoming out in
`(0, 0, 1)`:

    "11" × d            d internal nodes: flag 1, gamma(0)
    "01"                bottom-left leaf 0
    "0010"              bottom-right leaf 1
    "01" × (d − 1)      each ancestor's right sibling, leaf 0

Total `4d + 4` bits, `2d + 1` nodes, depth `d`. Normal form holds
everywhere (each internal node's spine child has base 0; the only
leaf pair is `(0, 1)`). This is the *densest* shape normal form
admits — ~2 bits per node, depth ~n/4 for n bits — maximizing node
count and recursion depth simultaneously.

**bigroot(B, d).** A root with base `2^B − 1` over S(d) and a 0-leaf:
`"1" · gamma(2^B − 1) · S(d) · "01"`, total `2B + 4d + 8` bits
(amended 2026-07-22: originally stated `2B + 4d + 7`; the committed
generator's length pin caught the off-by-one — `1` flag + `2B + 1`
gamma + `4d + 4` spine + `2` leaf). Puts a B-bit magnitude on every
root-to-node path sum while keeping paths long.

**hugeleaf(B).** A single leaf of value `2^B − 1`: `2B + 2` bits, one
node. Maximizes bit length per node.

**I(d, divert), the id spine.** A unary chain of `d` left-only tags
ending in a terminal: `"10" × d · "00"`, `2d + 2` bits, depth `d`.
With `divert`, the last unary node is right-only (`01`), so
`I(d, false)` and `I(d, true)` share their first `d − 1` levels and
own disjoint regions — the shape that drives two-operand id walks to
full lockstep depth.

## 3. Event-side amplifiers

### V1 — quadratic memory and time: owned per-frame path sums

Mechanism [derived, confirmed]: the comparison walk
(`version/compare.rs`, `CmpWalk::rec`) and the join/meet walk
(`version/event/combine.rs`, `CombineWalk::rec`) both execute, at
every node:

```rust
let a_sum = a_off + a_node.base();
let b_sum = b_off + b_node.base();
```

`a_sum` is an **owned** `Base` held in the frame while both children
recurse, and `Add<&Base> for &Base` in the mixed/big case routes
through `to_biguint()`, which clones the full accumulated magnitude.
On bigroot(B, d), every one of the `d` simultaneously-live frames
owns a private B-bit `BigUint` equal to the same path sum — and paid
O(B) time to clone it.

Cost model [derived]: peak ≈ `d·B/8` bytes and time ≈ `d·B` bit-ops
from `n = 2B + 4d` input bits; maximizing at `B = n/4`, `d = n/8`
gives **peak ≈ n²/32 bits = n²/256 bytes**.

Measured (§5): 14 KiB → 48 MiB (×3,335); 29 KiB → 191 MiB (×6,668);
doubling the input quadrupled the peak, and the model's prediction
(20 000 × 80 000 / 8 = 200 MB) matches. Time scaled ×3.8 per
doubling. Extrapolation [derived from the fitted model]: a 1 MiB
version costs ~275 GB peak on one `partial_cmp` — **against the empty
version**. This is the pure read path: no `Batch`, no working form.
`PartialOrd`/`PartialEq` (mixed forms), `concurrent`, join, and meet
all reach it.

A contributing structural fact: comparison against a *shallow*
operand does not prune. When one side bottoms out at a leaf, the walk
broadcasts a synthetic `Zero` and descends the deep side to its full
depth (`EvReader::Zero`, the paper's "leaf `n` behaves as
`(n, 0, 0)`"). Contrast the id side, §4/§11.2.

### V2 — linear ×782: recursion frames

Mechanism [measured]: comparing S(d) against `Version::new()`
allocates **zero heap** — the entire cost is stack segments.
Recursion depth is tree depth (~n/4) at the ~0.5 KiB/frame that
`recurse.rs`'s `RED_ZONE` comment itself records; `stacker`'s
segments bypass the global allocator (invisible to the counting
allocator, visible to RSS).

Measured: +93.3 MiB RSS for 122 KiB (×782); +186.7 MiB for 244 KiB
(×783) — cleanly linear, ~391 B/frame. Every recursive walk pays it:
compare, combine, fill, grow, rank, project. A 10 MiB version costs
~7.6 GiB of stack to compare once.

### V3 — quadratic time: spilled-gamma decode

Mechanism [derived, confirmed]: `codec/gamma.rs`, `decode_int_from`'s
wide fallback builds the `BigUint` mantissa one bit at a time
(`m <<= 1; m |= 1`); each shift copies O(B/64) limbs, so one B-bit
value costs **O(B²/64) limb-ops**.

Measured: `Version::decode` of hugeleaf(4M) — 977 KiB — took
**14.5 s** of CPU; a subsequent join re-paid 14.6 s (every operation
re-decodes bases through `EvReader::read`). Memory is unaffected
(×2). `skip_int` is immune (never materializes the value); encode is
linear.

### V4 — linear ×98–198: the working form and `Builder`

Mechanism [derived, confirmed]: `Base` is 24 bytes (niche-optimized
[measured via layout stand-in]), so `WorkingVersion` and `Builder`
cost ~24 B plus a topo bit per node that costs as little as 2 bits
packed: ×~100 per materialized copy. `Batch::tick` unpacks then
builds (`fill`, possibly `grow`), holding ~two copies: ×198 measured.
A join of two packed versions skips the unpack (the combine walk
reads packed directly) but still builds: ×98.

Sharper sub-finding [measured]: `Builder::with_capacity` pre-sizes
from `EvReader::node_capacity_bound` = *bit length* / 2, before any
node is visited. Joining hugeleaf(4M) — **one node** — pre-allocated
95.4 MiB (×100). (`WorkingVersion::unpack` deliberately push-grows
instead, per its own comment; `Builder` predates that reasoning.)

### V5 — linear ×118: `decode`'s parse stack

Mechanism [derived, confirmed]: `codec/tree.rs::parse_ev_from` keeps
one `EvFrame` (56 B measured via stand-in; `NeedRight` holds two
`Base`s) per unfinished ancestor. S(d) keeps ~n/4 live: ×118
measured. Decoding-and-dropping a 1 MiB crafted version transiently
costs ~118 MiB.

## 4. Id-side audit (`Party` and its operations)

The id representation has **no integers and no working form**: 2-bit
presence tags; operations run directly on packed bits;
`party/ops/build.rs::IdBuilder` emits packed output and normalizes
via a fixed-width in-place tag patch plus truncation-only collapses.
Consequently there are no analogues of V1 (no arithmetic), V3 (no
codes wider than 2 bits), or V4 (no unpacked form; builder capacity
hints are bit-proportional). What remains:

### P1 — linear ×418–456: recursion frames in the two-tree walks

`sum` (⇒ `Party::join`/`join_all`), `covers`, and `is_disjoint`
recurse per lockstep level (`party/ops/{sum,compare}.rs`); `diff` (⇒
`Party::without`) additionally recurses through `complement`
(`party/ops/diff.rs`). Measured on I(d) pairs at two scales each
(cleanly linear):

- `join` ×455–456, `covers`/`is_disjoint` ×418, `without` ×357–358
  against combined operand bytes — and `without` reaches its depth
  through **one** adversarial operand (`diff(1, b)` complements `b`
  single-sidedly), so per adversarial byte it is ~×715.

Two structural mitigations the event side lacks, worth naming
because they bound the honest-vs-adversarial cases [derived]:

- **Lockstep pruning**: `sum`/`covers`/`is_disjoint` descend only
  where *both* sides are internal; an `Empty`/`Full` leaf resolves
  its arm with an iterative skip or verbatim copy, no recursion. So
  recursion depth ≤ the *shallower* operand's depth: one honest
  operand caps the walk. The exceptions are `complement` (one-sided
  by nature) and, on the event side via `tick`, `grow`'s `Expand`
  arm, which descends the id to its full depth against virtual
  `Zero` events — so `tick` with an adversarial `Party` reaches id
  depth even over a tiny event tree.
- `split` (⇒ `fork`/`forks`) is immune: an iterative spine loop plus
  verbatim bit-range splices (`party/ops/split.rs`), O(1) auxiliary
  state beyond the output. This is the pattern §11.3 exports.

`Party::decode` is clean: `parse_id_from`'s `IdFrame` carries no
values (a few bytes per level; ×~4 worst case) [derived].

## 5. Measurement methodology and results of record

Instrument: a standalone crate — generators for §2's shapes as byte
writers; a counting `GlobalAlloc` (current/peak live bytes); a
`getrusage(RUSAGE_SELF)` max-RSS mode run one-scenario-per-process
(RSS is a high-water mark, and stacker bypasses the heap counter).
Raw results (release, Apple M4 Max, 2026-07-22):

    size_of Base-like: 24 B, EvFrame-like: 56 B

    decode dense(d=125000)                 input 0.060 MiB  peak +7.060 MiB   x118      3.0ms
    cmp    dense(d=125000) vs new          input 0.060 MiB  peak +0.000 MiB   x0        6.5ms
    join   dense(d=125000) | 1             input 0.060 MiB  peak +5.812 MiB   x98       8.8ms
    tick   dense(d=125000)                 input 0.060 MiB  peak +11.783 MiB  x198      5.5ms
    decode dense(d=250000)                 input 0.119 MiB  peak +14.119 MiB  x118      6.8ms
    cmp    dense(d=250000) vs new          input 0.119 MiB  peak +0.000 MiB   x0       13.3ms
    join   dense(d=250000) | 1             input 0.119 MiB  peak +11.623 MiB  x98      18.2ms
    tick   dense(d=250000)                 input 0.119 MiB  peak +23.566 MiB  x198     12.3ms
    decode bigroot(B=40000,d=10000)        input 0.014 MiB  peak +0.897 MiB   x63       2.0ms
    cmp    bigroot(B=40000,d=10000) vs new input 0.014 MiB  peak +47.706 MiB  x3335     8.2ms
    join   bigroot(B=40000,d=10000) | 1    input 0.014 MiB  peak +49.129 MiB  x3434    26.7ms
    decode bigroot(B=80000,d=20000)        input 0.029 MiB  peak +1.794 MiB   x63       6.8ms
    cmp    bigroot(B=80000,d=20000) vs new input 0.029 MiB  peak +190.779 MiB x6668    30.9ms
    join   bigroot(B=80000,d=20000) | 1    input 0.029 MiB  peak +193.626 MiB x6767    94.6ms
    decode hugeleaf(B=4M)                  input 0.954 MiB  peak +1.454 MiB   x2       14.5s
    join   hugeleaf(B=4M) | 1              input 0.954 MiB  peak +95.414 MiB  x100     14.6s

    densecmp   d=250000: input 0.119 MiB, rss delta  93.3 MiB (x782), 12.4ms
    densecmp   d=500000: input 0.238 MiB, rss delta 186.7 MiB (x783), 24.7ms
    idjoin     d=250000: input 0.119 MiB, rss delta  54.2 MiB (x455),  8.8ms
    idjoin     d=500000: input 0.238 MiB, rss delta 108.6 MiB (x456), 17.8ms
    idcovers   d=250000: input 0.119 MiB, rss delta  49.8 MiB (x418),  5.8ms
    idcovers   d=500000: input 0.238 MiB, rss delta  99.7 MiB (x418), 12.0ms
    iddisjoint d=250000: input 0.119 MiB, rss delta  49.8 MiB (x418),  6.8ms
    iddisjoint d=500000: input 0.238 MiB, rss delta  99.7 MiB (x418), 13.3ms
    idwithout  d=250000: input 0.119 MiB, rss delta  42.6 MiB (x357),  7.3ms
    idwithout  d=500000: input 0.238 MiB, rss delta  85.3 MiB (x358), 14.7ms

Notes: the `cmp dense vs new` rows verify the walk runs to
completion on these shapes (against the empty version one direction
stays possible throughout, so the early-`concurrent` exit never
fires). Id rows report combined operand bytes; `idwithout`'s depth
comes from one operand. The experiment crate is session-scratch;
Phase 0 (§14) commits its generators and harness into `testing/` so
these numbers become pinned regression envelopes.

### Ranked summary

| # | side | path | class | measured |
|---|------|------|-------|----------|
| V1 | event | compare, join, meet (read path included) | quadratic memory + time | ×6,668 @ 29 KiB; doubling ⇒ ×4 |
| V2 | event | every recursive walk | linear memory (stack) | ×782 |
| V3 | event | decode + every read of a spilled base | quadratic time | 14.5 s @ 977 KiB |
| V4 | event | tick, join, meet (emit paths) | linear memory | ×98–×198; ×100 on 1 node |
| V5 | event | decode | linear memory | ×118 |
| P1 | id | join, covers, is_disjoint, without | linear memory (stack) | ×357–×456 |

The hypothesis that started this investigation — "the working-form
conversion blows up" — is V4: real, but fourth-ranked. The two worst
amplifiers live on paths that never build a working form.

## 6. The design invariant

Adopt as a stated contract of the crate (crate docs; enforced by the
§13 metering gate):

> **No operation materializes transient state asymptotically larger
> than its packed operands, and every operation remains amortized
> `O(n + m)` in the packed input bits — with no bound on value
> magnitude, tree depth, or encoded size.**

Why it is achievable [derived]: comparison consumes only
`sign(a_sum − b_sum)`; the combine sink consumes a min of two sibling
values sharing an offset — a local difference; leaf emission consumes
a value provably within a locally-coded distance of a computable
anchor (§8.2); id operations consume presence bits. Nothing requires
an absolute path sum per frame, a machine word per 2-bit node, or
half a kilobyte per level. The id side already demonstrates the
target shape end-to-end except for its call-stack frames.

Remediation is tiered. Tiers 0–1 change no observable behavior and
no stored bytes; Tier 1.5 and Tier 2 are alternative endgames for the
event-side emit paths (§12 decides).

## 7. Tier 0 — point fixes

**T0.1 (kills V3).** In `decode_int_from`'s wide fallback,
accumulate the mantissa into limbs (64 bits at a time into a
`Vec<u64>`, or byte-aligned chunks) and construct the `BigUint` once
— O(B). The window fast path and all reject decisions are untouched.
Verify with the codec round-trip/canonicality suites plus a metered
envelope on hugeleaf.

**T0.2 (kills V4's pre-allocation half).** Stop pre-sizing `Builder`
from `node_capacity_bound`; push-grow, for exactly the reason
`WorkingVersion::unpack`'s comment records. Delete
`node_capacity_bound` if unused after.

**T0.3 (halves P1's worst per-byte case).** Make `complement`
iterative: it is a structure-preserving map (its collapse arm never
fires, per its own doc comment), so a pending-children counter over
the tag stream — the `skip_subtree` shape — emits it without
recursion. `Party::without` then prunes like its siblings: depth
bounded by the shallower *shared* structure.

## 8. Tier 1 — same representations, streaming intermediate state

### 8.1 Difference-tracked path sums (kills V1 in compare)

Replace the two owned per-frame sums in `CmpWalk::rec` with one
running signed difference `D = a_path_sum − b_path_sum`
(`num_bigint::BigInt`, or sign flag + `Base`), threaded `&mut`:

- reading the aligned pair: `D += a_base; D −= b_base`;
- the direction tests become sign tests on `D` (kill `le` if
  `D > 0`, `ge` if `D < 0`) — O(1);
- leaving the subtree: restore `D −= a_base; D += b_base`. The
  relative bases needed for the restore are the ones just decoded —
  in the frame either way, and their total live size along any path
  is ≤ the decoded input.

Memory [derived]: one accumulator (≤ max path-sum bits) plus
per-frame relative bases summing ≤ input. The d-copies-of-B
quadratic is gone. Time [derived; to be validated by the meter]:
`± u64` into a bignum is amortized O(1) (carry-run potential);
`± Big(x)` costs O(|x|) with Σ|xᵢ| ≤ input bits. The subtle case is
±-oscillation across a `2^64k` carry cliff (a single small op costing
O(k) limbs); the walk's Dyck structure bounds it — the accumulator
tracks path sums, monotone along any root-to-leaf path, so an
excursion across a cliff costs two long runs and forcing another
requires another comparably-coded magnitude in the input; total
O(n + m). Write this argument into the module doc when implemented
and pin it empirically with cliff-straddling generators (§13) — it is
what the probe-first practice exists for.

Amended 2026-07-23 (P2 carry-cliff round): the probe-first practice
caught the paragraph above — its conclusion survives only with a
stronger accumulator design than it states, because the
"comparably-coded magnitude" premise is true for some cliff shapes
and false for others. Two generator shapes separate them:

- **Paid crossings** — the boundary comb `C(k, n)`
  (`meter::cliff_comb`: teeth `(2^k − 1, 0, 1)` off a zero-base
  spine, terminal leaf 0; preorder leaf values oscillate
  `2^k − 1 ↔ 2^k`, so every consecutive-leaf step crosses the `2^k`
  carry boundary). Each tooth stores its own `gamma(2^k − 1)` —
  `2k + 1` bits — so every crossing is bought by a comparably-wide
  input code and the premise holds. **[measured** 2026-07-23,
  `tests/meter.rs` cliff envelopes, three identical runs**]**:
  decode/cmp/join per-input-byte limb constants flat across a 4×
  input growth (0.72 → 0.67, 0.16 → 0.14, 1.46 → 1.39 ops/byte at
  `k = n` = 1024 → 2048); ceilings pinned.
- **Unpaid crossings** [derived — build the generator and pin it
  before this section's implementation lands]: hang teeth
  `(1, 0, 1)` — 9 bits each — from a base-0 fan under a single
  stored `2^k − 1` root. The path sum sits at the cliff across the
  whole fan, every tooth's `+1`/`−1` re-crosses it, and one
  comparably-coded magnitude forces `n` excursions at O(1) input
  bits each: Θ(nk) limb work in a Θ(n + k)-bit input for any
  accumulator that materializes each `D ± base` as a plain big
  integer. The Dyck argument does not save this case — the
  excursions are siblings, not nested.

The implementation requirement this adds: the difference accumulator
must be cliff-immune. Keep `D` as a big part plus a machine-word
signed offset, folding small `±` into the offset and renormalizing
only on word overflow (amortized O(1) per small op; a wide `± Big(x)`
still costs O(|x|), paid by `x`'s own code); leaf direction tests
need `sign(D + x − y)` without mutating `D`. With that
representation the O(n + m) total stands on both shapes.

Coverage: `causal_cmp` sits on the oracle differential, exhaustive
small-scope, and algebraic-law suites; this is an internal rewrite
with unchanged verdicts.

### 8.2 Anchored relative emission (kills V1 in combine/fill/grow)

The emitting walks also need values — a combined leaf is
`leaf_op(a_sum, b_sum)` — and today absolute values transit frames
and `Builder` until `close_node`'s sink re-relativizes them. Emit
relative to a per-node anchor instead:

- at aligned node `v`, anchor `δ_v = op(A_v, B_v)` (the two path
  sums; `op` = max for join, min for meet);
- every output value in `v`'s subtree is ≥ `δ_v` [derived: for join,
  `max(A+x, B+y) ≥ max(A, B)` with `x, y ≥ 0`; dually for meet], so
  subtree values are emitted relative to `δ_v` as nonnegative `Base`s;
- magnitudes are locally bounded [derived]:
  `op(A+x, B+y) − op(A, B) ≤ max(x, y)`;
- a child's anchor delta `δ_c − δ_v ≥ 0` (monotonicity of `op`) is
  computable from `D`'s sign and the two local bases in local-sized
  time: it is `x` while `D` stays nonnegative, `y` while
  nonpositive, and `y − D_v` (with `|D_v| < y`) on a sign crossing —
  dually for the meet;
- `close_node`'s sink is translation-invariant, so its math is
  unchanged on anchored values.

No absolute magnitude is cloned per level; every materialized
quantity is bounded by the coded size of the node that produced it.
This also deletes the O(B)-per-level `to_biguint` clones — a strict
time win. While in `Base`: make the mixed-size `Add` arm add the
`Small` side into (a clone of) the `Big` side directly, and prefer
in-place `AddAssign` in hot paths.

### 8.3 Explicit compact stacks (kills V2 and P1; shrinks V5)

Depth costs ~0.5 KiB/level of stacker segment on every walk, both
sides. Convert the walks (event: compare, combine, fill, grow; id:
sum, diff, covers, is_disjoint) to explicit iteration whose frame is:

- resume state (which child next; each side's node kind): a few bits;
- what §8.1's restore needs: the two relative bases — or, smaller,
  the two *cursor positions*, re-decoding on unwind (one extra decode
  per node, O(1) each via the window path once T0.1 lands).

A `Vec` of position-pair frames is ~16–24 B/level (×782 → ~×40;
×455 → ~×10 on the id side, whose frames need only positions and 2–3
bits), one allocation, no stacker. The full-invariant refinement —
adopt if the meter says the constant matters — keeps restore values
as a *packed* side stack: push `gamma(a_base) · gamma(b_base)`
bit-reversed, so popping reads them forward from the top; the control
stack is then itself proportional to the packed input with constant
~1. Apply the same slimming to `parse_ev_from` (V5): the normal-form
checks need a zero-flag per child plus, for the equal-leaves test, a
re-decode at a recorded position — a few bytes per frame.

`recurse::descend!`/`stacker` remain only if some walk stays
recursive; audit at the end of the phase whether the dependency can
be dropped entirely. **[open]** convert all walks or only the eight
hot ones above (rank/project/max/min_ticks are same-shape but
lower-exposure).

### What Tier 1 does not fix

The event emit paths still build a ~24 B/node `Builder`/
`WorkingVersion` (V4's other half, ×~100). Closing that requires
changing output assembly — §9 or §10, decided at §12.

## 9. Tier 1.5 — packed event emission via a parent-close scratch

### 9.1 The one genuine obstruction

The single back-referential edit in the system is
`Builder::close_node`'s normalization sink: the parent's base
receives the children's common minimum (`+= m`: its gamma *widens*)
and the children's roots give it up (`−= m`: theirs *narrow*).
Preorder output places the parent's code before its children's
blocks, but the sink is bottom-up information — hence the fixed-width
array. `fill`'s `deferred_leaf` is the same phenomenon. Note the id
side has the same close-time normalization and *no* such problem,
because its patch is fixed-width (§11.1); the event-side problem is
purely the variable-width code.

A record is final only when its **parent's** sink is known. So write
records in parent-close order:

### 9.2 Pass 1: parent-close scratch

Run the (Tier-1-rewritten) walk once, emitting a packed scratch:

- hold each completed subtree's root `(flag, base)` on the spine
  stack; its descendants are already in the scratch;
- when a node closes: compute `m` from the two held child roots,
  apply the sink, apply the leaf-collapse if both held children are
  leaves (nothing was written for them — collapse is free), then
  **write the two finalized child-root records** and push the node's
  own `(flag, base)` as held;
- at the end of the walk, write the root's record.

Scratch layout, recursively:
`Inner(v) = Inner(L) · Inner(R) · rec(Lroot) · rec(Rroot)`, root
record last. Each record is written exactly once, already final,
**bit-reversed** so pass 2 can read the stream backward (gamma is
not otherwise backward-decodable). `fill`'s deferred left leaf
dissolves: by the time any leaf record is written, its right sibling
is closed and its value known.

### 9.3 Pass 2: backward read, preorder re-emission

Reading the scratch backward yields
`rec(v), rec(Rroot), rec(Lroot), Back-inner(R), Back-inner(L)` — each
node before its children, right-subtree material before left.
Preorder wants `v, pre(L), pre(R)`: walk the backward stream with an
explicit stack, translating the **right** subtree into a side buffer
first, streaming the **left** subtree directly, then splicing the
buffer. Pending right-blocks belong to ancestors and are disjoint
subtrees, so their total is ≤ the output size [derived].

Cost [derived]: two O(n + m) passes; transient ≤ scratch + pending
buffers + stacks ≈ **2–3× packed size, unconditionally**. Wire bytes
are *unchanged*: normal form is canonical and the algorithm computes
the same tree, so outputs are byte-identical — the snapshot suite and
a `join/meet/tick` equivalence proptest against the current
implementation give a total regression oracle for the rewrite.

`Batch` survives API-unchanged, retaining packed bits (each op is
already a full rebuild; the working form bought transcode constants,
which the window codec made word-scale). `WorkingVersion`, `Builder`,
and `EvReader::Working` are deleted; `EvReader` reads packed only.
Expected performance: plausibly a net win — per-node state drops from
24 B allocated to ~2–4 bits streamed; `benches/` arbitrates.
**[open]**: measure; pass 2 is mechanical (bit-block reversal + code
re-emission) if it shows up in profiles.

## 10. Tier 2 — representation change: topology + delta-coded leaf values

Internal bases are redundant: topology plus absolute leaf values
determine the event function. Storing exactly that dissolves the
obstruction instead of routing around it — and makes the event
representation morally identical to the id representation plus a
leaf-payload stream (§11.1). This section is the specification;
`design/before-skyline-encoding.md` is its expository companion,
building the same design up from the step-function semantics.

### 10.1 Encoding

Preorder topology bits as today. At each leaf position, in-stream:
the first leaf's value as `gamma(v₁)`; every later leaf as
`zigzag-gamma(vᵢ − vᵢ₋₁)` over consecutive leaves in preorder
(zigzag `2k` / `2|k| − 1`; one canonical sign convention, no negative
zero). The gamma window fast path applies unchanged (same `2k+1`
shape; unzigzag is a shift and mask).

### 10.2 Canonical form and validation

Canonical iff the topology is minimal: no internal node with two
equal-valued leaf children (recursively-uniform subtrees reduce to
this case, exactly as today's collapse does). Sibling leaves are
consecutive in leaf order, so equality is *the right sibling's delta
is zero*: the validator needs no values at all — per ancestor frame,
one is-leaf bit and one last-delta-was-zero bit. `decode` validation
drops from 56 B/level to ~2 bits/level with no arithmetic (V5
eliminated outright). Byte-equality remains `Eq`/`Hash`.

Amended 2026-07-23 (P2 carry-cliff round): "no arithmetic" was
overclaimed — it covers topology minimality only. Leaf values are
naturals, and a signed delta stream can drive a running leaf value
negative; no per-level topology bit sees that, so strict `decode`
must also enforce value validity, which means running-value state
across the delta stream. A plain big-integer accumulator makes that
Θ(W²) in wire bits on §10.6's boundary comb **[measured** — the
`meter/tier2` sweep pin**]**; the claim above survives only with the
cliff-immune accumulator §10.6 requires (big part + word offset:
nonnegativity is then a sign check on the redundant form, amortized
O(1) per small delta). V5's *frame* elimination stands either way.

### 10.3 Operations

All single forward passes over packed streams; depth costs bits:

- **compare**: versions are step functions over the unit id
  interval; merge-walk the two leaf sequences by dyadic boundary
  (topology stacks, ~2 bits/level), maintaining the running
  difference `D` (§8.1) and folding `le`/`ge` per elementary
  interval. No tree recursion, no broadcast machinery.
- **join/meet**: the same sweep emitting `op(v_a, v_b)` per
  elementary interval, re-delta-coded on the fly. Normalization is
  only the uniform-collapse — a pure **truncation** of emitted
  output back to a recorded subtree start (widths only shrink; each
  emitted bit is truncated at most once ⇒ amortized O(1)) plus one
  locally-sized repaired entry delta. The widening backpatch does
  not exist because there is no parent base — the same reason
  `IdBuilder` never needed one.
- **fill**: collapse fully-owned id regions to a streaming max over
  a leaf range; the sibling-raising value is known by emission time
  under the same truncate-and-re-emit discipline. **grow**: probe
  unchanged (small-int cost fold, bit-vector `Route`); emit rebuilds
  one root-to-leaf path and splices off-path spans verbatim with one
  boundary-delta repair each (§11.3).
- **rank / min_ticks / max / project**: sums/maxes of
  `value × interval` over the leaf sweep.

### 10.4 Compactness

Claim [derived — produce the full proof or refute with the §13 ratio
meter before committing]: Tier 2 coded size ≤ ~2× today's plus O(1)
bits per node, and is sometimes smaller. Argument: a
consecutive-leaf delta telescopes over the two path segments to the
LCA, so `|vᵢ − vᵢ₋₁| ≤ Σ` of stored relative bases on those
segments; each stored base lies on the exit path of exactly one
consecutive-leaf pair and the entry path of exactly one other (Euler
tour), so it is charged at most twice; and
`gamma(x + y) ≤ gamma(x) + gamma(y) + O(1)`. Topology is 1 bit/node
in both forms. Delta coding wins where similar magnitudes span
subtree boundaries, which min-lift cannot factor. The alternating
comb `(0, M, 0, M, …)` is tight for both forms. Property-test the
ratio over existing generators plus §2's shapes; document where the
envelope is tight. (Amended 2026-07-23: the win has an adversarial
dual — where Tier 2 is radically smaller, wire bits stop bounding
value content, and the operations must be priced against that; §10.6.)

### 10.5 Costs and migration

- **Wire break**: `Version::encode` (and therefore `Clock::encode`)
  bytes change; `before`'s codec pins and `rumors`'
  `gossip_snapshot` re-pin as a deliberate protocol change.

  Amended 2026-07-23 (P2 blast-radius round): the snapshot re-pin is
  the *smallest* part of the break. Three durable consumers of the
  canonical bytes sit beyond the test pins, all verified in code:

  - **Message identity.** Every trie leaf path is
    `hash(ContentHash(version.as_bytes()), ContentHash(value))`
    (`src/tree/typed/path.rs`), `Version::as_bytes` is
    doctest-pinned equal to `encode()`, and `Key`
    (`src/tree/key.rs`) is that path, publicly promised stable
    across replicas and freely persistable as its raw 32 bytes.
    Under Tier 2 every message's `Key` changes: application-persisted
    `Key`s dangle, and replicas on different code versions mint
    *different* `Key`s for the same message, so `redact(key)` and
    `get(key)` break across versions. This is a semantic identity
    break — a flag day plus an application-level `Key` migration
    story, not a re-pin.
  - **The bookmark store.** The durable, versioned
    (`BOOKMARK_FORMAT_VERSION = 1`) frame in
    `src/bookmark/format.rs` borsh-encodes `Clock`s, and
    `crates/before/src/borsh_impls.rs` defines the borsh form as
    exactly the canonical encode bytes with strict canonical decode
    on read. A previously persisted bookmark passes the frame's
    integrity hash and then fails `Clock` decode, surfacing as
    `FormatError::Decode` — documented as a logic error, not
    corruption — unless `BOOKMARK_FORMAT_VERSION` is bumped in the
    same change.
  - **Consumer-persisted serialized forms.**
    `crates/before/src/serde_impls.rs` serializes the same canonical
    bytes (deserializing through the strict validator), so any
    serde- or borsh-persisted `Party`/`Version`/`Clock` outside this
    workspace breaks identically.
- `Display`/`FromStr` keep the paper notation (internal bases are
  derivable in one pass for display; parsing accumulates path sums).
- `oracle.rs` is untouched and anchors the rewrite differentially;
  the exhaustive small-scope and algebraic-law suites transfer.
- Deleted: `WorkingVersion`, `Builder`, `EvReader`'s form split and
  `Zero` broadcast, `deferred_leaf`; `Base` stays for accumulators;
  `recurse`/`stacker` likely removable (everything is a sweep with
  bit-stacks).

### 10.6 The carry-cliff genre: wire bits no longer bound value content

Added 2026-07-23 (P2 carry-cliff round; found by the negative-space
review, constructed and measured here). Delta coding breaks the
premise every §10.3 sweep inherited from §8.1: that a cliff
excursion is always bought by a comparably-coded magnitude in the
input.

**The construction.** The boundary comb `C(k, n)`
(`meter::cliff_comb`; canonical — round-trips `Version::decode`
strictly, pinned to `n = k = 4096`): teeth `(2^k − 1, 0, 1)` off a
zero-base spine, terminal leaf 0. Its preorder leaf values oscillate
`2^k − 1 ↔ 2^k`, so the Tier 2 delta stream is 3-bit zigzag codes at
`10n + 4k + 2` total bits, while today's coding stores a fresh
`gamma(2^k − 1)` per tooth at `n(2k + 10) + 2` bits. At `n = k` that
is `14n + 2` against `2n² + 10n + 2` **[measured** — exact closed
forms pinned in `meter/tier2` tests**]**: current/Tier 2 = 9.84×
(n = 64), 146.98× (n = 1024), 585.84× (n = 4096), unbounded. The
§10.4 envelope holds in the useless direction: the comb's `2n + 1`
leaves carry Θ(nk) bits of absolute value content behind Θ(n + k)
Tier 2 wire bits, so **Tier 2 wire bits do not bound value content**
— the same lever behind the honest-regime "sometimes smaller" wins
on dense/bigroot, read adversarially.

**The cost.** Any sweep that materializes running leaf values (or a
running difference) in a plain big integer pays a full `k`-bit
carry or borrow per 3-bit delta: Θ(W²) total in Tier 2 wire bits
`W`. **[measured** 2026-07-23**]**: deterministically, the
`meter/tier2` limb pin (per-wire-bit limb cost roughly doubles per
size doubling, ratio ≥ 1.8 asserted at `n = k` = 512 → 1024); on
the wall clock (scratch probe, release, in-place `num-bigint` `±`),
21 → 44 → 83 → 158 ns per Tier 2 wire byte at
`n = k` = 4096/8192/16384/32768 — the per-byte cost doubles per
size doubling, the quadratic law of record (constants are
profile-dependent). This cost is not avoidable at decode: §10.2's
amendment — strict decode must enforce leaf-value nonnegativity, so
it runs the running-value state a plain topology check skips. Under
today's coding the same tree prices every crossing at `2k + 1`
stored bits, and decode/cmp/join stay linear per input bit
**[measured** — the `tests/meter.rs` cliff envelopes**]**.

**What it would take to keep Tier 2's operations linear** [open —
each must be designed and priced before a Tier 2 DECIDED entry]:

- **Cliff-immune accumulators** for compare, join/meet, fill, and
  decode validation: the §8.1-amendment representation (big part +
  machine-word signed offset, renormalized on word overflow) makes
  small-delta application and sign/nonnegativity checks amortized
  O(1) while wide deltas stay paid by their own codes. Note §8.1's
  fan shape means Tier 1/1.5 needs the same representation — the
  difference is that Tier 2 *cannot ship without it* (its strict
  decode is on the hook), while under today's coding only the
  future difference-tracked compare is.
- **Delta algebra for linear functionals** (`rank`, `min_ticks`,
  `max`, `project`): telescope `Σ vᵢ·wᵢ` into
  `v₁·W + Σ δⱼ·(suffix weight)` so each small stored delta
  contributes small work; never reconstruct absolute values.
- **Content-materializing operations are out of reach**: `Display`
  and the paper notation must output the absolute values — Θ(W²)
  bits of output from `W` wire bits on the comb — so the §6
  invariant *denominated over Tier 2 packed operands* is
  unsatisfiable for them. (Today's coding already has an
  output-superlinear case: bigroot's `Display` is Θ(bd) from
  Θ(b + d) input bits — §15's digit-capping note; Tier 2 widens the
  genre from formatting to anything that materializes content, and
  re-denominates §6 in a currency the adversary can deflate.) The
  invariant for Tier 2 must be restated over *content bits*
  (Σ leaf-value widths), with wire bits bounding only the
  delta-native operations.

**Decision weight.** Carried into §12 as a Tier 2 cost against Tier
1.5: under today's coding the genre is confined to the not-yet-built
§8.1 accumulator (and cured by the same representation); under Tier
2 it reaches the codec itself, and the mitigation set above is
design work that does not exist yet.

## 11. Cross-pollination between the two sides

Each side already contains the solution to the other side's
problems; this section makes the directions explicit.

### 11.1 id → event: `IdBuilder` is the existence proof

The id side normalizes at close time *on the packed stream*: a
fixed-width 2-bit tag patched in place, collapses as truncations. It
never needed a working form because nothing it writes ever widens.
Tier 2 (§10) is precisely the change that gives the event side the
same property — no internal payloads, collapse-by-truncation — after
which the two builders are one packed-tree-builder shape and should
be unified into shared machinery (one generic
open/close/copy-splice/truncate builder over a packed preorder
stream, parameterized by the per-node payload: none for ids, a leaf
delta for events).

### 11.2 id → event: leaf-dominance pruning in comparison

Id walks never recurse into a subtree that a leaf on the other side
dominates: `Empty`/`Full` settle the arm with an iterative skip. The
event comparison instead broadcasts `Zero` and recurses to full
depth. But leaf-versus-subtree dominance is a *scan*, not a walk
[derived]: for a leaf value `t` (path-sum-adjusted), `subtree ≤ t`
iff `max(subtree) ≤ t`, and `subtree ≥ t` iff every root-to-leaf
path sum ≥ t — both computable iteratively with a pending-children
counter and a running offset (the `skip_subtree` shape plus a small
value stack). Replacing the `Zero`-broadcast recursion with these
two scans makes comparison against a shallow operand cost O(deep
side) time but O(1)-ish space — which both neutralizes the
`cmp vs new()` shape of V1/V2 *and* speeds up the common honest case
of comparing against a much smaller version. Worth doing even if
Tier 2 later subsumes it, since it is small and self-contained.
**[open]**: `EvReader::max` (`version/event/max.rs`) currently
recurses; make it and the min-path scan iterative first.

### 11.3 id → event: splice-based single-path edits

`split` edits an id by copying verbatim bit ranges around a
retagged spine — no builder, no per-node work. `grow`'s emit is the
same shape on the event side: it rebuilds exactly one root-to-leaf
path and copies every off-path subtree unchanged. On a packed
output (either tier), grow-emit becomes: splice off-path spans
verbatim; re-emit only the path nodes' codes (whose widths change).
`tick`'s hot path (`fill` no-op, `grow` inflates a leaf) then costs
O(n) time with O(path) working state and no node-array at all.

### 11.4 event → id: word-scale scanning

The event side's window decoder settles a whole gamma code with one
`leading_zeros`. The id side still steps 2 bits at a time in
`skip_subtree`; tags pack 32 to a word, and the pending-children
counter's delta over a word is `popcount(word) − 32` with an early
exit when the counter would cross zero mid-word — a word-at-a-time
subtree skip [derived]. Same trick applies to the event topology
stream under Tier 2. Pure performance; adversary-neutral.

## 12. The decision: Tier 1.5 vs Tier 2 (holistic view)

Both reach the §6 invariant. The holistic question — performance,
hardening, *and* code simplification together — reframes it:

| | Tier 1.5 | Tier 2 |
|---|---|---|
| wire bytes | unchanged (byte-identical outputs) | breaking change: snapshots re-pin, **plus** the §10.5 blast radius — every message `Key` changes (flag day + application-level `Key` migration), `BOOKMARK_FORMAT_VERSION` bump, consumer-persisted serde/borsh forms break |
| transient memory | ~2–3× packed | ~1× packed + bit-stacks |
| passes per emit op | 2, plus mirror-read plumbing | 1 |
| decode validation | V5 fixed by frame slimming | 2 bits/level for topology; value nonnegativity needs running-value state — cliff-immune accumulator required (§10.2, §10.6) |
| carry-cliff genre (§10.6) | linear per input bit on the boundary comb **[measured**, envelopes pinned**]**; the §8.1 accumulator must be cliff-immune when it lands | Θ(W²) in wire bits for any plain running-value sweep, strict decode included **[measured]**; linear only under the §10.6 mitigation set, which is undesigned and unpriced |
| net code | *adds* machinery (scratch, mirror records, splice pass) beside the walks | *removes* machinery (working form, form-split reader, broadcast, deferred leaves, likely stacker) and unifies the two sides' builders (§11.1) |
| comparison | tree recursion retained (improved by §11.2) | interval sweep; §11.2 pruning falls out naturally |
| compactness | identical | ~≤2× envelope, sometimes better (§10.4) |
| compactness envelope **[measured** 2026-07-23, `testing/compactness` ratio meter + `meter/tier2` exact sizer, ~13k samples: arbitrary, organic-30/120/400, the §2 grids, alternating combs, realistic gossip populations**]** | identical (r = 1 by construction) | holds, strictly: `tier2 ≤ 2×current` outright on every sample (per-node excess `(tier2 − 2·current)/nodes` never above −1.57; ceiling pinned 0), Euler charged-at-most-twice validated with >1.3 bits/leaf slack; max r 1.9966 (comb, m=2048, 1024 pairs — monotone toward 2 from below, tight as §10.4 claims), max off-comb 1.9633; realistic gossip r median 0.9888, max 1.1798, Tier 2 smaller on 61.6%; Tier 2 smaller on most dense (10/11) and bigroot (30/36) samples, down to 0.75 |
| risk | moderate; total byte-identity oracle | higher (new codec + canonical form); oracle + differential suites carry it |

The win-win observation: Tier 1.5 hardens at the cost of *added*
complexity; Tier 2 hardens by *subtraction* — the event side ends up
shaped like the id side, one packed-builder abstraction serves both,
and the walk/cursor/broadcast scaffolding that exists to bridge
representations disappears. Its costs are the wire break and the
compactness envelope (§10.4), both checkable before commitment: the
ratio meter for compactness, the bench suite for speed, and the
oracle for correctness.

Amended 2026-07-23 (P2 carry-cliff and blast-radius rounds): two of
that cost accounting's premises were undersized, and the table above
now carries both. The wire break is not "snapshots re-pin" — it is a
durable-identity and persistence break (§10.5: `Key`s, bookmarks,
consumer serde/borsh forms) needing a flag day and an application
`Key` migration story. And Tier 2's packed operands stop bounding
value content (§10.6): without the cliff-immune accumulator and
delta-algebra designs, its sweeps — strict decode included — are
Θ(W²) on the boundary comb, a genre Tier 1.5 prices linearly under
today's coding. Neither kills Tier 2; both must be priced into any
DECIDED entry.

Recommendation: land Tiers 0–1 (common to both, independently
worthwhile, no format risk) plus the §11.2/§11.3 pruning and splice
wins; run the §10.4 ratio measurement; then choose. If the ratio
envelope holds and a protocol-change window exists, take Tier 2; Tier
1.5 is the fallback if the wire format must not move. A Tier 2
DECIDED entry is additionally gated on: (a) a designed-and-priced
§10.6 mitigation set (cliff-immune accumulators; delta algebra for
the linear functionals; the §6 invariant restated over content
bits), and (b) the user's explicit acceptance of the §10.5
identity/persistence blast radius, not just the snapshot re-pin.
**[open — record the DECIDED entry here.]**

## 13. The metering gate

Pin the §6 invariant the way `step!` pins time complexity:

- **Generators**: commit §2's four shapes (parameterized) beside the
  existing proptest generators in `testing/`.
- **Amplification board**: a red-green matrix over the *entire*
  public operation surface (`Version`, `Party`, `Clock`, `Batch` —
  including `Display`/`FromStr`/`Hash`, `rank`/`distance`/`lag`,
  `min_ticks`, `project`, `fork`/`forks`/`join_all`, `Clock::sync`,
  and both codec directions) × §2's input families. Each cell runs at
  two scales and reports, from the deterministic meters below (peak
  heap, stacker segments, limb ops; wall time shown but never
  asserted), a scaling exponent and a per-input-byte constant. Green
  = exponent ~≤ 1.15 with constants under pinned ceilings; red
  otherwise. Default sizes keep the whole board at seconds of
  runtime — the fast-iteration loop — with a size knob for records
  of record. The board exists to catch amplifiers the §3–§4 audit
  missed and to serve as the campaign's progress dashboard: it lands
  at P0 mostly red by construction, and the campaign's acceptance
  criterion is an all-green board.

  Landed 2026-07-22 (`before::meter::board`; runner
  `examples/amp_board.rs`, `just amp-board`; nextest smoke): 42
  operation rows × 5 families, with the 52 row × family combinations
  that lack an applicable operand excluded (the 19 version rows skip
  the id-pair family; the 10 party rows and the adversarial-party
  tick row skip the three event-only families), = 158 cells (amended
  2026-07-23: originally stated 41 rows; the committed table has 42,
  confirmed both by the `Op` entries in `board.rs` and by the
  distinct op labels in the rendered board — the cell total and the
  green/red split were already exact. Amended again 2026-07-23: the
  total was written as the bare product `42 × 5 = 158`, false as an
  equation; the inapplicable-combination exclusion above is the
  actual arithmetic, and the smoke test now pins the 158 exactly);
  P0 baseline **[measured]**
  59 green / 99 red (dev profile, `limb-meter` lit, meter columns
  byte-identical across runs). Ceilings pinned in the module:
  exponent ≤ 1.15; heap ≤ 16 B per input byte over an 8 KiB flat
  allowance; grown segments ≤ 1 flat; limb ≤ 128 ops per input byte,
  calibrated up from a first-cut 8 after the benign control ran red —
  amortized-linear per-node arithmetic records tens of unit-limb ops
  per packed byte, which is the contract's linear regime, and width
  blowups are caught by the exponent bound regardless. Surfaces red
  at P0 that §3–§4's path list does not name (the mechanisms are the
  known classes; the paths are new): **projection** (`Version /
  &Party`, `Clock::own_version`) — owned per-frame path sums (V1's
  mechanism) plus a `node_capacity_bound + id_bits.len()` builder
  pre-size (T0.2's pattern) — red heap ~95–130 B/B on every family;
  **the paper-notation parsers** (`FromStr`) — recursion frames on
  deep input, and magnitude-quadratic limb work rebuilding a huge
  parsed decimal (V3's mechanism on the string path, which V5 did
  not cover); **`Display`** — event- and id-side writer recursion
  frames plus ~24–31 B/B formatting heap (the id side is outside
  P1's op list); **`min_ticks`/`rank` on big bases** — the V3
  read-quadratic, correcting §15's "min_ticks … already
  invariant-clean" (true for heap, not for limb work). Two dashboard
  caveats of record: the board shares one process, so its heap
  numbers are indicative and the process-isolated envelopes in
  `tests/meter.rs` remain the enforced record; and segment counts
  have a ~1 MiB growth threshold, so P1's id walks read green at
  board-default depths — the meter suite's d = 250k scenarios are
  what pin P1.
- **Peak-heap meter**: counting `GlobalAlloc` in a dedicated test
  binary (one global allocator per binary; nextest's
  process-per-test isolation applies). Assert per operation ×
  generator family: `peak ≤ C_op · packed_bytes + K`, constants
  pinned in the test. Phase 0 lands current (bad) envelopes as
  thresholds so every later phase tightens a committed number.
- **Stack meter**: stacker segments bypass the heap meter
  [measured]; count them at the source — a counter in
  `recurse::grow` (segments × `STACK_GROWTH`) — and envelope it too.
  It reads zero once §8.3 converts the walks.
- **Limb-work meter**: V3-class regressions are invisible to `step!`
  (node visits, not arithmetic width). Count big-integer limb
  operations behind a test-only feature (a thin shim around `Base`
  arithmetic) and assert amortized-linear envelopes on
  hugeleaf/bigroot and on §8.1's carry-cliff generators. (Landed
  2026-07-23, P2: `meter::cliff_comb` is the paid-crossing
  generator, with decode/cmp/join envelopes in `tests/meter.rs` and
  the Tier 2 size and plain-sweep pins in `meter/tier2`; §8.1's
  unpaid-crossing fan generator lands with §8.1's implementation.
  The comb joins the board's input families if Tier 2 proceeds to
  implementation.)
- **Fuzz under a cap**: the existing fuzz targets gain a
  counting-allocator harness with a hard ceiling tied to input size,
  turning any future amplifier into a crash finding instead of a
  latent one.

## 14. Execution plan

Dependency-ordered; each phase `just gate`-clean; wire bytes are
byte-identical through P4 (the snapshot suite enforces it for free).

- **P0 — yardstick.** Commit generators + all §13 meters (the board
  included) with current envelopes as thresholds; state the §6
  invariant in the meter's module docs. The crate-doc statement of
  the invariant lands at P5, with the user's sign-off, once it is
  true. No behavior change. (Spec-first: fix the criterion of record
  before touching the artifact.)
- **P1 — Tier 0.** T0.1 limb-wise mantissa; T0.2 builder capacity;
  T0.3 iterative complement. Tightens: hugeleaf decode to linear
  time; hugeleaf join peak ×100 → ~×1; `without` per-operand depth
  exposure gone.

  Landed 2026-07-23, plus §15's mixed-add and unused-import fixes.
  One transcription amendment to §7: T0.3's bare pending-children
  counter under-specifies the emit — a counter alone can neither
  order the deferred terminals for absent *right* children nor
  resolve a both-children node's right-child kind (that tag sits a
  whole left subtree ahead). The landed `complement` is two iterative
  passes over the fixed-width tag stream — a backward scan resolves
  right-child kinds onto a bit stack, the forward scan emits with a
  two-bit-per-level pending stack — still no recursion and no
  per-level frames, a few bits per level. T0.1 landed as
  `BigUint::set_bit` accumulation (top bit first sizes storage once;
  an interim byte-buffer draft regressed decode-hugeleaf peak heap
  and was rejected by the P0 envelope — the gate catching its first
  real regression). Results **[measured, three identical runs]**:
  board 59 green / 99 red → **96 / 62**, 37 cells flipped green,
  none red (`version_decode dense` heap stays red: V5, untouched);
  envelope re-pins: decode-hugeleaf limb 122.1M → 1_954 (linear) and
  heap 55_827 → 46_883, join-hugeleaf heap 3_127_365 → 111_771,
  decode-bigroot limb 12.52M → 626, id-without segments 138 → 0,
  join-dense heap → 4_797_477, tick-dense heap → 9_469_952 (T0.2's
  push-growth beat the pre-size on every family). Adversarial
  benches (`benches/amplify.rs`, added at P1): hugeleaf decode
  −94.6%/−97.9% (linear across the doubling), bigroot join ~−24%,
  seed-without-spine ~−37%.
- **P2 — decision (§12), pulled forward.** Run the §10.4 ratio
  measurement over the existing generators plus §2's shapes;
  validate the compactness envelope; record the DECIDED entry in
  §12, confirming the wire break with the user before any
  implementation. Deciding here avoids building §8.2/Tier-1.5 emit
  machinery that Tier 2 deletes; §8.1's difference accumulator is
  not throwaway either way — it is the core of the Tier 2 sweep
  (and must be cliff-immune per the §8.1 amendment).

  Amended 2026-07-23, P2 negative-space round: the review's two
  blocking findings are priced into the decision packet. The §10.5
  wire break is a durable identity/persistence break (Keys,
  bookmarks, consumer serde/borsh), and the §10.6 carry-cliff genre
  makes every plain Tier 2 running-value sweep Θ(W²) — the boundary
  comb generator, its `tests/meter.rs` envelopes, and the
  `meter/tier2` size and plain-sweep pins landed with this
  amendment. The DECIDED entry remains open, gated as §12 records.
- **P3 — endgame.** Tier 2 (expected): §10's representation, codec,
  validation, and sweeps; §11.1 builder unification; §11.2's
  pruning falls out of the sweep; §11.3 splice emit; snapshot
  re-pins. Fallback if the ratio envelope fails: §8.1 + §8.2 on the
  current representation with the `Base` mixed-add fix, §11.2
  leaf-dominance scans, §11.3 splice-based grow emit, then Tier 1.5
  (§9). Either way the working form is deleted.
- **P4 — stacks.** §8.3 explicit compact stacks for the id walks
  and every walk still recursive after P3; both parsers if V5
  machinery survives P3; §11.4 word-scale scanning where profiles
  justify. Tightens: id walks ×418–456 → ≤ ~×10; any surviving
  event-walk RSS to small constants. Audit whether `stacker` can be
  dropped.
- **P5 — closeout.** Tighten every envelope to its final constant;
  make the fuzz cap proportional to input; benches not regressed
  (improvement expected at P1 and P3); crate-doc invariant
  statement and any README/crate-doc updates (user sign-off);
  `just all` clean.
- Each phase updates metering thresholds downward in the same commit
  that earns them.

Amended 2026-07-22, execution start: the §12 decision moved from
last to P2, before any emit-path work — the user's prior favors
Tier 2, and deciding first avoids building Tier-1.5 machinery that
Tier 2 would delete. The §13 amplification board was added as a
deliverable and as the acceptance criterion. Process constraints of
record: user-facing documentation (crate docs, READMEs, public-item
rustdoc prose) changes only with the user's sign-off; test and bench
coverage only ratchets upward through the campaign; the board and
meter suites default to seconds-scale sizes so the inner loop stays
fast.

Added 2026-07-23 (P3 planning): full-surface measurability is a
deliverable of the rework, not a hope. Every public operation must
be (a) proptested against both reference oracles — the recursive
tree oracle and the semantic (function-space) oracle — and (b)
resource-pinned by a board row or an enforced meter envelope. Any
operation found lacking either, at any point in P3–P5, is a coverage
finding to be fixed in the phase that finds it.

Amended 2026-07-23, P0 review round 2: the "each phase
`just gate`-clean" bar is blocked at P0 by the streaming-wire mux
deadlock inherited from main (`rumors` gossip sessions park;
`run_to_quiescence` reports `Stalled`), recorded here as the roster
of record. Two provenances, both verified 2026-07-23 by replay:

- Main's own committed seeds already fail at the merge-base:
  `rumors::shadow_validity::shadow_predicts_live_state` stalls at
  plain 6b39482d replaying the second seed of
  `tests/shadow_validity.proptest-regressions` (committed at
  83edcd94). Main's gate is red today with no campaign changes
  applied.
- This campaign's gate runs generated two new seeds for the same
  deadlock — one line each in `tests/pairwise.proptest-regressions`
  and `tests/multi_peer.proptest-regressions` (20fd050a) — which
  also replay deterministically at the merge-base. Regression files
  replay against every property in their file, so these two lines
  fail ten tests: `pairwise::{gossip_converges, gossip_idempotent,
  gossip_side_symmetric, gossip_order_independent,
  gossip_unions_content}` and
  `multi_peer::{all_peers_converge_after_quiesce,
  each_key_observed_at_most_once_per_peer,
  keys_stable_across_peers, readout_matches_oracle_after_quiesce,
  quiesced_state_is_gossip_fixed_point}`.

The full roster is those eleven tests and no others: a
`--no-fail-fast` workspace run at this branch's tip shows 685
passed, 11 failed, every failure the `Stalled` witness, and every
other gate stage green. (Fail-fast cancellation hides roster
members on an ordinary gate run — earlier counts of two and nine
tests were both truncations — so gate runs during this campaign are
read against this roster.) Amended 2026-07-23, P1 close: the roster
is now **fourteen** tests. bc4320e5's retire seed fails both
`retire::{retire_matches_plain_gossip,
unsynchronized_retire_matches_plain_gossip}` (a regression file
replays against every property in its file), and P1's final
`--no-fail-fast` sweep generated one more seed for the same
deadlock, `tests/async_wire.proptest-regressions`, failing
`async_wire::async_gossip_converges_on_the_union` — verified by
replay at bc4320e5, before any P1 change, with the identical
`Stalled` witness (701 run: 687 passed, 14 failed, 2 skipped at P1's
tip; an earlier draft of this amendment recorded 698/684, the count
of a pre-tip sweep taken before P1's late test additions — corrected
2026-07-23 against a fresh `--no-fail-fast` sweep at the tip, same
fourteen-test roster). The stall is transport-buffer
independent (identical at 8 KiB and 64 MiB duplex), matching the
wait-cycle diagnosis in `design/streaming-wire-deadlock.md` on the
`link-transport` branch, whose determination is a stream-capable
transport contract and which rejects capacity mitigations as
unsound. At that branch's tip (b93541b4) with this branch's seed
files copied in, all fifteen tests across the three files pass.
This campaign therefore does not fork a competing transport fix:
the deadlock's fix of record is the in-flight `link-transport`
work, and these seeds become its regression pins when it lands.
Escalated for the user's ruling: either P0's gate bar is met by
rebasing onto main after `link-transport` merges, or the user
records an explicit exception here so P0 can close first.

Acceptance for the effort: every §5 row at a small pinned constant ×
input; the §13 board all green — no super-linear cell and no
large-constant cell anywhere in the op × family matrix; `benches/`
not regressed (improvement expected at P1 and P3).

## 15. Adjacent findings

- `Add<&Base> for &Base` clones both operands via `to_biguint()` in
  any mixed/big case; `Small + Big` should cost one big clone at
  most. Subsumed by P2; worth fixing in `Base` regardless.
- `Display`/`Debug` on a big-base version does full decimal
  conversion (superlinear); logging one adversarial or organic huge
  version is a CPU sink. Consider digit-capped rendering with an
  explicit elision marker. **[open]** whether any hot path formats
  versions.
- `grow`'s builder capacity adds `id_bits.len()`; an oversized party
  inflates the allocation. Subsumed by T0.2.
- Building `before` with default features emits an unused-import
  warning (`parse_ev_from`, `parse_id_from` in `codec`) visible to
  downstream consumers though hidden from the all-features workspace
  gate. Feature-gate the imports.
- `Rank`'s exponent-alignment shifts allocate ~tree-depth bits —
  linear, acceptable; the meter should cover `rank`/`distance`/`lag`
  to keep it that way.
- `min_ticks` (saturating u64 fold), `Route` (bit-vector), and
  `split` are already invariant-clean.

## 16. Cross-references

- Event-side architecture this document modifies:
  `src/version/event.rs` module doc (walk shape), `src/version/
  compare.rs` (comparison), `src/version/working.rs` (working form),
  `src/version/event/builder.rs` (the sink), `src/codec/gamma.rs`
  (codec + window fast path), `src/recurse.rs` (stack guard and its
  frame-size measurement).
- Id-side architecture: `src/idbits.rs` (cursor, `skip_subtree`),
  `src/party/ops/` (walks and `IdBuilder`).
- Testing architecture the plan leans on: `src/testing/` module docs
  (oracle differential, exhaustive small-scope, algebraic laws,
  complexity metering).
- Wire pinning affected by Tier 2 only: `tests/gossip_snapshot.rs`
  and the `insta` snapshots (workspace root) — and, beyond the test
  pins, the §10.5 durable surfaces: `src/tree/typed/path.rs` (leaf
  paths hash `Version::as_bytes`), `src/tree/key.rs` (`Key`'s
  cross-replica stability and persistability contract),
  `src/bookmark/format.rs` (versioned durable frame over
  borsh-encoded `Clock`s), `crates/before/src/borsh_impls.rs` and
  `crates/before/src/serde_impls.rs` (canonical bytes as the
  serialized forms).
