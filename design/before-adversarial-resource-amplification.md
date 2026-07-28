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
| two-operand jump comb | a version pair on one shared spine turning right every 33rd level (`d` isolated position digits), then `m` comb levels: bare `2^k + 3` teeth over `(1, 0)` gaps on one operand, a hoisted `2^k + 1` plateau with unit bumps on the other | wide height-difference crests over a dense-position spine: `2m` freezes per distance, each fired by the operand that did not pay for the drift, every absolute position `d` incompressible digits while every per-crest segment mass compacts to O(1) — the family that separates absolute-position freeze accounting (superlinear here) from the co-sweep's anchored-segment accounting (pinned flat), either operand's own rank flat |
| concurrent pair | a balanced fork of `n` single-leaf parties, both operands ticked on every leaf, dominance alternating by parity, adjacent plateaus never equal | the emit side switch at every one of the `n − 1` overlay boundaries, join and meet alike (the ticked-counterpart pairing reaches at most one switch corpus-wide) |

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
  segments residual retired at P4.2 (explicit stacks, segments → 0).
  A pricing obligation under §6, not an exploit.
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
  owner's ratification, §12): the 103 `debug_assert!` sites in the
  production kernels perform metered work (`Base` comparisons
  through the limb shim, skyline grow probes consuming metered
  cursors), so dev and release builds measure different programs.
  **Release is the board's measurement of record** — it prices the
  production work alone, the honest denominator; dev runs are a
  debugging view, never pinned; assertion-scoped meter suspension
  REJECTED on doctrine (a metering-pause mechanism is an F2
  hazard). `id_is_empty` spot-checks the O(1) consequences of its
  contract; the diff kernel's emission normal form is held by its
  differential suites (`without_arbitrary`, the exhaustive
  small-scope diff leg, the function-space realization) rather than
  by any shadow re-parse, so no owned dev-profile scan divergence
  remains at that seam (the oracle-end-state entry, §12). The
  record-scale segments columns were profile-dependent by codegen
  (release frames are smaller, so onset shifts) until P4.2 made the
  walks iterative: both profiles now read zero.
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
  differentially against the recursive oracle's `join_all`
  (`oracle::Party`/`oracle::Clock` — the transcribed hand-back
  contract; a deliberate wrong-child mutation trips the
  differentials on exactly the committed fold seeds), with the
  predicate seam itself pinned by
  `indexed_disjointness_matches_the_cursor_walk[_deep]`. The
  coalesce-first candidate was REJECTED by
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
  record is §12's C3 entry (ratified by owner, 2026-07-26).
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
it. The rule's decision record is §12's C3 entry (ratified by
owner, 2026-07-26).

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
  `48c6f7b5`; ratified by owner, 2026-07-26)**: the text
  limb constant leg's denominator gains the per-spelled-value
  pipeline term (`R = n_io + Σᵢ (digitsᵢ × limbsᵢ + 10)`,
  `TEXT_PIPELINE_LIMB_OPS_PER_VALUE`), and κ re-pins 0.25 → 0.75
  over it — the honest text cost law includes the delta⇄absolute
  pipeline's per-value arithmetic, measured 5–9 ops per spelled
  value on the production kernels. Rationale: without the term,
  small-value trees judge gamma-pipeline arithmetic against pure
  conversion work and read falsely red.
- **AMENDED 2026-07-26 (C3, the comb-scatter classification,
  commit `ce8f9e69`; ratified by owner, 2026-07-26)**:
  flat-denominator shapes fit their exponents against value
  content (§6's rule); packed-byte fits on the shape manufacture
  e ≈ 4 from flat marginal work over the intercept premise.
  Rationale: the exponent leg must read the work's scaling axis,
  and the shape's packed bytes do not scale with it.
- **DECIDED 2026-07-26 (P4.2): the residual depth recursion
  converts to explicit stacks; the route-fold seam stays per-bit**
  (`3431ec60` pin, `68eda2e3` parser, `330e058a` walks).
  Attribution measured before converting: every tick segment came
  from the fused fill walk (grow's splice contributed zero; the
  pre-scan 4 of 15 dev-profile segments on mirror-narrow, zero on
  the no-site shapes), the parser cells from the id text parser
  alone. Landed shape: the id text parser parses on a SmallVec
  frame stack (the `codec::tree` discipline, differential-pinned
  against a recursive reference first); the fill walk and pre-scan
  carry suspended ancestors as control bits plus pop-able word
  deltas (`Frames`/`PreFrames` on the route fold's `PopStack`),
  and the one wide per-site quantity — a left-full site's collapse
  maximum, alive across its sibling walk — is re-derived by one
  bounded replay of the site's disjoint collapse range rather than
  parked per frame, keeping nested-site chains word-free (frames
  of materialized magnitudes would have tripled mirror-narrow's
  heap constant past its ceiling). With no library recursion left,
  `recurse::descend!` compiles only for the test surface (the
  oracle bridge); the segment meter's liveness witness is a
  test-local guarded descent and the deep fill's zero reading is
  the committed ratchet (`meter::tests`), with the envelope rows'
  segments pinned to 0 (`tests/meter.rs`). Board movement, both
  scales enumerated at `board-p42-{lo,hi}.txt` vs the unify24
  renders: the 17 ×4 segments cells read 0 (15 flip green; the
  ascend-cliff tick pair stays red on its owned heap constant,
  65.3 → 66.3 B/B with the frame bits); frame bits price
  ≤ 8.1 B/B on the text rows and ≤ 1.3 B/B on the tick rows; the
  site-close replay adds ≤ 2.3 scan bits/B and ≤ 3.5 touches/B on
  the memo-site crosses (comb-scatter limb +0.5 ops/B, the
  replayed codes' decodes) — every reading inside its ceiling,
  work columns byte-identical on all other converted rows, and no
  movement on any unconverted cell at either scale.
- **DECIDED 2026-07-26 (#50): the oracle end-state — three
  implementations, one committed roster.** The recursive oracle is
  the semantic definition of record and now carries the `join_all`
  hand-back contract (`oracle::Party`/`oracle::Clock`) and
  `meet_all` (`0ab5bf23`, seeds red through the new differentials
  before any retirement); `testing::fold_oracle` RETIRED under the
  ratchet (`f4ed32ed`), the test-local min_ticks/rank oracle
  duplicates dissolved into the oracle's own methods (`8ac7c0db`),
  and the three shadow-recompute `debug_assert`s deleted with
  covering differentials cited per site (`e277ca97`, the 2026-07-26
  assertions doctrine). The triangle suite
  (`testing::triangle`, `066623f6`) commits the op × three-leg
  roster, total against the extracted public-fn surface with live
  citation checks and per-leg adequacy tripwires. Rationale: one
  reference spelling per contract, every public op's binding (or
  exclusion) named in a diff a reviewer sees. Function-space-leg
  dispositions stemming from the FS boundary are marked in-roster
  (ratified by owner, 2026-07-26); no FS differential was added or
  removed.
- **DECIDED 2026-07-27 (P5 measurement closeout): envelopes
  re-pinned, item 11 realized, record sampling retired from the
  standing cadence.** (i) The envelope suite re-measured whole
  (one dev run, all 96 scenarios green): three rows moved and were
  tightened — `ID_WITHOUT` heap 518,219 → 416,888 (`e277ca97`, no
  dev shadow re-parse of the diff emission; ceiling 647,774 →
  521,110, the P5.1 final ratchet), `FOLD_VERSION_SCATTER` heap
  390 → 294 (`8181247a`; ceiling 488 → 368),
  `FOLD_PARTY_SCATTER` scan 292,432 → 257,654 (`5aabc765`, the
  fold's per-call id index; ceiling 365,540 → 322,068 — the
  292,432 record was a mid-round reading, the C2 flag-day commit
  measures 276,044); every other row byte-identical to its
  record, so §17.2's item 12 closes with no further movement.
  (ii) Item 11 realized and judge-verified under both sampling
  regimes (246 green / 3 red / 54 sub-floor over the 303-cell
  pinned subset, roster satisfied; riders e 0.93–1.18; bigroot
  sweeps e 0.92–1.04; the hugeleaf display pair STAYS red at
  e ≈ 1.4 over general 1.3 — the κ hand-off did not cure it, the
  class question stays open with the text column). (iii) Owner
  decision: record-sampling judge runs are not a standing
  closeout step — quick mode judges the wall leg (the two regimes
  agreed cell for cell; record wall cost measured 33 min 27 s),
  and record sampling belongs to the acceptance sweep alone.
  (iv) `exhaustive_deep` (#24's follow-up) is measured
  combinatorially blown: a fully parallel release run had not
  completed after 8 h 55 m against its committed ~4.5-minute
  annotation; the annotations are re-denominated to the measured
  open state and the runtime attribution is a filed follow-up
  (#53), owned outside this closeout.
- **DECIDED 2026-07-27 (owner): the fuzz-fit instrument joins
  `just gate`** (`fuzzfit-build` then `fuzzfit`; measured basis
  8.6 s warm build + 59.9 s run). The #24/P4.2 kernel work moved
  guest fuel and the stale bands sat red for a day because no
  standing tier executed the harness; in the gate, fuel movement
  fails the commit that carries it, with `fuzzfit-calibrate` as
  the deliberate re-pin path. Companion entry in the instrument's
  own decision record (`design/before-fuzzfit-asymptotics.md` §9).
- **Landed 2026-07-27 (#47): uniform `# Complexity` rustdoc,
  board-bound.** Every public operation carries a `# Complexity`
  section led by Big-O tokens over user-held denominators (packed
  sizes, text bytes, result sizes; per-fn, or per the owner's
  ruling one type/module-level note where a whole family shares a
  bound), and `testing/complexity_claims` binds prose to
  measurement: a 95-row claims roster (78 method ops from the
  triangle extractor + 17 family rows, totality both directions)
  pins each op's tokens at its scanned doc site and its witness
  rows on the board's own op axis (`board::bench_cells`);
  superlinear-time claims must equal the bench judge's committed
  red set (`tools/benchjudge-expected.json`) in both directions;
  and the two non-linear classes keep deterministic liveness pins
  that read red when their cures land — the render merge
  (`Display` limb growth ×2.93 across a doubling on the wide
  left-full shape, floor 2.45, linear ~2.0) and the fold log
  factor (`join_all` scan growth ×5.16 across a ×4 scatter
  population, floor 4.6, linear ~4.0), both measured 2026-07-27.
  Dispute-the-seed corrections to the charter's summary,
  transcribed from the boards and this document rather than the
  charter sentence: the `join_all` family is documented
  `O(D log k)` (the balanced reduction's log factor, §17.2's
  fold-marginals item — cure or ratify, either moves the pin and
  the prose together), and both text directions carry the
  radix-conversion caveat (superlinear-though-subquadratic per
  wide value; the dashu decision entry above is the measurement
  of record), with `Display` alone claimed superlinear outright
  on the render-merge and conversion-dominated mechanisms the
  judge's red set owns.
- **DECIDED 2026-07-27 (#53): `exhaustive_deep` attributed, cut
  to the ratified leg split, rewired to the public ops; the
  verdict pair product measured above proportion and reported.**
  Attribution (stride-sampled leg toggles, release, 16 cores):
  the id pair worker prices at ~91 ns/pair wall — is_disjoint
  6.3, covers +5.8, sum +12.2, diff +66.2 — so the
  allocate-convert-compare legs are ~87% of the worker, the
  region-difference leg alone ~73%, confirming the filed
  mechanism in ratio. The filed hours-scale runtime is *mostly
  not those legs' sampled cost*: sampled extrapolation
  (linearity checked 16.8M → 268M pairs, +4%) prices even the
  full four-leg worker at ~7 min, but the full 65536² product
  runs its *verdict-only* trim past a 45-minute cap twice (one
  row-major, one cache-tiled, quiet machine, 2026-07-27) — the
  expensive verdict pairs are structurally *similar* trees,
  dense in the full product's near-diagonal blocks and
  quadratically thinned by any strided sample; cache tiling
  bought no measurable relief, so the cost is walk compute, not
  misses. Cure landed as ratified: deep = codec + fork +
  verdict legs + tick with the brute-force grow-minimality pin
  (its irreplaceable value, ~1 min of the run); `join`/`without`
  structural pair legs run exhaustively at the small bound, deep
  structural reach riding on the sampled differentials; the
  anonymous id leaves the corpus at lowering (never a standalone
  `Party`; its per-pair emptiness test was two boxed-tree walks
  × 4.3G pairs in a first, abandoned run). Annotations
  re-denominated to the measured state (hour-scale bound, dated,
  with the sampling caveat written down). Rewired to the public
  surface: `IdReader::split → Party::fork`, `IdReader::sum →
  Party::join` (Result contract, receiver-unmodified/hand-back
  on overlap now asserted), `IdReader::diff → Party::without`
  (Option contract); `id_join_is_commutative` replaces the
  reader-level sum law; event checks verified already-public; no
  internal entries kept. Oracle operating envelope written into
  `oracle.rs` (small-scope only; harnesses bound inputs — the
  oracle is never hardened; fidelity outranks robustness);
  envelope audit: every oracle-facing suite bounded (arb
  generators depth 4, op-traces ≤ 120 ops, family grids scale
  ≤ 64, exhaustive depth ≤ 4; the 4096-spine and 100k-spine
  suites are impl-only with closed-form witnesses). DECIDED
  2026-07-27 (owner): the hour-scale verdict-pair totality test
  stays, and stays out of the gate — `#[ignore]`d, run detached
  on demand, its budget and machine annotation the contract for
  whoever runs it.
- **DECIDED 2026-07-27 (#54): the diff sweep settles covered
  subtrees as blocks — the early-exit constants recovered, both
  arms linear.** Mechanism (`ac10e61b`): dyadic nesting makes
  "does the other operand's current plateau cover this subtree" a
  depth comparison at every subtree entry, so the sweep settles
  covered intervals whole — an unowned `other` cover splices the
  `self` subtree verbatim (`IdSkylineBuilder::subtree`, closing
  upward as `Built::Node`, which is exactly what plateau-by-plateau
  re-derivation closes as; a terminal-collapse across a splice
  boundary is region-theoretically unreachable, since a terminal
  sibling would mean `other` was empty over the parent and the
  parent itself would have spliced), an owned `other` or unowned
  `self` cover consumes the subtree in one iterative skip scan as a
  single unowned plateau; owned `self` over an `other` subtree
  remains the plateau-walked complement arm, and same-interval
  subtree pairs descend in lockstep. Every tag still read at most
  once; segments 0; heap transient unchanged (path bits). Fuel at
  1000 bits, per arm (pre-sweep pin → sweep pin → this pin):
  success 4.63 → 5.81 → 4.72 fitted decades — the +1.18-decade
  regression cured to within +0.09 of the pre-sweep line, and the
  10^3-bit bucket median (42 fuel/bit vs the pre-sweep fitted 43)
  reads parity; emptiness 5.07 → 5.64 → 5.47, top-decade medians
  flat at ~430 fuel/bit (the full-scan rejections are unchanged at
  scale). The emptiness arm's pooled envelope tilts to 1.41: the
  charter's ≤1.05 expectation is disputed with evidence — restoring
  early exits necessarily re-creates the rejection-mixture genre
  (cf. `party_join` 1.48), covered rejections now settle in one
  block scan and undercut the small buckets while `bin/diag`
  medians and the within-case shape leg (max healthy excess +0.005,
  unchanged) carry the linearity claim. Differential evidence: all
  diff differentials, the exhaustive pair leg, and the envelope
  rows green; a planted wrong-kind splice mutant
  (`close_up(Built::Empty)`) reddened `without_arbitrary` and
  `exhaustive_small` before revert. Boards: both scales
  row-for-row byte-identical to the boards of record — the
  board's two `without` cells exercise the complement arm
  (`seed().without(b)`) and the identical-operand rejection, the
  two paths with no coverable span, so the expected scan-constant
  movement rightly does not appear there; the fuzzfit corpus, whose
  fork-derived operand pairs do have coverable spans, is where the
  cure is priced. Re-pin `d9325886`.
- **Landed 2026-07-27 (#58, owner-ruled): `causally` composition
  validates; `Decode::Io` displays.** (1) `causally::Range`
  (`1b7a6d97`): pairing a start with an end passes a
  well-formedness gate — the start version must lie within the
  end bound (`start <= end` under `known_at`, `start < end` under
  `before`) — and a crossed pair returns the new `error::Crossed`,
  so every range that exists has a total `placement_of` trichotomy
  and the crossed-range case is unrepresentable rather than
  documented around. Consequence class: typed error, the crate's
  convention for compositions that violate a relational invariant
  (`Overlap` from `Party::join`/`Clock::join`/`sync`); panics and
  debug_asserts stay reserved for internal infallibility. Free
  single-bound constructors remain infallible `O(1)`; two-bound
  compositions cost at most one validating causal comparison (the
  claims roster re-denominates them; the `causally_contains` row
  prices the comparison). Family pin:
  `gate_admits_exactly_the_uncrossed_and_they_cohere` — the
  composition verdict differentially against `partial_cmp`, plus
  trichotomy coherence over lattice probes (bottom, bounds, join,
  meet), all four bound pairings in both composition orders. The
  rumors tree differential restates its naive-filter oracle inline
  over raw `Bound` pairs: `Tree::range`'s `RangeBounds` surface
  still accepts the crossed pairs the generator deliberately
  drives. (2) `Decode::Io` (`42aca1f2`) renders its wrapped
  `std::io::Error` by `Display` (`{0}`), not its Debug form; the
  committed prefix pin (`error_display_strings`) asserts only the
  crate-owned `read error: ` prefix and did not move, and no wire
  snapshot was touched.
- **FINDING 2026-07-27 (the presize-61 cure attempt; charter
  premise disputed with evidence): reserve-once pre-sizing is
  already landed at every builder site, and the capacity-phase
  pair is owned by the one op whose output is not size-derivable —
  no reserve change exists to make, and no cell moved.** The
  frontier's §2.3(b) move ("a two-line reserve change per builder;
  dissolves the capacity-phase red as a side effect") has no site
  to land on: all six output builders reserve once from operand
  sizes at construction and have since the commits that created
  them — the join/meet sweep (a+b bits), grow (ev+id+64), the
  projection (ev+id), the text parse (the text's own length), the
  fused tick fill (ev), and the id diff sweep (self+other). The
  artifact's mechanism, probed at release on a fine tooth sweep
  (`cliff_comb(1_000, t)` × `scattered_id(t/2)`, t = 96..1152
  step 32): projection output is mandatorily Θ(|v|·|p|) bits on a
  Θ(|v|+|p|) input (the `/` operator rustdoc's documented product
  growth; output/input reads 45–119 across the four board probe
  points), so the honest-as-a-floor ev+id reserve under-runs by
  that ratio and the builder walks a 6–7-step doubling chain
  anchored at the input size. Peak transient is the last
  realloc's old+new coexistence, ≈ 3·(n+m)·2^(k−1) with
  k = ⌈log2(output/(n+m))⌉ — every probed point fits within 2%,
  the residual the walk's cursor/accumulator/code transients
  (t = 224: model 166,272 B, measured 167,432) — and the board's
  default probe pair straddles
  the k 6→7 step (e 1.38) while the ×4 pair sits inside k = 7
  (e 0.70, peak tracking the input side, which grows slower than
  `n_io`). Refuted alongside: a finish-time shrink discipline
  cannot stabilize this reading — the peak is mid-walk, already
  set when finish runs (§17.2's projection sub-bullet and §17.3's
  owner note re-attributed to this entry). Candidate cures,
  priced but not landed under this charter's no-pre-scan rule:
  (i) a size pre-walk — the overlay sweep minus emission — feeding
  one exact reserve (peak → output plus walk transients, both
  cells flip; price ≈ ×2 on the input-side scan/limb/touch
  columns, ~+0.3 scan bits per `n_io` byte on this shape against
  the 96/B ceiling, no output-side re-scan; the text render's
  two-pass exact-sized discipline is the in-tree precedent);
  (ii) a segmented output assembled once at finish (peak →
  ~2×output, phase-free; price one extra copy of the output,
  +8 scan bits per output byte, and the builder's
  truncate/extract moves crossing segment boundaries). An
  un-anchored (organic) growth chain is REJECTED on doctrine: it
  reads e ≈ 1.0 only because ×2 probe pairs cancel the
  power-of-two sawtooth in the two-point fit — tuning the point,
  not fixing the shape. Disposition OPEN to the owner: land (i)
  or (ii), or ratify the pair as a stated-band residual (the
  constant leg reads 2.6 B/B at default and 1.6 B/B at ×4 against
  the 16 B/B ceiling; the exponent leg is the only red). Boards at both scales:
  byte-identical to the boards of record, 966/23 and 969/20 — no
  movement, no change landed. (Resolved 2026-07-27, owner ruling:
  RATIFIED as the stated-band residual — neither cure lands; the
  band is the documented honest profile of a non-derivable output.
  §17.3's capacity-phase entry carries the standing owner, and the
  decided lazy projection view, `design/own-version-view.md`, will
  make the materialization explicit-only.)
- **DECIDED 2026-07-27 (#67, owner-ruled): the skyline topology flag
  inverts (0 internal, 1 leaf) and the version-stream reader
  dissolves into `dsi-bitstream`.** The unshipped-window argument
  carries the protocol change: main still holds the pre-skyline
  format, so the inversion folds into the same revision that first
  ships skyline and no second protocol version is burned. Mechanism:
  a descent is now a unary run (zeros ended by the leaf's 1), read
  word-parallel by `codec::DsiCursor` (BufBitReader, BE, u32 words,
  safe zero-copy word source); payload values ride the in-house
  wide-arm wrapper (9-bit table tier, machine arm `k < 64`, `UBig`
  arm `k >= 64`), never dsi's `u64`-capped `read_gamma`, with
  word-seam witnesses at `k = 63/64/65/~100`. The bijection is
  pinned by transcode differentials (`topology_flag_bijection_*`:
  an independent inverted-flag encoder, flipped flag-by-flag onto
  the stored stream, both directions), and the rejection corpus
  re-pins with the same error classes at the same cut points.
  Dissolved: the sweeps' per-bit `LeafCursor` reads, the
  validator's and every skyline walk's per-bit topology loops,
  `codec::skip_int`. Kept hand-rolled, stated: the id-stream
  readers (party coding unchanged), the borsh `ReaderCursor`
  (incremental `io::Read`; dsi wants a materialized word source),
  writers (dsi's writer wants word sinks), and `decode_int` over
  mid-byte code sub-slices. Board identity: both scales
  cell-for-cell byte-identical to the boards of record except the
  two `*_decode_truncated hugeleaf` reject rows, whose heap drops
  0.5 B/B (still green, exponents unchanged) — the new reader
  proves a code's length before its wide arm allocates, so a
  truncated wide code no longer sizes the value it will never
  return; the truncation scan floors stayed enforced by recording
  the examined tail at every reject. Fuel: version-walking arms
  drop ×0.81–0.98 (`version_cmp` ×0.81, `version_rank` ×0.83,
  `version_decode` ×0.84); `clock_tick`/`clock_send` rise ×1.035/
  ×1.020 — the fill walk's id-interleaved single-flag reads pay the
  buffered reader's constant with no run to batch; slopes and the
  board's tick rows are unmoved. rumors rides the revision: content
  addresses move, wire/trace snapshots re-accepted (topology bits
  and hashes only; framing and sizes identical), searched fixture
  constants re-searched, and the role-sensitive proxy tests now
  arrange sides through the initiator election itself instead of a
  version-byte proxy the coding change falsified. Flip
  `40744605`; re-pin rides the same branch.
- **Landed 2026-07-27 (suanpan-64, phase 1 of the owner-decided
  extraction): the accumulator is the workspace crate `suanpan`,
  type `suanpan::Accumulator`; before consumes it through a pure
  re-export shim.** Dependency decision: suanpan speaks dashu-int's
  `UBig` directly (re-exported as `suanpan::UBig`; the IBig
  differential suite moved with the crate) and the small-helper
  half moved *as a trait* — a public `suanpan::Magnitude`
  (word-scale fast path + borrowed wide view) keeps the
  width-dispatched `*_base` entry points inherent generic methods,
  which is the only shape under which the shim can stay a bare
  `pub use suanpan::Accumulator as Accum;` with zero call-site
  edits (an alias cannot add inherent methods; an extension trait
  would need imports in the frozen files). Shim shape:
  `codec::accum` = the re-export, `touch_meter` under
  `limb-meter → suanpan/touch-meter`, `impl Magnitude for Base`,
  plus seam-differential tests of the Base dispatch;
  `meter::accum` re-exports unchanged; zero edits under
  `version/skyline`, `version/rank.rs`, `party/ops`. Ratchet
  triple through the new home: all 543 before tests green with the
  rank touch columns bit-identical to the annotated measured
  values (125,007 / 17,194 / 198,659 / 17,814 — ceilings and
  liveness floors both), and the tier-2 plain-sweep known-bad pin
  still reads quadratic. Fuzzfit: all 18 suites green with NO
  `#[inline]` annotations and no re-pin — cross-crate codegen
  parity held on its own (one parity-motivated choice: `sub_accum`
  keeps its own loop rather than delegating through the new
  `sub_accum_shl`). Boards byte-identical to the 86e85420 parent
  at both scales (`board-suanpan64-{before,after}-{lo,hi}.txt`).
  Docs protocol: four fresh-eyes Fable rounds (47 → 25 → 19 → 24
  findings; round 4 zero HIGH = convergence; raw reports
  `suanpan-review-r{1..4}.md` in the job dir). **FINDING (round 1,
  verified against code): `is_zero` was documented "exact" but
  reads false on a zero spelled by cancelling digits**
  (`add_wide(2^32); sub_small(2^32)`); behavior unchanged — the
  one call site (`skyline/query.rs` rank sweep) uses it as a
  skip-work fast path where a false negative only costs touches
  the envelopes pin — the doc now states the one-sided contract,
  a committed test pins the spelling behavior, and the sweep-rewrite
  track should read the note before leaning on it. Round 2
  refuted the ops table's per-call classification of wide writes
  (digit runs parked near the zone edge make every write bound
  amortized; table and derivation corrected). New API beyond the
  move: `sub_wide_shl`, `sub_accum_shl` (subtractive twins,
  oracle-covered), `Magnitude for UBig`, the `UBig` re-export.
  Build surface: workspace member added; justfile `clippy-default`
  and `features` gained suanpan lines; README derived via
  `tools/readme` (crate added to its roster). Phase 2 (recorded
  follow-up, post-protocol-pass): rename the call sites
  `Accum → Accumulator`, delete the shim, and take the API-shape
  candidates deferred from review — `*_base → *_magnitude`-style
  names, a possible `merge_into_wider` rename or spare-buffer
  newtype, an owner call on MSRV/`no_std` statements, and
  dissolving before's private `U64Limbs` twin into the crate seam.
  (Landed: this ledger's 2026-07-27 suanpan2-76 entry.)
- **INSTRUMENTS LANDED 2026-07-27 (the distance/lag co-sweep's
  gating families; instruments before cures).** Two committed
  pair families close the coverage gaps the materialization probe
  named (§2.4 of the constants frontier), with the CURRENT
  architecture's cost pinned before any cure:
  (i) **Two-operand jump comb** (`meter::jump_pair(k, m, d)`,
  family `jump-pair`): both operands share a descent spine that
  turns right every 33rd level — `d` isolated freeze-position
  bits the balanced signed compaction cannot merge — then an
  `m`-level comb where the teeth operand stores bare `2^k + 3`
  leaves over `(1, 0)` gaps and the band operand rides a
  once-paid `2^k + 1` plateau with unit bumps. Their meet
  interleaves them (`+W, −1, −W, −1` per level), so the rank fold
  freezes `2m` times, every eviction triggered by a cheap code
  from the operand that did NOT pay for the drift, at a
  `d`-digit freeze position. **FINDING, red-pinned: the
  cross-stream funding hole is real and quadratic-class.**
  `version_distance × jump-pair` reads limb/touch exponents
  1.67/1.76 at the default scale pair and 1.89/1.93 at ×4
  (envelope scale: 7.23 → 12.42 limb-ops per packed byte across
  one (m, d) doubling, ×1.72); either operand's own rank is flat
  (board `version_rank` green; the envelope band's two operand
  controls flat at ×1.00 and ×1.07), and `lag` — no meet leg;
  the join collapses every comb level because the band shades the
  gaps — is linear at ~0.95 limb-ops/B: the wedge is exactly the
  meet-side rank fold, reached only through the two-operand
  public surface with each input individually innocuous. Pinned
  red in four instruments: the board cell (both scales), the
  `DISTANCE_JUMP_PAIR` envelope + the `skyline_flatness` growth
  floor (per-byte limb work must rise ≥ ×1.5 across the doubling
  — the cure trips it deliberately and flips it to a flatness
  bound), the bench judge roster (`version_distance/jump-pair`
  red; a green time leg before the cure means the leg went dark),
  and `Version::distance`'s `# Complexity` re-classed to
  superlinear worst-case time (claims roster moved in the same
  change — the prose was factually wrong against the measured
  code).
  (ii) **Concurrent pair** (`meter::concurrent_pair(n)`, family
  `concurrent-pair`): a balanced fork of `n` single-leaf parties,
  both operands ticked on every leaf with dominance alternating
  by parity and adjacent plateaus never equal, so the emit side
  switch fires at every one of the `n − 1` overlay boundaries —
  join and meet alike — where the corpus-of-record pairing
  (`w = v + one seed tick`) reached at most one switch
  corpus-wide. Achieved density at the envelope scale
  (n = 4,096): 4,095 switches per emission over 5,889 packed
  bytes ≈ 0.70 switches per input byte per emission (distance
  runs two emissions: ~1.39/B). The realized-schedule witness is
  semantic: distance = the integer rank 2 exactly, at every `n`,
  pinned beside exact join/meet plateau counts. Honest linear
  pins: `DISTANCE_CONCURRENT`/`LAG_CONCURRENT` envelopes, board
  cells green at both scales.
  Board wiring: two `FamilyKind`s (both `designed` against the
  Measure group), the `version2` bundle slot now family-fillable
  (the post-pass derives the ticked counterpart only where a
  build arm left it empty), 1071 cells (the smoke pin moved:
  41 × 7 version-bearing + 36 + 62 × 11 + 2 + 64). Two floor
  derivations were corrected — forced honestly by the first
  operands that reach the early exits, all readings untouched:
  the comparison rows floor at the root codes exactly when the
  pair is concurrent (a comparable pair keeps the full floor),
  and `version_min_ticks` floors at the root codes exactly when
  the stream stores a payload code wider than a machine word
  (the fold may saturate and stop). Boards of record
  (`board-dlfam66-{lo,hi}.txt`): default 1047/24, record
  1050/21 — every pre-existing cell byte-identical to 966/23 and
  969/20 except the `flr` scan column of `version_min_ticks` on
  the eleven wide-code families (the corrected declaration;
  every reading, exponent, constant, and verdict identical).
  Cross-checked against the relayed one-sided `is_zero`
  contract: neither family's pins assume zero detection fires —
  the fold's live component is freshly reset at each eviction,
  and every pinned number is measured, so the pins price the
  general path.
  Recorded for later phases, not touched here: the fuzzfit
  corpus could gain a concurrent-pair operand arm (its fuel
  bands are an in-flight seam; phase-2 item), and the cure phase
  owes the overlay-scale freeze re-derivation plus digit-exact
  differential pins against the composed forms and the oracle —
  its improvement must move five committed readings at once: the
  `DISTANCE_JUMP_PAIR` envelope, the growth-floor band (to
  flat), the board cell (to green), the judge roster entry, and
  the distance complexity class.
- **CURED 2026-07-27 (#66 phase 2): the fused distance/lag
  co-sweep — the cross-stream funding hole is closed and the five
  committed readings moved in one commit.**
  `Version::distance`/`Version::lag` integrate their measures
  directly over the overlay (`∫|h_a − h_b|`, `∫(h_b − h_a)⁺`) in
  one merge walk on the accumulator: no join/meet stream
  materialized, no per-operand rank recomputed, the boundary
  algebra `dh* = (σ′ − σ)·D′ + σ·dD` with `|D′| ≤ |dD|` at every
  orientation change. The freeze discipline is re-derived for the
  overlay's access pattern as an anchored-segment split
  (`h* = B + P + L`): parked drift settles against per-segment
  masses (compacted span = within-segment depth variation; the
  spine's shared prefix cancels), promotions to the zero-anchored
  base pay the sweep's only absolute-position product once per
  wide arming, and the funding certificate is a **two-ledger
  potential, one per operand, every charge naming its deposit**
  (the potential-arity census: the composed form's per-stream
  argument failed exactly at the emission seam). Derivation and
  certificate live in `version/skyline/query.rs`'s pair-co-sweep
  section; `suanpan` gained the write-watermark scaled read
  (`sign_magnitude_shl`, O(written span)) the segment masses
  need, and `Accumulator::is_zero` was renamed
  `is_literally_zero` with the one-sided contract in name and
  rustdoc (owner ruling 2026-07-27) — the co-sweep never relies
  on exact zero detection. The five readings, before → after,
  parent-measured for attribution: (1) board
  `version_distance × jump-pair` red → green at both scales
  (default limb/touch e 1.67/1.76 → 1.00/1.00, 7.0/7.6 → 0.4/1.4
  per byte; record e 1.89/1.93 → 1.00/1.00, 22.5/26.4 →
  0.4/1.4); boards default 1046/25 → 1047/24, record 1049/22 →
  1050/21, the red-set delta exactly this cell at both scales,
  every other verdict identical (note: the §17.3 sums of record
  had drifted — `version_min_ticks × jump-pair` reads a
  scan-floor red at the parent already, both scales,
  un-enumerated; carried into §17.3 with an open disposition.
  Resolved 2026-07-27 at this branch's merge: the ticks(n)
  landing's exact fold cures it — see §17.3's dated resolution).
  (2) `DISTANCE_JUMP_PAIR` re-pinned tight: heap 138,809 → 5,008,
  limb 973,702 → 53,905, scan 6,464,538 → 3,218,320, touch
  1,029,327 → 184,494; the lag/concurrent rows re-pinned with it
  (lag-jump-pair touch 87,148 → 163,509 is the one rise — lag
  walks the full overlay now — bought with heap −96%, limb −60%,
  scan −25%). (3) the `skyline_flatness` growth floor tripped
  deliberately (per-byte limb 7.23 → 12.42 across the doubling
  before; 0.40 → 0.40 after) and flipped to a flatness bound
  with ceilings tightened ~18× limb / ~5.6× touch. (4) the judge
  roster: `version_distance/jump-pair` out of the red set, the
  time leg's liveness witnessed by the schoolbook tripwire's
  required red in the same run. Judge-run disclosure (quick
  mode, 2026-07-27 evening): the first run surfaced and fixed a
  flag-day breakage before any timing (the board bench built its
  wide-display operand by decoding a construction-language
  stream as wire bytes — Truncated since the topology-flag
  inversion; it now lifts through `Packed::version()`), then ran
  under a concurrent agent build storm (15-min load ~50) and
  flagged 22 unrostered noise reds; the re-run on a
  checked-quiet machine (no other cargo/rustc; ambient
  Backblaze/mediaanalysisd churn disclosed) flagged 4, disjoint
  from the first set but for `version_parse_noncanon/hugeleaf`
  (e 1.31/1.42). In BOTH runs the wedge cell read green under
  its ceiling and the tripwire read its required red; every
  flagged cell's deterministic work columns are exponent-flat on
  the board and the flagged sets do not reproduce across runs —
  the load-noise signature, left for a record-mode pass on a
  quiet machine rather than a third quick-mode fish. (5) `Version::distance`'s
  `# Complexity` re-classed superlinear → linear
  `O(|a| + |b|)`, the claims roster moved in the same change.
  Differential coverage landed ahead of the cure: distance/lag
  digit-exact vs the paper oracle AND the composed rank-of-meet
  arithmetic on the crate's own kernels, deterministic and
  proptest legs over the pair families, plus (with the cure) the
  exhaustive small scope's every *ordered pair* of normal-form
  trees against the composed forms — the total check over the
  boundary genres crossed with the orientation schedules; the
  concurrent-pair semantic witness (distance = the integer rank
  2 at every `n`) held throughout. Honest residual, documented at the code:
  promotion at position density once per wide arming, settles at
  within-segment depth variation — both mandatory-class (the
  measure's exact value embeds the product). Fuzzfit: bands held
  as pinned, staleness cross-check green — per the phase-2
  contract, no recalibration and no concurrent-pair operand arm
  was added (the condition demanding them did not arise). Side
  effect, priced: the accumulator's +8 B watermark field moves
  five tick-cell board heap constants by ~0.1 B/B (verdicts
  unchanged, work columns byte-identical).
- **DECIDED 2026-07-27 (#73, owner-ruled): the mirror proxy's error
  selection prefers a deposited root cause over a racing consequence,
  and the transport-fault attribution pin is universal again.** The
  flip-67 pass had bounded one arrangement (an Accept fault on the
  elected initiator whose cut fails the endpoint's own flush with a
  bare BrokenPipe before attribution) as a carve-out in
  `transport_failures_are_exact_and_fail_fast`, its committed seed
  replaying green about half the time. Mechanism found: the race was
  never deposit-vs-consequence — the accept driver deposits the supply
  failure's I/O cause in the same poll that observes the cut, strictly
  before any in-process consequence can become ready — but
  deposit-vs-*claim*: the first `SupplyClosed` reporter drained the
  deposit slot into a report that then lost the terminal's biased
  select to the consequence error, stranding the injected identity in
  an unread channel. Fix at the selection seam (`Work::execute`):
  reporters never touch the slot (`read_frames` reports `SupplyClosed`
  with no source); the session terminal is the slot's sole consumer
  and, on any failed selection, surfaces the deposit as `SupplyClosed`
  (keeping the reporting stream's origin when the selected symptom
  already named the dead supply, direction granularity otherwise);
  plus one final non-waiting poll of the accept driver flushes a
  supply failure whose readiness lost the biased race inside a single
  wave — the external-cut genre a real transport can produce,
  witnessed by `deposited_supply_failure_outranks_a_racing_consequence`
  (red without that poll). No waiting added anywhere: the flush is one
  poll (the driver deposits-and-parks or is pending), so the link
  contract's no-deadline liveness posture stands. Determinism proof:
  carve-out deleted, then 150 runs of the failures property (the
  committed seed plus 32 fresh generated cases per run) all green,
  where the pre-fix seed flipped roughly 50/50; a 30-run replay with
  the flush poll disabled also all green, attributing the in-harness
  fix to the slot discipline alone, exactly as the happens-before
  analysis predicts. The universal pin (surface + injected identity
  asserted on every injected fault, no carve-outs) is restored; the
  seed stays committed as the regression witness for the fix.
  Semantics scope: which error a failed session reports in one race;
  success paths, wire bytes, and snapshots unmoved.
- **Landed 2026-07-27 (suanpan2-76, phase 2 of the owner-decided
  extraction): the shim is dissolved — every call site names
  `suanpan::Accumulator` directly.** The rename sweep moved the
  skyline walks (fill/watermark/sweep/emit/text/validate/query),
  the rank fold, the board runner, and the resource-envelope suite
  onto the direct paths (the suite reads `suanpan::touch_meter`
  itself; feature unification keeps it the same counter before's
  code bumps); the `Magnitude` impl for `Base` and its
  seam-differential tests live in `codec::base`. Ratchet order
  held: the envelope ceilings and liveness floors, the rank touch
  columns (bit-identical at 125,007 / 17,194 / 198,659 / 17,814),
  and the tier-2 plain-sweep known-bad pin read green through the
  direct paths at the sweep commit, and again after `codec::accum`
  and the `meter::accum` re-export were deleted. `U64Limbs`
  dissolved into the crate seam as `suanpan::Limbs` — a public,
  named, double-ended borrow iterator over a magnitude's 64-bit
  limbs, the unit the crate's wide-operand costs are denominated
  in — now feeding the wide entry points, `Base::msb_cmp`'s
  streamed windows, and the query fold's digit compaction, so the
  cross-target words-per-limb packing lives in exactly one place.
  API-shape candidates: `*_base → *_magnitude` TAKEN (the old
  names carried one consumer's concrete type into the generic
  crate's API, propped by a minted "*base*" gloss whose only job
  was justifying them). `merge_into_wider` rename / spare-buffer
  newtype DEFERRED: no decided target shape exists, the only
  in-tree consumer is the watermark pool, and the drained-buffer
  contract is documented with a doctest — a newtype would add
  surface to encode a discipline the docs already state; revisit
  if a second pooling consumer appears. MSRV/`no_std` statements
  untouched (owner policy). Parity: boards byte-identical to the
  c1eb3428 parent at both scales
  (`board-suanpan2-{before,after}-{lo,hi}.txt`); fuzzfit all 18
  suites green with no `#[inline]` additions and no re-pin.
  Design-doc note, dated here: the tick-cost spec
  (`before-tick-cost-spec.md`) and the formal-tick notes
  denominate costs in "Accum digit touches"; that unit's type now
  spells `suanpan::Accumulator`. The spec text itself is
  untouched — it is a statement of record, and the rename amends
  no claim; this entry is the dated cross-reference.
- **DECIDED 2026-07-27 (#75, owner-commissioned): the algebraic laws
  factor into `before::laws` — one named-predicate collection, every
  consumer.** Shape: 12 signature-grouped slices of
  `(&'static str, fn(...) -> bool)` (`VERSION_{SOLO,PAIR,TRIPLE}`,
  `PARTY_{SOLO,PAIR,TRIPLE}`, `VERSION_PARTY`, `VERSION_PAIR_PARTY`,
  `VERSION_PARTY_PAIR`, `RANK_TRIPLE`, `CLOCK_SOLO`, `CLOCK_VERSION`),
  ~100 laws, predicates private behind public statics (the triangle
  roster's pub-fn totality is untouched; rows re-cite by predicate
  name where a bespoke test dissolved). Linearity: predicates take
  shared borrows and materialize consumable working copies via
  `dangerously_alias`, confined to the predicate's scope; fallible
  ops are outcome-quantified (arm and payload both). Visibility:
  `laws` cargo feature in the `oracle`/`meter` idiom
  (`#[cfg(any(test, feature = "laws"))] pub mod laws`). Consumers:
  the algebraic-laws suite's per-group drivers (arbitrary normal
  forms and organic op-trace populations), the version suite's
  seeded-adversarial-rank driver, and the `fuzz_laws` target
  (length-prefixed chunk framing, default fallback per failed chunk,
  committed seeds including wide-gamma bases — 2^64/2^128 — whose
  64+-zero unary prefixes random bytes never reach; a gate test pins
  the seeds' framing and wideness). Dissolved into the collection
  name-for-name: the bespoke algebraic_laws tests, version/tests'
  `order_laws`/`lattice_laws`/`meet_lattice_laws`/`monotone_tick`/
  `div_by_party_laws`/`div_is_additive_over_fork`/
  `rank_monoid_and_order_laws`/`rank_cross_path_normalization_and_hash`/
  `ranked_linearly_extends_causality`, clock/tests'
  `fork_preserves_version`/`peek_does_not_advance`/`own_receive_is_tick`,
  party/tests' `covers_tracks_fork_join`/`d_join_overlap_hands_back`/
  `dangerously_alias_aliases_region`/`without_inverts_fork`, and the
  law-only assertion lines inside `covers_arbitrary`/
  `disjoint_arbitrary` (their differentials stay). Kept deliberately
  outside: the exhaustive small-scope suite's four point laws (a
  totality instrument over enumerations, not a sampled driver), the
  population/fold laws (`join_all`/`meet_all`/`Sum` folds,
  `ranked_sort_respects_causality` — not fixed-arity), and the
  internal-seam `byte_equality_matches_bit_equality`. Recorded
  future addition: post-ticks laws (`ticks`/`min_ticks`) join the
  collection with the ticks landing (fulfilled 2026-07-27: eight
  laws across four existing groups; the #71 entry below).

- **DECIDED 2026-07-27 (Finch; landed #71, probed #68):
  `ticks(n)` — the fused multi-tick — is a first-class public
  operation, and every count-of-ticks surface is denominated in one
  opaque unbounded newtype, `Ticks`.** The probe
  (`design/probe-ticks-68.md`, branch `tickby-68` @ dbb8e53f) proved
  the mechanism: `n` sequential ticks = at most two fused walks plus
  one `+n` splice, byte-identical to the iterated public tick
  (fill is idempotent; grow preserves fill-fixedness; the route is
  value-blind and its site stable), at `O(|v| + |p| + log n)`.
  Landed shape: `grow::emit` generalized to the `+k` splice (one
  splice path — the probe's `emit_by` twin retired with the merge),
  `fill::ticks` the two-branch conditional, three public surfaces
  (`Version::ticks`, `Party::ticks`, `Clock::ticks`, all
  `impl Into<Ticks>`; the batch mirrors deliberately not built —
  the Batch API is dissolving on a parallel track), and
  `Version::min_ticks` returning `Ticks` exactly at any magnitude
  (u64 became partial the moment `ticks(2^100)` exists). The `k = 1`
  splice performs tick's exact metered operation sequence in both
  profiles: all five tick envelope rows read byte-identical to their
  pre-landing baselines [measured 2026-07-27: dense
  47_052/250_008/125_017/375_012 heap/limb/touch/scan, expand-cross
  488_989/750_010/250_013/2_875_025, expand-spine, nested-wide,
  mirror-wide — all equal to the captured baseline]. The exact
  `min_ticks` rides rank's frozen/live split (minima as narrow
  F-relative offsets, `F` by counting, epoch-tagged lazy re-basing);
  its envelopes re-pinned with the movement owned: dense limb
  250_002 → 500_002 (one signed offset compare per closing node),
  cliff heap 560 → 49_752 / scan 2_052 → 14_338 (the early
  saturation exit is retired, so the comb reads every leaf — its
  scan-floor carve-out in the board dissolved with it). Board
  movement (default scale, 25 → 31 reds): the exact fold CURES the
  `version_min_ticks/jump-pair` dead-meter red (the saturating exit
  left the scan counter below its floor) and joins the counter-red
  genres the honest work now meters — heap constants on
  mirror-wide/ascend-cliff/ascend-plateau (the tick rows' standing
  genre) and the close-reveal/undercut circulation on
  cliff/pure-comb/reveal-comb (limb/touch e 1.5–1.95; the rank
  fold reads the same reveal rows red at sub-default scales — the
  watermark anchor-web is the known cure for both, out of this
  landing's scope and named in the query module's cost section).
  `version_ticks/ascend-cliff` reads exactly `version_tick`'s
  standing heap-constant red (identical readings). OPEN for the
  owner: whether the four untimed new reds (`version_min_ticks` ×
  reveal-comb/pure-comb/ascend-cliff/ascend-plateau) join
  `BOARD_RED_BENCH_RIDERS` — their bench time legs are unmeasured,
  and the rider census (2026-07-26) already lagged the base board
  (base read 25 default reds against the census's 23), so the next
  realization should be one owner-reviewed sweep. New
  instruments: `version_ticks` board cells (fixed count 512; smoke
  pin 1071 → 1090), ticks envelope rows on the tick-designated
  families, and the flatness pin — `ticks(512) → ticks(4096)` moves
  scan by exactly the count codes' gamma delta [measured: 6 bits on
  all three families, bands 12/8/8] with liveness floors on the
  envelope rows; exhaustive small scope at n ∈ 0..=4; `Op::Ticks`
  in the organic vocabulary (oracle side literally iterated);
  `fuzz_decode_ops` gains the fused op at a `2^100` count (seed
  regenerated by its derivation; `fuzz_laws` framing untouched —
  the new laws ride the existing groups and draw wide counts from
  `min_ticks` of the decoded operands). Laws: eight named
  predicates across `VERSION_SOLO`/`VERSION_PARTY`/
  `VERSION_PAIR_PARTY`/`CLOCK_SOLO`, wide-count composition
  included. rumors: the bootstrap dominance seam
  (`NetworkMismatch`/`BootstrapHistoryConflict` min-events) carries
  `Ticks` — totally ordered at any magnitude, no conversion and no
  error arm between `min_ticks` and the rule — and re-exports the
  type; wire bytes unmoved (counts were always derived locally from
  the declared version, and the V1 wire snapshots stand unchanged
  under fused fixture construction, the byte-identity guarantee
  read back through the protocol pins).

- **DECIDED 2026-07-27 (owner, #72): the batch API is removed from
  `before`** — `Version::batch`/`Clock::batch`, the two `Batch`
  handles, and the `batch` re-export module — as "a footgun waiting
  to be accidentally discharged": the surface looked like it
  amortizes work and amortized nothing. The accretion story is the
  dissolution doctrine's textbook case: the original `Batch` held a
  deferred `work` state and its docs claimed multi-op efficiency;
  C2 (§12's flag-day entry) moved the operation kernels onto the
  packed stream, every op commits as it runs, and the working form
  ceased to exist — the handle survived its justification as pure
  chaining sugar priced as an economy. The honest replacement for
  repeated ticks is the ticks(n) surface (landed, #71; probed in
  `design/probe-ticks-68.md`). Landed shape: the `|`/`&`/comparison
  matrices collapse to owned/borrowed `Version` cells (the
  `join_view`/`meet_view` cores now live on `Version`; `Clock`'s
  join/sync/recv inline their part-wise bodies); the roster
  totality tests were the removal's mechanical proof — the
  `pub fn` extraction and both rosters fail on either side's
  leftovers — with 2 method rows, 9 batch-module rows, and the two
  batch `SURFACE_SOURCES` dropped from the triangle suite, 10
  claim rows from the complexity roster, and the
  `version_batch_snapshot` board row retired (the board pin moved
  1090 → 1071 cells with the diff; bench mirror 1073 judged cells,
  pinned subset 321). Tests whose entire subject was the handle
  were deleted, not rewritten into vacuity
  (`representation_parity`, `batch_equals_value_level`,
  `no_arith_batch_preserves_version`, `commit_on_drop`); the
  operator-matrix differentials were re-scoped to the surviving
  cells. rumors refactor (its own public `Batch` — a real
  amortizer at the rumors layer, batching sends/redactions into
  one commit — is untouched): the C2-era census of ~8 production
  call sites had already collapsed to one — `tree.rs::act`'s
  per-action version chain, now a plain `tick`-and-`clone` loop —
  plus three test-side ticks (`tree/tests.rs`); every other
  `.batch()` in the workspace is rumors' own surviving API. No
  wire snapshot moved.
- **FINDING 2026-07-27 (#72, at the removal's gate): the
  `ff_clock_join` rejection band under-prices the full-scan
  overlap genre at small denominators; pre-existing, not the
  removal's movement.** The enforcement sentry drew a program
  whose `Clock::join` overlap rejection at 139 denominated bits
  consumed 6805 fuel against a pinned ceiling of
  ~10^(3.156+0.335) plus the 0.2 enforcement margin (~4.9k
  fuel): +0.68 decades of residual against a +0.335 width.
  Attribution measured both sides: the identical committed seed
  reads 6820 fuel at the parent (21f99a7c) and 6805 at the
  removal tip (−0.2%; the guest kernels drive public ops only
  and never held a batch handle), so the removal is fuel-neutral
  and the escape is the band's. Mechanism: the rejection band is
  one line over a bimodal arm — cheap early overlap detections
  dominate its small buckets (146 corpus samples, intercept
  0.22) while full-scan rejections hold the top (the bands
  module doc's own mixture note) — and a legitimate full-scan
  rejection at small n (~49 fuel/bit, consistent with the band's
  own top decade at ~66 fuel/bit) sits above the small-n line.
  The shrunk program is committed as the finding of record
  (`enforce.proptest-regressions`, `cc f858383f958c…`), which
  makes the enforcement leg deterministically red on this branch
  until the band learns the genre; band re-pins are the protocol
  pass's seam, so no calibration landed here — a probe run
  (discarded, not committed) confirmed the fix is evidential,
  not structural: a 4096-program corpus triples the arm's
  evidence (146 → 409 samples) and prices the genre in-band
  (width_above 0.335 → 0.550) with no slope movement
  (1.370 → 1.374). Sequencing (resolved 2026-07-27, at the merge
  round): the recalibration landed with the merge itself — the
  landing entry below carries the numbers, and the committed seed
  prices in-band under the re-pin.
- **LANDED 2026-07-27 (#77): `OwnVersion`, the lazy projection view
  — projection is lazy at every spelling, and Θ(|v|·|p|)
  materialization is only ever the explicit call.** The charter is
  `design/own-version-view.md` (every DECIDED entry an owner
  ruling); the landed shape follows it exactly. `&v / &p` and
  `Clock::own_version()` both return the ref-owning view
  `OwnVersion<'a>` in O(1); the comparison matrix (vs `Version` in
  both directions and vs `OwnVersion`, owned and borrowed, `==`
  semantic, no `Hash`) fuses projection and comparison into one
  linear co-walk (`skyline::masked`: the pair sweep's overlay
  bookkeeping at up to four cursors, with the trichotomy's
  zero-check answered by per-side running-height integrators at
  amortized O(1) per boundary — measured, not asserted, per the
  charter's semantic note); `.to_version()` / `From` are the one
  product-growth path. By-value `Div` and `DivAssign` are dropped
  (the census found only their own law and tests; those
  re-denominated through the public composition). Differential
  laws: `own_version_cmp_matches_materialized` and
  `own_version_seed_mask_coherence` (three-stream) join
  `VERSION_PAIR_PARTY`, and the new `VERSION_PAIR_PARTY_PAIR`
  group carries `own_version_pair_cmp_matches_materialized`
  (four-stream), all wired to the three law consumers; the
  projection laws re-denominated along the grain (comparison-shaped
  laws ride the fused ops lazily, laws about the materialized
  object grow `.to_version()`), and the public-surface proptests
  bind both fused walks to the recursive oracle's composed
  projection-and-compare. Instruments: the board's projection rows
  re-denominate — `version_project` → `own_version_to_version`,
  `clock_own_version` → `clock_own_version_to_version` (same
  bodies under the honest new owner; the ratified doubling-band
  cells move with them) — and the fused rows `own_version_cmp` /
  `own_version_pair_cmp` join the Projection group,
  input-denominated on every shape, the output-domination crosses
  included (comparing a projection never pays its
  materialization); smoke pin 1071 → 1111 by its per-shape
  derivation, bench mirror 1073 → 1113 full / 321 → 335 pinned
  (320 diagonal + 13 riders + the wide-display pair,
  `--list`-verified), expected-verdict roster unchanged. The
  correlated families landed as the charter's relational genre at
  higher arity: `mask_drift_triple` (cliff comb × scattered id ×
  wide plateau; owned teeth read the difference mid-cancel,
  unowned intervals the zero-check on a wide height) and
  `mask_drift_quadruple` (sparse comb under the even mask vs full
  comb under the offset mask: the parities interleave tooth for
  tooth, and the even teeth read the zero-check on a
  semantically-zero height spelled by cancelling `2^k`-wide
  digits), both full-walk `Less` by construction (generator-pinned)
  so no early exit shortens a measurement. Envelope pins
  [measured 2026-07-27, k=512, n=1024]: the triple reads 1_032
  heap / 0 segments / 6_187 limb / 16_390 scan / 4_192 touches on
  2_051 input bytes (~2 touches per stored delta, one pass), the
  quadruple 2_176 / 0 / 31_778 / 1_073_674 / 67_157 on 134_212
  bytes (~8 scan bits per input byte); ceilings ×1.25, limb/touch
  floors ×0.75, plus flatness bands across a tooth doubling
  (per-delta touches 2.05 → 2.02 and 21.86 → 21.85; per-byte limb
  3.02 → 3.32 and 0.236 → 0.236) over one-touch-per-delta
  liveness floors. **No superlinear wedge: both families landed as
  green pins** — the balanced signed-digit integrators hold the
  fused walks linear, so there was nothing to red-pin. rumors:
  `bookmark.rs` moved to the fused comparisons with zero code
  change — `reclaim`'s `extract_if` predicate
  (`clock.own_version() <= *version`, the charter's cited seam,
  verified at src/bookmark.rs:451) and additionally `is_current`'s
  suppression test (`v / p == version / p`, a second projection
  comparison the charter did not enumerate — the doc correction of
  this entry) now fuse, so the reclamation path's product-growth
  materialization is gone. Eager call sites (the additive/
  homomorphism/idempotence laws' objects, the `own_version` meet
  doctest, the oracle differentials) grew `.to_version()`; the
  paper oracle keeps its eager `Div`, as chartered.
- **LANDED 2026-07-27 (#72, the merge round): the batch
  elimination is on the tree, with the owner-approved fuzz-fit
  recalibration.** The merge integrated the removal across the
  epochs that landed since its base (the prose/protocol/suanpan
  passes, the meter families, `before::laws`, ticks(n)): the
  smoke pin moved 1090 → 1071 (its per-shape derivation
  re-stated and test-verified), the bench mirror 1092 → 1073
  full / 326 → 321 pinned (both `--list`-verified; the pinned
  split is 306 diagonal + 13 riders + the wide-display pair,
  and the mirror verification also caught the wide-display and
  amplify benches still decoding generator construction-language
  bytes as wire bytes — stale since the skyline transcode, fixed
  by routing through `Packed::version`). Recalibration: the
  corpus of record widened 1536 → 4096 programs (~985k → ~2.64M
  steps; bands byte-identical across two sweeps), the
  `ff_clock_join` rejection band learned the full-scan genre
  (samples 146 → 409, width_above 0.335 → 0.550, slope
  1.370 → 1.374), and the committed sentry seed prices in-band
  (residual +0.675 against the 0.550 width + 0.2 margin) — the
  acceptance criterion met with no seed tuning and no exclusion.
  Constants re-derived from the sweep's evidence:
  `ENFORCE_MARGIN_BELOW` 1.0 → 0.8 (at 1.0 the widened
  `ff_rank_cmp` floor dipped 0.087 decades under nop, voiding
  the liveness claim on that key; at 0.8 the narrowest gap is
  +0.113 decades with the honest 0.29-decade cheap tail still
  absorbed); `ENFORCE_MARGIN`, `REFIT_TOLERANCE`, and
  `SLOPE_ALLOWANCE` re-evidenced unchanged (worst replay ceiling
  excess +0.023, prefix divergence 0.489 over 48 of 49 keys
  covered, max healthy within-case excess +0.081).

- **REVIEW 2026-07-28 (task #37, the whole-state adversarial
  review; branch `advrev-37` @ f08cae75).** Charter: falsify the
  amelioration claim — wrong committed claims, new amplification
  families, test blind spots, Goodhart constructions — with
  committed demonstrators; instruments only, no cures. All
  measurements below are exact deterministic counters (dev and
  release read identically on every quoted number unless noted);
  the one wall-clock claim is disclosed as single runs on a
  checked-quiet machine. Findings by severity, then seed
  dispositions, the attacked-and-sound map, and residual risks.

  **F1 (wrong committed claim + roster-binding hole, the highest
  severity): `Version::min_ticks` is documented `O(|v|)` time and
  space, roster-bound `Class::Linear`, and measured superlinear on
  three committed board families through the public API.**
  [measured, release and dev identical: touch per-byte cost growth
  ×1.88 across the pure-comb doubling and ×1.66 (touch) / ×1.68
  (limb) across the reveal-comb doubling, local exponents 1.82–1.93
  at s = 4000 and still rising; ascend-cliff touch e 1.87.] The
  excess is unbounded — the pending-minima merge circulates the
  full plateau width per closing node, which the freeze allowance
  does not cap (unlike rank's per-leaf adds, which the allowance
  bounds at 8 digits) — so this is a class defect, not a constant.
  The query module's own cost section says "the excess is not
  contractual": by statement-faithfulness the public claim is
  wrong TODAY, and every committed binding leg is structurally
  blind to it — `linear_claims_cite_no_judge_red_row` consults
  only the bench judge's red set (wall exponents), the
  version_min_ticks bench time legs are unmeasured (this entry's
  ticks(n) OPEN item), and no test reads the board's counter
  verdicts against the roster classes. The worst artifact that
  passes: any counter-superlinear kernel whose wall constant is
  small enough to stay under the judge's fit at bench scales keeps
  a Linear rustdoc claim with every gate green. Pins landed:
  `min_ticks_pure_comb_touches_read_superlinear` and
  `min_ticks_reveal_comb_reads_superlinear_in_both_width_currencies`
  (`tests/meter.rs`, `query_superlinearity_pins`): per-byte
  growth bands (floor midway between linear and measured — only a
  class change crosses it; ceiling = measured ×1.10) over
  closed-form semantic legs (`min_ticks = k·2^b` on both combs)
  and touch liveness floors. The cure (anchor-web) or the re-class
  must move the pins, the `# Complexity` section, and the roster
  row in one change. **Categorical seal proposed**: board reds
  carry a mechanism tag (constant vs exponent), and the claims
  roster gains a third binding leg — a `Class::Linear` claim may
  cite no exponent-mechanism red cell; the §17.3 constant reds
  remain citable.

  **F2 (new adversarial family; `Version::rank`'s `O(|v|)` claim
  falsified where the board reads green): the freeze-position
  family FP(k)** — a right spine of 2k descending leaves whose
  deltas alternate a 10-digit-wide drop (over the 8-digit freeze
  allowance) and a unit drop, so the fold freezes Θ(k) times;
  every committed family fires O(1) freezes, which is exactly the
  hole. Each freeze reads the position accumulator's full digit
  span (`sign_magnitude` walks digit 0 through the top digit —
  including the never-written zero prefix below the shallowest
  leaf's mass, and the span read survives even a
  `sign_magnitude_shl`-style bottom skip since the written span
  itself grows) against the O(1)-digit compacted ones-run the
  correction product actually consumes: Θ(k²) touches on a
  Θ(k)-byte operand, built through `FromStr` alone. [measured:
  touch per-byte growth ×1.50 and limb ×1.43 across
  FP(1,000) → FP(2,000) at 73–147 KB operands, local exponent
  1.74 at FP(4,000) and rising; every committed rank family reads
  e 1.00–1.01 at the same scales.] Pin landed:
  `rank_freeze_position_touches_read_superlinear` (same module,
  bands on touch and limb, `min_ticks` closed form as the
  cross-fold semantic leg). **This also disputes the query module
  doc's residual defense (seed 3 below): on FP(k) the measure's
  value embeds no wide×dense product — the family's positions
  compact to O(1) digits and an O(|v|) accounting exists (fold the
  ones-run product without materializing the position) — so the
  work is span-read overhead, not mandatory-class.** Categorical
  seal: FP joins the board family roster (the structural product
  then prices every operation against the many-freezes genre);
  the pin carries rank until then.

  **F3 (dependency cost-table refutation, seed 1 CONFIRMED):
  suanpan's `*_shl` rows claim "amortized O(operand limbs),
  independent of the shift", and `digit_count`'s doc claims the
  top-zeroing scan is paid "inside that write's own budget"; both
  are false at the alternating shifted pair.** [measured, exact:
  1,000 `sub_wide_shl(&1, s)`/`add_wide_shl(&1, s)` pairs cost
  1,004,000 touches at s = 32,000; 2,004,000 at 64,000; 4,004,000
  at 128,000; 8,004,000 at 256,000 — exactly (s/32 + 4) per pair,
  sustained, and (s/32 + 2) on the `*_magnitude_shl` word path.]
  Mechanism: the subtraction zeroes the only nonzero digit and
  `add_at`'s exact-`top` maintenance walks the zero gap to digit
  0, funded by no operand limb and no earlier deposit; the
  matching add re-raises `top` in O(1), so the pair repeats
  forever at the same price. Exposed surface: every shifted
  subtractive entry whose operand is narrower than the gap under
  it (`sub_wide_shl`, `sub_magnitude_shl`, `sub_accum_shl`, and
  the add twins against a negative held value). Pin landed:
  `alternating_shifted_writes_pay_the_zero_gap_per_pair`
  (`suanpan/src/tests.rs`, exact totals at two shifts + value
  legs). Can `before` be driven onto it through the public API?
  Not demonstrated: the query folds' shifted subtractions target
  accumulators whose top is maintained by ongoing deposits, and
  the board's touch column would price a hit on any committed
  family; constructing a stream that oscillates a fold total's
  top digit across a wide gap remains a residual risk (below).
  The cure (a lazy top watermark or gap-aware maintenance) must
  re-pin the test and re-derive the crate page's `*_shl` rows and
  the `digit_count` scan claim in one change.

  **F4 (wrong prose + unmetered work, the fold index):
  `Party::join_all`'s `# Complexity` says each input is
  overlap-tested "in `O(input)`", and the `IdIndex` module doc
  says "the fold's total is linear in its operands" one clause
  after granting "one `O(log n)` table search per both-present
  node visited" — self-contradictory as written; on
  both-present-rich populations the up-front tests cost
  Θ(Σ inputs × log |self|), which `O(D log k)` does not bound
  (k = 2 already exceeds it). The searches are `step!`-free and
  scan-unmetered — the #39 F2 genre — so no committed counter can
  see the term. [measured, wall, single runs, quiet machine,
  release: on the parity-halves pair (every internal node
  both-present in both operands) at d = 16 → 18, the cursor
  predicate `is_disjoint` reads 416 µs → 1.69 ms (×4.05 on ×4
  input) while join_all's index-build + indexed test reads
  495 µs → 2.81 ms (×5.67), i.e. ×1.66 the cursor walk at d = 18
  and growing — the indexed test is also a per-test regression
  against the walk it replaced on exactly this population.]
  Correlated deep both-present populations are in no fold family
  (the #76 gap, hereby confirmed and concretized). No red pin
  landed: without a meter on the search there is no deterministic
  reading to pin — the instrument gap IS the finding. Categorical
  seal: meter the partition-point search (one scan record per
  probed entry), add a both-present-rich fold family, then the
  board's exponent leg carries it; the `O(input)` clause and the
  module doc's linearity sentence must be re-derived against the
  metered reading.

  **F5 (tamper hole, seed 4 CONFIRMED): the triangle roster's
  citation check accepts any same-named `fn` anywhere under
  `src/`** — `declared_fn_names` harvests every declaration
  (helpers, kernels, unrelated tests) and
  `every_cited_binding_test_exists` demands bare membership, so a
  deleted binding test whose name collides with any helper leaves
  the roster green. Witness pin landed:
  `citation_scan_accepts_helper_fns_as_binding_tests`
  (`triangle/tests.rs`) — the haystack contains named non-test
  helpers today; the seal (resolve citations to `#[test]`/proptest
  items, ideally module-qualified) flips the witness, which
  leaves with the hole.

  **Seed dispositions** (owner's rule: evaluate, don't confirm):
  seed 1 (suanpan table) CONFIRMED and extended — the refutation
  is exact and shift-linear, the exposed surface is the six
  shifted entries, and the word path differs only by the two limb
  reads (F3). Seed 2 (min_ticks `O(|v|)` vs kernel 1.5–1.95)
  CONFIRMED as a wrong claim, not a contractual excess — the
  circulation is allowance-uncapped, so no constant re-derivation
  can save the class (F1). Seed 3 (query.rs mandatory-class
  residual) CONFIRMED as a non-sequitur where it is load-bearing —
  the defense argues the *value* embeds a wide×dense product, but
  the value's bit-length is width + log(mass), not their product,
  and FP(k) realizes superlinear fold work on a family whose exact
  value is computable in O(|v|) (F2); the defense's true content
  is one sentence — "no committed family fires ω(1) freezes" —
  which is a coverage gap, not a lower bound. Seed 4 (triangle
  bare-name scan) CONFIRMED (F5).

  **Attacked and sound** (the negative space of the findings):
  rank on every committed family is genuinely linear at and above
  the board scales — the sub-default reveal-comb readings are the
  freeze allowance's bounded constant regime (per-leaf adds capped
  at 8 digits), collapsing at b > 256 bits, measured non-monotone
  then flat e 1.00 to s = 4000, so the board's green there is
  honest; min_ticks on FP(k) is flat (e 1.00 both counters — the
  two folds' defects are disjoint, and each family catches exactly
  one); `meet_all`'s uncelled shrink argument holds (the
  accumulator never exceeds the smallest operand seen, so every
  step is bounded by its own input); the fold subadditivity
  escape is closed (join outputs ≤ sum − 2, so no
  composition-driven operand blowup exists); op-schedule attacks
  reduce to the family axis — the only inter-call state is the
  value, the #38 orbit pins bound value trajectories, and any
  reachable value shape is a single-call family instance;
  `OwnVersion`'s claims denominate the materialization by `|r|`
  explicitly (read, not probed); the suanpan sign-fold collapse
  amortization and the domination certificates verified sound by
  code reading against their stated invariants; the text κ
  two-leg criterion and the board's determinism tripwires were
  read and not re-attacked (each already carries a
  constructed-adversary refutation history). Not attacked:
  `before-viz`, the wasm demo surface, `rumormill`, and rumors'
  session layer beyond the bookmark seam (out of the charter's
  claim basis).

  **Residual risks (open, no demonstrator):** (1) a public-API
  stream driving a query fold's own accumulator onto F3's top-gap
  oscillation (the shifted subtractions exist on the rank/distance
  paths; not constructed); (2) the distance/lag co-sweep's settle
  and promotion charges under a many-freezes family — FP(k)'s
  two-operand analogue against the anchored-segment accounting —
  untested (the jump-pair wedge covered crest freezes, not
  span-read growth); (3) the bench judge's 10 µs floor plus
  fit-noise band could hide a superlinear term whose constant is
  below resolution at both bench scales on cells with no counter
  leg (`version_eq`'s byte-equality NA row is time-leg-only);
  (4) F4's unmetered searches generalize: any future index-shaped
  structure inherits the blind spot until search probes are a
  metered primitive.

## 13. The metering gate

The board (`before::meter::board`, `just amp-board`, runner
`examples/amp_board.rs`): a red-green matrix over the entire
public operation surface × §2's families — **1071 cells**,
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
  party join is the gate; `clock.rs`).
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
321 cells: the 319 designed-pairing board cells derived by rule
from the axes (`board::BenchMode::Pinned`: each shape's
designed-stress groups, the organic control, and the board-red
riders; count verified against the criterion `--list`) plus the
wide-display pair; `BOARD_BENCH_MODE=full`,
`just bench-judge-full`, times the whole 1071-cell product plus
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
regime. Population at this tip: **the permanent schoolbook
tripwire, the hugeleaf display pair, and the cross-stream freeze
wedge (`version_distance/jump-pair`, red until the fused
co-sweep cure lands — §12's 2026-07-27 instruments entry, with
its dark-time-leg caution: a green time leg before the cure
means the leg went dark); boundary empty** (item
11's realization, 2026-07-27 — §12's P5 closeout record: the
fifteen bigroot expectations left on the banked flip evidence and
read e 0.92–1.04 live at the realization run; the display pair
stays — its conversion-dominated hugeleaf-width render measured
e 1.39/1.42 at the general 1.3 ceiling, so the κ hand-off did not
cure the cells and the class question stays open with the text
column). Every other cell — the designed diagonal and the
populated `BOARD_RED_BENCH_RIDERS` alike — must fit under its own
ceiling: a constant-factor counter red is not a time-exponent red
(the thirteen riders measured e 0.93–1.18).

**Numbers of record** [measured 2026-07-27; release profile,
single runs per scale under the determinism tripwire — the
`board-merge72-{lo,hi}.txt` renders, at the tree carrying the
ticks(n) landing, the co-sweep cure, and the batch removal]:
board **1041 green / 30 red at the default scale; 1044 / 27 at
×4** over **1071 cells** (the `version_ticks` row's 19 cells in,
the `version_batch_snapshot` row's 19 out — green at both scales
when they left; the red roster names no batch cell). The
acceptance sweep's final renders re-baseline the full board. The
red roster, every red with exactly one owner, is §17.3; the
cell-count and verdict lineage across the campaign's rounds
(200 → 989 → 1071 → 1090 → 1071)
is in git history at the commits §14 names.

**Acceptance (the campaign's; protocol per §12's ratification):
all-green means the release-profile board green on counters and
floors at BOTH scales, one run each under the committed
determinism tripwire (the runner's in-process double measurement
plus the gate's cross-process byte-compare), AND the bench judge
roster-satisfied at both scales in both modes** — at the roster
membership current at the sweep (the permanent schoolbook
tripwire and the hugeleaf display pair — where both regimes read
satisfied at the item-11 realization, record wall 33 min 27 s —
plus the cross-stream wedge entry, which must read green through
a live time leg once the fused co-sweep cure lands; record
sampling belongs to this acceptance sweep alone — the standing
cadence judges in quick mode). A release record-scale run of the counter
board costs ~20 s wall [measured — the ratification baseline
runs]; dev runs remain a debugging view and never satisfy
acceptance.

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
  P4.2-owned — the linear acceptance was met on the work columns
  (closed at P4.2: segments → 0).
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

**The 2026-07-27 wave (in flight or decided; each lands its own
§12 entry at merge, so this list carries obligations, not
outcomes):**
- **The fused distance/lag co-sweep cure** (owner of §17.3's
  cross-stream freeze wedge): five committed readings move in one
  commit — the board cell at both scales, the
  `DISTANCE_JUMP_PAIR` envelope, the growth floor flipped to a
  flatness bound, the judge roster entry through a live time leg,
  and `Version::distance`'s complexity re-class — with the
  accumulator's zero probe renamed to carry its one-sided
  contract.
- **The lazy projection view** (decided 2026-07-27; charter of
  record `design/own-version-view.md`): `/` and
  `Clock::own_version` return a ref-owning `OwnVersion<'a>`;
  comparison fuses projection into linear co-walks;
  materialization explicit via `.to_version()`/`From`. Sequenced
  after the three items above; before the legibility and
  adversarial-review passes.
- **The fuzz soak** (owner re-scope 2026-07-27): the before-side
  targets including `fuzz_laws`, run at the acceptance sweep.
  Frame-level fuzzing of `rumors` is deferred to a future
  campaign; its spec (`design/rumors-frame-fuzz.md`) is the
  record that campaign resumes from.

**C3's bench-harness remainder (the queue of record, items
11–13).** Items 1–12 of the round's queue are done (§14's C3
entry; the cell-exact movement is §17.3; items 11 and 12 are
§12's 2026-07-27 P5 closeout record — the closeout re-measure
found the twelve event-side rows byte-identical to their pinned
records, so the deferred one-downward-re-pin resolved to no
movement). Remaining:

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
  default-scale heap exponent): not a shrink-discipline target —
  the peak is the mid-walk realloc's old+new coexistence, already
  set when finish runs — and not a reserve target either, since
  the projection's output is not size-derivable from its
  operands. CLOSED (owner ruling 2026-07-27): the doubling-chain
  band is RATIFIED as the stated-band residual — no pre-walk, no
  segmented output (§12's capacity-phase finding carries the
  pricing and the dated resolution).
*Acceptance*: the two render cells above flip green at both
scales with byte-identity across the differential suite;
movement annotated against the parent boards; any κ movement
re-derived at the constant. The projection pair is closed by
the owner's ratification and stays on §17.3's roster as an
accepted stated-band residual.

**The fold marginals — the n-cursor merge (C2-adjacent).** The
V7 reduction's n·log n reads marginally red against flat ceilings
on the fold cells (§17.3's fold-marginals genre). Candidate cure:
an n-cursor merge replacing the binary-counter reduction's
re-comparisons. *Acceptance*: the owned cells flip or the n·log n
optimum is recorded as the problem's own (the asymptotic bar's
fundamentally-superlinear clause) with the ceiling re-derived.

**P4.2 — word-scale scanning (the conversion half landed;
§12's 2026-07-26 P4.2 entry is the record).** The remaining open
half: apply the word-at-a-time subtree skip (popcount
pending-counter delta, mid-word zero-crossing exit) to `idbits`
and the skyline topology stream where benches justify — a
constants option no red owns. *Sequencing, resolved at #24
(2026-07-26)*: the id predicates stay on the lockstep walk
(§17.5), which is exactly the shape the word-scale skip fits;
`diff`'s sweep enumerates leaves and never skips, so the skip
does not touch it. The skip also interacts with the fused tick's
route fold, which reads each skipped id subtree per 2-bit tag on
leaf-under-internal-id arms — a correctness seam, kept per-bit at
P4.2, and the spec's §9 round-8 table carries the landed
interaction baseline. *Kills*: none (constants). *Acceptance*:
benches.

**P5.1 — envelope finalization**: the envelope leg is done
(§12's 2026-07-27 P5 closeout entry — the suite re-measured
whole, three rows tightened including `ID_WITHOUT`'s final
ratchet). Remaining: the board-ceiling leg — board ceilings
tightened to final constants at record scale (release, single
runs under the determinism tripwire) — which belongs to the
acceptance sweep, after the in-flight wave's cells settle.

**P5.2 — proportional fuzz cap**: counting-allocator harness with
a hard ceiling proportional to input size across all fuzz
targets; the seed-writer + canonicity check join `just all`.

**P5.3 — stacker-removal audit**: P4.2 established zero
remaining library-path depth recursion (`descend!` is test-only;
§12's P4.2 entry is the record), so the audit's condition is
met. Remaining: drop `recurse::descend!` and `stacker`,
re-denominate `clock::tests::deep_tree_stack_safety`, and update
the crate's AGENTS.md hard rule in the same change — or record
which test-only sites stay and why.

**P5.4 — documentation closeout (user sign-off, item by item)**:
the §6 invariant statement lands in the crate docs now that it is
true — over content bits for content-materializing operations,
packed operands for delta-native ones, the denomination stated as
contract, every cost claim carrying its epistemic status; the
`Key` stability promise in `rumors`' `src/tree/key.rs` gains its
same-code-version qualifier; the bookmark version-mismatch
semantics; `before`'s crate-doc Efficiency section re-measured
under skyline with `just readme` re-derivation. The prose
improvement pass landed 2026-07-27 (`86e85420`, the
four-quadrant docs); the owner's sentence-level copy-edits
remain open and are explicitly non-blocking (owner note,
2026-07-27).

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

### 17.3 Owned-red accounting (current; over the 1071 cells)

Sums [measured 2026-07-27, the `board-merge72-{lo,hi}.txt`
renders — the tree carrying the ticks(n) landing, the co-sweep
cure, and the batch removal; the prior boards of record read
966 + 23 / 969 + 20 over 989 cells]:
**default 1041 + 30 = 1071; record 1044 + 27 = 1071.** The
seven reds beyond this roster's named entries are the ticks
landing's (`version_min_ticks` × {ascend-cliff, ascend-plateau,
cliff, mirror-wide, pure-comb, reveal-comb} and the new
`version_ticks` row's ascend-cliff — the counter genres its §12
entry enumerates), absorbed into this roster at the acceptance
sweep's re-baseline. Every red
has exactly one owner and the sums close; the per-round movement
lineage (each round's flips, bucketed by mechanism, with every
untouched cell verified byte-identical) is in git history at the
commits §14 names.

The red roster, both scales enumerated from the renders:

(Resolved 2026-07-27, at the co-sweep cure's merge: the
`version_min_ticks × jump-pair` scan-floor red the cure's
parent-attribution runs surfaced — the saturating fold exiting
early while the floor declared a full-stream scan — was cured by
the exact `min_ticks` fold of the ticks(n) landing, which scans
the full stream; the merged renders read the cell green at both
scales with the scan floor exactly satisfied.)

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
  I/O-linear on every column; peak heap is the output builder's
  doubling chain anchored at the operand-size reserve —
  ≈ 3·(n+m)·2^(k−1) bytes, k = ⌈log2(output/(n+m))⌉, fitted
  within 2% at every probed point — with the default probe pair
  straddling a k step (e 1.38) and the ×4 pair inside one
  (e 0.70). The instrument wobbles, not the kernel — the reading
  is honest and stays red rather than softening the ceiling. The
  projection's output is not size-derivable from its operands
  (mandatory Θ(|v|·|p|) on Θ(|v|+|p|) input), so no reserve-once
  bound exists. Owner: **an accepted stated-band residual**
  (owner ruling 2026-07-27: the doubling-chain band is ratified
  as-is — §12's capacity-phase finding carries the pricing). The
  decided lazy projection view (`design/own-version-view.md`)
  additionally makes the materialization explicit-only, so the
  band will be reachable solely through `.to_version()`.
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
- **The ascending-cliff tick pair's heap constants** (64.1/66.3
  B/B, e 1.00, ~1 B/B of it the iterative walk's frame bits — the
  one committed family holding k
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
Bench riders (`BOARD_RED_BENCH_RIDERS`) are populated (item 11's
realization, 2026-07-26 — §12's P5 closeout record): the 13
standing reds above that the designed pairings do not already
time — the display and `min_ticks` rows on the tick-cross and
harmonic shapes — each keep a judged time leg in the pinned bench
subset, and every rider must fit under its own ceiling (the reds
above are counter constants, not time exponents). A rider and its
roster expectation move as one reviewed diff, judge-verified.

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
  byte-identical (518 219 B peak, 0 segments). The 26 green→green
  re-metered cells are the enumerated verdict-neutral class
  (ratified by owner, 2026-07-26).
- **The accumulator is the workspace crate `suanpan`** (§12's
  two 2026-07-27 extraction entries are the record). Standing
  policy: unpublished until a second consumer stabilizes the
  API; its amortization contract is subtle — reads mutate.
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
