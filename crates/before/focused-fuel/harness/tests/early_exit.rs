//! Early exits: an operation whose answer is decided stops reading.
//!
//! Each fixture decides its answer at the first elementary interval of a
//! comb whose tail grows with the probe's tooth count, so a walk that stops
//! there reads the same fuel at every tail length, and one that reads on
//! grows with the tail.

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

/// Comb tooth counts for the span and query exits, an eightfold range of
/// tail lengths behind a fixed deciding interval.
const TEETH: [u64; 3] = [512, 1_024, 4_096];

/// Span queries stop at the first refuted direction.
///
/// `dominance` stops when the start is concurrent with the probe or strictly
/// above it, `precedence` when the end is concurrent with the probe or
/// strictly below it, and `contains` when the probe lies above the end or
/// below the start.
///
/// Every fixture decides at the comb's first interval while the comb's tail
/// grows eightfold. Form: equal fuel (F-eq) over the tooth count, for each
/// case.
#[test]
fn span_queries_stop_at_the_first_refuted_direction() {
    for probe in [
        Probe::ExitDominanceConcurrentStart,
        Probe::ExitDominanceDominatingStart,
        Probe::ExitPrecedenceConcurrentEnd,
        Probe::ExitPrecedencePrecededEnd,
        Probe::ExitContainsAboveEnd,
        Probe::ExitContainsBelowStart,
    ] {
        assert_equal(probe, &TEETH);
    }
}

/// On a contained probe, `dominance` drops the end's stream once the end's
/// domination is refuted, and `precedence` drops the start's stream once
/// `probe <= lo` is refuted, each finishing on the other endpoint alone.
///
/// The dropped endpoint is the one whose tail grows. Form: equal fuel (F-eq)
/// over the tooth count, for each coarsening.
#[test]
fn coarsenings_drop_the_refuted_endpoint_stream() {
    for probe in [
        Probe::ExitDominanceDropsTheEnd,
        Probe::ExitPrecedenceDropsTheStart,
    ] {
        assert_equal(probe, &TEETH);
    }
}

/// A single-bound query answers at its first refuted direction: a refuted
/// ceiling and a satisfied hole stop at the first interval.
///
/// Form: equal fuel (F-eq) over the tooth count, for each bound.
#[test]
fn single_bound_queries_stop_at_the_first_refuted_direction() {
    for probe in [Probe::ExitCeilingRefuted, Probe::ExitHoleSatisfied] {
        assert_equal(probe, &TEETH);
    }
}
