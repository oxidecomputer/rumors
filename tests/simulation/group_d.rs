//! Group D — redaction semantics.
//!
//! Most of redaction is exercised generically by Group C (`C2` in
//! particular: the oracle bakes in `redact` events, and every peer's
//! readout must match). These tests target the redaction-specific
//! corners with smaller, more legible schedules.

use proptest::collection::vec;
use proptest::prelude::*;
use rumors::Key;

use crate::oracle::readout_multiset;
use crate::peer::{Peer, gossip_step, quiesce};

proptest! {
    /// D1: a redaction issued by *any* peer propagates contagiously
    /// to *every* peer after sufficient gossip — no peer retains the
    /// message and no peer re-introduces it.
    ///
    /// Schedule: peer 0 inserts a message, the message propagates to
    /// every other peer via quiesce, then a *different* peer (chosen
    /// by `redactor_idx`) issues the redaction. After a final quiesce,
    /// every peer's live multiset is empty.
    #[test]
    fn d1_contagion_from_arbitrary_peer(
        n_peers in 2usize..=6,
        value in any::<u64>(),
        redactor_idx in any::<usize>(),
    ) {
        let mut peers: Vec<Peer<u64>> = (0..n_peers)
            .map(|i| Peer::<u64>::new(format!("p{i}")))
            .collect();

        // Peer 0 inserts; quiesce so everyone learns it.
        let key = peers[0].insert_one(value);
        quiesce(&mut peers);

        // Sanity: every peer now has exactly one live value.
        for peer in &peers {
            let live = readout_multiset(&peer.local);
            prop_assert_eq!(live.get(&value).copied(), Some(1));
        }

        // Any peer redacts it (not necessarily peer 0).
        let r = redactor_idx % n_peers;
        peers[r].redact_one(key);
        quiesce(&mut peers);

        // Every peer's live multiset is empty.
        for (i, peer) in peers.iter().enumerate() {
            prop_assert!(
                readout_multiset(&peer.local).is_empty(),
                "peer {} still has live messages after redaction by peer {}",
                i, r,
            );
        }
    }

    /// D2: redaction is order-independent across concurrent peers.
    /// Two peers, each inserts then redacts, in interleaved order;
    /// both orderings of gossip produce the same converged state.
    #[test]
    fn d2_order_independent_redactions(
        a_values in vec(any::<u64>(), 1..=4),
        b_values in vec(any::<u64>(), 1..=4),
        redact_a_first in any::<bool>(),
    ) {
        let run = |a_first: bool| -> Vec<u64> {
            let mut a = Peer::<u64>::new("alice");
            let mut b = Peer::<u64>::new("bob");
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
                .into_iter()
                .flat_map(|(v, c)| std::iter::repeat(v).take(c))
                .collect()
        };
        let forward = {
            let mut v = run(redact_a_first);
            v.sort();
            v
        };
        let reverse = {
            let mut v = run(!redact_a_first);
            v.sort();
            v
        };
        prop_assert_eq!(forward, reverse);
    }

}
