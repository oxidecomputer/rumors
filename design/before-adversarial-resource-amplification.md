# `before`: adversarial resource amplification in Version and Party computation

Status (2026-07-31): the campaign is at its tail on branch
`before-hardening`. The audit's amplifier classes and every
adversarial round's findings are cured or owner-modeled: the board's
red-triage buffer (`BOARD_EXPECTED_REDS`, `meter::board`) is empty and
asserted empty at acceptance, with every former standing red resolved
to a cure or a dated owner-declared model at its declaration site
(`meter::board`'s ceilings module). What remains is the tail this
document plans: the fuelscape rank-view kernel item, the tick-seam
probe, the closeout obligations, the survey/soak and benchmark legs,
the final adversarial review (in flight), and the go-criteria for
merging to main (§14, with per-item acceptance contracts in §17.2). This
document is the single canonical source for the criteria the
instrumentation enforces and the work that remains, written to the
current state of the tree; the compact history is the decision record
(§12), and everything else — landed-work narratives, superseded
amendment chains, measurement logs whose conclusions are pinned in
code — lives in git history
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
ledger), all cured or owner-modeled, because every quantity the
algorithms need at a node is either one of two global accumulators
or bounded by that node's own coded size. The fix of record is the
Tier 2 skyline representation (§10), the shipped production coding
since C2.

## 2. Input families

The adversarial constructions are single-sourced in the family
registry (`before::meter::registry`): every family is a `FamilyId`
variant carrying its row of record (`FamilySpec` — shapes, operand
bundle, board-coverage answer, band roster), and `Shape` is the one
public construction door — the raw generators are private, so no
instrument, bench, example, or downstream crate can mint an
adversarial shape outside the registry. Every instrument derives its
family axis from that roster: the board's columns
(`FamilyId::board`), the envelope suite's flatness and adequacy
bands, the bench mirror, and the fuelscape atlas's coverage-parity
test. A family existing outside an instrument's coverage is
therefore structurally impossible, and family membership is
committed data (the board smoke suite holds the rendered matrix to
each spec's declared reach), never a prose tally. Each family's
bit-layout derivation, normal-form argument, and closed-form size
live on its generator's rustdoc.

The genres, as a reading map (each name below is a registry family
or its generator; its spec states which operations it stresses):

- **Depth and frame genres** — dense spines, alternating-binary
  spines, id spines and id pairs at full lockstep depth: recursion
  frames and per-level state (the V2/P1 cures; the iterative-walk
  discipline).
- **Width genres** — bigroot magnitudes over spines, hugeleaf single
  wide values: per-frame magnitude clones and width-quadratic decode
  (V1/V3).
- **Carry-cliff and delta genres** — cliff/boundary combs, wide-tooth
  combs, unpaid-crossing fans, cancelling/static prefixes, jump
  combs, harmonic rank ramps, scattered populations: value content
  exceeding wire bits (§10.6), accumulator collapse and freeze
  discipline, fold accumulator growth (V6/V7).
- **Tick-walk and memo genres** — nested-full-sibling,
  nested-wide, mirror/wide-tail, descending staircase, memo
  chain/comb/fan-out/oscillation/churn, reveal comb and hifloor,
  pure comb, ascending cliff/plateau: the fill walk's pre-scan and
  watermark stack, the frame ledger, width circulation, and fold
  direction (the tick cost spec's §9 rounds).
- **Freeze, promotion, and settle genres** — freeze-position,
  promotion re-arm (and its mate), dense-suffix (and mate),
  wide-arming, arming trains, puncture-product/plateau-puncture,
  lone-freeze, weight comb, freeze parade, tooth-tail: the query
  folds' anchored-segment integrator, the promotion ledger, the
  cluster-delegated settle products, and the accumulator's three
  skip mechanisms (§3's F-genre entries).
- **Two-operand and pair genres** — jump-pair, concurrent-pair,
  tooth-tail: cross-stream funding, emit side-switch density, and
  boundary-aligned pair floors.
- **Population genres** — scatter, stagger (comb/id/population),
  meet-shade, the weave fold family: n-ary fold correlation against
  the balanced counter, non-shrinking meet accumulators, and
  both-present-rich overlap testing.
- **Projection and mask genres** — comb-scatter crosses,
  mask-drift triple/quadruple: output-dominated materialization and
  the fused masked comparisons.
- **The control** — benign small organic values: the parity floor's
  referent.

## 3. Findings ledger

Every finding is cured, closed as a ratchet, or resolved to a dated
owner-declared model; each genre above witnesses at least one. The
entries here are the map — mechanism, cure, and where the cure is
pinned; mechanism detail and pre-cure measurements are in git
history at the commits §12's entries name, and the tick genres'
refutation-and-cure records are `design/before-tick-cost-spec.md`
§9.

- **V1** (quadratic memory+time; per-frame owned path sums in
  compare/combine): cured by the difference accumulator + the
  skyline sweeps. ×6,668 at 29 KiB when found.
- **V2** (linear ×782 stack; recursion frames in every event walk):
  cured by iterative sweeps/bit-stacks (kernels), explicit compact
  stacks elsewhere; the P4.2 round finished the residue (no
  library-path depth recursion remains; envelope segments pinned 0).
- **V3** (quadratic decode of wide gammas): cured by limb-wise
  mantissa accumulation; linear on hugeleaf.
- **V4** (linear working form + Builder pre-size): the working form
  is deleted (C2); push-growth builders.
- **V5** (linear parse stack): the skyline validator needs ~2
  bits/level and no values.
- **P1** (id-side linear recursion frames): iterative walks,
  segments pinned 0.
- **V6** (quadratic rank fold on the harmonic ramp): digit-routed
  merge + relative freeze trigger + summation-by-parts; the
  `RANK_HARMONIC` envelope and the jump/wide-tooth bands enforce it.
- **V7** (quadratic join-direction folds): balanced binary-counter
  reduction on every fold surface; the fold rows are judged under
  the reduction's own declared `O(D log k)` model (§13).
- **Fill's lookahead/pre-scan terms**: worst case O(|ev| ×
  local-id-depth) on matched spines; cured under the
  nested-full red pin — the right-full arm defers to an O(1) peek,
  the left-full pre-scan memoizes. A §6 pricing obligation, not an
  exploit.
- **Fill/tick's limb-dimension re-touching** (#34): materialized
  per-subtree magnitudes cost Θ(width) limb work per ancestor, and
  the memo's site resolution read Θ(k²) digit touches on
  consumption-order adversaries. Cured by the anchor-web watermark
  discipline plus the frame ledger (the tick spec's §9 rounds 3–4);
  the memo families guard the cure. The same machinery had carried
  a semantic staleness bug the families' first differential
  crossing caught — cost families and the semantic suite must
  cross.
- **Tick's width-circulation cycle** (the reveal comb): Θ(k·b)
  touches on Θ(k + b) input and output; cured by the latent
  boundary register (the spec's I4′ width conservation), pinned
  flat with absolute bands.
- **Propagate's fold direction** (the ascending cliff): the wide
  surviving residue folded into each popped narrow difference;
  cured by top-index domination deciding each hop in O(1), the
  dying side funding the fold.
- **Plateau projection output-domination**: materialization
  re-creates a wide absolute value per kept site — mandatory output
  Θ(k·b) on a Θ(k + b) input — so the materialization rows
  (`own_version_to_version`/`clock_own_version_to_version`) are
  `n_io`-denominated with the O(`n_io`)-tightness rider measured.
  Projection itself is lazy at every spelling (`OwnVersion`); the
  product-growth path is only the explicit `.to_version()`.
- **Profile-dependent meter readings**: dev `debug_assert!`s
  perform metered work, so dev and release measure different
  programs. **Release is the board's measurement of record**
  (§12's ratification); assertion-scoped meter suspension REJECTED
  on doctrine.
- **The join_all up-front re-scan**: Θ(inputs × accumulator) overlap
  testing; cured by the per-fold-call `IdIndex`
  (`party/ops/index.rs`), its partition-point searches metered and
  priced by a declared search allowance, with the weave family (the
  both-present-rich population) as the honest stressor.
- **The instrumentation census's blind spots** (#39, the F2 genre —
  work routed through a meter that is not pinned on that surface):
  landed the touch column as the board's fifth judged currency,
  emit/text-parse/decode/cmp floors, and the fork envelope row —
  ratchets against meter migration, not exploits.
- **Iterated-operation size trajectories** (#38): no amplification —
  deterministic orbit pins per genre (fork chains/fans affine,
  round trips stationary, the paper's §6 churn/static scenarios
  banded); single-call denomination stays the board's job.
- **The query folds' F-genre series** (the #37/#79/#82 reviews and
  their cure rounds): `min_ticks`' pending-minima circulation →
  the range-minimum anchor web plus epoch ledger (`query/web.rs`);
  rank's freeze-position span reads → the anchored-segment
  integrator (the pair co-sweep's single-stream instance); the
  promotion re-arm's absolute-position re-read → the promotion
  ledger settled once at the close; the dense-suffix per-arming
  walk → the mass-balanced product tree; the settle products'
  schoolbook quadratic → cluster-wise delegation to the backend at
  the integer-multiplication bound (rank/distance/lag are
  `Class::MulBound`, with the answer-embedded-product reduction as
  the Ω(M(|v|)) floor witness). Each cure's retired kernel stays
  committed and failing beside it (the adequacy ratchet), and each
  family is a registry member with flatness bands.
- **suanpan's zero-gap maintenance** (the alternating shifted
  pair): top maintenance walked an unfunded zero gap; cured by the
  zero-run ledger (certificates created O(1) by jump writes,
  consumed O(1) by scans), with the cost table verified row by row
  and the three skip mechanisms (ledger, `bottom` watermark, exact
  `top`) each witnessed load-bearing by a public-API family that is
  superlinear without it (weight comb, freeze parade, tooth-tail).
- **`Version::meet_all`'s non-shrinking accumulator** (the shade
  population): sequential reduce read Θ(k·d); cured by the shared
  balanced binary-counter fold (`crate::fold`), the sequential
  reduce committed and failing beside it.
- **The wide-arming text-parse quadratic** (found at the board
  promotion of the settle families): cured by settled-top
  extraction in the parse pipeline; the board column lands green.
- **The render merge's wide-summary re-fold** (mirror-wide
  display): a genuine kernel superlinearity, documented as the
  display impls' `SuperlinearTime` class, judge-rostered red, and
  owner-declared as the mirror-wide render limb model at the
  ceilings module — the one standing cure candidate (§17.3).

## 6. The design invariant and the denomination criterion

Adopted as the crate's contract, enforced by §13:

> **No operation materializes transient state asymptotically larger
> than its packed operands, and every operation remains amortized
> `O(n + m)` in the packed input bits — with no bound on value
> magnitude, tree depth, or encoded size.**

Fundamentally-superlinear problems satisfy the bar at their
problem's own optimum, stated and priced (§14's asymptotic bar):
radix conversion, the multiplication-bound query folds
(`Class::MulBound`), and the n-ary folds' `O(D log k)` are the
committed classes, each bound to its witnesses by the
complexity-claims roster.

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
  κ = 0.75 limb/`R` unit (`MAX_TEXT_LIMB_OPS_PER_RADIX_UNIT`)
  [measured at C3: honest cells read ≤ 0.59, the staircase
  pipeline; digit-by-digit schoolbook reads ~1, still excluded] —
  and the *exponent* leg against `n_io` (never `R`, on which any
  schoolbook converter reads a flat ~1). Two legs because each
  catches what the other cannot: the chunked-schoolbook refutation
  (2026-07-23) demonstrated a still-quadratic converter slipping
  under κ, so the exponent leg enforces the complexity class and κ
  the constant. Both anti-softening tripwires are committed in
  `meter::board`'s suite (the digit-by-digit parser must exceed
  κ; the chunked probe, driven through `evaluate` itself, must
  slip under κ and read red on exactly the limb exponent). An
  output-honesty ceiling closes the pad-the-output door, asserted
  against the conversion units alone (`TEXT_BYTES_PER_RADIX_UNIT`;
  the pipeline term must not loosen it; radix units, forced by the
  delta coding, derivation at the constant, tripwire pinned). The
  pipeline term's decision record is §12's C3 entry.
- **Output-dominated materialization** (`OwnVersion::to_version`,
  priced on the `own_version_to_version`/
  `clock_own_version_to_version` rows, on comb × scattered-party
  and the plateau crosses, per the owner's pre-approved ruling
  applied at C3): judged against `n_io` = packed input + packed
  output (canonical coding cannot be padded), with the sweep
  measured O(`n_io`)-tight — the owner's rider — on every declared
  cross. Projection and its comparisons stay lazy and
  input-denominated (§12's OwnVersion entry).
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
`meter::board`'s suite: the packed fit must stay broken on
measured-flat work over the intercept premise, and a
quadratic-in-teeth probe must read red against content. The
column's work is linear on its honest denominator; no cell exceeds
it. The rule's decision record is §12's C3 entry.

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
fill, text) and the builder; the workspace crate `suanpan`
(`suanpan::Accumulator`) for the balanced signed-digit accumulator
and its cost table.

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
`min_ticks`/`max`) use delta algebra (telescoped
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

The campaign's compact history: every ruling, refutation, and
landed shape, dated. Entries speak about decisions; measurement
narratives and per-round board movements live in git history at
the commits the entries name.

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
  representation of record (today `suanpan::Accumulator`), flat on
  every family [measured, enforced].
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
  become load-bearing for Eq/Hash). The serialized Rank encoding
  landed 2026-07-29 under exactly the deferred entry's condition:
  strict decode rejects non-normalized forms (the wire-form digest
  entry below).
- **Gate A (subadditivity) resolution policy (user ruling
  2026-07-23)**: the existing bound stays unless falsified;
  resolved GO — the lemma of record (§10).
- **GO-WITH-SHAPE 2026-07-24: boolean-skyline unification probe**
  (the user's decision, post-C3); **landed 2026-07-26
  (`2cd73716`), with the construction/predicate split reversed on
  cost evidence.** Under the skyline coding a `Party` is a boolean
  skyline: `diff` rides the sweep — an id leaf cursor per operand
  presenting absent children as synthetic unowned plateaus, the
  event sweep's advance/tie rule transferred verbatim (no boolean
  carve-out; each cursor's one owned bit replaces the running
  difference), one output plateau per elementary interval into a
  leaf-driven collapsing id builder — dissolving the id family's
  one depth recursion and the two-pass complement retagging. The
  probe verdict's predicate leg was reversed with evidence:
  `covers`/`is_disjoint` stay on the lockstep walk (a verdict-only
  walk carries no per-level state, and a leaf-enumerating sweep
  would pay two path-bit stacks per operand depth for interval
  geometry a predicate never reads — multiplying a pinned
  near-zero envelope to retire no red); `sum` stays on its frames
  walk (copy-splice has no sweep analogue). The re-metered
  green→green cells are the enumerated verdict-neutral class
  (ratified by owner, 2026-07-26).
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
  its design-loop record is that document's §9.
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
- **DECIDED 2026-07-26 (C3, the pre-approved arm applied): the
  plateau projection cells are `n_io`-denominated**, the
  O(`n_io`)-tightness rider measured and met (commit `1c32bb56`;
  the rows later re-denominated onto the explicit materialization
  door by the OwnVersion entry below).
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
  Attribution measured before converting. Landed shape: the id
  text parser parses on a SmallVec frame stack (the `codec::tree`
  discipline, differential-pinned against a recursive reference
  first); the fill walk and pre-scan carry suspended ancestors as
  control bits plus pop-able word deltas on the route fold's
  `PopStack`, with a left-full site's collapse maximum re-derived
  by one bounded replay of its disjoint range rather than parked
  per frame. With no library recursion left, `recurse::descend!`
  compiles only for the test surface (the oracle bridge); the deep
  fill's zero segment reading is the committed ratchet
  (`meter::tests`), envelope segments pinned 0.
- **DECIDED 2026-07-26 (#50): the oracle end-state — three
  implementations, one committed roster.** The recursive oracle is
  the semantic definition of record and carries the `join_all`
  hand-back contract and `meet_all` (seeds red through the new
  differentials before any retirement); the fold oracle and the
  test-local min_ticks/rank oracle duplicates RETIRED under the
  ratchet; the three shadow-recompute `debug_assert`s deleted with
  covering differentials cited per site (the 2026-07-26 assertions
  doctrine). The triangle suite (`testing::triangle`) commits the
  op × three-leg roster, total against the extracted public-fn
  surface with live citation checks and per-leg adequacy tripwires.
  Function-space-leg dispositions marked in-roster (ratified by
  owner, 2026-07-26).
- **DECIDED 2026-07-27 (P5 measurement closeout)**: the envelope
  suite re-measured whole, moved rows tightened as final ratchets;
  the judge-roster realization verified under both sampling
  regimes; owner decision: record-sampling judge runs are not a
  standing closeout step — quick mode judges the wall leg, record
  sampling belongs to the acceptance sweep alone. `exhaustive_deep`
  measured combinatorially blown against its committed annotation;
  re-denominated and attributed at #53 below.
- **DECIDED 2026-07-27 (owner): the fuzz-fit instrument joins
  `just gate`** (`fuzzfit-build` then `fuzzfit`): kernel work had
  moved guest fuel and stale bands sat red for a day because no
  standing tier executed the harness; in the gate, fuel movement
  fails the commit that carries it, with `fuzzfit-calibrate` as
  the deliberate re-pin path. Companion entry in
  `design/before-fuzzfit-asymptotics.md` §9.
- **Landed 2026-07-27 (#47): uniform `# Complexity` rustdoc,
  board-bound.** Every public operation carries a `# Complexity`
  section led by Big-O tokens over user-held denominators, and
  `testing/complexity_claims` binds prose to measurement: a claims
  roster pinning each op's tokens at its scanned doc site and its
  witness rows on the board's own op axis; superlinear-time claims
  must equal the bench judge's committed red set in both
  directions; non-linear classes keep deterministic liveness pins
  that read red when their cures land. Dispute-the-seed corrections
  transcribed from the boards rather than the charter sentence.
- **DECIDED 2026-07-27 (#53): `exhaustive_deep` attributed, cut to
  the ratified leg split, rewired to the public ops.** The
  hour-scale cost is verdict-pair walk compute on structurally
  similar trees dense in the near-diagonal (quadratically thinned
  by any strided sample; cache tiling bought nothing). Landed:
  deep = codec + fork + verdict legs + tick with the brute-force
  grow-minimality pin; structural pair legs exhaustive at the
  small bound; the anonymous id leaves the corpus at lowering.
  Rewired to the public surface (`Party::fork`/`join`/`without`,
  Result/Option contracts asserted); no internal entries kept.
  Oracle operating envelope written into `oracle.rs` (small-scope
  only; the oracle is never hardened; fidelity outranks
  robustness); every oracle-facing suite bounded. DECIDED (owner):
  the hour-scale verdict-pair totality test stays, and stays out
  of the gate — `#[ignore]`d, run detached on demand, its budget
  and machine annotation the contract for whoever runs it.
- **DECIDED 2026-07-27 (#54): the diff sweep settles covered
  subtrees as blocks — the early-exit constants recovered, both
  arms linear** (`ac10e61b`). Dyadic nesting makes cover testing a
  depth comparison at every subtree entry, so covered intervals
  settle whole (splice, skip-scan, or lockstep per arm); every tag
  still read at most once; segments 0. The emptiness arm's pooled
  fuel envelope tilts to 1.41: the charter's ≤1.05 expectation
  disputed with evidence — restoring early exits necessarily
  re-creates the rejection-mixture genre; medians and the
  within-case shape leg carry the linearity claim. A planted
  wrong-kind splice mutant reddened the differentials before
  revert.
- **Landed 2026-07-27 (#58, owner-ruled): `causally` composition
  validates; `Decode::Io` displays.** Pairing a start with an end
  passes a well-formedness gate; a crossed pair returns
  `error::Crossed`, so every range that exists has a total
  `placement_of` trichotomy. Consequence class: typed error, the
  crate's convention for compositions that violate a relational
  invariant; panics and debug_asserts stay reserved for internal
  infallibility. Free single-bound constructors remain infallible
  `O(1)`; two-bound compositions cost at most one validating
  causal comparison. `Decode::Io` renders its wrapped
  `std::io::Error` by `Display`, not Debug.
- **FINDING 2026-07-27 (the presize-61 cure attempt; charter
  premise disputed with evidence)**: reserve-once pre-sizing is
  already landed at every builder site; the capacity-phase pair is
  owned by the one op whose output is not size-derivable — the
  materialization's output is mandatorily Θ(|v|·|p|) on a
  Θ(|v|+|p|) input, so the honest-as-a-floor reserve under-runs
  and the builder walks a doubling chain anchored at the input
  size, peak ≈ 3·(n+m)·2^(k−1) with k = ⌈log2(output/(n+m))⌉
  (every probed point within 2%). A size pre-walk and a segmented
  output were priced, not landed; an un-anchored growth chain
  REJECTED on doctrine (tuning the probe pair, not fixing the
  shape). Resolved 2026-07-27 (owner): RATIFIED as the stated-band
  residual — the band is the documented honest profile of a
  non-derivable output, later encoded as the declared capacity
  model at the cell (§17.3), and the lazy projection view makes
  the materialization explicit-only.
- **DECIDED 2026-07-27 (#67, owner-ruled): the skyline topology
  flag inverts (0 internal, 1 leaf) and the version-stream reader
  dissolves into `dsi-bitstream`.** The unshipped-window argument
  carries the protocol change: the inversion folds into the same
  revision that first ships skyline, so no second protocol version
  is burned. A descent is a unary run read word-parallel by
  `codec::DsiCursor`; payload values ride the in-house wide-arm
  wrapper, never dsi's `u64`-capped gamma, with word-seam
  witnesses. The bijection is pinned by transcode differentials
  and the rejection corpus re-pins at the same cut points. Kept
  hand-rolled, stated: the id-stream readers, the borsh
  `ReaderCursor`, writers, and `decode_int` over mid-byte
  sub-slices. rumors rides the revision (content addresses move,
  wire/trace snapshots re-accepted, role-sensitive proxy tests
  arrange sides through the initiator election).
- **Landed 2026-07-27 (suanpan extraction, two phases,
  owner-decided): the accumulator is the workspace crate
  `suanpan`.** Phase 1: `suanpan::Accumulator` behind a pure
  re-export shim, `suanpan::Magnitude` carrying the width-dispatch
  as a trait, IBig differential suites moved with the crate, the
  ratchet triple green through the new home before anything
  retired. Phase 2: the shim dissolved — every call site names
  `suanpan::Accumulator` directly; `suanpan::Limbs` is the one
  home of cross-target words-per-limb packing; the
  `*_magnitude` names TAKEN (the old names carried one consumer's
  concrete type into the generic crate's API); a spare-buffer
  newtype DEFERRED (no decided target shape; one consumer; the
  drained-buffer contract documented with a doctest). Fresh-eyes
  docs rounds to convergence; one verified finding — the zero
  probe reads false on a zero spelled by cancelling digits — led
  to the one-sided contract in name and rustdoc
  (`is_literally_zero`, owner ruling 2026-07-27). Standing policy:
  unpublished until a second consumer stabilizes the API.
- **INSTRUMENTS LANDED 2026-07-27 (the distance/lag families;
  instruments before cures).** The two-operand jump comb — wide
  height-difference crests over a dense-position spine, each
  freeze fired by the operand that did not pay for the drift —
  red-pinned `version_distance` quadratic-class in four
  instruments at once (board cell, envelope + growth floor, judge
  roster, `# Complexity` re-class), with either operand's own rank
  flat. The concurrent pair — emit side switches at every overlay
  boundary — landed as the honest linear density witness.
- **CURED 2026-07-27 (#66 phase 2): the fused distance/lag
  co-sweep.** `Version::distance`/`lag` integrate their measures
  directly over the overlay in one merge walk on the accumulator:
  no join/meet stream materialized, no per-operand rank
  recomputed. The freeze discipline re-derived for the overlay as
  an anchored-segment split; the funding certificate is a
  two-ledger potential, one per operand, every charge naming its
  deposit (the composed form's per-stream argument failed exactly
  at the emission seam). Derivation and certificate live in
  `version/skyline/query.rs`; `suanpan` gained the
  write-watermark scaled read (`sign_magnitude_shl`). All five
  committed readings moved in one commit, parent-measured;
  differential coverage (paper oracle, composed forms, exhaustive
  ordered pairs) landed ahead of the cure.
- **DECIDED 2026-07-27 (#73, owner-ruled): the mirror proxy's
  error selection prefers a deposited root cause over a racing
  consequence.** Reporters never touch the deposit slot; the
  session terminal is its sole consumer and surfaces the deposit
  as `SupplyClosed` on any failed selection, plus one final
  non-waiting poll of the accept driver (no waiting added; the
  link contract's no-deadline liveness posture stands). The
  universal transport-fault attribution pin is restored with no
  carve-outs; determinism demonstrated by seed replay campaigns.
- **DECIDED 2026-07-27 (#75, owner-commissioned): the algebraic
  laws factor into `before::laws`** — one named-predicate
  collection, signature-grouped statics, predicates private behind
  public slices, every consumer wired (the per-group proptest
  drivers, the seeded-adversarial-rank driver, the `fuzz_laws`
  target with committed wide-gamma seeds). Linearity: predicates
  take shared borrows and materialize working copies via
  `dangerously_alias`, confined to predicate scope. Bespoke law
  tests dissolved name-for-name; kept deliberately outside: the
  exhaustive small-scope point laws, the population/fold laws
  (later reached by the arity-five clauses, then the variadic
  drawn-arity drivers — the 2026-07-31 amendment below), and the
  internal byte-equality seam law.
- **DECIDED 2026-07-27 (Finch; landed #71, probed #68):
  `ticks(n)` — the fused multi-tick — is a first-class public
  operation, and every count-of-ticks surface is denominated in
  one opaque unbounded newtype, `Ticks`.** The probe
  (`design/probe-ticks-68.md`) proved the mechanism: `n`
  sequential ticks = at most two fused walks plus one `+n` splice,
  byte-identical to the iterated public tick, at
  `O(|v| + |p| + log n)`. `grow::emit` generalized to the `+k`
  splice (one splice path); three public surfaces
  (`Version::ticks`, `Party::ticks`, `Clock::ticks`);
  `Version::min_ticks` returns `Ticks` exactly at any magnitude.
  The k = 1 splice performs tick's exact metered operation
  sequence in both profiles. Laws, exhaustive small scope, orbit
  vocabulary, and fuzz coverage landed with it; rumors' bootstrap
  dominance seam carries `Ticks` with wire bytes unmoved.
- **DECIDED 2026-07-27 (owner, #72): the batch API is removed from
  `before`** — the handle surface looked like it amortizes work
  and amortized nothing ("a footgun waiting to be accidentally
  discharged"): C2 moved the operation kernels onto the packed
  stream, every op commits as it runs, and the handle survived its
  justification as pure chaining sugar priced as an economy — the
  dissolution doctrine's textbook case. The honest replacement for
  repeated ticks is `ticks(n)`. The roster totality tests were the
  removal's mechanical proof; tests whose entire subject was the
  handle were deleted, not rewritten into vacuity. rumors' own
  public `Batch` (a real amortizer at that layer) is untouched.
  The merge round widened the fuzz-fit corpus of record and
  re-derived the enforcement constants from the sweep's evidence;
  the one gate finding (a rejection band under-pricing the
  full-scan overlap genre at small denominators) was recalibrated
  with the merge, the committed sentry seed pricing in-band.
- **LANDED 2026-07-27 (#77): `OwnVersion`, the lazy projection
  view — projection is lazy at every spelling, and Θ(|v|·|p|)
  materialization is only ever the explicit call.** Charter of
  record `design/own-version-view.md` (every DECIDED entry an
  owner ruling). `&v / &p` and `Clock::own_version()` return the
  ref-owning view in O(1); the comparison matrix fuses projection
  and comparison into one linear co-walk (`skyline::masked`);
  `.to_version()`/`From` are the one product-growth path. By-value
  `Div`/`DivAssign` dropped (census: only their own laws and
  tests). The board's projection rows re-denominate onto the
  materialization door (`own_version_to_version`,
  `clock_own_version_to_version`); the fused comparison rows are
  input-denominated on every shape, output-domination crosses
  included. The correlated mask-drift families landed green — the
  balanced signed-digit integrators hold the fused walks linear,
  so there was nothing to red-pin. rumors' bookmark reclamation
  and suppression seams fuse with zero code change.
- **REVIEW 2026-07-28 (task #37, the whole-state adversarial
  review): five findings, all landed as instruments.** F1:
  `Version::min_ticks` documented linear while
  counter-superlinear on committed families through the public
  API — a wrong committed claim plus a roster-binding hole (no
  test read the board's counter verdicts against the roster
  classes); red-pinned with closed-form semantic legs. F2: the
  freeze-position family FP(k) — Θ(k) freezes each reading the
  position accumulator's full span — falsified rank's linear
  claim where the board read green (every committed family fired
  O(1) freezes: the hole). F3: suanpan's shifted rows' claimed
  shift-independence falsified exactly by the alternating shifted
  pair (the zero-gap walk funded by nothing). F4: the fold
  index's searches were scan-unmetered (the #39 F2 genre) and its
  linearity prose self-contradictory; the instrument gap was the
  finding. F5: the triangle roster's citation check accepted any
  same-named `fn` as a binding test (tamper hole). Attacked and
  sound: rank on every committed family at board scales,
  `meet_all`'s shrink argument, the fold subadditivity escape,
  op-schedule reduction to the family axis, the κ two-leg
  criterion.
- **LANDED 2026-07-28 (the F3 cure): suanpan's cost table stands
  as written — the zero-run ledger prices top maintenance by the
  operand, not the gap.** Owner ruling: fix the code so the
  shifted rows' "amortized O(operand limbs), independent of the
  shift" is true; weakening the table was rejected. The
  accumulator keeps a zero-run ledger of certificates — created
  O(1) by any write landing above `top + 1`, split by interior
  carries, consumed O(1) by the settlement scan and zero-partial
  sign folds, which skip a certified run whole for one touch —
  with the ledger-potential amortization argument written in full
  on the crate page. Charter dispute resolved toward the goal: the
  suggested single write-watermark discipline was refuted before
  implementation (a value parked on digit 0 pins the watermark
  while the oscillation runs unfunded). A dissolution inside the
  cure: the sign collapse's post-zeroing top-walk was dead work,
  deleted not metered. The red pin flipped to exact green
  witnesses; the cost table verified row by row with named
  witnesses.
- **CURED 2026-07-28 (cure round #78): the query folds and the
  instruments, three tracks.** Track 1: rank moved onto the
  anchored-segment integrator (the pair co-sweep's single-stream
  instance — no freeze reads an absolute position), and
  `min_ticks` onto a range-minimum anchor web plus an epoch
  ledger (`skyline/query/web.rs`; reigns settle once at the dying
  record's own funded width, frozen drifts settle once by
  summation-by-parts). FP landed as a board family with the
  known-bad absolute-position kernel committed and failing beside
  it (the adequacy ratchet). Track 3, six landings: the F5
  citation seal (citations resolve to `#[test]` items and law
  tables, two directions); the render finalize cure (digits
  render at the merge that finalizes them into a node-keyed
  arena; the display heap-constant genre cured); declared
  per-cell models (a ratified cost law derived at the cell,
  disclosed on the row face, banded both sides, each with a
  committed wrong-artifact tripwire — the fold `O(D log k)` model
  and the capacity chain the first two); the F4 decision — the
  fold index stays, its searches metered and priced by a declared
  search allowance, the weave family landed, the granted cursor
  revert DECLINED with evidence (the committed overlap
  instruments pin the index's asymptotic win); the class-binding
  seal (board reds carry mechanism tags; no linear-class claim
  may cite a standing exponent-mechanism red); the smoke pin on
  the family axis (per-family expected cell counts as data,
  retiring the hand-counted prose total).
- **REVIEW 2026-07-28 (task #79, round 2): the promotion re-arm.**
  F1: rank's promotion re-read the absolute position accumulator
  over its full written span once per re-arm — counter-superlinear
  through the public API with no committed family firing a single
  promotion (the board structurally blind); red-pinned on the
  re-arm spine PR(p). F2: the co-sweep's freeze-position green
  covered the family, not the discipline (its mate was chosen
  monotone). F3: the fuzz-fit sentry point-judged only sampled
  draws — the deterministic corpus it already executes was judged
  by line fit alone, so a localized region escaped two gates at
  the expected probability; criterion proposed and landed at #81.
  F4: review-number provenance citations in code prose (hygiene;
  deleted). The suanpan ledger and the min-ticks web survived
  construction attempts; a conjectured certificate-stranding
  schedule was retracted by its author after completing the case
  split (dated amendment; the #82 exhaustive driver is the
  committed settlement).
- **CURED 2026-07-28 (cure round #81): the promotion ledger.** A
  promotion performs two funded-width reads and no product,
  recorded as one ledger entry; the ledger settles once at the
  close, suffix masses assembled newest-first as sparse balanced
  signed digits, each arming paying one charge at its parked width
  times its own suffix's balanced density — the epoch ledger's
  discipline with suffix masses where min_ticks has reference
  counts. One change cured rank/distance/lag. PR landed as the
  board family `promo-rearm` (with a charter deviation resolved
  toward the goal: the span builder reshaped so the family
  isolates the promotion mechanism instead of parking an
  unrelated stated-band red; the promotion payload untouched).
  The pair-form family landed (freezes and promotions fired at
  boundaries where the mate's cheap codes set the funded width).
  F3 closed: the sentry judges the deterministic prefix, every
  run, in the same pass as the staleness refit; adequacy shown by
  documented replay under the pre-re-pin bands.
- **VERIFIED 2026-07-28 (round #82, both evidence questions
  settled by construction).** Q1: the ledger settle's
  dense-suffix charge is reachable through the public API and
  quadratic — DS(p, d) (a gap spine of isolated,
  compaction-immune digits, then the re-arm schedule) read local
  exponent ~2.0 through public rank; rank's unqualified `O(|v|)`
  was a wrong committed claim; red pins landed, claims re-stated,
  the module doc, public claims, and committed instruments
  agreeing exactly. Q2: NOT REPRODUCED — no schedule strands a
  suanpan certificate; `hi ≤ top` is a standing invariant,
  collapse included; the mechanical settlement is a committed
  per-step differential driver with an exhaustive prefix-tree
  sweep over an 11-op alphabet. The recommended per-op ledger
  debug_assert DECLINED under the assertion doctrine: the
  exhaustive driver samples the same transition space
  systematically; same instrument, worse price.
- **PROBED 2026-07-28 (round #85, the skip-mechanism dissolution
  question; owner's suspicion, adversarial stance, dissolution
  authorized on an earned refutation). Verdict: LOAD-BEARING, all
  three.** Each of the accumulator's skip/extent mechanisms — the
  zero-run ledger, the `bottom` write watermark, exact-`top`
  maintenance — has a public-API family that is superlinear
  without it and flat with it (weight comb ×1.93/byte without the
  ledger; freeze parade ×1.91 without the watermark; tooth-tail
  ×1.98 without the settled top), committed as three green
  flatness bands whose ceiling docs carry both readings.
  Kill-switch probe builds were value-identical before any cost
  reading was trusted; each mechanism's own row witness read red
  under exactly its absence (a liveness check of the probe
  itself). The census: cheap gap creators exist in exactly one
  place `before` reaches (the query integrator's weight-shifted
  deposits, topology-priced); everything else enters at digit 0
  or at code-funded width. The one redesign that could re-open
  the question (a lazy top settled only at fold collapses) was
  steelmanned and declined without a build: it forfeits the
  public O(1) `digit_count` contract and hands stale width to
  `&self` read paths that never heal; anyone reviving it owes
  that witness campaign. Nothing dissolves; no envelope reverts.
  The witness families were subsequently promoted to board
  columns (weight-comb and freeze-parade; tooth-tail followed at
  the #76 floor re-derivation below), their generators
  single-sourced so the bands and the board consume the same
  constructions.
- **CURED 2026-07-28 (cure round #83): the balanced product-tree
  settle.** `settle_armings` became a balanced reduction over the
  ledger's entry sequence: every arming-window cross term rides
  exactly one aggregate product, nothing re-read more than
  logarithmically — the accounting-direction game closed (a
  shared dense suffix cannot be walked once per arming). The #82
  pins flipped to flatness bands under the declared log model;
  DS/DSM moved into `src/meter` as bit-exact generators; the
  retired per-arming walk committed and failing beside the
  kernel. The discrepancy, resolved toward honest claims and
  reported: the ratified unqualified `O(|v| log |v|)` is not
  achieved by any fixed-association settle — the wide-arming
  family WA (one arming as wide as the input ahead of a mass as
  dense as the input) reads the aggregate product itself
  superlinear; misaligned cancelling armings force the same price
  at any fixed tree association (only a cancellation-adaptive
  settle could exploit them — research-shaped). Class decision
  (smallest honest vocabulary): rank/distance/lag stayed
  `Class::Linear` with the superlinear rustdoc rider and the
  tests-only red witness; `FoldLog` declined at those rows
  (a false upper bound while the wide-arming pin stands);
  superseded by #91's `MulBound` below.
- **BUILT + RED 2026-07-28 (wedge round #76, the correlated n-ary
  fold populations).** The stagger population (operand `i` owning
  slot `i` of every block, fed in bit-reversed order so every
  counter merge joins top-diverging region sets — the foreclosed
  luck) proved the join folds HONEST under joint correlation:
  model-normalized constants flat, sequential controls ×2.5–×5
  worse, four committed bands. `Version::meet_all` was NOT
  honest: a non-shrinking accumulator (a meet shrinks the value,
  never necessarily the packed size) read the exact product law
  Θ(k·d) on the shade population — red pin committed, an
  in-campaign mandatory cure per the owner's standing ruling.
  The aligned-pair floor re-derivation (owner ruling, executed
  here): the old per-stored-delta pair floor premise was wrong —
  the fused sweep folds per stepping overlay boundary, so the
  floor is `max(deltas(v), deltas(w))`, derived from the sweep
  and true for ALL pairs with no per-family carve-outs; tooth-tail
  promoted to its full board reach under the re-derived floor.
- **CURED + TAPPED 2026-07-28 (fix round #94).** The meet cure:
  `Version::meet_all` runs the join folds' balanced
  binary-counter reduction — one shared helper, genericized over
  the combiner, so the two lattice folds' cost model is uniform
  by construction; the sequential reduce committed and failing
  beside the flatness band; claims `O(D log k)`/`O(D)`, roster
  `Class::FoldLog`; the board row landed. The counter's one home
  (owner-approved DRY): `crate::fold::balanced_try_fold`
  (fallible combiner, accept predicate, feed-order rejection
  channel) carries the party/clock/version folds; pure refactor,
  boards byte-identical. The window-digit tap: `WindowMass`
  combine moved digits invisible to every counter — one limb
  count now recorded per merged position, with the known-bad
  per-digit absorb committed and failing exactly because the tap
  exists. Claims to the committed witnesses: the #83
  no-intrinsic-amplifier argument RETRACTED by construction — the
  plateau-puncture family embeds an exact
  `Θ(w)-digit × Θ(d)-digit` product in the answer from
  `Θ(w + d)` input bits, so `Ω(M(|v|))` time is mandatory on
  adversarial inputs; rank/distance/lag upgraded to the
  three-part claim form.
- **CURED 2026-07-28 (cure round #91, owner-mandated: "I don't
  accept quadratic operations, honestly or otherwise"): the
  settle products to the multiplication bound.** Both residual
  sites charge through one path: settle products split into
  clusters at zero-gaps wider than the factor's own width, each
  cluster densifies into at most two magnitudes and rides one
  backend multiplication (dashu, at whatever tier its dispatch
  engages), metered as traffic; the close-time settle compacts
  the segment through the window spelling first; the ledger
  settle re-associates through a mass-balanced product tree
  (node products shrink geometrically, telescoping into the
  root's bound under any power-law tier — the entry-count
  counter was measured out). Cost bound [derived, query.rs
  module doc]: `O(M(|v|) · log |v|)` worst case — the depth is
  logarithmic in the input-funded settle mass, never the arming
  count (the #100 review's correction, committed as a
  linear-depth witness on exponential masses) — with the log
  absorbed below the backend's quasilinear threshold and on
  `O(1)`-arming inputs; `Ω(M(|v|))` mandatory (the
  answer-embedded product, re-shaped by the #100/#102 review
  fixes into the general `puncture_product(x, y)` reduction from
  arbitrary integer multiplication, `x` dense pseudorandom, `y`
  jitter-strided, so no closed form telescopes it); the
  quadratic ceiling derivation RETIRED under the dissolution
  ratchet (every family it held reads flat under strictly
  tighter probes; what it caught the schoolbook kernels keep
  catching red). Both red pins flipped to flatness bands with
  the schoolbook kernels committed and failing beside them; a
  public-fold differential drives dense factors through the
  settle at every backend tier boundary. The class vocabulary
  unified per the owner's ruling: every `Class` variant declares
  one `ClassContract` (exponent-red stance, judge-red
  membership, defining token with exclusivity, named witnesses),
  enforced by one uniform test with an exhaustive match;
  rank/distance/lag moved `Linear → MulBound`. The residual log
  is stated exactly, never hidden; removing it has no known
  fixed-decomposition path and is not commissioned.
- **Landed 2026-07-28..30 (the post-#91 tranche; landed shapes of
  record, each with its instruments — detail in git history and
  the named module docs):**
  - *The red-triage doctrine realized* (the owner's 2026-07-28
    "accepted red" ruling): `BOARD_EXPECTED_REDS` is an empty
    triage buffer — every entry must carry a live task, and the
    acceptance assertion
    (`expected_red_buffer_is_an_empty_triage_buffer`) refuses any
    entry at all at acceptance. Every former standing red resolved
    to a cure or a dated owner-declared model at the ceilings
    module (§17.3): the ascend-cliff tick certificate constant and
    min_ticks reign constant (ratified 2026-07-28, conditional on
    their measured flat-constant profiles), and the mirror-wide
    render limb model (exponent and per-`R` constant ceilings,
    with `render_merge_superlinearity_is_alive` as the class's
    liveness floor).
  - *The family registry* (`meter::registry`): `FamilyId` the
    single source of truth, `Shape` the only construction door,
    every instrument's family axis derived from the roster (§2).
  - *The board's structural rounds*: the board module split into
    single-responsibility submodules; process sharding on the cell
    grid (family-outer deal, byte-identical by the shard pin,
    `just amp-board-shard-pin`); the worst-case map (argmax family
    per operation per currency, `just worst-cases`, its ranking
    pin gate-enforced); the settle families promoted to board
    columns (dense-suffix, plateau-puncture, lone-freeze) — whose
    promotion surfaced and cured the wide-arming text-parse
    quadratic (settled-top extraction; the column lands green) —
    and the plateau-puncture base re-pinned to its measured
    margin floor for wall-budget.
  - *The Rank wire form and the Ranked view*: a canonical
    prefix-free Rank encoding whose byte order is `Ord` (strict
    decode rejects non-normalized forms, per the standing Rank
    deferral's condition); `Ranked` as a borrowing rank view —
    fused comparisons, fused encode, rank equality; `Ranked` as a
    total causal-ordering key (byte order == `Ord`, `Eq` is
    version identity); borsh transport composable; board rows,
    worst-map pins, and rosters landed with it; `Ranked` and
    `Rank` compare only with themselves (the rank question is
    spelled `to_rank`).
  - *The causal placement surface*: the `causally` placement
    specs (the bounded six-way and the range nine-state verdict
    faces, laws and witnesses); one fused placement co-walk
    generic over its verdict, with `placement_of` and `contains`
    as coarsenings and the composition as its oracle; the causal
    interval type named `Span` with position-denominated
    dominance variants, the `spanning` door total over any
    nonempty collection; the tree's Knowledge classifiers and
    range walk ride the placement face; the tree memo stored as
    one causal `Span` (`lo <= hi` rides the type).
  - *The fused hull* (#133, owner-requested capability): total
    hull construction on `Version` (`span`/`span_all` beside
    join/meet) — one pair walk feeds both span endpoints; the
    hull fold one balanced reduction carrying both endpoints;
    lattice-hull and lattice-fold laws reach every combine arm
    (the arity-five clauses); its adversarial review's fixes
    merged (the crossing-fold pin moved to the touch leg that
    can see it; the organic differential; the span saving stated
    exactly).
  - *The recv/sync fusion* (#119): `sync` rides one sum-split
    walk over the party pair (scan up to 7× lighter,
    worst-case argmax re-pinned), with composition laws as the
    total oracle; the recv fusion was disputed with evidence and
    not landed.
  - *Fold unification, phases A and B* (#108/#109; the survey and
    its outcome record are `design/fold-unification-survey.md`):
    the overlay-advance law once, over plateau cursors
    (`PlateauCursor`, the generic binary advance; sweep, emit,
    and the pair integrals migrated); projection, the masked
    walk, and the id difference onto the law; `OpenedPair` the
    one home of the two-skyline opening move. Phase C measured at
    the boundary and dropped by the survey's own criterion. The
    owner's identical-or-better acceptance ruling (2026-07-29,
    mid-flight) is recorded there: deliberate, accounted
    improvements supersede byte-identity acceptance gates;
    regressions remain findings.
  - *Surface totality from rustdoc JSON*: the coverage roster
    held to the compiler's own account of the public surface
    (`just surface-totality`, in the gate), replacing the
    source-scrape extraction as the totality authority.
  - *The fuelscape atlas* (`crates/before-fuelscape`, an external
    instrument): per-operation heatmaps of deterministic fuel
    against exact input size, sampled uniformly from each size's
    whole canonical input space, committed families overlaid; it
    enforces nothing (sampler correctness and coverage parity
    against `crate::surface` are its committed checks) and may
    read the roster but never mint a threshold. Guest kernels
    landed for five walks the atlas could not price; the
    then-queued span exemptions dissolved with the
    causally-kernel round's span half (the 2026-07-31 amendment
    below), and the surviving rank-view exemptions are §17.2's
    open item.
  - *The validation index* (`testing::validation_index`): the
    documentation-only map of every instrument, what failure
    class each catches that the others cannot, and the triage
    guide — the maintainer's cold-orientation page.
  - *Claims machinery generalized*: the crate-agnostic
    complexity-claims engine extracted to the workspace crate
    `complexity-claims`; `suanpan` carries tier-3 complexity
    claims (sections, roster, cost-table binding with exact
    touch totals).
  - *Meter honesty rounds*: pair-walk touch floors re-derived to
    minimum-possible work (the liveness-floor premise ruling —
    floors derive from the mechanism's irreducible per-boundary
    work, no per-family carve-outs); liveness floors under the
    accumulator-stream touch ceilings; the first-freeze-gate
    straddle bands, the ticks(n) wide-count width band, the
    n-ary aliased-rejection band, and the puncture reduction's
    stored-size premise pin; the query integrator's segment and
    window feeds open at the first freeze; the practical-regime
    rank gauge pinned on the concurrent operand; suanpan's
    `sign_dominates_at` saturating near `usize::MAX`.
  - *The exposition deck* (`design/exposition/`): fact-diffed
    against the landed implementation, with attack cards per
    family genre and the white-box construction audit in its
    method section.
  - *The entropy pin*: the exhaustive decoder census
    byte-compared against the count tables to 24 bits
    (`design/before-version-entropy.md`'s companion instrument).
  - *The Span wire form* (at this tip; its review since merged —
    the 2026-07-31 amendment below):
    a canonical composite encoding with a one-pass fused decode,
    `Span::new_unchecked` the trusted door, reborrow/into_owned
    doors, codec laws and pins (verdict identity, rejection
    witnesses, fused-decode meter legs), board rows and worst-map
    pins, and the streaming backend's Node trait collapsed to one
    span accessor.
- **AMENDED 2026-07-30 (round #148, owner-directed): this document
  restored to a forward-looking plan.** The re-accreted execution
  history (round-by-round measurement narratives, superseded plan
  waves, completed checklists, stale sums) is excised to git
  history; the decision record above is the consolidated compact
  history (every ruling preserved, execution chronicle condensed
  to landed shapes); §14/§17.2 re-derived to the remaining tail
  against the tree at this tip; every retained number re-verified
  against the code or replaced by the mechanically-enforced home
  the prose cites (the structure-not-tallies rule, §17.10).
- **AMENDED 2026-07-31 (review #141 fix round): the remaining
  tail's first five items landed; §14/§17.2 re-derived to the
  survivors.** Landed shapes, at the commits this entry names:
  the Span wire-format review merged on the mainline (the serde
  half of the Rank/Ranked/Span transport matrix at 1bd2c94c,
  total structural rejection-genre ordering in Span decode at
  8209cd7c, the review sweep at 1718dd48/712b8bd3); the
  causally-kernel round's span half (978dfcdd: span guest kernels
  with the span_place/span_dominance/span_encode/span_decode
  panels — the rank-view half survives as §17.2's open item, its
  exemptions carrying reasons, none citing a kernel that now
  exists); the `step!` retirement under the ratchet
  (05b40a62/72e3681f/3a4f3c8e, the replacement demonstrated red
  first through the board scan column), with the
  stacker/`descend!` remainder resolved in this fix round —
  `stacker` is a dev-dependency only, and the keep is a dated
  decision in `recurse.rs`'s module doc; the variadic law suites
  (8e35f95a/cfe4853c/8afeff30/30133936, the fixed-arity clauses
  dissolved under the retirement ratchet at 71dd0edc); and the
  Bytes representation round, both phases and its review closed
  (92ed2202/8ce1aac4/deb2cb76/20bd3c19/ee8e5fd7; review #157
  fixes af7bdc83/41f8ae80/2a701f85).

## 13. The metering gate

The board (`before::meter::board`, `just amp-board`, runner
`examples/amp_board.rs`): a red-green matrix over the entire
public operation surface × §2's families, judged at two scales
(default; `board::ACCEPTANCE_SCALE` = ×4,
`just amp-board-acceptance`) at the **release profile**, the
measurement of record (§12's ratification), from deterministic
meters only: peak heap, grown stacker segments, limb ops,
scanned/written bits, and accumulator digit touches — every cell
verdict plus five judged counter columns. The board is a
generalized cartesian product over three declarative axes: shapes
declare operand bundles, operations declare the slots their
signatures consume, and every judged quantity carries one field
per metering currency (`board::ByCurrency`), so
every-shape-everywhere and every-currency-everywhere hold
structurally — adding a shape or operation grows the product, and
adding a currency is a compile error until every operation
declares a floor or a written NA for it. The family axis derives
from the registry (§2), and cell membership is committed data:
the board smoke suite holds the rendered matrix to each family
spec's declared bundle reach, so the cell population is enforced
per family by name, never restated in prose. The board runs
sharded across child processes on the cell grid (family-outer
deal), byte-identical by pin (`just amp-board-shard-pin`); the
worst-case map (`just worst-cases`) folds an argmax family per
operation per currency with a gate-enforced ranking pin
(`just worst-cases-pin`).

**The board reads no clock**: its entire rendered output is
byte-identical at a given scale under any machine load, no
stripping, no carve-outs [measured — under a sustained
parallel-build load generator], and the claim is enforced on two
legs — the runner measures every cell twice in process and panics
on any counter disagreement, and the gate's
`just amp-board-determinism` byte-compares two cross-process
renders. Wall time is judged nowhere in the gate; the time leg
lives in the bench judge below, at `just bench-judge` / `just all`
cadence. Instruction-count asymptotics are the fuzz-fit harness's
territory (`crates/before/fuzzfit`, `just fuzzfit`, in the gate:
fuzzed operation programs replayed under wasmtime fuel,
deterministic and load-independent, judged against pinned
per-operation fuel bands over the deterministic prefix — totally —
plus random draws; its design record,
`design/before-fuzzfit-asymptotics.md`, is the instrument of
record for that claim). The fuel *distribution* is the fuelscape
atlas's territory (`crates/before-fuelscape`, `just fuelscape-test`
in the gate for its own checks): an audit view that enforces
nothing and may never mint a threshold.

Ceilings (`meter::board`'s ceilings module carries every constant
and its derivation): scaling exponent ≤ 1.15 (per cell, fitted
across the two scales against the cell's denominator bytes); heap
≤ 16 B per denominator byte over an 8 KiB flat allowance; grown
segments ≤ 1; limb ≤ 128 ops/byte on input-denominated rows; the
text rows per §6 (κ constant leg + `n_io` exponent leg); scan
≤ 96 bits/byte on walk rows; touch ≤ 96 digit touches/byte.
Exponent legs are fitted only where the cell's denominator pair
scales (≥ ×1.5 between probes) and, on heap, where a reading
clears the flat allowance the constant leg already forgives; an
unjudged exponent renders `-.--` and the cell rides its constants
and floors (§12's judgment-layer decision; guard tripwires
committed). **Declared per-cell models** replace a global leg
where the owner has ratified a derived cost law at the cell,
disclosed on the row face (`decl[...]`), banded both sides so an
improved kernel trips the stale-model floor and forces a
re-declaration, each with a committed wrong-artifact tripwire:
the fold rows' `O(D log k)` scan model and search allowance, the
capacity-chain peak on the materialization pair, the ascend-cliff
tick and min_ticks heap constants, and the mirror-wide render
limb model (§17.3 enumerates them with their owners). Green = all
columns within ceilings AND all floors met.

**Liveness floors** (user ruling 2026-07-24: the board judges the
API surface as well as the implementation — a ceiling over a dead
counter proves nothing). Every cell carries a floor-or-NA
declaration per judged column, demanded by the `Cell` type and
rendered as a legend; a floor trip is red with the mechanism named
("counter reads below floor: the meter is not watching this
work"). Floors derive from the mechanism's minimum *possible*
work, one universal premise per convention, no per-family
carve-outs (the owner's floor-premise ruling): scan floors 1 bit
per packed operand byte on every row that must examine its
operands (early-exit rows floor at 2 bits — the root codes); limb
floors where big-integer arithmetic is semantically mandatory, at
two derivations — stream-reading rows floor per stored payload
*code* wider than 128 bits, value-materializing parse rows per
stored *base* wider than 128 bits; heap floors on codec and text
rows plus the fork rows' child-copy floors; touch floors at one
touch per stepping overlay boundary on the pair kernels
(`max(deltas(v), deltas(w))` — the boundary premise, derived from
the fused sweep; equal operands declare NA) and per stored delta
or per wide code on the single-stream conventions; segments
ceiling-only (its honest floor is zero). NA genres: wholesale
byte moves (encode, hash, byte-decided equality), operands with
no packed stream, empty forms. A floor trip is a designed
stop-and-look; an implementation that legitimately does less work
lowers the floor deliberately — and an honest input tripping a
floor from below is a floor-premise finding, not a meter bug.
Floors have caught live regressions (the id-renderer scan
vacuity; unmetered window fast paths; the byte-decided equality)
— the instrument works.

Scan-meter contract notes in force (`codec/scan.rs` states them):
the gamma window fast paths record the same bits the per-bit loop
prices; the wire-side borsh `ReaderCursor` is deliberately
unmetered (no board row prices the wire path; instrumenting it is
a conscious future change with its own recalibration); the
`max_depth` caller-side record double-counts uniformly (2×,
deterministic) and carries a `TODO-recalibrate` for its own
future commit.

Tripwires (every criterion demonstrates the status quo fails it):
the bypassing-walk floor tripwire (committed in the board's
suite); the κ pair (§6); the judge's unmetered-quadratic bench
(`benches/tripwire.rs`, `just bench-judge-tripwire`,
`--expect-red`, e = 2.00 measured) plus its deterministic twin in
`tools/benchjudge --self-test`, run at the head of every judge
recipe.

Dashboard caveats of record: the board shares one process, so its
heap numbers are indicative and the process-isolated envelopes in
`tests/meter.rs` remain the enforced record; segment counts have
a ~1 MiB growth threshold, so the default scale under-detects
segment onset — which is why acceptance runs at ×4 too. One
recorded non-monotone verdict genre: a flat per-byte ceiling
against an n·log n constant can read red at default and green at
×4 — record-scale greenness never clears a default red.
Record-scale runtime budget: ≤ 30 s summed measured-body wall per
family.

**The red set** is empty on the settled tree: `BOARD_EXPECTED_REDS`
is an in-flight triage buffer whose every entry must carry a live
task, asserted EMPTY at acceptance
(`expected_red_buffer_is_an_empty_triage_buffer`) — red is
reserved for untriaged contradictions, and every resolved
contradiction is either a cure or a dated declared model (§17.3).

### The rejection surface (fallible operations)

Cost claims are total: rejecting an input is an outcome with a
cost, bounded like any other, whether or not the caller honored
the usage invariants (§12's rejection-cost decision). The board's
rejection rows (the `defect` module in `meter::board` is the
enumeration of record — overlap, fold hand-back, empty
difference, strict decode for every exposed codec including the
Rank/Ranked/Span composites, and text parse) measure the
rejection side under all five currencies, with the defect
**maximally deferred** in every committed shape — an
early-exit-only measurement is the cheapest artifact that would
pass, so every shape places its defect where rejection must
consume as much input as possible (truncation at the last byte,
trailing junk after the complete valid stream, non-canonicality
at the preorder-last position, overlap at the preorder-last
terminal, crossed spans judged after both component parses).
Adapter outputs for the overlap rows are semantically void by
design (a well-formed pair no legal fork/join history produces);
the cost claim is what the rows price. Not-rowed genres carry
their reasons in the defect module (word-scale operands,
delegation to a rowed validator, a failing reader being a
truncation carrying an error).

Rejection-row conventions: **denomination** — against the fed
stream alone (§6). **Floors** — packed-stream rejection rows
floor scan at one bit per fed byte with the defect-placement
derivation (a self-delimiting stream's terminal defect is only
discoverable by parsing to it; the packed coding has no random
access); heap, limb, and touch are NA on rejection rows
(rejection materializes no result and forces neither value work
nor an accumulator fold). Text-rejection rows declare no floor on
any column, by honest derivation: no deterministic counter
watches text-byte consumption, and a parser may find the defect
in tokenization before any packed or value work — their ceilings
judge live readings and the time leg times them like every row.

**The bench judge** (`tools/benchjudge`, stdlib Python;
`benches/board.rs` driven by the board's own cell table so bench
IDs mirror board cells by construction). The pinned mode times
the rule-derived subset — every operation on the benign control,
each shape's designed-stress pairings declared on the shape axis,
the declared-model riders (`BOARD_DECLARED_BENCH_RIDERS`: the
`version_min_ticks` reign-state cell and the mirror-wide display
pair — a cell judged green under a declared counter model keeps a
wall-clock witness even where no designed pairing times its
shape), plus the wide-display pair; membership is verified
against the criterion `--list`. `BOARD_BENCH_MODE=full` times the
whole product for final verdicts. The judge fits each cell's wall
exponent across two saved criterion baselines (scales 1 and
acceptance), denominated against the board's per-cell denominator
bytes (never the scale knob), judging every cell whose hi median
reaches the resolution-derived 10 µs floor. Ceilings ride the
**sidecar**, never the roster: bench code declares each cell's
ceiling class at its definition site (general 1.3, text 1.7; the
text-class set pinned as exactly the wide-display pair), the
judge cross-checks the two sidecars per cell, exit 2 on
disagreement. Sidecars are stamped (scale, profile, sampling, git
tip) and cross-checked — stale or mismatched baselines refuse.
Exit contract pinned end-to-end in `--self-test`. Sub-floor cells
are SKIPped and listed (documenting cheapness); the fit-noise
band bounds resolution's pull, and the roster's `boundary` class
accepts either verdict only within that band.

**The expected-red roster** (`tools/benchjudge-expected.json`):
membership by cell name, expectations only (`red`: must be judged
and read RED at the cell's own ceiling — GREEN is a
verdict-flip/liveness signal and SKIP a drift out of judgment,
both exit 1; `boundary`: within the band). Any unrostered red
fails. Membership and the text-class set are pinned by
`crates/before/tests/bench_judge_roster.rs`, so every edit trips
a reviewed diff. Population at this tip: **the permanent
schoolbook tripwire (`display_schoolbook/hugeleaf`) and the
hugeleaf display pair (`version_display/hugeleaf`,
`clock_display/hugeleaf` — conversion-dominated wide rendering
measured over the general 1.3 ceiling; whether the cure is a
faster render or a text-class migration is the open class
question, owned by §17.2's final review); boundary empty.** Every
other cell — designed diagonal and declared-model riders alike —
must fit under its own ceiling: a constant-factor counter model
is not a time-exponent red.

**Acceptance (the campaign's; protocol per §12's ratification):
all-green means the release-profile board green on counters and
floors at BOTH scales, one run each under the committed
determinism tripwire (the runner's in-process double measurement
plus the gate's cross-process byte-compare), with
`BOARD_EXPECTED_REDS` empty; AND the bench judge roster-satisfied
at both scales in both modes at the roster membership current at
the sweep** — record sampling belongs to this acceptance sweep
alone (the standing cadence judges in quick mode). Dev runs
remain a debugging view and never satisfy acceptance. The final
renders of the acceptance sweep are the numbers of record;
per-round movement lineage is in git history.

## 14. Execution plan

The completed phases' narratives (P0 through the post-#91
tranche) live in git history; §12 is their decision spine. What
follows is the remaining tail, in dependency order, with the
acceptance contracts in §17.2; nothing else remains.

**The remaining tail:**

1. **The fuelscape rank-view kernels** — the causally-kernel
   round's surviving half: guest kernels for the rank-view walks
   whose exemptions record that no guest kernel exports them,
   dissolving those exemptions into panels.
2. **The tick-seam cache probe** — measure-first, with its kill
   bar.
3. **Closeout obligations** — the proportional fuzz heap cap
   decision, the board-ceiling finalization leg, and the
   documentation closeout items (P5-genre; §17.2 item 3).
4. **The comprehensive fuelscape survey of record and the fuzz
   soak** — on the remote machine, at the final tip.
5. **The final adversarial review** — in flight: the lens deck
   plus the carried items.
6. **The wall-clock benchmark legs on the machine of record** —
   the bench judge at record sampling, both scales and modes,
   and the before/after table of record.
7. **The benchmark section and the final prose cycle** — the
   crate-doc Efficiency section re-measured, the remaining
   factual doc items, and the owner's sentence-level pass.

Then the go-criteria below gate the merge to main.

**The per-operation performance bar** (user ruling 2026-07-25;
judged by the before/after table and the acceptance sweep):
strict absolute improvement everywhere is the target and the
expectation; the floor per operation is **at or close to parity
on benign inputs, asymptotically optimal, and entirely free of
adversarial exploitation surface**. A benign cell slightly slower
under an adversarial mitigation's constant is triaged hard for a
cure but does not block; anything below the floor blocks.

**The asymptotic bar** (user ruling 2026-07-25): **every public
operation must be subquadratic in its total input, worst case;
linear is the ideal.** Fundamentally-superlinear problems (radix
conversion; multiplication-equivalent up to log factors — the
`MulBound` query folds; n log n comparison-ordered n-way folds)
satisfy the bar at their problem's own optimum, stated and
priced. The tick kernel's limb dimension sits at amortized
O(n + m), realized inside the fused tick (the tick cost spec's
T-tick).

**Go-criteria for merging to main** (the acceptance sweep of
record; every leg on a checked-quiet machine where wall time is
judged):

- `just all` clean at the final tip (the gate plus the feature
  matrix, fuzz smoke, lean, wasm, and the judge legs).
- The §13 acceptance criterion met in full: boards all-green at
  both scales under the single-run determinism protocol with the
  red buffer empty; bench judge roster-satisfied at both scales,
  both modes, at record sampling — only permanent expectations
  standing, which means the display-pair class question (§17.2
  item 5) is resolved or explicitly re-ratified by the owner at
  the sweep.
- The before/after table of record shows the parity floor met
  everywhere and improvement where claimed (§17.2 item 6's
  protocol).
- The fuzz soak and the fuelscape survey of record completed at
  the final tip on the remote machine, findings triaged to zero
  or owner-ratified (item 4).
- The coverage audit re-run with an empty gap list: every public
  operation names its oracle legs and its resource pin through
  the roster (surface totality from rustdoc JSON), and every
  exposed type's bytes/text/serde forms are snapshot-pinned
  in-crate (the representation-pin directive).
- The final adversarial review closed: findings shifted from
  "this is wrong" to "you might want to think about this", every
  landed finding a committed check, the carried items each
  resolved or explicitly owner-deferred with a dated record
  (item 5).
- The integration review against main: merge-seam re-sweep
  (mechanical greps plus marker/ledger checks) across everything
  merged since the last sweep, and function-level integration
  review of the merge itself.
- This document's §12 gains the acceptance entry (date, tip, the
  sweep's numbers of record), and the campaign branch merges.

## 17. Work items of record

### 17.2 Open items, with acceptance contracts

1. **The fuelscape rank-view kernels** (the causally-kernel
   round's surviving half; the span half landed — §12's
   2026-07-31 amendment). The atlas's exemption roster
   (`before-fuelscape`'s ops module) carries rank-view entries —
   the `Ranked` comparisons and encodings — whose reason is that
   no guest kernel exports the walk. The item adds the guest
   kernels (a `crates/before/fuzzfit` change, as the exemption
   text records) and the matching atlas panels, dissolving those
   exemptions. *Acceptance*: the rank-view exemption entries are
   gone, replaced by panels; the ops parity test
   (`panels_and_exemptions_tile_the_coverage_roster`) holds the
   tiling; new fuel bands pinned with the standing
   liveness-margin convention; no surviving exemption cites a
   kernel that now exists.
2. **The tick-seam cache probe** (coordinator-seeded; measure
   first, kill bar). The seam: rumors' `Tree::act` advances the
   running version by one full fused tick walk per action (the
   tick-and-clone chain recorded at §12's batch-removal entry).
   The probe measures the seam's real profile first — what
   fraction of an action batch's cost is the per-action tick
   walk on realistic trees — and only on measured evidence
   constructs the cache (the ticks(n) probe's mechanism note
   that the route is value-blind and its site stable is the
   candidate lever). The kill bar is legitimate here (a
   speculative probe, not an owner-requested capability): the
   probe dies unless measurement shows a win worth the
   machinery, and an honest kill is a first-class outcome.
   *Acceptance*: the measurement lands as a dated record either
   way; a landed cache rides the standing ratchets (instruments
   before cures; movement parent-measured; no new unmetered
   state).
3. **Closeout obligations** (the P5-genre remainders, each small
   and already scoped):
   - *The proportional fuzz heap cap*: the fuzz harness runs
     every input under a flat 1 GiB peak-heap cap
     (`fuzz/src/lib.rs`), documented as not yet proportional to
     input size. Land the proportional ceiling, or the owner
     ratifies the flat cap with the harness doc re-derived to
     the ratified shape.
   - *The board-ceiling finalization leg*: board ceilings
     tightened to final constants at record scale (release,
     single runs under the determinism tripwire) — belongs to
     the acceptance sweep, after the in-flight rounds' cells
     settle.
   - *Documentation closeout* (user sign-off, item by item): the
     §6 invariant statement lands in the crate docs as contract
     (over content bits for content-materializing operations,
     packed operands for delta-native ones, every cost claim
     carrying its epistemic status); the `Key` stability promise
     in `rumors`' `src/tree/key.rs` gains its same-code-version
     qualifier; the bookmark version-mismatch semantics stated
     at the bookmark docs. These are the factual-docs half; the
     style half is item 7's prose cycle.
4. **The comprehensive fuelscape survey of record and the fuzz
   soak** (remote machine, final tip). Both run on ox-east-1 at
   the tip every other item has landed on: the fuelscape atlas's
   survey of record (heavy sampling across every panel — the
   audit view of where the bulk of each operation's input space
   sends it; the atlas enforces nothing, so the survey's
   deliverable is the rendered distribution plus any anomaly
   triaged into an instrument), and the fuzz soak over the
   before-side targets (`fuzz_decode`, `fuzz_decode_ops`,
   `fuzz_laws`) under the heap cap. Frame-level fuzzing of
   `rumors` stays deferred to a future campaign; its spec
   (`design/rumors-frame-fuzz.md`) is the record that campaign
   resumes from. *Acceptance*: both runs dated and
   machine-annotated; every fuzz finding a committed seed with
   its fix or an owner-ratified disposition; survey anomalies
   triaged to instruments or explicitly recorded as expected
   strata.
5. **The final adversarial review** [in flight, 2026-07-31] (the
   campaign's closing review; independent agent,
   blind-spot-targeted). The lens deck, each lens a pass with a
   constructed-wrong-artifact win condition:
   - *mutation-adequacy attack*: plant wrong kernels/mutants and
     verify the committed instruments convict them (the
     adequacy ratchet audited end to end);
   - *blind-spot escalation*: the negative space of every prior
     review round — what no round examined;
   - *prose re-derivation*: every rustdoc and design-doc cost or
     contract claim re-derived against the tree
     (statement-faithfulness, both directions);
   - *white-box worst-case construction*: per operation, read
     the implementation and construct the maximizing family,
     duals included, diffed against the registry roster — the
     blindspots live in shapes no committed family exercises;
   - *criterion-gaming attacks*: for each committed criterion,
     the cheapest artifact that passes;
   - *merge-seam re-sweep*: every track merged since the last
     sweep re-swept mechanically;
   - *integration review vs main*: semantic (function-level)
     integration of the campaign branch against current main;
   - *ghost/doc gates*: nothing anywhere refers to code that no
     longer exists; both rustdoc builds warnings-denied;
   - *stale-count excision*: no hand-maintained tally survives
     where a mechanically-enforced home exists.
   Plus the carried items, each to be resolved or explicitly
   owner-deferred with a dated record: the bench judge's
   sub-resolution window (a superlinear term whose constant sits
   below the 10 µs floor at both bench scales on cells with no
   counter leg); index-shaped search metering as a general
   primitive (`metered_partition_point` is `IdIndex`-local); the
   deliberately unmetered wire-side `ReaderCursor` (its
   recalibration note in `codec/scan.rs`); the `max_depth`
   double-count `TODO-recalibrate`; and the hugeleaf display
   pair's class question (§13's judge roster — cure the
   conversion-dominated render or migrate the pair to the text
   class; the owner rules). *Acceptance*: the review closes when
   findings shift from "wrong" to "consider"; every landed
   finding is a committed check.
6. **The wall-clock benchmark legs on the machine of record.**
   The bench judge at record sampling, both scales, both modes,
   on a checked-quiet machine (poll-until-quiet, bounded; load
   disclosed in the report if any) — record sampling belongs to
   this sweep alone. And **the before/after table of record**
   (judged under §14's two bars). Protocol, mandatory: re-bench
   the pre-C2 tip under the FINAL harness — a temp worktree at
   the last pre-flip commit with the current bench files
   grafted on (they call only the public API), full sampling
   both tips, warm target dirs — **never the stored `base`
   baselines**, which are contaminated for delta purposes (the
   RNG consolidation regenerated every bench input family, and
   they mix sampling modes). Any benign regression beyond
   "slight" is a finding; the parity floor is the bar.
7. **The benchmark section and the final prose cycle.** The
   crate-doc Efficiency section re-measured under the landed
   representation with `just readme` re-derivation (the READMEs
   are derived, never hand-edited); the space-consumption
   figure re-drawn if its numbers moved; then the owner's
   sentence-level copy-edit pass over the user-facing prose —
   explicitly non-blocking for everything except itself, and
   the owner's final say on style (the standing docs policy).

*Risk register for the tail*: a cell green at default but red at
record is the two-scale design working — the cell's owner
reopens. In-flight rounds landing out of order re-derive each
other's pinned constants at their merges (the movement-annotation
discipline); the Bytes round's heap re-pin must come after every
other cost-moving item or its annotation is polluted.

### 17.3 Owned-red accounting

The board's red set is **empty**: `BOARD_EXPECTED_REDS`
(`meter::board`) is an in-flight triage buffer, every entry
carrying a live task, asserted empty at acceptance and empty on
the settled tree — red is reserved for untriaged contradictions
(the owner's 2026-07-28 ruling: an "accepted red" list would
mechanize normalization of deviance). Every contradiction the
campaign found resolves to exactly one of a cure (§3's ledger) or
a dated, owner-ratified declared model at its declaration site,
in `meter::board`'s ceilings module:

- **The ascend-cliff tick heap constant**
  (`ASCEND_CLIFF_TICK_HEAP_BYTES_PER_INPUT_BYTE`): the
  accumulator's zero-run certificates on a monotone climb occupy
  memory until consumed — honest Θ(input) work-state with a large
  constant on the one shape that defeats consumption; exponent
  stays at the global bound. Ratified 2026-07-28, conditional on
  the measured flat-constant profile.
- **The ascend-cliff `version_min_ticks` heap constant**
  (`ASCEND_CLIFF_MIN_TICKS_HEAP_BYTES_PER_INPUT_BYTE`): the
  anchor web holds one live reign record per simultaneously-open
  minimum, and the family holds Θ(k) open at once — the state
  that buys the exponent cure. Ratified 2026-07-28, same
  condition. A packed-frame representation is the candidate cure
  if one is ever warranted.
- **The mirror-wide render limb model**
  (`MIRROR_WIDE_RENDER_LIMB_EXPONENT_CEILING`,
  `MIRROR_WIDE_RENDER_LIMB_OPS_PER_RADIX_UNIT`): the render's
  summary merge on wide×deep trees is the documented
  `SuperlinearTime` class, judge-rostered red on the wall leg;
  the ceilings admit the measured class while a genuinely
  quadratic conversion still reads red, and the committed
  `render_merge_superlinearity_is_alive` pin reads red the day a
  render-merge cure lands, forcing the declaration's
  re-derivation in the same change. Ratified 2026-07-28. The
  anchor-web discipline remains the candidate cure, gated on the
  display class question (§17.2 item 5).
- **The capacity-chain peak** on the materialization pair
  (`capacity_chain_peak`'s declared model, band two-sided): the
  ratified stated-band residual of a non-derivable output (§12's
  presize finding).
- **The fold model and search allowance**
  (`FOLD_SCAN_BITS_PER_INPUT_BYTE_PER_LEVEL`,
  `INDEX_PROBE_SCAN_BITS`): the balanced reduction's own
  `O(D log k)` and the fold index's per-probe price, each with
  committed wrong-artifact tripwires (a quadratic fold or a
  regressed search reads red).

Each declared model is disclosed on the row face, banded so an
improved kernel trips the stale-model floor, and carried by the
declared-model bench riders (`BOARD_DECLARED_BENCH_RIDERS`) where
no designed pairing times its shape. The bench judge's standing
reds are §13's roster (the schoolbook tripwire and the display
pair); the display pair is the one open class question and is
owned by the final review.

### 17.5 Post-campaign docket (user directives)

- **The constants frontier** (`design/before-constants-frontier.md`,
  ideation of record): candidate directions for a future
  constants campaign, none committed; probe results land there as
  dated amendments. The word-scale subtree skip (popcount
  pending-counter delta over `idbits` and the skyline topology
  stream — a constants option no red owns, with the route-fold
  correctness seam its P4.2 sequencing decision names) belongs to
  that campaign.
- **The fold-vocabulary residue**: the fold-unification survey's
  optional cleanup row (its §4 row 6) stays open in that
  document; Phase C is measured-and-dropped by the survey's own
  criterion.
- **suanpan publication policy**: unpublished until a second
  consumer stabilizes the API; its amortization contract is
  subtle — reads mutate. The spare-buffer newtype question
  revisits if a second pooling consumer appears.
- **Stack-container decision: `SmallVec<[T; N]>` vs
  `Vec::with_capacity(N)` vs `Vec::new()`, measured, per site**
  (user directive 2026-07-25). The inline path saves one small
  call-scoped allocation but pays a per-access discriminant
  branch, a fatter struct, and a spill memcpy at exactly the deep
  input; pre-sizing at a defensive bound rounds into a larger
  allocator size class whose slack Vec does not adopt, while an
  organic `Vec::new()` on a shallow walk makes exactly one
  small-class allocation. "Capacity known in advance" splits into
  known-per-input (pre-size exactly) vs known-as-typical-bound
  (pre-sizing may lose); our walk stacks are almost all the
  latter. Discipline: every explicit stack from the P4 residual
  audit is implemented behind ONE type seam (a module-local alias
  or newtype, one line to swap); a measured phase under the final
  harness — benign AND deep families, both scales — benches all
  three contenders and decides per site; the losers' numbers ride
  the DECIDED entry. The choice is not judgment-neutral: shallow
  walks on smallvec are allocation-free and heap columns pin that
  — a Vec win on time re-pins those rows deliberately. Where a
  packed bit-stack (2 bits/level) suffices, neither applies and
  the bit-stack stays.
  - **DECIDED 2026-07-31 (adopt-68): plain `Vec::new()` at both
    parser-stack sites; the `smallvec` dependency retired from
    `before`.** The ratified A/B record ran the `stacks` bench's
    per-site arms against the shipped inline stacks: the id-parse
    stack *lost* as SmallVec (record geomean 0.782 for the Vec
    arm — Vec wins in microseconds on the deep cells against
    regressions of tens of nanoseconds on tiny spines), and the
    text stack read near-neutral (0.962), dissolved with it so
    the dependency goes entirely (dependency-dissolution over a
    cell-neutral hold). Heap envelope rows re-pinned deliberately
    where the Vec growth moved them, movements recorded at the
    parent. The per-site A/B seams, their `RUSTFLAGS` arms
    (`id_stack_vec`/`text_stack_vec`), and the `stacks` bench
    retired with the decision: the seam existed to price the
    choice, and the choice is made — re-opening it starts from
    this entry's numbers, not from standing machinery.
- **The envelope-harness unification** (the #39 census's deferred
  disposition): collapse the envelope harness shapes in
  `tests/meter.rs` into one five-column shape with per-column
  floor-or-NA — the board's totality mechanism applied to the
  gate suite; sibling of the standing harness-triplication item.
- **Defended keeps (2026-07-24 scaffolding sweep) — adjudicated
  once, not relitigated**: the limb/scan/segment/touch meters and
  their floors (domain-semantic; no external tool can produce
  them); the adversarial generators, envelope harnesses, and the
  ratchet convention; `tier2_size` and the compactness probes;
  `tools/memwatch` (no macOS cgroup equivalent); `tools/doclint`
  (measured non-subsumed by clippy); heap metering (`peak_alloc`
  + the one-global-allocator-per-binary plumbing; dhat cannot
  reset its high-water mark mid-process); κ and its tripwires
  (every sub-problem exists because review refuted a weaker
  criterion). Heap-column exact pins stay eyes-open under backend
  bumps.

### 17.10 Runbook conventions in force

- **Gate invocation**: `just gate` (codegen-running recipes route
  through `tools/memwatch`; per-process and swap limits default
  to the committed values, overridable per invocation);
  unqualified green before every commit — any failure anywhere
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
  substance improvements gated on the user (§17.2 item 7 is the
  slot).
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
  dirs make the bisect cheap). Acceptance criteria say "identical
  or better", never "byte-identical", except where any movement
  at all is a finding (owner ruling 2026-07-29).
- **Structure, not tallies** (owner rule, applied 2026-07-30):
  prose states the structure; any load-bearing count lives in a
  mechanically-enforced place the prose cites by name (the smoke
  suite's per-family expectations, the rosters, the pinned
  constants).
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
  freely deletable under disk pressure — but only by their
  owners: agents delete nothing outside their own worktree.
