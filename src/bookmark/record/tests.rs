//! Retention may lose recovery rights, but never their write requirements;
//! reclamation takes every right the frontier has caught up with.

use before::Clock;
use proptest::prelude::*;

use super::*;

/// Construct a network with disjoint identities of varying depth and nonempty writes.
fn network_record(forks: usize, ticks: u64) -> NetworkRecord {
    let (mut party, mut version) = Clock::seed().into_parts();
    let mut record = NetworkRecord::default();
    for _ in 0..forks {
        let fork = party.fork();
        version.ticks(&fork, ticks);
        record.record(&fork, &version);
    }
    version.ticks(&party, ticks);
    record.record(&party, &version);
    record
}

proptest! {
    /// Serialization preserves every network's and identity's recency, including
    /// after either level is refreshed repeatedly.
    #[test]
    fn serialization_preserves_both_recency_orders(
        entries in proptest::collection::vec((1usize..20, 1u64..20), 2..10),
        touches in proptest::collection::vec((any::<usize>(), any::<usize>()), 0..40),
    ) {
        let mut record = Record::default();
        let keys: Vec<_> = (0..entries.len())
            .map(|i| Network::from_bytes((i as u128).to_be_bytes())).collect();
        for (&key, &(forks, ticks)) in keys.iter().zip(&entries) {
            assert!(record.networks.insert(key, network_record(forks, ticks)));
        }
        for (network, identity) in touches {
            let entry = record.networks.get(&keys[network % keys.len()]).unwrap();
            let identity = entry.identities[identity % entry.identities.len()].dangerously_alias();
            entry.record(&identity, &entry.written.clone());
        }
        let restored = format::decode(&format::encode(&record)).unwrap();
        prop_assert_eq!(record.networks.len(), restored.networks.len());
        for ((key, entry), (decoded_key, decoded_entry)) in
            record.networks.iter().zip(restored.networks.iter())
        {
            prop_assert_eq!(key, decoded_key);
            prop_assert_eq!(&entry.identities, &decoded_entry.identities);
            prop_assert_eq!(&entry.written, &decoded_entry.written);
        }
    }

    /// A budget needing just one eviction removes only the oldest identity
    /// of the oldest network, preserving its other identities and newer networks.
    #[test]
    fn one_eviction_keeps_the_rest_of_the_oldest_network(
        entries in proptest::collection::vec((2usize..25, 1u64..20), 1..6),
    ) {
        let mut record = Record::default();
        for (index, (forks, ticks)) in entries.into_iter().enumerate() {
            let key = Network::from_bytes((index as u128).to_be_bytes());
            assert!(record.networks.insert(key, network_record(forks, ticks)));
        }
        let before = format::encode(&record);
        let mut expected = format::decode(&before).unwrap();
        let oldest = *expected.networks.peek_oldest().unwrap().0;
        let network = expected.networks.peek_mut(&oldest).unwrap();
        network.identities.pop_front();
        network.compact();
        let expected = format::encode(&expected);
        prop_assert!(
            expected.len() < before.len(),
            "removing one retained identity must reduce the encoded record"
        );
        let actual = record.bounded_bytes(expected.len());
        prop_assert_eq!(actual, expected);
    }

    /// Any byte limit retains a prefix of network recency and a suffix of each
    /// surviving identity list, without weakening any surviving write requirement.
    #[test]
    fn retention_preserves_recency_and_requirements(
        entries in proptest::collection::vec((0usize..40, 1u64..20), 1..10),
        touches in proptest::collection::vec(any::<usize>(), 0..20),
        limit in 0usize..3000,
    ) {
        let mut record = Record::default();
        let keys: Vec<_> = (0..entries.len()).map(|i| Network::from_bytes((i as u128).to_be_bytes())).collect();
        for (&key, &(forks, ticks)) in keys.iter().zip(&entries) {
            assert!(record.networks.insert(key, network_record(forks, ticks)));
        }
        for touch in touches { record.networks.get(&keys[touch % keys.len()]); }
        let original: Vec<_> = record.networks.iter().map(|(&key, value)| (
            key, value.written.clone(), value.identities.iter().map(Party::encode).collect::<Vec<_>>()
        )).collect();
        let original_bytes = format::encode(&record);
        // Exercise restored recency, not only the in-process ordering.
        record = format::decode(&original_bytes).unwrap();
        let bytes = record.bounded_bytes(limit.max(format::record_size(0, 0)));
        prop_assert!(bytes.len() <= limit.max(format::record_size(0, 0)));
        prop_assert_eq!(&format::encode(&format::decode(&bytes).unwrap()), &bytes);
        if original_bytes.len() <= limit { prop_assert_eq!(bytes, original_bytes); }
        for ((key, value), (expected, written, identities)) in record.networks.iter().zip(&original) {
            prop_assert_eq!(key, expected);
            let actual: Vec<_> = value.identities.iter().map(Party::encode).collect();
            prop_assert!(identities.ends_with(&actual));
            for identity in &value.identities {
                prop_assert_eq!(&value.written / identity, written / identity);
            }
        }
        prop_assert!(record.networks.len() <= original.len());
    }

    /// Refreshing an identity moves it to the newest end without forgetting writes.
    #[test]
    fn recording_refreshes_identity_recency(forks in 1usize..30, index: usize, ticks in 1u64..20) {
        let mut record = network_record(forks, ticks);
        let selected = record.identities[index % record.identities.len()].dangerously_alias();
        let previous = record.written.clone();
        let mut next = previous.clone();
        next.tick(&selected);
        record.record(&selected, &next);
        prop_assert_eq!(record.identities.back(), Some(&selected));
        prop_assert_eq!(record.identities.len(), forks + 1);
        prop_assert!(record.written >= previous);
        prop_assert_eq!(&record.written / &selected, &next / &selected);
    }

    /// Removing rights also removes their unneeded frontier, while all retained
    /// regions keep exactly their recovery requirement.
    #[test]
    fn compaction_forgets_only_unowned_progress(
        forks in 1usize..30, ticks in 1u64..20, keep in any::<usize>(),
    ) {
        let mut record = network_record(forks, ticks);
        let before = record.written.clone();
        let keep = keep % record.identities.len();
        let removed: Vec<_> = record.identities.drain(..keep).collect();
        record.compact();
        for identity in &record.identities {
            prop_assert_eq!(&record.written / identity, &before / identity);
        }
        for identity in &removed { prop_assert!((&record.written / identity).to_version().is_empty()); }
    }
}

/// Compare predicted and actual frame sizes for repeated byte-string items.
fn assert_record_size(networks: usize, item_bytes: usize) {
    let item = ciborium::value::Value::Bytes(vec![0; item_bytes]);
    let mut encoded_item = Vec::new();
    ciborium::ser::into_writer(&item, &mut encoded_item).unwrap();
    let mut encoded = Vec::new();
    ciborium::ser::into_writer(&vec![item; networks], &mut encoded).unwrap();
    assert_eq!(
        format::record_size(networks, networks * encoded_item.len()),
        format::frame(&encoded).len(),
    );
}

/// The size used for eviction includes each CBOR header-width transition.
///
/// Network counts and item sizes are independent in the formula, so varying
/// each through its boundaries avoids constructing huge Cartesian products.
#[test]
fn byte_accounting_matches_encoding_at_header_boundaries() {
    for networks in [0, 1, 23, 24, 255, 256] {
        for item_bytes in [0, 1, 23, 24, 255, 256] {
            assert_record_size(networks, item_bytes);
        }
    }
    for item_bytes in [65_535, 65_536] {
        assert_record_size(1, item_bytes);
    }
}

/// Join aliases of the pieces `selected` picks into one identity, or `None`
/// when it picks nothing.
fn union(pieces: &[Party], selected: &[bool]) -> Option<Party> {
    let mut chosen = pieces
        .iter()
        .zip(selected)
        .filter(|(_, picked)| **picked)
        .map(|(piece, _)| piece.dangerously_alias());
    let mut identity = chosen.next()?;
    for piece in chosen {
        identity
            .join(piece)
            .expect("the pieces are pairwise disjoint");
    }
    Some(identity)
}

proptest! {
    /// Reclamation is total: a checkpoint with the gate open joins every
    /// identity the frontier is caught up on, however the identities overlap
    /// and whatever their order.
    ///
    /// The live party ends as its start joined with each such identity, and
    /// only identities that are not caught up remain, in their original
    /// order. Identities are unions of disjoint pieces, each recorded at a
    /// frontier that advances every piece it contains, so an identity is
    /// caught up exactly when the restart has learned every piece in it.
    #[test]
    fn reclaim_consumes_every_caught_up_identity(
        piece_count in 1usize..=6,
        shapes in proptest::collection::vec(
            (proptest::collection::vec(any::<bool>(), 6), 1u64..6),
            1..8,
        ),
        learned in proptest::collection::vec(any::<bool>(), 6),
        rotation in any::<usize>(),
    ) {
        let mut restart = Party::seed();
        let mut owner = restart.fork();
        let mut pieces: Vec<Party> = owner.forks((piece_count - 1) as u64).collect();
        pieces.push(owner);
        let learned = &learned[..piece_count];

        // Record each distinct shape once, in a growing frontier.
        let mut record = NetworkRecord::default();
        let mut frontier = Version::new();
        let mut recorded: Vec<(Party, Vec<bool>)> = Vec::new();
        for (mask, ticks) in &shapes {
            let mask = &mask[..piece_count];
            if recorded.iter().any(|(_, seen)| seen == mask) {
                continue;
            }
            let Some(identity) = union(&pieces, mask) else { continue };
            for (piece, _) in pieces.iter().zip(mask).filter(|(_, picked)| **picked) {
                frontier.ticks(piece, *ticks);
            }
            record.record(&identity, &frontier);
            recorded.push((identity, mask.to_vec()));
        }
        prop_assume!(!recorded.is_empty());
        let rotation = rotation % recorded.len();
        record.identities.rotate_left(rotation);
        recorded.rotate_left(rotation);

        // The restart attaches with a fresh party. It has learned the writes
        // of the learned pieces and nothing else in the owner's region.
        let mut known = Version::new();
        known.tick(&restart);
        for (piece, _) in pieces.iter().zip(learned).filter(|(_, learned)| **learned) {
            known |= &(&record.written / piece).to_version();
        }
        record.record(&restart, &known);

        let caught_up = |mask: &[bool]| {
            mask.iter().zip(learned).all(|(picked, learned)| !picked || *learned)
        };
        let mut expected = restart.dangerously_alias();
        for (index, piece) in pieces.iter().enumerate() {
            if recorded.iter().any(|(_, mask)| mask[index] && caught_up(mask)) {
                expected.join(piece.dangerously_alias()).expect("the pieces avoid the restart");
            }
        }
        let expected_remaining: Vec<&Party> = recorded
            .iter()
            .filter(|(_, mask)| !caught_up(mask))
            .map(|(identity, _)| identity)
            .collect();

        record.reclaim(&mut restart, &known);
        prop_assert_eq!(&restart, &expected);
        prop_assert_eq!(record.identities.iter().collect::<Vec<_>>(), expected_remaining);
    }
}
