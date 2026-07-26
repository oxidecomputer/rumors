# `before`: adversarial resource amplification in Version and Party computation

Status: execution in progress on branch `before-hardening`. Complete:
the audit, the Tier 2 decision, the pre-flip kernel window, C0, C2
(the flag day), the tick limb cure and fusion (#34), and C3's
denomination and classification round. The campaign stands at C3's
bench-harness remainder (§17.2's queue of record, items 11–13), then
the materializing emitters, the P4.2 residual audit, and P5 closeout.
This document is the single canonical source for the criteria the
instrumentation enforces and the work that remains, written to the
current state of the tree; the compact history is the decision record
(§12) and the phase ledger (§14), and everything else — landed-work
narratives, superseded amendment chains, measurement logs whose
conclusions are pinned in code — lives in git history
(`git log design/before-adversarial-resource-amplification.md`).

Scope: `rumors`' model of record is authenticated-honest-peer, so
none of this is a `rumors` security finding — an authorized peer
holds write authority and needs no memory tricks. The goal is to
harden and performance-optimize `before` *unconditionally*: as a
standalone library whose `decode` boundary may face untrusted
bytes, and because every amplification constant is also a tax on
honest deep or large inputs. Native builds only; wasm is demo-only.
The yardstick is resource proportionality — transient cost as a
function of input size, for whoever presents the input — never
adversary economics.

Epistemic key: **[measured]** = observed in an instrumented
experiment; **[derived]** = argument from the code or arithmetic;
**[open]** = known unknown / decision pending.

## 1. Problem statement

A `Version` at rest is a packed preorder bit stream; a `Party` is a
packed preorder id tree of 2-bit child-presence tags. The question:
can an adversary craft canonical, normal-form inputs whose
*computation* costs memory or CPU grossly disproportionate to their
encoded size — and can the library compute with cost proportional
to the packed size, **without bounding inputs** and **without
losing the compactness of the representation or the `O(n + m)`
operation costs**? Answer: yes — the audit found seven amplifier
classes and the campaign's adversarial rounds a further set (§3's
ledger), all cured, because every quantity the algorithms need at a
node is either one of two global accumulators or bounded by that
node's own coded size. The fix of record is the Tier 2 skyline
representation (§10), the shipped production coding since C2.

## 2. Input families

The adversarial constructions live as committed generators
(`meter::` and `testing::generators`); their bit-layout derivations
are in the generators' rustdoc. The family vocabulary, which the
board (§13), the envelopes (`tests/meter.rs`), and the benches all
share:

| family | shape | witnesses |
|---|---|---|
| dense | zero-base left spine, depth ~n/4 | node count + recursion depth |
| bigroot | B-bit magnitude over a dense spine | per-frame magnitude clones (V1) |
| hugeleaf | one leaf, value `2^B − 1` | width-quadratic decode (V3) |
| cliff (boundary comb) | teeth `(2^k − 1, 0, 1)`, leaf values oscillating `2^k − 1 ↔ 2^k` | carry-cliff crossings paid by their own codes; delta-coded value content exceeding wire bits (§10.6) |
| wide-tooth comb | teeth `±2^192` across a `2^k` cliff, `k ≫ 192` | two-zone accumulator refutation; freeze-discipline funding |
| unpaid-crossing fan | `(1, 0, 1)` teeth under one stored `2^k − 1` root | cliff excursions NOT paid by local codes |
| cancelling-prefix / static-prefix | wide cancelling setup, then unit writes / sign reads | the accumulator's collapse rule |
| jump_comb | low tooth, one mid-stream `2^k` jump, cheap teeth after | exactly one freeze eviction |
| harmonic H(d) | `(0, ·, 1)` spine, rank `(2^d − 1)/2^d` | the rank fold's width ramp (V6) |
| alternating spine | deep alternating-binary topology | frame-count adversaries (grow) |
| id spine I(d) / id-pair | unary chains, shared prefix, divergent tips | two-operand id walks at full lockstep depth (P1) |
| scatter | n single-tick organic versions, evens before odds | fold accumulator growth (V7) |
| comb-scatter | cliff comb × scattered party | output-dominated projection; the flat-denominator exponent rule (§6) |
| benign | small organic values | the control; the parity floor's referent |
| nested-full-sibling | `(x,1)` repeated down a spine × matching event spine | the paired tick walk at maximal shortcut depth (the fill linearization's family; pinned scan-linear) |
| nested-wide | bigroot magnitude × the nested-full id | the wide right-full return chain (#34's genre; pinned limb-flat) |
| mirror / wide tail | `(1,x)` down a right spine × a zero spine with one wide tail leaf | the memoized pre-scan at full depth with wide minima per level (#34's genre); the unit-tail cross is the memo machinery's own cell |
| descending staircase | monotone-descending unit-delta leaves × the unary id spine | full-penetration minimum updates at every level, width-independent (the anchor-web walk's propagation witness) |
| memo chain | `k` consumption-sibling single-leaf left-full sites under one covering site, minima distinct or shared | the frame ledger's resolution cost, pinned flat ×2.00 across the doubling (the shared twin is the flat control) |
| memo comb | shallow and covering left-full sites interleaved per level | consumption order Θ(d) from recording order — refutes chain-walking resolutions under every record-to-record anchoring; pinned flat |
| memo fan-out / oscillation | one wide minimum shared by k sites over a unit plateau; minima alternating wide/narrow | the fan-out's k-independent wide cost (absolute touch ceiling) and its funding control |
| memo churn / descending raises | in-flight records under full-penetration drops; raises landing below the frame minimum | one live ledger head; the decide-then-emit ordering's oracle tripwire |
| reveal comb / hifloor | `k` sibling left-full sites sharing one `2^b` minimum over a zero floor, the left-leaning spine closing each site's frame back into the floor frame between consumes; the control's floor raised to `2^b − 2` | the tick walk's width-circulation genre (the spec's §9 rounds 5–6), pinned flat ×2.00 across the joint doubling; the narrow-gap control is flat — the wide gap is the driver, not the shape |
| pure comb | the same left-leaning comb with bare `2^b` leaves and no left-full site anywhere | the base watermark stack's own arm-move + close-pop cycle in isolation, pinned flat per byte — the layer the frame ledger amplified ~10× before the round-6 cure |
| ascending cliff / plateau | `k` ascending wide left leaves `2^b + i` down a right spine over a terminal 0-cliff, id descending to the cliff; the control's leaves leveled at `2^b + 1` | the undercut cascade's fold direction (the spec's §9 round 7), pinned flat ×2.00 across the joint doubling; the leveled control is flat — the nonzero hop schedule is the axis, not the undercut or the spine |

## 3. Findings ledger

Every finding is cured, closed, or landed as a ratchet; each family
above witnesses at least one. Mechanism detail and pre-cure
measurements are in git history at the commits §14 names; the tick
genres' refutation-and-cure records are
`design/before-tick-cost-spec.md` §9. The cures are pinned by the
enforced envelopes and board cells named.

- **V1** (quadratic memory+time; per-frame owned path sums in
  compare/combine): cured by the difference accumulator + the
  skyline sweeps. ×6,668 at 29 KiB when found.
- **V2** (linear ×782 stack; recursion frames in every event walk):
  cured by iterative sweeps/bit-stacks (kernels), explicit compact
  stacks elsewhere.
- **V3** (quadratic decode of wide gammas): cured by limb-wise
  mantissa accumulation (P1/T0.1); 14.5 s → linear on hugeleaf(4M).
- **V4** (linear ×98–198 working form + Builder pre-size): the
  working form is deleted (C2); push-growth (T0.2).
- **V5** (linear ×118 parse stack): the skyline validator needs ~2
  bits/level and no values.
- **P1** (id-side linear ×357–456 recursion frames): iterative
  walks (P4.1), segments → 0.
- **V6** (quadratic rank fold on H(d); 134.7M → 1,025 limbs):
  digit-routed merge + relative freeze trigger + summation-by-parts
  (the freeze cure's honest residual: a stream re-arming wide drift
  under cheap codes at a *dense* compacted position is the one
  shape the funding argument does not certify; every committed
  family's freeze positions are ones-runs). `RANK_HARMONIC` and the
  jump/wide-tooth bands enforce it.
- **V7** (quadratic join-direction folds; 156M → 3,908 limbs on
  `Sum<Rank>`): balanced binary-counter reduction on all five fold
  surfaces + the raw-accumulator `Sum`. The reduction's own
  n·log n reads marginally red against flat ceilings on the
  benign/scatter fold cells — owned by §17.2's fold-marginals item
  (the n-cursor merge).
- **Fill's lookahead/pre-scan terms** (2026-07-25): worst case
  O(|ev| × local-id-depth), quadratic on matched spines with zero
  shortcut sites — the multiplier is the LOCAL id's depth, not
  wire-suppliable by a hostile peer. Cured under the nested-full
  red pin: the right-full arm deferred to an O(1) peek at the
  cursor's arrival, the left-full pre-scan memoized (no position
  scanned twice). Scan e 1.00 both arms; the recursion-depth
  segments residual rides P4.2's pin. A pricing obligation under
  §6, not an exploit.
- **Fill/tick's limb-dimension re-touching** (#34, 2026-07-25/26):
  materialized per-subtree `(min, net)` magnitudes cost Θ(width)
  limb work per ancestor — quadratic on wide × deep crosses through
  BOTH shortcut arms — and the memo's site resolution read Θ(k²)
  accumulator digit touches on consumption-order adversaries, in a
  currency the limb column cannot see. Cured by the anchor-web
  watermark discipline plus the frame ledger of the tick cost spec
  (its §9 rounds 3–4): the wide crosses read limb and touch e 1.00
  at flat constants; memo-chain and memo-comb read ×2.00 touch
  growth across the doubling (×3.94/×3.92 under the refuted chain),
  the pins re-pinned never deleted. Four ledger adversaries guard
  the cure: the wide fan-out (k-independence by absolute touch
  ceiling), the oscillating funding control, the churn family (one
  live head through full-penetration drops), and the descending
  raises (the decide-then-emit ordering's oracle tripwire, verified
  live). The same machinery had carried a semantic staleness bug
  the families' first differential crossing caught (fixed,
  minimized seed committed) — cost families and the semantic suite
  must cross. A pricing obligation under §6 (local-depth
  multiplier).
- **Tick's width-circulation cycle** (found 2026-07-25 by the
  ledger cure's adversarial review; cured 2026-07-26, the spec's §9
  rounds 5–6): on the reveal comb, the consume/close cycle
  circulated a width-`b` boundary difference through
  per-object-legal moves with no input delta, no output code, and
  no funded descent — Θ(k·b) accumulator touches on Θ(k + b) input
  AND output (×3.91 across the joint doubling), with the base
  watermark stack alone paying ~2 wide folds per site and the frame
  ledger's follower ferry amplifying that ~10×. Cured by the latent
  boundary register (the spec's I4′ width conservation): reveal-comb
  reads ×2.00 exactly across the joint doubling and pure-comb flat
  per byte, both re-pinned with absolute bands; the high-floor
  control pins green, flat, and width-independent — the wide gap
  was the driver, not the site forest or the schedule. Semantics
  exact throughout (oracle differentials plus closed-form
  witnesses). A pricing obligation under §6, not an exploit.
- **Propagate's fold direction** (found and cured 2026-07-26, the
  spec's §9 round 7): the undercut cascade folded the wide
  surviving residue into each popped narrow dying difference —
  Θ(k·b) touches on Θ(k + b) input and output on the ascending
  cliff (×3.89 across the joint doubling), and a heap amplifier
  (each popped difference's buffer widened to residue width,
  heap e 1.82). Cured by inverting the hop to the spec's I4′
  rule 2: top-index domination decides each hop's direction in O(1)
  before any fold, the dying side funds the fold that consumes it,
  width guards keep comparable-scale hops at zero extra touches.
  ×2.00 exactly re-pinned flat; heap e 1.00; every other committed
  MEASURED reading byte-identical across the cure; the leveled
  control flat and byte-identical — the nonzero hop schedule is the
  axis. A pricing obligation under §6, not an exploit.
- **Plateau projection output-domination** (closed at C3,
  2026-07-26, under the owner's pre-approved §6 ruling):
  `version_project`/`clock_own_version` × {reveal-comb,
  reveal-hifloor, pure-comb} re-materialize a wide absolute value
  per kept site — mandatory output Θ(k·b) on a Θ(k + b) input —
  and are `n_io`-denominated like the comb-scatter cross. The
  owner's O(`n_io`)-tightness rider is met [measured, release,
  both scales: output ×4.0 per input doubling; limb e 0.96–0.99 at
  ≤ 0.2/B, scan e 1.00 at 8 bits/B, touch e 0.99 at ≤ 0.1/B, heap
  e 1.00 at 2.1 B/B]; the six `n_io` board cells are the committed
  check. A denomination gap, not a kernel finding.
- **Profile-dependent meter readings** (2026-07-26; resolved by the
  owner's ratification, §12): 102 `debug_assert!` sites in the
  production kernels perform metered work (`Base` comparisons
  through the limb shim, skyline grow probes consuming metered
  cursors), so dev and release builds measure different programs.
  **Release is the board's measurement of record** — it prices the
  production work alone, the honest denominator; dev runs are a
  debugging view, never pinned; assertion-scoped meter suspension
  REJECTED on doctrine (a metering-pause mechanism is an F2
  hazard). The one assertion whose metered work moved a rendered
  exponent class was repaired per the ratified policy:
  `id_is_empty` spot-checks the O(1) consequences of its contract,
  and the full O(n) normal-form assertion moved to the diff
  kernel's emission seam (`Party::without`), the one caller whose
  input is not just-parsed bytes — the recorded, owned dev-profile
  divergence (its dev scan reads ~16 bits/B above release by
  design). Post-repair dev-vs-release divergence: zero verdict
  flips and zero exponent-leg ceiling crossings on any work column
  at either scale; scan divergence confined to the owned seam's 13
  cells [measured — post-repair renders, byte-compared]. The
  record-scale segments columns remain profile-dependent by codegen
  (release frames are smaller, so onset shifts) — P4.2's genre, not
  assertion work.
- **The join_all up-front re-scan** (found and cured 2026-07-26).
  `Party::join_all` and `Clock::join_all` test every input against
  the *fixed* accumulator up front — semantically load-bearing for
  the best-effort hand-back granularity — and each test as a cursor
  walk re-scanned the accumulator: Θ(inputs × accumulator) scan on
  a Θ(accumulator + inputs) operand set, e 2.00–2.14 at 47–2,954
  bits/B across the overlap families [measured at the parent tip].
  The landed cure is `IdIndex` (`party/ops/index.rs`): built once
  per fold call in two linear passes (one `u32` per both-present
  node — transient state strictly under the operand; a
  `u32`-overflow operand ≥ 512 MiB falls back to the cursor walk),
  answering each up-front test in O(input) node visits plus one
  O(log accumulator) table search per both-present visit. A pure
  predicate-mechanism swap: hand-back contents, order, and
  accumulator bytes are decided by the identical fold, pinned
  differentially against the cursor-walk discipline preserved as
  `testing::fold_oracle` (a deliberate wrong-child mutation trips
  four differentials). The coalesce-first candidate was REJECTED by
  probe: on the witnessing population nothing coalesces, so it
  degenerates to the same per-input tests while reordering the
  hand-back vector the contract documents — not curative where
  priced, semantics-breaking everywhere. Pins:
  `join_all_overlap_upfront_test_reads_flat` — ×2.00 measured
  across the joint doubling (was ×4), ceiling ≤ ×2.05 over a
  liveness floor of one full accumulator pass; the board row reads
  scan e 1.00 at 15.7–15.9 bits/B on every family at both scales,
  heap at most 10.6/B under the 16 ceiling.
- **The instrumentation census's blind spots** (#39, 2026-07-26; a
  read-only census hunting the F2 genre — work routed through a
  meter that exists but is not pinned on that surface, or through
  no meter at all; every unpinned surface probed measured
  touch-linear, so all items were missing ratchets, not live
  amplifiers). Landed: **the touch column** as the board's fifth
  judged currency, ceiling + floor-or-NA on every cell (the tick F2
  quadratic would otherwise have been board-invisible); **emit and
  text-parse touch pins** (board per-delta floors, gate-side
  cliff-comb flatness pins over one-touch-per-delta liveness
  floors, a render zero-touch conservation pin so accumulator work
  cannot migrate between the text directions silently); **decode
  touch floors** stream-derived (the validator batches word-scale
  deltas in the accumulator's lazy zone, so a per-delta floor
  over-demands); **cmp per-delta floors**; **covers/disjoint
  absolute scan ceilings** ×1.25 over full-examination floors;
  **fork's first envelope row** (heap prices the materialized
  halves; the split kernel's deliberately raw 2-bit scan recorded
  so wiring it into the metered primitives is a deliberate re-pin)
  and the board `party_fork` heap declaration corrected to the
  fork-child floor; **the Shl width re-denomination** (the shim
  records output width; the rank-pair envelope re-pinned
  54,704 → 70,328 limb ops with the movement annotated). DEFERRED
  with reasons: metering fork's raw writes (O(n) copies, low risk,
  wiring moves board scan readings — a deliberate future
  re-denomination); collapsing the four envelope harnesses into one
  five-column shape with per-column floor-or-NA (the #35 totality
  mechanism applied to the gate suite — named for a future round;
  until then the board's tick × scan floors and touch column carry
  the cross-op cover). Ratchets against the F2 migration genre, not
  exploits.
- **Iterated-operation size trajectories** (#38, 2026-07-26; no
  amplification found — six committed deterministic pins, one per
  orbit genre): the board prices single calls, and a single-call
  bound does not preclude compounding across calls. **Fork
  orbits**: chain and fan exactly affine — both halves read exactly
  `2 + 2k` bits at the k-th fork over 512 steps, the fan unwinding
  to a byte-identical seed. **Fork+join round trips**: the untick'd
  trip byte-stationary over 256 rounds; the ticked variant's party
  byte-identical every round, the version one fixed two-leaf
  scaffold costing exactly `7 + 2⌊log2 k⌋` bits over 512 rounds.
  **The paper's §6 churn scenario** (4096 rounds): max id bits
  plateau in a fixed band; max version bits logarithmic per octave.
  **The paper's §6 static scenario**: ids byte-identical forever;
  max version bits exactly `8i − 4` at the octave ending `2^i`.
  The paper's "stabilizes with a minor logarithmic component" is a
  committed criterion, not a chart. Board and judge roster
  untouched: orbits are a test-surface pin genre — single-call
  denomination stays the board's job, trajectory shape the orbits'.

## 6. The design invariant and the denomination criterion

Adopted as the crate's contract, enforced by §13:

> **No operation materializes transient state asymptotically larger
> than its packed operands, and every operation remains amortized
> `O(n + m)` in the packed input bits — with no bound on value
> magnitude, tree depth, or encoded size.**

Denomination (the criterion of record; Gate B): "packed operands"
denominates every operation *except* the three classes whose
mandatory output is asymptotically larger than any constant times
their input — an input-only bound is unsatisfiable by construction
there and would degenerate into exemption holes:

- **Text I/O** (`Display`/`FromStr` for Version/Party/Clock):
  judged against `n_io` = packed input + text output (Display) or
  text input + packed output (FromStr), output read from the
  actual result. The limb column carries **two legs**: the
  *constant* leg against the radix-work denominator
  `R = n_io + Σᵢ (digitsᵢ × limbsᵢ + 10)` over the spelled event
  values — the honest text cost law: schoolbook conversion plus
  the delta⇄absolute pipeline's per-value arithmetic term of 10
  radix units (`TEXT_PIPELINE_LIMB_OPS_PER_VALUE`; [measured at
  C3, release, record scale: the production kernels spend 5–9
  limb ops per spelled value across the small-value families,
  both directions]; id tokens contribute nothing) — at
  κ = 0.75 limb/`R` unit [measured at C3: honest cells read
  ≤ 0.59, the staircase pipeline; digit-by-digit schoolbook reads
  ~1, still excluded] — and the *exponent* leg against `n_io`
  (never `R`, on which any schoolbook converter reads a flat ~1)
  at the unchanged 1.15. Two legs because each catches what the
  other cannot: the chunked-schoolbook refutation (2026-07-23)
  demonstrated a still-quadratic converter slipping under κ, so
  the exponent leg enforces the complexity class and κ the
  constant. Both anti-softening tripwires are committed in
  `meter::board`'s suite (the digit-by-digit parser must exceed
  κ; the chunked probe, driven through `evaluate` itself, must
  slip under κ and read red on exactly the limb exponent). An
  output-honesty ceiling closes the pad-the-output door, asserted
  against the conversion units alone (the pipeline term must not
  loosen it; radix units, forced by the delta coding, derivation
  at the constant, tripwire pinned). The pipeline term's decision
  record is §12's C3 entry — **pending owner ratification**.
- **Output-dominated projection** (`version_project`/
  `clock_own_version` on comb × scattered-party and on the plateau
  crosses reveal-comb/reveal-hifloor/pure-comb, per the owner's
  pre-approved ruling applied at C3): judged against
  `n_io` = packed input + packed output (canonical coding cannot
  be padded), with the sweep measured O(`n_io`)-tight — the
  owner's rider — on every declared cross.
- **Balanced share splitting** (`Party::forks(n)`): its mandatory
  output is `n` packed parties, so it is judged against
  `n_io` = packed input + Σ packed share bits (canonical coding
  cannot be padded). The fuzz-fit mirror computes this from the
  actual shares.

**Rejection rows denominate against the fed stream alone.** A
rejection produces no output, so the text rule's `n_io` degenerates
to its input side: a `FromStr` rejection row is judged per fed
*text* byte at the general limb ceiling (no radix-work term — `R`
prices conversion of the accepting direction, and a rejection
forces no conversion), and a decode/overlap rejection row per fed
packed byte. The pad-the-output door does not open here (the fed
stream is the adversary's own input: padding it inflates the
denominator only by bytes the operation is genuinely asked to
consume), so no honesty ceiling is needed on the rejection side.

Everything else stays input-denominated — both codec directions
(canonical 1:1), all scalar/comparison/query rows, and the
packed-output mutators, whose input denomination rests on the
1-Lipschitz property pinned in `meter/tier2` (output boundaries ⊆
union of the inputs'; total bits within 4 per input leaf of the
inputs' sum) rather than assumed. `meter::board`'s module doc
carries the do-not-re-denominate list. Rank rows denominate
against value content `bits(num) + exp`, which every public
construction path bounds by the producing wire; for consumers
without access to the crate-private parts, the `num/2^exp`
rendering's length is an admissible proxy — the numerator term is
proportional (`digits ≈ 0.301 · bits(num)`) while `exp` contributes
only its own digit count, so the proxy strictly *under-counts* the
denominator, over-flags, and never masks: a cost linear against the
proxy is linear against the criterion.

**Flat-denominator shapes fit their exponents against value
content** (the comb-scatter classification, closed at C3). The
shape scales tooth count at a fixed 1000-bit magnitude: packed
bytes are intercept-dominated (~×1.2 per level) while value content
(§10.6's Σ leaf-height bits) and measured per-tooth work double, so
a packed-byte power-law fit manufactures e ≈ 4 out of flat marginal
work [measured]. The shape's input-denominated cells fit exponents
against the bundle's value content (event-side leaf-height bits +
id-side packed bytes; row-disclosed as `expd[content ...]`);
constants and floors stay per packed byte; I/O cells keep `n_io`;
the bench mirror's denominators follow. Tripwires in
`meter::board::tests`: the packed fit must stay broken on
measured-flat work over the intercept premise, and a
quadratic-in-teeth probe must read red against content. The
column's work is linear on its honest denominator; no cell exceeds
it. The rule's decision record is §12's C3 entry — **pending owner
ratification**.

Statement-faithfulness (the user's standing bar) applies to every
claim in this document and the code's prose: never weaker than
stated, never stronger than proven.

## 10. The skyline representation (shipped at C2)

Preorder topology bits; at each leaf position, in-stream: the
first leaf's value as `gamma(v₁)`, every later leaf as
`zigzag-gamma(vᵢ − vᵢ₋₁)` over consecutive leaves in preorder.
Canonical iff the topology is minimal — the right sibling's delta
is zero exactly when sibling leaves are equal, so the validator
needs ~2 bits/level for minimality plus leaf-value nonnegativity
on the cliff-immune accumulator (a plain big-integer running value
is Θ(W²) on the boundary comb [measured — the `meter/tier2`
plain-sweep pin]). Byte-equality remains `Eq`/`Hash`. All
operations are single forward passes over packed streams; depth
costs bits. The module docs in `src/version/skyline/` are the
documentation of record for the kernels (sweep, emit, query, grow,
fill, text) and the builder; `codec::accum` for the balanced
signed-digit accumulator.

**The subadditivity lemma of record** (proven 2026-07-23; full
derivation in git history; pinned emitter-parameterized in
`meter/tier2` beside the 1-Lipschitz pins, instantiated on the
shipped emitters): for canonical `a`, `b` and `c` either `a ∨ b`
or `a ∧ b`,

    size(c) ≤ size(a) + size(b) − 2   (bits),

`size` the exact skyline bit length (`tier2_size`, bit-equality
with the encoder proptest-pinned). The byte-level corollary is the
form the `link-transport` window budget cites. The −2 is
structural (tight at the empty pair) and matched the ~1.5M-pair
probe's maximum excess exactly. The lemma prices coded size only —
not the sweep's work (§10.6 does), and not `B(c)` reaching the
union bound.

**§10.6, the carry-cliff genre — wire bits do not bound value
content.** Delta coding lets Θ(nk) bits of absolute value content
ride behind Θ(n + k) wire bits (the boundary comb; current/Tier 2
size ratio unbounded). Consequences, all shipped: every
running-value sweep — strict decode included — runs on the
cliff-immune balanced signed-digit accumulator (amortized O(1) per
small delta at every width; the two-zone alternative is REFUTED,
§12); the linear functionals (`rank`/`distance`/`lag`/
`min_ticks`/`max`/`project`) use delta algebra (telescoped
`v₁·W + Σ δⱼ·suffix-weight`), never reconstructing absolute
values; and the §6 invariant for content-materializing operations
(text) is denominated over content bits — which is exactly the
text-I/O criterion above. The cliff board column scales `k = n`
deliberately so any regression of this genre is board-visible.

**Compactness envelope** [measured, ~13k samples + exact closed
forms pinned]: skyline ≤ 2× the packed-era coding outright on
every sample (comb-tight, max ratio 1.9966, monotone toward 2 from
below); realistic gossip median 0.9888, skyline smaller on 61.6%.

## 12. Decisions ledger (DECIDED / REJECTED / RETIRED, dated)

- **DECIDED 2026-07-23 (Finch): Tier 2 — the skyline encoding — as
  a flag day.** Ratified with it: the identity/persistence break
  (content-address leaf paths, borsh/serde bytes; one universe
  upgrades atomically; application-level `Key` migration out of
  scope); the text-cell denomination (κ-pinned, harder-not-softer);
  blanket authorization for C2's mechanical byte re-pins, reviewed
  bytes-only. Bookmark: `BOOKMARK_FORMAT_VERSION` 1→2, strict
  reject, no migration (no v1 files exist). Tier 1.5 (packed
  emission via parent-close scratch) was the evaluated alternative,
  not pursued; its design is in git history.
- **REFUTED 2026-07-23: the two-zone accumulator** (normalized big
  part + machine-word offset) — the wide-tooth comb drives it
  through its normalized prefix every tooth, quadratic [measured].
  Any two-zone design has a boundary the input can oscillate
  across. The balanced base-2^32 signed-digit form is the
  representation of record (`codec::accum`), flat on every family
  [measured, enforced].
- **REFUTED 2026-07-23: κ-only text judgment** — the chunked
  schoolbook slips under κ while quadratic; hence the two-legged
  criterion (§6) with committed tripwires.
- **DECIDED 2026-07-24: Base → dashu-int 0.5** (subquadratic radix
  both directions: parse e 1.49, render 1.51; num-bigint parses
  quadratically; ibig disqualified — no borrowed word access, no
  release since 2022). **The `Small(u64)` arm is deleted, not
  retained** (user ruling: dashu stores double-word values inline;
  the four-arm operator matrix was exactly the owned bug surface
  the swap exists to delete). `Base` is a thin metered newtype
  over `UBig`; limb counts derive from bit width
  (`bits.div_ceil(64).max(1)`), value-determined and
  target-invariant (wasm32 words pair).
- **RETIRED 2026-07-24: the num-bigint D&C parse PR** — parse is
  delegation to `UBig::from_str`; nothing local to upstream.
- **RETIRED 2026-07-24: the display canary**, under the dissolution
  ratchet (replacement red first): the complexity-class judgment
  lives in the bench judge's wide-display pair —
  `version_display_wide × hugeleaf` (honest D&C, e ≈ 1.47) and
  `display_schoolbook × hugeleaf` (the permanent rostered-red
  tripwire, e ≈ 1.99) — judged at the text ceiling 1.7, separation
  ≥ 2× the fit-noise band, laundering attacks pinned failing in
  `tools/benchjudge --self-test`.
- **RETIRED 2026-07-24: min-of-K wall hardening** — the board's
  wall judgment was deleted, not calibrated (user decision): the
  board reads no clock; the time leg lives in the bench judge on
  criterion's statistics.
- **Rank representation (2026-07-23/24)**: class-first `Ord` +
  `msb_cmp` ADOPTED and landed; float-style re-denomination, lazy
  unnormalized sums, and shared-exponent containers REJECTED
  (reasons in git history); the compact inline/spill form HELD
  until bulk-memory pressure is observed (spill canonicality would
  become load-bearing for Eq/Hash); a serialized Rank encoding
  DEFERRED — if ever minted, strict decode must reject
  non-normalized forms so byte equality keeps implying value
  equality.
- **Gate A (subadditivity) resolution policy (user ruling
  2026-07-23)**: the existing bound stays unless falsified;
  resolved GO — the lemma of record (§10).
- **GO-WITH-SHAPE 2026-07-24: boolean-skyline unification probe**
  (§17.5; the user's decision, post-C3).
- **DECIDED 2026-07-25 (Finch): the tick/fill cost effort runs
  spec-first under an adversarial design loop** — attack/fix
  rounds on the design document itself until convergence, a
  lateral-redesign fork on unsatisfying local optima. Performance
  within the campaign's bars decides the design; readability is a
  tie-breaker, never a veto. **Fused tick pre-approved** given
  linearity with small constants. Confer-with-Finch stop
  conditions: a superlinear honest optimum; a §6 denomination
  change; linear achievable only via an at-rest representation
  change. The spec of record: `design/before-tick-cost-spec.md`;
  its design-loop record (seven adversarial rounds plus the fusion
  landing) is that document's §9; Finch's ratification lands there
  as a dated amendment (the formal campaign's Phase 0 schedules
  the ratification read).
- **DECIDED 2026-07-26 (owner, at the fusion landing)**: no
  composed fill/compare/grow path retained — the differentials of
  record pin the fused `tick` directly to the recursive oracle —
  and no runtime byte-compare assert retained (committed
  differentials are the entire pin of the flag seam, per the
  standing practice that proptest coverage retires shadow-recompute
  asserts). Recorded with the landed shape at the spec's §6 and §9
  round 8; commit `80131954`.
- **RATIFIED 2026-07-26 (owner): the board protocol** (commits
  `cca70c01`, `61afb65b`): (1) the determinism tripwire — the
  runner's in-process double measurement plus the gate's
  cross-process byte-compare — replaces the
  two-identical-runs-per-scale convention; acceptance runs are
  single runs per scale. (2) **Release is the board's measurement
  of record** and the board recipes run `--release`: dev counters
  price algorithm plus verification scaffolding, release the
  production work alone; dev boards stay runnable as a debugging
  view, never the record; assertion-scoped meter suspension
  REJECTED on doctrine (a metering-pause mechanism is an F2
  hazard — do not build one). Rider: the ratified assertion-repair
  policy (fix the assertion's cost, never pause the meter) — the
  one exponent-class trigger repaired at `df3c1cb9`.
- **DECIDED 2026-07-26 (owner): rejection cost is total** — cost
  claims are unconditional over outcomes (rejecting an input is an
  outcome with a bounded cost), while the linearity-of-parties rule
  stays a *semantic* safety rule (nothing crashes if violated).
  Rejection rows denominate against the fed stream alone (§6); the
  rejection surface's enumeration and conventions are §13's.
- **DECIDED 2026-07-26 (C3, the pre-approved arm applied): the six
  plateau projection cells are `n_io`-denominated**, the
  O(`n_io`)-tightness rider measured and met (§3's closed entry;
  commit `1c32bb56`).
- **DECIDED 2026-07-26 (C3): the judgment layer's exponent guards**
  — exponent legs are fitted only where the cell's denominator pair
  scales (≥ ×1.5 between probes) and, on heap, where a reading
  clears the flat allowance the constant leg already forgives;
  unjudged exponents render `-.--` and the cell rides its constants
  and floors; guard tripwires committed (commit `87e82b34`).
- **AMENDED 2026-07-26 (C3, the κ re-derivation, commit
  `48c6f7b5`) — pending owner ratification (2026-07-26)**: the text
  limb constant leg's denominator gains the per-spelled-value
  pipeline term (`R = n_io + Σᵢ (digitsᵢ × limbsᵢ + 10)`,
  `TEXT_PIPELINE_LIMB_OPS_PER_VALUE`), and κ re-pins 0.25 → 0.75
  over it — the honest text cost law includes the delta⇄absolute
  pipeline's per-value arithmetic, measured 5–9 ops per spelled
  value on the production kernels. Rationale: without the term,
  small-value trees judge gamma-pipeline arithmetic against pure
  conversion work and read falsely red.
- **AMENDED 2026-07-26 (C3, the comb-scatter classification,
  commit `ce8f9e69`) — pending owner ratification (2026-07-26)**:
  flat-denominator shapes fit their exponents against value
  content (§6's rule); packed-byte fits on the shape manufacture
  e ≈ 4 from flat marginal work over the intercept premise.
  Rationale: the exponent leg must read the work's scaling axis,
  and the shape's packed bytes do not scale with it.

## 13. The metering gate

The board (`before::meter::board`, `just amp-board`, runner
`examples/amp_board.rs`): a red-green matrix over the entire
public operation surface × §2's families — **989 cells**,
membership pinned by the smoke test — judged at two scales
(default; `board::RECORD_SCALE` = ×4, `just amp-board-record`) at
the **release profile**, the measurement of record (§12's
ratification), from deterministic meters only: peak heap, grown
stacker segments, limb ops, scanned/written bits, and accumulator
digit touches, so every cell is six-column: verdict plus five
judged counter columns. The board is a generalized cartesian
product over three declarative axes: shapes declare operand
bundles, operations declare the slots their signatures consume,
and every judged quantity carries one field per metering currency
(`board::ByCurrency`), so every-shape-everywhere and
every-currency-everywhere hold structurally — adding a shape or
operation grows the product, and adding a currency is a compile
error until every operation declares a floor or a written NA for
it. **The board reads no clock**: its entire rendered output is
byte-identical at a given scale under any machine load, no
stripping, no carve-outs [measured — under a sustained
parallel-build load generator], and the claim is enforced on two
legs — the runner measures every cell twice in process and panics
on any counter disagreement, and the gate's
`just amp-board-determinism` byte-compares two cross-process
renders. Wall time is judged nowhere in the gate; the time leg
lives in the bench judge below, at `just bench-judge` / `just all`
cadence. Instruction-count asymptotics are the fuzz-fit harness's
territory (`crates/before/fuzzfit`, `just fuzzfit`: fuzzed
operation programs replayed under wasmtime fuel, deterministic and
load-independent, judged against pinned per-operation fuel bands);
its design record, `design/before-fuzzfit-asymptotics.md`, is the
instrument of record for that claim.

Ceilings: scaling exponent ≤ 1.15 (per cell, fitted across the
two scales against the cell's denominator bytes); heap ≤ 16 B per
denominator byte over an 8 KiB flat allowance; grown segments
≤ 1; limb ≤ 128 ops/byte on input-denominated rows; the text rows
per §6 (κ constant leg + n_io exponent leg); scan ≤ 96 bits/byte
on walk rows; touch ≤ 96 digit touches/byte (calibrated at
release: heaviest honest reader the mirror-narrow tick cross at
30.8/B default, 30.9/B record — scan's own margin convention).
Exponent legs are fitted only where the cell's denominator pair
scales (≥ ×1.5 between probes) and, on heap, where a reading
clears the flat allowance the constant leg already forgives; an
unjudged exponent renders `-.--` and the cell rides its constants
and floors (§12's judgment-layer decision; guard tripwires
committed in `meter::board::tests` — the same readings must read
red the moment the denominator honestly doubles or the readings
clear the allowance). Green = all columns within ceilings AND all
floors met.

**Liveness floors** (user ruling 2026-07-24: the board judges the
API surface as well as the implementation — a ceiling over a dead
counter proves nothing). Every cell carries a floor-or-NA
declaration per judged column, demanded by the `Cell` type and
rendered as a legend; a floor trip is red with the mechanism named
("counter reads below floor: the meter is not watching this
work"). Conventions of record: scan floors 1 bit per packed
operand byte on every row that must examine its operands
(early-exit rows floor at 2 bits — the root codes); limb floors
where big-integer arithmetic is semantically mandatory, at two
derivations (the same split the touch floors carry): rows that
read the stored form as-is (decode, rank/distance/lag, tick)
floor at one op per 64 bits of every stored payload *code* wider
than 128 bits — a plateau of equal wide leaves stores its width
once, so a tree-derived floor demands limb work no conforming
walk does — and the value-materializing parse rows floor at one
op per 64 bits of every stored *base* wider than 128 bits
(conversion must materialize every spelled value); heap floors on
codec and text rows (the result materializes at least its packed
bytes), plus the fork rows' deterministic-liveness child-copy
floors (fork builds both halves, so the generic in-place NA would
misstate it); touch floors at two deterministic-liveness
derivations — one touch per stored delta on the delta-folding
kernels (sweep, emit, query folds, tick, parse: the envelope
suite's committed one-per-delta convention), one touch per 64 bits
of every stored wide code on the decode rows (the validator
legitimately batches word-scale deltas in the accumulator's lazy
zone, so a per-delta floor over-demands there — the stream-derived
convention, deliberately NOT the tree-derived one); segments
ceiling-only (its honest floor is zero). NA genres: wholesale byte
moves (encode, hash, and — by byte-decided equality —
`version_eq`, whose exposure sentence and time-leg backstop are on
the board face), operands with no packed stream, empty forms. A
floor trip is a designed stop-and-look; an implementation that
legitimately does less work lowers the floor deliberately. Floors
have caught three live regressions to date (the id-renderer scan
vacuity; main's unmetered window fast paths; the byte-decided
equality) — the instrument works.

Scan-meter contract notes in force: the gamma window fast paths
record the same `2k+1` bits the per-bit loop prices (fast and
slow paths meter identically); the wire-side borsh `ReaderCursor`
is deliberately unmetered (no board row prices the wire path;
`codec/scan.rs` states so; instrumenting it is a conscious future
change with its own recalibration); the `max_depth` caller-side
record double-counts uniformly (2×, deterministic) and carries a
`TODO-recalibrate` for its own future commit that must re-measure
the pins pricing that walk.

Tripwires (every criterion demonstrates the status quo fails it):
`bypassing_walk_is_green_under_ceilings_alone_and_red_under_floors`
(committed, `meter/board/tests.rs`); the κ pair (§6); the judge's
unmetered-quadratic bench (`benches/tripwire.rs`,
`just bench-judge-tripwire`, `--expect-red`, e = 2.00 measured)
plus its deterministic twin in `tools/benchjudge --self-test`,
run at the head of every judge recipe.

Dashboard caveats of record: the board shares one process, so its
heap numbers are indicative and the process-isolated envelopes in
`tests/meter.rs` remain the enforced record; segment counts have
a ~1 MiB growth threshold, so the default scale under-detects
segment onset — which is why acceptance runs at ×4 too. One
recorded non-monotone verdict genre: a flat per-byte ceiling
against an n·log n constant can read red at default and green at
×4 (`party_join_all × scatter` era) — record-scale greenness
never clears a default red. Record-scale runtime budget: ≤ 30 s
summed measured-body wall per family.

### The rejection surface (fallible operations, enumerated 2026-07-26)

Cost claims are total: rejecting an input is an outcome with a
cost, bounded like any other, whether or not the caller honored
the usage invariants (§12's rejection-cost decision). The board's
18 rejection rows (269 cells) measure the rejection side under all
five currencies, with the defect **maximally deferred** in every
committed shape — an early-exit-only measurement is the cheapest
artifact that would pass, so every shape places its defect where
rejection must consume as much input as possible:

- **Overlap** (`Party::join`, `Clock::join`, `Clock::sync` —
  `Err` on non-disjoint parties): the `party_join_overlap`/
  `clock_join_overlap`/`clock_sync_overlap` rows, over the
  overlap-mount adapter (the disjoint-mount adapter's
  counterpart): `a` = the shape mounted left plus a marker chain
  along the shape's rightmost-present path, `b` = the shape
  mounted right, so the pair's one overlapping position is the
  shape's preorder-last terminal — the last position the lockstep
  walk reaches, with every earlier region disjoint. Adapter
  outputs are semantically void by design (a well-formed pair no
  legal fork/join history produces); the cost claim is what the
  rows price. Clock overlap rejection does no version work (the
  party join is the gate; `clock/batch.rs`).
- **Overlap hand-back in the folds** (`Party::join_all` —
  `Err(Vec)` returning every overlapping input): the
  `party_join_all_overlap` row — one large mounted accumulator
  (the adapter's a-mount: the shape left, the marker right), many
  one-byte right-full probes each overlapping the accumulator's
  right half *behind* its whole left shape, all handed back. The
  witnessing pair sits past the left shape, so a per-input test
  priced in the accumulator (a cursor walk skip-scanning the left
  shape per probe — the coding has no random access) reads
  Θ(accumulator) per O(1)-byte input and the row goes red; the
  fold's per-call accumulator index (§3's landed cure) answers
  each test in O(probe), which is the separation the row watches.
  `Clock::join_all` runs the identical up-front indexed test
  against self inline, so the party row prices both (delegation,
  the board doc's NA list).
- **Empty difference** (`Party::without` → `None` when `other`
  covers `self`): the `party_without_none` row, identical-region
  operands, so the diff walks both streams in full and the empty
  result is known only at the end.
- **Strict decode** (`Version`/`Party`/`Clock::decode` —
  `Decode`): `*_decode_truncated` (the last byte dropped: a
  strict prefix of a preorder stream has an open subtree at every
  earlier position, so EOF is discoverable only at the end),
  `*_decode_trailing` (a `0xFF` byte appended after the complete
  valid stream), for all three types; `version_decode_noncanon`
  (the preorder-last leaf split into an equal-sibling pair — a
  zero right-sibling delta, the minimality violation the
  validator can only see at that pair's close, the stream's last
  position) and `party_decode_noncanon` (the preorder-last
  terminal split into a collapsible `(1, 1)`, same argument).
  Clock non-canonicality is the component validators on the same
  streams (delegation). Not rowed, with reasons:
  `Decode::Anonymous` (the accepting parse of the empty stream —
  a zero-byte operand, no adversarial scaling axis);
  `Decode::Io` (the caller's reader; a failing reader is a
  truncation carrying an error, priced by the truncated rows);
  other non-canonicality genres (a delta driving the running
  height negative, nonzero padding) ride the same single
  validator pass at the same full-parse cost — the committed
  tails are the maximally-deferred representatives.
- **Text parse** (`FromStr` for all three — `Parse`):
  `version_parse_trailing`/`party_parse_trailing`/
  `clock_parse_trailing` (junk after the complete valid text; the
  clock's junk sits inside the outer parens so the version
  component parses in full first — the clock parser's outer-paren
  check rejects appended junk in O(1), an early exit the row
  deliberately avoids) and `version_parse_noncanon` (the last
  spelled value `t` re-spelled `(0, t, t)`: equal sibling leaves,
  judged at that node's close, the text's end) /
  `party_parse_noncanon` (the last `1` re-spelled `(1, 1)`).
  The P3.8 text round priced the *accepting* direction (κ, the
  delegating-parser and schoolbook pins, the metered id renderer);
  these rows extend it to the rejecting direction and duplicate
  none of it. `Parse::Anonymous` ("0") is a word-scale input —
  not rowed. Clock text non-canonicality: component delegation.
- **Not rowed, bounded or delegated**: `Version::meet_all` →
  `None` on an empty iterator (no operand); the `TryFrom`
  literal forms (`u8`/`bool`/tuples/`u64`/`(I, E)` — word-scale
  or type-bounded operands); `encode_to`'s `io::Error` (the
  caller's writer: at most the encode row's work before the
  error propagates); `Rank::checked_sub` → `None` (measured on
  the `rank_pair_ops` row, which attempts both directions);
  serde/borsh deserialize errors (the strict decoder through the
  wrappers — the decode rejection rows; the borsh wire cursor
  rides the standing unmetered-wire note above).

Rejection-row conventions: **denomination** — a rejection produces
no output, so every rejection row denominates against the fed
stream alone (packed bytes, or text bytes on the parse rows; §6).
**Floors** — packed-stream rejection rows floor scan at one bit
per fed byte with the defect-placement derivation (a
self-delimiting stream's terminal defect is only discoverable by
parsing to it; the overlap rows' witnessing position sits at both
operands' stream ends and the packed coding has no random access);
heap, limb, and touch are NA on rejection rows — rejection
materializes no result and forces neither value work nor an
accumulator fold (a validator may defer both past the topology
walk that finds the defect). Text-rejection rows declare no floor
on any column, by honest derivation: no deterministic counter
watches text-byte consumption, and a parser may find the defect in
tokenization before any packed or value work — their ceilings
judge live readings (the parsers do metered work greedily) and the
time leg times them like every row.

**The bench judge** (`tools/benchjudge`, stdlib Python;
`benches/board.rs` driven by the board's own cell table so bench
IDs mirror board cells by construction — the pinned mode times
290 cells: the 288 designed-pairing board cells derived by rule
from the axes (`board::BenchMode::Pinned`: each shape's
designed-stress groups, the organic control, and the board-red
riders; count verified against the criterion `--list`) plus the
wide-display pair; `BOARD_BENCH_MODE=full`,
`just bench-judge-full`, times the whole 989-cell product plus
the pair for final verdicts): fits each cell's
wall exponent `ln(median_hi/median_lo) / ln(denom_hi/denom_lo)`
across two saved criterion baselines (scales 1 and record),
denominated against the board's per-cell denominator bytes (never
the scale knob), judging every cell whose hi median reaches the
resolution-derived 10 µs floor. Ceilings ride the **sidecar**,
never the roster: bench code declares each cell's ceiling class
at its definition site (`benches/common/sidecar.rs`: general 1.3,
text 1.7; `TEXT_CEILING_CELLS` pinned = exactly the wide-display
pair), the judge cross-checks the two sidecars per cell, exit 2
on disagreement. Sidecars are stamped (scale, profile `optimized`,
sampling, git tip) and cross-checked against each other and
`--tip` — stale or mismatched baselines refuse. Exit contract
pinned end-to-end in `--self-test`: 0 all-green, 1 per red /
dark tripwire / roster violation, 2 per input error (nonpositive,
NaN, missing medians are input errors, never skips or verdicts).
Sub-floor cells are SKIPped and listed (documenting cheapness);
an unfloored lo median is deliberate, and the fit-noise band
(≈ 0.052 at the 1.3 ceiling, ≈ 0.088 at 1.7; derivation in the
tool) bounds resolution's pull — the roster's `boundary` class
accepts either verdict only within that band of the cell's own
ceiling.

**The expected-red roster** (`tools/benchjudge-expected.json`):
membership by cell name, expectations only (`red`: must be judged
and read RED at the cell's own ceiling — GREEN is a
verdict-flip/liveness signal and SKIP a drift out of judgment,
both exit 1; `boundary`: within the band). Any unrostered red
fails. Membership and the text-class set are pinned by
`crates/before/tests/bench_judge_roster.rs`, so every edit trips
a reviewed diff. `bench-judge-record` (full sampling, the mode
for numbers of record) judges through the same roster — the
expectations are exponent classes, valid under either sampling
regime. Population at this tip: the fifteen bigroot sweeps + the
hugeleaf display pair (κ/C2-owned) + the permanent schoolbook
tripwire; boundary empty. **The bigroot set and the display pair
empty at C3's item 11; the schoolbook expectation is permanent.**
Between C2 and that realization every judge run fails on the
fifteen realized greens BY DESIGN — that failure is C3's
realization evidence, banked verbatim at the flip (e 0.94–1.00
fitted on all fifteen).

**Numbers of record at this tip** [measured 2026-07-26; release
profile, single runs per scale under the determinism tripwire —
the `board-unify24-{lo,hi}.txt` renders]: board **966 green / 23 red at
the default scale; 954 / 35 at ×4** over **989 cells**. The red
roster, every red with exactly one owner, is §17.3; the
cell-count and verdict lineage across the campaign's rounds
(200 → 989) is in git history at the commits §14 names.

**Acceptance (the campaign's; protocol per §12's ratification):
all-green means the release-profile board green on counters and
floors at BOTH scales, one run each under the committed
determinism tripwire (the runner's in-process double measurement
plus the gate's cross-process byte-compare), AND the bench judge
roster-satisfied at both scales in both modes** — at P5.5 with the
bigroot set emptied and only the permanent text expectations
remaining. A release record-scale run costs ~20 s wall [measured —
the ratification baseline runs]; dev runs remain a debugging view
and never satisfy acceptance.

## 14. Execution plan

Completed-phase ledger (narratives in git history at the named
commits):

- **P0** (2026-07-22): generators + meters + board landed; current
  envelopes pinned as thresholds; board born 59/99.
- **P1** (2026-07-23): Tier 0 — limb-wise wide-gamma decode,
  Builder push-growth, iterative complement. Board 96/62.
- **P2** (2026-07-23): the decision packet — compactness ratio
  measured, carry-cliff genre and blast radius priced, DECIDED
  entry recorded (§12).
- **P3.2–P3.8 pre-flip window** (2026-07-23/24): the accumulator
  of record + generator families; criterion hardenings (κ,
  RECORD_SCALE); skyline codec + validator + Gate A GO; sweep and
  emit kernels + unified builder; query folds + freeze-discipline
  cure; grow (bit-coded probe + splice emit); dual-oracle coverage
  audit (gap list EMPTY); text kernels + parse delegation +
  metered id renderer; the bench mandate (`benches/board.rs`) and
  the bench judge + roster; surface-judgment floors; P4.1 id
  walks (segments → 0); dashu swap; canary retirement; RNG
  consolidation. Board 137/63 and 128/72 at the window's seal.
- **C0** (2026-07-24, `1e96e6fd`): rebase onto the link-transport
  merge; the sixteen-test stall roster RETIRED (sweep 1183/1183;
  the committed stall seeds are link-transport's regression pins;
  any test failure anywhere blocks again, no provenance
  carve-out); the merge-seam re-sweep and the meter-coverage fix
  round (main's unmetered fast paths under the campaign's floors;
  five-cell vacuity catch cured; `version_eq` re-denominated).
  Board 139/61 and 130/70.
- **C2** (2026-07-25, `91fac33d`; fill kernel `c43740b8`;
  seed-corpus cure `61d1bcd4`): the flag day — storage flipped,
  every operation routed to the kernels, old codec deleted, 27
  snapshots re-pinned (bytes-only review: zero blocking),
  `BOOKMARK_FORMAT_VERSION` 1→2 with a reject test, byte-pinned
  doctests re-run. Board 185/15 and 184/16 over 200 cells; 49
  staged kills realized, zero unexplained movement; the judge's
  fifteen realization greens banked (the judge's last honest
  pre-realization reading, at the flip commit over 202 cells:
  157 green / 3 red / 42 sub-floor, exit 1 on exactly the fifteen
  banked violations). Workspace sweep at C0: 1183/1183, roster
  retired, unqualified green since.
- **Bench-coverage integration** (2026-07-25, `1312fba6`..
  `10232626`): the coverage branch merged (rustdoc-JSON API index,
  `benches/COVERAGE.md` — the census's living home; `rank_sum`
  row + fold benign controls; NA-prose repairs), re-adjudicated
  against the flipped tree. Board 188/17 and 187/18 over 205.
- **Post-flip fix round** (2026-07-25, `606e8f54`..`72f4a780`):
  every flip-review finding addressed — real oracle witnesses
  restored, the D2 window fast path restored on the flipped hot
  paths (board byte-identical proof), the `clippy-default` gate
  recipe (nine warnings caught on landing), scan double-records
  fixed with measured splits, fuzz seeds consumed, snapshot
  hexes corrected from measurement. Gate green.
- **The fill linearization** (2026-07-25, red pin `36c9339b` +
  cure `92d2fc31`): the nested-full-sibling family pinned the
  re-scan genres quadratic (scan/limb e 2.00), the cure flipped
  them (scan e 1.00 both arms, deep-4096 differential 67.4 s →
  1.3 s); board 190/17 default, 187/20 record over 207. Its
  adversarial review then refuted the limb-dimension O(n+m)
  claim (§3's re-touching entry) — the finding that opened #34
  and the tick cost spec's design loop. Acceptance deviation of
  record (2026-07-25): the ×4 segments leg stayed red,
  P4.2-owned — the linear acceptance was met on the work columns.
- **#34: the tick limb cure and the fusion** (2026-07-25..26,
  red pin `dc9a2c31`; the anchor-web walk `12b5e9a3` + the
  chained-memo pre-scan `39009918`; the frame ledger `4934db86` +
  `952159f7`; the latent boundary register `43d625e7` +
  `12a85d2f`; the fold-direction cure `b31ca059` (red pin
  `8fdba3d4`..`b0136781`, flip `5f40eec3`); the fused tick
  `80131954` with the flag width pin `74d6ec5e`): T-tick —
  amortized O(n + m) Accum digit touches — realized inside the
  fused tick; every #34-owned red flipped at both scales, and the
  fusion moved 51 tick-row constants per scale with zero verdict
  flips. The design loop's record is the spec's §9 (rounds 1–8).
- **The #35 board product refactor** (2026-07-26, `a4a233ae`..
  `cc4762c4`, ratified `cca70c01`/`61afb65b`): the three-axis
  product (bundles × slots × currencies), 225 → 720 cells, every
  pre-existing cell byte-identical; the determinism tripwire and
  the release profile of record ratified (§12); the
  assertion-repair policy's one trigger fixed (`df3c1cb9`).
- **The #40 representation migration** (2026-07-26, `8181247a` +
  `cbd52a94`): Version's at-rest form is `codec::Bits` (the wire
  bytes in a length-carrying container) with byte-level Eq/Hash;
  heap and record-scale segment readings moved down on 41/45
  cells, flipping three owned reds green.
- **The #39 instrumentation ratchet** (2026-07-26, `da80fa27` +
  `7b7f35d0`): the touch currency joined the board as the fifth
  judged column; the census's pin ratchets landed (§3's entry);
  zero verdict changes from the column itself.
- **The error-path round** (2026-07-26, `51509259`, red pin
  `90e88144`): the rejection surface joined the board (18 rows,
  269 cells, 720 → 989); the join_all re-scan found and
  red-pinned.
- **The join_all cure** (2026-07-26, `5aabc765` + `b71e146e`):
  the per-call `IdIndex`; twelve reds flipped per scale;
  `party_join_all × scatter` re-attributed green at default.
- **Orbit pins (#38)** (2026-07-26, `0ac28993`): six deterministic
  size-trajectory pins; no cell moved.
- **C3's denomination and classification round** (2026-07-26,
  `ab2cd0c4`, `ce8f9e69`, `1c32bb56`, `48c6f7b5`, `87e82b34`;
  reconciliation `e74275ce`): queue items 1–10 done — stream-
  derived limb floors, the content-exponent denominator, the
  plateau `n_io` ruling, the κ re-derivation, the judgment-layer
  guards; 75 (default) / 78 (×4) reds flipped green, zero flipped
  red, every flip bucketed by mechanism; sums 966 + 23 / 951 + 38
  over 989. The round opened the materializing-emitters item
  (§17.2).
- **The fuzz-fit harness** (2026-07-26, merged `a91bd97b`):
  instruction-count asymptotics instrument; its record is
  `design/before-fuzzfit-asymptotics.md`.
- **#24, the boolean-skyline unification** (2026-07-26,
  `447f6985`, `2cd73716`): id `diff` converted to the sweep, its
  depth recursion dissolved; `party_without_none × id-pair` ×4
  flipped green (54 grown segments → 0), the order-coupled tick
  and parser segment counts re-rolled downward (work columns
  byte-identical; two tick × nested-wide cells read green at a
  flat count), converted-cell scan/heap constants re-metered
  green→green; sums 966 + 23 / 954 + 35 over 989. Landed shape
  and the predicate-leg reversal: §17.5.

Remaining plan: **C3's bench-harness remainder** (§17.2: the
judge-roster realization, envelope tightening at P5.1, the
before/after table) → **the materializing emitters** (§17.2) →
**P4.2** residual audit → **P5.1–P5.5** closeout, with the
boolean-skyline decision (#24, the user's) after C3.

Acceptance for the effort: the §13 acceptance criterion; plus the
two 2026-07-25 user rulings:

**The per-operation performance bar** (for C3's before/after
table and P5's acceptance): strict absolute improvement
everywhere is the target and the expectation; the floor per
operation is **at or close to parity on benign inputs,
asymptotically optimal, and entirely free of adversarial
exploitation surface**. A benign cell slightly slower under an
adversarial mitigation's constant is triaged hard for a cure but
does not block; anything below the floor blocks.

**The asymptotic bar**: **every public operation must be
subquadratic in its total input, worst case; linear is the
ideal.** Fundamentally-superlinear problems (radix conversion,
multiplication-equivalent up to log factors; n log n
comparison-ordered n-way folds) satisfy the bar at their
problem's own optimum, stated and priced. The tick kernel's limb
dimension sits at amortized O(n + m), realized inside the fused
tick (the tick cost spec's T-tick).

## 17. Work items of record

### 17.2 Open items, with acceptance contracts

**C3's bench-harness remainder (the queue of record, items
11–13).** Items 1–10 of the round's queue are done (§14's C3
entry; the cell-exact movement is §17.3). Sequenced next, in
order:

11. **The judge-roster realization**: the fifteen bigroot
    expectations leave on the banked evidence (fitted e 0.94–1.00
    at the flip); the hugeleaf display pair resolves with the κ
    re-derivation — its ceiling class stays **text 1.7**,
    unchanged deliberately: radix conversion is fundamentally
    superlinear (honest D&C measured wall e ≈ 1.47 over the
    general 1.3), so the pair's class is the text ceiling on the
    merits; the schoolbook tripwire stays permanently. Roster
    membership pin updated in the same change, and the
    `BOARD_RED_BENCH_RIDERS` population rides this diff (riders
    and roster edits are one judge-verified change; the riders are
    committed empty until then — an unrostered red whose time leg
    reads red fails every judge run). `bench-judge` and
    `bench-judge-record` must then exit 0 — roster-satisfied — at
    both scales in both modes, with full-sampling record numbers
    captured.
12. **Envelope tightening** (the twelve event-side rows:
    DECODE/CMP/JOIN × DENSE/BIGROOT/HUGELEAF/CLIFF as applicable,
    TICK_DENSE): deferred to P5.1's envelope finalization, one
    downward re-pin from post-C3 stable readings instead of two.
13. **The before/after table of record** (judged under §14's two
    rulings). Protocol, mandatory: re-bench the pre-C2 tip under
    the FINAL harness — a temp worktree at the last pre-flip
    commit with the current bench files grafted on (they call only
    the public API), full sampling both tips, warm target dirs —
    **never the stored `base` baselines**, which are contaminated
    for delta purposes (the RNG consolidation regenerated every
    bench input family, and they mix sampling modes). Any benign
    regression beyond "slight" is a finding; the parity floor is
    the bar.

*Risk*: a cell green at default but red at record — that is the
two-scale design working; the cell's owner reopens. Adjacent but
not C3's: #24 (the user's decision, post-C3) and the
stack-container measured phase (§17.5, C3-adjacent, its own
harness).

**The materializing emitters (opened at C3, 2026-07-26).**
Three C3-classified residuals share one seam — the emitters that
materialize before they write — and its cure round owns them
(§17.3's roster carries the cells and readings):
- The render's finalize pass materializes a per-node `Base`
  vector, digit arena, and offset table before emit: the display
  heap constants (14 cells per scale, 17.4–33.2 B per `n_io`
  byte, linear). Candidate cure: convert bases to digits at
  their close (dropping the `Base` vector) or a two-pass
  finalize — either priced against κ's pipeline budget before
  landing.
- The render merge re-folds wide relative summaries per ancestor
  on wide×deep right-spine shapes (mirror-wide display, limb
  e 1.81 at ×4): the #34 re-touching genre alive in the render
  walk; the anchor-web discipline is the candidate cure. Parse is
  linear on the same shape, so the text format is not the
  obstruction.
- The projection output builder's growth transient makes peak
  heap capacity-phase-dependent (the projection × comb-scatter
  default-scale heap exponent): a finalization/shrink discipline
  stabilizes the measured quantity — the doctrine's "feed the
  threshold stable inputs" arm, kernel-side.
*Acceptance*: the cells above flip green at both scales with
byte-identity across the differential suite; movement annotated
against the parent boards; any κ movement re-derived at the
constant.

**The fold marginals — the n-cursor merge (C2-adjacent).** The
V7 reduction's n·log n reads marginally red against flat ceilings
on the fold cells (§17.3's fold-marginals genre). Candidate cure:
an n-cursor merge replacing the binary-counter reduction's
re-comparisons. *Acceptance*: the owned cells flip or the n·log n
optimum is recorded as the problem's own (the asymptotic bar's
fundamentally-superlinear clause) with the ceiling re-derived.

**P4.2 — residual recursion and word-scale scanning.**
Audit every remaining `recurse::descend!` site post-C2; convert
survivors per the explicit-stack pattern or record why they stay;
apply the word-at-a-time subtree skip (popcount pending-counter
delta, mid-word zero-crossing exit) to `idbits` and the skyline
topology stream where benches justify. Triage convention for the
genre's segments currency: the counter reads the stacker's
process-global segment cache, which is order-coupled to the
preceding cells' stack usage in the shared board process, so a
kernel change anywhere in the binary can re-roll the counts on
untouched kernels' rows — segment-count movement on a row whose
work columns are byte-identical is triaged as order coupling, not
kernel movement; the counts of record are §17.3's. *Sequencing,
resolved at #24 (2026-07-26)*: the id predicates stay on the
lockstep walk (§17.5), which is exactly the shape the word-scale
skip fits; `diff`'s sweep enumerates leaves and never skips, so
the skip does not touch it. The skip also interacts with the
fused tick's route fold, which reads each skipped id subtree per
2-bit tag on leaf-under-internal-id arms — P4.2's sequencing
decision must name the fused walk, and the spec's §9 round-8
table carries the landed interaction baseline. *Kills*: none
(constants). *Acceptance*: the audit list recorded; benches.

**P5.1 — envelope finalization**: every `tests/meter.rs` envelope
and board ceiling tightened to final constants at record scale
(the board's constants at release, single runs under the
determinism tripwire; the envelope suite's in its own dev-run
process-isolated harness, where its pins live);
`ID_WITHOUT`'s final ratchet (the one row
no earlier item re-pins).

**P5.2 — proportional fuzz cap**: counting-allocator harness with
a hard ceiling proportional to input size across all fuzz
targets; the seed-writer + canonicity check join `just all`.

**P5.3 — stacker-removal audit**: if P4.2 shows zero remaining
library-path depth recursion, drop `recurse::descend!` and
`stacker`; re-denominate `clock::tests::deep_tree_stack_safety`
and update the crate's AGENTS.md hard rule in the same change;
else record which sites stay and why.

**P5.4 — documentation closeout (user sign-off, item by item)**:
the §6 invariant statement lands in the crate docs now that it is
true — over content bits for content-materializing operations,
packed operands for delta-native ones, the denomination stated as
contract, every cost claim carrying its epistemic status; the
`Key` stability promise in `rumors`' `src/tree/key.rs` gains its
same-code-version qualifier; the bookmark version-mismatch
semantics; `before`'s crate-doc Efficiency section re-measured
under skyline with `just readme` re-derivation; the prose
improvement pass (the frozen-docs slot).

**P5.5 — acceptance sweep of record**: `just all` clean; the §13
acceptance criterion met in full (board all-green both scales
under the ratified single-run protocol; judge roster-satisfied
both scales both modes, only permanent expectations); the
before/after table showing the parity floor met everywhere and
improvement where claimed; the coverage audit re-run with an
empty gap list (method: walk the board's op enumeration and the
board-doc NA list; every public operation names its two oracle
legs and its resource pin — the representation-pin leg per the
2026-07-23 directive: every exposed type's bytes/text/serde forms
snapshot-pinned in-crate); the benign rank-pair operand scaling
if C3 chose that arm; the §14 acceptance entry recorded.

### 17.3 Owned-red accounting (current; over 989 cells)

Sums [measured 2026-07-26, the `board-unify24-{lo,hi}.txt` renders]:
**default 966 + 23 = 989; record 954 + 35 = 989.** Every red has
exactly one owner and the sums close; the per-round movement
lineage (each round's flips, bucketed by mechanism, with every
untouched cell verified byte-identical) is in git history at the
commits §14 names.

The red roster, both scales enumerated from the renders:

- **The render materialization genre** (14 default / 14 ×4 heap
  constants: `version_display` × {dense, bigroot, harmonic,
  nested-full, nested-wide, mirror-wide, mirror-narrow,
  staircase}, `clock_display` × {dense, bigroot, harmonic,
  nested-full, mirror-narrow, staircase} — 17.4–33.2 B per `n_io`
  byte at exponent ~1.00): the render's finalize pass
  materializes a per-node `Base` vector, a digit arena, and an
  offset table before the emit pass — linear, honestly over the
  16 B/B ceiling. Owner: **the materializing emitters item
  (§17.2)**.
- **The render merge's wide-summary re-read**
  (`version_display`/`clock_display` × mirror-wide: limb
  exponent 1.55–1.56 default / 1.81 ×4, the ×4 constants over κ;
  `version_display`'s heap constant rides the genre above):
  the finalize merge re-folds wide relative summaries per
  ancestor on the wide×deep right-spine shape — the #34
  re-touching genre alive in the render walk; parse on the same
  shape is linear (6.05 ops/node [measured]), so the text is not
  the obstruction. A genuine kernel superlinearity, red on an
  honest denominator. Owner: **the materializing emitters item**.
- **The capacity-phase heap exponent** (`version_project`/
  `clock_own_version` × comb-scatter, default scale only): work
  I/O-linear on every column; peak heap linear in the output with
  an allocator/buffer-doubling phase (scratch-to-output measured
  0.67–1.84 across four probe points, non-monotone; the two-point
  fit reads e 1.38 at default and 0.70 at ×4). The instrument
  wobbles, not the kernel — but the reading is honest and stays
  red rather than softening the ceiling. Owner: **the
  materializing emitters item** (builder finalization/shrink
  discipline stabilizes the measured quantity).
- **The fold marginals** (default: `version_join_all` ×
  {scatter, benign} limb/scan exponents, `party_join_all ×
  benign` scan constant at 100.1 bits/B; ×4: `party_join_all ×
  benign` only, 116.9 bits/B — the version-side pair reads green
  there, the n·log n signature): owner **the n-cursor merge item
  (§17.2)**.
- **`version_min_ticks` heap constants** (mirror-narrow at both
  scales, mirror-wide joining at ×4): the query walk's per-level
  owned heap entries on the deep left-full memo shapes — linear
  in site count, a constant the ceiling honestly rejects; the
  same constant genre as the tick memo's diff-coded record (the
  spec's §9 round 4). Owner: an accepted stated-band residual
  with the diff-coded memo as the candidate cure if one is ever
  warranted.
- **The ascending-cliff tick pair's heap constants** (63.0/65.3
  B/B, e 1.00 — the one committed family holding k
  simultaneously-armed nonzero boundary differences, one pooled
  unit-width buffer each; linear in the input, honestly over the
  16 B/B ceiling): owner: the spec's §9 round-7 record — a
  small-buffer diff-stack representation is the candidate cure if
  one is ever warranted. The constant is specific to the board's
  joint (s,s) axis, where the armed count k and the
  per-difference width b grow together: at fixed b the peak heap
  is exactly affine in k (91.3 B per armed nonzero difference
  plus a 2,905 B floor) and the B/B reading drifts with the k:b
  mix while the exponent holds 1.00. A future re-pin of this cell
  must re-measure on the board's own axis; the exponent claim is
  axis-invariant, the constant is not.
- **The ×4 segments legs** (17 cells: `version_tick`/`clock_tick`
  × {nested-full, mirror-narrow, staircase, pure-comb,
  ascend-cliff, ascend-plateau}, and the id-side parser cells
  `party_from_str`/`clock_from_str`/`party_parse_trailing`/
  `party_parse_noncanon`/`clock_parse_trailing` × id-pair — the
  id set-algebra side is clear: `diff` runs as the #24
  boolean-skyline sweep (§17.5), so the id-side residue is
  parser-only): owner **P4.2**, the recursion-depth genre
  (profile-dependent by codegen and order-coupled across the
  shared board process — the triage convention at P4.2's entry;
  the #24 landing re-rolled the untouched tick and parser counts
  downward with work columns byte-identical, and `version_tick`/
  `clock_tick` × nested-wide re-rolled to a flat 1 and read
  green). Counts of record at this tip, per tick op: nested-full
  7, mirror-narrow 7, staircase 14, pure-comb 2, ascend-cliff 2,
  ascend-plateau 2; the parser cells 12/12/12/6/12.

Bench riders (`BOARD_RED_BENCH_RIDERS`) are committed empty: the
surviving reds are classified above, but a rider outside the
judge's expected-red roster whose time leg reads red fails every
judge run, so the rider population and the roster edit are one
reviewed diff — item 11's (§17.2).

### 17.5 Post-campaign docket (user directives)

- **#24, the boolean-skyline unification (the user's decision,
  post-C3; probe verdict GO-WITH-SHAPE 2026-07-24; landed
  2026-07-26, `2cd73716`, with the construction/predicate split
  reversed).** Under the skyline coding a `Party` is a boolean
  skyline. Landed shape: `diff` rides the sweep — an id leaf
  cursor per operand presenting absent children as synthetic
  unowned plateaus, the event sweep's advance/tie rule
  transferred verbatim (no boolean carve-out; the boolean side
  needs no accumulator, each cursor's one owned bit replaces the
  running difference), one output plateau per elementary interval
  into a leaf-driven collapsing id builder (positions on a
  delta-coded bit stack) — dissolving the id family's one depth
  recursion, its `descend!` guard, and the two-pass complement
  retagging. `party_without_none × id-pair` ×4: 54 grown
  segments → 0; the id-side segments residue is parser-only
  (§17.3). The 2026-07-24 shape's predicate leg is reversed on
  cost evidence: `covers`/`is_disjoint` stay on the lockstep —
  a verdict-only walk carries no per-level state (the pending
  stack queues nothing on unary chains; `ID_COVERS`/`ID_DISJOINT`
  pin ~zero transient heap), where a leaf-enumerating sweep pays
  two path-bit stacks per operand depth for interval geometry a
  predicate never reads — so that conversion would multiply a
  pinned near-zero envelope to retire no red. `sum` stays on its
  frames walk (copy-splice has no sweep analogue), as recorded
  in 2026-07-24's verdict. Converted-cell re-metering (scan
  constants moved both directions at exponent 1.00: each input
  tag now read exactly once where the structural walk peeked and
  read; output written reserve-and-patch where it spliced; heap
  up ≤ 7.4 B/B, the recursion state relocated to metered bit
  stacks — the `ID_JOIN` precedent class) is enumerated in the
  #24 board diff; `id_without`'s envelope re-measured
  byte-identical (518 219 B peak, 0 segments).
- **Extract the accumulator as a workspace crate** (unpublished
  until a second consumer stabilizes the API; its amortization
  contract is subtle — reads mutate).
- **Stack-container decision: `SmallVec<[T; N]>` vs
  `Vec::with_capacity(N)` vs `Vec::new()`, measured, per site**
  (user directive 2026-07-25). It is not a priori clear that
  smallvec's constants beat Vec's: the inline path saves one small
  call-scoped allocation (tens of ns on a thread-cached allocator,
  paid once per op) but pays a per-access discriminant branch
  (well-predicted when never spilling, yet real in code size and
  inlining pressure), a fatter struct (locality when embedded or
  moved), and a spill memcpy at exactly the deep input. Nor is
  pre-sizing a free improvement over starting empty: allocator
  size classes mean `with_capacity(N)` at a defensive bound rounds
  into a larger class whose slack Vec does not adopt (capacity
  stays as requested — the slack is pure waste), while an organic
  `Vec::new()` on a shallow walk makes exactly one small-class
  allocation and never runs the doubling chain. "Capacity known in
  advance" splits into known-per-input (pre-size to the exact
  value) vs known-as-typical-bound (pre-sizing may lose to
  organic); our walk stacks are almost all the latter.
  Discipline: every explicit stack from the P4 residual audit
  (and the D3-inherited `PARSE_STACK_INLINE` stacks) is
  implemented behind ONE type seam (a module-local alias or
  newtype, one line to swap); a measured phase — C3-adjacent,
  under the final harness, benign AND deep families, both
  scales — benches all three contenders and decides per site;
  the losers' numbers ride the DECIDED entry. The
  choice is not judgment-neutral: shallow walks on smallvec are
  allocation-free and heap columns pin that — a Vec win on time
  re-pins those rows deliberately (the parity-floor ruling's
  genre). Where a packed bit-stack (2 bits/level) suffices,
  neither applies and the bit-stack stays.
- **The envelope-harness unification** (the #39 census's deferred
  disposition): collapse the four envelope harness shapes in
  `tests/meter.rs` into one five-column shape with per-column
  floor-or-NA — the #35 totality mechanism applied to the gate
  suite; P5-window candidate, beside the standing
  harness-triplication item below.
- **Defended keeps (2026-07-24 scaffolding sweep) — adjudicated
  once, not relitigated**: the limb/scan/segment/touch meters and
  their floors (domain-semantic; no external tool can produce
  them); the adversarial generators, envelope harnesses, and the
  ratchet convention; `tier2_size` and the compactness probes;
  `testing/metrics` step counts; `tools/memwatch` (no macOS
  cgroup equivalent); `tools/doclint` (measured non-subsumed by
  clippy); heap metering (`peak_alloc` + the
  one-global-allocator-per-binary plumbing; dhat cannot reset
  its high-water mark mid-process); κ and its tripwires (every
  sub-problem exists because review refuted a weaker criterion).
  Open internal-hygiene items from the same sweep: the
  `tests/meter.rs` harness triplication (one
  column-set-parameterized harness; P5-window candidate);
  heap-column exact pins kept eyes-open under backend bumps.

### 17.10 Runbook conventions in force

- **Gate invocation**: `SWAP_LIMIT_GB=24 PROC_LIMIT_GB=16 just
  gate`; unqualified green since C0 — any failure anywhere
  blocks.
- **Manual staging** (user preference): stages run as
  individually spawned subagents, the coordinator sequences by
  hand; the user watches live.
- **Wall-clock discipline** (every charter): the 120 s cap
  applies to test EXECUTION, never compilation (`--no-run`
  splits); no ignored deep suites; no release test runs; no
  background commands or polling (a poll that must exist also
  checks the awaited process is alive); exactly one full gate
  per agent; never iterate on timing.
- **Docs policy**: agents correct user-facing rustdoc that is
  factually wrong, toward the code, always surfaced; style and
  substance improvements gated on the user (P5.4 is the slot).
- **The justfile is documentation** (user directive 2026-07-24):
  every recipe's comment block ends in a self-standing one-liner
  directly above the recipe (`just --list` shows exactly that
  line), essays above, blank-line separated.
- **Instruments before cures; pin red, re-pin tighter**: every
  criterion lands with a tripwire the status quo fails;
  thresholds tighten in the commit that earns them; retiring an
  instrument requires its replacement's tripwire red first (the
  dissolution ratchet). Phase-boundary dissolution check: "did
  anything we landed this phase make something older
  dissolvable?"
- **Roster conventions**: expectation lists carry expectations
  only, membership by name, pinned by tests whose diffs
  reviewers see; ceilings ride definition-site sidecars, never
  rosters; a cured cell leaves the roster in the same change.
- **Representation pins** (user directive 2026-07-23): every
  exposed type's externally observable representation —
  encode/decode bytes, serde/borsh forms, Display/FromStr text,
  documented Ord contracts — is snapshot-pinned inside `before`'s
  own suite, never only downstream; a representation change must
  force a deliberate re-pin. Full-surface measurability is
  three-legged: dual-oracle proptests, a resource pin, and a
  representation pin per exposed type.
- **Coverage only ratchets upward** (process constraint of
  record, 2026-07-22): tests, benches, and board cells are never
  removed or weakened through the campaign; the board and meter
  suites default to seconds-scale sizes so the inner loop stays
  fast, with record-scale runs at acceptance time.
- **Movement annotations** split pre-existing drift from the
  attributed change (measure at the parent commit; warm target
  dirs make the bisect cheap).
- **Agent practices**: charters state the goal beside the
  mechanism; reviewers dispute the seeds; never transcribe a
  proposed constant — re-measure; relay mid-flight findings
  between concurrent agents; agents end by returning
  synchronously (no backgrounded final steps); on an agent
  death, salvage = read the transcript, adopt the partial diff
  critically, commit, resume.
- **Environment cautions**: cargo's global package-cache lock
  convoys behind wedged rust-analyzer `cargo metadata` children
  (`lsof ~/.cargo/.package-cache`; killing cargo children is
  safe); worktree target dirs regrow to hundreds of GB and are
  freely deletable under disk pressure.
