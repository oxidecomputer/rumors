//! Sanity checks: panic-freedom, fork independence, degenerate
//! inputs.

mod common;

use proptest::collection::vec;
use proptest::prelude::*;
use rumors::Known;

use crate::common::oracle::readout_multiset;
use crate::common::peer::{Peer, quiesce};
use crate::common::schedule::{arb_schedule, execute_and_quiesce};
use crate::common::wire::{bootstrap_fork, wire_gossip};

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

    /// Merging is non-destructive and path-independent: gossiping bob's
    /// content into a fresh party-disjoint fork of alice reaches the same
    /// live multiset as gossiping bob straight into alice. This is the
    /// documented propagation pattern — wire gossip is the merge, and any
    /// same-universe peer can carry content to any other.
    #[test]
    fn forked_gossip_matches_direct_gossip(
        alice_values in vec(any::<u64>(), 0..=MAX_CLONE_VALUES),
        bob_values in vec(any::<u64>(), 0..=MAX_CLONE_VALUES),
    ) {
        // One universe seed; alice and bob are genuine party-disjoint forks.
        let mut seed = Known::<u64>::seed();
        let mut alice = bootstrap_fork(&mut seed);
        {
            let mut batch = alice.batch();
            for v in &alice_values {
                batch.send(*v);
            }
        }

        let mut bob = bootstrap_fork(&mut seed);
        {
            let mut batch = bob.batch();
            for v in &bob_values {
                batch.send(*v);
            }
        }

        // Recombine a disjoint copy of alice with a carrier of bob's content.
        let mut recombined = bootstrap_fork(&mut alice);
        let mut bob_carrier = bootstrap_fork(&mut bob);
        wire_gossip(&mut recombined, &mut bob_carrier);

        // Direct: gossip bob straight into alice.
        wire_gossip(&mut alice, &mut bob);

        prop_assert_eq!(
            readout_multiset(&recombined.snapshot()),
            readout_multiset(&alice.snapshot()),
        );
    }
}

/// `quiesce` is a no-op on zero or one peer: it returns without
/// panicking, doesn't gossip the lone peer, and leaves its `Known`
/// content and observation log unchanged.
#[test]
fn quiesce_handles_zero_or_one_peer() {
    let mut zero: Vec<Peer<u64>> = Vec::new();
    quiesce(&mut zero);
    assert!(zero.is_empty());

    let mut peer = Peer::<u64>::new(Known::seed());
    peer.insert_one(42);
    let hash_before = peer.local.hash();
    let latest_before = peer.local.latest();
    let obs_before = peer.observations();

    let mut one = vec![peer];
    quiesce(&mut one);

    assert_eq!(one[0].local.hash(), hash_before);
    assert_eq!(one[0].local.latest(), latest_before);
    assert_eq!(one[0].observations(), obs_before);
}
