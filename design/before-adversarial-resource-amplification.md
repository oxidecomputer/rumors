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
| nested-full-sibling | `(x,1)` repeated down a spine × matching event spine | fill's pre-scan/lookahead terms (#33's adversary; lands with the fill red pin) |

## 3. Findings ledger

All cured; each family above witnesses at least one. Mechanism
detail and measurements are in git history; the cures are pinned by
the enforced envelopes and board cells named.

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
  id's depth — not wire-suppliable by a hostile peer, so a pricing
  obligation, not an exploit): the linearization is committed
  work, red-pinned first (§17.2, the #33 item).

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

## 13. The metering gate

The board (`before::meter::board`, `just amp-board`, runner
`examples/amp_board.rs`): a red-green matrix over the entire
public operation surface × §2's families — **205 cells at this
tip**, membership pinned by the smoke test — judged at two scales
(default; `board::RECORD_SCALE` = ×4, `just amp-board-record`)
from deterministic meters only: peak heap, grown stacker segments,
limb ops, scanned/written bits. **The board reads no clock**: its
entire rendered output is byte-identical at a given scale under
any machine load, no stripping, no carve-outs [measured — under a
sustained parallel-build load generator]. Wall time is judged
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
IDs mirror board cells by construction — 207 judged cells: the
205 board cells plus the wide-display pair): fits each cell's
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
empty at C3; the schoolbook expectation is permanent.** Between
C2 and C3 every judge run fails on the fifteen realized greens BY
DESIGN — that failure is C3's realization evidence, banked
verbatim at the flip (e 0.94–1.00 fitted on all fifteen).

**Numbers of record at this tip** [measured 2026-07-25, the
bench-coverage integration record; dev profile, limb+scan meters
lit]: board **188 green / 17 red at the default scale; 187 / 18
at ×4** over 205 cells. The judge's last honest reading, at the
flip commit over 202 cells: **157 green / 3 red / 42 sub-floor**,
exit 1 on exactly the fifteen banked realization violations; the
three reds all rostered-expected. Workspace sweep at C0:
1183/1183, roster retired, unqualified green since.

**Acceptance (the campaign's, re-denominated 2026-07-24):
all-green means the board green on counters and floors at BOTH
scales (three identical runs each) AND the bench judge
roster-satisfied at both scales in both modes** — at P5.5 with
the bigroot set emptied and only the permanent text expectations
remaining.

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
- **Post-flip fix rounds** (2026-07-25, in flight — §17.2).

Remaining plan: **C3** (§17.2) → **P4.2** residual audit →
**P5.1–P5.5** closeout, with the fill linearization (#33)
sequenced after its red pin and the boolean-skyline decision
(#24, the user's) after C3.

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
problem's own optimum, stated and priced. The fill/tick kernel as
first built — O(|ev| × local-id-depth) — is below the bar; its
linearization is committed (#33).

## 17. Work items of record

### 17.2 Open items, with acceptance contracts

**In flight (2026-07-25): the post-flip fix round.** Two of its
commits are landed (`606e8f54` ghosts + false rustdoc;
`0e52d550` tautologies relabeled, real witnesses restored — the
recursive oracle is the fill differential's witness, deep values
in closed form); the remainder is uncommitted at this writing:
default-feature warning gating + the gate hole (clippy is
--all-features only; the features recipe gains warnings
enforcement), the D2 window fast-path restoration on the flipped
hot paths (sweep/validate/text/literal call the per-bit loop
directly — one line per site routes through `read_int`; board and
meter must read unchanged, scan widths identical by
construction), the `IdLeafCursor` scan double-record disposition
(fix-with-remeasure preferred; TODO-recalibrate fallback per the
max_depth precedent), `packed_grow` rename, the fuzz-seed
consumption fix (the corpus is gated but never passed to
libFuzzer: the `fuzz` recipe gains the seeds dir per target, the
README sentence corrected), the stray-seed-DIRECTORY gate hole,
the fuzz framing cross-pointers, fill's cost-doc restatement to
the id-depth bound, the tick/grow empty-id `# Panics` sentences,
the stale re-seeded key hex in `gossip_snapshot.rs`, and a
non-degenerate skyline payload for the bookmark-v2 pin.
*Acceptance*: each finding addressed or explicitly disputed; the
window restoration's no-movement proof; one full gate.

**C3 — P3.10: realization verification, re-pins, and the
before/after table.**
*What*:
- Board re-run at both scales, three identical runs each; every
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
- **The §17.3 reconciliation** (open discrepancy, marked below).
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

**#33 — the fill/tick linearization (committed, user ruling; the
hot path — every tick calls fill).**
Step 1, the RED PIN (instruments before cures): verify the
probe cursors meter (the right-full lookahead's topology skip and
the `min_fill` pre-scans are "reader untouched" — if their reads
do not record scan bits, meter them first, else the pin is
vacuous); land the nested-full-sibling family (§2) as generators
+ board cells for `version_tick`/`clock_tick` with a LINEAR
envelope (ceiling and constant derived from the two-pass design's
budget; liveness floor above the linear walk's count on this
shape); the current kernel must read RED on those cells at both
scales — committed with the measured exponent; the same family's
judge cells rostered expected-red owned by this item. The family
also closes the fill differentials' deep-regime coverage gap
(deep drift-stack, multi-level raise cascades, deep × wide).
Step 2, the cure, two independent terms: (a) defer the
right-full decision to the cursor's arrival at the right child
(arm C's raise value is the max of the left event range, which
the consuming walk computes in-pass) — the whole-left-subtree
topology skip becomes an O(1) peek; (b) one backward pass
computes every node's min-fill bottom-up (memoized, each
position once), the forward pass emits with all raise values in
hand — the two-pass/parent-close genre.
*Acceptance*: the pinned cells flip GREEN at both scales; the
roster entry leaves with the measured linear exponent recorded;
byte-identity against the current kernel via the existing dual
oracles + the new family; fill's cost doc re-derives to O(n+m).
No green, no merge. *Deps*: the fix round; sequenced before or
with C3 at the coordinator's choice (the red pin must precede the
rewrite in any ordering).

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
and board ceiling tightened to final constants at record scale,
three identical runs; `ID_WITHOUT`'s final ratchet (the one row
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

### 17.3 Owned-red accounting (current; over 205 cells)

Default scale, 17 red: ten κ-owned text cells (the κ/C3
re-derivation); two fold marginals (`version_join_all × scatter`,
`party_join_all × scatter` — the balanced reduction's n·log n
against flat ceilings; cure = the C2-adjacent n-cursor merge over
the skyline kernels); two fold benign controls
(`version_join_all × benign` scale-1-only after the flip cut its
constant ~3×; `party_join_all × benign` — same genre, party folds
never routed through the version kernels); the three
newly-κ-genre cells pending C3's adjudication. At ×4, 18 red: as
above with `version_join_all × benign` green (its exponent falls
with scale — the log factor shrinking, the n·log n signature) and
`rank_sum × benign` + `rank_pair_ops × benign` red on the
judgment-layer artifact genre (C3 decides the criterion arm).

**Marked open reconciliation (the coordinator's C3-entry
amendment; do not resolve by transcription)**: the flip's landed
entry categorized its 15 default reds as 10 text + 2 fold
marginals + 3 κ-genre, but the integration record separately
names `version_decode`/`version_rank`/`clock_decode` × cliff as
floor-liveness trips (the cliff floor re-derivation item) —
three cells that do not obviously sit inside those categories.
Reconcile from a fresh board reading at C3: enumerate the actual
red list, assign every cell exactly one owner, and restate the
sums so they close (N owned + M green = 205 at each scale).

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
