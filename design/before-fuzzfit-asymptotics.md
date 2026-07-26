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
(release, `codegen-units = 1`, `panic = "abort"`) plus the toolchain named
in the pin make the constants reproducible.

## 3. Generators

One strategy per meter-board family — the archetype with randomized
dimensions and structural jitter, built **exclusively through public
operations** (seed, fork, tick, join, …), so every constructed value is
API-reachable by definition and the crate-doc safety rules hold by
construction inside each universe. The board's control variants ride as
parameters (`hifloor`, `plateau`, `tail_ticks = 1`). Two family kinds sit
on top of the roster:

- **Combination programs**: weighted random walks over the whole op
  vocabulary, operands drawn from everything constructed so far —
  the composition space between the named shapes.
- **Independent programs** (two operand regimes, per the project owner's
  direction 2026-07-26): every multi-operand operation is exercised both
  on *coupled* operands (one universe, valid together — every other
  family) and on operands from *separately seeded universes*, where the
  result is meaningless but the cost claim still binds. Operations that
  reject such operands (`Party::join`, `Clock::join`/`sync` on overlap)
  have the rejection arm measured as its own outcome, predicted by the
  mirror per case, never assumed.

Budgets (`strategies::BUDGET`) cap ops, ticks, forks, and fold width
unconditionally in the builder, bounding constructed size a priori and
serving as the honesty bound for composed cases. **Scope consequence**:
magnitude is explored only up to the tick budget — a `2^b`-wide leaf is
API-reachable only by paying `2^b` ticks — so the wide-magnitude regime
(bigroot/hugeleaf at record scale) deliberately remains the meter board's
hand-built territory. This instrument's envelope is the API-reachable
region within budget.

## 4. Denomination

Exactly §6's rules, computed by the mirror from real values: packed
operand bits everywhere, except text I/O (packed + text, output read from
the actual result), output-dominated projection (`Version / Party`,
`Clock::own_version`: input + packed output) and balanced share splitting
(`Party::forks(n)`: input + n packed shares), and rank operations (value
content, proxied by the rank's decimal rendering; the constant folds into
the intercept).

## 5. Bands: calibration and enforcement

**Calibration** (`just fuzzfit-calibrate`) sweeps a deterministic corpus
(1536 programs, ~670k steps; proptest's deterministic runner + case-index
seeds; two sweeps are byte-identical) and fits, per kernel, `log₁₀ fuel`
against `log₁₀ denom`:

- the **slope** over half-decade *bucket medians* above a 128-bit fit
  floor — per-step fuel is heteroscedastic (fast-path mass below the
  constant-overhead knee, amortization spikes above), and raw OLS read up
  to +0.4 slope on kernels whose per-shape medians are flat;
- the **width** as the max |residual| of *all* floored samples against
  that line — bounded amortization spikes land inside the committed band;
  only unbounded (asymptotic) departures escape;
- kernels spanning under a decade (or three buckets) classify **constant**
  (slope 0), the honest reading for O(1) rows.

The result is rewritten atomically into `harness/src/bands.rs` and
committed — reviewed like a snapshot, with a dated movement annotation in
the module doc. The fit is never recomputed in enforcement (refitting on
every run would mask drift). **Re-pin events**: a guest toolchain bump, a
kernel change, a strategy change. Re-pin = run the recipe, read the diff,
date the annotation.

**Enforcement** (`just fuzzfit`, `tests/enforce.rs`) draws fresh programs
(48 cases; the calibration corpus is the big sweep, this is the sentry)
and asserts every measured step lands in its kernel's band at its size,
judged only at `denom ≥ min_denom`. Above-band is an asymptotic
regression; below-band is a liveness flag; a kernel without a band fails
(totality). Fuel determinism makes replay exact: a failure shrinks to a
minimal out-of-band shape and rides as a committed proptest seed. The
judgment's own tripwire is committed (`sanity.rs`): a quadratic reading
must flag Above, a dead-meter reading Below, before any fuzzing counts.
Meter liveness is pinned separately (`ff_nop` fuel in (0, 100)).

## 6. Findings of the initial pin (2026-07-26)

The owner's expectation was confirmation, and confirmation is what the
instrument produced — all 45 kernels linear-or-better above the floor
(slopes 0.0–1.11), with every reading above 1.1 ground-truthed to a known
mechanism (`bin/diag`, the fit-free bucket-median view):

- `ff_version_join_all` 1.24 / `ff_version_meet_all` 1.16: `join_all`'s
  documented balanced binary-counter fold — every input passes through
  O(log n) joins; fold width is budget-capped at 64, so the factor is
  bounded and the band carries it.
- `ff_clock_tick` 1.22: family-mixture composition. Every family's own
  fuel/bit medians are flat or falling; cheap-constant shapes (hugeleaf,
  bigroot's tick mass) dominate the small buckets and deep structured
  shapes the large ones, so the mixture envelope tilts. Pinning the
  envelope is correct for enforcement (per-step judgments hold under any
  mixture).
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
