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
  budget): one universe grown to several times the roster's fork cap
  with a snapshot-clock size ladder, so the pair, fold, and query rows
  sample every half-decade bucket decades past the rest of the roster —
  the fitted *slope*, not the band's width, carries the judgment there.

Reach runs along **two axes**, because superlinearity can live in
either: operand *bytes* (the escalation spine: folds to ~160k bits,
coupled party joins to ~9k bits over 12.8k samples) and fold *width*
(the ScatterFold width ladder: shuffled folds at doubling widths up to
1024 in every draw). A degenerate fold reduction is quadratic in width
at fixed operand size, and byte reach cannot stand in for width reach —
§8's first demonstration attempt is the proof.

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
(1536 programs, ~973k steps; proptest's deterministic runner + case-index
seeds; two sweeps are byte-identical) and fits, per kernel, `log₁₀ fuel`
against `log₁₀ denom`:

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
  (slope 0), the honest reading for O(1) rows.

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
(48 cases; the calibration corpus is the big sweep, this is the sentry)
and judges two legs with different failure modes:

- the **point leg**: every measured step lands in its kernel's band at
  its size, judged only at `denom ≥ min_denom`. Above-band is an
  asymptotic regression; below-band is a liveness flag; a kernel without
  a band fails (totality).
- the **shape leg** (`curve.rs`): no kernel's within-case bucket-median
  trend out-climbs its pinned slope by more than a measured allowance
  (+0.3, pinned 10x above the corpus's observed +0.030 healthy maximum).
  One case is one family at one draw — a family-pure population — so the
  cross-family mixture tilt that moves pooled medians cannot occur
  there, and a rising within-case trend is a mechanism's own curvature:
  the leg that sees a regression tilting *into* a wide band with small
  point residuals. The fold rows are exempt (their honest law trends
  along the width axis by the documented bounded log factor; the point
  leg plus the width ladder own them).

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
- **detection-path adequacy**: the guest ships a deliberately quadratic,
  `black_box`-pinned self-test burner (`ff_selftest_quadratic` — not a
  kernel; no strategy emits it, calibration never bands it), and the
  suite asserts it reads ABOVE a linear band anchored on its own
  small-input cost, and Below when stalled — the continuous answer to
  "could this instrument still see a quadratic", not a one-time
  demonstration.
- **pin staleness** (the quantity-computable-two-ways convention): the
  suite refits the deterministic 256-program prefix of the calibration
  stream and compares each kernel's fresh line against its pin
  (`fit::line_divergence`), with a measured tolerance (0.7, 1.5x the
  observed 0.463 prefix-vs-corpus sampling dispersion) and a coverage
  floor (40 of 44 kernels), so calibration drift fails loud instead of
  silently widening the gap between pin and reality. The comparator's
  own tripwire is committed (`fit/tests.rs`: a hand-perturbed pin reads
  back exactly its perturbation). This is a drift detector, not a
  mechanism catcher: §8 records that the join_all mutant slipped under
  its tolerance while the point leg flagged it.
- **pin provenance**: the building toolchain must equal
  `bands::PINNED_RUSTC` (§2).

## 6. Findings of the pin of record (2026-07-26)

The owner's expectation was confirmation, and confirmation is what the
instrument produced — all 44 kernels read linear within every family
above the floor. The mechanical form of that claim is the shape
diagnostic: across every evidence-bearing (kernel, case) pair in the
corpus, the maximum within-case slope excess over the pin is +0.030.
Twelve *pooled* envelope slopes sit in 1.10–1.23, every one
ground-truthed (`bin/diag`, the fit-free bucket-median view) to a known
mechanism:

- **Family-mixture composition** (the dominant genre: `ff_clock_tick`
  1.15, `ff_version_cmp`/`concurrent` 1.16, `ff_version_decode` 1.17,
  `ff_version_rank` 1.16, `ff_version_min_ticks` 1.14, `ff_version_meet`
  1.14, `ff_version_lag` 1.10, `ff_party_join` 1.23,
  `ff_party_is_disjoint` 1.20): families' per-bit cost levels differ
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
- `ff_party_covers` 1.09: same mixture genre, re-measured on this
  corpus. The reach family put flat ~79-fuel/bit samples at 10³·⁵–10⁴
  bits (DenseSpine's lane is flat ~80 across two decades), which pulled
  the pooled envelope down from the 1.18 the pre-reach corpus read;
  what remains is the cross-universe mixes' cheap mass (~36–58
  fuel/bit) under-pricing the small buckets. No lane rises.
- `ff_version_join_all` 1.15: `join_all`'s documented balanced
  binary-counter fold — every input passes through O(log n) joins, and
  the width ladder samples the factor across widths 8..1024 (healthy
  ladder medians 2906→5897 fuel/bit); fold width is budget-capped, so
  the factor is bounded and the band prices it. `ff_version_meet_all`
  −0.26 is the same row's opposite economics: escalated ladder meets
  collapse toward the common floor almost immediately (~0.2 fuel/bit at
  10⁵ bits against ~500–600 at 10²), so its envelope *falls* — sublinear,
  benign.
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

### Standing (continuous) demonstrations

- `ff_selftest_quadratic`: the guest's black_box-pinned quadratic burner
  must read ABOVE a linear band (and a stalled reading Below) on every
  suite run — the detection path's liveness, from wasm execution through
  fuel metering to the judge.
- `curve/tests.rs` and `fit/tests.rs`: the shape leg's and the staleness
  comparator's synthetic tripwires (quadratic flags / flat passes /
  under-evidenced abstains; a perturbed pin reads back its
  perturbation).
