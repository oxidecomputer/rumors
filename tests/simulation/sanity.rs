//! Sanity checks: panic-freedom, clone independence, degenerate
//! inputs.

use proptest::collection::vec;
use proptest::prelude::*;
use rumors::Local;

use crate::oracle::readout_multiset;
use crate::peer::{Peer, quiesce};
use crate::schedule::{arb_schedule, execute_and_quiesce};

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

    /// Cloning is non-destructive: a clone that ingests new values,
    /// recombined with the original via `+`, yields the same content
    /// multiset as the original ingesting those values directly. This
    /// is the documented use case — clones drive remote gossip in
    /// parallel; mutation happens on one side and recombines.
    #[test]
    fn clone_then_merge_matches_direct_ingest(
        original_values in vec(any::<u64>(), 0..=MAX_CLONE_VALUES),
        helper_values in vec(any::<u64>(), 0..=MAX_CLONE_VALUES),
    ) {
        let mut base: Local<u64> = Local::for_party("alice");
        base.message(original_values.clone(), |_, _, _| {});

        let mut helper = base.clone();
        helper.message(helper_values.clone(), |_, _, _| {});

        let recombined = base.clone() + helper;

        let mut direct = base;
        direct.message(helper_values, |_, _, _| {});

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
    let local_before = peer.local.clone();
    let obs_before = peer.observations.clone();

    let mut one = vec![peer];
    quiesce(&mut one);

    assert_eq!(one[0].local, local_before);
    assert_eq!(one[0].observations, obs_before);
}
