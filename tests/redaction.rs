//! Redaction-specific corners.
//!
//! Most of redaction is exercised generically by the multi-peer suite
//! (`readout_matches_oracle_after_quiesce` in particular: the oracle
//! bakes in `redact` events, and every peer's readout must match).
//! These tests target the redaction-specific corners with smaller,
//! more legible schedules.

mod common;

use std::collections::BTreeMap;

use proptest::collection::vec;
use proptest::prelude::*;
use rumors::Key;
use rumors::sync::Known;

use crate::common::oracle::readout_multiset;
use crate::common::peer::{Peer, gossip_step, quiesce};
use crate::common::sync_wire::sync_bootstrap_fork;

proptest! {
    /// A redaction issued by *any* peer propagates contagiously to
    /// every peer after sufficient gossip — no peer retains the
    /// message and no peer re-introduces it.
    ///
    /// Peer 0 inserts a message and quiesce propagates it everywhere;
    /// then a peer chosen by `redactor_idx` (not necessarily peer 0)
    /// issues the redaction; after a final quiesce, every peer's
    /// live multiset is empty.
    #[test]
    fn redaction_propagates_from_any_peer(
        n_peers in 2usize..=6,
        value in any::<u64>(),
        redactor_idx in any::<usize>(),
    ) {
        let seed = Known::<u64>::seed();
        let mut peers: Vec<Peer<u64>> = (0..n_peers)
            .map(|_| Peer::new(sync_bootstrap_fork(&seed)))
            .collect();

        let key = peers[0].insert_one(value);
        quiesce(&mut peers);

        for peer in &peers {
            let live = readout_multiset(&peer.local);
            prop_assert_eq!(live.get(&value).copied(), Some(1));
        }

        let r = redactor_idx % n_peers;
        peers[r].redact_one(key);
        quiesce(&mut peers);

        for (i, peer) in peers.iter().enumerate() {
            prop_assert!(
                readout_multiset(&peer.local).is_empty(),
                "peer {} still has live messages after redaction by peer {}",
                i, r,
            );
        }
    }

    /// Two peers each insert several values, then each redacts one of
    /// its own keys. The converged content is the same regardless of
    /// which side issues its redaction first across the gossip
    /// boundary.
    #[test]
    fn concurrent_redactions_order_independent(
        a_values in vec(any::<u64>(), 1..=4),
        b_values in vec(any::<u64>(), 1..=4),
    ) {
        let run = |a_first: bool| -> BTreeMap<u64, usize> {
            let seed = Known::<u64>::seed();
            let mut a = Peer::new(sync_bootstrap_fork(&seed));
            let mut b = Peer::new(sync_bootstrap_fork(&seed));
            let mut a_keys: Vec<Key> = Vec::new();
            let mut b_keys: Vec<Key> = Vec::new();
            for v in &a_values { a_keys.push(a.insert_one(*v)); }
            for v in &b_values { b_keys.push(b.insert_one(*v)); }
            if a_first {
                a.redact_one(a_keys[0]);
                gossip_step(&mut a, &mut b);
                b.redact_one(b_keys[0]);
            } else {
                b.redact_one(b_keys[0]);
                gossip_step(&mut a, &mut b);
                a.redact_one(a_keys[0]);
            }
            let mut peers = [a, b];
            quiesce(&mut peers);
            readout_multiset(&peers[0].local)
        };
        prop_assert_eq!(run(true), run(false));
    }

    /// Redacting the same `Key` a second time is idempotent: the live
    /// readout is unchanged and no callback fires. (The second redact is a
    /// nil action — the leaf is already gone — so the tree's version is
    /// also unchanged.)
    #[test]
    fn redact_twice_is_idempotent(value in any::<u64>()) {
        let mut peer = Peer::<u64>::new(Known::seed());
        let key = peer.insert_one(value);
        peer.redact_one(key);

        let readout_before = readout_multiset(&peer.local);
        let obs_before = peer.observations.lock().unwrap().len();

        peer.redact_one(key);

        prop_assert_eq!(readout_multiset(&peer.local), readout_before);
        prop_assert_eq!(peer.observations.lock().unwrap().len(), obs_before);
    }

    /// Redacting a `Key` minted on a different peer that this peer
    /// has never observed has no effect on live content and fires
    /// no callback. Pins down the currently implemented behavior so
    /// future regressions surface; the public docs are silent on
    /// this corner.
    #[test]
    fn redact_unknown_key_is_noop(value in any::<u64>()) {
        let seed = Known::<u64>::seed();
        let mut bob = Peer::new(sync_bootstrap_fork(&seed));
        let foreign_key = bob.insert_one(value);

        let mut alice = Peer::new(sync_bootstrap_fork(&seed));
        let readout_before = readout_multiset(&alice.local);
        let obs_before = alice.observations.lock().unwrap().len();

        alice.redact_one(foreign_key);

        prop_assert_eq!(readout_multiset(&alice.local), readout_before);
        prop_assert_eq!(alice.observations.lock().unwrap().len(), obs_before);
    }
}
