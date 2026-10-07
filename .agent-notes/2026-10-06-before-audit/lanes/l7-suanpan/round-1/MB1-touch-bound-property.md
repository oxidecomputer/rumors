# MB1: a touch-bound property over arbitrary operation programs, with embedded gap-split rounds

Kind: machinery brief (a passing instrument to build on a clean branch from `main`).
Prototype: `crates/suanpan/src/accumulator/tests/explore_l7.rs`, module `metered`, test `l7_touches_are_linear_in_table_work`, on `explore/l7-suanpan` at `c30c045f`. Illustration only; the builder writes it fresh.

## The failure class it catches

An amortization regression on an interleaving that no pinned shape exercises. Constructed instance: when a write lands strictly inside a recorded zero range, `ZeroRanges::remove_written` must keep the part of the range above the write (`crates/suanpan/src/accumulator/digits/zero_ranges.rs:112-114` in the `One` branch, `:126-128` in the `Many` branch). Dropping that upper remnant keeps every value correct, but the next time the top cancels, `trim_high_zeros` steps down the whole gap one digit at a time. A four-write round (jump to a high digit, write inside the gap, cancel both) then costs `O(gap)` touches, every round.

- Both mutants survive the entire existing suanpan suite: 64 of 64 tests pass under each (mutants M14 and M15 in my calibration; `S/calibrate_existing.out`).
- The prototype property fails on each at 2,000 cases with a one-step program (`S/calibrate_touch_split2.out`):
  - M14 (`One` branch): `touches 216 > 8 * work 5 + 64`, input `[Limbs { shift: 0, limbs: [] }, GapRound { low: None, inner: 2, top: 213 }]`
  - M15 (`Many` branch): `touches 718 > 8 * work 6 + 64`, input `[GapRound { low: Some(4), inner: 8, top: 718 }]`
- It also catches M16 (lower remnant dropped), which the existing `sign_flip_oscillation_has_no_width_product` and `sign_query_skips_recorded_zero_ranges` also catch.

This is the instrument the September triage ruled (suanpan-2, ruling 91: a touch-meter proptest over arbitrary streams, `touches <= K * work + D`), which never landed: no test outside `metered.rs` and `tests/amortized_sequences.rs` reads `suanpan::touch_meter`.

## Which instrument it extends

suanpan's metered suite (`crates/suanpan/src/accumulator/tests/metered.rs`), which pins exact totals at canonical shapes only. The property samples the space between those shapes. `before`'s validation index (`crates/before/src/testing/validation_index.rs`) maps `before`'s instruments and has no suanpan section; I suggest stating this property's rationale in `metered.rs`'s module doc, beside the pins it complements. Whether suanpan instruments should enter the index is for the coordinator.

## Generator construction (builds valid programs directly; no filtering)

- An operation enum covering the public surface. The prototype's receiver is a pool of three accumulators, so operands carry arbitrary histories:
  - primitive `+=`/`-=` across widths
  - `i128` and `u128` values at the extremes and near `k * 2^(32 d)`
  - shifted limb streams, including empty and high-zero-padded ones
  - accumulator operands in borrowed, shifted, owned-clone, and owned-take forms
  - extreme-digit parking (two word deposits that leave `-(2^33 - 1)` in one digit)
  - `<<=`, unary `-`, `reset`, `normalize`, `cmp_zero`, `cmp_zero_stable_under` with literal widths and with another pool member's `stored_bits`
  - `reserve_digits`, clone-replace, and borrowed or owned `Sum`
- Swarm testing: a per-case weight vector of one weight per operation kind (each zero or 1..6), so cases differ in mix; zero-weight arms are dropped before building the `Union`.
- Shifts: the representation boundaries (29..33, 63..65, 95..97, ..., 255..257), small values, multiples of 32 and 32k + 31, a random range up to 6,000, and a small set of large *repeated* values (3,200, 3,207, 32,000, 32,031, 64,000), so writes re-cross the same gaps.
- The embedded pattern `GapRound { low, inner, top }`, in digit positions: write 1 at `low` (optional, which forces `Many` mode), at `top`, then at `inner` with `low < inner < top`; then cancel `top`, `inner`, and `low`. The round is value-neutral. Without it, random programs reach the split-then-cancel interleaving too rarely: at 2,000 cases the property missed M14 and M15 before the pattern was added.

## Work units and the bound

Touches are bracketed per step (reset the meter, apply, read), and each step is entitled to the cost table's operand work, never to growth: touches never pay for allocation, so charging growth `G` would pay for exactly the gap crossings this property must catch.

| Step | Work units |
|---|---|
| primitive update up to 64 bits | 1 |
| `i128` or `u128` update (four 32-bit pieces) | 4 |
| `L`-limb stream | `L + 1` |
| accumulator operand, borrowed or shifted, and owned subtraction | operand's stored digits + 1 |
| owned addition | lesser of the two stored widths + 1 (it reads the narrower) |
| extreme-digit park | 2 |
| `<<=`, unary `-`, `reset`, clone | receiver's stored digits + 1 |
| `normalize` | stored digits before + after + 1 |
| comparisons, reservations | 1 |
| `GapRound` | 4, or 6 with `low` |

The prototype asserts `touches <= 8 * work + 64` over the whole program. Observed ratios over 20,000 random programs stay below 6.4, set by the repeated-comparison steady state (the value `2 * 2^2048` costs 6 touches per `cmp_zero`, every time; a probe confirmed it). A hill-climbing adversary (`l7_adversarial_touch_search`, 12 seeds of 30,000 mutations) reached 9.6 by stacking `+= i128::MAX` on digit-form accumulators. The prototype prices those at 1 unit, so the 9.6 reflects its accounting, not amortization; rescaling every shift of the best programs by 2, 4, and 8 left each ratio unchanged or lower, so no found program's cost grows with width. With 128-bit primitives priced at 4 units, as in the table above, that family should read about 2.4 per unit (inferred by dividing; not re-run). Per ruling 91, the builder should derive `K` and `D` in the test's doc comment from the crate page's arguments (recentering, the compacting sign scan, the zero-range skips) and re-measure before committing. A number handed over here is a hypothesis.

Run the property alone in its process. The meter is process-global, and running it beside other tests in one process (libtest `--test-threads 2`) produced a spurious failure here; nextest's process-per-test isolation is sufficient.

## Calibration evidence (prototype)

- Caught: M14, M15, M16 (range remnants), M12 (`take_below` refusing a range ending exactly at the query: gap crossings go digit by digit; caught in one 2,000-case run and missed in another after the distribution changed, while the existing metered pins catch it reliably).
- Not caught, by design: M13 (one extra touch per consumed range, a constant).
- The committed property should carry a known-bad demonstration (the owner's adversarial-instrumentation rule). Options: commit M14's remnant-dropping mutation as a recorded calibration in the commit message, or build a test-only model of the trim that ignores remnants and show the property's bound fails against its counts. The builder should pick whichever the coordinator prefers.

## Runtime

Debug build, 2,000 cases about 16 s; 20,000 cases about 145 s serial. The default 256-case run is a few seconds.
