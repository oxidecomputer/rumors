//! Record encodings round-trip exactly, and the allocator's names are
//! unique across blocks, crashes, and concurrent exhaustion.

use before::{Clock, Version};
use proptest::prelude::*;

use super::*;
use crate::store::Memory;
use crate::tree::typed::Hash;

/// A `Version` stored in a record round-trips to the same canonical
/// bytes: the store never perturbs what content addressing hashed.
#[test]
fn version_passthrough_is_exact() {
    let party = before::Party::seed();
    let mut version = Version::new();
    for _ in 0..17 {
        version.tick(&party);
    }
    let record = NodeRecord {
        strong: 3,
        body: NodeBody::Leaf {
            prefix: vec![7, 9],
            version: version.clone(),
            payload: b"payload".to_vec(),
        },
    };
    let decoded = NodeRecord::decode(&record.encode());
    assert_eq!(decoded, record);
    let NodeBody::Leaf {
        version: reloaded, ..
    } = decoded.body
    else {
        unreachable!("leaf decoded as branch");
    };
    assert_eq!(reloaded.as_bytes(), version.as_bytes());
}

proptest! {
    /// Every record shape round-trips through its row encoding: what the
    /// custody layer writes is exactly what a later fetch decodes.
    #[test]
    fn node_records_round_trip(
        strong in 0u64..1 << 40,
        prefix in proptest::collection::vec(any::<u8>(), 0..32),
        payload in proptest::collection::vec(any::<u8>(), 0..256),
        children in proptest::collection::btree_map(any::<u8>(), any::<u64>(), 2..40),
        leaf in any::<bool>(),
    ) {
        let body = if leaf {
            NodeBody::Leaf {
                prefix: prefix.clone(),
                version: Version::new(),
                payload,
            }
        } else {
            NodeBody::Branch {
                prefix,
                hash: Hash::leaf(b"x"),
                ceiling: Version::new(),
                floor: Version::new(),
                leaves: 5,
                version_bytes: 9,
                children: children
                    .into_iter()
                    .map(|(radix, id)| (radix, NodeId(id), Hash::leaf(&[radix])))
                    .collect(),
            }
        };
        let record = NodeRecord { strong, body };
        prop_assert_eq!(NodeRecord::decode(&record.encode()), record);
    }

    /// The canonical-root record round-trips, absent identity included:
    /// "holds no identity" is a stored fact, not an absent row.
    #[test]
    fn canonical_root_round_trips(root in proptest::option::of(any::<u64>()), with_identity in any::<bool>()) {
        let identity = with_identity.then(|| {
            let party = before::Party::seed();
            let mut version = Version::new();
            version.tick(&party);
            Clock::from_parts(party, version).encode()
        });
        let record = CanonicalRoot {
            network: Some(crate::Network::from_rng(&mut rand::rngs::OsRng)),
            ceiling: Version::new(),
            root: root.map(NodeId),
            identity,
        };
        let encoded = borsh::to_vec(&record).unwrap();
        let decoded: CanonicalRoot = borsh::from_slice(&encoded).unwrap();
        prop_assert_eq!(decoded, record);
    }
}

/// Node-ID row keys order numerically: a table scan walks IDs in
/// allocation order.
#[test]
fn id_keys_order_numerically() {
    let ids = [0u64, 1, 255, 256, 1 << 20, u64::MAX];
    let mut keys: Vec<_> = ids.iter().map(|&id| NodeId(id).key()).collect();
    keys.sort();
    let sorted: Vec<_> = keys.iter().map(|key| NodeId::from_key(key)).collect();
    assert_eq!(sorted, ids.map(NodeId).to_vec());
}

/// Held keys split back into the `(node, pin)` that built them.
#[test]
fn held_keys_round_trip() {
    let key = held_key(NodeId(77), PinId(u64::MAX));
    assert_eq!(split_held_key(&key), (NodeId(77), PinId(u64::MAX)));
}

/// Allocation never repeats an ID: not within a block, not across the
/// block boundary, and not across a crash that wastes a block's
/// remainder (the reopened allocator starts a fresh block above
/// everything ever handed out).
#[pollster::test]
async fn allocation_is_unique_across_blocks_and_crashes() {
    let store = Memory::recording();
    let allocator = IdAllocator::default();
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..3 {
        assert!(seen.insert(allocator.allocate(&store).await.unwrap()));
    }
    // A crash: everything in-process is lost, the store survives.
    let reopened = store.reopen_at(store.history_len() - 1);
    let fresh = IdAllocator::default();
    for _ in 0..3 {
        assert!(seen.insert(fresh.allocate(&reopened).await.unwrap()));
    }
    // Two allocators on one store (never sound in production — the
    // backend owns its store single-process — but exactly the shape two
    // concurrent reservation transactions must survive): blocks are
    // disjoint because reservation is one serializable read-modify-write.
    let rival = IdAllocator::default();
    for _ in 0..3 {
        assert!(seen.insert(rival.allocate(&reopened).await.unwrap()));
    }
}
