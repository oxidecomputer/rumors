//! Allocation traffic for a divergent local join, including metadata preparation.

mod common;

use std::alloc::System;
use std::collections::BTreeSet;

use rumors::Peer;
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};

/// Count allocations in this test binary; it runs one synchronous measurement.
#[global_allocator]
static ALLOCATOR: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

/// A broad root fan, with enough leaves for internal branches below it.
const SHARED: usize = 4096;
/// Each side adds and redacts this many leaves in distinct parts of the tree.
const CHANGES: usize = 16;

/// Budget for this fixture's join and metadata, including temporary allocations.
/// The wide shared base should stay shared; copying it would exceed this bound.
const JOIN_ALLOCATION_BUDGET: usize = 128 * 1024;

/// A mostly shared, wide tree must not be copied wholesale to merge a small
/// divergence. Measure the join and its reader metadata, after fixture setup.
#[test]
fn wide_join_allocation_is_bounded() {
    let ours = Peer::<usize>::seed().into_rumors();
    ours.send_all(0..SHARED).unwrap();
    let theirs = common::wire::bootstrap_fork(&ours);
    let redactions: Vec<_> = ours
        .snapshot()
        .iter()
        .take(2 * CHANGES)
        .map(|(version, value)| (version.clone(), *value))
        .collect();
    for (side, peer) in [&ours, &theirs].into_iter().enumerate() {
        peer.send_all(SHARED + side * CHANGES..SHARED + (side + 1) * CHANGES)
            .unwrap();
        peer.redact_all(
            redactions[side * CHANGES..(side + 1) * CHANGES]
                .iter()
                .map(|(v, _)| v),
        );
    }
    let ours = ours.snapshot();
    let theirs = theirs.snapshot();
    let region = Region::new(ALLOCATOR);
    let joined = rumors::testing::join_snapshots(&ours, &theirs);
    let stats = region.change();
    assert!(
        stats.bytes_allocated <= JOIN_ALLOCATION_BUDGET,
        "join allocated {} bytes, exceeding its {}-byte budget",
        stats.bytes_allocated,
        JOIN_ALLOCATION_BUDGET,
    );

    let removed: BTreeSet<_> = redactions.into_iter().map(|(_, value)| value).collect();
    let expected: BTreeSet<_> = (0..SHARED + 2 * CHANGES)
        .filter(|value| !removed.contains(value))
        .collect();
    let actual: BTreeSet<_> = joined.iter().map(|(_, value)| *value).collect();
    assert_eq!(actual, expected);
}
