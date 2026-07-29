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
//! The backend trusts its own tables: it is the only writer, so a record
//! that fails to decode is an invariant violation (a bug here, or storage
//! the deployment let something else write), and the decode helpers
//! **panic** rather than launder corruption into a storage error the
//! caller would retry. Integrity below that — torn pages, bit rot — is
//! the store's own department, per the [`Kv`] contract.

use before::Version;
use borsh::{BorshDeserialize, BorshSerialize};

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
    /// # Panics
    ///
    /// On a malformed key: held and GC rows are written only by this
    /// module, so a bad key is corruption (see the module docs).
    pub(crate) fn from_key(key: &[u8]) -> Self {
        NodeId(u64::from_be_bytes(
            key.try_into().expect("malformed node-ID row key"),
        ))
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
/// # Panics
///
/// On a malformed key (corruption; see the module docs).
pub(crate) fn split_held_key(key: &[u8]) -> (NodeId, PinId) {
    assert_eq!(key.len(), 16, "malformed held row key");
    (
        NodeId::from_key(&key[..8]),
        PinId(u64::from_be_bytes(key[8..].try_into().unwrap())),
    )
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
    /// `None` only in a store no peer has ever committed to. What lets a
    /// reopened store reconstruct the peer's network identity without a
    /// side channel that could disagree with the tree.
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
        /// The compressed span above the leaf, **deepest byte at index
        /// 0** — the in-memory node's own order, kept so hash preimages
        /// feed [`Hash::leaf`]/[`Hash::branch`] byte-for-byte without a
        /// reversal (hash agreement across backends is what session
        /// pruning rests on).
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
        ceiling: Version,
        floor: Version,
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
    /// Decodes a row value.
    ///
    /// # Panics
    ///
    /// On undecodable bytes (corruption; see the module docs).
    pub(crate) fn decode(value: &[u8]) -> Self {
        borsh::from_slice(value).expect("corrupt node record")
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
    /// # Panics
    ///
    /// On undecodable bytes (corruption; see the module docs).
    pub(crate) fn read<T: ReadTxn + ?Sized>(txn: &mut T) -> Result<Self, T::Error> {
        Ok(txn
            .get(META, ROOT_KEY)?
            .map(|value| borsh::from_slice(&value).expect("corrupt canonical-root record"))
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
    pub(crate) async fn allocate<K: Kv>(&self, kv: &K) -> Result<u64, K::Error> {
        let mut block = self.block.lock().await;
        if block.0 == block.1 {
            let reserved = kv
                .write(|txn| {
                    let floor = txn
                        .get(META, IDS_KEY)?
                        .map(|value| {
                            borsh::from_slice(&value).expect("corrupt ID-allocator record")
                        })
                        .unwrap_or(0u64);
                    let ceiling = floor
                        .checked_add(ID_BLOCK)
                        .expect("node-ID space exhausted");
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
