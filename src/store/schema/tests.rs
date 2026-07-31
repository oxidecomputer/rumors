//! Record encodings round-trip exactly, the decode doors refuse
//! corrupted rows as observable errors, and the allocator's names are
//! unique across blocks, crashes, and concurrent exhaustion.

use before::{Clock, Version};
use borsh::BorshSerialize;
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
    let decoded = NodeRecord::decode(NodeId(0), &record.encode()).expect("round trip");
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
                bounds: Version::new().span(&Version::new()),
                leaves: 5,
                version_bytes: 9,
                children: children
                    .into_iter()
                    .map(|(radix, id)| (radix, NodeId(id), Hash::leaf(&[radix])))
                    .collect(),
            }
        };
        let record = NodeRecord { strong, body };
        prop_assert_eq!(NodeRecord::decode(NodeId(0), &record.encode()).expect("round trip"), record);
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
    let sorted: Vec<_> = keys
        .iter()
        .map(|key| NodeId::from_key(NODES, key).expect("eight bytes"))
        .collect();
    assert_eq!(sorted, ids.map(NodeId).to_vec());
}

/// Held keys split back into the `(node, pin)` that built them.
#[test]
fn held_keys_round_trip() {
    let key = held_key(NodeId(77), PinId(u64::MAX));
    assert_eq!(
        split_held_key(&key).expect("sixteen bytes"),
        (NodeId(77), PinId(u64::MAX))
    );
}

/// A truncated node record refuses as an observable [`Corruption`]
/// naming its table and key — never a panic, and never a value.
///
/// The bytes are a valid record's encoding cut short: exactly what a
/// torn write or a rotted length leaves behind.
#[test]
fn truncated_record_refuses_as_corruption() {
    let record = NodeRecord {
        strong: 2,
        body: NodeBody::Leaf {
            prefix: vec![3, 1, 4],
            version: Version::new(),
            payload: b"payload".to_vec(),
        },
    };
    let whole = record.encode();
    let truncated = &whole[..whole.len() - 1];
    let node = NodeId(42);
    let refusal = NodeRecord::decode(node, truncated).expect_err("truncated bytes must refuse");
    assert_eq!(refusal.table(), NODES);
    assert_eq!(refusal.key(), node.key());
}

/// Byte-layout mirror of a branch [`NodeRecord`] whose bounds are two
/// *independent* versions: what lets the test write a crossed pair the
/// real record's validating span field can never construct.
#[derive(BorshSerialize)]
struct RawBranchRecord {
    strong: u64,
    /// [`NodeBody`]'s borsh enum discriminant; `Branch` is variant 1.
    variant: u8,
    prefix: Vec<u8>,
    hash: Hash,
    /// The bounds field as raw endpoints (a `Span` serializes as the
    /// meet's canonical bytes then the join's, unframed).
    meet: Version,
    join: Version,
    leaves: u64,
    version_bytes: u64,
    children: Vec<(u8, NodeId, Hash)>,
}

/// A stored branch whose bounds pair is crossed (meet strictly above
/// join) refuses as an observable [`Corruption`].
///
/// The span field's validating parse is a decode door like any other,
/// so bounds no write could produce surface as the same error, never a
/// panic and never an unordered span handed to the classifiers.
#[test]
fn crossed_span_record_refuses_as_corruption() {
    let party = before::Party::seed();
    let mut version = Version::new();
    version.tick(&party);
    let below = version.clone();
    version.tick(&party);
    let above = version;

    let raw = RawBranchRecord {
        strong: 1,
        variant: 1,
        prefix: vec![9],
        hash: Hash::leaf(b"x"),
        // Crossed: the stored meet strictly dominates the stored join.
        meet: above,
        join: below,
        leaves: 2,
        version_bytes: 3,
        children: vec![
            (0, NodeId(1), Hash::leaf(&[0])),
            (1, NodeId(2), Hash::leaf(&[1])),
        ],
    };
    let bytes = borsh::to_vec(&raw).expect("the mirror encodes");
    let node = NodeId(7);
    let refusal = NodeRecord::decode(node, &bytes).expect_err("a crossed bounds pair must refuse");
    assert_eq!(refusal.table(), NODES);
    assert_eq!(refusal.key(), node.key());
}

/// The mirror's layout premise holds: an *ordered* pair through the
/// mirror decodes to the real record, so the crossed case above fails
/// on the crossing alone, not on some layout mismatch.
#[test]
fn raw_branch_mirror_matches_record_layout() {
    let party = before::Party::seed();
    let mut version = Version::new();
    version.tick(&party);
    let below = version.clone();
    version.tick(&party);
    let above = version;

    let raw = RawBranchRecord {
        strong: 1,
        variant: 1,
        prefix: vec![9],
        hash: Hash::leaf(b"x"),
        meet: below.clone(),
        join: above.clone(),
        leaves: 2,
        version_bytes: 3,
        children: vec![
            (0, NodeId(1), Hash::leaf(&[0])),
            (1, NodeId(2), Hash::leaf(&[1])),
        ],
    };
    let bytes = borsh::to_vec(&raw).expect("the mirror encodes");
    let decoded = NodeRecord::decode(NodeId(7), &bytes).expect("an ordered pair decodes");
    let NodeBody::Branch { bounds, .. } = &decoded.body else {
        unreachable!("variant 1 is Branch");
    };
    assert_eq!(bounds.meet().as_bytes(), below.as_bytes());
    assert_eq!(bounds.join().as_bytes(), above.as_bytes());
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
