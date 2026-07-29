//! Custody keeps the liveness invariant at every committed prefix.
//!
//! The centerpiece is the crash-point sweep: a generated op sequence runs
//! against a recording [`Memory`], and then *every* committed prefix is
//! reopened as a crash survivor, recovered, vacuumed, and audited — the
//! full-scan audit recomputes the strong counts from the edges, chases
//! every reference, and checks the identity record against the model.

use std::collections::{BTreeMap, BTreeSet};

use before::{Clock, Version};
use proptest::prelude::*;

use super::*;
use crate::store::schema::{IDS_KEY, IdAllocator, META, NodeBody, ROOT_KEY};
use crate::store::{Kv, Memory, Table};
use crate::tree::typed::Hash;

/// Any network identifier: the custody layer stores it opaquely, so one
/// fresh draw serves every test here.
fn test_network() -> crate::Network {
    crate::Network::from_rng(&mut rand::rngs::OsRng)
}

/// A distinct, inert identity record for model comparison: the encoding
/// of `n`+1 ticks of a fresh party. Stored and compared as bytes, never
/// decoded back to a live clock, so the one-universe safety rule is
/// untouched.
fn clock_tag(n: usize) -> Vec<u8> {
    let party = before::Party::seed();
    let mut version = Version::new();
    for _ in 0..=n {
        version.tick(&party);
    }
    Clock::from_parts(party, version).encode()
}

/// A leaf record body with `strong` prepared by the caller's edges.
fn leaf_record(strong: u64, payload: &[u8]) -> NodeRecord {
    NodeRecord {
        strong,
        body: NodeBody::Leaf {
            prefix: Vec::new(),
            version: Version::new(),
            payload: payload.to_vec(),
        },
    }
}

/// A branch record over `children`, radixes assigned in order.
fn branch_record(children: &[NodeId]) -> NodeRecord {
    NodeRecord {
        strong: 0,
        body: NodeBody::Branch {
            prefix: Vec::new(),
            hash: Hash::leaf(b"branch"),
            ceiling: Version::new(),
            floor: Version::new(),
            leaves: children.len() as u64,
            version_bytes: 1,
            children: children
                .iter()
                .enumerate()
                .map(|(radix, &id)| (radix as u8, id, Hash::leaf(&[radix as u8])))
                .collect(),
        },
    }
}

/// Every entry of `table`, by full cursor walk.
async fn scan(store: &Memory, table: Table) -> Vec<(Vec<u8>, Vec<u8>)> {
    store
        .read(move |txn| {
            let mut entries = Vec::new();
            let mut cursor = None;
            while let Some((key, value)) = txn.next_after(table, cursor.as_deref())? {
                entries.push((key.clone(), value));
                cursor = Some(key);
            }
            Ok(entries)
        })
        .await
        .unwrap()
}

/// The full-scan audit: internal consistency of counts, references, and
/// (after recovery + vacuum) the reachable-set equality; the identity
/// record against the model's expectation.
async fn audit(store: &Memory, expected_identity: Option<Vec<u8>>, quiesced: bool) {
    let nodes: BTreeMap<NodeId, NodeRecord> = scan(store, NODES)
        .await
        .into_iter()
        .map(|(key, value)| (NodeId::from_key(&key), NodeRecord::decode(&value)))
        .collect();
    let root = store.read(|txn| CanonicalRoot::read(txn)).await.unwrap();

    // Strong counts equal recomputed durable edges: parent links plus the
    // canonical-root edge.
    let mut edges: BTreeMap<NodeId, u64> = BTreeMap::new();
    if let Some(id) = root.root {
        *edges.entry(id).or_default() += 1;
    }
    for record in nodes.values() {
        for child in record.children() {
            *edges.entry(child).or_default() += 1;
        }
    }
    for (&id, record) in &nodes {
        assert_eq!(
            record.strong,
            edges.get(&id).copied().unwrap_or_default(),
            "strong count of {id:?} diverges from its recomputed edges"
        );
    }

    // No dangling references: every edge and the root resolve, and every
    // held row registers an existing node.
    for (&id, record) in &nodes {
        for child in record.children() {
            assert!(
                nodes.contains_key(&child),
                "{id:?} references absent {child:?}"
            );
        }
    }
    if let Some(id) = root.root {
        assert!(nodes.contains_key(&id), "canonical root {id:?} is absent");
    }
    for (key, _) in scan(store, HELD).await {
        let (node, _) = split_held_key(&key);
        assert!(
            nodes.contains_key(&node),
            "held row registers absent {node:?}"
        );
    }

    // The identity record is exactly the model's last committed
    // write-or-clear.
    assert_eq!(root.identity, expected_identity, "identity record diverges");

    if quiesced {
        // Recovery swept every registration and reclamation drained: the
        // store holds exactly the canonical tree.
        assert!(
            scan(store, HELD).await.is_empty(),
            "held rows survived recovery"
        );
        assert!(scan(store, GC).await.is_empty(), "GC queue survived vacuum");
        let mut reachable = BTreeSet::new();
        let mut frontier: Vec<NodeId> = root.root.into_iter().collect();
        while let Some(id) = frontier.pop() {
            if reachable.insert(id) {
                frontier.extend(nodes[&id].children());
            }
        }
        assert_eq!(
            nodes.keys().copied().collect::<BTreeSet<_>>(),
            reachable,
            "storage differs from the reachable closure of the canonical root"
        );
    }
}

/// One op of the generated custody workload.
#[derive(Debug, Clone)]
enum Op {
    /// Install a fresh pinned leaf.
    Leaf,
    /// Install a fresh pinned branch over 2–4 currently-pinned nodes,
    /// bumping each child's strong count in the same transaction.
    Branch { picks: Vec<prop::sample::Index> },
    /// Queue the release of one live pin.
    Release { pick: prop::sample::Index },
    /// One flush transaction of queued releases.
    Flush,
    /// One bounded reclamation transaction.
    Reclaim,
    /// Flush and reclaim to quiescence.
    Vacuum,
    /// Flip the canonical root to a pinned node (or empty), with or
    /// without an identity record.
    FlipRoot {
        pick: Option<prop::sample::Index>,
        identity: bool,
    },
    /// Rewrite only the identity record (the party-shrink write), or
    /// clear it.
    RecordIdentity { identity: bool },
}

fn op_strategy() -> impl Strategy<Value = Op> {
    prop_oneof![
        3 => Just(Op::Leaf),
        2 => proptest::collection::vec(any::<prop::sample::Index>(), 2..=4)
            .prop_map(|picks| Op::Branch { picks }),
        3 => any::<prop::sample::Index>().prop_map(|pick| Op::Release { pick }),
        2 => Just(Op::Flush),
        2 => Just(Op::Reclaim),
        1 => Just(Op::Vacuum),
        2 => (proptest::option::of(any::<prop::sample::Index>()), any::<bool>())
            .prop_map(|(pick, identity)| Op::FlipRoot { pick, identity }),
        1 => any::<bool>().prop_map(|identity| Op::RecordIdentity { identity }),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 48, ..ProptestConfig::default() })]

    /// After surviving any committed prefix of any custody workload,
    /// recovery and vacuum restore the liveness invariant exactly.
    ///
    /// Strong counts equal recomputed edges, nothing dangles, storage
    /// equals the canonical root's reachable closure, and the identity
    /// record is the last committed write-or-clear.
    #[test]
    fn every_committed_prefix_recovers_consistent(ops in proptest::collection::vec(op_strategy(), 1..40)) {
        pollster::block_on(async move {
            let store = Memory::recording();
            let allocator = IdAllocator::default();
            let queue = ReleaseQueue::default();

            // The model: live pins, and the identity expectation per
            // committed history index (checkpointed after every op).
            let mut pins: Vec<(NodeId, PinId)> = Vec::new();
            let mut identity: Option<Vec<u8>> = None;
            let mut identity_seq = 0usize;
            let mut checkpoints: Vec<(usize, Option<Vec<u8>>)> = vec![(store.history_len(), None)];

            for op in ops {
                match op {
                    Op::Leaf => {
                        let node = NodeId(allocator.allocate(&store).await.unwrap());
                        let pin = PinId(allocator.allocate(&store).await.unwrap());
                        let record = leaf_record(0, b"leaf");
                        store
                            .write(move |txn| install(txn, node, pin, &record))
                            .await
                            .unwrap();
                        pins.push((node, pin));
                    }
                    Op::Branch { picks } => {
                        if pins.is_empty() {
                            continue;
                        }
                        // Distinct children, drawn from live pins.
                        let children: Vec<NodeId> = {
                            let mut seen = BTreeSet::new();
                            picks
                                .iter()
                                .map(|pick| pins[pick.index(pins.len())].0)
                                .filter(|id| seen.insert(*id))
                                .collect()
                        };
                        if children.len() < 2 {
                            continue;
                        }
                        let node = NodeId(allocator.allocate(&store).await.unwrap());
                        let pin = PinId(allocator.allocate(&store).await.unwrap());
                        let record = branch_record(&children);
                        store
                            .write(move |txn| {
                                install(txn, node, pin, &record)?;
                                for &child in &children {
                                    adjust_strong(txn, child, 1)?;
                                }
                                Ok(())
                            })
                            .await
                            .unwrap();
                        pins.push((node, pin));
                    }
                    Op::Release { pick } => {
                        if pins.is_empty() {
                            continue;
                        }
                        let (node, pin) = pins.swap_remove(pick.index(pins.len()));
                        queue.push(node, pin);
                    }
                    Op::Flush => {
                        let batch = queue.take();
                        if !batch.is_empty() {
                            let applied = batch.clone();
                            store
                                .write(move |txn| release(txn, &applied))
                                .await
                                .unwrap();
                        }
                    }
                    Op::Reclaim => {
                        store.write(|txn| reclaim_step(txn)).await.unwrap();
                    }
                    Op::Vacuum => {
                        vacuum(&store, &queue).await.unwrap();
                    }
                    Op::FlipRoot { pick, identity: with_identity } => {
                        let root = pick
                            .filter(|_| !pins.is_empty())
                            .map(|pick| pins[pick.index(pins.len())].0);
                        identity = with_identity.then(|| {
                            identity_seq += 1;
                            clock_tag(identity_seq)
                        });
                        let record = identity.clone();
                        store
                            .write(move |txn| flip_root(txn, test_network(), root, Version::new(), record.clone()))
                            .await
                            .unwrap();
                    }
                    Op::RecordIdentity { identity: with_identity } => {
                        identity = with_identity.then(|| {
                            identity_seq += 1;
                            clock_tag(identity_seq)
                        });
                        let record = identity.clone();
                        store
                            .write(move |txn| record_identity(txn, record.clone()))
                            .await
                            .unwrap();
                    }
                }
                checkpoints.push((store.history_len(), identity.clone()));
            }

            // Every committed prefix is a crash survivor: recover, drain,
            // audit.
            for prefix in 0..store.history_len() {
                let survivor = store.reopen_at(prefix);
                recover(&survivor).await.unwrap();
                vacuum(&survivor, &ReleaseQueue::default()).await.unwrap();
                let expected = checkpoints
                    .iter()
                    .rfind(|(len, _)| *len <= prefix + 1)
                    .map(|(_, identity)| identity.clone())
                    .unwrap_or_default();
                audit(&survivor, expected, true).await;
            }
        });
    }
}

/// A release cascade of any depth stays bounded per transaction: a chain
/// of branches reclaims fully, but only through repeated bounded steps.
#[pollster::test]
async fn cascades_are_queued_not_inlined() {
    let store = Memory::recording();
    let allocator = IdAllocator::default();
    let queue = ReleaseQueue::default();

    // A chain: leaf <- b0 <- b1 <- ... requires 2 children per branch, so
    // pair each level with a fresh leaf sibling.
    let mut level = {
        let node = NodeId(allocator.allocate(&store).await.unwrap());
        let pin = PinId(allocator.allocate(&store).await.unwrap());
        let record = leaf_record(0, b"base");
        store
            .write(move |txn| install(txn, node, pin, &record))
            .await
            .unwrap();
        queue.push(node, pin);
        node
    };
    for _ in 0..GC_BUDGET + 3 {
        let sibling = NodeId(allocator.allocate(&store).await.unwrap());
        let sibling_pin = PinId(allocator.allocate(&store).await.unwrap());
        let parent = NodeId(allocator.allocate(&store).await.unwrap());
        let parent_pin = PinId(allocator.allocate(&store).await.unwrap());
        let leaf = leaf_record(0, b"sibling");
        let branch = branch_record(&[level, sibling]);
        store
            .write(move |txn| {
                install(txn, sibling, sibling_pin, &leaf)?;
                install(txn, parent, parent_pin, &branch)?;
                adjust_strong(txn, level, 1)?;
                adjust_strong(txn, sibling, 1)
            })
            .await
            .unwrap();
        queue.push(sibling, sibling_pin);
        queue.push(parent, parent_pin);
        level = parent;
    }

    // Everything is queued for release and nothing is the root: vacuum
    // must reclaim the entire structure, across multiple bounded steps.
    vacuum(&store, &queue).await.unwrap();
    audit(&store, None, true).await;
    assert!(scan(&store, NODES).await.is_empty());
}

/// A stale reclamation entry — a node re-linked after being queued — is
/// dropped without reclaiming the node.
#[pollster::test]
async fn stale_queue_entries_are_dropped() {
    let store = Memory::recording();
    let allocator = IdAllocator::default();

    let node = NodeId(allocator.allocate(&store).await.unwrap());
    let pin = PinId(allocator.allocate(&store).await.unwrap());
    let record = leaf_record(0, b"leaf");
    store
        .write(move |txn| install(txn, node, pin, &record))
        .await
        .unwrap();
    // Released: the node is queued (strong 0, no rows).
    store
        .write(move |txn| release(txn, &[(node, pin)]))
        .await
        .unwrap();
    // Re-registered before reclamation ran (a fresh fetch pinned it).
    let repin = PinId(allocator.allocate(&store).await.unwrap());
    store
        .write(move |txn| register(txn, node, repin))
        .await
        .unwrap();

    while store.write(|txn| reclaim_step(txn)).await.unwrap() > 0 {}
    assert!(
        store
            .read(move |txn| Ok(read_node(txn, node)?.is_some()))
            .await
            .unwrap(),
        "a re-registered node must survive its stale queue entry"
    );
}

/// Recovery is idempotent and safe to interrupt: sweeping a store that
/// already recovered changes nothing, and the canonical root's tree is
/// never touched.
#[pollster::test]
async fn recovery_is_idempotent_and_spares_the_root() {
    let store = Memory::recording();
    let allocator = IdAllocator::default();
    let queue = ReleaseQueue::default();

    let kept = NodeId(allocator.allocate(&store).await.unwrap());
    let kept_pin = PinId(allocator.allocate(&store).await.unwrap());
    let stranded = NodeId(allocator.allocate(&store).await.unwrap());
    let stranded_pin = PinId(allocator.allocate(&store).await.unwrap());
    let kept_record = leaf_record(0, b"kept");
    let stranded_record = leaf_record(0, b"stranded");
    store
        .write(move |txn| {
            install(txn, kept, kept_pin, &kept_record)?;
            install(txn, stranded, stranded_pin, &stranded_record)?;
            flip_root(
                txn,
                test_network(),
                Some(kept),
                Version::new(),
                Some(clock_tag(1)),
            )
        })
        .await
        .unwrap();

    // A crash strands both registrations; recovery reclaims only what the
    // root does not reach, and a second recovery finds nothing to do.
    let survivor = store.reopen_at(store.history_len() - 1);
    recover(&survivor).await.unwrap();
    recover(&survivor).await.unwrap();
    vacuum(&survivor, &queue).await.unwrap();
    audit(&survivor, Some(clock_tag(1)), true).await;
    let (kept_alive, stranded_alive) = survivor
        .read(move |txn| {
            Ok((
                read_node(txn, kept)?.is_some(),
                read_node(txn, stranded)?.is_some(),
            ))
        })
        .await
        .unwrap();
    assert!(kept_alive, "the canonical root's tree survives recovery");
    assert!(!stranded_alive, "a stranded registration is reclaimed");
}

/// Releasing is idempotent under the committed-or-not ambiguity: a batch
/// whose acknowledgment was lost re-applies harmlessly.
#[pollster::test]
async fn ambiguous_release_reapplies_harmlessly() {
    let store = Memory::recording();
    let allocator = IdAllocator::default();
    let queue = ReleaseQueue::default();

    let node = NodeId(allocator.allocate(&store).await.unwrap());
    let pin = PinId(allocator.allocate(&store).await.unwrap());
    let record = leaf_record(0, b"leaf");
    store
        .write(move |txn| {
            install(txn, node, pin, &record)?;
            flip_root(txn, test_network(), Some(node), Version::new(), None)
        })
        .await
        .unwrap();

    queue.push(node, pin);
    // The flush commits but reports failure; the queue re-submits.
    store.inject_commit_then_error(0);
    let error = vacuum(&store, &queue).await.unwrap_err();
    assert_eq!(error, crate::store::MemoryError::Injected);
    assert!(!queue.is_empty(), "an unacknowledged batch is requeued");
    vacuum(&store, &queue).await.unwrap();
    audit(&store, None, true).await;
    let alive = store
        .read(move |txn| Ok(read_node(txn, node)?.is_some()))
        .await
        .unwrap();
    assert!(alive, "the root-linked node survives the double release");
}

/// The unused-import guard for META/ROOT_KEY/IDS_KEY: the schema surface
/// the crash sweep exercises indirectly is nailed here so the audit reads
/// the same rows the custody layer writes.
#[pollster::test]
async fn canonical_root_row_lives_in_meta() {
    let store = Memory::new();
    store
        .write(|txn| {
            flip_root(
                txn,
                test_network(),
                None,
                Version::new(),
                Some(clock_tag(1)),
            )
        })
        .await
        .unwrap();
    let raw = store.read(|txn| txn.get(META, ROOT_KEY)).await.unwrap();
    assert!(raw.is_some(), "flip_root writes META[root]");
    let ids = store.read(|txn| txn.get(META, IDS_KEY)).await.unwrap();
    assert!(ids.is_none(), "no allocation ran, so no ceiling row exists");
}
