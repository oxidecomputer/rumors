//! Sanity checks: panic-freedom, clone independence, degenerate
//! inputs.

mod common;

use proptest::collection::vec;
use proptest::prelude::*;
use rumors::sync::{Local, ignore};

use crate::common::oracle::readout_multiset;
use crate::common::peer::{Peer, quiesce};
use crate::common::schedule::{arb_schedule, execute_and_quiesce};

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

    /// Forking is non-destructive: merging a peer's fork into an
    /// independent peer reaches the same multiset as processing that
    /// peer in directly. This is the documented gossip pattern —
    /// forks carry observations between Originals.
    #[test]
    fn fork_then_merge_matches_direct_process(
        alice_values in vec(any::<u64>(), 0..=MAX_CLONE_VALUES),
        bob_values in vec(any::<u64>(), 0..=MAX_CLONE_VALUES),
    ) {
        let mut alice = Local::<u64, _>::for_party("alice", 0).unwrap();
        alice.message(alice_values, ignore);

        let mut bob = Local::<u64, _>::for_party("bob", 0).unwrap();
        bob.message(bob_values, ignore);

        // Recombine alice with bob's fork via `+` on a fork of alice.
        let bob_fork = bob.fork();
        let recombined = alice.fork() + bob_fork.clone();

        // Direct: process bob's fork straight into alice.
        let mut direct = alice;
        direct.process(bob_fork, ignore);

        prop_assert_eq!(
            readout_multiset(&recombined),
            readout_multiset(&direct),
        );
    }
}

/// `quiesce` is a no-op on zero or one peer: it returns without
/// panicking, doesn't fire callbacks on the lone peer, and leaves
/// its `Local` content and observation log unchanged.
#[test]
fn quiesce_handles_zero_or_one_peer() {
    let mut zero: Vec<Peer<u64>> = Vec::new();
    quiesce(&mut zero);
    assert!(zero.is_empty());

    let mut peer = Peer::<u64>::new("alone");
    peer.insert_one(42);
    let local_before = peer.local.fork();
    let obs_before = peer.observations();

    let mut one = vec![peer];
    quiesce(&mut one);

    assert_eq!(one[0].local, local_before);
    assert_eq!(one[0].observations(), obs_before);
}
