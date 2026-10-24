//! Party walks that resolve deep subtrees whole: `sync`'s fused join and
//! fork, and `without`'s splice and block paths.

use focused_fuel_harness::{assert_at_most, assert_flat, measure, Probe};

/// Party depths, a sixteenfold step apart.
const DEPTHS: [u64; 2] = [256, 4_096];

/// Synchronization costs no more than a separate join and fork, on a pair
/// that delegates deep cells along a shared path, a pair resolved by one
/// splice, and a pair of adjacent deep cells.
///
/// `Clock::sync` reaches the fused party walk, and joins two empty versions
/// besides. Form: derived bound (F-bound) by the reading of `Party::join`
/// then `Party::fork` on the same pair, at each depth.
#[test]
fn sync_costs_no_more_than_join_then_fork() {
    for depth in DEPTHS {
        for (sync, composed) in [
            (Probe::SyncDelegated, Probe::JoinForkDelegated),
            (Probe::SyncSplice, Probe::JoinForkSplice),
            (Probe::SyncAdjacent, Probe::JoinForkAdjacent),
        ] {
            assert_at_most(
                &format!("{sync:?} vs {composed:?} at depth {depth}"),
                measure(sync, depth, 0),
                measure(composed, depth, 0),
            );
        }
    }
}

/// A spliced pair syncs in work linear in its operands: growing the deep
/// cell's depth sixteenfold multiplies the fuel per level by at most 1.25.
///
/// Form: flat per unit (F-lin) across a sixteenfold growth of the depth,
/// taking the depth as the unit, which the operands' encoded bits are
/// proportional to.
#[test]
fn spliced_sync_is_linear_in_its_operands() {
    let [small, large] = DEPTHS;
    assert_flat(
        "spliced sync per level",
        (measure(Probe::SyncSplice, small, 0), small),
        (measure(Probe::SyncSplice, large, 0), large),
    );
}

/// Copying or discarding a covered deep subtree costs no more than carving
/// a deep cell out of the seed, which builds its complement level by level,
/// and costs work linear in its operands.
///
/// The deep cell lies beside an overlap in the right half, so the public
/// `without` passes its disjointness check and walks: the cell under an
/// unowned cover survives whole, and under an owned cover vanishes whole.
/// Forms: derived bound (F-bound) by the complement walk's reading at each
/// depth, and flat per unit (F-lin) across a sixteenfold growth of the
/// depth.
#[test]
fn without_resolves_a_covered_subtree_whole() {
    for probe in [Probe::WithoutSplice, Probe::WithoutBlock] {
        for depth in DEPTHS {
            assert_at_most(
                &format!("{probe:?} vs the complement walk at depth {depth}"),
                measure(probe, depth, 0),
                measure(Probe::WithoutComplement, depth, 0),
            );
        }
        let [small, large] = DEPTHS;
        assert_flat(
            &format!("{probe:?} per level"),
            (measure(probe, small, 0), small),
            (measure(probe, large, 0), large),
        );
    }
}
