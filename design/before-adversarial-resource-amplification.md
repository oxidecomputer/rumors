# `before`: adversarial resource amplification in Version and Party computation

Status: execution in progress on branch `before-hardening`. The
audit (2026-07-22), the Tier 2 decision (2026-07-23), the pre-flip
kernel window (2026-07-23/24), C0 (2026-07-24), and C2 — the flag
day, commit `91fac33d` (2026-07-25) — are complete; the campaign
stands at C3 (§17.2), then the P4 residual audit, then P5 closeout.
This document was compressed 2026-07-25 (user directive): it
specifies the criteria the instrumentation enforces and the work
that remains; landed-work narratives, superseded amendment chains,
and measurement logs whose conclusions are pinned in code live in
git history (`git log design/before-adversarial-resource-amplification.md`),
which is the provenance record. Completed phases appear as the
ledger in §14.

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
operation costs**? Answer: yes — seven amplifier classes were
found (§3), all removable, because every quantity the algorithms
need at a node is either one of two global accumulators or bounded
by that node's own coded size. The fix of record is the Tier 2
skyline representation (§10), DECIDED 2026-07-23 and the shipped
production coding since C2 (2026-07-25).

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
| comb-scatter | cliff comb × scattered party | output-dominated projection |
| benign | small organic values | the control; the parity floor's referent |
| nested-full-sibling | `(x,1)` repeated down a spine × matching event spine | the paired tick walk at maximal shortcut depth (the fill red pin's family; scan-linear since the cure) |
| nested-wide | bigroot magnitude × the nested-full id | the wide right-full chain: the absolute first payload nets the deepest subtree and every ancestor's materialized sum re-touches it (#34) |
| mirror / wide tail | `(1,x)` down a right spine × a zero spine with one wide tail leaf | the memoized pre-scan at full depth — wide minima in every memo entry, wide nets per level (#34); the unit-tail cross is the memo machinery's own cell |
| descending staircase | monotone-descending unit-delta leaves × the unary id spine | full-penetration minimum updates at every level, width-independent (the cure's propagation witness) |
| memo chain / comb | consumption-sibling and interleaved left-full site forests | the ledger's linearity on consumption-order adversaries (every refuted memo resolution reads quadratic) |
| memo fan-out / oscillation | one wide minimum shared by k sites over a unit plateau; minima alternating wide/narrow | the fan-out's k-independent wide cost (absolute touch ceiling) and its funding control |
| memo churn / descending raises | in-flight records under full-penetration drops; raises landing below the frame minimum | one live ledger head (the live-anchored followers' tombstone); the decide-then-emit ordering's oracle tripwire |
| memo chain | `k` consumption-sibling single-leaf left-full sites under one covering site, minima distinct or shared | the memo resolution's touch cost (#34: quadratic re-reads, red-pinned; the shared twin is the flat control) |
| memo comb | shallow and covering left-full sites interleaved per level | consumption order Θ(d) from recording order — refutes chain-walking resolutions under every record-to-record anchoring |
| reveal comb / hifloor | `k` sibling left-full sites sharing one `2^b` minimum over a zero floor, the left-leaning spine closing each site's frame back into the floor frame between consumes; the control's floor raised to `2^b − 2` | the tick walk's width-circulation cure (#34 rounds 5–6: read Θ(k·b) touches on a Θ(k + b) input AND output until the latent boundary register landed; pinned flat ×2.00 across the joint doubling since, 2026-07-26); the narrow-gap control is flat — the wide gap was the driver, not the shape |
| pure comb | the same left-leaning comb with bare `2^b` leaves and no left-full site anywhere | the base watermark stack's own arm-move + close-pop cycle in isolation (~2 wide folds per site until the register landed; pinned flat per byte since, 2026-07-26) — the layer the frame ledger amplified ~10× |
| ascending cliff / plateau | `k` ascending wide left leaves `2^b + i` down a right spine over a terminal 0-cliff, id descending to the cliff; the control's leaves leveled at `2^b + 1` | the undercut cascade's fold direction (#34 round 7: one wide residue through k − 1 nonzero unit differences read Θ(k·b) touches on a Θ(k + b) input AND output until propagate's hops folded dying-side digits under domination decisions; pinned flat ×2.00 across the joint doubling since, 2026-07-26); the leveled control (differences all zero, the residue passes the compressed run whole) is flat — the nonzero hop schedule is the axis, not the undercut or the spine |

## 3. Findings ledger

All cured except the entries marked OPEN; each family
above witnesses at least one. Mechanism detail and measurements are
in git history; the cures are pinned by the enforced envelopes and
board cells named.

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
  benign/scatter fold cells — owned, C2-adjacent n-cursor merge
  (§17.3).
- **Fill's lookahead/pre-scan terms** (found post-C2 by review:
  worst case O(|ev| × local-id-depth), quadratic on matched
  spines with zero shortcut sites; the multiplier is the LOCAL
  id's depth — not wire-suppliable by a hostile peer): cured
  2026-07-25 under the nested-full red pin — the right-full arm
  deferred to an O(1) peek at the cursor's arrival, the left-full
  pre-scan memoized (no position scanned twice). Scan columns
  e 1.00 both arms; the recursion-depth segments residual rides
  P4.2's pin (§17.3).
- **Fill/tick's limb-dimension re-touching** (2026-07-25; found
  by the linearization's adversarial review, widened by the spec
  probes): materialized per-subtree `(min, net)` magnitudes cost
  Θ(width) limb work per ancestor, quadratic on wide × deep
  crosses through BOTH shortcut arms. CURED for the walk by the
  anchor-web/watermark discipline of
  `design/before-tick-cost-spec.md` (the wide crosses read limb
  and touch e 1.00 at flat constants, board- and
  envelope-pinned). The residual its own
  adversarial review found — the memo's site resolution at Θ(k²)
  accumulator digit touches on consumption-order adversaries, in
  a currency the limb column cannot see — is CURED by the frame
  ledger (the spec's §9 round-4 record): one link per site,
  sibling-chained, first-child links deferred to the parent's
  close, zero links unstored, each link read once and dying into
  its raise decision [measured: ×2.00 touch growth across the
  doubling on the memo-chain and memo-comb families, ×3.94/×3.92
  under the refuted chain — the gate pins flipped with the cure,
  re-pinned never deleted]. The same machinery had carried a
  semantic staleness bug the families' first differential
  crossing caught (fixed, minimized seed committed). Four ledger
  adversaries guard the cure: the wide fan-out (k-independence by
  absolute touch ceiling), the oscillating funding control, the
  churn family (one live head through full-penetration drops —
  the refuted live-anchored followers' tombstone), and the
  descending raises (the decide-then-emit ordering's oracle
  tripwire, verified live). A pricing obligation under §6, not an
  exploit (local-depth multiplier).
- **Tick's width-circulation cycle** (2026-07-25; found by the
  frame ledger's adversarial review; **CURED 2026-07-26** — the
  latent boundary register, spec §9 round 6: closes move the popped
  boundary into a per-stack register instead of folding it, arms
  recycle the register at the narrow anchor-relative offset, and
  followers ride a one-bit anchor-relative tag, so the cycle's
  marginal cost is the unit inter-site movement; reveal_comb
  738,449 → 2,884,881 touches (×3.91) became 48,857 → 97,705
  (×2.00 exactly), pure_comb per-byte 50.8 → 82.0 became 5.18 →
  4.46, both re-pinned flat with absolute bands, the hifloor
  control's band tightened to its cured measurement, byte-identity
  and board sums unchanged at both scales. The refutation record,
  kept as found): on a shared-wide-minimum comb whose
  spine closes each site's frame back into the floor frame between
  consecutive consumes, the consume decision mints a width-`b`
  boundary difference, the site's close pops it back into the base
  stack and the relation follower, and the next consume re-mints
  it — every object individually create-once/read-once/die, the
  width circulating through per-object-legal moves with no input
  delta, no output code, and no undercut descent funding any hop
  (I4's funded-cascade clause enumerated undercut hops only;
  L1/I4 reopened and T-tick read REFUTED-pending-revision at spec
  §9 round 5, resolved by round 6's I4′ width-conservation
  invariant). Θ(k·b) accumulator touches on a Θ(k + b) input whose
  output is Θ(k + b) too, so the blowup survives the I/O
  denominator: reveal_comb per-byte 146.9 → 267.7 → 478.7 → 808.8
  as `b` doubles at k = 1,000; ×3.91 touches on ×2.00 input across
  the joint doubling — gate-pinned ≥ ×3.5 until the cure landed
  and re-pinned it flat. Attribution pinned at both layers:
  pure_comb (no left-full site, no memo, no pre-scan) paid ~2 wide
  folds per site in the base watermark stack alone (per-byte 30.4 →
  50.8 → 82.0, pinned ≥ ×1.45 per doubling until the same
  re-pin) — the defect predated the
  frame ledger, whose follower ferry amplified it ~10× (~21 wide
  folds per site on reveal_comb). The high-floor control
  (identical forest and cycle, consume-time gap 2) pins GREEN flat
  and width-independent: the wide gap was the driver. Semantics are
  exact everywhere (oracle differentials across the pools plus a
  4096-site closed-form witness; every pin pairs its cost leg with
  the shape's closed-form tick). The executable emit model
  reproduced the class unmodified, at the base layer's constant —
  the cure round calibrated against it. Board tick-cross
  rows at both scales read green on every counter column (the
  touch currency rides no board column); the four
  `version_tick`/`clock_tick` × `reveal-comb`/`pure-comb` time-leg
  cells joined the bench-judge roster as owned reds at the
  refutation and leave it with the cure's linear readings. A
  pricing obligation under §6, not an exploit (the shape needed the
  local id's site forest).
- **Propagate's fold direction** (2026-07-26; round 6's disclosed
  kernel–prose divergence, constructed into a reachable family by
  the round-6 landing's adversarial review; **CURED 2026-07-26** —
  spec §9 round 7): the undercut cascade folded the wide surviving
  residue into each popped narrow dying difference — the surviving
  side's digits, re-read per hop — where width conservation (I4′
  rule 2) demands the dying side's. The ascending cliff (k
  ascending wide leaves stacking k − 1 nonzero unit differences
  under one wide terminal undercut) read Θ(k·b) accumulator touches
  on a Θ(k + b) input whose output is Θ(k + b) too: 203,435 →
  790,851 (×3.89 on ×2.00 input) across the joint doubling,
  gate-pinned ≥ ×3.5 until the cure re-pinned it flat at 12,626 →
  25,234 (×2.00 exactly, band 31,542/18,925). The cure inverts the
  hop: top-index domination decides each hop's direction in O(1)
  before any fold, the dying side funds the fold that consumes it,
  and width guards keep comparable-scale hops on the old path at
  zero extra touches — every other committed MEASURED reading
  byte-identical across the cure, byte-identity across the full
  differential suite. The same defect was a heap amplifier (each
  popped difference's buffer widened to residue width: board heap
  e 1.82 → 1.00 on the family). The leveled control (differences
  all zero) is flat and byte-identical across the cure: the
  nonzero hop schedule is the axis. Witness-axis advisory recorded
  (spec §7): undercut families need both a depth axis and a
  residue-width axis — the staircase descends (every residue
  narrow), and this family is its ascending mirror. A pricing
  obligation under §6, not an exploit.
- **OPEN — plateau projection output-domination** (2026-07-26;
  materialized by the #35 board product, which crossed every
  shape with every operation its bundle supplies): projecting a
  shared-wide-plateau event through its site-owning comb id
  (`version_project`/`clock_own_version` × {reveal-comb,
  reveal-hifloor, pure-comb}) re-materializes a wide absolute
  value per kept site — mandatory output Θ(k·b) against a
  Θ(k + b) input, exactly the comb-scatter case §6 already prices
  by `n_io`, but these six cells read under the input denominator
  and are red on every column at ×4 [measured — board rows at the
  #35 tip, both scales]. The reading is a denomination gap, not a
  kernel finding; the cure is declaring the output domination
  (output read back from the actual result, §6's rule) at the
  owning phase, never a silent green. Owner: the §6 denomination
  criterion at C3; §17.3 carries the cells.
- **Profile-dependent meter readings** (2026-07-26; found by the
  #35 dev-vs-release identity check, which stopped the planned
  `--release` switch of the board recipes pending the owner's
  ruling): 102 `debug_assert!` sites in the production kernels
  perform metered work — `Base` comparisons through the limb shim
  (`codec/base.rs`'s subtraction-underflow and shift guards),
  skyline grow probes that consume metered cursors — so dev and
  release builds measure different programs: at the default scale
  limb readings differed on 95 of 720 cells, scan on 59, heap on 3
  (release lower; no verdict flips; denominators byte-identical)
  [measured — dev and release renders at the #35 tip,
  byte-compared]. RESOLVED by the owner's ratification (2026-07-26,
  §13): **release is the board's measurement of record** — dev
  counters measure algorithm plus verification scaffolding, release
  the production work alone, the honest denominator — with dev runs
  a debugging view whose readings are never pinned; assertion-scoped
  meter suspension REJECTED on doctrine (a metering-pause mechanism
  is an F2 hazard). The one assertion whose metered work moved a
  rendered exponent class — `id_is_empty`'s normal-form check
  re-parsing its whole input through the metered cursor, doubling
  every decode path's dev scan (16.0 vs 8.0 bits/B on
  `party_decode`) and carrying `clock_decode × comb-scatter`'s scan
  leg across the 1.15 ceiling in dev only (e 1.25 vs 1.13) — was
  repaired per the ratified policy: the helper spot-checks the O(1)
  consequences of its contract, and the full O(n) assertion moved to
  the diff kernel's emission seam (`Party::without`), the one caller
  whose input is not just-parsed bytes, where the detection value
  genuinely requires the expensive form (owned in §17.3). Post-repair
  dev-vs-release divergence: zero verdict flips and zero exponent-leg
  ceiling crossings on any work column at either scale, scan
  divergence 59 → 13 cells (the owned seam) [measured — post-repair
  dev and release renders, byte-compared]. The record-scale segments
  columns remain profile-dependent by codegen (release frames are
  smaller, so onset shifts and counts drop; every affected cell reads
  red on segment count under both profiles) — the recursion-depth
  genre P4.2 owns, not assertion work.

## 6. The design invariant and the denomination criterion

Adopted as the crate's contract, enforced by §13:

> **No operation materializes transient state asymptotically larger
> than its packed operands, and every operation remains amortized
> `O(n + m)` in the packed input bits — with no bound on value
> magnitude, tree depth, or encoded size.**

Denomination (the criterion of record, 2026-07-23, ratified with
the DECIDED entry; Gate B): "packed operands" denominates every
operation *except* the two classes whose mandatory output is
asymptotically larger than any constant times their input — an
input-only bound is unsatisfiable by construction there and would
degenerate into exemption holes:

- **Text I/O** (`Display`/`FromStr` for Version/Party/Clock):
  judged against `n_io` = packed input + text output (Display) or
  text input + packed output (FromStr), output read from the
  actual result. The limb column carries **two legs**: the
  *constant* leg against the radix-work denominator
  `R = n_io + Σ digitsᵢ × limbsᵢ` (the schoolbook conversion cost
  law) at κ = 0.25 limb/`R` unit — provisional [derived: 4× under
  measured schoolbook, ~4× over the D&C extrapolation], re-pinned
  from the observed meter at record scale at C3 (the κ rustdoc's
  hand-off) — and the *exponent* leg against `n_io` (never `R`, on
  which any schoolbook converter reads a flat ~1) at the unchanged
  1.15. Two legs because each catches what the other cannot: the
  chunked-schoolbook refutation (2026-07-23) demonstrated a
  still-quadratic converter slipping under κ, so the exponent leg
  enforces the complexity class and κ the constant. Both
  anti-softening tripwires are committed in `meter::board`'s suite
  (the digit-by-digit parser must exceed κ; the chunked probe,
  driven through `evaluate` itself, must slip under κ and read red
  on exactly the limb exponent). An output-honesty ceiling closes
  the pad-the-output door — re-denominated at C2 from wire bits to
  radix units (forced by the delta coding; derivation at the
  constant, tripwire re-pinned).
- **Output-dominated projection** (`version_project`/
  `clock_own_version` on comb × scattered-party): judged against
  `n_io` = packed input + packed output (canonical coding cannot
  be padded).

Everything else stays input-denominated — both codec directions
(canonical 1:1), all scalar/comparison/query rows, and the
packed-output mutators, whose input denomination rests on the
1-Lipschitz property pinned in `meter/tier2` (output boundaries ⊆
union of the inputs'; total bits within 4 per input leaf of the
inputs' sum) rather than assumed. `meter::board`'s module doc
carries the do-not-re-denominate list. Rank rows denominate
against value content `bits(num) + exp`, which every public
construction path bounds by the producing wire.

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
- **SUPERSEDED 2026-07-24: min-of-K wall hardening** — the board's
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
  rounds on the design document itself until convergence (a round
  with no falsifications), a lateral-redesign fork on
  unsatisfying local optima. Performance within the campaign's
  bars decides the design; readability is a tie-breaker, never a
  veto (the recursive oracle is the readable paper-faithful
  reference, the differential suite carries semantics, the
  kernel's prose explains the walk against the oracle's
  equations). **Fused tick is pre-approved** given linearity with
  small constants. Confer-with-Finch stop conditions: a
  superlinear honest optimum; a §6 denomination change; linear
  achievable only via an at-rest representation change (the
  discovery is welcome and reported — obstruction, sketch, blast
  radius — the action always held). The spec of record:
  `design/before-tick-cost-spec.md`, converged at revision 3
  after two rounds (round 1 FALSIFIED two clauses, fixes
  validated in-harness; round 2 HOLDS with the emission
  discipline validated on the limb-faithful composed model);
  Finch's ratification lands there as a dated amendment.

## 13. The metering gate

The board (`before::meter::board`, `just amp-board`, runner
`examples/amp_board.rs`): a red-green matrix over the entire
public operation surface × §2's families — **720 cells at this
tip**, membership pinned by the smoke test — judged at two scales
(default; `board::RECORD_SCALE` = ×4, `just amp-board-record`) at
the **release profile**, the measurement of record (ratified
2026-07-26: debug assertions perform metered work, so dev boards
price verification scaffolding into the counters; dev runs are a
debugging view, never pinned), from deterministic meters only:
peak heap, grown stacker segments,
limb ops, scanned/written bits. The board is a generalized
cartesian product over three declarative axes (amendment of
2026-07-26 below): shapes declare operand bundles, operations
declare the slots their signatures consume, and every judged
quantity carries one field per metering currency
(`board::ByCurrency`), so every-shape-everywhere and
every-currency-everywhere hold structurally — adding a shape or
operation grows the product, and adding a currency is a compile
error until every operation declares a floor or a written NA for
it. **The board reads no clock**: its
entire rendered output is byte-identical at a given scale under
any machine load, no stripping, no carve-outs [measured — under a
sustained parallel-build load generator], and the claim is
enforced on two legs — the runner measures every cell twice in
process and panics on any counter disagreement, and the gate's
`just amp-board-determinism` byte-compares two cross-process
renders. Wall time is judged
nowhere in the gate; the time leg lives in the bench judge below,
at `just bench-judge` / `just all` cadence.

Ceilings: scaling exponent ≤ 1.15 (per cell, fitted across the
two scales against the cell's denominator bytes); heap ≤ 16 B per
denominator byte over an 8 KiB flat allowance; grown segments
≤ 1; limb ≤ 128 ops/byte on input-denominated rows; the text rows
per §6 (κ constant leg + n_io exponent leg); scan ≤ 96 bits/byte
on walk rows. Green = all columns within ceilings AND all floors
met.

**Liveness floors** (user ruling 2026-07-24: the board judges the
API surface as well as the implementation — a ceiling over a dead
counter proves nothing). Every cell carries a floor-or-NA
declaration per judged column, demanded by the `Cell` type and
rendered as a legend; a floor trip is red with the mechanism named
("counter reads below floor: the meter is not watching this
work"). Conventions of record: scan floors 1 bit per packed
operand byte on every row that must examine its operands
(early-exit rows floor at 2 bits — the root codes); limb floors
where big-integer arithmetic is semantically mandatory (rank
family, parsing text rows, decode rows), at one op per 64 bits of
every stored magnitude wider than 128 bits; heap floors on codec
and text rows (the result materializes at least its packed
bytes); segments ceiling-only (its honest floor is zero). NA
genres: wholesale byte moves (encode, hash, and — since main's
byte-decided equality — `version_eq`, whose exposure sentence and
time-leg backstop are on the board face), operands with no packed
stream, empty forms. A floor trip is a designed stop-and-look; an
implementation that legitimately does less work lowers the floor
deliberately. Floors have caught three live regressions to date
(the id-renderer scan vacuity; main's unmetered window fast
paths; the byte-decided equality) — the instrument works.

Scan-meter contract notes in force: the gamma window fast paths
record the same `2k+1` bits the per-bit loop prices (fast and
slow paths meter identically); the wire-side borsh `ReaderCursor`
is deliberately unmetered (no board row prices the wire path;
`codec/scan.rs` states so; instrumenting it is a conscious future
change with its own recalibration); the `max_depth` caller-side
record double-counts uniformly (2×, deterministic) and carries a
`TODO-recalibrate` for its own future commit that must re-measure
the pins pricing that walk; the `IdLeafCursor::descend` tag
double-record is in the post-flip fix round (§17.2 in-flight).

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

**The bench judge** (`tools/benchjudge`, stdlib Python;
`benches/board.rs` driven by the board's own cell table so bench
IDs mirror board cells by construction — the pinned mode times
227 cells: the 225 designed-pairing board cells derived by rule
from the axes (`board::BenchMode::Pinned`: each shape's
designed-stress groups, the organic control, and the board-red
riders) plus the wide-display pair; `BOARD_BENCH_MODE=full`,
`just bench-judge-full`, times the whole 720-cell product plus
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
tripwire; boundary empty. The four width-circulation tick crosses
(`version_tick`/`clock_tick` × `reveal-comb`/`pure-comb`) left
the set 2026-07-26 when the latent boundary register landed —
their touch currency rode no board counter column, so the time
leg was the one board-side leg that saw the cycle, and it now
reads green at the general ceiling. **The bigroot set and the display pair
empty at C3; the schoolbook expectation is permanent.** Between
C2 and C3 every judge run fails on the fifteen realized greens BY
DESIGN — that failure is C3's realization evidence, banked
verbatim at the flip (e 0.94–1.00 fitted on all fifteen).

**Numbers of record at this tip** [measured 2026-07-26; release
profile — the profile of record — limb+scan meters lit]: board
**627 green / 93 red at the default scale; 616 / 104 at ×4** over
**720 cells**, the same verdict sums as the dev renders at this
tip (the profile switch flipped no verdict at either scale)
(amended 2026-07-26, the #35 board product refactor —
**RATIFIED by the project owner, 2026-07-26**, on the two protocol
changes it carries. The board became the three-axis product above:
the hand-picked crossings dissolved into per-shape operand bundles
(a cross shape's version is its event side; its id side becomes a
disjoint party pair through the disjoint-mount adapter, one fresh
root with the shape under opposite children, so independently
generated ids never share a universe), 225 → 720 cells, smoke pin
re-derived (33 × 5 version shapes + 23 id-pair + 44 × 11 cross
shapes + 2 scatter + 46 benign). Every pre-existing cell's
rendered row is **byte-identical** at both scales against fresh
pre-refactor runs at the parent tip — the movement is exactly the
495 new cells, whose 74 (default) / 72 (×4) new reds are triaged
in §17.3 under their owning genres, none exponent-class on a
linear input axis. Protocol change 1 (ratified): the determinism
tripwire —
the runner self-verifies every cell twice in process and the
gate's `amp-board-determinism` recipe byte-compares two
cross-process renders — **replaces the two-identical-runs-per-scale
convention**; acceptance runs are single runs per scale. The
tripwire holds under release: both scales' release renders are
byte-identical across processes, and the in-process self-verify
passes on every cell [measured — paired release renders at the
ratification tip, byte-compared]. Protocol change 2 (ratified):
**release is the board's measurement of record** and the board
recipes run `--release`. Dev and
release measure different programs — 102 `debug_assert!` sites in
the production kernels do metered work (a `Base` comparison in
`Sub`, cursor-consuming skyline probes), so dev counters price
algorithm plus verification scaffolding while release prices the
production work alone, the honest denominator; dev boards stay
runnable as a debugging view, never the record; assertion-scoped
meter suspension is REJECTED on doctrine (a metering-pause
mechanism is an F2 hazard — do not build one). At ratification the
switch moved limb readings
on 95 cells (default) / 90 (×4), scan on 59, heap on 3/11, and the
×4 segment counts on 16 P4.2-owned legs (no
verdict flips at either scale; release reads lower; denominators
byte-identical). One dev-only exponent-class artifact fell under
the ratified assertion-repair policy and was repaired
(`id_is_empty`, §3's profile entry and §17.3); the segments
movement is codegen frame size, owned by P4.2, not assertion
work. The
board-red bench riders (`board::BOARD_RED_BENCH_RIDERS`) are
committed empty: populating them with the 78 unclassified new
reds would put unrostered time-exponent reds in every judge run,
so the riders land with the reds' classification)
(amended 2026-07-26, the fold-direction
cure: the ascending cliff and its leveled control joined the tick
rows at 221 → 225 cells; every pre-existing cell byte-identical at
both scales, the movement exactly the four new cells; the cure
moved the ascend-cliff heap column e 1.82 → 1.00 — the old fold
direction widened every popped difference's buffer to residue
width — and the residual reds are owned: the family's
heap-constant (k simultaneously-armed nonzero differences, one
pooled unit-width buffer each; linear, the mirror-narrow genre)
and the record-scale segment onsets on both new families inside
the P4.2-owned recursion genre; the judge roster is unchanged, the
new time-leg cells green-by-default at the general ceiling)
(amended 2026-07-26, the latent
boundary register: the round-5 tick-cross rows joined at 215 →
221 cells and the cure moved no counter cell's verdict — sums
byte-stable across the cure at both scales, movement only in the
reveal-comb heap constants, down, and the record-scale segment
onsets inside the P4.2-owned reds; the four width-circulation
judge legs left the roster, 22 → 18)
(amended 2026-07-25, the #34 cure: the anchor-web
walk and the chained-memo pre-scan flipped every #34-owned red —
nested-wide limb e 1.57/1.83 → 1.00 at 5.4/B flat across scales;
mirror-wide limb e 1.86/1.94 → 1.00 at 8.9/B and heap e
1.63/1.84 → 0.97/0.99 at 11.2/9.1 per byte, zero grown segments;
mirror-narrow heap constant 93.2/95.6 → 13.9/9.0 per byte;
staircase held green, limb 11.4 → 16.0/B e 1.00; nested-full's
limb constant dropped 20.6 → 9.1/B. The ×4 segments residual
stays — the recursion-depth genre, P4.2-owned — with counts
re-pinned deliberately to the new walk's call shape: nested-full
11, nested-wide 5, mirror-narrow 12, staircase 22, mirror-wide 0.
The four #34 judge legs left the roster with the flip, 22 → 18;
C3's run verifies the wall leg. §17.3 restates the sums)
(amended 2026-07-25, the #34 red pin: four new
tick-walk families — nested-wide, mirror-wide, mirror-narrow,
staircase — landed as board columns with full-examination scan
floors (8 bits/B) and mandatory-width limb floors on the wide
crosses; their pre-cure readings and the linearization's own
flip are in git history at the pin and cure commits). The judge's last honest reading, at the
flip commit over 202 cells: **157 green / 3 red / 42 sub-floor**,
exit 1 on exactly the fifteen banked realization violations; the
three reds all rostered-expected. Workspace sweep at C0:
1183/1183, roster retired, unqualified green since.

**Acceptance (the campaign's, re-denominated 2026-07-24; protocol
ratified 2026-07-26): all-green means the release-profile board
green on counters and floors at BOTH scales, one run each under
the committed determinism tripwire (the runner's in-process
double measurement plus the gate's cross-process byte-compare),
AND the bench judge roster-satisfied at both scales in both
modes** — at P5.5 with the bigroot set emptied and only the
permanent text expectations remaining. A release record-scale run
costs ~20 s wall [measured — the ratification baseline runs]; dev
runs remain a debugging view and never satisfy acceptance.

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
- **C0** (2026-07-24, §17.2 P3.1's landed entry `1e96e6fd`):
  rebase onto the link-transport merge; §14's sixteen-test stall
  roster RETIRED (sweep 1183/1183; the committed stall seeds are
  link-transport's regression pins; any test failure anywhere
  blocks again, no provenance carve-out); the merge-seam re-sweep
  and the meter-coverage fix round (main's unmetered fast paths
  under the campaign's floors; five-cell vacuity catch cured;
  `version_eq` re-denominated; two owned kills realized early by
  main's byte-decided equality); board 139/61 and 130/70.
- **C2** (2026-07-25, commit `91fac33d`; fill kernel `c43740b8`;
  seed-corpus cure `61d1bcd4`; §17.2 P3.9's landed entry): the
  flag day — storage flipped, every operation routed to the
  kernels, old codec deleted, 27 snapshots re-pinned (bytes-only
  review: zero blocking), `BOOKMARK_FORMAT_VERSION` 1→2 with a
  reject test, byte-pinned doctests re-run. Board 185/15 and
  184/16 over 200 cells; 49 staged kills realized, zero
  unexplained movement; the judge's fifteen realization greens
  banked. Deviations of record: output-honesty in radix units;
  three newly κ-genre cells pending adjudication (§17.2 C3);
  tier2/compactness denominated against the packed construction
  language; bridge recursions descend-guarded.
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
  adversarial review then REFUTED the limb-dimension O(n+m)
  claim (§3's re-touching entry) — the finding that opened #34
  and the tick cost spec's design loop (two attack/fix rounds to
  convergence; the loop record is the spec's §9). Acceptance
  deviation of record (2026-07-25): the ×4 segments leg stayed
  red, P4.2-owned — the linear acceptance was met on the work
  columns.

Remaining plan: **#34** (the tick limb cure + fusion per
`design/before-tick-cost-spec.md`, red pin first) → **C3**
(§17.2) → **P4.2** residual audit → **P5.1–P5.5** closeout, with
the boolean-skyline decision (#24, the user's) after C3.

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
problem's own optimum, stated and priced. The tick kernel's
limb dimension — quadratic on wide × deep crosses (§3) — is
below the bar; its cure is committed (#34, the tick cost spec).

## 17. Work items of record

### 17.2 Open items, with acceptance contracts

**In flight (2026-07-25): #34's red-pin round.** The fill
linearization's review bookkeeping rides it: the ghost prose
re-denominated to the deferral design (`fill.rs`'s
drift-accumulator sentence, the tests' drift-stack phrasing,
`nested_full_id`'s and the board family's
lookahead-and-pre-scan present-tense rustdoc); `fill.rs`'s
`# Cost` restated to the measured truth (scan linear; limb
quadratic both arms, red-pinned, cure committed) until the cure
re-derives it; the tick cells' floors raised from the generic
walk floors toward honest measured counts; #33's acceptance
deviation (the ×4 segments residual, P4.2-owned) recorded as a
dated amendment here. *Acceptance*: the red-pin contract in
`design/before-tick-cost-spec.md` §7 plus these ride-alongs;
one full gate.

**C3 — P3.10: realization verification, re-pins, and the
before/after table.**
*What*:
- Board re-run at both scales (release, single runs under the
  determinism tripwire); every
  movement against §17.3's accounting.
- **The judge roster empties its realized set**: the fifteen
  bigroot expectations leave on the banked evidence (fitted
  e 0.94–1.00 at the flip); the hugeleaf display pair resolves
  with the κ re-derivation below; the schoolbook tripwire stays
  permanently. Roster membership pin updated in the same change.
  `bench-judge` and `bench-judge-record` must then exit 0 —
  roster-satisfied — at both scales in both modes, and
  full-sampling record numbers are captured.
- **The κ re-derivation** (the text column's hand-off): κ = 0.25
  re-pinned from the kernels' observed meter at record scale;
  the ten κ-owned text reds and the display pair's ceiling-class
  question (general 1.3 vs text 1.7 — the class is the sidecar's
  to declare, changed deliberately or not at all) resolve here.
- **The comb-scatter κ-genre adjudication**: the flip recorded
  `version_min_ticks × cliff`, `version_project × comb-scatter`,
  `clock_own_version × comb-scatter` as newly κ-genre (both
  denominator sides delta-coded) — but `query.rs` claims the
  projection sweep is I/O-linear on exactly that cross. Run a
  single-cell column attribution (which column went red, at what
  reading) BEFORE accepting the classification; either the
  classification or the prose is wrong. min_ticks' case is
  arguable as a missing optimization (it reads `height_word` per
  leaf where rank's frozen/live split avoids exactly that).
- **The cliff limb-floor re-derivation**: `version_decode`/
  `version_rank`/`clock_decode` × cliff trip floor-liveness —
  the floors' per-tooth derivations predate the skyline coding's
  ~150× collapse of cliff's packed size. Re-derive from what the
  operation must do on the NEW coding; floors keep their
  derivation rationale.
- **The §17.3 reconciliation**: done 2026-07-25 (the accounting
  there is cell-exact at both scales); C3 re-enumerates after its
  re-pins and restates the sums.
- **The judgment-layer question**: `rank_sum × benign` (record)
  and `rank_pair_ops × benign` read manufactured exponents over
  sub-allowance heap / near-constant denominators. Either the
  exponent leg learns a sub-allowance guard (a criterion change,
  deliberate) or the benign operands scale with the knob (the
  P5.5-recorded one-line population change). Decide once, apply
  to the genre.
- **Envelope tightening**: every event-side envelope in
  `tests/meter.rs` re-pinned downward at sweep-earned constants
  (the twelve rows: DECODE/CMP/JOIN × DENSE/BIGROOT/HUGELEAF/
  CLIFF as applicable, TICK_DENSE) in the commit that earns them.
- **The before/after table of record** (judged under §14's two
  rulings). Protocol, mandatory: re-bench the pre-C2 tip under
  the FINAL harness — a temp worktree at the last pre-flip
  commit with the current bench files grafted on (they call only
  the public API), full sampling both tips, warm target dirs —
  **never the stored `base` baselines**, which are contaminated
  for delta purposes (the RNG consolidation regenerated every
  bench input family, and they mix sampling modes). Any benign
  regression beyond "slight" is a finding; the parity floor is
  the bar.
*Deps*: the fix round above (its window restoration moves wall
constants). *Risk*: a cell green at default but red at record —
that is the two-scale design working; the cell's owner reopens.

**#34 — the tick limb cure and fusion (committed, user rulings;
the hot path — every tick calls fill).**
The statement of record is `design/before-tick-cost-spec.md`
(converged revision 3); its §7 acceptance contract is binding and
this entry only sequences it. Target: T-tick — amortized
O(n + m) Accum digit touches — via the anchor-web discipline
(the zero-run-compressed watermark stack, anchored entries,
per-operand lifetime pricing, the L2×L6 pricing chain), then
fused tick (one walk carrying fill emission, the changed flag,
and grow's route DP; copy-on-first-divergence) as a separate
post-cure commit. Sequencing: red pin first (four wide×deep
cells through both shortcut arms + descending-staircase cells,
measured not assumed + mirror-narrow green memo cells with
honest floors + judge roster entries owned here), then the
watermark rewrite (right-full arm first), then the pre-scan/memo
conversion, then fusion. Byte-identity at every stage; all
pinned cells green at both scales; the L6 output-bound proptest
pin lands with the cure; `fill.rs # Cost` restates exactly what
is proven; §13/§17.3 amendments with sums restated; Accum
pooling per the spec's §6 constants note. No green, no merge.
*Deps*: none open; sequenced before C3's before/after table.

**P4.2 — residual recursion and word-scale scanning.**
Audit every remaining `recurse::descend!` site post-C2; convert
survivors per the explicit-stack pattern or record why they stay;
apply the word-at-a-time subtree skip (popcount pending-counter
delta, mid-word zero-crossing exit; the item earlier drafts
numbered §11.4) to `idbits` and the skyline topology stream where
benches justify. *Sequencing risk, resolve at #24 decision time*:
the word-scale skip fits the lockstep walk shape, not a
leaf-enumerating sweep — if it lands first, the predicate-sweep
constants regress relative to it; the id predicate envelope rows
re-pin deliberately under either ordering. *Kills*: none
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
three runs; judge roster-satisfied both scales both modes, only
permanent expectations); the before/after table showing the
parity floor met everywhere and improvement where claimed; the
coverage audit re-run with an empty gap list (method: walk the
board's op enumeration and the board-doc NA list; every public
operation names its two oracle legs and its resource pin — the
representation-pin leg per the 2026-07-23 directive: every
exposed type's bytes/text/serde forms snapshot-pinned in-crate);
the benign rank-pair operand scaling if C3 chose that arm; the
§14 acceptance entry recorded.

### 17.3 Owned-red accounting (current; over 720 cells)

Reconciled 2026-07-25 from a fresh board reading at the fix-round
tip (both scales enumerated cell by cell; the apparent
discrepancy dissolved — the flip entry's ×4 categories contained
the cliff and id-pair cells without enumerating them). Every red
has exactly one owner and the sums close:

Amended 2026-07-25 (the fill red pin, and its cure the same day):
the nested-full tick cells pinned the kernel quadratic — scan and
limb exponents 2.00 [measured] with constants two orders over
their ceilings, segments e 2.90 at ×4 — and the O(n+m) rewrite
flipped every work column green (scan/limb e 1.00, heap 0.3/B;
byte-identity against the recursive oracle, the exhaustive scope,
and the closed-form deep witnesses held throughout; the deep-4096
differential's wall fell 67.4 s → 1.3 s in the debug harness).
The ×4 residual: segments only, e 1.49 / 28 grown — the walk's
recursion depth, owned by **P4.2** (iterative walk, built behind
the stack-container seam and measured per the 2026-07-25
directive), red in advance by this same family. The judge legs
left the roster with the cure (C3's run verifies the wall).
Heap constants on width-carrying families moved +0.3–0.5/B, all
green: the walk's per-subtree signed returns materialize wide
magnitudes once per node — the limb-dimension re-touching §3
records and the #34 red pin below prices.

Amended 2026-07-25 (the #34 red pin): four tick-walk families
join the board (eight cells; smoke pin 207 → 215, derivation
restated there), all measured on the landed kernel, twice per
scale byte-identical, no movement on any pre-existing cell:

- `version_tick`/`clock_tick` × **nested-wide**: RED, limb
  exponent 1.57 at 43.6/B (default) and 1.83 at 126.3/B (×4) —
  the wide right-full return chain; owner **#34**. At ×4 also
  segments e 2.42 / 16 grown — owner **P4.2**.
- `version_tick`/`clock_tick` × **mirror-wide**: RED, limb
  e 1.86 at 142.9/B and heap e 1.63 at 352.9/B (default); limb
  e 1.94 at 518.9/B, heap e 1.84 at 1,098.1/B (×4) — the memo
  arm's wide chains and wide owned entries; owner **#34**. At ×4
  also segments e 2.00 / 4 — owner **P4.2**.
- `version_tick`/`clock_tick` × **mirror-narrow**: RED, heap
  constant 93.2/B default / 95.6/B ×4 at exponent 1.00 — the
  memo's one owned heap entry per left-full site, linear in
  count but a constant the ceiling honestly rejects; owner
  **#34** (the diff-coded memo). At ×4 also segments e 1.64 /
  50 — owner **P4.2**. Deviation from the spec's §7 expectation
  (green with honest floors) recorded there as a dated
  amendment: the meters read the memo's constant, and the pin
  keeps the honest reading.
- `version_tick`/`clock_tick` × **staircase**: GREEN at the
  default scale on every work column (limb 29.7/B, scan 40.0/B,
  e 1.00 — the landed kernel is linear on narrow
  full-penetration schedules; the cell holds the cure's
  propagation to the same reading); at ×4 RED on segments only,
  e 1.49 / 56 — owner **P4.2**.

Amended 2026-07-25 (the #34 cure): the anchor-web walk (stage
one, the per-subtree materialized returns dissolved into the
zero-run-compressed watermark stack) and the chained-memo
pre-scan (stage two, per-site minima diff-coded along the
recording chain, resolved by interval folds against per-level
anchors) flipped every #34-owned red at both scales —
nested-wide and mirror-wide limb/heap e 1.00 at flat constants
(5.4 and 8.9 limb/B; mirror-wide heap 11.2/9.1 per byte with
zero grown segments at ×4), mirror-narrow's memo heap constant
93.2/95.6 → 13.9/9.0 per byte, staircase and nested-full held
green with the staircase's full-penetration propagation flat
(limb 16.0/B) — byte-identity across the full differential
suite including the mirror telescoped-collapse witness, two
identical board runs per scale, scan columns unchanged on every
cell, and the only non-tick movement the tick-calling
`version_batch_snapshot`/`clock_recv` heap improvements. The ×4
segments residual (recursion depth, owner **P4.2**) stays on
eight tick-walk legs with counts re-pinned deliberately to the
new walk's call shape: nested-full 11, nested-wide 5,
mirror-narrow 12, staircase 22 (mirror-wide 0 — its walk is
shallow once the memo chains). The four #34 judge legs left the
roster with the flip (22 → 18, membership pin updated); the L6
output-bound proptest (`tick_output_is_input_bounded`) and the
`TICK_NESTED_WIDE`/`TICK_MIRROR_WIDE` envelope rows pin the
cure-earned constants. Sums: default 198 + 17 = 215; record
189 + 26 = 215.

Amended 2026-07-26 (the width-circulation and fold-direction
rounds): the six round-5 width-circulation cells and the four
round-7 ascending-cliff cells moved the base 215 → 221 → 225.
New reds, each owned: the pure-comb pair's ×4 segments (the
recursion-depth genre, **P4.2**) took the record sum 26 → 28 at
round 5; the ascending-cliff pair is red at both scales on the
heap constant (64.4/66.7 B/B at exponent 1.00 — the first
committed family to hold k simultaneously-armed nonzero boundary
differences, one pooled unit-width buffer each; linear in the
input, honestly over the 16 B/B ceiling, the mirror-narrow genre;
owner: the round-7 record, spec §9 — a small-buffer diff-stack
representation is the candidate cure if one is ever warranted)
and at ×4 additionally on segments (e 3.81 / 14, **P4.2**), and
the plateau control pair at ×4 on segments only (**P4.2**). The
64.4/66.7 constant is specific to the board's joint (s,s) axis,
where the armed count k and the per-difference width b grow
together: at fixed b the peak heap is exactly affine in k —
91.3 B per armed nonzero difference plus a 2,905 B floor, ~104
B/B on the k-axis alone — and the B/B reading drifts with the
k:b mix (49.6 → 81.3 across k-doublings) while the exponent
holds 1.00. A future re-pin of this cell must re-measure on the
board's own axis; the exponent claim is axis-invariant, the
constant is not. Sums:
default 206 + 19 = 225; record 193 + 32 = 225.

Amended 2026-07-26 (the #35 board product refactor): the product
took the base 225 → 720 with every pre-existing cell's row
byte-identical at both scales; the 74 (default) / 72 (×4) new
reds fall into five genres, each with one owner, none
exponent-class on a linear input axis:

- **The comb-scatter column** (21 default / 19 ×4: the
  merge/compare/query limb exponents, the tick pair's limb+scan
  exponents, `version_decode`/`version_rank`/`clock_decode` with
  their limb floors, and at the default scale
  `clock_encode`/`rank_sum` heap exponents and `clock_fork`'s
  scan exponent). The family scales tooth *count* at fixed
  1000-bit tooth magnitude, and the skyline coding stores the
  oscillation as unit deltas, so its packed bytes grow ×1.18 per
  level doubling while value-content-linear work doubles: every
  exponent on the column reads against a nearly flat denominator
  (log 2 / log 1.18 ≈ 4). The same value-content-vs-packed-bytes
  question as the three κ-genre exponents, now a column; owner:
  **C3's classification question**, and the cliff generators'
  leaf-delta representation question is the substance.
- **Twenty (×4; 18 default) plateau limb-floor trips**
  (`version_decode`/`version_rank`/`version_distance`/
  `version_lag`/`clock_decode` × {ascend-cliff, ascend-plateau,
  pure-comb, reveal-comb}): the tree-derived mandatory-limb floor
  (`mandatory_limbs_version`, one limb per 64 bits of every
  min-lifted stored base) over-demands on plateau shapes whose
  stream stores its width once and steps by units — a conforming
  walk provably does less limb work than the decoded tree's
  absolute values imply, the same over-derivation as the cliff
  limb-floor trips; owner: **C3's floor re-derivation**.
- **The κ-text extension** (28 default / 25 ×4:
  `version_from_str`/`clock_from_str`/`version_display`/
  `clock_display` × {harmonic, nested-full, nested-wide,
  mirror-narrow, mirror-wide, staircase, reveal-hifloor}, limb/`R`
  over κ, the dense-like shapes also over the display heap
  constant, plus `version_display × nested-wide`'s ×4 heap
  constant): per-value gamma-encode arithmetic on small-value
  trees, the exact genre of the ten pre-existing κ-text
  constants; owner: **the κ/C3 re-derivation**.
- **Six plateau projection cells** (`version_project`/
  `clock_own_version` × {reveal-comb, reveal-hifloor, pure-comb};
  every column red at ×4): projecting the plateau event through
  the site-owning comb id re-materializes a wide absolute value
  per kept site — mandatory output Θ(k·b) against a Θ(k + b)
  input, read under the input denominator. The comb-scatter
  output-domination case on the plateau crosses, needing the same
  `n_io` treatment §6 grants that cross; OPEN in §3; owner: **the
  §6 denomination criterion at C3**. Never re-denominated by the
  refactor itself (a green earned by re-derivation is not a
  migration).
- **`version_min_ticks` heap constants** (mirror-narrow at both
  scales, mirror-wide joining at ×4): the query walk's per-level
  owned heap entries on the deep left-full memo shapes — the
  mirror-narrow linear-heap-constant genre; owner: **#34's
  diff-coded memo record** (the tick-side cure priced the tick
  rows; the query walk's reading is the same constant genre).

Amended 2026-07-26 (the profile of record moves to release; the
#35 protocol ratification): the release renders carry the same
verdict as dev on every cell at both scales — the sums below are
unchanged and no red changed owner — so the movement is readings
only, re-cited at release where this section quotes them: limb
constants on 95 (default) / 90 (×4) cells (release lower; the
largest drops on the plateau-projection constants, 193.4 →
145.4/B on the pure-comb pair at the default scale), heap
constants on 3 / 11 cells (all `party_without`: the owned
assertion seam below), scan constants on 13 cells after the
assertion repair below, and the ×4 segment counts on the sixteen
P4.2-owned recursion legs — per tick op: nested-full 7,
nested-wide 2, mirror-narrow 7, staircase 14, ascend-cliff 4,
ascend-plateau 4, pure-comb 4; the id-side parser pair at
count 12 — release frames are smaller, so segment onset shifts
with codegen: the segments currency is profile-dependent by
nature, every affected cell reads red on segment count under both
profiles, and P4.2's iterative walks remain the cure. The
assertion repair (the ratified policy's one trigger):
`id_is_empty` asserted normal form by re-parsing its whole input
through the metered cursor, doubling every decode path's dev scan
(`party_decode` 16.0 vs 8.0 bits/B) and carrying `clock_decode ×
comb-scatter`'s scan leg across the 1.15 ceiling in dev only
(e 1.25 vs 1.13, the one dev-vs-release exponent-class divergence
on a work column); it now spot-checks the O(1) consequences of
its contract (root tag arity vs stream length), and the full O(n)
normal-form assertion moved to `Party::without` — the diff
kernel's emission seam, the one caller whose input is not
just-parsed bytes. That seam is the recorded, owned dev-profile
divergence: the check genuinely requires the full parse (a
collapsible node can hide anywhere in the emission), so
`party_without`'s dev scan reads ~16 bits/B above release by
design, and its dev heap carries the parse stack. Release
readings — the record — carry no assertion work anywhere.

Sums: default 627 + 93 = 720; record 616 + 104 = 720.

Default, 93 = 627 green + 93: the 74 new reds above + the
nineteen pre-existing (byte-identical through the refactor):
**ten κ-text constants**
(`version_display`/`clock_display` × {dense, bigroot, benign},
`version_from_str`/`clock_from_str` × {dense, benign} — limb/`R`
vs κ; the κ/C3 re-derivation) + **four fold marginals**
(`version_join_all`/`party_join_all` × {scatter, benign} — the
reduction's n·log n; the C2-adjacent n-cursor merge) + **three
κ-genre exponents** (`version_min_ticks × cliff`,
`version_project × comb-scatter`, `clock_own_version ×
comb-scatter`; single-cell column attribution at C3 BEFORE the
classification is accepted) + **the ascending-cliff pair's heap
constants** (the round-7 record).

Record scale, 104 = 616 green + 104: the 72 new reds above + the
thirty-two pre-existing (byte-identical through the refactor):
the eight P4.2 tick-walk
segments legs above (nested-full ×2, nested-wide ×2,
mirror-narrow ×2, staircase ×2) + the pure-comb pair's and
plateau-control pair's segments legs and the ascending-cliff
pair's heap-constant + segments legs (the round-5/round-7
records) + **the ten κ-text constants**
(as above) + **two id-side parser recursion cells**
(`party_from_str`/`clock_from_str` × id-pair — segments e
3.58/3.59, count 12 at the release profile of record: the text
parser's remaining recursive walk; owner
**P4.2**, the explicit-stack residual) + **three cliff
limb-floor liveness trips** (`version_decode`/`version_rank`/
`clock_decode` × cliff — measured ~10.4k limbs against a floor
of 16384 derived before the coding collapsed cliff's packed size
~150×; owner: C3's floor re-derivation) + **two judgment-layer
artifacts** (`rank_sum`/`rank_pair_ops` × benign — exponent legs
over near-zero denominators under flat allowances; owner: C3's
criterion question) + **one fold marginal**
(`party_join_all × benign`; the version-side pair reads green at
×4, the n·log n signature). `version_join_all × scatter` is also
green at ×4 (same signature).

### 17.5 Post-campaign docket (user directives)

- **#24, the boolean-skyline unification (the user's decision,
  post-C3; probe verdict GO-WITH-SHAPE 2026-07-24).** Under the
  skyline coding a `Party` is a boolean skyline; the id
  predicates are sweep kernels over the boolean semiring. The
  probe measured the comparison sweep 57% boundary bookkeeping
  vs 13% value plumbing (proceed condition met), found no
  type-system friction (the leaf-cursor trait's associated State
  keeps the accumulator out of the boolean instantiation), and
  confirmed one specialization: verdict logic does not
  generalize. Shape of record: genericize `advance` + `Side` +
  the leaf-cursor contract only (~70 lines); an id cursor
  (~100 lines); `covers`/`is_disjoint` as boolean folds (~25
  each), retiring the lockstep vocabulary; `sum`/`without`/
  `complement` stay on their walks (copy-splice and retagging
  have no sweep analogue). Net lines a wash; the payoff is one
  walk discipline retired. Sequencing risk vs the word-scale
  skip recorded at P4.2. If confirmed early enough, land before
  P5.4 so the crate docs present the unified model once.
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
