//! Early exits: an operation whose answer is decided stops reading.

use focused_fuel_harness::{assert_equal, Probe};

/// The semantic equality walk on a pair the first elementary interval
/// refutes reads only that interval: its fuel is the same whatever the length
/// of the comb's tail.
///
/// The comb `C(1024, teeth)` keeps its first leaf at `2^1024 − 1` at every
/// tooth count, so the deciding interval's work is fixed by construction and
/// only the tail grows, eightfold across the sizes. Form: equal fuel (F-eq)
/// over the tooth count.
#[test]
fn eq_exit_cost_is_tail_independent_on_first_interval_refuted_pair() {
    assert_equal(Probe::EqRefutedAtFirstInterval, &[512, 1_024, 4_096]);
}
