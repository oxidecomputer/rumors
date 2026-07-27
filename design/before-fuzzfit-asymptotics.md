# The fuzz-fit asymptotics harness

Statement of the instrument. Code of record: `crates/before/fuzzfit`
(detached workspace: `guest/` + `harness/`); recipes `just fuzzfit-build`,
`just fuzzfit`, `just fuzzfit-calibrate`. The pinned bands, the judgment
constants, and the dated movement annotations live in
`harness/src/bands.rs`; this document is the instrument of record for
`before`'s instruction-count asymptotics (the campaign document,
`design/before-adversarial-resource-amplification.md`, cites it as such
from its metering-gate section).

## 1. What it guards, and what it adds

`before`'s asymptotic contract — every public operation amortized linear
in its denominated size (`design/before-adversarial-resource-amplification.md`
§6) — is enforced elsewhere by *chosen* adversarial families (the meter
board) over *chosen* currencies (limbs, scan bits, heap, segments). Two
structural blind spots remain by construction:

1. **Shapes nobody chose.** The board prices the shapes its designers
   thought of. A cost cliff on an unconsidered shape reads green there
   forever.
2. **Work escaping the metered currencies.** A counter prices what it
   counts. Machine-word work that allocates nothing, recurses nothing,
   scans no packed stream, and touches no `Base` arithmetic is invisible
   to every board column (the bench judge's time leg sees it, but only on
   the board's own shapes — blind spot 1 again).

This instrument closes both at once: *fuzzed* operation programs (shapes
drawn from a biased distribution, not a roster) measured in *executed wasm
instructions* (a total currency: every instruction counts, whatever it
does).

## 2. The mechanism

The guest crate compiles `before`'s public surface to
`wasm32-unknown-unknown` behind a C-ABI register machine (one export per
public operation; values live in a linear register file; bulk bytes cross
through a staging buffer). The harness executes every generated program
twice, in lockstep:

- **natively** (the mirror): computes each step's §6 denominator from the
  real operand sizes, predicts each step's outcome, and supplies expected
  bytes;
- **in the guest** under wasmtime fuel metering: fuel decrements per
  executed wasm instruction, so a reading is deterministic,
  host-independent, and byte-reproducible under any machine load.

The probe (`harness/src/bin/probe.rs`, re-run at each pin; the readings
below are the pin of record's): fuel for a fixed call sequence is
byte-identical across fresh in-process instances and across process
invocations; the per-call overhead baseline (`ff_nop`) is 2 fuel; guest
results byte-match the native mirror. The lockstep replay doubles as a
standing wasm-vs-native differential oracle: every live register's final
bytes must agree.

Wall time and hardware counters are never read. Fuel constants are an
artifact of the guest codegen (they differ from native constants); the
instrument fits and enforces *slopes*, and the committed guest profile
(release, `codegen-units = 1`, `panic = "abort"`) plus the pinned
toolchain make the constants reproducible. The toolchain match is
mechanical, not conventional: the harness's build script embeds the
building `rustc --version`, calibration writes it into the pin
(`bands::PINNED_RUSTC`), and the suite asserts the two agree — a
toolchain bump reads red until a deliberate re-pin. wasmtime, the fuel
schedule's other half, is pinned exactly by the workspace `Cargo.lock`.

## 3. Generators

One strategy per meter-board family — the archetype with randomized
dimensions and structural jitter, built **exclusively through public
operations** (seed, fork, tick, join, …), so every constructed value is
API-reachable by definition and the crate-doc safety rules hold by
construction inside each universe. The board's control variants ride as
parameters (`hifloor`, `plateau`, `tail_ticks = 1`). Three family kinds
sit on top of the roster:

- **Combination programs**: weighted random walks over the whole op
  vocabulary, operands drawn from everything constructed so far —
  the composition space between the named shapes.
- **Independent programs** (the two operand regimes; owner ruling, §9):
  every multi-operand operation is exercised both on *coupled* operands
  (one universe, valid together — every other family) and on operands
  from *separately seeded universes*, where the result is meaningless
  but the cost claim still binds — including cross-universe
  `Clock::join`, `Clock::from_parts` on mongrel halves, mixed-universe
  folds, and `Party::without` against foreign parties. Operations that
  reject such operands (`Party::join`, `Clock::join`/`sync` on overlap,
  `without` on emptiness) have the rejection arm measured as its own
  outcome, predicted by the mirror per case, never assumed.
- **Escalation programs** (`Family::Escalation`, low-weighted, its own
  budget): one universe grown far past the roster's fork cap with a
  snapshot-clock size ladder, so the pair, fold, and query rows sample
  every half-decade bucket decades past the rest of the roster — the
  fitted *slope*, not the band's width, carries the judgment there. A
  *cadence battery* rides the ladder: at each kept snapshot, the
  single-operand rows (send, recv, version ticks, part splits and
  reassembly, party forks/shares/differences) each sample that rung's
  operand size, and a *meet ladder* (each rung's version against the
  previous rung's) gives the widest thin-per-case row dense within-case
  mass. Codec duplicates (`encode` then `decode`, the API-reachable
  route to an aliased id inside one universe) place overlap so every
  rejection arm is priced at escalated size too, including one
  *maximally-deferred* overlap: one deep child's party rides into both
  accumulator lanes, so the two finished halves overlap in exactly one
  interval far down the id tree and the top-size queries and join
  rejection do detection work that scales with the operands.

Reach runs along **two axes**, because superlinearity can live in
either: operand *bytes* (the escalation spine: folds to ~166k bits,
coupled party joins to 7.7k bits over 13.4k samples, single-operand and
rejection rows to 9–21k bits) and fold *width* (the ScatterFold width
ladder: shuffled folds at doubling widths up to 1024 in every draw). A
degenerate fold reduction is quadratic in width at fixed operand size,
and byte reach cannot stand in for width reach — §8's width-degenerate
fold demonstration is the proof. §8's reachless-kernel demonstration is
the matching proof for the other axis: reach a kernel's arms never
sample is reach the instrument does not have, whatever the burner
self-test says.

Budgets (`strategies::BUDGET`, `strategies::ESCALATION_BUDGET`) cap
ops, ticks, forks, and fold width unconditionally in the builder,
bounding constructed size a priori and serving as the honesty bound for
composed cases. **Scope consequences**: the generators construct
operands exclusively through value-producing operations, never through
crafted codec input, so a `2^b`-wide leaf costs `2^b` ticks here — even
though `Version::decode` reaches one from `O(b)` crafted input bits.
The wide-magnitude regime (bigroot/hugeleaf at record scale,
decode-constructed or hand-built) deliberately remains the meter
board's territory. Likewise the staged bytes are always the canonical
encoding the immediately preceding encode/display step produced, so
`decode`/`FromStr` *rejection* paths are never measured here —
malformed-input cost is the decode fuzz targets' and the board's
territory. This instrument's envelope is the API-reachable region
within budget, paid for one operation at a time.

## 4. Denomination

Exactly §6's rules (as amended 2026-07-26), computed by the mirror from
real values: packed operand bits everywhere, except text I/O (packed +
text, output read from the actual result — `Rank`'s `Display` included),
output-dominated projection (`Version / Party`, `Clock::own_version`:
input + packed output) and balanced share splitting (`Party::forks(n)`:
input + n packed shares), and rank operations (value content
`bits(num) + exp`, proxied by the rank's `num/2^exp` rendering: the
numerator term is proportional and its constant folds into the
intercept, while `exp` enters as its digit count — logarithmically
compressed against the criterion. The compression only *under-counts*
the denominator, which reads as more fuel per denominated bit, so the
proxy can mask no superlinear growth).

## 5. Bands: calibration and enforcement

**Calibration** (`just fuzzfit-calibrate`) sweeps a deterministic corpus
(1536 programs, ~985k steps; proptest's deterministic runner + case-index
seeds; two sweeps are byte-identical) and fits, per *band key* —
kernel × outcome, so an operation's rejection arm (`ERR_OP`, predicted
per step by the mirror) is priced separately from its success path,
and a superlinear regression in either arm cannot hide under the
other's ceiling — `log₁₀ fuel` against `log₁₀ denom`:

- the **slope** over half-decade *bucket medians* above a 128-bit fit
  floor — per-step fuel is heteroscedastic (fast-path mass below the
  constant-overhead knee, amortization spikes above), and raw OLS read up
  to +0.4 slope on kernels whose per-shape medians are flat;
- **two widths**, priced separately because the residual cloud is
  one-sidedly heavy: the **ceiling** (`width_above`, max positive
  residual of all floored samples) carries the asymptotic claim, and the
  **floor** (`width_below`, max negative-residual magnitude) carries
  liveness. A single symmetric max-|residual| width would let the
  fast-path cloud price the ceiling — ±1.4..2.9 on the pair/fold/query
  rows, room a superlinear mechanism's whole in-range excess fits
  inside; the split ceilings read +0.1..0.9 over the same corpus.
  Bounded amortization spikes still land inside the committed band; only
  unbounded (asymptotic) departures escape;
- kernels spanning under a decade (or three buckets) classify **constant**
  (slope 0). This classification is honest only when the generators
  genuinely cannot grow the row — the two seed rows. Everywhere else a
  constant band means "span too narrow to fit," which enforces no slope
  at all — the trap §8's reachless-kernel demonstration exploits, and
  why the reach family's cadence battery exists. Every non-seed row fits
  over a real span; the O(1) rows read as fitted slope ≈ 0 with
  near-zero width (`into_parts` and `from_parts`), which is a *proof* of
  constancy, not an abstention.

The result is rewritten atomically into `harness/src/bands.rs` and
committed — reviewed like a snapshot, with a dated movement annotation in
the module doc. The fit is never recomputed in enforcement (refitting on
every run would mask drift). **Re-pin events**: a guest toolchain bump
(asserted mechanically, §2), a kernel change, a `before` public-API
addition (a new operation means a new kernel, a new op, and a new band —
the harness's kernel-roster test fails by name until the band is
pinned), a strategy change. Re-pin =
run the recipe, read the diff, date the annotation — and the recipe
re-prints the judgment constants' evidence (the shape allowance's and
the refit tolerance's observed maxima, and the floor margin's narrowest
floor-vs-nop gap) so their pins re-derive rather than persist on trust.

**Enforcement** (`just fuzzfit`, `tests/enforce.rs`) draws fresh programs
(48 cases; the calibration corpus is the big sweep, this is the sentry),
replays two *fixed* escalation programs besides (depth 1024 and the
family's 1792 depth cap, distinct seeds: the sentry's random draws pick
the reach family about once in 137 cases, so without the deterministic
replays a typical run never leaves the small-operand regime and the
at-scale bands have no standing exercise — §8's reachless-kernel
mechanism reads red through the mid-depth replay alone; the second
replay keeps the upper depth range and the reach proof off a single
(depth, seed) point), and judges two legs with different failure modes:

- the **point leg**: every measured step lands in its band key's band at
  its size, judged only at `denom ≥ min_denom`. Above-band is an
  asymptotic regression; below-band is a liveness flag; a band key
  without a band fails (totality). The two flags carry different claims
  and get different slack: the ceiling's margin stays tight (+0.2,
  allocator-history variance), while the floor's sits at 1.0 — a
  liveness threshold left inside the honest cheap tail's dispersion
  relocates that dispersion into flakiness (the derivation is at
  `bands::ENFORCE_MARGIN_BELOW`), and every pinned floor still clears a
  dead meter's nop-level reading with the slack subtracted (narrowest
  measured gap 0.14 decades, `ff_rank_cmp` at its 128-bit fit floor,
  widening with size; calibration re-derives the minimum on every
  re-pin).
- the **shape leg** (`curve.rs`): no band key's within-case bucket-median
  trend out-climbs its pinned slope by more than a measured allowance
  (+0.3; the corpus's observed healthy maximum re-derives on every
  re-pin and currently reads +0.006). One case is one family at one
  draw — a family-pure population — so the cross-family mixture tilt
  that moves pooled medians cannot occur there, and a rising
  within-case trend is a mechanism's own curvature: the leg that sees a
  regression tilting *into* a wide band with small point residuals. The
  fold rows are exempt (their honest law trends along the width axis by
  the documented bounded log factor; the point leg plus the width
  ladder own them).

Fuel determinism makes replay exact: a failure shrinks to a minimal
out-of-band shape and rides as a committed proptest seed.

Standing self-checks ride with the suite, each with its own committed
tripwire:

- **judgment tripwires** (`sanity.rs`): a quadratic reading must flag
  Above and a dead-meter reading Below against a synthetic band, before
  any fuzzing counts; the shape leg's synthetic tripwires live in
  `curve/tests.rs` (quadratic flags, noisy-linear passes, under-evidenced
  abstains).
- **meter liveness**: `ff_nop` fuel pinned in (0, 100).
- **detection-path liveness**: the guest ships a deliberately quadratic,
  `black_box`-pinned self-test burner (`ff_selftest_quadratic` — not a
  kernel; no strategy emits it, calibration never bands it), and the
  suite asserts it reads ABOVE a linear band anchored on its own
  small-input cost, and Below when stalled. This proves the
  wasm-execution → fuel-metering → judgment path can flag a quadratic at
  all; it says nothing about whether the *generators* place real
  kernels where a regression must flag — a reachless kernel rides
  green past a green burner (§8). Generator reach is accepted
  only by reconstruction demonstrations against the roster itself.
- **pin staleness** (the quantity-computable-two-ways convention): the
  suite refits the deterministic 256-program prefix of the calibration
  stream and, for every band key on the committed `REFIT_COVERAGE` list
  (generated at pin time; currently 48 of 49 — `ff_rank_checked_sub`'s
  rejection arm spans under a decade inside the prefix and
  classification-flips against its fitted pin, so calibration prints it
  as uncovered for the re-pinner instead of listing it), requires a
  prefix fit to exist, its constant/linear classification to match the
  pin's (a flip is the reach-regression tell — the generators stopped
  placing that key where its slope is measurable — and fails as a stale
  pin, never a silent skip), and its line to agree within a measured
  tolerance (0.7, above the observed 0.55 prefix-vs-corpus sampling
  dispersion; `fit::line_divergence`). Coverage decay, flips, and drift
  each fail by name. The comparator's own tripwire is committed
  (`fit/tests.rs`: a hand-perturbed pin reads back exactly its
  perturbation). This is a drift detector, not a mechanism catcher: §8's
  width-degenerate fold slips under its tolerance while the point leg
  flags it.
- **pin provenance**: the building toolchain must equal
  `bands::PINNED_RUSTC` (§2).

## 6. The pin of record (2026-07-26, at the merge onto the campaign line)

49 band keys: 44 kernels, of which five have sampled rejection arms
(`clock_join` and `clock_sync` on overlap, `party_join` on overlap,
`party_without` on emptiness, `rank_checked_sub` on underflow). The one
deliberately unpriced outcome is `meet_all([])`'s `None`:
production-reachable but structurally constant — no operand exists, so
there is nothing for a regression to scale with and nothing to
denominate a cost against (the generators' fold operands come from
constructed clock populations, never an empty range; `judge`'s totality
panic prices every outcome the generators do sample).

All 49 band keys read linear-or-flatter within every family above the
128-bit fit floor. The mechanical form of that claim is the shape
diagnostic: across every evidence-bearing (band key, case) pair in the
corpus, the maximum within-case slope excess over the pin is +0.006
(re-derived at each re-pin; `bin/diag`, the fit-free bucket-median
view, is the per-family ground truth). Pooled envelope slopes above 1.1
are composition, not mechanism:

- **Family-mixture composition** (the success rows in 1.14–1.24:
  `ff_version_cmp`/`concurrent` 1.15, `ff_version_decode` 1.17,
  `ff_version_rank` 1.16, `ff_version_min_ticks` 1.14,
  `ff_party_is_disjoint` 1.20, `ff_party_join` 1.24): families' per-bit
  cost levels differ severalfold, and the cheap families' mass sits in
  the small buckets, so the pooled envelope tilts along a line no
  single family follows. Pinning the envelope is correct for
  enforcement (per-step judgments hold under any mixture), and the
  within-case shape leg is the standing check that no lane's own trend
  rises.
- **Rejection-arm mixture** (`ff_clock_join [err]` 1.37,
  `ff_clock_sync [err]` 1.39, `ff_party_join [err]` 1.48,
  `ff_party_without [err]` 1.32): the same composition at smaller n —
  cross-universe rejections detect their overlap near the root and
  dominate the small buckets cheaply, while the escalation family's
  codec-duplicate and deferred-overlap rejections scan deep at the top.
- `ff_version_join_all` 1.14: `join_all`'s documented balanced
  binary-counter fold — every input passes through O(log n) joins, and
  the width ladder samples the factor across widths 8..1024; fold width
  is budget-capped, so the factor is bounded and the band prices it.
  `ff_version_meet_all` 1.06 is the same row under the ladder's
  economics: the cadence recv's shared events keep adjacent-rung meets
  from collapsing to a common floor immediately, so the envelope sits
  near linear.
- `ff_rank_display` 1.10: the schoolbook decimal radix conversion the
  meter board's text-I/O legs document (digits × limbs), read against
  the text denominator within this instrument's rank reach (≤7k bits);
  the board owns the record-scale conversion regime. The other rank
  rows sit sublinear (0.41–0.57) on the pooled unequal-rank battery:
  `checked_sub`'s underflow arm pins flat (−0.13, the ordering
  pre-check's early exit) and its success band carries the
  alignment-subtract mass beside the equal-operand fast path.
- `ff_version_meet` 1.04 with a +0.31 ceiling: the escalation meet
  ladder's dense adjacent-rung meets dominate the row and hold the
  band tight — this is the row §8's residual-risk analysis watches as
  the widest thin-per-case surface, and the ladder is what keeps it
  neither wide nor thin.

Movement between pins is recorded where the constants live: the dated
movement annotation in `harness/src/bands.rs`'s module doc, one entry
per re-pin, split by mechanism.

## 7. Platform note

`stacker` cannot grow a wasm stack (its fallback runs the callback in
place), so the guest's deep traversals consume real wasm stack; the
harness raises wasmtime's ceiling to 48 MiB and the budgets cap depth far
below it. Native depth guarantees are the depth-100k stack-safety test's
business, not this instrument's.

## 8. Adequacy: demonstrations by construction, and the residual risk

The instrument's catching power is accepted by construction — a
known-bad mechanism built, run against the committed criteria, and
shown red — never by argument. Five mechanism genres are accepted this
way; each acceptance is a dated record in git history at its pin
commit, and each forced a committed defense that stands in the tree:

- **The width-degenerate fold** (`Version::join_all` as the left fold
  its own comment warns about — quadratic in fold *width* at fixed
  operand bytes; associativity makes the differential oracle blind to
  it, and byte reach cannot amplify it): reads red through the point
  leg on the fold rows. Forced the width-reach axis — `max_fold` 1024
  and the shuffled doubling ladder in every ScatterFold draw — and a
  permanent replay seed (`harness/tests/enforce.proptest-regressions`)
  that passes on the balanced fold and meets any width-degenerate fold
  first.
- **The reachless-kernel quadratic** (`black_box`-pinned n² inside
  `Clock::recv` while that row spans under a decade — a
  constant-classified band enforces no slope): reads red
  deterministically through the mid-depth escalation replay. Forced
  the escalation cadence battery, real fitted spans on every non-seed
  row, and the two fixed replays as committed tests — the 48 random
  sentry draws alone stay green under this mechanism, which is why
  at-scale detection is a committed test and not a probability.
- **The rejection-arm quadratic** (`black_box`-pinned n² on
  `Party::join`'s `Err` branch, priced inside a success ceiling that
  sits decades above the rejections' cheap cloud): reads red three
  ways — the escalation replay, the sentry (whose shrunk
  cross-universe rejection shape is a committed seed), and the
  staleness cross-check. Forced outcome-keyed bands (kernel × outcome)
  and escalated rejection construction (codec-duplicate overlap along
  the ladder; the deferred-overlap poison between the finished
  halves). Replaying the committed seed on healthy code also priced
  the floor margin: liveness needs decade-scale sensitivity only, so
  `ENFORCE_MARGIN_BELOW` is 1.0, inside the measured gap between
  honest cheap readings and dead-meter readings (§5).
- **The in-ceiling mild superlinearity** (n²/64 `black_box` work in
  `Version::meet`, sized inside a wide thin-per-case ceiling): reads
  red against the meet-ladder pin — the ladder's dense within-case
  mass both feeds the shape leg and holds the pinned ceiling tight
  (+0.31), so the headroom the mechanism needs does not exist.
- **The unpriced-arm quadratic** (`black_box`-pinned n² on
  `Rank::checked_sub`'s underflow arm, which an equal-operand rank
  pool never fires): reads red through the sentry, met first by a
  committed replay seed. Forced the pooled unequal-rank battery —
  every distance/lag output joins the pool and `checked_sub` issues in
  both operand orders, so any unequal pair fires the underflow arm in
  exactly one order and the arm pins its own band — plus `judge`'s
  totality panic, which fails any sampled outcome with no band by
  name.

**Residual risk, bounded honestly.** Within the instrument's reach, a
smooth superlinear mechanism with a small enough constant escapes both
legs whenever its whole in-reach rise stays inside
`width_above + 0.2` (point leg) and its within-case tilt stays inside
`pinned slope + 0.3` (shape leg, where evidence exists). Per row that
bounds the escaping exponent at `slope + min(0.3,
(width_above + 0.2)/span_decades)`, shape-exempt rows uncapped;
against the pin of record: ~1.28 on `version_meet`
(slope 1.04, (0.31 + 0.2)/2.08 = 0.24), ~1.23 on `version_join`
(slope 1.01, (0.24 + 0.2)/2.01 = 0.22), ~1.17 on `version_project`
(slope 0.87, (0.58 + 0.2)/2.52 = 0.31, capped at the shape leg's 0.3),
and ~1.42 on the shape-exempt `meet_all` (slope 1.06,
(0.89 + 0.2)/2.95 = 0.37, uncapped — no within-case leg backstops the
fold rows). Tightening further is architectural — per-family bands or
per-case baselines, and a shape allowance re-derived from fresh-draw
dispersion rather than the corpus maximum (+0.006 today, 50x under the
0.3 allowance) — and deliberately out of scope. The bound is a
property of any reach-limited envelope instrument: it prices
mechanisms by their in-reach excess, and a mechanism whose in-reach
excess is sub-threshold is, within the priced envelope, behaviorally
the envelope.

**The meter is inside the threat model, and its degradation bound is
measured.** A partially dead meter — fuel uniformly undercounted by a
factor k — shifts every reading down `log₁₀ k` without tilting any
slope, so neither leg's shape criteria see it; the floor's 1.0-decade
liveness slack absorbs the shift, and what bounds it is the staleness
cross-check's line comparison against the pinned intercepts. Measured
against the pin of record (a uniform ÷k probe in the fuel readout,
judged by the deterministic detectors — the prefix refit and both
escalation replays): k = 2 hides (every deterministic detector reads
green); k = 3 is caught (`ff_version_meet_all`'s prefix refit diverges
0.820 against the 0.7 tolerance). The accepted residual until the next
re-pin: meter degradation under ~3× — equivalently, up to ~0.48
decades of effective extra ceiling generosity — can hide, and a real
regression must exceed `width_above + 0.2 + log₁₀ k` to flag through
it. No new machinery guards this, by the dissolution counterweight's
ruling (§9): a dedicated meter-calibration instrument would exist
mostly to defend itself, while the staleness check already bounds the
exposure and re-derives its evidence at every re-pin.

**Standing (continuous) demonstrations**, run by every suite
invocation:

- `ff_selftest_quadratic`: the guest's black_box-pinned quadratic burner
  must read ABOVE a linear band (and a stalled reading Below) on every
  suite run — the detection path's liveness, from wasm execution through
  fuel metering to the judge.
- The fixed escalation replays (`tests/enforce.rs`; depth 1024 and the
  1792 depth cap, distinct seeds): the reach regime's bands —
  single-operand rows, rejection arms, the deferred-overlap scans, the
  meet ladder — get exercised deterministically on every suite run,
  across the family's whole depth range.
- `curve/tests.rs` and `fit/tests.rs`: the shape leg's and the staleness
  comparator's synthetic tripwires (quadratic flags / flat passes /
  under-evidenced abstains; a perturbed pin reads back its
  perturbation).
- The staleness cross-check's committed `REFIT_COVERAGE` list: every
  band key covered at pin time must keep its prefix fit, its
  classification, and its line — reach decay (the genre the
  reachless-kernel demonstration exploits) fails by name instead of
  shrinking a count.
- The committed proptest seeds (`enforce.proptest-regressions`): every
  shape a red run ever shrank replays in-band on healthy code, forever.

## 9. Decision record (dated; owner rulings and major design decisions)

- **2026-07-26 (owner): two operand regimes.** Every multi-operand
  operation is exercised under both coupled and independent operand
  scaling — one universe, valid by construction; and separately seeded
  universes, where the result is meaningless but the cost claim still
  binds. Rejection arms are legitimate outcomes, measured and priced,
  never filtered.
- **2026-07-26: bands are keyed kernel × outcome.** A rejection arm can
  legitimately undercut its success line by decades; pooling the two
  lets a superlinear regression in either hide under the other's
  ceiling. (Accepted by the rejection-arm reconstruction, §8.)
- **2026-07-26: reach runs on two axes** — operand bytes (the
  escalation family: spine, size ladder, cadence battery, meet ladder,
  escalated rejections, two fixed replays) and fold width (the
  ScatterFold doubling ladder to 1024). (Accepted by the
  width-degenerate fold and reachless-kernel reconstructions, §8.)
- **2026-07-26: asymmetric enforcement margins.** The ceiling's margin
  is 0.2 (allocator-history variance; the regression claim stays
  tight); the floor's is 1.0 (liveness needs decade-scale sensitivity
  only, and a floor threshold inside the honest cheap tail's dispersion
  is flakiness, not sensitivity). Derivations at `bands::ENFORCE_MARGIN`
  and `bands::ENFORCE_MARGIN_BELOW`.
- **2026-07-26 (owner, exclusion of record): `meet_all([])` stays
  unpriced.** Production-reachable but structurally constant — no
  operand exists, so there is nothing for a regression to scale with
  and nothing to denominate a cost against.
- **2026-07-26 (the dissolution counterweight): the sub-3× uniform
  meter-degradation residual is accepted without new machinery.** The
  staleness cross-check bounds the exposure and re-derives its evidence
  at every re-pin; a dedicated meter-calibration instrument would exist
  mostly to defend itself. (The bound's measurement is §8's.)
- **2026-07-26: judgment constants re-derive, never persist on trust.**
  Calibration re-prints the shape allowance's, the refit tolerance's,
  and the floor margin's observed evidence on every re-pin, so each
  re-pinner re-derives the constants' standing instead of inheriting
  it.
- **2026-07-26 (the merge onto the campaign line): re-pin absorbing the
  main line's kernel work.** The movement annotation in
  `harness/src/bands.rs` records the mechanisms; sample counts per band
  key are unchanged (the deterministic corpus replayed identically), so
  every movement is guest fuel.
- **2026-07-27: re-pin absorbing the explicit-stack conversions and the
  skyline diff sweep.** The prefix-refit staleness check caught the
  stale pin (`ff_party_without` diverged 1.292 against the 0.7
  tolerance). Movement by mechanism, with owning commits, is annotated
  in `harness/src/bands.rs`; sample counts and denominator spans per
  band key are unchanged, so every movement is guest fuel.
- **2026-07-27 (#54): re-pin absorbing the diff sweep's covered-block
  early exits.** Only `party_without`'s two arms move (every other
  band key replays the previous pin byte-for-byte); the movement
  annotation in `harness/src/bands.rs` carries the numbers. The
  emptiness arm's envelope joins the rejection-mixture genre (pooled
  slope 1.41 while the arm's top-decade per-bit medians hold flat at
  ~430 fuel/bit): a pooled rejection envelope tilts whenever an early
  exit makes part of the rejection surface cheap, so the per-family
  medians (`bin/diag`) and the within-case shape leg, never the pooled
  slope, carry the linearity claim.
- **2026-07-27 (owner): the instrument joins the commit gate.**
  `just gate` runs `fuzzfit-build` then `fuzzfit` (measured basis:
  8.6 s warm build + 59.9 s run). The staleness red above sat outside
  every tier for a day because nothing standing executed the harness;
  in the gate, a kernel change that moves fuel fails the commit that
  carries it, and the deliberate path is a `fuzzfit-calibrate` re-pin
  riding the same commit.
