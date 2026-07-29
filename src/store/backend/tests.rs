use super::*;
use crate::link::memory;
use crate::store::Memory;
use crate::store::schema::{GC, HELD, NODES};
use crate::{Peer, Rumors, store::OpenError};

/// A fresh KV-backed peer over the reference store, seeded as its own
/// universe; the backend clone shares the release queue the peer's
/// handles drop into.
// The inline tuple reads clearer than a minted alias for a test helper.
#[allow(clippy::type_complexity)]
fn kv_peer() -> (
    Memory,
    KvBackend<Memory, u64>,
    Rumors<u64, crate::bookmark::NoBookmark, KvBackend<Memory, u64>>,
) {
    let store = Memory::default();
    let backend = KvBackend::new(store.clone());
    let peer: Peer<u64, _, KvBackend<Memory, u64>> = Peer::seed_in(backend.clone());
    (store, backend, peer.into_rumors())
}

/// Collect a snapshot's live messages as sorted `(key, value)` pairs.
fn contents<S: crate::tree::backend::Store<u64>>(
    rumors: &Rumors<u64, crate::bookmark::NoBookmark, S>,
) -> Vec<(crate::Key, u64)> {
    let mut collected: Vec<_> = crate::testing::collect(&rumors.snapshot())
        .into_iter()
        .map(|(key, _, value)| (key, *value))
        .collect();
    collected.sort();
    collected
}

/// Audit the store at quiescence: every stored node is reachable from the
/// canonical root, its strong count equals its recomputed durable edges,
/// and the held and reclamation tables are empty.
fn audit(store: &Memory) {
    pollster::block_on(async {
        use crate::store::kv::Kv as _;
        store
            .read(|txn| {
                // The reachable closure of the canonical root, with each
                // node's expected strong count (durable edges + the root
                // edge).
                let record = CanonicalRoot::read(txn)?;
                let mut reachable = std::collections::HashMap::<u64, u64>::new();
                let mut frontier: Vec<NodeId> = Vec::new();
                if let Some(root) = record.root {
                    reachable.insert(root.0, 1);
                    frontier.push(root);
                }
                while let Some(id) = frontier.pop() {
                    let node = refcount::read_node(txn, id)?.expect("dangling reachable edge");
                    for child in node.children() {
                        let strong = reachable.entry(child.0).or_insert(0);
                        *strong += 1;
                        if *strong == 1 {
                            frontier.push(child);
                        }
                    }
                }
                // Storage holds exactly the closure, with exact counts.
                let mut stored = 0;
                let mut cursor: Option<Vec<u8>> = None;
                while let Some((key, value)) = txn.next_after(NODES, cursor.as_deref())? {
                    let id = NodeId::from_key(&key);
                    let node = NodeRecord::decode(&value);
                    assert_eq!(
                        Some(&node.strong),
                        reachable.get(&id.0),
                        "stored strong count matches recomputed reachability for {id:?}"
                    );
                    stored += 1;
                    cursor = Some(key);
                }
                assert_eq!(stored, reachable.len(), "storage is exactly the closure");
                assert!(
                    txn.next_after(HELD, None)?.is_none(),
                    "no held rows at quiescence"
                );
                assert!(
                    txn.next_after(GC, None)?.is_none(),
                    "no queued reclamation at quiescence"
                );
                Ok(())
            })
            .await
            .expect("audit transaction");
    });
}

/// A KV-backed peer stores what it sends and reads it back through every
/// public read face: point lookups, the full walk, and causal ranges.
#[test]
fn kv_peer_sends_and_reads_back() {
    pollster::block_on(async {
        let (_store, _backend, rumors) = kv_peer();
        for message in 0..48u64 {
            rumors.send(message).await.expect("send");
        }
        let snapshot = rumors.snapshot();
        assert_eq!(snapshot.len(), 48);
        let mut seen: Vec<u64> = Vec::new();
        for (key, version, value) in crate::testing::collect(&snapshot) {
            seen.push(*value);
            let (got_version, got_value) = snapshot
                .get(&key)
                .await
                .expect("get")
                .expect("walked key is live");
            assert_eq!(got_version, version, "point lookup agrees with the walk");
            assert_eq!(*got_value, *value);
        }
        seen.sort();
        assert_eq!(seen, (0..48).collect::<Vec<_>>());
        // A causal range bounded by the frontier holds everything; one
        // that subtracts it holds nothing.
        let latest = snapshot.latest().clone();
        assert_eq!(
            crate::testing::collect_range(&snapshot, ..=latest.clone()).len(),
            48
        );
        let after = crate::testing::collect_range(
            &snapshot,
            (
                std::ops::Bound::Excluded(latest),
                std::ops::Bound::<crate::Version>::Unbounded,
            ),
        );
        assert!(after.is_empty(), "nothing lies past the frontier");
    });
}

/// Redaction removes exactly the targeted message from a KV-backed
/// replica, and the causal ceiling still advances.
#[test]
fn kv_peer_redacts() {
    pollster::block_on(async {
        let (_store, _backend, rumors) = kv_peer();
        for message in 0..8u64 {
            rumors.send(message).await.expect("send");
        }
        let snapshot = rumors.snapshot();
        let (victim, _, _) = crate::testing::collect(&snapshot)[3];
        let before = snapshot.latest().clone();
        rumors.redact(victim).await.expect("redact");
        let after = rumors.snapshot();
        assert_eq!(after.len(), 7);
        assert!(after.get(&victim).await.expect("get").is_none());
        assert!(
            before < *after.latest(),
            "an effectual redaction advances the ceiling"
        );
    });
}

/// A KV-backed peer and an in-memory peer converge over the existing wire
/// protocol, in both directions: the storage backend changes where a
/// replica lives, never what crosses the wire.
#[test]
fn kv_and_local_converge_over_gossip() {
    pollster::block_on(async {
        let local: Peer<u64> = Peer::seed();
        let local = local.into_rumors();
        for message in 0..32u64 {
            local.send(message).await.expect("send");
        }

        // A KV-backed peer bootstraps from the in-memory one.
        let (mut a, mut b) = memory();
        let store = Memory::default();
        let (served, joined) = tokio::join!(
            local.gossip(&mut a),
            Peer::<u64>::bootstrap()
                .backend(KvBackend::new(store.clone()))
                .join(&mut b),
        );
        served.expect("serve the bootstrap");
        let kv = joined
            .expect("bootstrap")
            .expect("not a mutual bootstrap")
            .into_rumors();
        assert_eq!(
            local.snapshot().hash(),
            kv.snapshot().hash(),
            "bootstrap converged"
        );
        assert_eq!(contents(&local), contents(&kv));

        // Divergence on both sides reconverges over plain gossip.
        for message in 100..116u64 {
            kv.send(message).await.expect("kv send");
        }
        local.send(200).await.expect("local send");
        let redacted = crate::testing::collect(&local.snapshot())[0].0;
        local.redact(redacted).await.expect("local redact");
        let (mut a, mut b) = memory();
        let (kv_out, local_out) = tokio::join!(kv.gossip(&mut a), local.gossip(&mut b));
        kv_out.expect("kv gossip");
        local_out.expect("local gossip");
        assert_eq!(
            local.snapshot().hash(),
            kv.snapshot().hash(),
            "gossip reconverged"
        );
        assert_eq!(contents(&local), contents(&kv));
        assert!(
            kv.snapshot().get(&redacted).await.expect("get").is_none(),
            "the redaction was honored across backends"
        );
    });
}

/// Reopening a store resumes the replica: same network, same content,
/// and a minting identity whose next sends gossip cleanly with a peer
/// that saw the pre-restart state.
#[test]
fn reopen_resumes_the_replica() {
    pollster::block_on(async {
        let (store, _backend, rumors) = kv_peer();
        for message in 0..24u64 {
            rumors.send(message).await.expect("send");
        }

        // A second replica holds the pre-restart state.
        let (mut a, mut b) = memory();
        let (served, joined) =
            tokio::join!(rumors.gossip(&mut a), Peer::<u64>::bootstrap().join(&mut b),);
        served.expect("serve");
        let witness = joined.expect("bootstrap").expect("real join").into_rumors();

        let network = rumors.network();
        let expected = contents(&rumors);
        drop(rumors);

        let reopened = Peer::<u64, _, _>::open(store)
            .await
            .expect("open")
            .into_rumors();
        assert_eq!(reopened.network(), network, "the network resumed");
        assert_eq!(contents(&reopened), expected, "the content resumed");

        // The resumed identity keeps minting: new sends converge with the
        // witness without any coordinate collision.
        for message in 300..308u64 {
            reopened.send(message).await.expect("resumed send");
        }
        let (mut a, mut b) = memory();
        let (reopened_out, witness_out) =
            tokio::join!(reopened.gossip(&mut a), witness.gossip(&mut b));
        reopened_out.expect("reopened gossip");
        witness_out.expect("witness gossip");
        assert_eq!(
            reopened.snapshot().hash(),
            witness.snapshot().hash(),
            "post-restart sends converge"
        );
    });
}

/// An untouched store cannot be opened as a peer, and a retired one
/// reports that its identity lives elsewhere.
#[test]
fn open_reports_empty_and_retired() {
    pollster::block_on(async {
        let empty = Memory::default();
        assert!(matches!(
            Peer::<u64, _, _>::open(empty).await,
            Err(OpenError::Empty)
        ));

        let (store, _backend, retiree) = kv_peer();
        retiree.send(7).await.expect("send");
        // Bootstrap a sibling to absorb the retirement.
        let (mut a, mut b) = memory();
        let (served, joined) = tokio::join!(
            retiree.gossip(&mut a),
            Peer::<u64>::bootstrap().join(&mut b),
        );
        served.expect("serve");
        let sibling = joined.expect("bootstrap").expect("real join").into_rumors();
        let retiree = retiree.try_into_peer().await.expect("sole handle");
        let (mut a, mut b) = memory();
        let (retired, absorbed) = tokio::join!(retiree.retire(&mut a), sibling.gossip(&mut b));
        assert!(matches!(retired, crate::Retire::Retired));
        absorbed.expect("absorb");

        assert!(matches!(
            Peer::<u64, _, _>::open(store).await,
            Err(OpenError::Retired)
        ));
    });
}

/// After every handle drops and reclamation drains, the store holds
/// exactly the canonical root's closure — no leaked records,
/// registrations, or queue entries.
#[test]
fn quiesced_store_is_exactly_the_reachable_set() {
    pollster::block_on(async {
        let (store, backend, rumors) = kv_peer();
        for message in 0..32u64 {
            rumors.send(message).await.expect("send");
        }
        // Churn: redact some, take snapshots across commits, drop them.
        let keys: Vec<_> = crate::testing::collect(&rumors.snapshot())
            .into_iter()
            .map(|(key, _, _)| key)
            .collect();
        for key in keys.iter().take(12) {
            rumors.redact(*key).await.expect("redact");
        }
        drop(rumors);
        backend.vacuum().await.expect("vacuum");
        audit(&store);
    });
}

/// Focused differential probe: the KV backend's `Store::act` agrees with
/// the in-memory engine on the root hash, insert by insert.
#[test]
fn kv_act_matches_local_hash() {
    use crate::tree::backend::{Action, Local, Node as _, Store as _};
    use crate::tree::typed::Path;
    pollster::block_on(async {
        let mut clock = before::Clock::seed();
        let kv = KvBackend::<Memory, u64>::new(Memory::default());
        let mut local_root = None;
        let mut kv_root = None;
        for message in 0..16u64 {
            let message = crate::message::Message::from(message);
            let version = clock.tick().clone();
            let path = Path::for_leaf(&version, message.bytes());
            let actions = vec![(path, version.clone(), Action::Insert(message.clone()))];
            local_root = Local
                .act(local_root, actions.clone(), |_: &crate::Version| {})
                .await
                .expect("local act");
            kv_root = kv
                .clone()
                .act(kv_root, actions, |_: &crate::Version| {})
                .await
                .expect("kv act");
            let local_hash = local_root.as_ref().map(|node| node.hash());
            let kv_hash = kv_root.as_ref().map(|node| node.hash());
            assert_eq!(local_hash, kv_hash, "diverged at insert {message:?}");
        }
    });
}
