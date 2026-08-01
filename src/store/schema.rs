//! The record schema: what one persistent tree looks like as rows.
//!
//! Four tables, all owned by the backend (single-process ownership is the
//! backend's usage requirement, stated in [`kv`](super::kv)):
//!
//! - [`META`]: the canonical-root record ([`CanonicalRoot`]: ceiling,
//!   optional root node ID, and the peer's durable identity record) plus
//!   the ID allocator's ceiling.
//! - [`NODES`]: one [`NodeRecord`] per node, keyed by [`NodeId`] —
//!   a reference count plus either a leaf's payload or a branch's stored
//!   memos and child table.
//! - [`HELD`]: presence rows registering live in-process handles, keyed by
//!   `(node, pin)` ID pairs. A row is one registration; releases delete
//!   the exact row, so releasing is idempotent under transaction retry
//!   and the committed-or-not ambiguity of an interrupted commit.
//! - [`GC`]: presence rows queuing nodes whose storage is reclaimable;
//!   drained in bounded steps by the custody layer
//!   ([`refcount`](super::refcount)).
//!
//! The liveness invariant, restored by every committed transaction: a
//! record's `strong` equals its durable parent edges plus one if the
//! canonical root names it; a node is reclaimable exactly when `strong`
//! is zero and no held row registers it.
//!
//! Records are borsh-encoded (canonical, like everything this crate
//! stores or ships). Node records embed structure the tree layer defined:
//! the compressed prefix, the leaf's exact `(Version, payload)` bytes, a
//! branch's memo fields — the same values the in-memory nodes memoize,
//! stored eagerly so a fetched handle answers every summary accessor
//! without I/O.
//!
//! # Corruption
//!
//! Corruption is environmental: bit rot, a torn write the store's
//! integrity layer missed, an operator's misdirected script. This layer
//! writes only canonical records, so bytes that fail to decode — or
//! decode to a shape no write produces, like a crossed bounds pair — are
//! evidence the store no longer holds what was written. Every decode
//! door here refuses such bytes with a [`Corruption`] naming the table
//! and key: the enclosing transaction applies nothing (the
//! [`checked`] views guarantee it), and the refusal
//! surfaces through the backend's error surface
//! ([`KvError::Corrupt`]) as its own genre,
//! distinct from a store that merely *failed* — the deployment can then
//! tell "retry against healthy storage" from "the stored replica lied",
//! and decide what recovery means. Detecting the torn page itself —
//! integrity below the byte level this layer reads — is the store's own
//! department, per the [`Kv`] contract.

use before::{Version, causally};
use borsh::{BorshDeserialize, BorshSerialize};

use super::checked;
use super::error::{Corruption, KvError};
use super::kv::{Kv, ReadTxn, Table, WriteTxn};
use crate::tree::typed::Hash;

/// The metadata table: [`ROOT_KEY`] and [`IDS_KEY`].
pub(crate) const META: Table = Table("rumors:meta");

/// The bulk node table: [`NodeRecord`]s keyed by [`NodeId::key`].
pub(crate) const NODES: Table = Table("rumors:nodes");

/// The held table: `(node, pin)` presence rows, keyed by
/// [`held_key`].
pub(crate) const HELD: Table = Table("rumors:held");

/// The reclamation queue: [`NodeId::key`] presence rows.
pub(crate) const GC: Table = Table("rumors:gc");

/// META key of the [`CanonicalRoot`] record.
pub(crate) const ROOT_KEY: &[u8] = b"root";

/// META key of the ID allocator's reserved ceiling (borsh `u64`).
pub(crate) const IDS_KEY: &[u8] = b"ids";

/// A store-allocated node identity: the `Arc` allocation's durable
/// analog.
///
/// IDs are never reused — the allocator's ceiling only grows — so a
/// stale reference to a reclaimed ID reads as *absent*, never as some
/// later node's row.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, BorshSerialize, BorshDeserialize,
)]
pub(crate) struct NodeId(pub(crate) u64);

impl NodeId {
    /// The row key: big-endian so byte order is numeric order and table
    /// scans walk IDs in allocation order.
    pub(crate) fn key(self) -> [u8; 8] {
        self.0.to_be_bytes()
    }

    /// Reads a [`key`](Self::key) back.
    ///
    /// # Errors
    ///
    /// [`Corruption`] on a key that is not exactly eight bytes: node-ID
    /// rows are written only by this module, so a malformed key in
    /// `table` is corruption (see the module docs).
    pub(crate) fn from_key(table: Table, key: &[u8]) -> Result<Self, Corruption> {
        Ok(NodeId(u64::from_be_bytes(key.try_into().map_err(
            |_| Corruption::new(table, key, "node-ID row key"),
        )?)))
    }
}

/// A held-table row key: one live registration of `node` by `pin`.
pub(crate) fn held_key(node: NodeId, pin: PinId) -> [u8; 16] {
    let mut key = [0; 16];
    key[..8].copy_from_slice(&node.key());
    key[8..].copy_from_slice(&pin.0.to_be_bytes());
    key
}

/// Splits a held-table row key back into `(node, pin)`.
///
/// # Errors
///
/// [`Corruption`] on a key that is not exactly sixteen bytes
/// (see the module docs).
pub(crate) fn split_held_key(key: &[u8]) -> Result<(NodeId, PinId), Corruption> {
    let (node, pin) = key
        .split_at_checked(8)
        .filter(|(_, pin)| pin.len() == 8)
        .ok_or_else(|| Corruption::new(HELD, key, "held row key"))?;
    Ok((
        NodeId(u64::from_be_bytes(node.try_into().expect("split at 8"))),
        PinId(u64::from_be_bytes(pin.try_into().expect("checked to 8"))),
    ))
}

/// A registration identity for one held-table row.
///
/// Allocated from the same ceiling as [`NodeId`]s (uniqueness is all that
/// matters), so a registration can be named before its transaction runs
/// and released idempotently after it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct PinId(pub(crate) u64);

/// The canonical-root record: the durable tree *and* the durable
/// identity, updated atomically in one row.
///
/// The identity record is authoritative for a persisting peer: a restart
/// reconstructs the minting clock from here alone, so every transaction
/// that changes what this peer may mint (a root flip carrying fresh local
/// versions; a party shrink donating a fork) writes this row in the same
/// transaction — or before the donation crosses the wire, for shrinks.
/// An absent identity means the peer holds none (cleared by retirement);
/// it is *not* "retain the previous record".
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Default)]
pub(crate) struct CanonicalRoot {
    /// The universe this replica belongs to, written by every root flip;
    /// `None` only in a store no peer has ever committed to.
    ///
    /// What lets a reopened store reconstruct the peer's network
    /// identity without a side channel that could disagree with the
    /// tree.
    pub(crate) network: Option<crate::Network>,
    /// The tree's incorporated ceiling (rides outside the nodes, exactly
    /// as in the in-memory root).
    pub(crate) ceiling: Version,
    /// The root node, absent for the empty tree.
    pub(crate) root: Option<NodeId>,
    /// The peer's identity record as its canonical clock encoding;
    /// `None` = holds no identity.
    ///
    /// Stored as bytes: the record layer can serialize and compare an
    /// identity but structurally cannot tick or join it (the alias handed
    /// to a commit is for recording only).
    pub(crate) identity: Option<Vec<u8>>,
}

/// One node's row: its reference count and its body.
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub(crate) struct NodeRecord {
    /// Durable references: parent edges plus the canonical-root edge.
    /// Live process handles are *not* counted here — they are held rows.
    pub(crate) strong: u64,
    pub(crate) body: NodeBody,
}

/// A node's stored structure: exactly what the in-memory node holds,
/// with memos eager and children as ID references.
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub(crate) enum NodeBody {
    Leaf {
        /// The compressed span above the leaf, **deepest byte at
        /// index 0** — the in-memory node's own storage order.
        ///
        /// Kept that way so hash preimages derive by one reversal into
        /// path order, exactly as the in-memory node hashes (hash
        /// agreement across backends is what session pruning rests
        /// on).
        prefix: Vec<u8>,
        version: Version,
        /// The message's exact serialized bytes (the borsh passthrough
        /// round-trips them without re-encoding).
        payload: Vec<u8>,
    },
    Branch {
        /// The compressed span above the branch, **deepest byte at index
        /// 0** (see the leaf arm's ordering note).
        prefix: Vec<u8>,
        /// The node hash at this record's full stored prefix.
        hash: Hash,
        /// The subtree's version bounds — floor as the span's meet,
        /// ceiling as its join — stored as the canonical composite span
        /// encoding.
        ///
        /// Decoding *is* the validating load door: borsh deserialization
        /// runs the fused one-pass parse that proves the pair ordered, so
        /// corrupt bytes and crossed pairs alike refuse the record (the
        /// module's corruption policy), and every span this store hands
        /// back upholds the ordering the classifiers rely on.
        bounds: causally::Span<'static>,
        /// Live leaves beneath (the `len` summary).
        leaves: u64,
        /// The largest bound encoding beneath (the `version_bytes`
        /// summary).
        version_bytes: u64,
        /// Ascending radix; ≥ 2 entries (path compression). Each edge
        /// stores the child's hash fat, so a virtual-level hash
        /// re-derivation needs no child fetches.
        children: Vec<(u8, NodeId, Hash)>,
    },
}

impl NodeRecord {
    /// Decodes `node`'s row value.
    ///
    /// # Errors
    ///
    /// [`Corruption`] on undecodable bytes — the [`bounds`] field's
    /// validating parse counts, so a crossed or concurrent stored pair
    /// refuses here exactly as truncated or garbled bytes do (see the
    /// module docs).
    ///
    /// [`bounds`]: NodeBody::Branch::bounds
    pub(crate) fn decode(node: NodeId, value: &[u8]) -> Result<Self, Corruption> {
        borsh::from_slice(value).map_err(|_| Corruption::new(NODES, &node.key(), "node record"))
    }

    /// Encodes this record as a row value.
    pub(crate) fn encode(&self) -> Vec<u8> {
        borsh::to_vec(self).expect("node record encoding is infallible")
    }

    /// The child IDs a branch references; empty for a leaf.
    pub(crate) fn children(&self) -> impl Iterator<Item = NodeId> + '_ {
        match &self.body {
            NodeBody::Leaf { .. } => [].iter(),
            NodeBody::Branch { children, .. } => children.as_slice().iter(),
        }
        .map(|(_, id, _)| *id)
    }
}

impl CanonicalRoot {
    /// Decodes the META root row, or the default (empty tree, no
    /// identity) when the row is absent.
    ///
    /// # Errors
    ///
    /// The transaction's own failure, or [`Corruption`] on undecodable
    /// bytes (see the module docs).
    pub(crate) fn read<T>(txn: &mut T) -> Result<Self, T::Error>
    where
        T: ReadTxn + ?Sized,
        T::Error: From<Corruption>,
    {
        Ok(txn
            .get(META, ROOT_KEY)?
            .map(|value| {
                borsh::from_slice(&value)
                    .map_err(|_| Corruption::new(META, ROOT_KEY, "canonical-root record"))
            })
            .transpose()?
            .unwrap_or_default())
    }

    /// Writes this record as the META root row.
    pub(crate) fn write<W: WriteTxn + ?Sized>(&self, txn: &mut W) -> Result<(), W::Error> {
        let value = borsh::to_vec(self).expect("canonical-root encoding is infallible");
        txn.put(META, ROOT_KEY, &value)
    }
}

/// How many IDs one reservation transaction claims.
///
/// Large enough that reservation is a vanishing fraction of node builds,
/// small enough that a crash's wasted remainder never matters against a
/// 64-bit space.
pub(crate) const ID_BLOCK: u64 = 1 << 20;

/// The ID allocator: block reservation against [`IDS_KEY`], handing out
/// fresh IDs *synchronously* from the block in hand.
///
/// A node or pin can therefore be named before the transaction that
/// stores it runs, which is what makes an interrupted insert reclaimable
/// garbage instead of a mystery.
#[derive(Debug, Default)]
pub(crate) struct IdAllocator {
    /// `(next, limit)` of the in-hand block; empty when equal.
    ///
    /// A tokio mutex: the slow path holds it across the reservation
    /// write, serializing concurrent exhaustions onto one block claim.
    block: tokio::sync::Mutex<(u64, u64)>,
}

impl IdAllocator {
    /// Allocates one fresh ID, reserving a new block when the in-hand one
    /// is exhausted. IDs are unique for the store's lifetime and never
    /// reused; a crash wastes the unconsumed remainder of the block.
    ///
    /// # Errors
    ///
    /// The store's failure, or [`Corruption`] when the ceiling row is
    /// undecodable or absurd — a ceiling within one block of the 64-bit
    /// space is unreachable by honest reservation (a block per
    /// nanosecond would take centuries), so an overflowing reservation
    /// is a rotted high byte, not exhaustion.
    pub(crate) async fn allocate<K: Kv>(&self, kv: &K) -> Result<u64, KvError<K::Error>> {
        let mut block = self.block.lock().await;
        if block.0 == block.1 {
            let reserved = checked::write(kv, |txn| {
                let floor = txn
                    .get(META, IDS_KEY)?
                    .map(|value| {
                        borsh::from_slice(&value)
                            .map_err(|_| Corruption::new(META, IDS_KEY, "ID-allocator record"))
                    })
                    .transpose()?
                    .unwrap_or(0u64);
                let ceiling = floor
                    .checked_add(ID_BLOCK)
                    .ok_or_else(|| Corruption::new(META, IDS_KEY, "ID-allocator ceiling"))?;
                txn.put(
                    META,
                    IDS_KEY,
                    &borsh::to_vec(&ceiling).expect("u64 encoding is infallible"),
                )?;
                Ok(floor)
            })
            .await?;
            *block = (reserved, reserved + ID_BLOCK);
        }
        let id = block.0;
        block.0 += 1;
        Ok(id)
    }
}

#[cfg(test)]
mod tests;
