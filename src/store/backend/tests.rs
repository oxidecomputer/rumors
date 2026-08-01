use futures::StreamExt as _;

use super::*;
use crate::link::memory;
use crate::store::Memory;
use crate::store::kv::ReadTxn as _;
use crate::store::schema::{GC, HELD};
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
    // The re-execution schedule rides under every battery in this file:
    // each backend transaction closure runs twice with the first
    // execution discarded, so a closure leaking any effect outside its
    // transaction argument diverges under the assertions below.
    let store = Memory::new().retrying();
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
        crate::store::checked::read(store, |txn| {
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
                let id = NodeId::from_key(NODES, &key)?;
                let node = NodeRecord::decode(id, &value)?;
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
        let store = Memory::new().retrying();
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

/// A store whose canonical record rotted refuses to open with the
/// corruption genre, naming the corrupt row.
///
/// The genre is distinct from [`OpenError::Empty`],
/// [`OpenError::Retired`], and a store failure, so a deployment can
/// tell "the store lied" from every other refusal to resume.
#[test]
fn open_reports_corruption_distinctly() {
    pollster::block_on(async {
        use crate::store::kv::Kv as _;
        let (store, _backend, rumors) = kv_peer();
        rumors.send(7).await.expect("send");
        drop(rumors);

        // Bit rot at the canonical record: truncate the stored row.
        let rotted = store
            .read(|txn| txn.get(crate::store::schema::META, crate::store::schema::ROOT_KEY))
            .await
            .expect("read the root row")
            .expect("a seeded store holds a root row");
        store
            .write(move |txn| {
                txn.put(
                    crate::store::schema::META,
                    crate::store::schema::ROOT_KEY,
                    &rotted[..rotted.len() - 1],
                )
            })
            .await
            .expect("plant the rot");

        match Peer::<u64, _, _>::open(store).await {
            Err(OpenError::Corrupt(corruption)) => {
                assert_eq!(corruption.table(), crate::store::schema::META);
                assert_eq!(corruption.key(), crate::store::schema::ROOT_KEY);
            }
            Err(other) => panic!("a rotted record must refuse as corruption, got {other:?}"),
            Ok(_) => panic!("a rotted record must refuse to open"),
        }
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
        let kv = KvBackend::<Memory, u64>::new(Memory::new().retrying());
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

/// The crash-point battery: every committed prefix reopens consistently.
///
/// For every committed transaction prefix of a real workload, reopening
/// the store yields a consistent replica — the audit invariants hold
/// after recovery and vacuum, the tree walks, and its state is one the
/// live run actually published (prefix consistency lifted to the
/// replica level).
#[test]
fn every_crash_prefix_reopens_consistently() {
    pollster::block_on(async {
        let store = Memory::recording().retrying();
        let backend = KvBackend::<Memory, u64>::new(store.clone());
        let peer: Peer<u64, _, KvBackend<Memory, u64>> = Peer::seed_in(backend.clone());
        let rumors = peer.into_rumors();

        // A workload of sends and redactions; record every published
        // (hash, contents) pair the live run exposed.
        let mut published = std::collections::HashMap::new();
        let snapshot = rumors.snapshot();
        published.insert(snapshot.hash(), Vec::new());
        for message in 0..12u64 {
            rumors.send(message).await.expect("send");
            let snapshot = rumors.snapshot();
            published.insert(snapshot.hash(), contents(&rumors));
        }
        for index in [0usize, 3, 7] {
            let (key, _, _) = crate::testing::collect(&rumors.snapshot())[index];
            rumors.redact(key).await.expect("redact");
            let snapshot = rumors.snapshot();
            published.insert(snapshot.hash(), contents(&rumors));
        }
        drop(rumors);
        backend.vacuum().await.expect("vacuum");

        for prefix in 0..store.history_len() {
            let crashed = store.reopen_at(prefix);
            let reopened = match Peer::<u64, _, _>::open(crashed.clone()).await {
                // Prefixes before the first flip hold no replica.
                Err(OpenError::Empty) => continue,
                other => other.expect("open at prefix"),
            };
            let reopened = reopened.into_rumors();
            let snapshot = reopened.snapshot();
            let expected = published.get(&snapshot.hash()).unwrap_or_else(|| {
                panic!("prefix {prefix} reopened to a state the live run never published")
            });
            assert_eq!(
                &contents(&reopened),
                expected,
                "prefix {prefix}: reopened contents match the published state"
            );
            // `open` built its own backend, whose release queue died with
            // the dropped peer: recovery is the sanctioned sweep for
            // registrations whose process is gone.
            drop(reopened);
            crate::store::refcount::recover(&crashed)
                .await
                .expect("recover");
            KvBackend::<Memory, u64>::new(crashed.clone())
                .vacuum()
                .await
                .expect("vacuum");
            audit(&crashed);
        }
    });
}

/// The fault battery: an injected store failure aborts a commit cleanly.
///
/// A failure surfacing mid-commit leaves the published replica exactly
/// at its pre-operation state, wakes no observer, reports through
/// `StorageError` — and the store still audits clean after recovery,
/// whichever transaction the fault hit.
#[test]
fn injected_faults_abort_commits_cleanly() {
    // Sweep the fault across the first forty write transactions.
    for nth in 0..40u64 {
        pollster::block_on(async {
            let store = Memory::new().retrying();
            let backend = KvBackend::<Memory, u64>::new(store.clone());
            let peer: Peer<u64, _, KvBackend<Memory, u64>> = Peer::seed_in(backend.clone());
            let rumors = peer.into_rumors();
            let mut ticks = std::pin::pin!(rumors.changes());
            // Drain any initial readiness so the loop observes exactly
            // the ticks its own commits produce.
            let _ = futures::poll!(ticks.as_mut().next());
            let mut wakeups = 0usize;

            store.inject_abort(nth);
            let mut failed = None;
            for message in 0..8u64 {
                let before_hash = rumors.snapshot().hash();
                let before_contents = contents(&rumors);
                match rumors.send(message).await {
                    Ok(()) => {
                        wakeups += 1;
                        assert!(
                            futures::poll!(ticks.as_mut().next()).is_ready(),
                            "a committed send ticks its observer"
                        );
                    }
                    Err(crate::StorageError(_)) => {
                        assert_eq!(
                            rumors.snapshot().hash(),
                            before_hash,
                            "a failed send leaves the published tree unchanged"
                        );
                        assert_eq!(contents(&rumors), before_contents);
                        assert!(
                            futures::poll!(ticks.as_mut().next()).is_pending(),
                            "a failed send wakes no observer"
                        );
                        failed = Some(message);
                        break;
                    }
                }
            }
            let sent = wakeups;
            if failed.is_none() {
                // The fault landed in upkeep-only traffic or past the
                // workload; the sends all committed.
                assert_eq!(sent, 8);
            }
            // The pinned observer's borrow ends here; the fault window is
            // over, so recovery and the audit run against the store's
            // honest behavior.
            drop(rumors);
            store.clear_faults();
            // The store heals: recovery + vacuum reach a clean audit.
            let healed = KvBackend::<Memory, u64>::new(store.clone());
            crate::store::refcount::recover(&store)
                .await
                .expect("recover");
            healed.vacuum().await.expect("vacuum");
            audit(&store);
        });
    }
}

/// The ambiguous-commit window resolves forward.
///
/// A commit whose acknowledgment is lost (the transaction applied, the
/// caller saw an error) is superseded by the next successful commit,
/// and the store never double-counts or leaks through the retried
/// release traffic.
#[test]
fn ambiguous_commits_are_superseded_cleanly() {
    for nth in 0..24u64 {
        pollster::block_on(async {
            let store = Memory::new().retrying();
            let backend = KvBackend::<Memory, u64>::new(store.clone());
            let peer: Peer<u64, _, KvBackend<Memory, u64>> = Peer::seed_in(backend.clone());
            let rumors = peer.into_rumors();

            store.inject_commit_then_error(nth);
            let mut observed = Vec::new();
            for message in 0..8u64 {
                // An Err here may or may not have committed: both are
                // legal outcomes; later commits must supersede either way.
                let _ = rumors.send(message).await;
                observed.push(rumors.snapshot().hash());
            }
            store.clear_faults();
            rumors
                .send(99)
                .await
                .expect("the fault window is over; this commits");
            let final_contents = contents(&rumors);
            assert!(
                final_contents.iter().any(|(_, value)| *value == 99),
                "the post-fault commit landed"
            );
            drop(rumors);
            backend.vacuum().await.expect("vacuum");
            audit(&store);
        });
    }
}

/// The wire barrier's watermark discipline: at most one flush per
/// commit-then-send window.
///
/// The first barrier after an identity-bearing commit flushes, a
/// commit-free window flushes nothing, and local churn alone never
/// flushes — the lazy store's commit-latency win, priced at one
/// [`Kv::sync`] per commit-then-send window.
#[test]
fn barrier_flushes_once_per_commit_window() {
    use crate::tree::backend::Store as _;
    pollster::block_on(async {
        let (store, backend, rumors) = kv_peer();
        rumors.send(1).await.expect("send");
        rumors.send(2).await.expect("send");
        assert_eq!(store.sync_count(), 0, "local churn never waits on a flush");
        backend.barrier().await.expect("barrier");
        assert_eq!(
            store.sync_count(),
            1,
            "the first barrier after commits flushes"
        );
        backend.barrier().await.expect("barrier");
        backend.barrier().await.expect("barrier");
        assert_eq!(
            store.sync_count(),
            1,
            "a commit-free window flushes nothing"
        );
        rumors.send(3).await.expect("send");
        backend.barrier().await.expect("barrier");
        assert_eq!(store.sync_count(), 2, "a fresh commit re-arms the barrier");
    });
}

/// Acknowledged and escaped implies durable, for every store policy.
///
/// A session flushes exactly once before transmitting, and at the
/// worst crash a write-behind store's policy permits (everything since
/// the last completed flush lost), the store still holds every message
/// the session shipped — with the flush landing strictly before the
/// session's closing transactions, i.e. before the wire moved, not
/// after.
#[test]
fn escaped_state_is_durable_before_it_ships() {
    pollster::block_on(async {
        let store = Memory::recording().retrying();
        let backend = KvBackend::<Memory, u64>::new(store.clone());
        let peer: Peer<u64, _, KvBackend<Memory, u64>> = Peer::seed_in(backend.clone());
        let rumors = peer.into_rumors();
        rumors.send(0).await.expect("send");

        // An established witness to gossip with (its bootstrap is a
        // session of its own; the assertions below count from after it).
        let (mut a, mut b) = memory();
        let (served, joined) =
            tokio::join!(rumors.gossip(&mut a), Peer::<u64>::bootstrap().join(&mut b));
        served.expect("serve");
        let witness = joined.expect("bootstrap").expect("real join").into_rumors();

        for message in 1..8u64 {
            rumors.send(message).await.expect("send");
        }
        let shipped = contents(&rumors);
        let syncs_before = store.sync_count();

        let (mut a, mut b) = memory();
        let (ours, theirs) = tokio::join!(rumors.gossip(&mut a), witness.gossip(&mut b));
        ours.expect("gossip");
        theirs.expect("witness gossip");
        assert_eq!(
            store.sync_count(),
            syncs_before + 1,
            "one flush covers the whole commit-then-send window"
        );
        assert!(
            store.synced_prefix() < store.history_len() - 1,
            "the flush landed strictly before the session's closing transactions"
        );

        // The worst legal crash for a write-behind store: everything
        // after the last completed flush is gone. Everything the wire
        // carried must still be there.
        let crashed = store.reopen_at(store.synced_prefix());
        let reopened = Peer::<u64, _, _>::open(crashed)
            .await
            .expect("the synced prefix resumes")
            .into_rumors();
        assert_eq!(
            contents(&reopened),
            shipped,
            "every version the session shipped survives the crash"
        );
    });
}

/// The send blocks on the flush: with a failing [`Kv::sync`], a session
/// dies before anything escapes.
///
/// The crash window between commit-acknowledge and durability can
/// never contain a wire send, by construction rather than by luck.
#[test]
fn failing_sync_blocks_the_send() {
    pollster::block_on(async {
        let (store, _backend, rumors) = kv_peer();
        rumors.send(0).await.expect("send");
        let (mut a, mut b) = memory();
        let (served, joined) =
            tokio::join!(rumors.gossip(&mut a), Peer::<u64>::bootstrap().join(&mut b));
        served.expect("serve");
        let witness = joined.expect("bootstrap").expect("real join").into_rumors();
        let before = contents(&witness);

        rumors
            .send(99)
            .await
            .expect("the commit itself never flushes");
        store.inject_sync_error();
        let (mut a, mut b) = memory();
        // The failing side hangs up when it aborts (dropping its link)
        // so the counterparty's half-run session resolves as a dead
        // session instead of parking on a silent peer.
        let ours = async {
            let outcome = rumors.gossip(&mut a).await;
            drop(a);
            outcome
        };
        let (ours, theirs) = tokio::join!(ours, witness.gossip(&mut b));
        assert!(
            matches!(
                ours,
                Err(crate::error::Error::Storage(crate::StorageError(
                    KvError::Store(crate::store::MemoryError::Injected)
                )))
            ),
            "the barrier's failure surfaces as the store-failed genre"
        );
        // The counterparty saw a session die, never the unflushed commit.
        let _ = theirs.expect_err("the counterparty's session dies too");
        assert_eq!(
            contents(&witness),
            before,
            "nothing escaped ahead of durability"
        );

        // The flush machinery healed: the same session now completes and
        // the commit escapes normally.
        let (mut a, mut b) = memory();
        let (ours, theirs) = tokio::join!(rumors.gossip(&mut a), witness.gossip(&mut b));
        ours.expect("gossip after the fault window");
        theirs.expect("witness gossip");
        assert_eq!(contents(&witness), contents(&rumors));
    });
}

/// The durable-identity shrink law: a donation is recorded before it ships.
///
/// Serving a bootstrap writes the post-donation identity record before
/// the fork crosses the wire, so a crash at ANY committed prefix after
/// the donation shipped reopens to a party disjoint from the
/// bootstrapped peer's — the donated region is never resurrected.
/// Disjointness is checked by `Party::join`, which refuses overlap;
/// monotonicity (once disjoint, disjoint at every later prefix) is what
/// pins "recorded before the wire, never rolled back".
#[test]
fn donation_is_recorded_before_it_ships() {
    pollster::block_on(async {
        let store = Memory::recording().retrying();
        let backend = KvBackend::<Memory, u64>::new(store.clone());
        let peer: Peer<u64, _, KvBackend<Memory, u64>> = Peer::seed_in(backend.clone());
        let rumors = peer.into_rumors();
        for message in 0..8u64 {
            rumors.send(message).await.expect("send");
        }

        // Serve a bootstrap: a fork of this identity leaves over the wire.
        let (mut a, mut b) = memory();
        let (served, joined) =
            tokio::join!(rumors.gossip(&mut a), Peer::<u64>::bootstrap().join(&mut b),);
        served.expect("serve");
        let booted = joined.expect("bootstrap").expect("real join").into_rumors();
        let donated = booted
            .dangerously_alias_party()
            .expect("the bootstrapped peer holds the donated fork");

        // Reopen at every committed prefix and classify: does the stored
        // identity overlap the donated fork?
        let mut overlapping = Vec::new();
        let mut disjoint = Vec::new();
        for prefix in 0..store.history_len() {
            let crashed = store.reopen_at(prefix);
            let reopened = match Peer::<u64, _, _>::open(crashed).await {
                Err(OpenError::Empty) => continue,
                other => other.expect("open at prefix"),
            };
            let mut resumed = reopened
                .into_rumors()
                .dangerously_alias_party()
                .expect("a reopened peer holds a party");
            // `Party::join` is the disjointness oracle: the sum of two
            // parties is defined exactly when they overlap nowhere.
            match resumed.join(donated.dangerously_alias()) {
                Ok(()) => disjoint.push(prefix),
                Err(_) => overlapping.push(prefix),
            }
        }

        // The donation shipped, so the final record must be disjoint; and
        // once the shrink is recorded it is never rolled back.
        let first_disjoint = *disjoint
            .first()
            .expect("the shrink write reached the store before the session ended");
        assert!(
            overlapping.iter().all(|&prefix| prefix < first_disjoint),
            "a prefix after the shrink write resurrected the donated region: \
             overlapping {overlapping:?}, disjoint {disjoint:?}"
        );
        assert!(
            disjoint.contains(&(store.history_len() - 1)),
            "the final state records the post-donation identity"
        );
        // The teeth: the shrink must be recorded in its own transaction
        // STRICTLY BEFORE the session's closing install flip (the final
        // transaction here). The flip also records the post-fork
        // identity, but it runs after the party crossed the wire — a
        // crash between the crossing and the flip resurrects the
        // donation unless an earlier transaction recorded the shrink.
        assert!(
            first_disjoint < store.history_len() - 1,
            "no transaction before the closing flip records the shrink: \
             the donation crossed the wire unrecorded"
        );
    });
}

/// The cancellation battery: dropped sends are full-or-nothing.
///
/// A send future dropped at every poll depth either committed in full
/// or left no trace — never a prefix — the party is never lost, no
/// observer wakes for an uncommitted state, and whatever the drop
/// stranded reclaims through recovery and vacuum.
#[test]
fn sends_dropped_at_every_poll_depth_are_full_or_nothing() {
    use std::future::Future as _;
    use std::task::{Context, Poll};

    // Depths past the future's natural completion just commit; the sweep
    // covers every genuine suspension point along the way.
    for depth in 0..12usize {
        pollster::block_on(async {
            let store = Memory::new().retrying();
            let backend = KvBackend::<Memory, u64>::new(store.clone());
            let peer: Peer<u64, _, KvBackend<Memory, u64>> = Peer::seed_in(backend.clone());
            let rumors = peer.into_rumors();
            rumors.send(1).await.expect("baseline send");
            let baseline_hash = rumors.snapshot().hash();
            let baseline = contents(&rumors);

            let mut ticks = std::pin::pin!(rumors.changes());
            let _ = futures::poll!(ticks.as_mut().next());

            // Poll the victim exactly `depth` times, then drop it.
            let committed = {
                let mut victim = Box::pin(rumors.send(2));
                let waker = std::task::Waker::noop();
                let mut context = Context::from_waker(waker);
                let mut committed = false;
                for _ in 0..depth {
                    if let Poll::Ready(result) = victim.as_mut().poll(&mut context) {
                        result.expect("a completed send committed");
                        committed = true;
                        break;
                    }
                }
                committed
            };

            let after = rumors.snapshot();
            if committed {
                assert_eq!(
                    after.len(),
                    2,
                    "depth {depth}: the completed send is visible"
                );
                assert!(
                    futures::poll!(ticks.as_mut().next()).is_ready(),
                    "depth {depth}: a committed send ticked its observer"
                );
            } else {
                assert_eq!(
                    after.hash(),
                    baseline_hash,
                    "depth {depth}: no partial commit"
                );
                assert_eq!(contents(&rumors), baseline);
                assert!(
                    futures::poll!(ticks.as_mut().next()).is_pending(),
                    "depth {depth}: no observer woke for an uncommitted state"
                );
            }
            // The party survived the drop: the next send commits.
            rumors
                .send(3)
                .await
                .expect("depth {depth}: the party survived");

            drop(rumors);
            crate::store::refcount::recover(&store)
                .await
                .expect("recover");
            KvBackend::<Memory, u64>::new(store.clone())
                .vacuum()
                .await
                .expect("vacuum");
            audit(&store);
        });
    }
}

/// A retirement's cleared identity is recorded before the party ships.
///
/// Reopening at any committed prefix after the donation crossed the
/// wire reports [`OpenError::Retired`] — a restarted retiree never
/// resurrects what it donated. (The whole-party analog of the
/// fork-donation shrink law above, sharing its transaction-ordering
/// teeth: the clear must land strictly before the session's final
/// transactions.)
#[test]
fn retirement_clears_the_record_before_it_ships() {
    pollster::block_on(async {
        let store = Memory::recording().retrying();
        let backend = KvBackend::<Memory, u64>::new(store.clone());
        let peer: Peer<u64, _, KvBackend<Memory, u64>> = Peer::seed_in(backend.clone());
        let rumors = peer.into_rumors();
        rumors.send(7).await.expect("send");
        let (mut a, mut b) = memory();
        let (served, joined) =
            tokio::join!(rumors.gossip(&mut a), Peer::<u64>::bootstrap().join(&mut b),);
        served.expect("serve");
        let sibling = joined.expect("bootstrap").expect("real join").into_rumors();
        let retiree = rumors.try_into_peer().await.expect("sole handle");
        let (mut a, mut b) = memory();
        let (retired, absorbed) = tokio::join!(retiree.retire(&mut a), sibling.gossip(&mut b));
        assert!(matches!(retired, crate::Retire::Retired));
        absorbed.expect("absorb");

        // Classify every committed prefix; once cleared, always cleared.
        let mut cleared = Vec::new();
        let mut holding = Vec::new();
        for prefix in 0..store.history_len() {
            let crashed = store.reopen_at(prefix);
            match Peer::<u64, _, _>::open(crashed).await {
                Err(OpenError::Empty) => continue,
                Err(OpenError::Retired) => cleared.push(prefix),
                Ok(_) => holding.push(prefix),
                Err(other) => panic!("prefix {prefix}: unexpected open failure {other:?}"),
            }
        }
        let first_cleared = *cleared
            .first()
            .expect("the retirement's clear reached the store");
        assert!(
            holding.iter().all(|&prefix| prefix < first_cleared),
            "a prefix after the clear resurrected the retiree: \
             holding {holding:?}, cleared {cleared:?}"
        );
        assert!(cleared.contains(&(store.history_len() - 1)));
    });
}

/// A live edge that references an absent node record refuses as
/// corruption, naming the swept row.
///
/// Every identifier a fetch resolves comes from a strong edge or a
/// registration this backend wrote, so within one live backend the
/// absent row is unreachable — reaching it is evidence of a foreign
/// sweep or lost writes, and it surfaces exactly like every other way
/// the store's contents can diverge from what this crate wrote.
#[test]
fn dangling_edge_refuses_as_corruption() {
    pollster::block_on(async {
        use crate::store::kv::Kv as _;
        let (store, _backend, rumors) = kv_peer();
        rumors.send(7).await.expect("send");
        drop(rumors);

        // Sweep the root node's record out from underneath the canonical
        // edge that references it.
        let root = crate::store::checked::read(&store, |txn| {
            crate::store::schema::CanonicalRoot::read(txn)
        })
        .await
        .expect("read the canonical record")
        .root
        .expect("a store that committed a send names a root");
        store
            .write(move |txn| txn.delete(crate::store::schema::NODES, &root.key()))
            .await
            .expect("sweep the row");

        match Peer::<u64, _, _>::open(store).await {
            Err(OpenError::Corrupt(corruption)) => {
                assert_eq!(corruption.table(), crate::store::schema::NODES);
                assert_eq!(corruption.key(), root.key());
            }
            Err(other) => panic!("a dangling edge must refuse as corruption, got {other:?}"),
            Ok(_) => panic!("a dangling edge must refuse to open"),
        }
    });
}
