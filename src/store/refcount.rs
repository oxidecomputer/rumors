//! Custody: reference counting, deferred reclamation, and recovery.
//!
//! This layer keeps the [schema](super::schema)'s liveness invariant true
//! at every committed transaction: `strong(n)` equals `n`'s durable parent
//! edges plus one if the canonical root names it, held rows register
//! exactly the live in-process entry handles, and a node is reclaimed
//! exactly when both say it is unreachable.
//!
//! Three disciplines make every transaction here small and every failure
//! recoverable:
//!
//! - **Names before writes.** IDs come from the block allocator, so a
//!   node and its registration are named before the transaction that
//!   stores them runs; an interrupted insert leaves at worst a registered
//!   record the release path or the recovery sweep reclaims.
//! - **Idempotent releases.** A release deletes one exact `(node, pin)`
//!   row. Re-applying it — after a transaction retry, or after a commit
//!   whose acknowledgment was lost — is a no-op, so the in-process
//!   release queue may safely re-submit a batch whose fate it never
//!   learned.
//! - **Queued cascades.** Reclaiming a branch decrements its children
//!   inside one bounded transaction and *queues* any that reach zero;
//!   a cascade of any depth is queue entries, never transaction growth.
//!
//! Recovery is the same machinery: on open, every held row is dead
//! process state by definition (single-process ownership), so the sweep
//! releases them all in bounded transactions and lets reclamation drain.
//! The canonical root is never swept — its liveness is the root edge in
//! `strong`, not a held row.

use std::sync::Mutex;

use super::checked;
use super::error::{Corruption, KvError};
use super::kv::{Kv, ReadTxn, WriteTxn};
use super::schema::{
    CanonicalRoot, GC, HELD, NODES, NodeId, NodeRecord, PinId, held_key, split_held_key,
};

/// Held-row releases applied per write transaction (piggybacked or
/// draining): bounds the flush's share of any transaction.
pub(crate) const RELEASE_BUDGET: usize = 64;

/// Nodes reclaimed per drain transaction; each contributes at most its
/// child fan (≤ 256) of decrements.
pub(crate) const GC_BUDGET: usize = 16;

/// The in-process release queue: where dropped pins land, since `Drop`
/// cannot await.
///
/// Entries are exact `(node, pin)` registrations; applying one twice is
/// harmless (idempotent releases, module docs), so the queue re-submits
/// batches whose transaction failed or was never acknowledged.
#[derive(Debug, Default)]
pub(crate) struct ReleaseQueue {
    pending: Mutex<Vec<(NodeId, PinId)>>,
}

impl ReleaseQueue {
    /// Queues one registration for release at the next flush.
    pub(crate) fn push(&self, node: NodeId, pin: PinId) {
        self.pending
            .lock()
            .expect("release queue lock poisoned")
            .push((node, pin));
    }

    /// Takes up to [`RELEASE_BUDGET`] queued releases for one flush
    /// transaction.
    ///
    /// The batch must be [`requeue`](Self::requeue)d if the transaction's
    /// commit is not positively known to have happened — re-applying is
    /// safe, forgetting is a leak (until recovery).
    pub(crate) fn take(&self) -> Vec<(NodeId, PinId)> {
        let mut pending = self.pending.lock().expect("release queue lock poisoned");
        let take = pending.len().min(RELEASE_BUDGET);
        pending.drain(..take).collect()
    }

    /// Returns a batch whose transaction failed (or may have failed).
    pub(crate) fn requeue(&self, batch: Vec<(NodeId, PinId)>) {
        self.pending
            .lock()
            .expect("release queue lock poisoned")
            .extend(batch);
    }

    /// Whether anything is waiting to flush.
    pub(crate) fn is_empty(&self) -> bool {
        self.pending
            .lock()
            .expect("release queue lock poisoned")
            .is_empty()
    }
}

/// True iff any held row registers `node`.
fn held<T: ReadTxn + ?Sized>(txn: &mut T, node: NodeId) -> Result<bool, T::Error> {
    // Held keys are (node BE, pin BE): the first row at or after
    // (node, 0) belongs to `node` iff its 8-byte prefix matches. A key
    // too short to carry the prefix registers nothing here; the
    // recovery sweep is where a malformed held key surfaces as the
    // corruption it is.
    let probe = held_key(node, PinId(0));
    Ok(match txn.get(HELD, &probe)? {
        Some(_) => true,
        None => txn
            .next_after(HELD, Some(&probe))?
            .is_some_and(|(key, _)| key.get(..8) == Some(&node.key()[..])),
    })
}

/// Enqueues `node` for reclamation iff it is fully unreachable
/// (`strong == 0` and no held row).
fn queue_if_dead<W: WriteTxn + ?Sized>(
    txn: &mut W,
    node: NodeId,
    strong: u64,
) -> Result<(), W::Error> {
    if strong == 0 && !held(txn, node)? {
        txn.put(GC, &node.key(), &[])?;
    }
    Ok(())
}

/// Reads a node's record, or `None` if it was already reclaimed.
///
/// # Errors
///
/// The transaction's own failure, or [`Corruption`] when the row's
/// bytes fail [`NodeRecord::decode`].
pub(crate) fn read_node<T>(txn: &mut T, node: NodeId) -> Result<Option<NodeRecord>, T::Error>
where
    T: ReadTxn + ?Sized,
    T::Error: From<Corruption>,
{
    txn.get(NODES, &node.key())?
        .map(|value| NodeRecord::decode(node, &value).map_err(T::Error::from))
        .transpose()
}

/// Stores a fresh node under a fresh registration.
///
/// `record.strong` counts only the durable edges the same transaction
/// creates toward it (usually zero: a fresh node is kept alive by its
/// registration until a parent or the root links it).
pub(crate) fn install<W: WriteTxn + ?Sized>(
    txn: &mut W,
    node: NodeId,
    pin: PinId,
    record: &NodeRecord,
) -> Result<(), W::Error> {
    txn.put(NODES, &node.key(), &record.encode())?;
    txn.put(HELD, &held_key(node, pin), &[])
}

/// Registers one more live handle on an existing node.
pub(crate) fn register<W: WriteTxn + ?Sized>(
    txn: &mut W,
    node: NodeId,
    pin: PinId,
) -> Result<(), W::Error> {
    txn.put(HELD, &held_key(node, pin), &[])
}

/// Adjusts a node's strong count by `delta`, queuing it for reclamation
/// if it drops to zero with no registrations.
///
/// A *decrement* against an already-reclaimed node is a no-op: IDs are
/// never reused, so the absent row means the work is already done. An
/// *increment* against one is a custody bug — something is linking a
/// record it failed to keep registered — and panics rather than
/// installing a dangling edge, as does a count that over- or
/// underflows. Both asserts are the custody detector [`Kv`]'s
/// exclusive-ownership requirement licenses: within one live backend
/// they are unreachable, and a second backend sweeping this one's
/// registrations is the documented usage violation the panic reports.
pub(crate) fn adjust_strong<W>(txn: &mut W, node: NodeId, delta: i64) -> Result<(), W::Error>
where
    W: WriteTxn + ?Sized,
    W::Error: From<Corruption>,
{
    let Some(mut record) = read_node(txn, node)? else {
        assert!(
            delta < 0,
            "linking a reclaimed record: custody accounting bug"
        );
        return Ok(());
    };
    record.strong = record
        .strong
        .checked_add_signed(delta)
        .expect("strong count over/underflow: custody accounting bug");
    txn.put(NODES, &node.key(), &record.encode())?;
    queue_if_dead(txn, node, record.strong)
}

/// Releases a batch of registrations: deletes each exact row and queues
/// any node thereby made unreachable. Idempotent per entry.
pub(crate) fn release<W>(txn: &mut W, batch: &[(NodeId, PinId)]) -> Result<(), W::Error>
where
    W: WriteTxn + ?Sized,
    W::Error: From<Corruption>,
{
    for &(node, pin) in batch {
        txn.delete(HELD, &held_key(node, pin))?;
        if let Some(record) = read_node(txn, node)? {
            queue_if_dead(txn, node, record.strong)?;
        }
    }
    Ok(())
}

/// Replaces the canonical root (and identity record) atomically,
/// re-pointing the root edge's strong count from the old root to the new.
///
/// `clock` is written as given: `Some` records the identity, `None`
/// clears it (retirement) — never "retain".
pub(crate) fn flip_root<W>(
    txn: &mut W,
    network: crate::Network,
    root: Option<NodeId>,
    ceiling: before::Version,
    identity: Option<Vec<u8>>,
) -> Result<(), W::Error>
where
    W: WriteTxn + ?Sized,
    W::Error: From<Corruption>,
{
    let previous = CanonicalRoot::read(txn)?;
    CanonicalRoot {
        network: Some(network),
        ceiling,
        root,
        identity,
    }
    .write(txn)?;
    if let Some(new) = root {
        adjust_strong(txn, new, 1)?;
    }
    match previous.root {
        // An unchanged root keeps its edge: the +1 above and this -1
        // cancel inside one transaction.
        Some(old) => adjust_strong(txn, old, -1),
        None => Ok(()),
    }
}

/// Rewrites only the identity record, leaving the tree untouched: the
/// party-shrink write that must land before a donation crosses the wire.
pub(crate) fn record_identity<W>(txn: &mut W, identity: Option<Vec<u8>>) -> Result<(), W::Error>
where
    W: WriteTxn + ?Sized,
    W::Error: From<Corruption>,
{
    let mut record = CanonicalRoot::read(txn)?;
    record.identity = identity;
    record.write(txn)
}

/// One bounded reclamation step: processes up to [`GC_BUDGET`] queued
/// nodes, decrementing children (queuing any that die) and deleting the
/// processed rows. Returns how many it processed; zero means the queue
/// is empty.
pub(crate) fn reclaim_step<W>(txn: &mut W) -> Result<usize, W::Error>
where
    W: WriteTxn + ?Sized,
    W::Error: From<Corruption>,
{
    let mut processed = 0;
    let mut cursor: Option<Vec<u8>> = None;
    while processed < GC_BUDGET {
        let Some((key, _)) = txn.next_after(GC, cursor.as_deref())? else {
            break;
        };
        let node = NodeId::from_key(GC, &key)?;
        if let Some(record) = read_node(txn, node)? {
            if record.strong > 0 || held(txn, node)? {
                // A stale queue entry: the node was re-linked or
                // re-registered after it was queued. Drop the entry.
            } else {
                for child in record.children() {
                    adjust_strong(txn, child, -1)?;
                }
                txn.delete(NODES, &node.key())?;
            }
        }
        txn.delete(GC, &key)?;
        cursor = Some(key);
        processed += 1;
    }
    Ok(processed)
}

/// Flushes queued releases and drains reclamation until both are empty.
///
/// The explicit maintenance entry point; the backend also piggybacks
/// bounded flush/reclaim steps onto its ordinary write transactions, so
/// an idle store converges without ever calling this.
pub(crate) async fn vacuum<K: Kv>(kv: &K, queue: &ReleaseQueue) -> Result<(), KvError<K::Error>> {
    loop {
        let batch = queue.take();
        if batch.is_empty() {
            let processed = checked::write(kv, |txn| reclaim_step(txn)).await?;
            if processed == 0 && queue.is_empty() {
                return Ok(());
            }
        } else {
            let applied = batch.clone();
            if let Err(error) = checked::write(kv, move |txn| release(txn, &applied)).await {
                queue.requeue(batch);
                return Err(error);
            }
        }
    }
}

/// The recovery sweep: releases *every* held row in bounded transactions,
/// then drains reclamation.
///
/// Runs on open, before any handle is minted: the store is single-process,
/// so every held row is a dead process's registration — a clean shutdown
/// that flushed everything leaves the table empty, and one that didn't is
/// indistinguishable from a crash and equally swept. Idempotent: a crash
/// mid-sweep leaves the remaining rows for the next open.
pub(crate) async fn recover<K: Kv>(kv: &K) -> Result<(), KvError<K::Error>> {
    loop {
        let swept = checked::write(kv, |txn| {
            let mut swept = 0;
            let mut cursor: Option<Vec<u8>> = None;
            while swept < RELEASE_BUDGET {
                let Some((key, _)) = txn.next_after(HELD, cursor.as_deref())? else {
                    break;
                };
                let (node, pin) = split_held_key(&key)?;
                release(txn, &[(node, pin)])?;
                cursor = Some(key);
                swept += 1;
            }
            Ok(swept)
        })
        .await?;
        if swept == 0 {
            break;
        }
    }
    while checked::write(kv, |txn| reclaim_step(txn)).await? > 0 {}
    Ok(())
}

#[cfg(test)]
mod tests;
