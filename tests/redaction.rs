//! Redaction-specific corners.
//!
//! Most redaction behavior is exercised by the multi-peer properties, whose
//! oracle includes redactions and whose exact readout must match every peer.
//! These tests target the redaction-specific corners with smaller,
//! more legible schedules.

mod common;

use std::collections::BTreeMap;

use proptest::collection::vec;
use proptest::prelude::*;
use rumors::{Peer as RumorsPeer, TryTick};

use crate::common::oracle::readout_multiset;
use crate::common::peer::{Peer, gossip_step, quiesce};
use crate::common::wire::bootstrap_fork;

proptest! {
    /// A redaction issued by *any* peer propagates contagiously to
    /// every peer after sufficient gossip — no peer retains the
    /// message and no peer re-introduces it.
    ///
    /// Peer 0 inserts a message and quiesce propagates it everywhere;
    /// then a generated peer (not necessarily peer 0)
    /// issues the redaction; after a final quiesce, every peer's
    /// live multiset is empty.
    #[test]
    fn redaction_propagates_from_any_peer(
        (n_peers, redactor) in (2usize..=6)
            .prop_flat_map(|n_peers| (Just(n_peers), 0..n_peers)),
        value in any::<u64>(),
    ) {
        let seed = rumors::Peer::<u64>::seed().sync_window_floor().into_rumors();
        let mut peers: Vec<Peer<u64>> = (0..n_peers)
            .map(|_| Peer::new(bootstrap_fork(&seed)))
            .collect();

        let version = peers[0].insert_one(value);
        quiesce(&mut peers);

        for peer in &peers {
            let live = readout_multiset(&peer.local.snapshot());
            prop_assert_eq!(live.get(&value).copied(), Some(1));
        }

        peers[redactor].redact_one(&version);
        quiesce(&mut peers);

        for (i, peer) in peers.iter().enumerate() {
            prop_assert!(
                readout_multiset(&peer.local.snapshot()).is_empty(),
                "peer {} still has live messages after redaction by peer {}",
                i, redactor,
            );
        }
    }

    /// Two peers each insert several values, then each redacts one of
    /// its own messages. The converged content is the same regardless of
    /// which side issues its redaction first across the gossip
    /// boundary.
    #[test]
    fn concurrent_redactions_order_independent(
        a_values in vec(any::<u64>(), 1..=4),
        b_values in vec(any::<u64>(), 1..=4),
    ) {
        let run = |a_first: bool| -> BTreeMap<u64, usize> {
            let seed = rumors::Peer::<u64>::seed().sync_window_floor().into_rumors();
            let mut a = Peer::new(bootstrap_fork(&seed));
            let mut b = Peer::new(bootstrap_fork(&seed));
            let mut a_versions: Vec<rumors::Version> = Vec::new();
            let mut b_versions: Vec<rumors::Version> = Vec::new();
            for v in &a_values { a_versions.push(a.insert_one(*v)); }
            for v in &b_values { b_versions.push(b.insert_one(*v)); }
            if a_first {
                a.redact_one(&a_versions[0]);
                gossip_step(&mut a, &mut b);
                b.redact_one(&b_versions[0]);
            } else {
                b.redact_one(&b_versions[0]);
                gossip_step(&mut a, &mut b);
                a.redact_one(&a_versions[0]);
            }
            let mut peers = [a, b];
            quiesce(&mut peers);
            readout_multiset(&peers[0].local.snapshot())
        };
        prop_assert_eq!(run(true), run(false));
    }

    /// Redacting an absent version is a complete no-op, whether the replica
    /// never held it or already redacted it: live content, causal frontier,
    /// and change notifications all remain unchanged.
    ///
    /// Each generated payload exhausts both absent-version states. Empty roots
    /// and roots retaining unrelated content must both remain unchanged.
    #[test]
    fn absent_redactions_are_noops(value in any::<u64>()) {
        for already_redacted in [false, true] {
            for retain_unrelated in [false, true] {
                let seed = RumorsPeer::<u64>::seed().sync_window_floor().into_rumors();
                let subject = bootstrap_fork(&seed);
                let absent = if already_redacted {
                    let version = subject.send(value).unwrap();
                    subject.redact(&version);
                    version
                } else {
                    seed.send(value).unwrap()
                };
                if retain_unrelated {
                    subject.send(u64::MAX).unwrap();
                }

                let before = subject.snapshot();
                let mut changes = subject.changes();
                prop_assert_eq!(changes.try_next(), TryTick::Tick);
                prop_assert_eq!(changes.try_next(), TryTick::Quiet);

                subject.redact(&absent);

                prop_assert_eq!(subject.snapshot(), before);
                prop_assert_eq!(changes.try_next(), TryTick::Quiet);
            }
        }
    }
}
