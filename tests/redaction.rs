//! Redaction-specific corners.
//!
//! Most redaction behavior is exercised by the multi-peer properties, whose
//! oracle includes redactions and whose exact readout must match every peer.
//! These tests target the redaction-specific corners with smaller, more legible
//! schedules.

use rumors_testkit::common;

use std::collections::{BTreeMap, BTreeSet};

use proptest::collection::vec;
use proptest::prelude::*;
use rumors::{EncodeError, TryTick, Version};

use crate::common::oracle::readout_multiset;
use crate::common::peer::{Peer, gossip_step, quiesce};
use crate::common::wire::bootstrap_fork;
use rumors_testkit::common::property::{FLEET_DIVISOR, SESSION_DIVISOR};
use rumors_testkit::common::seed::network_seed;
use rumors_testkit::divided_proptest;

divided_proptest! {
    #![divisor(FLEET_DIVISOR)]

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
        network in network_seed(),
    ) {
        let seed = network.peer::<u64>().sync_memory_budget(0).into_rumors();
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
}

divided_proptest! {
    #![divisor(SESSION_DIVISOR)]

    /// Two peers each insert several values, then each redacts one of
    /// its own messages. The converged content is the same regardless of
    /// which side issues its redaction first across the gossip
    /// boundary.
    #[test]
    fn concurrent_redactions_order_independent(
        a_values in vec(any::<u64>(), 1..=4),
        b_values in vec(any::<u64>(), 1..=4),
        network in network_seed(),
    ) {
        let run = |a_first: bool| -> BTreeMap<u64, usize> {
            let seed = network.peer::<u64>().sync_memory_budget(0).into_rumors();
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
    /// never held it or already redacted it.
    ///
    /// The redaction reports that it removed nothing, and live content,
    /// causal frontier, and change notifications all remain unchanged.
    ///
    /// Each generated payload exhausts both absent-version states. Empty roots
    /// and roots retaining unrelated content must both remain unchanged.
    #[test]
    fn absent_redactions_are_noops(value in any::<u64>(), network in network_seed()) {
        for already_redacted in [false, true] {
            for retain_unrelated in [false, true] {
                let seed = network.peer::<u64>().sync_memory_budget(0).into_rumors();
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

                prop_assert!(!subject.redact(&absent), "redacting an absent version reports nothing removed");

                prop_assert_eq!(subject.snapshot(), before);
                prop_assert_eq!(changes.try_next(), TryTick::Quiet);
            }
        }
    }
}

/// How a redaction request reaches a replica: one call per version, one
/// `redact_all`, or a batch that also sends around its redactions.
#[derive(Debug, Clone, Copy)]
enum Redacting {
    /// One `Rumors::redact` per requested version, in order.
    OneByOne,
    /// One `Rumors::redact_all` of the whole request.
    All,
    /// One `Rumors::batch` sending a message, redacting the request, and
    /// sending another.
    Batch,
}

divided_proptest! {
    #![divisor(SESSION_DIVISOR)]

    /// Redaction reports exactly what it removed, through every entry
    /// point: the distinct requested versions the replica held live.
    ///
    /// The replica holds some messages, has redacted others, and lacks
    /// messages only another replica of its network holds; a request draws
    /// from all three, repeats included. `redact` returns whether each call
    /// removed its message, `redact_all` and a batch's `Committed::redacted`
    /// count the removals, and afterwards the replica holds none of the
    /// requested versions and every held message nobody requested.
    #[test]
    fn redaction_reports_what_it_removed(
        held in 1usize..6,
        already_redacted in 0usize..4,
        elsewhere in 0usize..4,
        requests in vec((0usize..3, any::<prop::sample::Index>()), 0..12),
        redacting in prop_oneof![
            Just(Redacting::OneByOne),
            Just(Redacting::All),
            Just(Redacting::Batch),
        ],
        network in network_seed(),
    ) {
        let seed = network.peer::<u64>().sync_memory_budget(0).into_rumors();
        let subject = bootstrap_fork(&seed);
        let held: Vec<Version> = (0..held as u64).map(|i| subject.send(i).unwrap()).collect();
        let already_redacted: Vec<Version> = (0..already_redacted as u64)
            .map(|i| {
                let version = subject.send(100 + i).unwrap();
                assert!(subject.redact(&version));
                version
            })
            .collect();
        // Sent on the seed after the fork, so the subject never receives them.
        let elsewhere: Vec<Version> =
            (0..elsewhere as u64).map(|i| seed.send(200 + i).unwrap()).collect();
        let pools = [&held, &already_redacted, &elsewhere];
        let request: Vec<Version> = requests
            .iter()
            .filter_map(|(pool, index)| {
                let pool = pools[*pool];
                (!pool.is_empty()).then(|| pool[index.index(pool.len())].clone())
            })
            .collect();
        let removed: BTreeSet<&[u8]> = request
            .iter()
            .filter(|version| held.contains(version))
            .map(Version::as_bytes)
            .collect();

        match redacting {
            Redacting::OneByOne => {
                let mut count = 0;
                for version in &request {
                    let held_now = subject.snapshot().contains(version);
                    let reported = subject.redact(version);
                    prop_assert_eq!(reported, held_now, "redact reports whether it removed");
                    count += usize::from(reported);
                }
                prop_assert_eq!(count, removed.len());
            }
            Redacting::All => prop_assert_eq!(subject.redact_all(&request), removed.len()),
            Redacting::Batch => {
                let committed = subject
                    .batch(|batch| {
                        batch.send(1_000)?;
                        batch.redact_all(&request);
                        batch.send(1_001)?;
                        Ok::<_, EncodeError>(())
                    })
                    .unwrap();
                prop_assert_eq!(committed.redacted, removed.len());
            }
        }

        let snapshot = subject.snapshot();
        for version in &request {
            prop_assert!(!snapshot.contains(version), "a requested version stays redacted");
        }
        for version in held.iter().filter(|version| !removed.contains(version.as_bytes())) {
            prop_assert!(snapshot.contains(version), "an unrequested message stays live");
        }
    }
}
