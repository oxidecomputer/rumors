//! `Version::ticks`' cost in its count, an axis the board holds fixed.
//!
//! `ticks(n)` claims `O(|v| + |p| + log n)`: the whole dependence on `n` is
//! the work that carries the count's bits, so its cost above a baseline
//! count must grow linearly in those bits.

use focused_fuel_harness::{assert_flat, assert_less, measure, Probe};

/// The three operands of the tick claims: a dense spine through the seed,
/// and two wide crosses through nested ids.
const FAMILIES: [Probe; 3] = [
    Probe::TicksDense,
    Probe::TicksNestedWide,
    Probe::TicksMirrorWide,
];

/// Asserts that `probe`'s fuel above the count `2^base` is flat per count
/// bit between the counts `2^near` and `2^far`, ticking as is when `grow` is
/// zero and after one public tick when it is one.
fn assert_flat_in_count_bits(probe: Probe, grow: u64, base: u64, near: u64, far: u64) {
    let baseline = measure(probe, base, grow);
    let span = |exponent: u64| measure(probe, exponent, grow).saturating_sub(baseline);
    assert_flat(
        &format!("{probe:?} above 2^{base} per count bit"),
        (span(near), near - base),
        (span(far), far - base),
    );
}

/// The fraction of a count that a walk iterating over the count would visit
/// at least, for the word-count bound: one tick in 64.
const ITERATED_FRACTION: u64 = 64;

/// `ticks(n)` at counts that fit a machine word does not iterate over the
/// count: from `ticks(2^9)` to `ticks(2^15)`, its cost moves by less than a
/// walk visiting one tick in 64 of the added count would add.
///
/// Every loop iteration executes at least one instruction, so such a walk
/// adds at least `(2^15 − 2^9) / 64 = 504` fuel. The codes carrying the count
/// widen by two bits per doubling, which costs a few instructions. Form:
/// derived bound (F-bound) on the cost's movement, for each family.
#[test]
fn ticks_cost_does_not_iterate_over_word_counts() {
    let (low, high) = (9, 15);
    let bound = ((1u64 << high) - (1u64 << low)) / ITERATED_FRACTION;
    for probe in FAMILIES {
        let moved = measure(probe, high, 0).abs_diff(measure(probe, low, 0));
        assert_less(
            &format!("{probe:?} movement from 2^{low} to 2^{high} vs one tick in 64"),
            moved,
            bound,
        );
    }
}

/// `ticks(n)` keeps its cost linear in the count's width at counts far past
/// every machine integer.
///
/// On the grow branch, from a word-sized baseline to
/// an 8,192-bit count and its width doubling, the count-attributable cost
/// grows with the count's bits, including the count's own arithmetic.
///
/// A public tick first makes the measured call the pure grow branch, where
/// registering `n` events grows the minimum tick count by exactly `n`, which
/// the probe checks. Form: flat per unit (F-lin) of the cost above
/// `ticks(2^9)` per count bit.
#[test]
fn ticks_cost_is_linear_in_wide_count_bits() {
    for probe in FAMILIES {
        assert_flat_in_count_bits(probe, 1, 9, 8_192, 16_384);
    }
}
