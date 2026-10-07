# Instrument calibrations (lane L8)

Each row injects a defect an instrument claims to catch, by reversible
string swap in the detached probe copy `l8/probe/` (rebuilt from `crates/`
by `l8/mkprobe.sh` before and after every injection), and records the
verdict. Verified on ox-east-1 unless marked.

## Summary

| instrument | injections | caught | record |
|---|---|---|---|
| wasm32 pins | 11 named narrowings or route errors | 7 | findings/wasm32-pins-calibration.md |
| surface check | 2 | 2 | below |
| asymptotics fold pins | 1 (fold degraded to quadratic) | 0 (by design: floors only) | below |
| nextest timeout, detached workspaces | 1 (200 s test) | not terminated | briefs/machinery-detached-nextest-timeouts.md |
| amplification board, worst-case pin | platform change (x86 vs non-x86 `num-bigint`) | pin drifts | findings/count-display-heap.md |

Earlier, separately: the generator census's own floors
(`generators/tests.rs`) and the differential table's known-bad descriptors
(`diff_ops/tests.rs`) are committed calibrations; I read them and did not
re-run them as injections.

## Surface check (`crates/before/surfacecheck`)

Script `l8/surface.sh` builds `before`'s nightly rustdoc JSON from the probe
copy and runs the check on it (logs in surface/).

- Unmodified copy: exit 0, "211 public function-like items = 134
  board-covered + 77 excepted (0 item exceptions, 3 module-scope); 523 trait
  impls = 453 pinned + 70 module-excepted; 46 items = 3 pinned + 43
  module-excepted".
- A new public function at the crate root (`pub fn l8_unpriced_probe() -> u8`):
  exit 1, "public function-like items with no board disposition and no
  exception (1): l8_unpriced_probe".
- A new public trait impl (`impl fmt::Binary for Count`): exit 1,
  "reachable trait impls with no census pin and no exception (1): Count:
  impl core::fmt::Binary for Count".

Boundary, documented by the check itself (`surfacecheck/src/extract.rs:30-31`):
`#[doc(hidden)]` public items never appear in rustdoc JSON, so they escape
the census. The probe copy's own `#[doc(hidden)] pub mod l8probe` (with a
public function) passed unnoticed, confirming it. Production `before` has no
`#[doc(hidden)]` items today (grep), so this is a stated boundary, not a gap.

The check is an existence and resource census: it requires every public
item to be priced or excused, not to be semantically tested. The trait-impl
mutation survivors (operator spellings, `Debug`, `Hash`, `size_hint`) all
pass it, as they should.

## Asymptotics fold pins (`src/testing/asymptotics.rs`)

The mutation campaign's `fold.rs` survivors (`balanced_try_fold`:
`weight += 1` as `weight *= 1`, and `*w == weight` as `!=`) keep every value
and destroy the balance: every input merges into one growing accumulator.
Measured with the first, from the probe copy (logs fold-original.log,
fold-mutant.log), scan bits of each fold entry point on the committed
populations at arity 256 and 1,024:

| entry point | original 256 | original 1,024 | mutant 256 | mutant 1,024 |
|---|---:|---:|---:|---:|
| `Version::join_all` | 4,906,266 | 28,537,882 | 25,725,854 | 405,299,102 |
| `Version::meet_all` | 4,907,680 | 28,543,880 | 25,725,292 | 405,296,996 |
| `Version::span_all` | 5,241,754 | 30,212,634 | 25,727,316 | 405,300,828 |
| `Party::join_all` | 4,011,272 | 22,639,368 | 17,984,264 | 274,320,136 |
| `Clock::join_all` | 8,917,542 | 51,218,342 | 43,642,206 | 679,353,822 |

Growth across the 4x arity step: 5.82 original, 15.75 mutant (quadratic)
for `Version::join_all`. All five log-factor pins pass under the mutant:
they assert a growth floor ("If a linear fold lands behind this entry
point, this pin reads red"), which guards the log factor against
disappearing, not against degrading. The board is the instrument for
degradation: under its declared `D log k` model the mutant reads
405,299,102 / (303,104 bytes x 10 levels), about 134 bits per unit, against
the ceiling `FOLD_SCAN_BITS_PER_INPUT_BYTE_PER_LEVEL = 17.0`, and its trend
fit would see quadratic growth. So the board's acceptance run should catch
it (inferred from the arithmetic; I did not run the board on the mutant,
which takes a full acceptance sweep).

Consequence for reading the mutation survivors: cargo-mutants judges
against the nextest suites only, so a cost-only survivor may still be caught
by the gate's board leg. The survivor files mark such mutants.
