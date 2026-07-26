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
- **Plateau projection output-domination** (CLOSED at C3
  2026-07-26 under the owner's pre-approved §6 ruling):
  `version_project`/`clock_own_version` × {reveal-comb,
  reveal-hifloor, pure-comb} re-materialize a wide absolute value
  per kept site — mandatory output Θ(k·b) on a Θ(k + b) input —
  and are `n_io`-denominated like the comb-scatter cross. The
  owner's O(`n_io`)-tightness rider is met [measured, release,
  both scales: output ×4.0 per input doubling; limb e 0.96–0.99
  at ≤ 0.20/B, scan e 1.00 at 8 bits/B, touch e ≈ 0.99 at
  ≤ 0.13/B, heap e 1.00 at 2.1 B/B]; the six `n_io` board cells
  are the committed check. A denomination gap, not a kernel
  finding.
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
- **LANDED — the join_all up-front re-scan cure: the per-call id
  index** (found 2026-07-26 by the error-path round's rejection
  survey; cured the same day). `Party::join_all` and
  `Clock::join_all` test every input against the *fixed*
  accumulator up front — semantically load-bearing for the
  best-effort hand-back granularity — and each test as a cursor
  walk re-scanned the accumulator (no random access in the packed
  coding): Θ(inputs × accumulator) scan on a
  Θ(accumulator + inputs) operand set, e 2.00–2.14 at
  47–2,954 bits/B across the overlap families [measured at the
  parent tip]. Each *call* was honestly linear; the fold's
  repetition against one fixed operand was the amplifier, on the
  rejection and success paths alike. The landed mechanism is the
  entry's first candidate, `IdIndex` (`party/ops/index.rs`): built
  once per fold call in two linear passes (every both-present
  node's right-child position, one `u32` per such node — transient
  state strictly under the operand; a `u32`-overflow operand
  ≥ 512 MiB falls back to the cursor walk), it answers each
  up-front test in O(input) node visits plus one
  O(log accumulator) table search per both-present visit,
  addressing indexed-side children in O(1) and skipping their
  subtrees by never visiting them. A pure predicate-mechanism
  swap: hand-back contents, order, and accumulator bytes are
  decided by the identical fold, pinned differentially against
  the cursor-walk discipline preserved as `testing::fold_oracle`
  (arbitrary pool-with-repetition mixes, overlap position
  first/interior/last, duplicates, all-overlapping deferred
  witness, none-overlapping; a deliberate wrong-child mutation
  trips four differentials). The second candidate
  (coalesce-first) was rejected by probe: on the witnessing
  population itself — duplicate probes overlapping the
  accumulator *and* each other — nothing coalesces, so it
  degenerates to the same per-input tests while reordering and
  regrouping the hand-back vector the contract documents
  (an input the fixed-`self` test hands back individually would
  instead surface later, possibly merged); not curative where it
  is priced, and semantics-breaking everywhere. Numbers: the gate
  pin re-pinned in the cure's own commit,
  `join_all_overlap_upfront_test_reads_flat` — growth across the
  joint doubling ×4 (the ≥ ×3.5 red era) → ×2.00 measured
  (33,036 → 66,060 bits), pinned ≤ ×2.05 over a liveness floor of
  one full accumulator pass; the board row reads scan e 1.00 at
  15.9 bits/B on *every* family at both scales (the index build's
  two tag passes dominate), heap gaining the table's constant, at
  most 10.6/B (nested-full, ×4) under the 16 ceiling. Cell
  accounting and the success-path re-attribution: §17.3's
  2026-07-26 cure amendment.
- **The instrumentation census's blind spots** (2026-07-26; found
  by a read-only census of meter coverage hunting the F2 genre —
  work routed through a mechanism whose meter exists but is not
  pinned on that surface, or through a mechanism with no meter at
  all; every unpinned surface probed measured touch-linear, so all
  items were missing ratchets, not live amplifiers. Dispositions,
  landed by the #39 round): **the board had no touch column**
  (structural: the tick F2 quadratic would have been invisible on
  the board even at record scale) — LANDED as the fifth
  `ByCurrency` field, ceiling + floor-or-NA on every cell of the
  720-product (§13). **Emit (join/meet/recv/sync/folds) and text
  parse carried live touch counters with zero pins** — parse the
  touch-heaviest surface measured (~18× emit) — LANDED: board
  per-delta floors plus gate-side cliff-comb flatness pins over
  one-touch-per-delta liveness floors (per delta for the emitter,
  per text byte for the parse — its honest denominator), beside a
  render zero-touch conservation pin so accumulator work cannot
  migrate between the text directions silently; `text.rs`'s
  enforcement claim re-worded to name exactly what is pinned (it
  had cited aggregate ceilings that carried no touch column).
  **Validate/decode touches unpinned** — LANDED as stream-derived
  wide-code board floors (the validator batches word-scale deltas
  in the accumulator's lazy zone: a per-delta floor over-demands,
  measured 0.0 touches/B on dense — the same over-derivation
  trap as the tree-derived limb floors §17.3 owns). **Cmp** —
  LANDED (board per-delta floors; the pre-existing cliff flat pin
  stands). **Id covers/disjoint pinned nothing that sees the
  walk's work** (every enforced column structurally near-zero;
  the cost is scan) — LANDED: absolute scan ceilings ×1.25 over
  full-examination floors, flat per byte across a depth doubling.
  **Fork/split had no envelope row and bypasses the scan hooks**
  — LANDED as fork's first envelope row (heap prices the
  materialized halves; the scan pin records the split kernel's
  deliberately raw path at 2 bits, so wiring it into the metered
  primitives is a deliberate re-pin) and the board `party_fork`
  heap declaration corrected to the fork-child floor; metering
  the raw writes themselves DEFERRED with reason (O(n) copies,
  low risk, and the wiring would move board scan readings — a
  deliberate future re-denomination, not a rider on this round).
  **The Shl width undercount** (operand+1 recorded for a widening
  shift, so a shift-and-discard loop would read near-zero) —
  LANDED: the shim records output width; the rank-pair envelope
  re-pinned 54,704 → 70,328 limb ops as a re-denomination with the
  movement annotated. **The tick envelope rows' missing scan
  column, and the four-envelope-harness split that produced every
  per-surface blindness above** (each harness was built for the
  columns its first surface needed; later surfaces inherited the
  blindness) — DEFERRED, named for a future round: collapse
  `Envelope`/`TouchEnvelope`/`SweepEnvelope`/`QueryEnvelope` into
  one five-column shape with per-column floor-or-NA, the #35
  totality mechanism applied to the gate suite; until then the
  board's tick × scan floors and touch column carry the cross-op
  cover. Not exploits: ratchets against the F2 migration genre.
- **Iterated-operation size trajectories** (2026-07-26, the #38
  ORBIT PINS round; no amplification found — six committed
  deterministic pins, one per orbit genre): the board prices single
  calls, and a single-call bound does not preclude compounding
  across calls — an operation linear per call can grow its own next
  input until iterated application is quadratic in total. The tick
  orbit pins (`tick_orbit_growth_is_transient_plus_log`,
  `tick_deep_orbits_stay_banded`) were the in-tree precedent; this
  round landed the remaining legs as byte-exact trajectory pins
  (deterministic schedules, shape asserted at every step or at
  octave resolution, each pin carrying its liveness floor).
  **Fork orbits** (`party::tests`): the chain and the fan are
  exactly affine — both halves read exactly `2 + 2k` bits at the
  k-th fork over 512 steps (one two-bit level per split, nothing
  compounding), and the fan unwinds along the same trajectory to a
  byte-identical seed. **Fork+join round trips** (`clock::tests`):
  the untick'd round trip is byte-stationary over 256 rounds; the
  ticked variant returns the party byte-identical every round while
  the version stays one fixed two-leaf scaffold whose counter costs
  exactly `7 + 2⌊log2 k⌋` bits over 512 rounds — gamma width, never
  a ratcheting tree. **The paper's §6 churn scenario**
  (`churn_orbit_sizes_reach_a_bounded_band`; fork + tick +
  anonymous exchange + retiring join per round on a fixed
  arithmetic schedule, population 8, 4096 rounds): max id bits
  plateau in a fixed band — octave maxima [20, 26, 42, 70, 92,
  100, 122, 134, 132, 132], tail no higher than the plateau's
  first octave — and max version bits grow per doubling only
  ([24, 58, 102, 126, 176, 226, 270, 335, 383, 425]; tail steps
  65 → 48 → 42 bits/octave, logarithmic). **The paper's §6 static
  scenario** (`static_orbit_ids_freeze_and_versions_grow_log`,
  8 peers, 4096 rounds): every id byte-identical forever, max
  version bits monotone and exactly `8i − 4` over the octave
  ending at round `2^i` — 8 bits per doubling, the counters'
  gamma widths. The paper's "stabilizes with a minor logarithmic
  component" is now a committed criterion, not a chart
  (`examples/space_consumption.rs` remains the statistical
  reproduction). Board and bench-judge roster untouched: orbits
  are a test-surface pin genre, not board rows — single-call
  denomination stays the board's job, trajectory shape the
  orbits'.

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
  at the constant, tripwire pinned).
- **Output-dominated projection** (`version_project`/
  `clock_own_version` on comb × scattered-party and, since C3
  applied the owner's pre-approved ruling to the plateau crosses,
  on reveal-comb/reveal-hifloor/pure-comb): judged against
  `n_io` = packed input + packed output (canonical coding cannot
  be padded), with the sweep measured O(`n_io`)-tight — the
  owner's rider — on every declared cross.

Everything else stays input-denominated — both codec directions
(canonical 1:1), all scalar/comparison/query rows, and the
packed-output mutators, whose input denomination rests on the
1-Lipschitz property pinned in `meter/tier2` (output boundaries ⊆
union of the inputs'; total bits within 4 per input leaf of the
inputs' sum) rather than assumed. `meter::board`'s module doc
carries the do-not-re-denominate list. Rank rows denominate
against value content `bits(num) + exp`, which every public
construction path bounds by the producing wire.

- **Flat-denominator shapes fit their exponents against value
  content** (C3, 2026-07-26 — the comb-scatter classification,
  closed). The shape scales tooth count at a fixed 1000-bit
  magnitude: packed bytes are intercept-dominated (~×1.2 per
  level) while value content (§10.6's Σ leaf-height bits) and
  measured per-tooth work double, so a packed-byte power-law fit
  manufactures e ≈ 4 out of flat marginal work [measured]. The
  shape's input-denominated cells fit exponents against the
  bundle's value content (event-side leaf-height bits + id-side
  packed bytes; row-disclosed as `expd[content ...]`); constants
  and floors stay per packed byte; I/O cells keep `n_io`; the
  bench mirror's denominators follow. Tripwires in
  `meter::board::tests`: the packed fit must stay broken on
  measured-flat work over the intercept premise, and a
  quadratic-in-teeth probe must read red against content. The
  column's work is linear on its honest denominator; no cell
  exceeds it.

Amendment (2026-07-26, the error-path round): **rejection rows
denominate against the fed stream alone.** A rejection produces
no output, so the text rule's `n_io` degenerates to its input
side: a `FromStr` rejection row is judged per fed *text* byte at
the general limb ceiling (no radix-work term — `R` prices
conversion of the accepting direction, and a rejection forces no
conversion), and a decode/overlap rejection row per fed packed
byte. The pad-the-output door does not open here (the fed stream
is the adversary's own input: padding it inflates the denominator
only by bytes the operation is genuinely asked to consume), so no
honesty ceiling is needed on the rejection side.

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
public operation surface × §2's families — **989 cells at this
tip** (amended 2026-07-26, the error-path round: the 18 rejection
rows below added 269 cells, 720 → 989), membership pinned by the
smoke test — judged at two scales
(default; `board::RECORD_SCALE` = ×4, `just amp-board-record`) at
the **release profile**, the measurement of record (ratified
2026-07-26: debug assertions perform metered work, so dev boards
price verification scaffolding into the counters; dev runs are a
debugging view, never pinned), from deterministic meters only:
peak heap, grown stacker segments,
limb ops, scanned/written bits, and — since the touch column landed
(2026-07-26, the #39 instrumentation ratchet) — accumulator digit
touches, so every cell is six-column: verdict plus five judged
counter columns. The board is a generalized
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
on walk rows; touch ≤ 96 digit touches/byte (calibrated 2026-07-26,
release: heaviest honest reader the mirror-narrow tick cross at
30.8/B default, 24.3/B record — scan's own margin convention).
Green = all columns within ceilings AND all floors
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
where big-integer arithmetic is semantically mandatory, at two
derivations since C3 re-derived the walk rows' (the same split
the touch floors always had): rows that read the stored form
as-is (decode, rank/distance/lag, tick) floor at one op per 64
bits of every stored payload *code* wider than 128 bits — a
plateau of equal wide leaves stores its width once, so a
tree-derived floor demands limb work no conforming walk does —
and the value-materializing parse rows floor at one op per 64
bits of every stored *base* wider than 128 bits (conversion
must materialize every spelled value); heap floors on codec
and text rows (the result materializes at least its packed
bytes), plus the fork rows' deterministic-liveness child-copy
floors (clock fork since the floors landed; party fork since #39 —
fork builds both halves, so the generic in-place NA misstated it);
touch floors (2026-07-26, #39) at two deterministic-liveness
derivations — one touch per stored delta on the delta-folding
kernels (sweep, emit, query folds, tick, parse: the envelope
suite's committed one-per-delta convention), one touch per 64 bits
of every stored wide code on the decode rows (the validator
legitimately batches word-scale deltas in the accumulator's lazy
zone, so a per-delta floor over-demands there — the stream-derived
convention, deliberately NOT the tree-derived one whose
over-derivation §17.3 already owns on the limb column);
segments ceiling-only (its honest floor is zero). NA
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

### The rejection surface (fallible operations, enumerated 2026-07-26)

Cost claims are total: rejecting an input is an outcome with a
cost, bounded like any other, whether or not the caller honored
the usage invariants. (The linearity-of-parties rule is a
*semantic* safety rule, not a soundness rule — nothing crashes if
violated, programs just stop meaning what the caller wants; the
owner's ruling, 2026-07-26. Semantic claims stay conditional on
the invariants; cost claims are unconditional.) The board's
rejection rows (this round) measure the rejection side under all
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

Rejection-row conventions (this round): **denomination** — a
rejection produces no output, so every rejection row denominates
against the fed stream alone (packed bytes, or text bytes on the
parse rows; the §6 amendment). **Floors** — packed-stream
rejection rows floor scan at one bit per fed byte with the
defect-placement derivation (a self-delimiting stream's terminal
defect is only discoverable by parsing to it; the overlap rows'
witnessing position sits at both operands' stream ends and the
packed coding has no random access); heap, limb, and touch are
NA on rejection rows — rejection materializes no result and
forces neither value work nor an accumulator fold (a validator
may defer both past the topology walk that finds the defect).
Text-rejection rows declare no floor on any column, by honest
derivation: no deterministic counter watches text-byte
consumption, and a parser may find the defect in tokenization
before any packed or value work — their ceilings judge live
readings (the parsers do metered work greedily) and the time leg
times them like every row.

**The bench judge** (`tools/benchjudge`, stdlib Python;
`benches/board.rs` driven by the board's own cell table so bench
IDs mirror board cells by construction — the pinned mode times
290 cells: the 288 designed-pairing board cells derived by rule
from the axes (`board::BenchMode::Pinned`: each shape's
designed-stress groups, the organic control, and the board-red
riders; 225 → 288 at the error-path round, the rejection rows'
designed pairings, count verified against the criterion `--list`)
plus the wide-display pair; `BOARD_BENCH_MODE=full`,
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
profile — the profile of record — limb+scan+touch meters lit]:
board **890 green / 99 red at the default scale; 873 / 116 at ×4**
over **989 cells**
(amended 2026-07-26, the join_all cure — §3's landed entry:
879 + 110 → 890 + 99 default, 863 + 126 → 873 + 116 at ×4; the
twelve flips per scale, the benign heap-exponent rider, the
success-path re-attribution, and the six segments-onset movements
are enumerated cell-exact in §17.3's cure amendment; every other
cell byte-identical at both scales against the error-path round's
renders [measured — strip-diffed])
(amended 2026-07-26, the error-path round: the 18 rejection rows
added 269 cells, 720 → 989, and every pre-existing cell's rendered
row is **byte-identical** at both scales against the #39
acceptance renders [measured — the touch39 and errpath45 renders,
mechanically stripped and byte-compared] — the movement is exactly
the new cells, whose 18 (default) / 24 (×4) reds are triaged in
§17.3 under four genres: the round's own OPEN join_all re-scan
finding (red-pinned), the comb-scatter flat-denominator column,
the id-side parser recursion, and the diff kernel's both-internal
recursion, the last two P4.2's standing genre. Sums 628 + 92 = 720
→ 879 + 110 = 989 default; 618 + 102 = 720 → 863 + 126 = 989 at
×4)
(amended 2026-07-26, the #39 instrumentation ratchet: the touch
currency joined the board as the fifth judged column, live on
every cell with a floor-or-NA declaration per operation; every
pre-existing column's rendered value byte-identical at both scales
except the widening-shift limb re-denomination — the shift shim
now records output width, operand plus shifted-in limbs, moving
limb constants on the `rank_pair_ops` row and four `distance`/
`lag` cells by at most +0.2/B, no verdict flips — and the
`party_fork` heap declaration, which became the fork-child floor.
The touch column's 24 red reasons all land on already-red cells in
two owned genres, the comb-scatter column (18) and the plateau
projection cells (6); zero touch floor trips at either scale)
(amended 2026-07-26, the #40 representation migration — Version's
at-rest form is `codec::Bits`, byte-level Eq/Hash, BitWriter
dissolved into raw-slice writes: heap and record-scale segment
readings moved down on 41 cells (default) / 45 (×4), flipping
three owned reds GREEN — `clock_encode × comb-scatter`'s
default-scale heap exponent (the comb-scatter genre count 21 → 20)
and the `version_tick`/`clock_tick` × nested-wide record-scale
segment legs (the P4.2 genre, eight legs → six; nested-wide's ×4
segment count now 1, under the ceiling) — and lowering the
remaining ×4 tick-op segment counts to: nested-full 7,
mirror-narrow 7, staircase 14, ascend-cliff 2, ascend-plateau 2,
pure-comb 2, the id-side parser pair still 12; sums 627/93 →
628/92 default, 616/104 → 618/102 record; no other verdict moved.
The dev-render agreement claim rides at its own tip: the profile
ratification's no-flip comparison was made before these rounds)
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
- **#34: the tick limb cure and the fusion** (2026-07-25..26,
  red pin `dc9a2c31`, the anchor-web walk `12b5e9a3` + the
  chained-memo pre-scan `39009918` through the round-5..7 cures,
  the fused tick `80131954`): T-tick — amortized O(n + m) Accum
  digit touches — realized inside the fused tick; every
  #34-owned red flipped at both scales, and the fusion moved 51
  tick-row constants per scale with zero verdict flips. The
  design loop's record is the spec's §9 (rounds 3–8); the cell
  accounting is §17.3's amendments.

Remaining plan: **C3** (§17.2) → **P4.2** residual audit →
**P5.1–P5.5** closeout, with the boolean-skyline decision (#24,
the user's) after C3.

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
limb dimension was below the bar — quadratic on wide × deep
crosses (§3) — until #34's cure landed it at amortized
O(n + m), realized inside the fused tick (the tick cost spec).

## 17. Work items of record

### 17.2 Open items, with acceptance contracts

**#34's red-pin round (done 2026-07-25).** The fill
linearization's review bookkeeping rode it: the ghost prose
re-denominated to the deferral design (`fill.rs`'s
drift-accumulator sentence, the tests' drift-stack phrasing,
`nested_full_id`'s and the board family's
lookahead-and-pre-scan present-tense rustdoc); `fill.rs`'s
`# Cost` restated to the measured truth at each stage (the
red-pinned quadratic until the cure re-derived it); the tick
cells' floors raised from the generic walk floors toward honest
measured counts; #33's acceptance deviation (the ×4 segments
residual, P4.2-owned) recorded as a dated amendment here.
*Acceptance*: met — the red-pin contract in
`design/before-tick-cost-spec.md` §7 plus these ride-alongs;
one full gate.

**C3 — P3.10: realization verification, re-pins, and the
before/after table.**

*Queue of record* (enumerated 2026-07-26 at the round's opening,
gathered from every C3/OPEN owner mention in this document and the
κ rustdoc's hand-off; the deterministic-meter items run in this
round, the bench-harness items are sequenced after it because the
κ re-pin and the denomination moves below change exactly the
readings those runs would capture):

1. Board re-run at both scales, movement vs §17.3 (bullet 1
   below); this round.
2. The κ re-derivation (§6, the κ rustdoc's hand-off, the ten
   κ-owned text reds + the #35 κ-text extension); this round.
3. The comb-scatter κ-genre adjudication (single-cell column
   attribution before any classification is accepted); this
   round.
4. The §6 `n_io` ruling on the six plateau projection cells
   (§3's OPEN entry; pre-approved by the owner 2026-07-26 with
   the O(`n_io`)-tightness rider: apply mechanically if the
   measured cost tracks the output, escalate only if it does
   not); this round.
5. The comb-scatter flat-denominator column classification
   (§17.3's #35 genre 1 and error-path genre 2; the cliff
   generators' leaf-delta representation question is the
   substance); this round.
6. The cliff limb-floor re-derivation (three ×4 liveness trips);
   this round.
7. The plateau limb-floor re-derivation (§17.3's #35 genre 2,
   18 default / 20 ×4 trips); this round.
8. The judgment-layer question (sub-allowance exponents:
   `rank_sum`/`rank_pair_ops` × benign, the
   `party_join_all_overlap × benign` ×4 rider); decide once,
   apply to the genre; this round.
9. Bench-rider population (`BOARD_RED_BENCH_RIDERS` lands with
   the reds' classification, per the #35 amendment); this round,
   after items 3–5.
10. The §17.3 reconciliation and restated sums (bullet 6);
    this round.
11. The judge-roster realization (fifteen bigroot expectations
    leave on the banked evidence; the display pair resolves with
    the κ re-derivation; the schoolbook expectation is
    permanent) plus `bench-judge`/`bench-judge-record` exit 0
    both scales both modes with record numbers captured: bench
    harness, sequenced immediately after this round's re-pins.
12. Envelope tightening (the twelve event-side rows): deferred
    to P5.1's envelope finalization, one downward re-pin from
    post-C3 stable readings instead of two.
13. The before/after table of record: bench harness, after
    item 11.

Adjacent but not C3's: #24 (the user's decision, post-C3) and
the stack-container measured phase (§17.5, C3-adjacent, its own
harness).

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

**#34 — the tick limb cure and fusion (done: the cure
2026-07-25, the fusion 2026-07-26; the hot path — every tick
calls fill).**
The statement of record is `design/before-tick-cost-spec.md`
(converged revision 3); its §7 acceptance contract was binding
and was met in the sequenced shape: red pin first (the wide×deep
and descending-staircase cells measured not assumed;
mirror-narrow's green memo cells held at honest floors), then
T-tick — amortized O(n + m) Accum digit touches — via the
anchor-web discipline (the zero-run-compressed watermark stack,
anchored entries, per-operand lifetime pricing, the L2×L6
pricing chain; §17.3's 2026-07-25 amendments carry the
cell-exact flips), then the fused tick as its own bisectable
commit (one walk carrying fill emission, the changed flag, and
grow's route DP; copy-on-first-divergence; the owner rulings and
the landed record are the spec's §6 amendment and §9 round 8;
§17.3's fusion amendment restates the sums). Byte-identity held
at every stage; the L6 output-bound proptest pin landed with the
cure; `fill.rs # Cost` restates exactly what is proven; Accum
pooling per the spec's §6 constants note. The flag seam's
full-width witnesses and `arb_base`'s `2^64`-aligned arm (the
fusion review's fix round) pin the width axis the size-axis
instruments do not discriminate.

**The join_all up-front re-scan (done 2026-07-26, the error-path
round's cure).** The §3 entry's cure, landed as the per-call
decode-once index of `self` (`IdIndex`; the coalesce-first
candidate rejected by probe — §3 records the decision): the
Θ(inputs × accumulator) per-input re-walk dissolved with the
hand-back granularity preserved verbatim, pinned differentially
against the cursor-walk discipline held as
`testing::fold_oracle`. Acceptance met: the red pin (≥ ×3.5 scan
growth across a joint doubling) re-pinned flat in the cure's own
commit at the measured ×2.00, ceiling ≤ ×2.05 over a
build-liveness floor (`join_all_overlap_upfront_test_reads_flat`);
the board row's scan exponent 1.00 at 15.9 bits/B on every family
at both scales; the success-path fold rows re-attributed with
movement annotated against the parent-tip boards (§17.3's
2026-07-26 cure amendment — `party_join_all × scatter` flips
green at default); byte-identity across the differential suite.
Two acceptance deviations, both annotated there: the
`party_join_all_overlap × benign` ×4 cell keeps its
sub-allowance heap-exponent red (the C3 judgment-layer genre
that rode it; its scan legs flipped green), and the segments
column moved on six tick/diff cells whose every other column is
byte-identical (the P4.2 codegen-onset genre).

**P4.2 — residual recursion and word-scale scanning.**
Audit every remaining `recurse::descend!` site post-C2; convert
survivors per the explicit-stack pattern or record why they stay;
apply the word-at-a-time subtree skip (popcount pending-counter
delta, mid-word zero-crossing exit; the item earlier drafts
numbered §11.4) to `idbits` and the skyline topology stream where
benches justify. Triage convention for the genre's segments
currency (the fusion round's finding, spec §9 round 8): the
counter reads the stacker's process-global segment cache, which
is order-coupled to the preceding cells' stack usage in the
shared board process, so a kernel change anywhere in the binary
can re-roll the counts on untouched kernels' rows — the fusion
round moved the six record-scale id-pair rows in both directions
(70 → 54 and 12 → 16) with statuses unchanged and every other
column byte-identical. Segment-count movement on a row whose
work columns are byte-identical is triaged as order coupling,
not kernel movement; the counts of record re-cite in §17.3's
owning amendment. *Sequencing risk, resolve at #24 decision time*:
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

### 17.3 Owned-red accounting (current; over 989 cells)

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

Amended 2026-07-26 (the #40 representation migration; this
re-cite owed by the #39 round's doc pass): Version's at-rest move
to `codec::Bits` with byte-level Eq/Hash lowered heap and
record-scale segment readings on 41 cells (default) / 45 (×4)
[measured — the pre- and post-migration release renders,
byte-compared cell by cell] and flipped three owned reds GREEN,
each closed in its owning genre with the genre surviving on its
other cells:

- `clock_encode × comb-scatter` (default scale, heap exponent):
  leaves the comb-scatter column's genre, 21 → 20 default cells;
  the column's limb/scan/touch exponents and floors still carry
  the genre, owner unchanged (**C3's classification question**).
- `version_tick`/`clock_tick` × **nested-wide** (record scale,
  segments): leave the P4.2 recursion genre, eight pre-existing
  tick-walk segment legs → six (nested-full ×2, mirror-narrow ×2,
  staircase ×2); nested-wide's ×4 segment count reads 1, under
  the ceiling. The remaining ×4 tick-op segment counts re-cite at
  the migrated representation: nested-full 7, mirror-narrow 7,
  staircase 14, ascend-cliff 2, ascend-plateau 2, pure-comb 2;
  the id-side parser pair holds at 12. Owner unchanged (**P4.2**,
  iterative walks remain the cure).

No other verdict moved at either scale; the ascend-cliff heap
constants (64.4/66.7 B/B) and every genre list below are verified
current against the migrated renders by mechanical grep.

Amended 2026-07-26 (the #39 instrumentation ratchet): the touch
currency joined the board as the fifth judged column — every cell
now carries a touch reading, ceiling (96/B), and floor-or-NA
declaration; the heaviest honest green reader is the mirror-narrow
tick cross (30.8/B default, 24.3/B record). The column changed no
verdict: its 24 red reasons all land on already-red cells in two
owned genres — the comb-scatter column (18 cells: touch exponents
join the limb exponents, the same flat-denominator question) and
the plateau projection cells (6: `version_project`/
`clock_own_version` × {reveal-comb, reveal-hifloor, pure-comb},
touch exponent ~1.9 joining every-column reds; the §3 OPEN
denomination entry now includes the touch column among the columns
awaiting the `n_io` re-denomination). Zero touch floor trips at
either scale. Two deliberate old-column movements ride the same
round, neither a verdict change: the widening-shift limb
re-denomination (`rank_pair_ops` row and four `distance`/`lag`
cells, at most +0.2/B — the exponent-alignment work newly counted,
envelope re-pinned 54,704 → 70,328 with the movement annotated)
and the `party_fork` heap declaration (generic in-place NA → the
fork-child deterministic-liveness floor; all twelve cells hold
their readings above it).

Amended 2026-07-26 (the error-path round): the 18 rejection rows
took the base 720 → 989 with every pre-existing cell's row
byte-identical at both scales (the #39 acceptance renders,
mechanically stripped and byte-compared); the 18 (default) /
24 (×4) new reds fall into four genres, each with one owner:

- **The join_all up-front re-scan** (11 default / 13 ×4:
  `party_join_all_overlap` × every overlap-bearing family at ×4;
  comb-scatter and benign read green at the default scale only
  because the probe count clamps at its minimum there — the
  non-monotone-verdict genre §13 records, and the ×4 reading is
  the honest one): scan e ~2.0 at constants 47–2,954 bits/B —
  the fold's per-input re-walk of the fixed accumulator, §3's
  OPEN entry, gate-pinned ≥ ×3.5
  (`join_all_overlap_upfront_rescan_reads_quadratic`). The benign
  ×4 cell also reads a sub-allowance heap exponent (e 2.39 over a
  near-zero constant, the hand-back vector's growth) — the
  §17.2-C3 judgment-layer genre riding a cell already red on
  scan. Owner: **§17.2's join_all open item (this round)**.
- **The comb-scatter flat-denominator column** (7 / 7:
  `version_decode_truncated`/`_trailing`/`_noncanon` and
  `clock_decode_truncated`/`_trailing` × comb-scatter — limb and
  touch exponents over honest constants of 2.8–5.8 ops/B, the
  validator's per-tooth work against packed bytes growing ×1.24
  per doubling — plus `clock_join_overlap`/`clock_sync_overlap` ×
  comb-scatter, heap and scan exponents over near-zero readings
  (the id side grows with the teeth while the version-dominated
  denominator stays nearly flat)): the standing column question,
  no new mechanism. Owner: **C3's classification question**.
- **The id-side parser recursion** (0 / 3:
  `party_parse_trailing` (12 grown segments, e 3.58),
  `party_parse_noncanon` (6), `clock_parse_trailing` (12) ×
  id-pair at ×4): the same recursive `parse_id_node` the
  accepting `party_from_str`/`clock_from_str` × id-pair reds
  already carry. Owner: **P4.2**, the explicit-stack residual.
- **The diff kernel's both-internal recursion** (0 / 1:
  `party_without_none × id-pair` at ×4, segments e 1.49 / 62
  grown): `diff` recurses exactly where both operands are
  internal — the documented deliberate exception whose
  one-shallow-operand-caps-depth reasoning does not bind on
  identical deep operands, the shape this row commits. Owner:
  **P4.2** (its audit list gains this site's deep-both-internal
  case; the pre-existing `party_without` row never reached it —
  its seed minuend keeps the walk shallow).

Sums: default 879 + 110 = 989; record 863 + 126 = 989.

Default, 110 = the 92 pre-existing below + the 18 error-path reds
above. Record, 126 = the 102 pre-existing below + the 24 above.

Amended 2026-07-26 (the join_all cure, §3's landed entry; single
release runs at both scales, strip-diffed cell-exact against the
parent-tip boards — every cell not named here byte-identical):

- **The join_all genre empties, less one rider.** All 11 default
  and 12 of 13 ×4 `party_join_all_overlap` reds flip GREEN: scan
  e 2.00–2.14 at 47–2,954 bits/B → e 1.00 at 15.9 bits/B on every
  family at both scales; heap gains the index table's constant,
  at most 10.6/B (nested-full ×4), e 1.00, under the 16 ceiling.
  The survivor, `party_join_all_overlap × benign` ×4: its scan
  legs flip green (e 2.13 → 1.00) and the cell stays RED on the
  sub-allowance heap exponent alone (e 2.39 → 1.17 over a 0.0/B
  constant, the hand-back vector's growth against a near-constant
  denominator) — the §17.2-C3 judgment-layer genre this
  accounting already assigned while it rode a scan-red cell.
  Owner: **C3's judgment-layer question** (unchanged).
- **The success path re-attributed** (the same up-front test
  priced it): `party_join_all × scatter` flips RED → GREEN at
  default (scan constant 98.6 → 91.9 bits/B under the 96 ceiling;
  ×4 stays green, 87.0 → 81.0); `party_join_all × benign` moves
  106.2 → 100.1 (default) and 123.5 → 116.9 (×4) bits/B, still
  RED on the scan constant — the n·log n coalescing constant the
  non-monotone-verdict caveat records, its owner unchanged.
- **Segments-onset movement, six cells, no other column moved**
  (every deterministic work counter byte-identical; the segments
  column's documented codegen sensitivity — frame sizes shifted
  under the new code in the binary; owner **P4.2**, whose genre
  list already carries every site): `version_tick`/`clock_tick`
  × nested-wide ×4 go GREEN → RED at 1 → 2 grown segments against
  the flat ceiling of 1; `version_tick`/`clock_tick` ×
  pure-comb/ascend-cliff/ascend-plateau ×4 (already red on
  segments) read 2 → 4 grown; `party_without_none × id-pair`
  (the diff both-internal recursion) goes GREEN → RED at default
  (0 → 2 grown) and reads 62 → 70 at ×4 where it was already red.

Sums after the cure amendment: default 890 + 99 = 989; record
873 + 116 = 989. Default, 99 = 110 − 12 flips
(11 `party_join_all_overlap` + `party_join_all × scatter`)
+ 1 segments onset (`party_without_none × id-pair`). Record,
116 = 126 − 12 flips (`party_join_all_overlap` × the 11 plus
comb-scatter) + 2 segments onsets (the tick × nested-wide pair).

Amended 2026-07-26 (the #34 fusion landing; single release runs
at both scales, strip-diffed cell-exact against the
join_all-cure tip boards — `board-fusion-{lo,hi}.txt`): 51 of
the 83 tick-carrying rows per scale moved on work-column
constants — the eliminated probe pass and byte compare,
copy-on-first-divergence, the deferred output buffer; the
mechanism table and re-pins are spec §9 round 8 — with zero
verdict flips among them. The ascending-cliff pair's owned heap
constant re-cites at the fused walk: 63.0 (default) / 65.3 (×4)
B/B, exponent 1.00, the cell red as before. One verdict flip,
not a tick row: `party_without_none × id-pair` at the default
scale RED → GREEN, 2 → 0 grown segments — the same order-coupled
cell the join_all cure amendment above added as its +1 segments
onset. The record-scale id-pair segment counts re-cite at the
fusion tip, readings moved in both directions with statuses
unchanged (each already red in the P4.2-owned recursion genre):
the parser pair `party_from_str`/`clock_from_str` 12 → 16
(e 4.00), `party_parse_trailing` 12 → 16, `party_parse_noncanon`
6 → 8, `clock_parse_trailing` 12 → 16, `party_without_none`
70 → 54 — five parse kernels plus the diff kernel. Every other
column on those rows is byte-identical, and the ×4 tick-op
segment counts hold (nested-full 7, nested-wide 2, mirror-narrow
7, staircase 14, pure-comb 4, ascend-cliff 4, ascend-plateau 4).
Triage mechanism, recorded at P4.2's §17.2 entry: the segments
counter reads the stacker's process-global segment cache, which
is order-coupled to the preceding cells' stack usage in the
shared board process — a kernel change anywhere in the binary
re-rolls the counts on untouched kernels' rows. Sums: default
891 + 98 = 989; record 873 + 116 = 989.

Default, 92 pre-existing: the #35 refactor's new reds less the
#40-flipped `clock_encode × comb-scatter` (73) + the
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

Record scale, 102 pre-existing: the 72 #35 reds above + the
thirty older:
the six P4.2 tick-walk
segments legs above (nested-full ×2,
mirror-narrow ×2, staircase ×2 — the nested-wide pair left at
#40) + the pure-comb pair's and
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
