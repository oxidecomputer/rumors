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

Amended 2026-07-23 (P3.3, the denomination criterion of record):
"packed operands" and "packed input bits" denominate every
operation *except* the two classes whose mandatory output is
asymptotically larger than any constant times their input, for
which an input-only bound is unsatisfiable by construction (a
perfect text writer emits Θ(nk) mandatory digits from Θ(n + k)
wire bits on the §10.6 comb) and would degenerate into exemption
holes. Those denominate against total I/O:

- **Text I/O** (`Display`/`FromStr` for `Version`/`Party`/`Clock`):
  judged against `n_io` = packed input + text output (`Display`) or
  text input + packed output (`FromStr`), output read from the
  actual result, with every ceiling numerically unchanged. The limb
  column on these rows carries two legs (amended 2026-07-23, P3 fix
  round: the original κ-only wording claimed subquadratic
  enforcement a constant ceiling cannot give — a u32-chunked
  schoolbook probe scored 0.11 limb/`R` on hugeleaf and 0.15 on
  bigroot, under κ while quadratic **[measured]**). The *constant*
  leg is judged against the radix-work denominator
  `R = n_io + Σ digitsᵢ × limbsᵢ` (the schoolbook conversion cost
  law), pinned at the divide-and-conquer target κ = 0.25
  limb/`R`-unit — which the digit-by-digit parser measurably
  exceeds ~4×, so re-denomination alone flips nothing on the
  limb-red cells. The *exponent* leg is judged against `n_io` —
  never against `R`, on which any schoolbook converter reads a flat
  ~1 — at the unchanged 1.15 ceiling, which chunked schoolbook
  exceeds at ~2 **[measured]**; that leg is what enforces the
  complexity class. Only a converter with near-linear recorded limb
  work and a D&C-class constant reads green. An output-honesty
  assertion (text bytes ≤ 4 × packed content bits, checked against
  actual bytes) closes the pad-the-output door.
- **Output-dominated projection** (`version_project`/
  `clock_own_version` on the comb × scattered-party cross): judged
  against `n_io` = packed input + packed output (canonical coding
  cannot be padded), ceilings unchanged.

Everything else stays input-denominated — both binary codec
directions (canonical 1:1), all scalar/comparison/query rows, and
the packed-output mutators, whose input denomination rests on
output coding ≤ inputs + O(1) per overlay boundary, pinned for
join/meet as the 1-Lipschitz proptest in `meter/tier2`'s suite
(boundaries ⊆ union of the inputs'; total Tier 2 bits within 4
bits per input leaf of the inputs' sum) rather than assumed.

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
- **Unpaid crossings** [derived; the generator landed 2026-07-23 at
  P3.2 as `meter::cliff_fan`, canonicality and closed-form size
  pinned, its entry/exit stream in the accumulator envelopes]: hang
  teeth `(1, 0, 1)` — 12 stored bits each — from a base-0 fan under
  a single stored `2^k − 1` root. The path sum sits at the cliff across the
  whole fan, every tooth's `+1`/`−1` re-crosses it, and one
  comparably-coded magnitude forces `n` excursions at O(1) input
  bits each: Θ(nk) limb work in a Θ(n + k)-bit input for any
  accumulator that materializes each `D ± base` as a plain big
  integer. The Dyck argument does not save this case — the
  excursions are siblings, not nested.

The implementation requirement this adds: the difference accumulator
must be cliff-immune. Re-amended 2026-07-23 (P3.2; the accumulator
probe): the representation of record is the redundant balanced
base-2^32 signed-digit form, landed as `codec::accum` — `Vec<i64>`
digits, value `Σ dᵢ·2^32ⁱ`, lazy zone `|dᵢ| < 2^33`, every
out-of-zone write carrying `c = (t + 2^31) >> 32` and recentering
the remainder into `[−2^31, 2^31)`; `sign()` folds from the top,
decides at running partial `|s| ≥ 3` (top-digit domination), and
collapses scanned cancelling prefixes — a value-preserving `&mut`
rewrite, amortized against the writes that built the prefix.
Because every write recenters, no normalized region exists anywhere
in the representation: amortized O(1) per small op, O(|x|) per wide
`± Big(x)` paid by `x`'s own code, at every delta width. The first
amendment's design — a normalized big part plus one machine-word
signed offset, renormalized on word overflow — is **refuted**: any
such two-zone form has a zone boundary, and the wide-tooth comb
(teeth `±2^192` across a `2^k` cliff, `k ≫ 192`;
`meter::wide_tooth_comb` is the packed family) drives it through
its normalized prefix every tooth, measured 70.5 → 134.5
limbs/delta as `k` doubles — quadratic again **[measured** —
accumulator probe, exact-oracle-checked; the §17.1 finding of
record**]**. The balanced form is flat on both shapes: 2.000 digit
touches/delta on the boundary comb and 6.000 on the wide-tooth comb
across a size doubling **[measured** 2026-07-23 — the
`tests/meter.rs` accumulator envelopes, pinned ×1.25, three
identical runs**]**. Leaf direction tests `sign(D + x − y)` apply
`+x, −y`, read the sign, and restore `−x, +y` — every step paid by
the codes of `x` and `y`, and exact because `sign()`'s rewrite
never changes the held value. With that representation the O(n + m)
total stands on both shapes.

Amended 2026-07-23 (P3 accumulator review round): the four delta
streams in the P3.2 acceptance list (boundary comb, wide-tooth,
fan, cancelling-prefix) fund every deep sign scan with an
immediately adjacent wide write, so none of them enforces the
collapse — with the collapse branch disabled outright, all four
envelopes still pass (the cancelling stream reads 252/251
milli-touches per coded byte against the 314 ceiling) **[measured**
— perturbation run, collapse branch disabled**]**. The
collapse-is-load-bearing shape is read-heavy: a cancelling prefix
built once, then many sign reads with unit writes. The suite now
pins it — the static-prefix stream (`+2^k`, `−(2^k − 1)` as
excluded setup, then `n` `±1`/sign cycles): 2.006 digit touches per
sign read, flat across the `k`, `n` doubling, versus 66.0 per read
at `k = 2048` (growing linearly with `k`) with the collapse
disabled **[measured** 2026-07-23 —
`tests/meter.rs::accum_static_prefix_touches_flat`, pinned ×1.25,
three identical runs; perturbation run for the no-collapse
figure**]**.

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
cliff-immune accumulator §10.6 requires (the balanced signed-digit
form of record, `codec::accum`: nonnegativity is then a sign check
on the redundant form, amortized O(1) per small delta). V5's *frame*
elimination stands either way.

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

Amended 2026-07-23 (P3.5, closing §17.1's Gate A): **the join/meet
subadditivity lemma of record** [derived — full proof below; pinned
by `meter/tier2`'s emitter-parameterized subadditivity suite]. For
canonical versions `a`, `b` and `c` either `a ∨ b` (pointwise max)
or `a ∧ b` (pointwise min), the skyline coded size satisfies

    size(c) ≤ size(a) + size(b) − 2   (bits),

where `size` is the exact skyline bit length (`meter::tier2::
tier2_size`; its bit-for-bit equality with the skyline encoder's
output length is proptest-pinned). The byte-level corollary
`⌈size(c)/8⌉ ≤ ⌈size(a)/8⌉ + ⌈size(b)/8⌉` follows immediately
(`⌈(x + y − 2)/8⌉ ≤ ⌈x/8⌉ + ⌈y/8⌉`), and is the form the
`link-transport` window budget cites after the C2 flip — the same
shape as the bound it holds today against the current coding, so
Gate A's reroute clause is never exercised.

Notation. `size(x) = N(x) + γ(v₁(x)) + Σ_{t ∈ B(x)} g(|Δx(t)|)`:
`N(x)` nodes of the canonical tree (topology flag bits), `v₁(x)`
the first preorder leaf's absolute height, `B(x)` the set of
internal boundaries of x's canonical leaf partition, `Δx(t)` the
signed (possibly zero) jump of the step function x at boundary t,
and `g(m)` the zigzag-gamma code length of a delta of magnitude m.
Every step below is proven; nothing in the lemma's chain is
measured-only.

1. **Sign flips are free, and `g` is monotone.** `g(0) = γ(0) = 1`;
   for `m ≥ 1` a positive delta codes as `gamma(2m)` and a negative
   one as `gamma(2m − 1)`, and `⌊log₂(2m + 1)⌋ = ⌊log₂(2m)⌋`
   because `2m + 1` is odd and greater than 1, never a power of
   two — so both signs cost `2⌊log₂(2m)⌋ + 1` bits. `g` therefore
   depends only on the magnitude and is nondecreasing in it.

2. **The output topology embeds in the union of the input
   topologies.** Take `T_a`, `T_b` as the canonical trees' node
   sets: prefix-closed, every internal node carrying both children.
   Their union is again such a set (a node internal in the union is
   internal in an input, which carries both children there), and
   `c` is constant on each union leaf: a union leaf is a leaf of
   one input and lies inside a leaf of the other (walking up from a
   node absent from a tree, the first ancestor present in it must
   be one of its leaves), so both arguments of the pointwise op are
   constant there. The union tree therefore represents `c`. The
   canonical tree of a step function is contained in *every* tree
   representing it — a node the function is non-constant on cannot
   be a leaf of any representing tree, so by induction from the
   shared root the canonical node set descends inside — hence
   `N(c) ≤ |T_a ∪ T_b| ≤ N(a) + N(b) − 1`, the −1 because both
   inputs contain the root.

3. **Output boundaries embed in the union of input boundaries.**
   The union tree's partition has boundary set exactly
   `B(a) ∪ B(b)`: every partition boundary is the midpoint of an
   internal union node, which is internal in at least one input and
   is that input's boundary between two consecutive leaves; and
   conversely every input's internal node stays internal in the
   union. `c`'s canonical partition coarsens the union's (step 2),
   so `B(c) ⊆ B(a) ∪ B(b)`.

4. **Pointwise max/min is 1-Lipschitz at every boundary.** At
   `t ∈ B(c)`, `δ_c(t) = op(a(t⁺), b(t⁺)) − op(a(t⁻), b(t⁻))`, and
   for op ∈ {max, min},
   `|op(x₁, y₁) − op(x₂, y₂)| ≤ max(|x₁ − x₂|, |y₁ − y₂|)`, so
   `|δ_c(t)| ≤ max(|Δa(t)|, |Δb(t)|)`, where `Δx(t) = 0` whenever
   `t ∉ B(x)`.

5. **The delta charge is an injection.** Charge each `t ∈ B(c)` to
   one input delta code at the same point: if `δ_c(t) ≠ 0`, to an
   input whose jump at t has magnitude `≥ |δ_c(t)|` — it exists by
   step 4, its jump is nonzero so t is genuinely its boundary, and
   its code there covers the output's by step 1
   (`g(|δ_c(t)|) ≤ g(|Δx(t)|)`); if `δ_c(t) = 0` (a canonical zero
   delta across a subtree boundary), to either input holding a
   boundary at t — one exists by step 3, and any delta code costs
   at least `1 = g(0)` bit. Distinct output boundaries charge codes
   at distinct points, so the charge is injective and
   `Σ_{B(c)} g(|δ_c|) ≤ Σ_{B(a)} g(|Δa|) + Σ_{B(b)} g(|Δb|)`.

6. **The first leaf is one of the inputs'.**
   `v₁(c) = op(v₁(a), v₁(b))` equals `v₁(a)` or `v₁(b)`, so
   `γ(v₁(c)) ≤ γ(v₁(a)) + γ(v₁(b)) − 1`: the unmatched input's
   first-leaf code is at least 1 bit.

Summing steps 2, 5, and 6 gives `size(c) ≤ size(a) + size(b) − 2`.
The constant is tight: `a = b =` the empty version gives
`2 = 2 + 2 − 2`. The Gate A probe agrees at exactly this constant
**[measured** — ~1.5M pairs: every §13 generator family and
adversarial cross, the exhaustive small scope, and a directed
hill-climb on the excess `size(c) − size(a) − size(b)`; maximum
observed excess −2**]**.

Two scope notes, per the statement-faithfulness bar. The lemma is a
statement about the canonical skyline stream of the operation's
*value*, so it binds any correct emitter's output but says nothing
about a buggy emitter's bytes — which is why the `meter/tier2` pins
take the join/meet emitters as parameters, run today against the
packed-form operators, and must be re-instantiated against the
skyline emission kernel's actual output when that emitter exists.
And the lemma prices coded size only: it does not bound the *work*
of the sweep producing `c` (§10.6's ledger does), and it does not
claim `B(c)` reaches the union bound — normalization can erase
boundaries, and step 2's shared root alone already yields the
1-Lipschitz pin's strict leaf inequality
(`L(c) ≤ L(a) + L(b) − 1`, from `N = 2L − 1`).

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
  decode validation. Re-amended 2026-07-23 (P3.2): the
  representation of record is the redundant balanced base-2^32
  signed-digit accumulator (`codec::accum`; the §8.1 re-amendment
  carries the form and the measured record) — no normalized region
  anywhere, so small-delta application and sign/nonnegativity
  checks are amortized O(1) and wide deltas stay paid by their own
  codes, at every delta width. The two-zone design this bullet
  first named (big part + machine-word signed offset) is refuted by
  the wide-tooth comb — 70.5 → 134.5 limbs/delta as `k` doubles
  (§17.1) — where the balanced form measures flat (2.000 and 6.000
  digit touches/delta on the boundary and wide-tooth combs,
  `tests/meter.rs`). Note §8.1's fan shape means Tier 1/1.5 needs
  the same representation — the difference is that Tier 2 *cannot
  ship without it* (its strict decode is on the hook), while under
  today's coding only the future difference-tracked compare is.
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
**DECIDED 2026-07-23 (Finch): Tier 2 — the skyline encoding — as a
flag day.** Evidence of record: the §10.4 envelope measured to hold
outright (max ratio 1.9966, comb-tight; honest-regime median
0.9888, §12 table); all five P3 derisk probes promising (§17); the
fate map killing 49/62 default-scale red cells at the flip.
Ratified with the decision: (a) the identity and persistence break —
content-address leaf paths, borsh/serde bytes, one universe upgrades
atomically, application-level `Key` migration out of scope; (b) the
§17 P3.3 text-cell denomination amendment (R-denominated, κ-pinned,
harder-not-softer per the recorded arithmetic); (c) blanket
authorization for the mechanical byte re-pins at C2 (~55+ snapshots,
fuzz seeds, codec witness literals), reviewed bytes-only. Bookmark
posture: no v1 bookmark files exist anywhere (pre-production), so
`BOOKMARK_FORMAT_VERSION` bumps 1→2 with strict reject and no
migration machinery. Display/FromStr hardening is deprioritized to
the plan's tail (debugging surfaces, not interoperation — user
ruling, §17). The window-budget subadditivity lemma stands unless
the §17 Gate A probe falsifies it; on falsification only, a
proptested and analytically derived replacement estimate. Tier 1.5
(§9) is not pursued; it remains in this document as the evaluated
alternative.

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

  Landed 2026-07-23 (P3.3, the denomination hardening; the
  criterion itself is the §6 amendment): the ten text-I/O cells and
  the new projection cross are I/O-denominated with the output side
  read from the actual result; the text rows' limb column is judged
  against `R` at κ = 0.25 (provisional **[derived]** — 4× under the
  measured schoolbook ratio, ~4× over the probe-extrapolated D&C
  ratio at the default hugeleaf scale; re-pinned from the observed
  meter when the chunked converter lands, verified at record scale
  before enforcement); the output-honesty assertion and the
  join/meet 1-Lipschitz proptest (`meter/tier2`) are in the tree,
  and `meter::board`'s module doc records the do-not-re-denominate
  list. The board gains the one operand cross the probe sweep found
  missing — `version_project`/`clock_own_version` × comb-scatter
  (`cliff_comb` × the `scattered_id` generator, every-other-tooth
  ownership) — so the output-dominated case is visible: 42 rows ×
  6 families minus 92 inapplicable = **160 cells**, pinned by the
  smoke test. Re-baselined **[measured** — board re-run at landing,
  dev profile, `limb-meter` lit**]**: 96 green / 64 red at the
  default scale. Within that: the ten pre-amendment text reds stay
  red — the four limb-only `FromStr` cells now read exactly the
  schoolbook law (hugeleaf and bigroot at 1.0 limb/`R` against
  κ = 0.25; the `meter::board` test suite pins that the schoolbook
  parser exceeds κ, so the ceiling cannot silently soften), the
  rest stay red on their unchanged segment legs while their
  mandatory-output heap constants restate honestly
  (`version_display × dense` 23.5 → 1.4 B/B against `n_io`, still
  red on segments) — and the criterion being *harder* surfaces two
  reds the input denomination hid: `version_from_str`/
  `clock_from_str` × benign at 1.4 limb/`R` (schoolbook on organic
  values), owner P3.8 with the rest of the text column. The two
  cross cells read green at the default scale (heap 0.9 B/B of
  `n_io`, segments 0): the cross exists for record-scale
  visibility and for the post-flip era, when the comb input
  collapses to Θ(n + k) while the projected output stays Θ(e·k).
  `Display`'s limb column remains an under-count (0.0/`R`: today's
  writer routes around the metered arithmetic — the false green
  §17.2's P3.3 entry names) until the shared metered converter
  lands with P3.8.

  Amended 2026-07-23 (P3 fix round, the chunked-schoolbook
  refutation): κ alone did not enforce subquadratic conversion. A
  u32-chunked schoolbook probe — 9 digits per metered mul-add
  pair, still Θ(digits × limbs) — scored 0.112 limb/`R` on
  hugeleaf(16k), 0.112 on hugeleaf(32k) (flat across the
  doubling), and 0.154 on bigroot(8k, 2k), all under κ = 0.25
  **[measured** — probe through the board's own metered ops**]**,
  so a chunked-schoolbook P3.8 would have read the four `FromStr`
  limb cells (and the two benign ones) green with no committed
  check objecting. The criterion of record now judges the text
  rows' limb *exponent* against `n_io` — like every other
  exponent, and never against `R`, on which any schoolbook
  converter reads a flat ~1 — at the unchanged 1.15; the constant
  stays against `R` at κ. Chunked schoolbook reads exponent 1.99
  against `n_io` **[measured]**; the committed digit-by-digit
  parser 1.92–1.99 (its four cells now red on both limb legs);
  P3.8's D&C target sits in the ~1.05–1.1 recorded band, verified
  at record scale before the κ re-pin. Board re-run at landing,
  both scales: verdicts identical (96/64 default, 85/75 at ×4 with
  exactly the §17.3 eleven). Anti-softening is now two committed
  tripwires in `meter::board`'s test suite: the digit-by-digit
  parser exceeds κ, and the chunked probe — driven through the
  board's own `evaluate` — slips under κ and must read red on
  exactly the limb exponent.

  Landed 2026-07-23 (P3.4, the acceptance scale of record). The
  pinned record scale is **×4** (`board::RECORD_SCALE`,
  `just amp-board-record`) — the §17.3 witness floor, at which
  every known segment-onset amplifier reads red. **Campaign
  acceptance is all cells green at BOTH the default scale and the
  record scale, three identical runs each**; record runs are
  acceptance-time only and the enforced record remains
  `tests/meter.rs`. Both-scale baseline of record **[measured** —
  2026-07-23, dev profile, `limb-meter` lit; the record-scale red
  enumeration byte-identical across two consecutive runs**]**:
  **96 green / 64 red at the default scale; 85 green / 75 red at
  ×4** — the 64 default reds unchanged (none flips green at ×4)
  plus exactly the §17.3 witness's eleven segment-onset cells, no
  cells beyond them, each owner-tagged per §17.3: `version_rank ×
  bigroot` and `version_min_ticks × dense` (P3.6);
  `version_display`/`clock_display` × bigroot and
  `party_from_str`/`clock_from_str` × id-pair (P3.8);
  `party_join`/`party_covers`/`party_disjoint`/`clock_join`/
  `clock_sync` × id-pair (P4.1). Calibration, per mechanism
  **[measured** — ×1/×2/×4 sweep**]**: the id-walk lockstep
  recursion cells (the seven id-pair onsets) and the
  `min_ticks` event walk first read red at ×2; the bigroot-spine
  onsets (`rank`, both `Display` cells) need ×4 — so ×2 under-
  detects and the floor is ×4, not lower. The ×2 sweep also shows
  why an intermediate scale is not the record: two sub-KiB benign
  `encode` cells read a heap-exponent artifact there (allocator
  rounding on inputs under 1 KiB, green at both ×1 and ×4 — the
  indicative-heap caveat above). Record-scale runtime budget: ≤
  30 s of summed measured-body wall per family; today's total is
  ~21 s, dominated by bigroot at 12.5 s (its schoolbook text
  conversions), against ~3 s for the whole default board.

  Landed 2026-07-23 (P3.5's board family, C1a). The boundary comb
  joins the board's input families as the `cliff` column
  (`cliff_comb` at `k = n`, `board::CLIFF_BASE_SCALE` = 128, ~4 KiB
  packed at scale 1.0): 42 rows × 7 families minus 103 inapplicable
  = **191 cells**, pinned by the smoke test; scaling `k` with `n` is
  the deliberate separating choice — the comb's absolute value
  content grows quadratically in its packed input, so a
  running-leaf-value sweep in a plain big integer reads a
  superlinear exponent instead of hiding a `k`-sized constant.
  Re-baselined at both scales **[measured** — 2026-07-23, dev
  profile, `limb-meter` lit; red enumerations byte-identical across
  two consecutive runs per scale**]**: **125 green / 66 red at the
  default scale; 114 green / 77 red at ×4** — the prior red sets
  unchanged at both scales, plus exactly two cliff cells:
  `version_from_str × cliff` and `clock_from_str × cliff`, red on
  both text limb legs (exponent 1.27 against `n_io`, constant
  1.2/`R` at the default scale; 1.41 and 1.1 at ×4), owner **P3.8**
  with the rest of the text column. The anticipated sweep-path
  interim reds did not materialize, and the mechanism says why
  rather than excusing it: the board's operands are today's packed
  coding, which stores a fresh `gamma(2^k − 1)` per tooth, so every
  cliff crossing is paid by a comparably-wide input code and the
  sweep-path rows are linear per input bit — exactly the
  `tests/meter.rs` cliff envelopes' record. The §10.6 Θ(W²) genre is
  denominated in *Tier 2 wire bits*, which reach the board only at
  C2, when the comb's packed operands collapse to Θ(n + k) against
  Θ(nk) value content; the column exists (and its `k = n` scaling is
  pinned) so that regression becomes board-visible the moment the
  flip lands. Realization staging is unchanged: the 29 green cliff
  cells must stay green through C2 on the new codec's sweeps.

  Landed 2026-07-24 (the 200-cell re-baseline; P3.6 fix round — the
  both-scale record the board extension landed without). The
  §17.7/§17.9 additions — the harmonic column on the four
  linear-functional rows, the `rank_pair_ops` row × {dense,
  harmonic, benign}, and the `version_join_all`/`party_join_all`
  fold rows on the scatter recipe — grew the board to 45 rows and
  **200 cells**, pinned by the smoke test. Both-scale baseline of
  record over the enlarged board **[measured** — 2026-07-24, dev
  profile, limb-meter and scan-meter lit; each scale run twice,
  red enumerations byte-identical**]**: **129 green / 71 red at
  the default scale; 117 green / 83 red at ×4**. Against the
  191-cell record: the 66 default reds reproduce unchanged plus
  exactly the five new-family cells (`version_rank`/
  `version_distance`/`version_lag` × harmonic — V6's remaining
  segment legs; `version_join_all`/`party_join_all` × scatter —
  the §17.9 n·log n marginals); at ×4 the §17.3 eleven reproduce
  plus two record-scale-only reds owned by §17.3's dated amendment
  (`version_min_ticks × harmonic`, segments 24 at exponent 3.59;
  `rank_pair_ops × benign`, limb exponent 1.87 over a 6 → 7-byte
  input), and one default red flips green: `party_join_all ×
  scatter` reads scan 104.6/B against the 96 ceiling at the default
  scale but 92.5/B at ×4 — a **non-monotone verdict** (the
  reduction's n·log n constant meets a flat per-byte ceiling whose
  slack grows with the operand), so the default scale is the
  binding witness for this cell and the ×4 witness under-detects
  it: record-scale greenness must never be read as clearing it.
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

### Surface judgment: liveness floors and the judged time exponent (recorded 2026-07-24, user directive)

The board's call sites are the public API, but three of its four
judged columns are sensors inside the implementation: segments
(`recurse::grow`), limb (the `Base` hooks), and scan (the cursor
and builder primitives). A ceiling over such a counter proves the
*instrumented* work is small, not that the operation is cheap — the
coupling assumption ("all work flows through instrumented
primitives") is exactly what an implementation change breaks, and
the C2 flip re-routes every operation at once. The empirical
instance is on record: P3.5's cliff-immunity envelope read green
over an accumulator the limb meter could not see, until the touch
floor landed (b905b584). Sharpest form of the gap: wall time is
displayed but never judged, so a kernel doing quadratic work in
plain machine-word arithmetic — no allocation, no recursion, no
metered reads — reads green on all four columns.

The user's ruling (2026-07-24): the board judges the API surface
as well as its implementation. Two legs, both landing before C2:

1. **Uniform liveness floors, cell × column.** The cell-judging
   code requires a floor-or-NA declaration per judged column: a
   semantic floor (what the counter must read, at minimum, if the
   meter is watching the work — derived from what the operation
   must do, never from how it does it), or a documented reason the
   column cannot be exercised, so a cell cannot be added without
   answering the question. The universal leg is scan: an operation
   that must examine its input scans at least a stated fraction of
   the input bits. Limb floors bind where big-integer arithmetic
   is semantically mandatory (the rank family, text). Segments
   keeps ceiling-only (its honest floor is zero). A floor trip is
   a designed stop-and-look: an implementation that legitimately
   does less work lowers the floor deliberately — the ceilings'
   ratchet discipline pointed the other way.
2. **The wall-time exponent leg.** Judge the time exponent fitted
   across the board's two scales at a generous ceiling (≤ 1.3:
   robust to scheduler noise, impassable for a quadratic's ~2.0);
   constants stay displayed-never-judged. Heap is the
   implementation-agnostic witness for space; this leg is the same
   witness for time.

Tripwires per §14's rule (every criterion needs a demonstration
that the status quo fails it): a test-only implementation that
does its work outside the metered primitives must read green under
ceilings alone and red under the committed floors; the time leg's
tripwire is an unmetered quadratic reading red on the exponent.

Acceptance is re-denominated: the all-green board of record means
ceilings AND floors AND the time-exponent leg, at both scales.

Landed 2026-07-24 (both legs, this directive's implementation).
Every board cell now carries a `Floors` declaration the type
demands: `Cell` cannot be constructed without a floor-or-NA answer
per judged column (`board::Liveness`, `Floor { min, why } |
NotApplicable { reason }`), rendered per cell (`flr[...]`) with the
derivations and NA reasons printed as a legend above the matrix;
segments is ceiling-only by policy (its honest floor is zero),
stated once at the type and in the board header, not per cell.
Floors bind in the same `evaluate` pass the ceilings do, at both
scales; a trip is red with the column and mechanism named
("counter reads below floor: the meter is not watching this
work"). The floor-derivation conventions of record:

- **Scan** (the universal leg): full-examination floor
  `SCAN_FLOOR_BITS_PER_INPUT_BYTE` = 1 bit per packed operand byte
  (an eighth of the stored bits; honest walks measure ~8) on every
  row that must examine its operands; the early-exit rows
  (`covers`, `fork` on a nontrivial party) floor at
  `SCAN_TOUCH_FLOOR_BITS` = 2 (the root codes); `version_eq`
  carries the one deterministic-liveness floor (the causal walk
  reads its operands in full today; a bytewise equality lowers it
  deliberately). NA is reserved for wholesale byte moves
  (encode, hash), operands with no packed stream (the rank pair),
  and the seed party's empty form (`clock_fork` on the event
  families).
- **Limb**: floors bind where big-integer arithmetic is
  semantically mandatory, at one op per 64 bits of every stored
  magnitude wider than `MACHINE_WORD_MAGNITUDE_BITS` = 128
  (`mandatory_limbs_version`, hand-count-pinned): the rank family
  (`rank`/`distance`/`lag`), the parsing text rows, and both
  decode rows; `rank_pair_ops` floors at one limb write per 64
  content bits of the wider operand (the sum's numerator spans
  it). The display rows are the load-bearing NA: their conversion
  runs inside the bignum dependency, below the limb shim — the
  display canary and the wall leg judge them, and the declaration
  now says so on the board face.
- **Heap**: floors on the codec and text rows (the result
  materializes at least its packed bytes); NA elsewhere
  (allocation is not semantically forced, and the process
  allocator cannot be re-routed around).

The wall leg: `MAX_WALL_SCALING_EXPONENT` = 1.3 judges the wall
exponent fitted across the two scales, only when the larger
scale's wall reaches `MIN_JUDGED_WALL_MILLIS` = 100 (marked `*` on
the board); wall constants stay displayed-never-judged. The
threshold is the calibrated determinism point **[measured** —
2026-07-24, two runs per scale, dev profile**]**: a 50 ms draft
left three bigroot cells' smaller-scale walls (24–35 ms) noise-
dominated and their "wall exponent" reason flickering across
runs; at 100 ms every judged cell reads ~1.9 (the bigroot
quadratics) or ≤ ~1.13 (the linear controls), and both scales'
judged enumerations — verdicts, all counter columns, floors, and
red reasons, wall text excluded — are byte-identical across two
consecutive runs. Today's wall-exponent reds are all cells already
red on their counters (default: `version_distance × bigroot`;
record: the nine bigroot join/meet/sync/recv/distance/lag legs plus
`version_from_str`/`clock_from_str` × hugeleaf): the leg adds no
verdict flip, exactly as a deterministic-first suite should read.

Both §14 tripwires are committed in `meter/board/tests.rs`:
`bypassing_walk_is_green_under_ceilings_alone_and_red_under_floors`
(an index-walk over the stored bits — real linear work, every
counter near zero — green through `evaluate` under all-NA
declarations, red on exactly the scan floor under the committed
walk convention) and
`unmetered_quadratic_reads_red_on_the_wall_exponent_leg_alone`
(plain machine-word arithmetic, walls measured live, red on
exactly the wall exponent with all four counter columns green;
runner-reserved in `.config/nextest.toml` on the display canary's
idiom).

**Vacuity finding (real, predicted by this directive's mechanism):
the id text renderer walks its operand outside the scan meter.**
`write_id` (`codec/display.rs`) reads the stored id bits by direct
slice indexing, so `party_display` records zero scan bits over a
63 KB walk while the event renderer's walk (through the metered
`decode_int`) records ~8 bits per packed byte. The committed scan
floor trips on exactly three cells — `party_display × id-pair`,
`party_display × benign`, `clock_display × id-pair` (its version
half is the empty version; the party half is the unwatched walk) —
and one verdict flips at each scale: `party_display × benign`,
green → red. Per the directive's records rule the floor stands and
the cell is red until the id renderer's walk is metered (or C2's
kernel routing subsumes it); the other two cells were already red
on segments. `party_fork × id-pair` and `clock_fork × id-pair`,
the other anticipated bypass candidates, pass their touch floors:
the split path does read the root codes through metered
primitives (their scan counts are small constants, invisible at
the board's per-byte display precision, above the 2-bit floor).

Board of record after both legs **[measured** — 2026-07-24, dev
profile, limb-meter and scan-meter lit; each scale run twice,
judged enumerations byte-identical as above**]**: **128 green /
72 red at the default scale; 116 green / 84 red at ×4** — the
prior red sets reproduce unchanged at both scales plus exactly
`party_display × benign` at each, the scan-floor flip recorded
above. §17.3's record-scale ownership table is unchanged (the
flip's owner is this finding, P3.8's text column owns the cure
alongside the id renderer's metering).

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

Added 2026-07-23 (P3 planning, benches): the bench suite grows to
full computational coverage of the new representation — every public
operation benchmarked over representative and adversarial shapes,
with before/after deltas reported at each phase that touches the
operation. The suite is reorganized for targeted iteration: one
command runs a single operation's benches (a justfile recipe taking
a filter), and a documented reduced-sampling mode (smaller sample
count / measurement window) gives agents a fast feedback loop —
quick mode for iteration, full sampling for any number quoted as a
result of record.

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

Amended 2026-07-23, P3 accumulator fix round: the roster is now
**fifteen** tests. Gate runs generated two seeds (both committed in
`proptest-regressions/tree/mirror/streaming/remote/proxy/tests/failures.txt`,
`cc 532cbe72…` and `cc 83fded99…`) for
`rumors tree::mirror::streaming::remote::proxy::tests::failures::transport_failures_are_exact_and_fail_fast`.
The witness is new to the roster: not the `Stalled` deadlock but a
fail-fast conformance assertion — a transport failure injected on
one side (the shrunk case: `fail_left = true` during `Flush`) is
not observed as an error on the other (`outcome.right.is_err()`
fails). Provenance verified 2026-07-23 by replay: deterministic in
isolation at this branch's tip with every campaign change stashed,
and at the plain merge-base 6b39482d with the seed file copied in —
main is red on these seeds with no campaign changes applied. The
fifteen-test sweep of record at this branch's tip:
`--no-fail-fast`, 745 run: 730 passed, 15 failed, 2 skipped, the
fourteen stall-roster tests plus this one and no others. Same
streaming-transport genre as the stall roster and the same expected
fix of record (`link-transport` reworks `streaming/remote/proxy`),
but whether that branch cures this witness is **unverified**: these
seeds must be replayed there before the C0 rebase closes the
roster.

Amended 2026-07-24, P3.6 fix round: the roster is now **sixteen**
tests. A gate run at this branch's tip minted one more seed for the
inherited stall — `cc 36aa4c82…` in
`tests/multi_peer.proptest-regressions` — failing
`rumors::multi_peer::readout_matches_oracle_after_quiesce_string`
with the identical `Stalled` witness. Provenance verified 2026-07-24
by replay at the plain merge-base 6b39482d: the test fails there
with the seed file copied in and passes there without it — main's
transport, no campaign changes applied. The sixteen-test sweep of
record at this branch's tip: `--no-fail-fast`, 879 run: 863 passed,
16 failed, 2 skipped, the fifteen recorded tests plus this one and
no others. Process rule, recorded with this amendment: until
`link-transport` lands, the roster is not a stable set — any
sufficiently long proptest run can mint a fresh schedule that walks
into the same wait cycle — so a gate read is judged by provenance,
not by count alone. A novel failure joins the roster (commit the
seed, amend this section) exactly when all three hold: the witness
is the `Stalled` panic, the cause is a new seed line in one of the
stall files (`tests/pairwise.proptest-regressions`,
`tests/multi_peer.proptest-regressions`,
`tests/async_wire.proptest-regressions`), and the test replays red
at the plain merge-base with the seed file copied in. Any novel
failure outside that shape blocks.

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
- Amended 2026-07-23 (Rank audit): this entry originally read
  "`Rank`'s exponent-alignment shifts allocate ~tree-depth bits —
  linear, acceptable; the meter should cover `rank`/`distance`/`lag`
  to keep it that way" — true of the single-op alignments and false
  of the fold that produces them. **The rank fold's plain-bignum
  accumulator is quadratic: V6.** The harmonic spine H(d) — d
  internal nodes `(0, ·, 1)` bottoming in `(0, 0, 1)`, canonical at
  `6d + 2` wire bits — has rank `(2^d − 1)/2^d`, so the fold's
  running numerator crosses every width from 1 to d and each level's
  exponent-alignment shift-and-add walks the full accumulator:
  Θ(d²) limb work from Θ(d) wire bits against a Θ(d)-bit mandatory
  output **[measured** — scratch probe, release, inputs round-tripped
  through strict decode: limb ops 33.8M → 134.7M → 537.9M at
  d = 32Ki/64Ki/128Ki, ratio 3.99 per input doubling; wall
  12.9 → 44.1 → 164 ms; peak heap linear at ~0.5 B per wire
  bit**]**. `distance`/`lag` inherit it through their rank calls;
  `min_ticks` (u64 fold) does not. It is not V3 — H(d) contains no
  wide gammas — and no §2 family can see it (dense keeps the
  numerator at one limb, bigroot's magnitude sits at the root,
  hugeleaf has no depth), so H(d) joins the generators and the rank
  rows join the enforced envelopes. The cure is P3.6's delta-algebra
  kernel with its accumulator requirement made load-bearing: the
  telescoped rank sum runs on the §8.1/P3.2 balanced signed-digit
  form, under today's coding as under Tier 2, with H(d) as the
  regression witness — a plain-bignum telescoped fold re-imports the
  quadratic. Landed 2026-07-24: the packed fold now merges each
  child's numerator into its sibling's accumulator digit-routed at
  the exponent gap (never a materialized shift of the accumulated
  value; a u128 inline arm keeps all-small trees allocation-free),
  and the exponent add is checked — the u32-wrap edge panics loudly.
  RANK_HARMONIC's limb pin moved 134,740,995 → 1,025 **[measured**
  — the enforced row, three identical runs**]**; the board's
  `version_rank × harmonic` limb legs read exponent ~0.99 (its
  remaining red is the recursion-frame segment leg, retired at C2 by
  the skyline kernel). The rank envelope rows gained an
  accumulator-touch column so the fold's arithmetic stays metered
  (the limb column alone would read a vacuous near-zero); the
  bigroot heap and dense segment columns re-pinned at the new
  implementation's measured values (the container accumulator and
  the wider fold frame). The rest of the Rank surface is clean **[measured]**:
  `cmp`/`checked_sub`/`+` materialize Θ(exp) transient bits
  (62.5 KB → 250 KB across exp = 5×10⁵ → 10⁶, doubling exactly) —
  four thousand times the 32-byte operands, but linear in the
  operands' value content `bits(num) + exp`, which every public
  construction path bounds by the producing wire (exp ≤ depth ≤
  wire/4; num bits ≤ exp + max-path-sum bits; addition and
  subtraction never raise exp above their inputs') — the §10.6
  stored-bits-don't-bound-content genre in miniature, priced
  honestly; Rank rows on the board denominate against value content
  accordingly. §10.6's delta algebra covers Rank *production* only,
  and that is enough: a produced rank's content stays wire-bounded
  even under Tier 2 (a sum's width is max magnitude + depth + log
  leaf-count, each term wire-bounded — the comb's Θ(nk) leaf content
  aggregates to a Θ(n + k)-bit rank), so Rank-vs-Rank arithmetic
  needs no Tier 2 mechanism. One edge **[derived]**: `rank_rec`'s
  u32 exponent wraps silently past depth 2^32 (≥ 2 GiB of wire); a
  checked add converts a wrong answer into a panic under the
  no-depth-bound contract.
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

## 17. P3–P5 plan of record (pre-DECIDED draft)

Added 2026-07-23 (P3 planning round). Assembled from the five P3
planning probes — accumulator, radix, grow-iterativization,
display/denomination, migration — each run as an independent
executable probe against the worktree and each returning
**promising-with-conditions**; none negative. This section is the
dependency-ordered path from today's board (96 green / 62 red at
the default scale **[measured** — re-run at this section's writing,
matching the P2 record**]**; the record-scale red set is strictly
larger — §17.3) to all green with strictly-improved performance,
under Tier 2. It is an input to the §12 DECIDED entry, which
remains open and is not written here; nothing below is a decision
record.

### 17.1 Honesty preamble: conditions, and who discharges them

Every probe condition is either discharged by a named work item
below or held as an explicit gate this plan cannot pre-discharge.
The gates first, because the user's GO is conditional on seeing
them plainly:

- **Gate A — skyline join subadditivity [resolved GO 2026-07-23].**
  The `link-transport` window budget cites the pinned lemma
  `encode(a|b).len() ≤ encode(a).len() + encode(b).len()`
  (`window.rs`), proven against today's coding. Under delta coding
  a crossing switch emits a delta of magnitude ~`|hₐ − h_b|` that
  neither input's local codes paid for; a near-tight sketch exists
  (single huge step vs a flat half-height version), so failure is
  plausible. P3.5 probes it adversarially before anything depends
  on it. A refutation is a fully successful probe: the reroute
  (re-derive the lemma statement and the window's `version_bound` —
  the capacity arithmetic consumes greeting-exchanged version sizes
  dynamically, so a constant-factor lemma re-denominates rather
  than breaks) happens before C2, and if no bounded form exists the
  Tier 2 decision itself reopens. Resolution policy (the user's
  ruling, 2026-07-23): the existing bound stays unless the probe
  *falsifies* it — no preemptive replacement. On falsification
  only, the replacement is a different estimate function that is
  both proptested over the adversarial families and analytically
  validated — a written derivation of the new bound, not an
  empirical fit — and the reroute is recorded in this plan as
  exactly that.

  Resolved 2026-07-23 (P3.5): GO. The probe did not falsify — over
  ~1.5M pairs (every §13 generator family and adversarial cross,
  the exhaustive small scope, and a directed hill-climb on the
  excess) the excess `size(a ∨ b) − size(a) − size(b)` never rose
  above −2 — and the near-tight sketch above dissolves under the
  full accounting: the huge step's own boundary code covers the
  switch's delta at that same boundary. Per the resolution policy
  the existing bound stands unchanged; what lands beside it is the
  derivation of record (§10.4's dated addendum): skyline
  `size(a ∨ b) ≤ size(a) + size(b) − 2` bits, meet dual identical,
  every step proven, the −2 structural (tight at the empty pair)
  and matching the probe's maximum exactly. Pinned
  emitter-parameterized in `meter/tier2` beside the 1-Lipschitz
  pins, so P3.6's emission kernel re-instantiates the same pins
  over its actual output.
- **Gate B — the user's §12(b) acceptance and two criterion
  ratifications.** The §10.5 identity/persistence blast radius (the
  `Key` migration story is explicitly out of scope for the swap);
  the P3.3 text-I/O denomination amendment (a contract-statement
  change — harder, not softer, an arithmetic claim P3.3 now carries
  the numbers for rather than asserting: the pinned limb constant
  is one the current schoolbook parser measurably exceeds — but the
  user ratifies it, not this plan); and a recorded blanket
  sign-off for C2's mechanical byte re-pins (prose and figures
  stay under per-item sign-off at P5).
- **Gate C — external timing.** C0 rebases onto main after
  `link-transport` merges (the §14 gate-roster escalation's
  resolution of record). C1 work items are pure additions and can
  proceed before the rebase; the C2 artifact list cannot be
  finalized until after it.

One probe finding amends this document before anything is built on
it: the accumulator representation the §8.1 amendment and §10.6
first named ("big part + machine-word signed offset") is
**refuted** by a canonical input — a wide-tooth comb (teeth ±2^192,
~387 wire bits each, oscillating across a `2^k` cliff, `k ≫ 192`)
forces the two-zone form through its normalized prefix every tooth,
measured 70.5 → 134.5 limbs/delta as `k` doubles — quadratic again
**[measured** — accumulator probe, exact-oracle-checked, release
with overflow checks**]**. The structural reason: *any* two-zone
(normalized + fixed-width window) design has a zone boundary the
input can oscillate across at paid-but-constant cost; widening the
window only moves it. P3.2 records the dated re-amendment and the
replacement representation (landed 2026-07-23: §8.1/§10.6 name the
balanced signed-digit form of record, and `codec::accum` is the
implementation).

Two denomination honesty notes carried from the probes, so green is
never misread: binary↔decimal conversion is inherently superlinear
in wall time (Θ(M(n)·log n)); the text rows' limb *constant* is
judged against the radix-work denominator `R` (P3.3), not against
wall time, and the criterion says so explicitly — and (amended
2026-07-23, plan adversarial review; re-amended the same day, P3
fix round) those cells go green only under both limb legs: the
constant at P3.3's pinned κ, which the current digit-by-digit
parser exceeds ~4×, and the exponent against `n_io`, which any
chunked schoolbook converter exceeds at ~2 **[measured** — the
committed tripwire probe**]** — so green on those cells arrives
with P3.8's converter (an item the same day's text-deprioritization
ruling resequences to the P4 tail), never with re-denomination or
constant-shrinking chunking alone. And the board's
default-scale run under-detects segment amplifiers (the ~1 MiB
growth threshold — the §13 caveat of record); acceptance therefore
runs at a pinned record scale (P3.4) at which the known amplifiers
were demonstrably red — a scale at which the red set is strictly
larger than the default board's: eleven default-green cells read
red at a ×4 witness run, each assigned an owning item in §17.3.

### 17.2 Work items

Each item states: what / where / kills (board cells and enforced
envelope rows) / acceptance / risk → retirement / dependencies.
Kill lists name the item whose *mechanism* retires the cell; every
kill on a version path is *realized* at C2 (P3.9, when public
operations route to the new code) and *verified* at C3 (P3.10) —
with two recorded exceptions: the text cells' realization moves
with their item to the P4 tail (the user's 2026-07-23
deprioritization ruling, recorded at P3.8), and the record-scale
id cells are realized at P4.1.
Riskiest-first within dependency ties: the accumulator and codec
core precede everything built on them.

**P3.1 — C0: rebase and merge-seam re-sweep.**
*What*: rebase `before-hardening` onto main once `link-transport`
merges (resolving the §14 gate-bar escalation; the fourteen-test
stall roster becomes that branch's regression pins). Re-derive the
C2 artifact list from the rebased tree: the worktree-derived list
(55 snapshots, 10 fuzz seeds, 2 witness literals, byte-pinned
doctests, `BOOKMARK_FORMAT_VERSION`) is a floor — `link-transport`
touches ~20 snapshots and adds window/hop-trace/pipelining tests
that may embed wire bytes. *Where*: branch operation; the re-derived
list is recorded in P3.9's landed entry. *Kills*: none.
*Acceptance*: `just gate` fully green at the rebased tip (first time
in the campaign); board re-run reproduces the 96/62 baseline modulo
seam. *Risk*: semantic merge conflicts line-level resolution misses
→ retired by the merge-seam re-sweep practice (mechanical greps,
full board + meter suite at the rebased tip). *Deps*: Gate C.

**P3.2 — the accumulator of record, its generator families, and the
§8.1/§10.6 re-amendment.**
*What*: land the cliff-immune accumulator as the shared arithmetic
core for every running-height/difference sweep and for strict
decode's nonnegativity check: a redundant balanced base-2^32
signed-digit form — `Vec<i64>` digits, value `Σ dᵢ·2^32ⁱ`, lazy
zone `|dᵢ| < 2^33`; a write leaving the zone carries
`c = (t + 2^31) >> 32` and stores the recentered remainder in
`[−2^31, 2^31)`, so a digit needs ~2^33 fresh net drift before
carrying again — amortized O(1) per small delta, O(delta limbs) per
wide delta, no preset attack because every write recenters; `sign()`
folds from the top, decides at running partial `|s| ≥ 3` (top-digit
domination: everything below contributes < 2.01·2^32ⁱ), and
collapses any deeper-scanned cancelling prefix so scans amortize
against the writes that built them; one low-to-high signed-carry
pass converts to normalized limbs. There is no normalized region
anywhere, hence no cliff at any delta width **[measured** — probe:
2.00 limbs/delta flat on the boundary comb as k doubles; 6.00 flat
on the wide-tooth comb where the two-zone form is quadratic; flat
per-coded-bit on the magnitude ramp; wall ~4.4 ns/delta flat on the
comb at k = 4096→32768**]**. The module doc carries the two
load-bearing arguments (the `|s| ≥ 3` domination bound; the
collapse/write amortization). Spec-first: the adversarial generator
families join the meters in this item, before any codec exists —
the wide-tooth comb, the unpaid-crossing fan (§8.1's promised
generator), cancelling-prefix chains, and the deep
alternating-binary spine (P3.7's frame-count adversary). And the
dated re-amendment to §8.1 and §10.6: the two-zone representation
is refuted as recorded in §17.1; this form is the representation of
record for Tier 2's sweeps and for the eventual difference-tracked
compare under any tier. Same item, carved out of the text work by
the 2026-07-23 deprioritization ruling because it is a one-line
dependency floor that fixes `Display`'s complexity class by
itself: raise the `num-bigint` floor to ≥ 0.4.8 in `Cargo.toml`,
not just the lockfile (0.4.7 ships divide-and-conquer
`to_radix_digits`; 0.4.8 fixes its Burnikel–Ziegler regression;
the workspace lock currently resolves 0.4.6, whose `to_string` is
measured quadratic). *Where*: new module (proposed
`src/version/skyline/accum.rs`); generators in `src/meter.rs`;
envelopes in `tests/meter.rs`; `Cargo.toml` (the floor);
amendments in this document.
*Kills*: no cells (foundation for P3.5–P3.8). *Acceptance*:
exact-value differential proptest against a `BigInt` oracle with
sign compared every step and periodic snapshots; limb-meter
envelopes flat per-delta across boundary comb / wide-tooth / fan /
cancelling-prefix at two scales, plus flat per sign read on the
static-prefix read stream (the §8.1 2026-07-23 amendment: the
write-funded streams pass with the collapse deleted, so this stream
is the collapse's enforcing pin), all pinned via the committed
`limb-meter` feature (the probe's hand counters are not the record).
*Risk*: (1) ~2× constants vs the refuted two-level form on
non-adversarial workloads — accepted and re-pinned with the real
meter; (2) `sign()` mutates the representation (reads need `&mut`
discipline) → retired by keeping the type sweep-internal and
documenting the invariant; (3) probe-counter/meter divergence →
retired by re-pinning at landing. *Deps*: none (pure addition; may
precede P3.1).

**P3.3 — board-criterion hardening: output-denominated text I/O.**
*What*: amend §6 and §13 with the denomination criterion of record.
Text I/O rows (`Display`/`FromStr` for Version/Party/Clock) are
judged against total I/O bytes `n_io` — packed input + text output
(Display), text input + packed output (FromStr) — with every
ceiling unchanged: heap exponent ≤ 1.15, ≤ 16 B per I/O byte over
the flat allowance, segments ≤ 1. The limb column on those rows is
judged against the radix-work denominator
`R = n_io + Σ (digitsᵢ × limbsᵢ)` over rendered/parsed values — the
derived cost law of *schoolbook* binary↔decimal conversion — with
the text codec routing all radix arithmetic through one shared,
limb-metered chunked converter so `R` is observed, not assumed,
and with the ceiling PINNED at the divide-and-conquer target: limb
≤ κ = 0.25 per `R` unit, replacing the 128-per-input-byte board
ceiling on these rows (κ is provisional — set 4× under the
measured schoolbook ratio and ~4× over the probe-extrapolated D&C
ratio at the default hugeleaf scale **[derived]** — re-pinned from
the observed meter when P3.8 lands, and verified at record scale
before enforcement). Amended 2026-07-23 (plan adversarial review):
the pinned κ is load-bearing, because a bare `R` re-denomination
under the old ceiling is *softer*, not harder, on four cells.
`version_from_str`/`clock_from_str` × {bigroot, hugeleaf} are red
today only on limb-vs-input, at exactly the schoolbook law:
hugeleaf, limb exponent 1.99 at 503 limb/B × 9633 B ≈ 4.85M limbs
against an `R` term ≈ 9633 digits × 500 limbs ≈ 4.8M, ratio
≈ 1.01; bigroot, exponent 1.92 at 34.6 limb/B × 36 825 B ≈ 1.27M
against ≈ 1.2M, ratio ≈ 1.06 **[measured** — board, re-run this
round**]**. Because `R` *is* the schoolbook cost law,
re-denomination alone flips all four green with zero code change,
and the exponent leg is toothless there by construction
(schoolbook's limb count grows exactly as `R`, so its exponent
against `R` reads a flat ~1.0). The pinned constant is the
discriminator the exponent cannot be: schoolbook's ratio ≈ 1.0
exceeds κ by ~4× at every scale, so the current parser cannot pass
the amended criterion, and these four cells go green only when
P3.8 (at the P4 tail) lands the D&C parse (measured exponent
~1.05–1.1 with a small constant). The harder-not-softer statement
Gate B ratifies is this arithmetic, not an assertion. An
output-honesty assertion (`text_bytes ≤ C · content_bits`) plus the
round-trip and snapshot pins close the pad-the-output door. Why
harder, not softer: the input-only criterion is unsatisfiable by
construction on expansive shapes — on the §10.6 comb a *perfect*
writer emits Θ(nk) mandatory digits from Θ(n + k) wire bits (heap
exponent ~2 with no defect anywhere) — and an unsatisfiable
criterion degenerates into exemption holes; the I/O criterion keeps
every ceiling falsifiable (a real amplifier still trips 16 B per
I/O byte), and today it is the *limb column* that is quietly soft:
`Base`'s `Display` delegates to `BigUint`'s with no `meter_limbs`
call, so `version_display × hugeleaf` records 0.1 limb/B while
doing ~(16000/64)²-scale conversion work — a false green the new
denominator replaces with an observed number **[measured** — board
+ code**]**. The amendment also enumerates the rows that must NOT
re-denominate: both binary codec directions (canonical 1:1;
input-denominated stays the honest bound), all scalar/comparison/
query rows, and the packed-output mutators — whose input
denomination rests on output ≤ C·input, pinned for join/meet by
the 1-Lipschitz boundary argument (|δ_out| ≤ max |δ_in| at each
overlay boundary, boundaries ⊆ union of inputs') as a `meter/tier2`
test rather than an assumption. Finally it resolves the one
output-domination coverage gap the probe sweep found:
`version_project`/`clock_own_version` on comb × scattered-party is
Θ(e·k) output from Θ(e + n + k) input and no board cell builds that
operand cross — add the cell with an explicit denomination
decision, or record why the family set excludes it. *Where*: §6/§13
amendments; `src/meter/board.rs` (denominator plumbing);
`src/meter/tier2` (Lipschitz pin). *Kills*: none directly — it is
the criterion under which P3.8's fourteen cells (ten at the
default scale) are judged; landing it first is the spec-first
practice. *Acceptance*: board renders the
new denominators; the current (recursive, unmetered) text paths
still read red under the new criterion — the four limb-only
`FromStr` cells because schoolbook's ≈ 1.0 limb/`R` exceeds κ, the
rest on their unchanged segment/heap legs — so the hardening flips
nothing by itself except the re-denominated mandatory-output heap
constants it exists to state honestly; a criterion under which the
pre-P3.8 parser reads green anywhere is a bug in this item.
*Risk*: criterion drift
reads as grading-to-pass → retired by Gate B (the user ratifies the
amendment with the GO) and by the ceilings staying numerically
identical. *Deps*: none; must precede P3.8 acceptance.
Amended 2026-07-23 (P3 fix round, the chunked-schoolbook
refutation): "the pinned constant is the discriminator the
exponent cannot be" overclaimed — it holds against the committed
digit-by-digit parser, not against the schoolbook class. A
u32-chunked schoolbook probe through the board's metered ops (9
digits per mul-add pair, still Θ(digits × limbs), quadratic in
value bits) measured 0.112 limb/`R` on hugeleaf(16k) and
hugeleaf(32k) — flat across the doubling — and 0.154 on
bigroot(8k, 2k), all under κ **[measured]**; under the entry as
first written, a chunked-schoolbook P3.8 would have flipped all
four `FromStr` limb cells plus the two benign ones green with no
committed check objecting. The discriminator of record is the
exponent leg: the text rows' limb exponent is judged against
`n_io` — like every other exponent, never against `R`, on which
any schoolbook converter reads a flat ~1 — at the unchanged 1.15.
Chunked schoolbook reads 1.99 there, the digit-by-digit parser
1.92–1.99 (its four cells now red on both limb legs)
**[measured]**, and the D&C converter's recorded limb work sits in
the ~1.05–1.1 band (P3.8's number, verified at record scale before
the κ re-pin); κ keeps the constant leg only. Anti-softening is
two committed tripwires in `meter::board`'s suite, the chunked one
driven through `evaluate` itself: the digit-by-digit parser
exceeds κ; the chunked probe slips under κ and must read red on
exactly the limb exponent. Board re-run at the amendment,
both scales: verdicts identical (96/64 default, 85/75 at ×4).

**P3.4 — board-criterion hardening: the acceptance scale of
record.**
*What*: pin a record-mode board scale (a committed multiplier and a
`just amp-board-record` recipe) at which every known amplifier
class was red under pre-fix code — calibrated against the P0
baseline mechanisms, with the segment meter's ~1 MiB growth
threshold as the sizing driver (P1's id walks read green at board
defaults while red at meter-suite depths; onset-above-default false
greens must not be able to pass acceptance). The ×4 witness run
(§17.3) is the calibration floor: eleven default-green cells read
red there, every one segment-onset; the pinned scale sits at or
above it, the baseline red enumeration is re-witnessed at the
pinned scale, and any cells beyond §17.3's eleven are assigned
owning items by dated amendment. Campaign acceptance
becomes: all green at BOTH the default scale (inner loop) and the
record scale, three identical runs. *Where*: `src/meter/board.rs`,
`examples/amp_board.rs`, `justfile`, §13 amendment. *Kills*: none
(prevents false acceptance). *Acceptance*: the calibration table —
per family, the scale at which each historically-red mechanism
first reads red — recorded in the §13 amendment, together with the
record-scale baseline red enumeration; record-scale
runtime stays within a documented per-family seconds budget (the
comb text cells' mandatory Θ(W²) output wall time bounds the comb's
record size; the enforced record remains `tests/meter.rs`
regardless of board onset). *Risk*: record scale inflates runtime →
retired by keeping record runs acceptance-time only. *Deps*: none;
must precede P3.10's acceptance claim.

**P3.5 — C1a: the skyline codec core and the subadditivity
GO/NO-GO probe.**
*What*: land the skyline encoder, strict validator, and a
transcoder from the current `Version`, module-private beside the
old codec (old codec = behavioral oracle; wire bytes untouched;
gate green). The validator enforces topology minimality at ~2
bits/level (zero-delta right sibling, per §10.2) and leaf-value
nonnegativity on the P3.2 accumulator (the §10.2 amendment's
requirement — a plain big-integer running value is Θ(W²) on the
comb **[measured** — the `meter/tier2` plain-sweep pin**]**).
First inside this item, Gate A's probe: an executable adversarial
test of skyline join subadditivity over the §2 families plus
hugeleaf-vs-step, comb-vs-flat, and `arbitrary` — with the verdict
recorded as a dated entry either way, and a NO-GO rerouting to
lemma/`version_bound` re-derivation before any dependent work
piles on. The boundary comb joins the board's input families
(§13's landed note anticipates this). *Where*: proposed
`src/version/skyline/{codec,validate}.rs`; subadditivity pins in
`version/tests.rs`; board family in `src/meter/board.rs`. *Kills*
(realized at C2): the four decode-V5 cells — `version_decode ×
dense`, `version_decode × bigroot`, `clock_decode × dense`,
`clock_decode × bigroot`. *Acceptance*: transcode round-trip
differential proptests over all generator families including the
comb; strict-reject suites (negative running value, sibling zero
delta, non-canonical codes); decode meter at ~2 bits/level frames
and limb-linear on the comb; the subadditivity verdict. *Risk*:
Gate A (the riskiest seam in the plan — scheduled first for that
reason); a validator bug is a byte-equality bug → retired by the
oracle differential, exhaustive small-scope, and deliberate
snapshot re-pins at C2. *Deps*: P3.2.

Landed 2026-07-23 (the subadditivity lemma of record, retiring
Gate A): verdict GO — §17.1's Gate A entry carries the dated
resolution and probe record, §10.4's dated addendum the full
derivation (`size(a ∨ b) ≤ size(a) + size(b) − 2` bits, meet dual
identical, every step proven, the −2 tight). The pins landed in
`src/meter/tier2/tests.rs` beside the 1-Lipschitz pins rather than
`version/tests.rs` (they are statements over `tier2_size`, whose
suite lives there): join and meet over every §13 event family and
its full adversarial cross grid, the probe's recipe families
rebuilt on the committed generators (hugeleaf-vs-step,
comb-vs-flat, interleaved plateaus, staggered widths, cliff
staircases), the crate-internal deep-shape grid the external probe
could not reach, arbitrary trees, organic histories, and the
equality-case pin that keeps the −2 the strongest constant. The
checks take the join/meet emitters as function parameters — today
instantiated at the packed-form operators — so P3.6's skyline
emission kernel re-instantiates the identical pins over its actual
output.

**P3.6 — C1b: the sweep kernels and the bench baseline.**
*What*: the merge-sweep kernels over the new codec — compare/eq/
concurrent/`causally::contains` (sign-fold over the overlay
partition); join/meet emission on the append-truncate builder,
unified with `IdBuilder` per §11.1 (one generic open/close/
copy-splice/truncate builder over a packed preorder stream,
parameterized by per-node payload); `fill`; and the §10.6 delta
algebra for the linear functionals (`rank`, `distance`, `lag`,
`min_ticks`, `max`, `project`/`own_version`): telescoped
`v₁·W + Σ δⱼ·(suffix weight)` folds, never reconstructing absolute
heights. The emission side-switch algebra (output delta `−D + δₐ`
on a switch to `a`, `D + δ_b` to `b`; ties `D = 0` stay on the
current side) is re-derived for the real *asymmetric* two-cursor
overlay walk — the probe validated it on aligned streams only —
with the alternating-protocol oracle as the behavioral
differential; materialization stays bounded by the switch's own
input+output codes **[measured** — probe: 3.5 limbs/switch flat at
any `2^k` altitude when |D| = 1; 0.126 → 0.125 limbs per
input+output code byte when |D| = 2^(k−1)**]**. Same item, the §14
bench constraint discharged in full: extend `benches/` to full
operation-surface coverage — every public operation over
representative and adversarial shapes, bench IDs mirroring the
board's op names so a board cell names its bench — with a justfile
recipe for targeted single-operation runs (`just bench <target>
<filter>`, criterion filter passthrough) and a documented
reduced-sampling quick mode (criterion `--sample-size` /
`--measurement-time` flags) for agent iteration, full sampling
required for any number of record; before-numbers captured at the
pre-flip tip, so C3 can report required before/after deltas.
*Where*: proposed
`src/version/skyline/sweep.rs` and kernels;
`party/ops/build.rs` unification; `benches/`; `justfile`. *Kills*
(realized at C2), 45 default-scale cells plus 2 record-scale cells
(§17.3):
comparison reads ×8 — `version_cmp × {dense, bigroot}`,
`version_eq × {dense, bigroot}`,
`version_concurrent × {dense, bigroot}`,
`causally_contains × {dense, bigroot}`;
emit paths ×31 — `version_join × {dense, bigroot}`,
`version_join_assign × {dense, bigroot}`,
`version_meet × {dense, bigroot, benign}`,
`version_meet_assign × {dense, bigroot, benign}`,
`version_tick × {dense, bigroot, benign}`,
`version_batch_snapshot × {dense, bigroot, benign}`,
`version_distance × {dense, bigroot, benign}`,
`version_lag × {dense, bigroot}`,
`clock_tick × {dense, bigroot, benign}`,
`clock_join × {dense, bigroot}`, `clock_sync × {dense, bigroot}`,
`clock_recv × {dense, bigroot, benign}`;
projection ×5 — `version_project × id-pair`,
`clock_own_version × {dense, bigroot, id-pair, benign}`;
rank ×1 — `version_rank × dense`;
record scale ×2 — `version_rank × bigroot` (segments 6 at the ×4
witness) and `version_min_ticks × dense` (segments 24): event-walk
recursion frames above the default segment onset, retired by the
same iterative sweeps **[measured** — ×4 witness run**]**.
*Acceptance*: differential proptests against both reference oracles
(recursive tree oracle and function-space oracle) over all families
per the §14 full-surface-measurability constraint; kernel-level
meter envelopes asserted pre-flip; join/meet output-vs-input
Lipschitz pin (P3.3) holding on every family. *Risk*: silent
side-switch bookkeeping error on asymmetric cursors (misreads a
direction rather than panicking) → retired by the oracle
differentials over comb/wide-tooth/fan plus pointwise-max/min
verification at every emitted boundary in the proptest harness.
*Deps*: P3.2, P3.5.

Landed 2026-07-23 (partial: the join/meet emission slice). The
emission sweep (`version/skyline/emit.rs`) rides the comparison
sweep's cursors; the side-switch algebra landed in the post-fold
orientation — output delta `±D′ + δ_old` with `D′` the difference
after the boundary's folds and `δ_old` the departed side's step
delta — verified by the packed-form byte-identity differential over
families/arbitraries/organic/exhaustive-small-scope, a three-cursor
pointwise overlay walk, and the lattice laws on emitted streams.
The unified builder landed in two layers: `codec::build` (the
append/reserve-patch/copy-splice/truncate move set with uniform
write metering) with `IdBuilder` refactored onto it —
behavior-identical under the id oracle suites and the `ID_*`
envelope rows, so no C2 follow-up remains — and
`version/skyline/build.rs` (the leaf-delta instantiation). One
addition beyond the sketch: the builder *holds* the newest leaf's
code out of the stream until the next leaf decides its fate, because
a flushed-then-truncated code would shift left one bit per collapse
level — Θ(depth × code width) on a flat operand dominating a deep
one (join(flat 2^b − 1, dense(d))) — while the held code never
moves; the re-anchor direction's copy is paid by its own deletion.
The truncation coordinates live in per-level bit stacks (the
left-sibling code lengths on a pop-able unary+value bit stack), so
no per-level machine word survives. Envelope rows
(`tests/meter.rs::emit_env`, measured ×1.25, three identical runs):
join × {dense, bigroot, cliff, wide-tooth, absorb} and meet ×
{cliff, wide-tooth}, all four columns, all zero segments; the
subadditivity and 1-Lipschitz pins re-instantiated on the skyline
emitters (`meter/tier2`'s four-entry emitter table), with exact
emitted-length agreement against `tier2_size` asserted inside the
emitter hooks. Still open in this item: `fill` (below), the delta
algebra for the linear functionals, and the §14 bench-surface
constraint.

Amended 2026-07-23 (fill placement): `fill` does not ride the
join/meet machinery pre-flip and lands at C2 with the tick splice.
Reasons, in order of force: (a) its sweep merges a skyline stream
against an *id* stream (2-bit presence tags, `Empty`/`Full`
dominance), which is the Tier 2 topology-cursor pairing P3.7's grow
emit builds on P3.5 — landing a second ad-hoc id cursor here would
be the double migration §17.1 warns against; (b) its collapse is a
different discipline — a fully-owned region's raised value is a
*streaming max over a leaf range*, known only at region close, so
the repair is a region truncation plus one re-emitted leaf, not the
equal-sibling-pair rule the output builder normalizes by; the
truncate-to-recorded-position primitive is shared, the region
bookkeeping is not; (c) fill's only production caller is `tick`
(fill + grow), whose cells the P2 fate map already assigns to the
C2 splice, so a pre-flip fill kernel would move no measurable cell
and gain no third-reference differential it does not already get at
C2 from the alternating oracle. The output builder's `leaf`
interface is the piece fill reuses; nothing in this slice
forecloses it.

Landed 2026-07-24 (the query-fold slice). The linear functionals and
projection landed module-private in `version/skyline/query.rs` on the
comparison sweep's cursors: `rank` integrates the step function at a
pre-scanned maximum depth on a frozen/live height split (live drift on
the accumulator under an 8-digit freeze threshold; the frozen part
contributes once per segment against its signed-compacted dyadic
mass, so the boundary comb costs two frozen-width products for its
one wide borrow and the wide-tooth comb never freezes at all);
`distance`/`lag` are rank differences over the landed join/meet
emitters through the class-first `checked_sub`; `min_ticks` folds
leaf-heights-minus-subtree-minima on a `u64` word stack with an early
exit the moment any height leaves the word range (the tick floor
dominates every leaf height); `project` overlays the skyline against
a packed id via an id-side leaf cursor (1-bit ownership payloads,
synthetic unowned regions for absent children, exhaustion tracked by
the path's left-branch count) and re-emits through the collapsing
builder, materializing the height only at ownership transitions —
priced by the mandatory output on the comb × scattered-party cross
(262,976 output bytes from 2,178 input bytes, heap 1.6× output,
I/O-linear). Differentials: exact agreement with the packed
implementations over all families, arbitraries, organic histories,
and the exhaustive small scope; rank additionally against the tree
fold and the Riemann sum; projection additionally against the oracle
mask. Envelope rows (`tests/meter.rs`, five columns with accumulator
touches and scanned bits, measured ×1.25, three identical runs):
rank × {dense, bigroot, harmonic, cliff, wide-tooth}, min_ticks ×
{dense, cliff-early-exit}, project × comb-scatter. The public rank
fold was cured in the same round (the §15 V6 entry); still open in
this item: the §14 bench-surface constraint (untouched by this
slice) and `fill` at C2 per the recorded amendment.

Amended 2026-07-24 (P3.6 fix round: the freeze quadratic). The
query-fold slice's original freeze discipline — an absolute 8-digit
width threshold on the live component, the frozen part flushed as a
frozen-width product per segment — was refuted by the wide-tooth
comb one notch over that threshold: with the frozen part set wide
once by the first leaf, every 300-bit tooth bought an O(frozen-width)
flush funded only by its own code. Executed **[measured]**: the rank
kernel on `wide_tooth_comb(w = 300)` read 13.5 accumulator touches
per skyline byte at `k = 76,800, n = 128` and 26.8 at the doubling —
per-byte cost doubling per input doubling, ~36× the `w = 192`
control's flat 0.4/B — and no committed family priced the band (the
committed wide-tooth width sat below the threshold as the no-freeze
pin; the boundary comb freezes at most once). Per the ratchet
convention the band landed red first
(`skyline_rank_wide_tooth_freeze_band`: absolute ceilings at the
measured quadratic baseline plus a ×1.8 per-byte growth floor proving
the defect reproduces), and the cure's own commit retired the floor,
re-pinned the band flat (×1.25 across the doubling), and tightened
the ceilings ~16×. The cure, two moves in the kernel: (a) the freeze
trigger is *relative* — a fold that leaves the live component more
than 8 digits wider than its own delta's code freezes — so bounded
oscillation never freezes at any width, and a freeze fires exactly
when stale wide drift is about to ride under cheaper codes; (b) the
frozen part reaches the total by summation by parts,
`Σᵢ F(i)·massᵢ = F_final·2^S − Σ_freezes drift·position`: one
frozen-wide shifted add at stream end plus, per freeze, one
drift-wide product per nonzero signed digit of the compacted freeze
position — each freeze is charged to the drift it evicts (built and
funded by the codes since the last freeze, consumed by the eviction),
and nothing ever multiplies by the frozen width. The freeze path's
own family is new: `meter::jump_comb` (a low tooth, a mid-stream
`2^k`-scale jump, cheap teeth behind it) fires exactly one eviction
and reads flat per byte across a doubling, under ceilings an
un-evicted live component exceeds ~15-fold
(`skyline_rank_jump_eviction_is_flat_per_unit`); `jump_comb` and the
over-threshold wide tooth joined the query differential pool. All
five rank envelope rows re-pinned at the cured mechanism — dense,
harmonic, cliff, and wide-tooth tighten on every moved column;
bigroot's heap and touch cells rose within their standing ceilings
(the frozen component now lives on the accumulator) and keep the
older, tighter pins. Honest residual, stated in the module doc's
cost section: a freeze correction is priced at drift width ×
compacted-position density, so a stream that re-arms wide drift
under cheap codes at a *dense* position (deep alternating topology
around every freeze) is the one shape the funding argument does not
certify; every committed family's freeze positions are ones-runs.
`distance`/`lag` inherit the cure through their rank calls.

Landed 2026-07-24 (the bench-surface slice; the §14 bench constraint
discharged in full — `fill` at C2 is now this item's only open
piece). The suite gained `benches/board.rs`: the full operation
surface as the amplification board's wall-time shadow, one criterion
group per board row and one bench per family, driven by the board's
own cell table (`meter::board::bench_cells`) so bench IDs mirror
board cells by construction — 200 cells at this tip, and a new board
row is a new bench with no second registry to drift. Targeted runs:
`just bench <target> <filter>` (criterion filter passthrough; full
sampling — criterion's 100-sample regime over the committed 500 ms /
2 s windows — is the mode required for any number of record) and
`just bench-quick` (`--sample-size 10 --measurement-time 1`, the
iteration inner loop; the CLI flags keep precedence over the
committed windows). Each measured body is the board row's own
prepare/body pair, operands decoded fresh in untimed setup, so
destructive operations time the operation alone; the rank pair moved
to family construction so the `rank_pair_ops` prepare stays cheap
under per-iteration rebuilds (behavior-identical for the board:
prepare was already outside measurement). Pre-flip bench baseline of
record **[measured** — full sampling, one run at this tip, mean
estimates; `clock_recv` and `clock_own_version` re-run on a quiet
machine after a build overlapped their window**]**, the table C3's
before/after deltas are judged against, covering this item's mandated
op list (compare/join/meet/tick/rank/distance/lag/min_ticks/project/
decode/encode × representative + adversarial). The party, text, hash,
fork, and rank-pair rows this table omits ran in the same 200-cell
sweep and rerun with `just bench board`; the text rows' deltas land
with the P4-tail P3.8, the party rows' with P4.1. Wall time is
displayed, never judged — the deterministic record stays with the
board and the envelope suite:

| op | dense | bigroot | hugeleaf | cliff | harmonic | id-pair | scatter | comb-scatter | benign |
|---|---|---|---|---|---|---|---|---|---|
| `version_decode` | 81.9 µs | 35.1 µs | 25.3 µs | 33.4 µs | — | — | — | — | 2.25 µs |
| `version_encode` | 181 ns | 119 ns | 123 ns | 156 ns | — | — | — | — | 61.9 ns |
| `version_cmp` | 245 µs | 693 µs | 54.8 µs | 53.8 µs | — | — | — | — | 5.36 µs |
| `version_eq` | 609 µs | 2.03 ms | 175 µs | 79.3 µs | — | — | — | — | 8.34 µs |
| `version_concurrent` | 349 µs | 927 µs | 72.1 µs | 68.3 µs | — | — | — | — | 5.77 µs |
| `causally_contains` | 558 µs | 1.72 ms | 123 µs | 110 µs | — | — | — | — | 8.97 µs |
| `version_join` | 418 µs | 2.23 ms | 97.9 µs | 99.7 µs | — | — | — | — | 10.0 µs |
| `version_join_assign` | 377 µs | 2.36 ms | 95.5 µs | 96.7 µs | — | — | — | — | 9.84 µs |
| `version_meet` | 447 µs | 2.12 ms | 97.5 µs | 133 µs | — | — | — | — | 12.6 µs |
| `version_meet_assign` | 445 µs | 3.93 ms | 100 µs | 132 µs | — | — | — | — | 12.5 µs |
| `version_tick` | 233 µs | 366 µs | 66.1 µs | 47.1 µs | — | — | — | — | 5.13 µs |
| `version_tick_adv_party` | — | — | — | — | — | 705 µs | — | — | 2.67 µs |
| `version_batch_snapshot` | 233 µs | 380 µs | 106 µs | 48.0 µs | — | — | — | — | 5.13 µs |
| `version_rank` | 164 µs | 55.0 µs | 26.8 µs | 45.2 µs | 215 µs | — | — | — | 3.68 µs |
| `version_distance` | 1.27 ms | 4.48 ms | 247 µs | 276 µs | 1.09 ms | — | — | — | 26.3 µs |
| `version_lag` | 551 µs | 2.22 ms | 149 µs | 171 µs | 616 µs | — | — | — | 14.7 µs |
| `version_min_ticks` | 131 µs | 45.7 µs | 24.5 µs | 32.7 µs | 145 µs | — | — | — | 2.83 µs |
| `version_join_all` | — | — | — | — | — | — | 1.73 ms | — | — |
| `version_project` | 10.5 µs | 37.0 µs | 67.1 µs | 93.1 µs | — | 559 µs | — | 299 µs | 1.39 µs |
| `clock_decode` | 304 µs | 132 µs | 86.7 µs | 136 µs | — | 123 µs | — | — | 7.59 µs |
| `clock_encode` | 375 ns | 402 ns | 442 ns | 701 ns | — | 336 ns | — | — | 143 ns |
| `clock_tick` | 409 µs | 658 µs | 104 µs | 76.6 µs | — | 1.17 ms | — | — | 10.7 µs |
| `clock_join` | 1.43 ms | 8.15 ms | 308 µs | 329 µs | — | 864 µs | — | — | 53.8 µs |
| `clock_sync` | 1.45 ms | 6.62 ms | 196 µs | 212 µs | — | 605 µs | — | — | 35.9 µs |
| `clock_recv` | 1.18 ms | 4.24 ms | 125 µs | 133 µs | — | 2.50 ms | — | — | 29.9 µs |
| `clock_own_version` | 382 µs | 158 µs | 111 µs | 139 µs | — | 638 µs | — | 463 µs | 11.8 µs |

**P3.6b — the full-surface dual-oracle coverage audit.**
*What* (added 2026-07-23, discharging the §14 full-surface
measurability constraint as a work item rather than a hope): every
public operation — the §13 board's op enumeration is the checklist
— proptested against BOTH reference oracles, the recursive tree
oracle and the semantic (function-space) oracle; and the sweep
kernels additionally differential-tested against the OLD
implementation as a third reference for as long as it exists (C1
only — C2 deletes it, and coverage re-anchors on `oracle.rs`). The
audit itself is a recorded enumeration: for each public operation,
which oracles cover it and which board row or enforced
`tests/meter.rs` envelope pins its resources; any operation
lacking either reference or a pin is a coverage finding, fixed in
the phase that finds it per the §14 constraint. *Where*:
`src/testing/` proptest suites; the audit enumeration in this
document's landed entry. *Kills*: none (coverage, not mechanism).
*Acceptance*: the audit's gap list is empty — and stays empty:
re-running it with an empty result is a P5.5 acceptance clause.
*Deps*: P3.6 (the kernels exist to be tested); the first full pass
runs before C2, while the third-reference differentials are still
possible.

Landed 2026-07-24 (the audit of record: the first full pass, pre-C2,
third references live). Method, re-runnable as stated: walk the
board's op enumeration — the §17.9 census's computational surface;
everything else on the 335-entry census is on the board module doc's
not-applicable list with a mechanism-based reason (the census's three
prose omissions and `meet_all`'s bounded-accumulator reason are in
that list; its exhaustiveness claim is repaired) — and for each row
name the suite carrying each of the three legs: the recursive tree
oracle (reached through `testing::bridge`'s structural lowering,
never the byte codec), the function-space oracle
(`testing::semantic_oracle`, the paper's §4 construction as boxed
closures), and the resource pin (board row and/or enforced
`tests/meter.rs` envelope). Representation pins (§17.6) are the
snapshot suite (`testing::snapshots`: display/bits/bytes blocks per
type, the Rank blocks, error strings) plus the serde/borsh mirror
suites (byte-form ≡ canonical encoding, strict-reject legs, the
typed-bytes CBOR path). The kernels' third reference — agreement
with the packed implementation over families, arbitraries, organic
histories, and the exhaustive small scope — is per-kernel
(`skyline/{sweep,emit,query,grow}/tests.rs`) and dies at C2 by
design, when coverage re-anchors on `oracle.rs`. The enumeration:

- *Comparisons* (`version_cmp`/`version_eq`/`version_concurrent`,
  `causally_contains`): tree leg `compare_matches_oracle`,
  `compare_matrix_matches_oracle` (every borrow shape),
  `causal_cmp_arbitrary`, the exhaustive event pair matrix, and the
  `causally` suite (nine range/bound tests on the same walk);
  function leg `replay_matches_across_references` (the three-way
  comparison-descriptor agreement over op-trace populations) with
  `order_is_a_partial_order`, plus the impl-direct order laws;
  pins: board rows × {dense, bigroot, hugeleaf, cliff, benign},
  `cmp_{dense,bigroot,cliff}` envelopes, `skyline_cmp_*` rows;
  third ref: the sweep differentials incl. the four-outcome matrix.
- *Join/meet emitters* (`version_join`/`_assign`, `version_meet`/
  `_assign`, the batch operator matrix riding them): tree leg
  `merge/meet_matches_oracle`, the join/meet/assign matrices,
  `materialized_{join,meet,batch}_parity`, `merge/meet_arbitrary`,
  exhaustive pairs; function leg `join_is_the_lub`,
  `meet_is_the_glb`, `meet_realizes_pointwise_min`, the replay
  trace (joins via receive), plus the impl-direct lattice laws;
  pins: board rows, `join_{dense,bigroot,hugeleaf,cliff}` and
  `skyline_{join,meet}_*` envelopes, the tier2 1-Lipschitz and
  subadditivity pins on both emitter tables; third ref: the emit
  byte-identity differentials.
- *Grow* (`version_tick`/`version_tick_adv_party`/
  `version_batch_snapshot`, `clock_tick`/`clock_recv`): tree leg
  `tick_matches_oracle`, `tick_arbitrary`, `monotone_tick`, the
  exhaustive tick cross (events × owning ids), and the brute-force
  minimal-inflation reference (`grow_matches_brute_force`,
  `grow_minimal`, and directly on the kernel); function leg the
  replay trace's random-inflation policy invariance plus
  `event_dominates_local_and_advances`; pins: board rows (the three
  id-pair cells red, owner P3.7's realized-at-C2 recording),
  `tick_dense` and `grow_env` envelopes; third ref: packed-grow
  byte identity plus the `Route`/root-cost probe contract.
- *Linear functionals* (`version_rank`, `version_distance`,
  `version_lag`, `version_min_ticks`, `rank_pair_ops`): tree leg
  `rank_matches_oracle`, `min_ticks_matches_oracle`,
  `min_ticks_floors_every_history`, rank monotonicity ×3; function
  leg `rank_realizes_riemann_sum`, `min_ticks_realizes_base_sum`,
  and `distance_and_lag_realize_both_oracles` (impl against both
  references directly); Rank self-laws (monoid/order,
  `rank_sum_equals_the_pairwise_fold`, the 25k-pair msb_cmp
  alignment-oracle sweep, cross-path normalization/Hash witness);
  pins: board rows incl. the harmonic column and the rank pair row,
  `rank_*`/`rank_pair_mismatch`/`rank_sum_mixed` and
  `skyline_{rank,min_ticks}_*` envelopes; third ref: the query-fold
  differentials (rank additionally vs the tree fold and Riemann sum).
- *Projection* (`version_project`, `clock_own_version`): tree leg
  `div_matches_oracle`, `own_version_matches_oracle`,
  `div_by_party_laws`, `div_is_additive_over_fork`; function leg
  `quotient_realizes_region_mask`; pins: board rows incl. both
  I/O-denominated comb-scatter cells,
  `skyline_project_comb_scatter`; third ref: the projection
  differentials plus the oracle mask.
- *Folds* (`version_join_all`/`Sum`/`FromIterator`,
  `party_join_all`, `Clock::join_all`): the reduction is joins by
  construction, so the oracle legs are the join legs plus
  `join_all_{equals,matches}_the_sequential_fold` in both orders
  with the aliased-duplicate pins and the party fold laws
  (reunites/empty/best-effort); pins: the two scatter board cells
  and `fold_{version,party}_scatter_envelope` — the cells'
  marginal reds stay owned by §17.9's C2-adjacent n-cursor merge.
- *Party algebra* (`party_fork`, `party_join`, `party_covers`,
  `party_disjoint`, `party_without`): tree leg
  `{split,sum,covers,disjoint,without}_arbitrary`, the
  fork/join round-trips, overlap hand-back, `covers_tracks_fork_join`,
  `without_inverts_fork`, the exhaustive id pair/split matrices, and
  the `Forks` laws (`forks_matches_from_array`,
  `forks_partial_drop_folds_back` — the consuming array splits ride
  these); function leg `fork_partitions`, `sum_of_disjoint_is_union`,
  `covers_realizes_containment`, `without_realizes_region_difference`,
  and the replay trace's three-way disjointness descriptor; pins:
  board rows and the `id_{join,covers,disjoint,without}` envelopes
  (segment reds owned by P4.1).
- *Clock composites* (`clock_fork`, `clock_join`, `clock_sync`):
  `master_differential` (structural three-type agreement over
  op-trace populations), `clock_observers_match_oracle`,
  `clock_assign_join_matches_oracle`, `sync`,
  `heterogeneous_joins`, `fork_preserves_version`,
  `own_receive_is_tick`, `batch_equals_value_level`,
  `ops_preserve_normal_form`; function leg: the replay trace runs
  every one of these through the `FunctionClock`; pins: board rows.
- *Representation surfaces* (`encode`/`decode`, `Display`/
  `FromStr`, `Hash`, serde/borsh, for all three types): the
  function-space oracle is definitionally inapplicable — bytes and
  text are representations of the tree, and the §4 function space
  has no encoding to compare — so the second independent reference
  here is the strict decoder/validator (reject corpora, bit-flip
  and truncation probes, byte-uniqueness), the snapshot-pinned
  canonical and text forms, and roundtrips anchored on
  oracle-built values (`decode_encode_arbitrary`,
  `as_bytes_matches_encode*`, `display_fromstr_roundtrip`,
  `display_matches_paper_notation`, the serde/borsh mirror
  suites); `Hash` rides normal form (derived over canonical bits;
  `ops_preserve_normal_form`, `as_bytes_matches_encode*`, and the
  codec byte-uniqueness pins are what make byte-hash ≡ causal-Eq
  sound, with the Rank cross-path witness covering the one
  manual-Eq pairing). Pins: board codec/text/hash rows (text reds
  owned by P3.8) and the decode envelopes.

**The gap list is empty.** No public operation lacks a reference
leg or a resource pin. The recorded qualifications, each already
owned: the representation-surface function-space exception above
(mechanism-based, not an owner-less hole); the two marginal scatter
cells (owner: §17.9's C2-adjacent follow-up); the red board cells
enumerated in §17.3's kill lists (instruments, not coverage gaps —
each names its curing item); and the third-reference differentials'
scheduled death at C2 (coverage re-anchors on `oracle.rs`, per this
item's *What*). Re-run of record for P5.5: repeat this walk over
the board's op list and the board-doc NA list against the then-
current test tree; any row that cannot name all its legs is a
finding.

**P3.7 — C1c: grow — iterative bit-packed probe + splice emit.**
*What*: replace `ProbeWalk::rec` with one loop over the two cursors
plus a bit-coded explicit frame stack — per frame: kind 2b, phase
1b, gamma(key delta vs the nearest same-regime ancestor, two
last-key registers restored on pop), and on the phase flip
{1 infeasible bit, gamma(expansions), gamma(depth)} for the saved
left cost — as two parallel stacks (fixed-width control bits +
value bits, the landed complement rewrite's pattern). The synthetic
readers are regime, not state (Expand ⇒ event virtual-zero below;
FullEvNode ⇒ id full below); the Expand arm degenerates to an
id-only descent with a per-level value stack — a bare
pending-children counter under-specifies, the T0.3 genre. Written
against the Tier 2 topology cursor (inside P3, avoiding a double
migration). `Route` is unchanged (position-keyed random-access bit
vector; write order irrelevant); the cost fold replays exact unwind
order so tie-breaking is preserved bit-for-bit. Emission: §11.3
splice — off-path subtrees copied as verbatim bit ranges, path
nodes re-emitted, one boundary-delta repair per splice edge on the
accumulator. Bit-packing is load-bearing, not polish: fixed
16-byte `Vec` frames cost ~32 B per input byte on the alternating
binary adversary, over the 16 B/B ceiling; bit-coded frames are
~0.5–3 b/b **[derived** — probe frame accounting; the adversary
generator lands at P3.2**]**. The probe is topology-only (reads no
leaf bases) — zero §10.6 exposure. *Where*:
`src/version/event/grow.rs` (~150–250 lines replacing the walk);
splice emit beside the P3.6 builder. *Kills* (realized at C2; probe
conversion alone leaves them red because today's emit recurses the
full chosen path — never claimed early): the three grow cells —
`version_tick_adv_party × id-pair`, `clock_tick × id-pair`,
`clock_recv × id-pair` (segments retired by the iterative probe,
heap by the splice emit — the P2 fate map's component split).
*Acceptance*: `Route` bit-vector equality against a reference
recursive probe on spine and alternating families; the existing
grow-optimality proptests against brute force; the deep-spine meter
pin (≤ ~1 B stack per input byte); board cells at C3. *Risk*:
tie-break/coordinate drift (silent misdirection) → retired by the
bit-for-bit `Route` assertion during development. *Deps*: P3.5
(topology cursor), P3.6 (builder for the emit), P3.2 (boundary
repair arithmetic).

Landed 2026-07-23, module-private (`version/skyline/grow.rs`; wire
bytes untouched, the packed grow stays the production path and the
byte-level oracle, exported crate-internally and as
`meter::packed_grow` for the differential and envelope asserts — a
C2 deletion alongside the old path). The probe landed as sketched:
one loop, two parallel bit stacks — 3 control bits per frame (kind,
phase flipped in place), and a pop-able unary+value integer stack
(reverse-readable, the same width class as gamma) holding each
frame's key delta against the two per-regime last-key registers plus
the deferred left cost behind one infeasibility flag — with id-child
presence re-read O(1) at the frame key, the Expand arm running as
the id-only iterative scan inside the same loop, `Route`'s
position-keyed bit vector unchanged (event keys are skyline bit
positions), and the probe topology-only (payload codes skipped by
width: the alt-spine envelope row's whole limb column is 8). The
emit landed as the §11.3 splice through the P3.6 builder: a
`continue_verbatim` move splices a canonical subtree's remainder in
one copy around the held-leaf discipline (interior collapse records
are placeholders — canonicity rules out any merge they could
suppress, pinned by per-leaf-equivalence builder tests); exactly two
payload codes are re-derived per grow (the grown leaf's `+1`, the
successor's `−1` when the grown leaf is its preorder predecessor);
expansion chains code fresh `0`/`±1` sibling deltas directly; the
inflation-point collapse (both directions) rides the builder's
absorb/re-anchor cascade with no new machinery. Differentials, all
green: byte identity against the packed grow over family × party
grids, arbitraries, organic histories, and the exhaustive small
scope (events at depth 2 × owning ids at depth 3); the `Route`
bit-vector and root-cost equality against a reference recursive
probe on every one of those pairs (the named tie-break/coordinate
risk, retired); and the brute-force right-favoring minimal
inflation held *directly* on the kernel — stronger than the
planned transitive claim through the packed impl. Envelope rows
(`tests/meter.rs::grow_env`, four columns, measured ×1.25, three
identical runs): `skyline_grow_alt_spine` (125k alternating spine
under the seed — the frame-count adversary: heap 276,252 B on a
46,877 B input, ~5.9 B/B total transient including the route and
the output stream, against ~32 B/B for one 16-byte machine frame
per level), `skyline_grow_probe_alt_spine` (the probe alone, route
pre-allocated outside the measurement through the meter-only
`Probe` handle: heap 229,376 B, limb column 0, scan exactly 1.0×
input bits — the deep-spine stack pin, landing at ~3.3 live stack
bits per input bit / 4.9 B/B with allocator growth; a dated
correction to this item's derived 0.5–3 b/b frame accounting and
its ≤ ~1 B/B acceptance shorthand, whose low end assumed keyless
frames — the stored key deltas plus the deferred costs price ~10
bits against the adversary's ~3-bit levels, and the load-bearing
16 B/B ceiling is cleared better than 3×), `skyline_grow_id_spine`
(250k unary id over the empty version: the Expand scan plus
mandatory chain output), `skyline_grow_cross` (both spines), all
four at zero grown segments. Kills unchanged: the three grow cells stay red until C2
routes `tick` here, per this item's realized-at-C2 recording;
`fill` stays at C2 with the tick splice per P3.6's 2026-07-23
amendment (grow rides alone pre-flip; at C2 `tick` = fill's sweep
else this emit).

**P3.8 — text: sweep writers/parsers and the shared radix core
(resequenced to the P4 tail).**
Resequenced 2026-07-23 by the user's ruling: `Display`/`FromStr`
are debugging surfaces, not interoperation — prioritized below
everything else, though hardening them remains wanted. The item
moves from C1 to the tail of P4 (after P4.2), stays inside P5.5's
all-green acceptance (the campaign still ends all green; text is
last in line), and leaves the DECIDED entry's critical path
entirely — C2 flips text as a correctness-only port (P3.9) and
this item hardens it afterward.
*What*, three parts. (a) **Display**: two-pass iterative writer.
Pass 1 (finalize): forward sweep with a ~2 b/level topology stack,
the running height in the P3.2 accumulator, a pending-min spine
stack holding `m(closed left child)` per open ancestor — stored as
small offsets against the stack neighbor/anchor, spilling to big
only when a comparably-wide input delta paid for it (a naive
Base-per-level stack is amplifier V1 resurrected: d×B transient on
bigroot-like shapes, and O(width) min-compares) — and a side store
receiving printed bases finalized at each internal close
(`b(child) = m(child) − m(node)`, parent-close information, §9.2's
insight reused). Pass 2 (emit): iterative preorder re-read
consuming the side store (indexed digit arena ≤ C·output). One
preorder text emitter parameterized by payload serves
Version/Party/Clock (party is the degenerate no-value
instantiation; clock is composition). PRECONDITION, probe-first:
an executable pin of the offset-coded pending-min stack's
O(input + depth) coded-size bound — a new Euler-charging-genre
derivation — over the §2 families + comb + `arbitrary`, before any
of this item's writer work builds on it (the resequencing takes
the bound off the DECIDED entry's critical path — the flag day no
longer waits on any text design — so the probe runs first *within*
this item, not earlier in the campaign). Its fallback is named
now: if the bound fails, printed bases come from a two-pass
derivation — a bounded second forward sweep recomputes each node's
pending min at its close, in place of any spine stack holding it —
costing one extra pass over the input, retaining only the
~2 b/level topology stack, ceilings unchanged. (b) **FromStr**:
iterative parse;
per-level retained bases (≤ input by definition); running path sum
add-on-descend / subtract-on-ascend on the accumulator (each base
charged ≤ 2×); per-leaf delta accumulator; canonicality =
O(1) zero-tests. (c) **The shared radix core**: one limb-metered
chunked converter (19 digits per u64 chunk) through which both
directions route — making `R` observed and cutting the quadratic
constant ~19× — plus the divide-and-conquer parse (right-aligned
power-of-two digit blocks, squared-power-of-10 ladder, 64-digit
leaf threshold; recursion depth log₂(digits), no `descend!`)
replacing the schoolbook `parse_base` **[measured** — radix probe:
doubling ratio ~2.7 (exponent ~1.44), exact against the oracle at
every scale, 48× faster than schoolbook at 158k digits**]** (the
`num-bigint` ≥ 0.4.8 floor is carved out to P3.2 — a free
dependency line that fixes `Display`'s complexity class well ahead
of this item) — with a limb recording in the big `Display` arm
(or a wall-ratio canary) because the board is structurally blind to a Display-class
wall regression. The derived board-band exponent for the D&C parse
(~1.05–1.1 against the recording scheme) is verified through the
actual limb meter at record scale before pinning — the margin to
1.15 is real but not huge and the log factor creeps with scale.
*Where*: `src/codec/{display,text,base}.rs` reworked into the
skyline text module. *Kills* (realized when this item lands — the
recorded exception to the realized-at-C2 convention, since the
item now follows C2 — judged under P3.3's criterion), 10
default-scale cells: `version_display × dense`,
`clock_display × dense`, `party_display × id-pair`,
`clock_display × id-pair`, `version_from_str × {dense, bigroot,
hugeleaf}`, `clock_from_str × {dense, bigroot, hugeleaf}`; plus 4
record-scale cells (§17.3, all segment-onset at the ×4 witness):
`version_display × bigroot` and `clock_display × bigroot` (writer
recursion, segments 14) and `party_from_str × id-pair` and
`clock_from_str × id-pair` (parser recursion, segments 48) —
retired by the same iterative writer and parser **[measured** — ×4
witness run**]**.
*Acceptance*: round-trip pins (parse∘display = id, display∘parse =
canonical text, strict `NotCanonical` rejection); the
output-honesty assertion; the pending-min probe pin; segments 0 on
all fourteen cells at both scales; heap ≤ ceiling per I/O byte;
limb ≤ κ per `R` unit (P3.3's pinned constant) with limb exponent
≤ 1.15 against `n_io` (P3.3's exponent leg — the subquadraticity
discriminator, per its 2026-07-23 fix-round amendment) at record
scale. *Risk*: (1) the pending-min bound is
unproven → retired by its probe, scheduled first within the item;
(2) bigroot `FromStr` may carry a second superlinear path
(path-sum accumulation of big bases, the V3 read-quadratic genre) —
the accumulator-based path sum is designed to absorb it; any
residue after the swap belongs to the sweeps and is re-measured
end-to-end here; (3) leading-zero runs / threshold crossings in the
D&C parse → edge and prop tests at productization. *Deps*: P3.2,
P3.3, and P3.9 (it hardens the post-flip skyline text module);
scheduled after P4.2, and nothing waits on it except P5's
closeout.

**P3.9 — C2: the flag day (one commit).**
*What*: flip `Version`'s storage to skyline bits; route every
algorithm to the P3.5–P3.7 implementations; serde/borsh/encode/
decode emit skyline bytes; `Display`/`FromStr` swap to a
correctness-only port over the skyline form — the simplest
`descend!`-routed walk, behavior-preserving on the text bytes; its
board cells stay red, owned by the P4-tail P3.8 (the 2026-07-23
ruling); and in the
same commit delete `WorkingVersion`, the event `Builder`,
`EvReader`'s form split and `Zero` broadcast, the deferred leaf,
and the old event codec — deferred deletion is impossible
gate-clean because the old algorithms read the old packed format
out of `Version`'s own bits and cannot run after the flip.
Differential coverage re-anchors on `oracle.rs` (untouched; speaks
semantics, not bytes). The same commit re-pins the full artifact
list of record (re-derived at P3.1; the worktree floor, verified by
the migration probe): the codec witness byte literals
(`codec/tests.rs`), every byte-pinned doctest (`encoded_bits`,
`as_bytes == encode`), the 10 fuzz seeds regenerated by a
*committed* seed-writer plus a canonicity check on the seed
directory (stale seeds fail no gate — they silently degrade the
corpus), all workspace snapshots (55 at the worktree baseline: 20
gossip/bootstrap/retire, 28 alternating, 5 streaming-codec, 2
bookmark; plus whatever the rebase adds) bulk re-accepted with
`cargo-insta` and each diff reviewed as bytes-only with structure
intact, and `BOOKMARK_FORMAT_VERSION` 1 → 2. Proptest-regressions
files stay untouched (RNG seeds, not bytes; committed per policy).
The public API is unchanged — no public item leaks the
representation; `rumors` compiles with zero source changes (its
`as_bytes` uses are content hashing, arbitrary-total-order
tiebreaks, and one frame length) **[derived** — migration probe,
verified against every import and call site**]**. Blast radius:
~3.6k `before` source lines in scope, of which C1 pre-lands the
novel logic, so C2 itself is rewiring + deletion + ~70 mechanical
artifact files. *Where*: as enumerated. *Kills*: none of its own —
it realizes P3.5 (4) + P3.6 (47) + P3.7 (3) = 54 of the
record-scale red set, 52 of the 62 default-scale reds; the text
cells (P3.8's ten default-scale, fourteen at record scale) are
realized when the P4-tail item lands, and P4.1's five at P4.1.
*Acceptance*: `just gate` green at the single commit; a dedicated
review pass confirming every snapshot diff is bytes-only;
oracle/small-scope/algebraic suites green; `cargo-insta`/`insta`
version-skew compatibility verified before relying on the bulk
accept. *Risk*: one large commit → retired by maximal C1
front-loading and the review pass; Gate B's sign-offs are recorded
*before* this commit (the DECIDED entry precedes C2 by
definition). *Deps*: P3.1–P3.7 (P3.8 deferred to the P4 tail),
Gate A GO, Gate B, the user's DECIDED entry, and §13's
surface-judgment legs (uniform liveness floors and the judged
wall-time exponent, the 2026-07-24 ruling) green on the pre-flip
board — the flip is the moment every operation walks out from
under its old sensors at once, so the floors must be watching
before it lands.

**P3.10 — C3: board and envelope re-pin; bench deltas.**
*What*: board re-run at default and record scale — the 54
C2-realized cells green at both scales (52 of the 62 default-scale
reds), the remaining reds enumerated as expected with named
owners: the text cells (ten at default, fourteen at record scale;
the P4-tail P3.8) and the five record-scale id cells (P4.1) — and
every event-side envelope in
`tests/meter.rs` tightened downward in this commit to the
sweep-earned constants (§14's rule: thresholds tighten in the
commit that earns them). Bench after-numbers against P3.6's
baselines, reported as the before/after table of record;
improvement expected on every touched operation, any regression a
finding. *Where*: `tests/meter.rs`, `src/meter/board.rs`
thresholds, bench report in this document's landed entry. *Kills*
(as enforced envelope rows, mechanism earned at P3.5/P3.6):
`DECODE_DENSE`, `CMP_DENSE`, `JOIN_DENSE`, `TICK_DENSE`,
`DECODE_BIGROOT`, `CMP_BIGROOT`, `JOIN_BIGROOT`,
`DECODE_HUGELEAF`, `JOIN_HUGELEAF`, `DECODE_CLIFF`, `CMP_CLIFF`,
`JOIN_CLIFF` — twelve rows re-pinned at sweep constants (segments
0, heap ~1× packed + bit stacks, limb amortized-linear).
*Acceptance*: three identical runs at both scales. *Risk*: a cell
green at default but red at record scale → that is P3.4 working as
designed; the cell's owning item reopens. *Deps*: P3.9, P3.4.

**P4.1 — id-side explicit compact stacks (§8.3).**
*What*: convert `sum`, `covers`, `is_disjoint` to explicit
iteration (cursor positions + 2–3 bits/frame; `complement` is
already iterative from T0.3). The id side is untouched by Tier 2;
this is the remaining P1 exposure. *Where*:
`party/ops/{sum,compare}.rs`. *Kills* — five record-scale board
cells (the board reads green at default depths per the §13
segment-onset caveat and red at record scale, all on segments at
the ×4 witness): `party_join`/`party_covers`/`party_disjoint` ×
id-pair (segments 60/24/48 at ×4; 202/85/170 at the meter suite's
d = 250k scenarios, which remain the record) and
`clock_join`/`clock_sync` × id-pair (segments 60 each at ×4 —
both route the id side through the same recursive `sum` walk)
**[measured** — ×4 witness run; envelope numbers from
`tests/meter.rs`**]** — plus the enforced envelope rows:
`ID_JOIN` (segments 202 → 0), `ID_COVERS` (85 → 0), `ID_DISJOINT`
(170 → 0). *Acceptance*: `tests/meter.rs` re-pins segments to 0;
the five board cells green at the record scale; the party
differential and algebraic suites unchanged. *Risk*: low
(mechanical; id frames carry no values) → existing proptests.
*Deps*: none hard (scheduled P4 to keep P3 single-purpose).

**P4.2 — residual recursion and word-scale scanning.**
*What*: audit every remaining `recurse::descend!` site after C2
(expected survivors: test-only walks, the C2 correctness-port text
walks — owned by the P4-tail P3.8, not converted here — and the
documented iterative exceptions, which stay); convert the
remaining survivors per §8.3; apply §11.4's word-at-a-time subtree skip
(popcount pending-counter delta with mid-word zero-crossing exit)
to `idbits` and the Tier 2 topology stream where `benches/` says it
pays. *Where*: `src/recurse.rs` call sites; `src/idbits.rs`.
*Kills*: none (constants only). *Acceptance*: the audit list
recorded; benches. *Deps*: P3.9, P4.1.

**P5.1 — envelope finalization at record scale.**
*What*: tighten every `tests/meter.rs` envelope — including
`ID_WITHOUT` (the one enforced row no earlier item re-pins; its
segments went to 0 at T0.3, its heap constant finalizes here) and
the P3.2-born families — and every board ceiling to final
constants, at the record scale, three identical runs. *Kills*
(enforced envelope row): `ID_WITHOUT` final ratchet. *Deps*: P4.*
including the P4-tail P3.8.

**P5.2 — proportional fuzz cap.**
*What*: the §13 fuzz item — counting-allocator harness with a hard
ceiling proportional to input size across all fuzz targets, turning
any future amplifier into a crash finding; the P3.9 seed-writer and
seed-canonicity check become part of `just all`. *Deps*: P3.9.

**P5.3 — stacker-removal audit.**
*What*: if P4.2's audit shows zero remaining depth-recursion on
library paths, drop `recurse::descend!` and the `stacker`
dependency; re-denominate `clock::tests::deep_tree_stack_safety`
(the depth-100k proof becomes a plain deep-input test of the
iterative walks) and update the crate's AGENTS.md hard rule in the
same change. If any site must stay recursive, record which and why.
*Deps*: P4.2 and the P4-tail P3.8 (the text walks are the last
library-path recursion scheduled to convert).

**P5.4 — documentation closeout (user sign-off, per the process
constraint of record).**
*What*: the §6 invariant statement lands in the crate docs now that
it is true — restated over content bits for the
content-materializing operations per §10.6, over packed operands
for the delta-native ones, with the P3.3 denomination stated as
part of the contract and every cost claim carrying its epistemic
status; the `Key` stability promise in `rumors`' `src/tree/key.rs`
gains its same-code-version qualifier; the bookmark format's
version-mismatch semantics; `before`'s crate-doc Efficiency section
re-measured under skyline (the ~100-bytes-per-Version figure and
the space plot) with `just readme` re-derivation; and the
design-doc distillation pass over this campaign's documents. All
user-facing prose flagged for sign-off item by item. *Deps*:
everything prior.

**P5.5 — acceptance sweep of record.**
*What*: `just all` clean (feature matrix, wasm, bench builds, fuzz
targets, viz bundle); the board all green at default and record
scale, three identical runs; the bench before/after table of
record showing no regression and improvement on every touched
operation; the P3.6b coverage audit re-run with an empty gap list
(every public operation proptested against both reference oracles
and resource-pinned by a board row or an enforced envelope); the
§14 acceptance entry recorded. *Deps*: all.

### 17.3 Coverage accounting

Every red cell and every enforced envelope row appears in exactly
one kill list; the sums close — at both acceptance scales, because
the red set is scale-dependent. Amended 2026-07-23 (plan
adversarial review): this section originally assumed the 96
default-green cells stay green at record scale; a ×4 witness run
refutes that — 85 green / 73 red (158 cells), the 62 default reds
unchanged plus eleven record-scale-only reds, every one a segment
red (recursion-frame onset above the board's default depths — the
§13 segment-onset caveat, which is not id-specific) **[measured**
— single ×4 run, dev profile, limb-meter lit; the enumeration is
re-witnessed at the P3.4 pinned scale, and any cells beyond these
eleven are assigned owners there by dated amendment**]**.

| item | kills, default scale | count |
|---|---|---|
| P3.5 | decode-V5 cells (`version_decode`/`clock_decode` × dense, bigroot) | 4 |
| P3.6 | comparison reads (8) + emit paths (31) + projection (5) + rank (1) | 45 |
| P3.7 | grow cells (`version_tick_adv_party`/`clock_tick`/`clock_recv` × id-pair) | 3 |
| P3.8 | Display (4) + FromStr (6) | 10 |
| **default-scale board total** | | **62** |

| item | record-scale additions (×4 witness, all segment-onset) | count |
|---|---|---|
| P3.6 | `version_rank × bigroot`, `version_min_ticks × dense` | 2 |
| P3.8 | `version_display`/`clock_display` × bigroot, `party_from_str`/`clock_from_str` × id-pair | 4 |
| P4.1 | `party_join`/`party_covers`/`party_disjoint`/`clock_join`/`clock_sync` × id-pair | 5 |
| **record-scale board total** | 62 + 11 | **73** |

| item | envelope kills | count |
|---|---|---|
| P3.10 | event + cliff envelope rows re-pinned at sweep constants | 12 |
| P4.1 | `ID_JOIN`/`ID_COVERS`/`ID_DISJOINT` segments → 0 | 3 |
| P5.1 | `ID_WITHOUT` final ratchet (all rows final-tightened here) | 1 |
| **envelope total** | | **16** |

62 kills + the 96 default-green cells = 158 = the whole board at
the default scale; 73 + 85 = 158 at the ×4 record witness.

Amended 2026-07-23 (P3.3 landed): the board is 160 cells — the
two comb-scatter projection cells joined (green at the default
scale, so the default kill lists are untouched by them) — and the
amended criterion surfaces two default reds beyond the table:
`version_from_str`/`clock_from_str` × benign, red on the κ limb
ceiling (schoolbook on organic values), owner **P3.8** with the
rest of the text column, whose default-scale kill count is
therefore 12 and the default-scale total 64. The sums close as
64 kills + 96 default-green = 160.

Amended 2026-07-23 (P3.4 landed): the pinned record scale is ×4,
and the baseline red enumeration is re-witnessed there against the
enlarged board **[measured** — twice, byte-identical**]**: 85
green / 75 red — the 64 default reds plus this section's eleven
segment-onset cells exactly, no new cells to assign, both
comb-scatter projection cells green at both scales. The
record-scale sums close as 75 (P3.5's 4 + P3.6's 47 + P3.7's 3 +
P3.8's 16 + P4.1's 5) + 85 = 160; P3.8's sixteen are the ten
original text cells, the two benign κ reds, and its four
record-scale onsets. Realization staging is unchanged: C2 realizes
54; the P4-tail P3.8 its 16; P4.1 its 5.
Realization is staged: C2 realizes 54 of the 73 (P3.5's 4 +
P3.6's 47 + P3.7's 3); the P4-tail P3.8 realizes its 14 when it
lands; P4.1 its 5 — so the board is all green at both scales only
after the P4 tail, P3.10 verifies the 54 with the remainder
enumerated as expected, and the all-green-at-both-scales claim
belongs to P5.5. P3.6b kills nothing: its deliverable is the empty
gap list (every public operation dual-oracle proptested and
resource-pinned), carried as a P5.5 acceptance clause. The sixteen
enforced `tests/meter.rs` rows all re-pinned at their final
mechanisms.

The families P3.2 adds (wide-tooth, fan, cancelling-prefix, deep
alternating spine) are born green against their pinned envelopes —
they meter the new accumulator, which lands with those envelopes
in the same commit. The comb board column is not born green
(amended 2026-07-23, plan adversarial review — the original
sentence claimed it was): it joins at P3.5 while every public
operation still routes through the old code, whose plain
running-value sweeps are the §10.6 Θ(W²) genre on the comb
**[measured** — the `meter/tier2` plain-sweep pin**]**, so the
column joins with EXPECTED interim reds — enumerated as such in
P3.5's landed entry, each tagged with its owner (realization at C2
for the sweep-path rows, at the P4-tail P3.8 for the text rows;
verification at the following board re-pin): dashboard honesty,
not a coverage regression. The column adds ~31 cells (the 11
id-side rows are inapplicable, per §13's exclusion arithmetic)
**[derived** — confirmed against the committed row set when P3.5
lands**]**, and the sums above are restated over the enlarged
board in that landed entry. Coverage ratchets upward, per the
process constraint.

Amended 2026-07-23 (P3.5's board family landed): the board is 191
cells — the cliff column's 31 joined (the 11 id-side rows
inapplicable, confirming the derived count) — and the column's
interim red set is exactly two cells, not the broader sweep-path
set this section anticipated: under today's coding every comb
crossing is paid by its own stored code, so the sweep-path rows
read green pre-flip and the §10.6 genre becomes board-visible only
at C2 (the §13 landed record carries the mechanism). The kill
lists move only on P3.8: `version_from_str`/`clock_from_str` ×
cliff join its text column, its default-scale count 12 → 14 and
its all-in count 16 → 18. The sums close as 66 kills (P3.5's 4 +
P3.6's 45 + P3.7's 3 + P3.8's 14) + 125 default-green = 191, and
75 → 77 at ×4 (P3.5's 4 + P3.6's 47 + P3.7's 3 + P3.8's 18 +
P4.1's 5) + 114 = 191. Realization staging is unchanged: C2
realizes 54; the P4-tail P3.8 its 18; P4.1 its 5.

Amended 2026-07-24 (P3.6 fix round: the 200-cell restatement this
section's convention requires, skipped when the board grew). The
board is 200 cells — the §17.7 harmonic column on the four
linear-functional rows (4), the §17.7 `rank_pair_ops` row ×
{dense, harmonic, benign} (3), and the §17.9 fold rows
`version_join_all`/`party_join_all` × scatter (2) — and the sums
are restated over it at both scales from §13's 2026-07-24 landed
record (each scale run twice, byte-identical enumerations):
**71 kills + 129 default-green = 200; 83 + 117 = 200 at ×4.**

Default-scale movements: **P3.6** gains `version_rank`/
`version_distance`/`version_lag` × harmonic (V6's remaining
segment legs, retired with the query kernels at C2), 45 → 48; a
new **C2-adjacent** line owns `version_join_all`/`party_join_all`
× scatter — the §17.9 n·log n marginals, cured only by the
post-flip n-cursor merge, so their realization sits beside C2
rather than inside P3.6's count — for 2. Total 66 + 5 = 71.

Record-scale additions beyond the eleven, each owned here by this
dated amendment: `version_min_ticks × harmonic` (segments 24,
exponent 3.59 at ×4) is the min_ticks event-walk recursion-frame
onset — its dense sibling's mechanism on the deeper harmonic
operand — owner **P3.6**, retired at C2 by the same iterative
sweep (P3.6's record-scale additions 2 → 3). `rank_pair_ops ×
benign` (limb exponent 1.87 over a 6 → 7-byte input at ×4) is the
recorded sub-KiB artifact genre, red at the acceptance scale
because the benign rank pair's packed operand grows by one byte
per size doubling: the exponent leg divides by a near-constant
denominator and reads rounding noise, not law (absolute work is
tens of limb ops; the row's dense and harmonic cells, whose
operands scale, read exponent ~0.98). Owner **P5.5**: the
acceptance sweep scales the benign rank-pair operand with the
board's size knob so the leg reads law — a one-line population
change deliberately deferred so this baseline stays comparable
across the C2 flip — expected green by mechanism once the
denominator moves. One verdict is non-monotone and recorded as
such: `party_join_all × scatter` is red at the default scale
(scan 104.6/B against 96) and green at ×4 (92.5/B), so the ×4
witness under-detects this cell, the default scale is its binding
witness, and it stays in the C2-adjacent line's default count.

The ×4 sums close as 83 = P3.5's 4 + P3.6's 51 (47 + the three
harmonic linear-functional cells + `version_min_ticks ×
harmonic`) + P3.7's 3 + P3.8's 18 + P4.1's 5 + the C2-adjacent
line's 1 (`version_join_all × scatter`; its party sibling is
green at ×4 per the non-monotonicity note) + P5.5's 1
(`rank_pair_ops × benign`); 83 + 117 = 200. Realization staging:
C2 realizes 54 + P3.6's four additions above = 58 of the ×4 set;
the P4-tail P3.8 its 18; P4.1 its 5; the C2-adjacent merge its 2
(1 of which is red at ×4); P5.5 its 1.

### 17.4 Commit choreography summary

- **C0** (P3.1): rebase onto post-`link-transport` main; seam
  re-sweep; gate green for the first time in the campaign.
- **C1** (P3.2–P3.7 with P3.6b, several commits, wire bytes
  untouched, every commit gate-green — read modulo the recorded
  fourteen-test stall roster until C0 lands, the §14 convention, so
  the C1 items that may precede the rebase are unambiguous;
  unqualified gate-green starts at C0): accumulator + generators +
  re-amendment + the `num-bigint` floor; criterion amendments;
  codec + validator + transcoder + subadditivity verdict; sweep
  kernels + bench baselines + the dual-oracle coverage audit; grow.
  The old codec is the oracle throughout.
- **C2** (P3.9, one commit): the flag day — flip, rewire, delete,
  re-pin the full artifact enumeration; text flips as a
  correctness-only port. Preceded by the DECIDED entry and Gate B's
  recorded sign-offs.
- **C3** (P3.10): board + envelope re-pin; bench deltas of record.
- P4 and P5 land as ordinary gate-green commits per item — P4.1,
  P4.2, then the resequenced P3.8 (text, the P4 tail, per the
  user's 2026-07-23 ruling), then P5.

The DECIDED entry in §12 remains the user's to record; this
section is the plan it points at.

### 17.5 Post-campaign docket (recorded 2026-07-23, user directives)

Adjacent work that deliberately waits for the P5 acceptance gate —
each item is a pure refactor or an upstream contribution whose
safety argument IS the finished gate (all-green board at both
scales, envelopes, byte-pinned snapshots):

- **Extract the accumulator as a workspace crate.** The balanced
  signed-digit accumulator's natural API (add-small, add-wide, sign,
  drain-to-limbs) has nothing `before`-specific; a workspace crate
  (unpublished until a second consumer stabilizes the API) gets it
  isolated proptests and a fuzz target. Its amortization contract is
  subtle (reads mutate; bounds tied to the lazy-zone constants), so
  publication waits.
- **Evaluate replacing `Base`'s internals with an existing
  small-value-optimized bignum crate** (candidates: `ibig`, `dashu`;
  `num-bigint` lacks the inline-small representation). `Base` never
  reaches the wire — magnitudes are gamma-coded by this crate's own
  encoder from the value's bits — so the swap is byte-invisible and
  fully checkable under the gate. Target shape: `Base` stays as a
  thin metered newtype (the limb-meter seam survives as arithmetic
  delegation), the owned arithmetic is deleted. Decide by: the bench
  suite (no regression, full sampling), the limb envelopes re-pinned
  through the wrapper, dependency health/MSRV, and the no-unsafe
  policy read (internal `unsafe` in a dependency is acceptable;
  owned `unsafe` is not).
- **Upstream the divide-and-conquer radix parsing to `num-bigint`.**
  The rendering direction landed upstream in 0.4.7; the parsing
  direction (this campaign's D&C parse and its measured exponent)
  is a natural PR. Prep after the text item (P4-tail P3.8) lands
  the local implementation; after an upstream release ships it,
  bump the dependency floor and delete the local copy.

### 17.6 Representation pins for all exposed types (recorded 2026-07-23, user directive)

`before` is independently useful, so every exposed type's externally
observable representation is snapshot-pinned inside `before`'s own
test suite — encode/decode bytes, serde/borsh forms, `Display`/
`FromStr` text, and documented `Ord` contracts — even for types
`rumors` never wire-communicates (`Rank` named specifically). A
representation change must force a deliberate re-pin; it must never
pass silently, and never be pinned only downstream. The Rank audit's
returned scope carries the gap inventory; the pins land with the
coverage audit (P3.6b) or, for types whose representation changes at
the flip, in the same commit as their re-pin. The full-surface
measurability constraint (§14) is read as three-legged from here on:
dual-oracle proptests, a resource pin, and a representation pin per
exposed type.

### 17.7 Rank parity scope (recorded 2026-07-23, from the Rank audit)

The V6 finding (§15 amendment) and the Rank coverage gaps attach to
the plan as follows; the Rank-representation ideation (compactness;
green field — Rank has no serialized surface today) reports
separately before anything beyond this scope is planned.

- **Generators**: `meter::harmonic(d)` (H(d), `6d + 2` bits,
  closed-form pin) joins the §2 families. Adversarial Rank pairs are
  built through the public API only: dense-derived max-exp ranks ×
  exp-0 ranks; harmonic-derived max-width numerators; bigroot-derived
  wide integer parts.
- **Board** (P3.6b): the rank/distance/lag/min_ticks rows gain the
  harmonic family; a new `rank_pair_ops` row (cmp + checked_sub +
  add on the mismatched-exponent pair) replaces the drowned
  "runs inside distance/lag rows" NA rationale; the board doc
  records Rank's denominator of record — value content
  `bits(num) + exp`, wire-bounded via the audit's construction-path
  bound. `version_rank × harmonic` lands red (limb exponent ~2.0),
  an honest baseline retired at P3.6.
- **Envelopes** (tests/meter.rs): RANK_HARMONIC (d = 65,536),
  RANK_DENSE (control), RANK_BIGROOT, RANK_PAIR_MISMATCH
  (d = 500,000) — discharging §15's meter note in the enforced
  suite, current-cost ×1.25 convention.
- **Proptests** (P3.6b): distance/lag dual-oracle differentials
  (tree oracle pins the arithmetic; semantic Riemann-sum oracle pins
  the meaning); Rank monoid and order self-laws (associativity,
  commutativity, identity, add-monotonicity, checked_sub inverse
  and `Some ⟺ rhs ≤ self`, `Sum == fold`); a cross-path
  normalization/Hash witness.
- **Benches** (P3.6): rank/distance/lag × {dense, bigroot, harmonic,
  benign}, IDs mirroring the board.
- **Kernel requirement** (P3.6, load-bearing): the rank/distance/lag
  delta-algebra kernels run their telescoped sums on the balanced
  signed-digit accumulator, with H(d) as the regression witness.
- **Point fix**: checked `exp + 1` in the rank fold (V6's u32-wrap
  edge), Tier-0-sized, lands with the kernel work.
- **Representation pins** (§17.6 standard): a Rank snapshot block
  (integral, fractional, normalized-after-sub, ZERO, spilled >u64
  numerator rendering), a Debug≡Display assert, and an error-type
  Display-string snapshot block; recorded fact — Rank and Ranked
  have no serde/borsh/encode surface, so the ideation starts green
  field.

Landed 2026-07-24: every item above is in the tree. The kernel
requirement and point fix landed with the fold cure (§15's dated
note); the envelope rows carry their movements in place
(`tests/meter.rs`, four-column rank harness); the proptests landed as
`distance_and_lag_realize_both_oracles`, the Rank monoid/order law
suite, and the cross-path normalization/Hash witness
(`version/tests.rs`); the board's linear-functional × harmonic column
reads limb exponent ~1 with the segment legs left for C2.

### 17.8 Rank representation (returned scope, recorded 2026-07-23)

The §17.7 ideation returned; the finding is that no representation
change is warranted, and one algorithmic change is.

- **Consumer inventory [measured]**: production storage of `Rank` is
  exactly one site — `CausalMessages`' staged backlog
  (`BTreeMap<(Rank, Key), Leaf<T>>`, one rank per undelivered
  message, worst case the whole set on a fresh subscribe; key 64 B of
  which Rank is 32). `Ranked` has no production consumer; `before-viz`
  none; no serialized surface (the §17.7 green field, confirmed by
  sweep of both trees).
- **Class-first comparison (adopted; lands with P3.6)**: `Rank::cmp`
  decides mismatched magnitude classes from `bits(num) − exp` in
  O(1), and resolves class ties by MSB-aligned mantissa windows
  streamed most-significant-limb-first — no alignment shift is ever
  materialized, and `checked_sub` routes its ordering pre-check
  through the same path, so only its `Some` arm (whose transient is
  the output's own value content) still aligns. Soundness of the
  prefix rule rides on the stored normalization invariant (odd
  numerator, zero pinned to exponent zero): equal shared windows with
  unequal mantissa lengths order by length because the longer
  mantissa's last bit is 1. Cost: O(1) on unequal classes, O(min
  mantissa limbs) on ties, zero allocation **[measured — scratch
  probe, release, 25k random pairs fuzz-agreed against the alignment
  oracle: the RANK_PAIR_MISMATCH shape 1.4–2.6 µs → 1 ns at
  d = 500k/1M; same-class 17-bit-vs-(d+17)-bit 6.9 µs → 3 ns at
  d = 1M; deep shared-prefix ties at parity or better]**. The
  RANK_PAIR_MISMATCH envelope pins at the new cost after this lands.
  Display, Eq/Hash, and the exp ≤ depth bound are untouched; the limb
  meter records streamed windows so the metered cost stays honest.
  Landed 2026-07-24 as `Base::msb_cmp` plus the class-first `Ord`:
  RANK_PAIR_MISMATCH's limb pin moved 54,710 → 39,078 (the remainder
  is `checked_sub`'s `Some` arm and the addition, both the outputs'
  own value content); the 25,000-pair alignment-oracle agreement
  landed as a deterministic sweep with antisymmetry and the pre-check
  consistency asserted on every pair.
- **Representation changes rejected**: float-style re-denomination
  (the class is O(1)-derivable from the stored form; the field change
  buys nothing) **[derived]**; lazy unnormalized sums (Eq/Hash force
  normalization at every observation — the cost moves rather than
  disappears, or interior mutability imports a sync and audit
  surface) **[derived]**; shared-exponent batch containers (the one
  bulk consumer needs per-key `Ord` in a `BTreeMap`; no access
  pattern fits).
- **Compact inline/spill form (held, not planned)**: an
  inline-`(u64, u32)`-or-boxed-spill repacking measures 16 bytes and
  covers 100% of organic ranks inline — gossip meshes at 4→1024
  parties produce exp = log₂(parties) and ≤ 16-bit numerators;
  fork-leak churn at ~6k parties reaches exp 28 / 32-bit numerators;
  tighter sub-16-byte packings lose 37–50% of the churn regime
  **[measured — public-API harness, seeded, five regimes]**. It
  would save 22% of a staged-map entry's payload — a transient
  backlog, so held until bulk-memory pressure is observed. Spill
  canonicality (un-spill on subtraction, the `Base::from_big`
  discipline) would become load-bearing for structural Eq/Hash.
- **Serialized encoding (deferred)**: no consumer stores or wires a
  `Rank`; minting a codec surface creates §17.6 obligations with no
  beneficiary. If one is ever wanted: strict decode must reject
  non-normalized forms (even numerator with nonzero exponent; zero
  with nonzero exponent) so byte equality keeps implying value
  equality.
- **`Sum` fold amplification (probed 2026-07-23)**: the public
  `Sum` impls fold through a growing normalized accumulator; one
  high-exponent rank plus n integer ranks costs Θ(n·exp) limb work
  against Θ(n + exp) input value content **[measured — scratch
  probe, release, public API only (`dense(d)` rank `1/2^d` via
  decode, integer-leaf ranks via `try_from`), n = 10k/20k/40k ×
  exp = 250k/500k/1M: limb ops = 4·n·exp/64 exactly in every cell
  (four full-width passes per element: accumulator shift, operand
  shift, add, renormalization shift — the odd numerator means
  normalization never narrows the accumulator), 625M limb ops /
  0.19 s at n = 20k, exp = 500k against ~168k limbs of value
  content (~3,700×); every sum value checked equal to a two-pass
  reference]**. Order-dependence pins the mechanism: the same
  multiset with the high-exp rank last costs 8n + 2·exp/64 (176k
  limb ops, 0.3 ms — the accumulator widens only at the final add),
  and `Sum` accepts arbitrary order. Curable inside `Sum` with a
  raw accumulator and a single final normalization, no semantic
  change: the cure shape measures 2·exp/64 independent of n (15.6k
  limb ops at the largest cell, ~40,000× headroom) and returns the
  identical `Rank`. The cure attaches to P3.6 alongside the rank
  kernel and the class-first comparison; the §17.7 envelope list
  gains a RANK_SUM_MIXED row (n integer ranks + one dense-derived
  high-exp rank, high-first ordering) pinned at current cost ×1.25
  before the cure and re-pinned after. Landed 2026-07-24: both `Sum`
  impls fold through one raw accumulator anchored at the running
  maximum exponent (each summand digit-routed at its exponent gap;
  one rescale per exponent raise, paid by the raising summand's own
  exponent; one normalization at the end), value-identical to the
  pairwise fold by proptest over arbitrary multisets in arbitrary
  order. RANK_SUM_MIXED's limb pin moved 156,312,196 → 3,908
  **[measured** — the enforced row, three identical runs**]**, peak
  heap 125,032 → 62,512.

### 17.9 Public-API census (recorded 2026-07-23)

The full-surface enumeration the §14 measurability constraint calls
for, run as an independent census against the worktree at P3.4's
landing. Method of record: rustdoc JSON (nightly, all features)
walked from the crate root along public reachability — 335 census
entries (22 types, 109 methods, 25 free functions, 10 modules, 9
constants, 160 name-level trait impls), reproduced byte-identically
across two runs. Apparent `Div`/`RangeBounds` rows on `Party`/
`Version` are rustdoc cross-listings of `impl Div<&Party> for
Version` and `impl RangeBounds<Version> for causally::Range`
**[measured** — source-verified**]**, not surface. Every board-NA
item was triaged by implementation read; the census found one new
amplifier and two coverage gaps.

- **The join-direction folds are quadratic: V7 — the fold genre
  (V6's mechanism) generalized beyond Rank.** `Version::join_all`/
  `Sum`/`FromIterator`, `Party::join_all`, and `Clock::join_all`
  fold sequentially through a growing accumulator, each step
  walking — and, on the version side, repacking — the whole
  accumulated value. On n scatter-ordered organic single-tick
  versions (built through the public API: balanced forks, one tick
  each, evens before odds so the accumulator never coalesces), limb
  work quadruples per n-doubling — 193.8M → 773.7M → 3,091M at
  n = 4Ki/8Ki/16Ki against 28→131 KiB of packed input, wall
  0.55 → 2.2 → 9.0 s **[measured** — scratch probe, release,
  public API only, result verified equal to a balanced-reduction
  reference each run**]**; the balanced reduction over the same
  multiset is amortized-linear (14.4M limb ops, 47 ms at the
  largest cell, ~190× and growing). `Party::join_all` shows the
  same law on wall (×3.8–3.9 per doubling) while **every
  deterministic meter column reads zero** — id walks do no `Base`
  arithmetic, join mutates in place, and the per-join walk is
  iterative — so the board is structurally blind to the id-side
  fold the way it was blind to rendering class before the display
  canary. `meet_all` does not amplify: its accumulator is bounded
  by the smaller operand **[derived]**. No `rumors` production path
  calls any of these folds; the exposure is `before`'s
  independent-library surface, the §17.6 population. The cure
  attaches to **P3.6** (balanced reduction now, or the n-cursor
  sweep once the kernels exist); the board's "folds of measured
  rows" NA rationale — the same drowned genre §17.7 retired for
  Rank — is replaced at **P3.6b** with a `version_join_all` ×
  scatter row plus a wall-witnessed party-fold scenario beside the
  display canary, pinned red before the cure and re-pinned after.
  Landed 2026-07-24 (the balanced-reduction arm): all five fold
  surfaces reduce on a binary-counter stack (every input O(log n)
  joins against similarly-sized partners; the fallible folds test
  each input against the *fixed* `self` up front and join surviving
  groups into `self` at the end — infallible on well-formed input,
  with aliased inputs colliding pairwise on the way in), identical
  results pinned by proptests against the sequential fold reference
  over organic populations in both orders plus a deterministic
  aliased-duplicate pin. Two enforced envelope rows land on the
  scatter recipe at 1,024 clocks: version fold limb 690,310 against
  the sequential 14,281,732 (20.7×), party fold scan 292,432 bits
  against 3,284,952 (11.2×) **[measured** — `tests/meter.rs`, three
  identical runs**]**. The board's scatter cells tighten from
  exponent ~2 to the reduction's n·log n law but stay marginally red
  at the default scale (version limb exponent 1.16 against the 1.15
  ceiling; party scan constant 104.6/B against 96) — the flat
  ceilings cannot certify an n·log n fold, so full green on these
  two cells awaits an n-cursor merge over the skyline kernels after
  the flip, recorded here as the C2-adjacent follow-up. One forced
  public-prose touch, flagged at the commit: the two `join_all`
  `# Errors` paragraphs now describe pairwise coalescing (the
  running-union sentence described the left fold) and note that
  aliased inputs may return already joined.
- **serde forms are untested in-crate.** No roundtrip, no byte-form
  pin, no strict-reject test exists for the serde
  `Party`/`Version`/`Clock` impls (borsh has all three legs via its
  `bytes ≡ as_bytes` proptests anchored to the snapshot-pinned
  canonical forms). This is the §17.6 downstream-only-pinning hole
  verbatim; the mirror suite lands with **P3.6b**'s pin inventory.
- **NA-list prose omissions**, no new mechanism **[derived** — each
  delegates to a measured kernel, verified by read**]**: the
  `batch::Version` operator matrix (routes through
  `join_view`/`meet_view` and the cmp walk), the consuming
  `From<Party> for [Party; N]`/`From<Clock> for [Clock; N]` splits
  (the `forks` machinery), `Forks::Drop`'s O(log n) rejoin, and
  `causally::Range`'s refinement/accessor methods. The board module
  doc's exhaustiveness claim is repaired at **P3.6b** in the same
  commit as the audit enumeration, together with `meet_all`'s
  mechanism-based NA reason.

Everything else on the 335-entry surface is pinned or NA with a
mechanism-based reason that survived an implementation read; the
Rank gaps remain owned by §17.7–§17.8 exactly as recorded.

Addendum (2026-07-23, user directive following the census): the
id-side blind spot gets a deterministic instrument rather than a
wall-clock witness. **The scan-work meter** — a fourth meter column
counting bits scanned and written at the packed-stream primitives
(idbits tag reads and skip steps, IdBuilder bit writes and splice
lengths, the event topology cursor's advances and gamma code-skips)
behind an additive off-by-default feature in the limb-meter idiom —
lands at P3.6b. The board gains the column (exponent + per-byte
constant); envelopes may pin scan bits ≤ C·(input + output) per
operation; the V7 party-fold scenario pins deterministically instead
of by wall; P4.1's id-walk rows gain a work pin beside their segment
pin; §11.4's word-scale scanning, when it lands, tightens the same
envelope it is measured by. Known exception, unchanged: the display
canary stays wall-clock (num-bigint's internal work is invisible to
any in-crate shim; recorded at its landing).

### 17.10 Campaign operations runbook (recorded 2026-07-23, checkpoint)

State of record at this checkpoint: branch tip b905b584; P3.5 closed
and verified (gate green modulo the fifteen-test roster; meter suite
34/34 twice, byte-identical; board 125/66 default and 114/77 at ×4,
records exact; 53 pins green). Execution resumes with the staged
kernels workflow (P3.6/P3.6b/P3.7), then C2 (awaits the user's
link-transport-landed signal, the C0 rebase, and the artifact-list
re-derivation), C3, P4.1, the P4-tail text item, P5.

- Pending micro-item: §13's record-scale runtime note predates the
  cliff family's board landing (P3.5 review advisory); correct at the
  next §13 touch.
- Gate invocation of record: `SWAP_LIMIT_GB=24 PROC_LIMIT_GB=16 just
  gate` (documented memwatch overrides — stale system swap, per-proc
  rustc peaks); the display canary keeps its reserved-runner nextest
  wiring; the workspace test step fails with exactly the §14
  inherited roster until C0 (sixteen tests as of 2026-07-24), read
  under §14's provenance-replay rule: a novel `Stalled` failure from
  a fresh seed line in one of the three stall files that replays red
  at the plain merge-base joins the roster by amendment; any other
  novel failure blocks.
- Environment cautions: cargo's global package-cache lock can convoy
  behind wedged rust-analyzer `cargo metadata` children — diagnose
  with `lsof ~/.cargo/.package-cache`, suspect the oldest zero-CPU
  holder; killing the cargo children is safe (the editor respawns
  them). Worktree target dirs regrow to hundreds of GB across gate
  runs and are freely deletable under disk pressure.
- Workflow-script practices for the scripts still to be written
  (C2, P4, P5): embed absolute paths as constants in the script;
  instruct every agent to end by calling its structured-output tool
  and to run final verification synchronously; on an agent death,
  salvage = read its transcript, adopt the partial diff critically,
  commit, resume from the run id (completed agents replay from
  cache); instruments and red baselines land before the cures that
  retire them.
