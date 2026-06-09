//! Sanity checks: panic-freedom, clone independence, degenerate
//! inputs.

mod common;

use proptest::collection::vec;
use proptest::prelude::*;
use rumors::sync::Known;

use crate::common::oracle::readout_multiset;
use crate::common::peer::{Peer, quiesce};
use crate::common::schedule::{arb_schedule, execute_and_quiesce};
use crate::common::sync_wire::sync_bootstrap_fork;

const N_PEERS: std::ops::RangeInclusive<usize> = 2..=8;
const MAX_EVENTS: usize = 50;
const MAX_CLONE_VALUES: usize = 8;

proptest! {
    /// Arbitrary schedules complete without panicking and produce a
    /// finite converged state. The safety net for every other
    /// invariant in the suite — if this fails, the others cannot run.
    #[test]
    fn arbitrary_schedules_dont_panic(
        schedule in arb_schedule(any::<u64>(), N_PEERS, MAX_EVENTS),
    ) {
        let _ = execute_and_quiesce(&schedule);
    }

    /// Merging is non-destructive: joining a snapshot of one peer into a copy
    /// of another reaches the same multiset as joining it straight in. This is
    /// the documented gossip pattern — `rumors` snapshots carry observations
    /// between peers, and `join` (a content merge) absorbs them.
    #[test]
    fn bootstrap_then_join_matches_direct_join(
        alice_values in vec(any::<u64>(), 0..=MAX_CLONE_VALUES),
        bob_values in vec(any::<u64>(), 0..=MAX_CLONE_VALUES),
    ) {
        // One universe seed; alice and bob are genuine party-disjoint forks.
        let seed = Known::<u64>::seed();
        let mut alice = sync_bootstrap_fork(&seed);
        alice.message(alice_values);

        let mut bob = sync_bootstrap_fork(&seed);
        bob.message(bob_values);

        // Two snapshots of bob's observations to feed both paths.
        let bob_snap = bob.rumors();
        let bob_snap2 = bob.rumors();

        // Recombine a disjoint copy of alice with one snapshot of bob.
        let mut recombined = sync_bootstrap_fork(&alice);
        recombined.join(bob_snap2).unwrap();

        // Direct: join bob's other snapshot straight into alice.
        let mut direct = alice;
        direct.join(bob_snap).unwrap();

        prop_assert_eq!(
            readout_multiset(&recombined),
            readout_multiset(&direct),
        );
    }
}

/// `quiesce` is a no-op on zero or one peer: it returns without
/// panicking, doesn't fire callbacks on the lone peer, and leaves
/// its `Known` content and observation log unchanged.
#[test]
fn quiesce_handles_zero_or_one_peer() {
    let mut zero: Vec<Peer<u64>> = Vec::new();
    quiesce(&mut zero);
    assert!(zero.is_empty());

    let mut peer = Peer::<u64>::new(Known::seed());
    peer.insert_one(42);
    let local_before = peer.local.rumors();
    let obs_before = peer.observations();

    let mut one = vec![peer];
    quiesce(&mut one);

    assert_eq!(one[0].local, local_before);
    assert_eq!(one[0].observations(), obs_before);
}
