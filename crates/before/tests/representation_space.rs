//! Resident-space comparison against the recursive oracle.
//!
//! For an unshared recursive version tree, branching shape does not change the
//! number of nodes or child allocations at a fixed leaf count. The production
//! representation also spends one topology bit per node, but its payload size
//! depends on differences between adjacent leaf values. With event counts
//! capped at `2^64`, alternating `0` and `2^64` therefore gives the least
//! favorable large-tree comparison: every encoded difference is as wide as
//! the cap permits while the oracle's node count is unchanged.

use std::hint::black_box;
use std::mem::size_of_val;
use std::sync::Arc;

use before::testing::oracles::tree as oracle;
use before::{Clock, Count};
use num_bigint::BigUint;
use peak_alloc::PeakAlloc;

#[global_allocator]
static HEAP: PeakAlloc = PeakAlloc;

/// Build `count` clocks by repeatedly doubling a balanced population.
fn production_population(count: usize) -> Vec<Clock> {
    let mut clocks = vec![Clock::seed()];
    while clocks.len() < count {
        let round = clocks.len();
        for parent in 0..round.min(count - clocks.len()) {
            let child = clocks[parent].fork();
            clocks.push(child);
        }
    }
    clocks
}

/// Build the oracle population corresponding to [`production_population`].
fn oracle_population(count: usize) -> Vec<oracle::Clock> {
    let mut clocks = vec![oracle::Clock::seed()];
    while clocks.len() < count {
        let round = clocks.len();
        for parent in 0..round.min(count - clocks.len()) {
            let child = clocks[parent].fork();
            clocks.push(child);
        }
    }
    clocks
}

/// Measure a value's inline size and owned heap, excluding the harness baseline.
fn resident_bytes<T>(build: impl FnOnce() -> T) -> usize {
    let baseline = HEAP.current_usage();
    let value = black_box(build());
    let resident = size_of_val(&value) + HEAP.current_usage() - baseline;
    black_box(&value);
    drop(value);
    assert_eq!(
        HEAP.current_usage(),
        baseline,
        "the measured value must release every allocation it owns"
    );
    resident
}

/// Reunite a production population into one clock.
fn reunite_production(clocks: Vec<Clock>) -> Clock {
    let mut clocks = clocks.into_iter();
    let mut reunited = clocks.next().expect("the population is nonempty");
    for clock in clocks {
        reunited.join(clock).expect("forked parties are disjoint");
    }
    reunited
}

/// Reunite an oracle population into one clock.
fn reunite_oracle(clocks: Vec<oracle::Clock>) -> oracle::Clock {
    let mut clocks = clocks.into_iter();
    let mut reunited = clocks.next().expect("the population is nonempty");
    for clock in clocks {
        reunited.join(clock).expect("forked parties are disjoint");
    }
    reunited
}

/// Reunite a production population whose leaf-order counts alternate between
/// zero and `high`.
fn alternating_production_clock(parties: usize, high: &Count) -> Clock {
    let mut clocks = production_population(parties);
    if parties == 1 {
        clocks[0].ticks(high.clone());
    } else {
        // Balanced forking stores leaf paths in bit-reversed index order. The
        // high index bit is therefore the low path bit: setting the upper half
        // makes adjacent leaves alternate.
        for clock in &mut clocks[parties / 2..] {
            clock.ticks(high.clone());
        }
    }
    reunite_production(clocks)
}

/// Build the oracle event tree with `leaves` alternating zero/high leaves.
fn alternating_oracle_version(leaves: usize, high: &BigUint) -> oracle::Version {
    assert!(leaves.is_power_of_two(), "the tree must be balanced");

    /// Build one balanced range of the alternating leaf sequence.
    fn build(start: usize, leaves: usize, high: &BigUint) -> oracle::Version {
        if leaves == 1 {
            return oracle::Version::Leaf(if start.is_multiple_of(2) {
                BigUint::ZERO
            } else {
                high.clone()
            });
        }
        let half = leaves / 2;
        oracle::Version::Node(
            BigUint::ZERO,
            Arc::new(build(start, half, high)),
            Arc::new(build(start + half, half, high)),
        )
    }

    build(0, leaves, high)
}

/// Representation savings depend on structure: a balanced history exceeds a
/// 100× reduction, while one wide leaf removes that structural advantage.
#[test]
fn resident_space_comparison_is_scoped_by_shape() {
    const PARTIES: usize = 512;

    let production_tree = resident_bytes(|| {
        let mut clocks = production_population(PARTIES);
        for (index, clock) in clocks.iter_mut().enumerate() {
            clock.ticks(index.count_ones() as usize + 1);
        }
        reunite_production(clocks)
    });
    let oracle_tree = resident_bytes(|| {
        let mut clocks = oracle_population(PARTIES);
        for (index, clock) in clocks.iter_mut().enumerate() {
            for _ in 0..=index.count_ones() {
                clock.tick();
            }
        }
        reunite_oracle(clocks)
    });

    assert!(
        production_tree * 100 <= oracle_tree,
        "production retained {production_tree} B; oracle retained {oracle_tree} B"
    );

    let high_ticks = Count::from(u64::MAX) + Count::from(1u8);
    let high_value = BigUint::from(u64::MAX) + 1u8;
    for parties in [1, 2, 4, 8, 16, 32, 64, 128, 256, 512] {
        let production = resident_bytes(|| alternating_production_clock(parties, &high_ticks));
        let oracle = resident_bytes(|| {
            oracle::Clock::from_parts(
                oracle::Party::seed(),
                alternating_oracle_version(parties, &high_value),
            )
        });
        assert!(
            oracle < production * 8,
            "the alternating-extremes family should keep the recursive form below 8×: \
             parties={parties}, production={production} B, oracle={oracle} B"
        );
    }
}
