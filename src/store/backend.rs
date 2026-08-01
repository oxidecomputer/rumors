//! The persistent tree backend: [`KvBackend`], a [`Store`] over any [`Kv`].
//!
//! One decoded record per resident node, shared through a weak dedup map;
//! thin height-typed views over it; custody riding inside every handle.
//! The layers below ([`schema`](super::schema), [`refcount`])
//! own the rows; this module owns what a *handle* is.
//!
//! # Views and virtual levels
//!
//! A stored record materializes a whole compressed span: its prefix bytes
//! are the single-child levels path compression elides. A node handle is a
//! view of one record at one height — an `Arc` of the decoded record plus
//! an *offset* counting how many span bytes the view has consumed — so
//! exploding through a compressed span mints views of the same allocation
//! with zero I/O, exactly as the in-memory backend re-wraps one node. Each
//! view carries its own position-relative hash, derived at mint time from
//! the record alone (the remaining span, and for a branch the stored child
//! edges — fat `(radix, id, hash)` entries exist precisely so no child is
//! fetched to re-derive a hash).
//!
//! # Pending nodes
//!
//! [`Leaf::leaf`] constructs without a backend value in scope, and every
//! rebuilt spine passes through one [`Backend::parent`] call per level —
//! so construction stages *pending* nodes in memory and installs a record
//! only when something durable references one: a multi-child branch
//! installs its children (and itself) eagerly, and [`Store::commit`]
//! installs the root it flips to. A chain of single-child extensions
//! therefore collapses to exactly one record per compressed span, the
//! same shape the in-memory tree holds, and the store never writes a row
//! that no surviving tree references. Two invariants make pending nodes
//! sound:
//!
//! - **Every child edge in any body is already installed.** A branch
//!   assembles only after its children persist, so a pending body — always
//!   a single-child *extension* of some base — copies edges that are
//!   already durable, kept alive by the stored base record's own edges.
//! - **The custody chain bottoms at durability.** A pending handle keeps
//!   its base handle alive; the chain ends at a stored record (whose
//!   registration the chain holds, and whose edges keep everything beneath
//!   it) or a pending leaf (which owns its payload outright). Dropping the
//!   chain drops, at worst, unreferenced staged memory.
//!
//! # Custody
//!
//! A handle minted from storage registers in the held table (or rides an
//! ancestor's registration — fetching children through an exploded parent
//! shares the entry handle's row). `Drop` cannot await, so a dropped
//! registration queues its release; every write transaction the backend
//! runs piggybacks a bounded flush of that queue and a bounded reclamation
//! step, and [`vacuum`](KvBackend::vacuum) drains both to empty. Recovery
//! on open sweeps the whole held table: exclusive ownership — at most one
//! live backend per store's tables, in-process handles included — makes
//! every held row dead process state by definition.
//!
//! # Corruption
//!
//! Per the [schema policy](super::schema), a record that fails to decode
//! — or one whose shape disagrees with the height of the edge naming it —
//! refuses with [`Corruption`], surfacing through this backend's error
//! type ([`KvError`]) as its own genre so a deployment can tell a store
//! that failed from a store that lied. The refusal applies nothing: the
//! [`checked`] transaction views buffer every mutation
//! until the enclosing closure succeeds. A child edge pointing at an
//! *absent* row is different — records are reachable only through
//! counted edges and registrations, so absence means the custody
//! accounting itself was violated (a second backend swept this one's
//! registrations; see [`Kv`]'s exclusive-ownership requirement), and the
//! fetch panics as that detector.

use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, Weak};

use borsh::BorshDeserialize;
use bytes::Bytes;
use futures::future;

use before::{Version, causally};

use crate::{
    Network,
    message::Message,
    tree::{
        backend::{Backend, Leaf, LeafWalk, Node, NodeStream, Root, Store, VersionBounds, ranged},
        typed::{
            Hash, Prefix,
            height::{self, Height, S, Z},
        },
    },
};

use super::checked::{self, CheckedWrite};
use super::error::{Corruption, KvError};
use super::kv::Kv;
use super::refcount::{self, ReleaseQueue};
use super::schema::{
    CanonicalRoot, IdAllocator, META, NODES, NodeBody, NodeId, NodeRecord, PinId, ROOT_KEY,
};

/// The persistent tree backend: a cheap cloneable handle over one [`Kv`]
/// store's tables.
///
/// Construct one with [`new`](Self::new) around any conforming store, then
/// hand it to [`Peer::seed_in`](crate::Peer::seed_in),
/// [`Bootstrap::backend`](crate::Bootstrap::backend), or — for a store that
/// already holds a replica — [`Peer::open`](crate::Peer::open). The backend
/// owns its tables outright (see [`Kv`] on exclusive ownership) and
/// delegates durability to the store: an acknowledged commit is as durable
/// as the store's own commits are, and the crate awaits the store's
/// [`sync`](Kv::sync) barrier wherever durability is load-bearing (before
/// identity leaves the replica). Call [`vacuum`](Self::vacuum) in
/// maintenance windows to drain deferred reclamation eagerly; an active
/// replica converges without it, one bounded step per write.
///
/// # Space
///
/// Redacted and superseded content is reclaimed by reference counting,
/// and every live in-process handle is a reference: a long-held
/// [`Snapshot`](crate::Snapshot) (or an observer parked mid-walk) keeps
/// its whole tree version registered in the store, so the space of
/// everything that version reaches waits on the handle's release. Hold
/// snapshots for as long as you need them — the cost is storage, never
/// correctness — but a deployment sizing its store should count the
/// oldest snapshot it keeps alive, not just the live set. Releases queue
/// when handles drop and settle into the store one bounded step per
/// write; [`vacuum`](Self::vacuum) is the eager drain.
pub struct KvBackend<K: Kv, T: Send + Sync + 'static> {
    shared: Arc<Shared<K, T>>,
}

impl<K: Kv, T: Send + Sync + 'static> Clone for KvBackend<K, T> {
    fn clone(&self) -> Self {
        Self {
            shared: self.shared.clone(),
        }
    }
}

impl<K: Kv, T: Send + Sync + 'static> std::fmt::Debug for KvBackend<K, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KvBackend").finish_non_exhaustive()
    }
}

/// The backend's shared state: the store handle, the ID allocator, the
/// deferred-release queue, the resident-node dedup map, and the wire
/// barrier's watermarks.
struct Shared<K: Kv, T: Send + Sync + 'static> {
    kv: K,
    ids: IdAllocator,
    releases: ReleaseQueue,
    /// One resident allocation per node: stored-record fetches funnel
    /// through here, so every traversal holding a node shares one
    /// [`Fetched`].
    dedup: Mutex<HashMap<u64, Weak<Fetched<K, T>>>>,
    /// Identity-bearing commits acknowledged: every root flip and
    /// identity record bumps this after its transaction acknowledges.
    /// One half of the wire barrier's watermark pair (see
    /// [`Store::barrier`]'s implementation below).
    committed: AtomicU64,
    /// The `committed` value some completed [`Kv::sync`] has covered:
    /// the other half of the watermark pair. `barrier` compares the two
    /// so a commit-free window costs no flush.
    durable: AtomicU64,
}

/// One resident node: the decoded record (or staged pending body) plus
/// its custody.
struct Fetched<K: Kv, T: Send + Sync + 'static> {
    body: Body<T>,
    provenance: Provenance<K, T>,
}

/// What a resident node is backed by.
enum Provenance<K: Kv, T: Send + Sync + 'static> {
    /// A stored record, with whatever keeps this handle's view of it
    /// registered.
    Stored {
        id: NodeId,
        /// Held for its `Drop`: what keeps the registration (own or an
        /// ancestor's) alive as long as this node is resident.
        #[allow(dead_code)]
        custody: Custody<K, T>,
    },
    /// A staged construction: installed on first
    /// [`persisted`](Fetched::persisted), memoized here. `base` is the
    /// custody chain (see the module docs); `None` only for a staged
    /// leaf, which owns its payload outright.
    Pending {
        /// Held for its `Drop`: the custody chain (module docs) that
        /// keeps everything this staged body references alive.
        #[allow(dead_code)]
        base: Option<Arc<Fetched<K, T>>>,
        installed: OnceLock<(NodeId, Registration<K, T>)>,
    },
}

/// What keeps a stored node registered in the held table.
enum Custody<K: Kv, T: Send + Sync + 'static> {
    /// This handle's own `(node, pin)` row.
    Registered(#[allow(dead_code)] Registration<K, T>),
    /// An ancestor entry handle's registration.
    ///
    /// Children fetched through an exploded parent stay alive through
    /// the entry that reached them: records are immutable, so every
    /// durable edge beneath a registered entry persists while the
    /// registration does.
    Under(#[allow(dead_code)] Arc<Fetched<K, T>>),
}

/// One held-table registration; dropping it queues the release (`Drop`
/// cannot await, so deregistration is deferred to the backend's next
/// write transaction or vacuum).
struct Registration<K: Kv, T: Send + Sync + 'static> {
    node: NodeId,
    pin: PinId,
    shared: Arc<Shared<K, T>>,
}

impl<K: Kv, T: Send + Sync + 'static> Drop for Registration<K, T> {
    fn drop(&mut self) {
        self.shared.releases.push(self.node, self.pin);
    }
}

/// A node's decoded structure, resident in the handle.
///
/// The prefix rides in the record's own order (deepest byte at index 0,
/// the in-memory node's convention) so hash preimages feed the shared
/// digests byte-for-byte.
enum Body<T> {
    Leaf {
        prefix: Vec<u8>,
        version: Version,
        message: Message<T>,
    },
    Branch {
        prefix: Vec<u8>,
        /// The record's hash at offset zero (its full stored prefix).
        hash: Hash,
        /// The stored bounds span, already proven ordered by the record
        /// decode's fused parse; [`Node::span`] reborrows it.
        bounds: causally::Span<'static>,
        leaves: u64,
        version_bytes: u64,
        /// Ascending radix, ≥ 2 entries; hashes stored fat per edge.
        children: Vec<(u8, NodeId, Hash)>,
    },
}

impl<T: Send + Sync + 'static> Body<T> {
    /// The stored span (deepest byte first).
    fn prefix(&self) -> &[u8] {
        match self {
            Body::Leaf { prefix, .. } | Body::Branch { prefix, .. } => prefix,
        }
    }

    /// The position-relative hash of the view that has consumed
    /// `offset` span bytes.
    ///
    /// The leaf preimage over the remaining span, or the branch preimage
    /// over the remaining span and the stored edges; the offset-zero
    /// branch hash is the stored field.
    ///
    /// Preimages take the span in *path order* (shallowest byte first)
    /// while storage is shallowest-last, exactly as the in-memory node
    /// hashes; [`path_order`] is the shared reversal.
    fn view_hash(&self, offset: usize) -> Hash {
        match self {
            Body::Leaf { prefix, .. } => Hash::leaf(&path_order(&prefix[..prefix.len() - offset])),
            Body::Branch {
                prefix,
                hash,
                children,
                ..
            } => {
                if offset == 0 {
                    *hash
                } else {
                    Hash::branch(
                        &path_order(&prefix[..prefix.len() - offset]),
                        children.iter().map(|&(radix, _, hash)| (radix, hash)),
                    )
                }
            }
        }
    }

    /// Decode `node`'s stored record body.
    ///
    /// # Errors
    ///
    /// [`Corruption`] on an undecodable leaf payload (module docs).
    fn decode(node: NodeId, body: NodeBody) -> Result<Self, Corruption>
    where
        T: BorshDeserialize,
    {
        Ok(match body {
            NodeBody::Leaf {
                prefix,
                version,
                payload,
            } => Body::Leaf {
                prefix,
                version,
                message: Message::from_bytes(Bytes::from(payload))
                    .map_err(|_| Corruption::new(NODES, &node.key(), "leaf payload"))?,
            },
            NodeBody::Branch {
                prefix,
                hash,
                bounds,
                leaves,
                version_bytes,
                children,
            } => Body::Branch {
                prefix,
                hash,
                bounds,
                leaves,
                version_bytes,
                children,
            },
        })
    }

    /// Encode this body as its stored record form.
    fn encode(&self) -> NodeBody {
        match self {
            Body::Leaf {
                prefix,
                version,
                message,
            } => NodeBody::Leaf {
                prefix: prefix.clone(),
                version: version.clone(),
                payload: message.bytes().to_vec(),
            },
            Body::Branch {
                prefix,
                hash,
                bounds,
                leaves,
                version_bytes,
                children,
            } => NodeBody::Branch {
                prefix: prefix.clone(),
                hash: *hash,
                bounds: bounds.clone(),
                leaves: *leaves,
                version_bytes: *version_bytes,
                children: children.clone(),
            },
        }
    }

    /// This body extended one level upward: the same structure with
    /// `radix` as its new shallowest span byte, re-hashed at the new
    /// position. Cheap: the payload and edge vector are shared or small.
    fn extend(&self, radix: u8) -> Self {
        match self {
            Body::Leaf {
                prefix,
                version,
                message,
            } => {
                let mut prefix = prefix.clone();
                prefix.push(radix);
                Body::Leaf {
                    prefix,
                    version: version.clone(),
                    message: message.clone(),
                }
            }
            Body::Branch {
                prefix,
                bounds,
                leaves,
                version_bytes,
                children,
                ..
            } => {
                let mut prefix = prefix.clone();
                prefix.push(radix);
                let hash = Hash::branch(
                    &path_order(&prefix),
                    children.iter().map(|&(r, _, h)| (r, h)),
                );
                Body::Branch {
                    prefix,
                    hash,
                    bounds: bounds.clone(),
                    leaves: *leaves,
                    version_bytes: *version_bytes,
                    children: children.clone(),
                }
            }
        }
    }
}

/// A stored span reversed into path order (shallowest byte first): the
/// order every hash preimage takes, per the in-memory node's convention.
/// A span never exceeds the 32-byte path.
fn path_order(stored: &[u8]) -> tinyvec::ArrayVec<[u8; 32]> {
    stored.iter().rev().copied().collect()
}

impl<K: Kv, T: Send + Sync + 'static> Fetched<K, T>
where
    T: BorshDeserialize,
{
    /// The installed record ID of this node's offset-zero view, installing
    /// a pending body on first need.
    ///
    /// Installation is one bounded transaction: the record, its fresh
    /// held-table registration, and one strong increment per child edge.
    /// Concurrent callers may race to install; the loser's registration
    /// drops into the release queue and its duplicate record is reclaimed
    /// — wasted bytes, never a dangling reference.
    async fn persisted(
        self: &Arc<Self>,
        backend: &KvBackend<K, T>,
    ) -> Result<NodeId, KvError<K::Error>> {
        match &self.provenance {
            Provenance::Stored { id, .. } => Ok(*id),
            Provenance::Pending { installed, .. } => {
                if let Some((id, _)) = installed.get() {
                    return Ok(*id);
                }
                let shared = &backend.shared;
                let node = NodeId(shared.ids.allocate(&shared.kv).await?);
                let pin = PinId(shared.ids.allocate(&shared.kv).await?);
                // Arm the registration before the transaction: dropped at
                // any await point below, it queues a release that is a
                // no-op if the install never committed and reclaims the
                // orphan if it did.
                let registration = Registration {
                    node,
                    pin,
                    shared: shared.clone(),
                };
                let record = NodeRecord {
                    strong: 0,
                    body: self.body.encode(),
                };
                let children: Vec<NodeId> = record.children().collect();
                backend
                    .write_upkeep(move |txn| {
                        refcount::install(txn, node, pin, &record)?;
                        for &child in &children {
                            refcount::adjust_strong(txn, child, 1)?;
                        }
                        Ok(())
                    })
                    .await?;
                // A concurrent install may have won; either way the
                // memoized entry is what every later caller reads, and a
                // losing registration drops here, queuing its duplicate
                // for reclamation.
                let _ = installed.set((node, registration));
                let (id, _) = installed.get().expect("just set");
                shared
                    .dedup
                    .lock()
                    .expect("dedup lock poisoned")
                    .insert(id.0, Arc::downgrade(self));
                Ok(*id)
            }
        }
    }
}

/// A height-typed view of one resident node.
///
/// `offset` counts consumed span bytes; views along one compressed span
/// share the [`Fetched`] allocation. The hash is derived at mint time
/// (see the module docs), so every summary accessor answers without I/O.
pub struct KvNode<K: Kv, T: Send + Sync + 'static, H: Height> {
    inner: Arc<Fetched<K, T>>,
    offset: usize,
    hash: Hash,
    height: PhantomData<fn() -> H>,
}

impl<K: Kv, T: Send + Sync + 'static, H: Height> Clone for KvNode<K, T, H> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            offset: self.offset,
            hash: self.hash,
            height: PhantomData,
        }
    }
}

impl<K: Kv, T: Send + Sync + 'static, H: Height> std::fmt::Debug for KvNode<K, T, H> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KvNode")
            .field("height", &H::HEIGHT)
            .field("offset", &self.offset)
            .field("hash", &self.hash)
            .finish_non_exhaustive()
    }
}

impl<K: Kv, T: Send + Sync + 'static, H: Height> KvNode<K, T, H> {
    /// Mint the view of `inner` that has consumed `offset` span bytes.
    fn view(inner: Arc<Fetched<K, T>>, offset: usize) -> Self
    where
        T: BorshDeserialize,
    {
        let hash = inner.body.view_hash(offset);
        Self {
            inner,
            offset,
            hash,
            height: PhantomData,
        }
    }

    /// How many span bytes remain below this view.
    fn remaining(&self) -> usize {
        self.inner.body.prefix().len() - self.offset
    }

    /// The next radix along the span, when any span remains: the byte an
    /// explosion at this view descends through.
    fn span_radix(&self) -> Option<u8> {
        let prefix = self.inner.body.prefix();
        (self.remaining() > 0).then(|| prefix[prefix.len() - 1 - self.offset])
    }
}

#[cfg(test)]
impl<K: Kv, T: Send + Sync + 'static + borsh::BorshDeserialize, H: Height> KvNode<K, T, H> {
    /// The actual bytes this view keeps resident, measured for the
    /// conformance suite's census: the view, the decoded record's fixed
    /// part, and the record's heap (span, bounds encodings, edge table).
    ///
    /// Leaf payload bytes are deliberately excluded, mirroring
    /// [`Backend::node_bytes`]'s account: custody happens at
    /// [`Leaf::leaf`] and in-flight payload is priced by
    /// `target_message_size`, so neither side of the pointwise comparison
    /// counts it.
    pub(crate) fn resident_bytes(&self) -> usize {
        use crate::tree::backend::Node as _;
        let span = self.span();
        let heap = self.inner.body.prefix().len()
            + span.meet().as_bytes().len()
            + span.join().as_bytes().len()
            + match &self.inner.body {
                Body::Leaf { .. } => 0,
                Body::Branch { children, .. } => {
                    children.len() * std::mem::size_of::<(u8, NodeId, Hash)>()
                }
            };
        std::mem::size_of::<Self>() + std::mem::size_of::<Fetched<K, T>>() + heap
    }
}

impl<K: Kv, T: Send + Sync + 'static, H: Height> Node<T> for KvNode<K, T, H>
where
    T: BorshDeserialize,
{
    type Backend = KvBackend<K, T>;
    type Height = H;

    fn span(&self) -> causally::Span<'_> {
        match &self.inner.body {
            // A leaf's bounds coincide at its version: the trusted door is
            // exactly the in-memory leaf's own answer, a structural
            // guarantee rather than a loaded pair.
            Body::Leaf { version, .. } => causally::Span::new_unchecked(version, version),
            // A branch reborrows the stored span the record decode already
            // proved ordered — the load-time validation is the fused borsh
            // parse in the schema layer, so no per-read check is owed here.
            Body::Branch { bounds, .. } => bounds.reborrow(),
        }
    }

    fn hash(&self) -> Hash {
        self.hash
    }

    fn len(&self) -> usize {
        match &self.inner.body {
            Body::Leaf { .. } => 1,
            Body::Branch { leaves, .. } => *leaves as usize,
        }
    }

    fn version_bytes(&self) -> usize {
        match &self.inner.body {
            Body::Leaf { version, .. } => version.as_bytes().len(),
            Body::Branch { version_bytes, .. } => *version_bytes as usize,
        }
    }
}

impl<K: Kv, T: Send + Sync + 'static> Leaf<T> for KvNode<K, T, Z>
where
    T: BorshDeserialize,
{
    fn message(&self) -> &Message<T> {
        match &self.inner.body {
            Body::Leaf { message, .. } => message,
            // Structural: every stored record enters a walk through the
            // shape door ([`coherent`]), which places a branch body
            // strictly above height zero, and construction never builds
            // one at it — so a height-zero branch view cannot be minted.
            Body::Branch { .. } => unreachable!("height-zero view over a branch record"),
        }
    }

    fn version(&self) -> &Version {
        match &self.inner.body {
            Body::Leaf { version, .. } => version,
            Body::Branch { .. } => unreachable!("height-zero view over a branch record"),
        }
    }

    fn leaf(
        version: Version,
        message: Message<T>,
    ) -> impl Future<Output = Result<Self, KvError<K::Error>>> + Send {
        // No backend value is in scope, so the leaf stages as a pending
        // body — the write-behind shape this method's contract sanctions —
        // and installs when a branch or root flip links it. A session
        // dropped mid-decode drops staged memory and nothing else; a
        // meanwhile-redacted message correctly never re-arrives. Staged
        // handles are priced through `node_bytes` at `children = 0`.
        future::ready(Ok(KvNode::view(
            Arc::new(Fetched {
                body: Body::Leaf {
                    prefix: Vec::new(),
                    version,
                    message,
                },
                provenance: Provenance::Pending {
                    base: None,
                    installed: OnceLock::new(),
                },
            }),
            0,
        )))
    }
}

impl<K: Kv, T: Send + Sync + 'static> KvBackend<K, T>
where
    T: BorshDeserialize,
{
    /// Wrap a conforming store.
    ///
    /// Cheap and non-destructive: nothing is read or written until the
    /// backend is used ([`Peer::open`](crate::Peer::open) is what runs
    /// recovery against a store holding an earlier replica).
    pub fn new(kv: K) -> Self {
        Self {
            shared: Arc::new(Shared {
                kv,
                ids: IdAllocator::default(),
                releases: ReleaseQueue::default(),
                dedup: Mutex::new(HashMap::new()),
                committed: AtomicU64::new(0),
                durable: AtomicU64::new(0),
            }),
        }
    }

    /// Flush queued handle releases and drain deferred reclamation until
    /// both are empty.
    ///
    /// Maintenance, never required for correctness: every write
    /// transaction piggybacks one bounded step of each, so an active
    /// replica converges on its own, and recovery on open reclaims
    /// whatever a crash left. Run it in idle windows to return space
    /// eagerly, or in tests to reach the quiescent state audits expect.
    ///
    /// Keep a clone of the backend you constructed the peer with — the
    /// handle is cheap, and it is the vacuum entry point:
    ///
    /// ```
    /// use rumors::{KvBackend, Memory, Peer};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let store = KvBackend::new(Memory::default());
    /// let peer: Peer<String, _, _> = Peer::seed_in(store.clone());
    /// let rumors = peer.into_rumors();
    ///
    /// rumors.send("here today".to_string()).await?;
    /// // Snapshots taken along the way register what they reach; space
    /// // for anything later redacted returns after they drop.
    ///
    /// // An idle window: drain queued releases and reclamation to empty.
    /// store.vacuum().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn vacuum(&self) -> Result<(), KvError<K::Error>> {
        refcount::vacuum(&self.shared.kv, &self.shared.releases).await
    }

    /// One write transaction with the custody upkeep every backend write
    /// piggybacks: a bounded flush of queued releases and one bounded
    /// reclamation step.
    ///
    /// The taken release batch is re-queued unless the transaction
    /// positively committed — releases are idempotent, so re-applying a
    /// batch whose fate was never learned is safe, while forgetting one
    /// would leak until recovery.
    async fn write_upkeep<R, F>(&self, mut f: F) -> Result<R, KvError<K::Error>>
    where
        R: Send + 'static,
        F: FnMut(&mut CheckedWrite<'_, K::Error>) -> Result<R, KvError<K::Error>> + Send + 'static,
    {
        /// Re-queues a taken batch on drop unless defused: covers the
        /// `Err` return and the dropped-mid-await (committed-or-not)
        /// case in one place.
        struct Requeue<'q> {
            queue: &'q ReleaseQueue,
            batch: Option<Vec<(NodeId, PinId)>>,
        }
        impl Drop for Requeue<'_> {
            fn drop(&mut self) {
                if let Some(batch) = self.batch.take() {
                    self.queue.requeue(batch);
                }
            }
        }

        let batch = self.shared.releases.take();
        let mut guard = Requeue {
            queue: &self.shared.releases,
            batch: Some(batch.clone()),
        };
        let result = checked::write(&self.shared.kv, move |txn| {
            let result = f(txn)?;
            refcount::release(txn, &batch)?;
            refcount::reclaim_step(txn)?;
            Ok(result)
        })
        .await;
        if result.is_ok() {
            guard.batch = None;
        }
        result
    }

    /// Fetch one stored record as a resident node, through the dedup
    /// funnel; `custody` is what keeps the returned node registered.
    ///
    /// # Errors
    ///
    /// The store's failure, or [`Corruption`] when the record's bytes
    /// or leaf payload fail to decode — or when the record is *absent*:
    /// every caller resolves an ID out of a strong edge or a
    /// registration, so within one live backend absence is unreachable
    /// ([`Kv`]'s exclusive-ownership requirement; see the module docs),
    /// and the refusal is the custody detector for a second backend
    /// sweeping this one's registrations.
    async fn fetch(
        &self,
        id: NodeId,
        custody: impl FnOnce() -> Custody<K, T>,
    ) -> Result<Arc<Fetched<K, T>>, KvError<K::Error>> {
        if let Some(resident) = self
            .shared
            .dedup
            .lock()
            .expect("dedup lock poisoned")
            .get(&id.0)
            .and_then(Weak::upgrade)
        {
            return Ok(resident);
        }
        let record = checked::read(&self.shared.kv, move |txn| refcount::read_node(txn, id))
            .await?
            .ok_or_else(|| {
                Corruption::new(NODES, &id.key(), "node record (a live edge references it)")
            })?;
        let fetched = Arc::new(Fetched {
            body: Body::decode(id, record.body)?,
            provenance: Provenance::Stored {
                id,
                custody: custody(),
            },
        });
        // A concurrent fetch may have inserted first; prefer the resident
        // one so every holder shares a single allocation.
        let mut dedup = self.shared.dedup.lock().expect("dedup lock poisoned");
        if let Some(resident) = dedup.get(&id.0).and_then(Weak::upgrade) {
            return Ok(resident);
        }
        dedup.insert(id.0, Arc::downgrade(&fetched));
        Ok(fetched)
    }

    /// Register a fresh held-table row on `id` and fetch it as an entry
    /// handle: how a root enters the process from storage.
    async fn fetch_entry(&self, id: NodeId) -> Result<Arc<Fetched<K, T>>, KvError<K::Error>> {
        let pin = PinId(self.shared.ids.allocate(&self.shared.kv).await?);
        let registration = Registration {
            node: id,
            pin,
            shared: self.shared.clone(),
        };
        self.write_upkeep(move |txn| refcount::register(txn, id, pin))
            .await?;
        self.fetch(id, move || Custody::Registered(registration))
            .await
    }
}

impl<K: Kv, T> Backend<T> for KvBackend<K, T>
where
    T: BorshDeserialize + Send + Sync + 'static,
{
    type Node<H: Height> = KvNode<K, T, H>;
    type Error = KvError<K::Error>;

    fn node_bytes(children: usize, version_bound: usize) -> usize {
        // What one view keeps resident: the view itself, its share of the
        // decoded record (counted once per handle, an upper bound over
        // span-sharing), the span capacity, the stored hash, the bounds'
        // encodings, and the fat child table. Leaf payloads are priced by
        // `target_message_size` (custody happens at `Leaf::leaf`), not
        // here — the contract's `children = 0` clause. Constant slack
        // covers the dedup entry and one queued release. Monotone in both
        // arguments by construction.
        const SPAN_CAPACITY: usize = 32;
        const SLACK: usize = 64;
        std::mem::size_of::<KvNode<K, T, Z>>()
            + std::mem::size_of::<Fetched<K, T>>()
            + SPAN_CAPACITY
            + std::mem::size_of::<Hash>()
            + version_bound
            + children * std::mem::size_of::<(u8, NodeId, Hash)>()
            + SLACK
    }

    async fn parent<H>(
        self,
        _prefix: Prefix<S<H>>,
        children: Vec<(u8, Option<Self::Node<H>>)>,
    ) -> Result<Option<Self::Node<S<H>>>, Self::Error>
    where
        H: Height,
        S<H>: Height,
    {
        {
            let mut survivors: Vec<(u8, KvNode<K, T, H>)> = children
                .into_iter()
                .filter_map(|(radix, child)| Some((radix, child?)))
                .collect();
            match survivors.len() {
                // Every child deleted (or the group was empty): the
                // deletion cascades one level up. Reclamation of what the
                // dropped children stored is the custody layer's, keyed on
                // the last reference — never this call's.
                0 => Ok(None),
                // One survivor: the parent is the child one level up —
                // path compression. A mid-span view un-consumes one byte
                // with zero I/O; a whole view extends its body in memory,
                // staying pending until something durable links it.
                1 => {
                    let (radix, child) = survivors.pop().expect("one survivor");
                    if child.offset > 0 {
                        debug_assert_eq!(
                            child.span_radix_above(),
                            radix,
                            "a mid-span child reassembles at the radix it exploded from"
                        );
                        Ok(Some(KvNode::view(child.inner, child.offset - 1)))
                    } else {
                        let body = child.inner.body.extend(radix);
                        Ok(Some(KvNode::view(
                            Arc::new(Fetched {
                                body,
                                provenance: Provenance::Pending {
                                    base: Some(child.inner),
                                    installed: OnceLock::new(),
                                },
                            }),
                            0,
                        )))
                    }
                }
                // A real branch: persist the children (each install is its
                // own bounded transaction), then install the branch record
                // with one strong edge per child.
                _ => {
                    let mut edges = Vec::with_capacity(survivors.len());
                    let mut spans = Vec::with_capacity(survivors.len());
                    let mut leaves: u64 = 0;
                    let mut version_bytes: usize = 0;
                    // Guards from mid-span reifications, held until the
                    // branch install below commits the edges that keep
                    // their records alive.
                    let mut guards = Vec::new();
                    for (radix, child) in &survivors {
                        let (id, guard) = child.reified(&self).await?;
                        guards.extend(guard);
                        edges.push((*radix, id, child.hash()));
                        spans.push(child.span());
                        leaves += child.len() as u64;
                        version_bytes = version_bytes.max(child.version_bytes());
                    }
                    // The parent's bounds are the hull of the children's
                    // spans: meet of meets, join of joins, by-ref balanced
                    // folds over the borrowing spans.
                    let floor = Version::meet_all(spans.iter().map(causally::Span::meet))
                        .expect("at least two children");
                    let ceiling = Version::join_all(spans.iter().map(causally::Span::join));
                    drop(spans);
                    version_bytes = version_bytes
                        .max(ceiling.as_bytes().len())
                        .max(floor.as_bytes().len());
                    let bounds = floor.span(&ceiling);
                    let hash = Hash::branch(&[], edges.iter().map(|&(r, _, h)| (r, h)));
                    let body = Body::Branch {
                        prefix: Vec::new(),
                        hash,
                        bounds,
                        leaves,
                        version_bytes: version_bytes as u64,
                        children: edges,
                    };
                    let fetched = Arc::new(Fetched {
                        body,
                        provenance: Provenance::Pending {
                            base: None,
                            installed: OnceLock::new(),
                        },
                    });
                    // Install eagerly: the branch's children are durable,
                    // and linking them under a durable parent is what
                    // frees their staged custody to unwind. The children's
                    // handles (`survivors`) and the reification guards
                    // keep every edge registered until this commits.
                    fetched.persisted(&self).await?;
                    drop(guards);
                    Ok(Some(KvNode::view(fetched, 0)))
                }
            }
        }
    }

    fn children<H>(
        self,
        prefix: Prefix<S<H>>,
        parent: Self::Node<S<H>>,
    ) -> impl NodeStream<Self, T, H>
    where
        H: Height,
        S<H>: Height,
    {
        async_stream::try_stream! {
            if let Some(radix) = parent.span_radix() {
                // Mid-span: one virtual child, same allocation, zero I/O.
                let child = KvNode::view(parent.inner.clone(), parent.offset + 1);
                yield (prefix.push(radix), child);
                return;
            }
            let edges = match &parent.inner.body {
                Body::Leaf { .. } => unreachable!(
                    "a leaf record's span ends at height zero, where nothing explodes"
                ),
                Body::Branch { children, .. } => children.clone(),
            };
            for (radix, id, _) in edges {
                let child = self
                    .fetch(id, || Custody::Under(parent.inner.clone()))
                    .await?;
                coherent::<T, H>(id, &child.body)?;
                yield (prefix.push(radix), KvNode::view(child, 0));
            }
        }
    }
}

/// The shape door for stored records entering a walk.
///
/// A record fetched through an edge at height `H` must place its body
/// exactly where the edge claims — a leaf's stored span ends at height
/// zero (`prefix.len() == H`), a branch's strictly above it
/// (`prefix.len() < H`).
///
/// Every record enters the process through this check (child fetches
/// and the root load), so the height-typed views above it can trust
/// their shape structurally: it is what keeps the height-zero leaf
/// accessors' branch arm unreachable.
///
/// # Errors
///
/// [`Corruption`] naming the record whose span disagrees with its edge.
fn coherent<T, H: Height>(node: NodeId, body: &Body<T>) -> Result<(), Corruption> {
    let sound = match body {
        Body::Leaf { prefix, .. } => prefix.len() == H::HEIGHT,
        Body::Branch { prefix, .. } => prefix.len() < H::HEIGHT,
    };
    if sound {
        Ok(())
    } else {
        Err(Corruption::new(
            NODES,
            &node.key(),
            "node record shape (its span disagrees with the edge naming it)",
        ))
    }
}

impl<K: Kv, T: Send + Sync + 'static, H: Height> KvNode<K, T, H>
where
    T: BorshDeserialize,
{
    /// The byte one level above a mid-span view: what
    /// [`Backend::parent`]'s compression case asserts against.
    fn span_radix_above(&self) -> u8 {
        let prefix = self.inner.body.prefix();
        prefix[prefix.len() - self.offset]
    }

    /// The installed record ID for this view's position, plus the guard
    /// that keeps it registered.
    ///
    /// An offset-zero view persists its own record (a no-op when already
    /// stored); the caller's handle keeps it registered, so the guard is
    /// empty. A mid-span view names a *position* inside a record, which
    /// no edge can reference — so it reifies: a fresh record holding the
    /// remaining span, its edges re-counted (or its payload shared), the
    /// analog of the in-memory backend's copy-on-write span split. The
    /// returned guard carries that fresh record's registration, and the
    /// caller **must hold it across the transaction that links the ID**:
    /// dropped earlier, the release can ride an intermediate
    /// transaction's upkeep and reclaim the record before anything
    /// references it.
    async fn reified(
        &self,
        backend: &KvBackend<K, T>,
    ) -> Result<(NodeId, Option<Arc<Fetched<K, T>>>), KvError<K::Error>> {
        if self.offset == 0 {
            return Ok((self.inner.persisted(backend).await?, None));
        }
        let body = match &self.inner.body {
            Body::Leaf {
                prefix,
                version,
                message,
            } => Body::Leaf {
                prefix: prefix[..prefix.len() - self.offset].to_vec(),
                version: version.clone(),
                message: message.clone(),
            },
            Body::Branch {
                prefix,
                bounds,
                leaves,
                version_bytes,
                children,
                ..
            } => {
                let prefix = prefix[..prefix.len() - self.offset].to_vec();
                let hash = Hash::branch(
                    &path_order(&prefix),
                    children.iter().map(|&(r, _, h)| (r, h)),
                );
                Body::Branch {
                    prefix,
                    hash,
                    bounds: bounds.clone(),
                    leaves: *leaves,
                    version_bytes: *version_bytes,
                    children: children.clone(),
                }
            }
        };
        let split = Arc::new(Fetched {
            body,
            provenance: Provenance::Pending {
                base: Some(self.inner.clone()),
                installed: OnceLock::new(),
            },
        });
        let id = split.persisted(backend).await?;
        Ok((id, Some(split)))
    }
}

impl<K: Kv, T> Store<T> for KvBackend<K, T>
where
    T: BorshDeserialize + Send + Sync + 'static,
{
    /// Every root flip records durably; committers prepare the identity
    /// clock.
    const PERSISTS: bool = true;

    /// The boxed generic walk: every leaf resolves through store reads.
    type Walk = LeafWalk<T, Self>;

    fn range(self, root: Option<Self::Node<height::Root>>, bounds: VersionBounds) -> Self::Walk {
        ranged(self, root, bounds)
    }

    // The mutation seams keep their generic tower defaults but box the
    // entry: the towers are `BoxFuture`-per-level internally, yet the
    // entry frame's locals would otherwise inline into every public
    // future that awaits a commit (`tests/future_size.rs` pins the
    // budget).
    fn act<F>(
        self,
        root: Option<Self::Node<crate::tree::typed::height::Root>>,
        actions: Vec<(
            crate::tree::typed::Path,
            Version,
            crate::tree::backend::Action<T>,
        )>,
        on_action: F,
    ) -> impl Future<
        Output = Result<Option<Self::Node<crate::tree::typed::height::Root>>, Self::Error>,
    > + Send
    where
        F: FnMut(&Version) + Send,
    {
        Box::pin(
            async move { crate::tree::traverse::store::act(&self, root, actions, on_action).await },
        )
    }

    fn join(
        self,
        a: Option<Self::Node<crate::tree::typed::height::Root>>,
        b: Option<Self::Node<crate::tree::typed::height::Root>>,
        a_version: &Version,
        b_version: &Version,
        changed: &mut bool,
    ) -> impl Future<
        Output = Result<Option<Self::Node<crate::tree::typed::height::Root>>, Self::Error>,
    > + Send {
        Box::pin(async move {
            crate::tree::traverse::store::join(&self, a, b, a_version, b_version, changed).await
        })
    }

    fn same<H: Height>(a: &Self::Node<H>, b: &Self::Node<H>) -> bool {
        // One resident allocation per node (the dedup funnel), so shared
        // structure is shared `Fetched`s; the offset distinguishes views
        // along one span. `false` falls back to the hash, as the contract
        // requires.
        Arc::ptr_eq(&a.inner, &b.inner) && a.offset == b.offset
    }

    async fn child<H>(
        self,
        _prefix: Prefix<S<H>>,
        parent: Self::Node<S<H>>,
        radix: u8,
    ) -> Result<Option<Self::Node<H>>, Self::Error>
    where
        H: Height,
        S<H>: Height,
    {
        {
            if let Some(span) = parent.span_radix() {
                // Mid-span: the single virtual child either matches the
                // requested radix or nothing does.
                return Ok(
                    (span == radix).then(|| KvNode::view(parent.inner.clone(), parent.offset + 1))
                );
            }
            let edge = match &parent.inner.body {
                Body::Leaf { .. } => {
                    unreachable!("a leaf record's span ends at height zero, where nothing explodes")
                }
                Body::Branch { children, .. } => children
                    .binary_search_by_key(&radix, |&(r, _, _)| r)
                    .ok()
                    .map(|found| children[found].1),
            };
            match edge {
                None => Ok(None),
                Some(id) => {
                    let child = self
                        .fetch(id, || Custody::Under(parent.inner.clone()))
                        .await?;
                    coherent::<T, H>(id, &child.body)?;
                    Ok(Some(KvNode::view(child, 0)))
                }
            }
        }
    }

    fn commit(
        &self,
        root: &Root<Self, T>,
        clock: Option<before::Clock>,
        network: Network,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        // Boxed: the flip's locals must not inline into every public
        // future that awaits a commit (`tests/future_size.rs`).
        let this = self.clone();
        let root: Root<Self, T> = Root {
            ceiling: root.ceiling.clone(),
            root: root.root.clone(),
        };
        Box::pin(async move {
            let this = &this;
            let root = &root;
            // Persist the root's record first (a no-op when it is already
            // stored); a crash between the two transactions leaves an
            // orphan the recovery sweep reclaims, never a flip naming an
            // absent record.
            let (id, guard) = match &root.root {
                None => (None, None),
                Some(node) => {
                    let (id, guard) = node.reified(this).await?;
                    (Some(id), guard)
                }
            };
            let ceiling = root.ceiling.clone();
            // The one sanctioned use of the alias: serialization. The
            // record layer stores bytes it structurally cannot tick or
            // join.
            let identity =
                clock.map(|clock| borsh::to_vec(&clock).expect("clock encoding is infallible"));
            let flipped = this
                .write_upkeep(move |txn| {
                    refcount::flip_root(txn, network, id, ceiling.clone(), identity.clone())
                })
                .await;
            // The reification guard held the root's fresh record
            // registered through the flip that just linked it.
            drop(guard);
            if flipped.is_ok() {
                // Acknowledged before the committer publishes: any
                // snapshot that can carry this flip's versions is taken
                // after the bump, so a later `barrier` sees it.
                this.shared.committed.fetch_add(1, Ordering::Release);
            }
            flipped
        })
    }

    fn record(
        &self,
        clock: Option<before::Clock>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        let identity =
            clock.map(|clock| borsh::to_vec(&clock).expect("clock encoding is infallible"));
        let this = self.clone();
        // Boxed for the same layout reason as `commit`.
        Box::pin(async move {
            this.write_upkeep(move |txn| refcount::record_identity(txn, identity.clone()))
                .await?;
            this.shared.committed.fetch_add(1, Ordering::Release);
            Ok(())
        })
    }

    fn barrier(&self) -> impl Future<Output = Result<(), Self::Error>> + Send {
        let this = self.clone();
        async move {
            // The watermark pair makes the barrier idle-cheap: flush
            // only when an identity-bearing commit postdates the last
            // completed flush, so a caller may barrier before every
            // transmission and pay at most one [`Kv::sync`] per
            // commit-then-send window — sessions after local-only quiet
            // wait on nothing. The count is snapshotted *before* the
            // flush: commits acknowledged mid-flush may not be covered
            // by it, and must not be marked durable here.
            let mark = this.shared.committed.load(Ordering::Acquire);
            if this.shared.durable.load(Ordering::Acquire) < mark {
                this.shared.kv.sync().await.map_err(KvError::Store)?;
                this.shared.durable.fetch_max(mark, Ordering::AcqRel);
            }
            Ok(())
        }
    }
}

/// Why a store could not be [opened](crate::Peer::open) as a peer.
#[derive(Debug, thiserror::Error)]
pub enum OpenError<E> {
    /// The store holds no replica: no peer has ever committed to it.
    ///
    /// Seed or bootstrap into the store instead
    /// ([`Peer::seed_in`](crate::Peer::seed_in),
    /// [`Bootstrap::backend`](crate::Bootstrap::backend)); opening is for
    /// resuming a replica that already lives there.
    #[error("the store holds no replica: seed or bootstrap into it first")]
    Empty,

    /// The replica in this store retired: it donated its whole identity
    /// away, so no peer can resume from it.
    ///
    /// The stored content is an archive of what the replica held when it
    /// left the universe; a new participant joins by bootstrapping from a
    /// live peer, never by resurrecting a retiree.
    #[error("the stored replica retired; its identity lives elsewhere")]
    Retired,

    /// The store served bytes no peer ever wrote: the stored replica is
    /// corrupt, and reopening it cannot go better on retry.
    ///
    /// The payload names the table and key. What recovery means is the
    /// deployment's call — and content needs no backup ritual: any other
    /// replica of the set restores it through ordinary gossip (the crate
    /// docs' storage section).
    #[error(transparent)]
    Corrupt(Corruption),

    /// The store itself failed.
    #[error(transparent)]
    Storage(E),
}

/// Splits a backend failure into [`OpenError`]'s two storage-genre arms.
impl<E> From<KvError<E>> for OpenError<E> {
    fn from(error: KvError<E>) -> Self {
        match error {
            KvError::Store(error) => OpenError::Storage(error),
            KvError::Corrupt(corruption) => OpenError::Corrupt(corruption),
        }
    }
}

impl<K: Kv, T> KvBackend<K, T>
where
    T: BorshDeserialize + Send + Sync + 'static,
{
    /// Recover this store and load the replica it holds: the pieces
    /// [`Peer::open`](crate::Peer::open) assembles.
    ///
    /// Runs the recovery sweep (every held row is dead process state),
    /// then reads the canonical record: the network, the identity clock,
    /// and the tree root as a freshly registered entry handle.
    pub(crate) async fn open_replica(
        &self,
    ) -> Result<(Network, before::Clock, Root<Self, T>), OpenError<K::Error>> {
        refcount::recover(&self.shared.kv).await?;
        let record = checked::read(&self.shared.kv, |txn| CanonicalRoot::read(txn)).await?;
        let Some(network) = record.network else {
            return Err(OpenError::Empty);
        };
        let Some(identity) = record.identity else {
            return Err(OpenError::Retired);
        };
        let clock: before::Clock = borsh::from_slice(&identity)
            .map_err(|_| OpenError::Corrupt(Corruption::new(META, ROOT_KEY, "identity clock")))?;
        let root = match record.root {
            None => None,
            Some(id) => {
                let entry = self.fetch_entry(id).await?;
                coherent::<T, height::Root>(id, &entry.body).map_err(KvError::from)?;
                Some(KvNode::view(entry, 0))
            }
        };
        Ok((
            network,
            clock,
            Root {
                ceiling: record.ceiling,
                root,
            },
        ))
    }
}

#[cfg(test)]
mod tests;
