//! The transactional key-value contract a persistence backend builds on.
//!
//! [`Kv`] is the boundary a deployment implements to give a peer durable
//! storage: a handful of named byte tables with atomic, serializable
//! read-write transactions over them. The crate's persistent tree backend
//! is generic over this trait the way sessions are generic over
//! [`Link`](crate::link::Link) — implement `Kv` for an embedded store and
//! every layer above (records, reference counting, recovery, the session
//! protocol) comes for free.
//!
//! The trait is deliberately small. Transactions take **synchronous**
//! closures over cursor-shaped views, which every embedded store can
//! provide directly (an object transaction, a locked map, a `spawn_blocking`
//! adapter around a blocking store), and the async entry points leave room
//! for stores whose commits genuinely suspend. There are no typed tables,
//! no iterators borrowed out of a transaction, and no durability dial
//! beyond [`sync`](Kv::sync): everything else lives above this boundary.
//!
//! # The transaction contract
//!
//! These clauses are what the persistent backend's crash-consistency
//! argument rests on; the `conformance` feature's kv suite probes what a
//! black box can reach, and the rest is the implementation's documented
//! obligation.
//!
//! - **Atomicity.** A [`write`](Kv::write) whose closure returns `Err`
//!   applies nothing. A `write` whose future is dropped before completion
//!   is *committed-or-not*: it may have applied in full (an adapter that
//!   offloads its commit cannot be un-asked) or not at all, but never
//!   partially.
//! - **Serializability.** Concurrent `write` transactions behave as if run
//!   one after another. [`read`](Kv::read) sees some committed state, never
//!   a write in progress.
//! - **Prefix consistency after a crash.** The state that survives a crash
//!   is the state after some *prefix* of the committed transaction
//!   sequence. How long that prefix is — whether an acknowledged commit
//!   survives — is the store's documented durability policy;
//!   [`sync`](Kv::sync) is the barrier for callers that need an answer
//!   before proceeding. Prefix consistency is not optional: the backend's
//!   "the canonical root always points at a fully persisted tree" guarantee
//!   is a theorem over it, in every durability mode.
//! - **Re-execution.** A transaction closure may run more than once (an
//!   optimistically-concurrent store retries on conflict). Closures must
//!   route every effect through the transaction argument and tolerate
//!   re-execution from scratch.
//!
//! Single-process ownership is *not* a clause of this trait: it is the
//! persistent backend's own usage requirement (it caches, counts
//! references, and recovers on the assumption that no other process
//! touches the tables it owns). A `Kv` implementation shared with other
//! tables and other processes is fine; the backend's tables are not.
//!
//! # When not to implement this
//!
//! [`Memory`](super::Memory) is the reference implementation and the
//! right store for tests and simulations. If the goal is only "the
//! message set outlives the process" and an embedded database is already
//! in the deployment, wrap it here. If there is no such store, reach for
//! one before considering a bespoke log: the contract above is exactly the
//! hard part of storage engines, and this crate deliberately does not
//! re-implement it.

use std::future::Future;

/// A named keyspace within a store.
///
/// The persistent backend uses a small fixed set of tables; the name is a
/// stable identifier an implementation may map to anything with the same
/// semantics (a named tree, a key prefix, a column family). Two distinct
/// tables never observe each other's keys.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Table(pub &'static str);

/// The read operations available inside any transaction.
///
/// Both operations take `&mut self` so an implementation may drive an
/// internal cursor or buffer; neither borrows from the transaction beyond
/// the call, so no lifetime escapes into the caller.
pub trait ReadTxn {
    /// The store's error type, shared with its [`Kv`].
    type Error;

    /// The value at `key` in `table`, or `None` when absent.
    fn get(&mut self, table: Table, key: &[u8]) -> Result<Option<Vec<u8>>, Self::Error>;

    /// The first entry whose key is strictly greater than `after`
    /// (`None` starts from the beginning), in ascending byte order —
    /// a cursor, not an iterator, so implementations lend nothing.
    ///
    /// Yields `None` when no greater key exists. Repeatedly feeding the
    /// returned key back as `after` visits every entry of the table
    /// exactly once, in order, and terminates.
    // The inline `(key, value)` tuple is clearer than a minted alias
    // that would carry no meaning of its own.
    #[allow(clippy::type_complexity)]
    fn next_after(
        &mut self,
        table: Table,
        after: Option<&[u8]>,
    ) -> Result<Option<(Vec<u8>, Vec<u8>)>, Self::Error>;
}

/// The mutations available inside a write transaction.
///
/// Mutations become visible to this transaction's own reads immediately
/// (read-your-writes) and to other transactions only if the commit
/// succeeds, atomically.
pub trait WriteTxn: ReadTxn {
    /// Sets `key` to `value` in `table`, replacing any prior value.
    fn put(&mut self, table: Table, key: &[u8], value: &[u8]) -> Result<(), Self::Error>;

    /// Removes `key` from `table`; removing an absent key is a no-op.
    fn delete(&mut self, table: Table, key: &[u8]) -> Result<(), Self::Error>;
}

/// An abstract transactional key-value store: what a persistence backend
/// requires of a deployment.
///
/// A `Kv` value is a cheap cloneable *handle* to one store, in the same
/// sense as every backend handle in this crate. See the [module
/// docs](self) for the transaction contract implementations must honor.
///
/// Transaction closures receive their view as `&mut dyn` — one virtual
/// call per operation, noise against any real storage — so an
/// implementation's concrete transaction type is free to borrow from the
/// store, own it, or be a `spawn_blocking` envelope, without that choice
/// surfacing in the trait.
pub trait Kv: Clone + Send + Sync + 'static {
    /// The store's error type.
    ///
    /// The bound lets a storage failure surface through the crate's public
    /// error enums as a `thiserror` source.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Runs one read-only transaction over a committed snapshot.
    ///
    /// The closure may run more than once (see the module docs) and its
    /// `Err` aborts the transaction, surfacing as this call's `Err`.
    fn read<R, F>(&self, f: F) -> impl Future<Output = Result<R, Self::Error>> + Send
    where
        R: Send + 'static,
        F: FnMut(&mut dyn ReadTxn<Error = Self::Error>) -> Result<R, Self::Error> + Send + 'static;

    /// Runs one atomic, serializable read-write transaction.
    ///
    /// The closure may run more than once; its `Err` aborts with nothing
    /// applied. If the returned future is dropped before resolving, the
    /// transaction is committed-or-not, never partial.
    fn write<R, F>(&self, f: F) -> impl Future<Output = Result<R, Self::Error>> + Send
    where
        R: Send + 'static,
        F: FnMut(&mut dyn WriteTxn<Error = Self::Error>) -> Result<R, Self::Error> + Send + 'static;

    /// A durability barrier: resolves when every previously acknowledged
    /// commit is as durable as this store gets.
    ///
    /// The default is an immediate no-op, correct for stores whose commits
    /// are already durable when acknowledged. A store with a weaker
    /// default (write-behind, group commit) overrides this with its
    /// flush.
    fn sync(&self) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async { Ok(()) }
    }
}
