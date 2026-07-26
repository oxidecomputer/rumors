# The fuzz-fit asymptotics harness

Statement of the instrument (2026-07-26). Code of record:
`crates/before/fuzzfit` (detached workspace: `guest/` + `harness/`);
recipes `just fuzzfit-build`, `just fuzzfit`, `just fuzzfit-calibrate`.

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

The probe of record (2026-07-26, `harness/src/bin/probe.rs`): fuel for a
fixed call sequence is byte-identical across fresh in-process instances
and across process invocations; the per-call overhead baseline (`ff_nop`)
is 2 fuel; guest results byte-match the native mirror. The lockstep replay
doubles as a standing wasm-vs-native differential oracle: every live
register's final bytes must agree.

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
- **Independent programs** (two operand regimes, per the project owner's
  direction 2026-07-26): every multi-operand operation is exercised both
  on *coupled* operands (one universe, valid together — every other
  family) and on operands from *separately seeded universes*, where the
  result is meaningless but the cost claim still binds — including
  cross-universe `Clock::join`, `Clock::from_parts` on mongrel halves,
  mixed-universe folds, and `Party::without` against foreign parties.
  Operations that reject such operands (`Party::join`,
  `Clock::join`/`sync` on overlap, `without` on emptiness) have the
  rejection arm measured as its own outcome, predicted by the mirror per
  case, never assumed.
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
and byte reach cannot stand in for width reach — §8's first
demonstration attempt is the proof. §8's round-2 demonstrations are the
matching proof for the other axis: reach a kernel's arms never sample
is reach the instrument does not have, whatever the burner self-test
says.

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
(1536 programs, ~979k steps; proptest's deterministic runner + case-index
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
  liveness. A single symmetric max-|residual| width let the fast-path
  cloud price the ceiling — ±1.4..2.8 on the pair/fold/query rows, room
  a superlinear mechanism's whole in-range excess fit inside; the split
  ceilings read +0.1..0.9 over the same corpus. Bounded amortization
  spikes still land inside the committed band; only unbounded
  (asymptotic) departures escape;
- kernels spanning under a decade (or three buckets) classify **constant**
  (slope 0). This classification is honest only when the generators
  genuinely cannot grow the row — the two seed rows. Everywhere else a
  constant band means "span too narrow to fit," which enforces no slope
  at all: §8's round-2 demonstrations built a quadratic that rode green
  behind exactly such a row, which is why the reach family's cadence
  battery exists and every non-seed row now fits over a real span (the
  O(1) rows read as fitted slope ≈ 0 with near-zero width — `into_parts`
  and `from_parts` — which is a *proof* of constancy, not an abstention).

The result is rewritten atomically into `harness/src/bands.rs` and
committed — reviewed like a snapshot, with a dated movement annotation in
the module doc. The fit is never recomputed in enforcement (refitting on
every run would mask drift). **Re-pin events**: a guest toolchain bump
(asserted mechanically, §2), a kernel change, a strategy change. Re-pin =
run the recipe, read the diff, date the annotation — and the recipe
re-prints the judgment constants' evidence (the shape allowance's and the
refit tolerance's observed maxima) so their pins re-derive rather than
persist on trust.

**Enforcement** (`just fuzzfit`, `tests/enforce.rs`) draws fresh programs
(48 cases; the calibration corpus is the big sweep, this is the sentry),
replays one *fixed* escalation program besides (depth 1024, fixed seed:
the sentry's random draws pick the reach family about once in 137 cases,
so without the deterministic replay a typical run never leaves the
small-operand regime and the at-scale bands have no standing exercise —
§8's M1 demonstration reads red through this replay alone), and judges
two legs with different failure modes:

- the **point leg**: every measured step lands in its band key's band at
  its size, judged only at `denom ≥ min_denom`. Above-band is an
  asymptotic regression; below-band is a liveness flag; a band key
  without a band fails (totality). The two flags carry different claims
  and get different slack: the ceiling's margin stays tight (+0.2,
  allocator-history variance), while the floor's sits at 1.0 — a dead
  meter reads decades below any honest floor, and a liveness threshold
  left inside the honest cheap tail's dispersion (a committed seed's
  `join_all` step reads 0.29 below the corpus floor width) relocates
  that dispersion into flakiness.
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
  kernels where a regression must flag — §8's round-2 demonstrations
  rode green past a green burner. Generator reach is accepted only by
  reconstruction demonstrations against the roster itself.
- **pin staleness** (the quantity-computable-two-ways convention): the
  suite refits the deterministic 256-program prefix of the calibration
  stream and, for every band key on the committed `REFIT_COVERAGE` list
  (generated at pin time; currently all 48), requires a prefix fit to
  exist, its constant/linear classification to match the pin's (a flip
  is the reach-regression tell — the generators stopped placing that
  key where its slope is measurable — and fails as a stale pin, never
  a silent skip), and its line to agree within a measured tolerance
  (0.7, above the observed 0.55 prefix-vs-corpus sampling dispersion;
  `fit::line_divergence`). Coverage decay, flips, and drift each fail
  by name. The comparator's own tripwire is committed (`fit/tests.rs`:
  a hand-perturbed pin reads back exactly its perturbation). This is a
  drift detector, not a mechanism catcher: §8 records that the join_all
  mutant slipped under its tolerance while the point leg flagged it.
- **pin provenance**: the building toolchain must equal
  `bands::PINNED_RUSTC` (§2).

## 6. Findings of the pin of record (2026-07-26)

The owner's expectation was confirmation, and confirmation is what the
instrument produced — all 48 band keys (44 kernels, four sampled
rejection arms) read linear-or-flatter within every family above the
floor. The mechanical form of that claim is the shape diagnostic:
across every evidence-bearing (band key, case) pair in the corpus, the
maximum within-case slope excess over the pin is +0.006. The pooled
envelope slopes above 1.1 — the success rows in 1.10–1.24 and the
rejection arms in 1.32–1.48 — are ground-truthed (`bin/diag`, the
fit-free bucket-median view) to known mechanisms:

- **Family-mixture composition** (the dominant genre: `ff_clock_tick`
  1.15, `ff_version_cmp`/`concurrent` 1.15, `ff_version_decode` 1.16,
  `ff_version_rank` 1.15, `ff_version_min_ticks` 1.14,
  `ff_version_lag` 1.09, `ff_party_join` 1.24,
  `ff_party_is_disjoint` 1.21): families' per-bit cost levels differ
  severalfold, and the cheap families' mass sits in the small buckets,
  so the pooled envelope tilts along a line no single family follows.
  Representative medians: `party_is_disjoint` at 8–25 fuel/bit on the
  scatter populations against ~80 flat on the structured shapes;
  `clock_tick` at ~570–750 on harmonic/big-root mass against ~1500 flat
  on dense spines; `version_cmp` falling within every family (174→69 on
  Combination, 252→112 on RevealComb, flat 246 at 10⁴ bits on
  Escalation). Pinning the envelope is correct for enforcement
  (per-step judgments hold under any mixture), and the within-case
  shape leg is the standing check that no lane's own trend rises.
- `ff_clock_tick`'s Harmonic lane is the one mild riser: 751→858
  fuel/bit across its top half-decade (~+0.11 local), bounded and
  inside the shape allowance; DenseSpine (1472→1494), BigRoot
  (1598→1649), and NestedFull (1235→1178, falling) are flat.
- `ff_party_covers` 1.07: same mixture genre, re-measured on this
  corpus. The reach family put flat ~79-fuel/bit samples at 10³·⁵–10⁴
  bits (DenseSpine's lane is flat ~80 across two decades), which pulled
  the pooled envelope down from the 1.18 the pre-reach corpus read;
  what remains is the cross-universe mixes' cheap mass (~36–58
  fuel/bit) under-pricing the small buckets. No lane rises.
- `ff_version_meet` 1.03 with a +0.30 ceiling: the escalation meet
  ladder's dense adjacent-rung meets now dominate the row, collapsing
  what was a +0.87-ceiling thin cloud onto a tight near-linear law —
  the row §8's residual-risk analysis names as the widest thin-per-case
  surface is now the opposite.
- **Rejection-arm mixture** (`ff_clock_join [err]` 1.37,
  `ff_clock_sync [err]` 1.39, `ff_party_join [err]` 1.48,
  `ff_party_without [err]` 1.32): the same composition at smaller n —
  cross-universe rejections detect their overlap near the root and
  dominate the small buckets cheaply, while the escalation family's
  codec-duplicate and deferred-overlap rejections scan deep at the top
  (per-lane medians are flat per bit: the duplicate `clock_join`
  rejection reads ~42 fuel/bit across two decades).
- `ff_version_join_all` 1.14: `join_all`'s documented balanced
  binary-counter fold — every input passes through O(log n) joins, and
  the width ladder samples the factor across widths 8..1024 (healthy
  ladder medians 2906→5897 fuel/bit); fold width is budget-capped, so
  the factor is bounded and the band prices it. `ff_version_meet_all`
  1.05 is the same row under the ladder's economics: meets of
  snapshots that share the cadence recv's events no longer collapse to
  a common floor immediately, so the envelope sits near linear.
- `ff_rank_display` 1.12: the schoolbook decimal radix conversion the
  meter board's text-I/O legs document (digits × limbs), read against
  the text denominator within this instrument's rank reach (≤7k bits);
  the board owns the record-scale conversion regime.
- One real catch during bring-up, of the harness itself: the register
  file's `Vec` doubling landed inside a measured window and read as an
  above-band flag on `ff_party_seed`. Fixed by pre-reserving at
  instantiation; the shrunken seed is committed and replays against the
  fix. The instrument's first blood was drawn on its own bookkeeping —
  the enforcement leg works.

## 7. Platform note

`stacker` cannot grow a wasm stack (its fallback runs the callback in
place), so the guest's deep traversals consume real wasm stack; the
harness raises wasmtime's ceiling to 48 MiB and the budgets cap depth far
below it. Native depth guarantees are the depth-100k stack-safety test's
business, not this instrument's.

## 8. Demonstrations ledger

Reconstruction demonstrations: the instrument's catching power is
accepted by construction — a known-bad mechanism built, run against the
committed criteria, and shown red — never by argument. Each entry
records the mechanism, the commands, and the readings.

### 2026-07-26 — the left-fold quadratic fold (round-1 review acceptance)

The mechanism: `Version::join_all`'s balanced binary-counter reduction
replaced by the left fold its own comment warns about (`iter.fold(new,
|acc, v| acc | &v)`) — quadratic scan work on populations whose
accumulator never coalesces. Real work, so no `black_box` is needed
(synthetic arithmetic mutants strength-reduce; see the adequacy burner
for that arm). The differential oracle is silent on it by design:
associativity makes both groupings value-identical.

**Attempt 1 read green, and the failure is the entry's finding.** With
folds capped at 64 operands, the mutant's whole in-range excess is
bounded near `n / (2 log₂ n)`: measured mutant/healthy top-bucket
median ratio was 3.5x (+0.55 decades) at width 256 — inside even the
split ceiling once the sentry's 48 draws mostly missed wide folds. Two
lessons, both structural: this mutant is quadratic in fold *width*, not
operand bytes, so byte-size reach (the escalation spine) cannot amplify
it; and a reach axis the sentry samples rarely is a reach axis the
instrument does not have. The staleness cross-check also stayed green
under the mutant (its prefix-refit divergence sat under the 0.7
tolerance) — it is a calibration-drift detector, not a mechanism
catcher, and is documented as such.

The fix: `max_fold` 64→1024, ScatterFold populations to 1024, and a
doubling *width ladder* of shuffled folds in every ScatterFold draw, so
the fold rows are sampled along the width axis in every case. Measured
mutant/healthy ladder medians after the fix (diag, `ScatterFold`
filter): 1.34x at 10² bits rising to 10.2x (+1.01 decades) at 10⁴·⁵,
against a re-pinned join_all ceiling of +0.350 (+0.2 margin).

**Attempt 2, against the re-pinned bands** (`cargo nextest run -p
fuzzfit-harness --cargo-profile release -E 'binary(enforce)'` with the
mutant applied and the guest rebuilt):

> ABOVE BAND (asymptotic regression): ff_version_join_all at 765 bits
> consumed 5183049 fuel; the pinned law predicts ~10^6.163
> +0.350/-2.110

(the marginal-looking excess is the *shrunk minimal* case — proptest
shrank to the smallest failing fold; pre-shrink cases sat decades
over). Reverting the mutant and rebuilding: the full suite reads green,
15/15, with the shrunk shape committed as a permanent replay seed
(`harness/tests/enforce.proptest-regressions`) that passes on the
balanced fold and meets any future width-degenerate fold first.

### 2026-07-26 — the reachless-kernel quadratic (round-2 review acceptance)

The round-2 review's M1: a `black_box`-pinned quadratic (n²/32 in
encoded bits) inside `Clock::recv`. Seven kernels then had constant
bands with sub-decade spans — "span too narrow to fit," enforcing no
slope — and the mutant rode green through all fifteen tests: reach a
kernel's rows never sample is reach the instrument does not have,
whatever the burner self-test proves about the judgment path.

The fix: the escalation cadence battery (§3) routes escalated operands
through every single-operand row, the pin re-keys and refits every row
over real spans, and one fixed escalation program replays in every
suite run. Re-constructed against the new pin, the mutant reads red
through that deterministic replay:

> ABOVE BAND (asymptotic regression): ff_clock_recv at 10698 bits
> consumed 54983245 fuel; the pinned law predicts ~10^7.218
> +0.314/-0.467

The 48 random sentry draws alone stayed green under the mutant — the
deterministic replay, not the draws, carries the at-scale detection,
which is why it is a committed test and not a probability.

### 2026-07-26 — the rejection-arm quadratic (round-2 review acceptance)

The round-2 review's M2: a `black_box`-pinned quadratic (n²/16 in
encoded bits) on `Party::join`'s `Err` branch. Rejection outcomes were
then priced inside the success bands, whose ceilings sit decades above
the rejections' cheap cloud — the mutant rode green while the
cross-universe lane's cost visibly rose.

The fix: the band key is kernel × outcome (§5), and the escalation
family constructs rejections whose work scales with operand size
(codec-duplicate overlap along the ladder; the deferred-overlap poison
between the finished halves). Re-constructed against the outcome-keyed
pin, the mutant reads red three ways at once — the deterministic
replay:

> ABOVE BAND (asymptotic regression): ff_party_join [err] at 588 bits
> consumed 308650 fuel; the pinned law predicts ~10^4.509 +0.455/-0.865

plus the random sentry itself (which shrank a minimal cross-universe
rejection shape, committed as a permanent replay seed) and the
staleness cross-check (the rejection arm's refit line left its
tolerance). Reverting the mutant: the committed seed exposed a real
instrument defect on *healthy* code — its `join_all` step read BELOW
the pinned floor by 0.29 widths, an honest cheap fold the corpus never
sampled. The floor's slack was the ceiling's 0.2, calibrated for
allocator variance, silently reused for a claim it doesn't fit;
liveness needs decade-scale sensitivity only, so the floor margin is
now priced at 1.0 inside the measured gap between honest cheap readings
and dead-meter readings (§5), and the seed replays in-band forever.

### 2026-07-26 — the tilting mild superlinearity (residual-risk probe)

The round-2 review's residual risk: smooth mild superlinearity
(~n^1.3) on wide thin-per-case rows — the shape leg abstains without
within-case evidence, and `ff_version_meet`'s +0.87 ceiling over ~2
decades left ε ≈ +0.54 of slope headroom to the point leg. The cheap
mitigation inside the architecture: the escalation meet ladder (§3)
gives the row dense within-case mass, which both feeds the shape leg
and — the larger effect — collapses the pinned ceiling to +0.30.
Demonstrated by construction (n²/64 `black_box` work in
`ff_version_meet`, sized inside the round-2 pin's ceiling):

> ABOVE BAND (asymptotic regression): ff_version_meet at 6755 bits
> consumed 11199862 fuel; the pinned law predicts ~10^6.545
> +0.302/-2.850

That reading is 10^7.05 against the round-2 pin's 10^7.20 ceiling at
the same size: the mechanism the round-2 criteria blessed is red under
the meet-ladder pin.

**Residual risk, bounded honestly.** Within the instrument's reach, a
smooth superlinear mechanism with a small enough constant escapes both
legs whenever its whole in-reach rise stays inside
`width_above + 0.2` (point leg) and its within-case tilt stays inside
`pinned slope + 0.3` (shape leg, where evidence exists). Per row that
bounds the escaping exponent at roughly `slope + min(0.3,
(width_above + 0.2)/span_decades)`: ~1.33 on the post-ladder
`version_meet` (was ~1.68), ~1.3–1.45 on the remaining wide rows
(`version_join` +0.70 over ~2 decades ≈ slope+0.45 point-side, shape
evidence on the high-mass lanes; `version_project` +0.58 over ~2.5 ≈
slope+0.31; `meet_all`, shape-exempt, +0.89 over ~3 ≈ slope+0.30).
Tightening further is architectural — per-family bands or per-case
baselines, and a shape allowance re-derived from fresh-draw dispersion
rather than the corpus maximum (+0.006 today, 50x under the 0.3
allowance) — and deliberately out of scope for this round. The bound
is a property of any reach-limited envelope instrument: it prices
mechanisms by their in-reach excess, and a mechanism whose in-reach
excess is sub-threshold is, within the priced envelope, behaviorally
the envelope.

### Standing (continuous) demonstrations

- `ff_selftest_quadratic`: the guest's black_box-pinned quadratic burner
  must read ABOVE a linear band (and a stalled reading Below) on every
  suite run — the detection path's liveness, from wasm execution through
  fuel metering to the judge.
- The fixed escalation replay (`tests/enforce.rs`): the reach regime's
  bands — single-operand rows, rejection arms, the deferred-overlap
  scans, the meet ladder — get exercised deterministically on every
  suite run; §8's M1 re-construction reads red through this replay
  alone.
- `curve/tests.rs` and `fit/tests.rs`: the shape leg's and the staleness
  comparator's synthetic tripwires (quadratic flags / flat passes /
  under-evidenced abstains; a perturbed pin reads back its
  perturbation).
- The staleness cross-check's committed `REFIT_COVERAGE` list: every
  band key covered at pin time must keep its prefix fit, its
  classification, and its line — reach decay (the genre M1 exploited)
  fails by name instead of shrinking a count.
