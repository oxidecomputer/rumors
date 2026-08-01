//! The local-operations capability of a backend: what a resident tree
//! requires beyond session service.
//!
//! [`Backend`] is the *session* contract — priced custody of in-flight node
//! references, exploded and reassembled on the mirror's schedule. [`Store`]
//! is the *replica* contract layered above it: copy-on-write construction
//! and identity for the tree a peer holds and mutates locally. The two are
//! separate traits because several backends serve sessions without ever
//! holding a replica — the conformance suite's decorators, a test's
//! fault-injection wrapper — and forcing local mutation on them would be
//! wrong by construction.
//!
//! Every operation is an overridable seam over a generic default, the same
//! discipline as [`Backend::leaves`] and [`Backend::assemble`]: the
//! defaults recurse through [`Backend::children`] / [`Backend::parent`]
//! (the towers in `traverse::store`), and a backend overrides where its own
//! storage answers better. [`Local`](super::Local) overrides everything
//! with the synchronous in-memory engines; a persistent backend overrides
//! [`child`](Store::child) with a record read and [`commit`](Store::commit)
//! with its root-flip transaction. The crate's backend conformance suite
//! pins every override against the defaults by differential proptest.

use std::pin::pin;

use futures::{Stream, StreamExt as _};

use crate::{
    Version,
    tree::{
        Key,
        backend::{Action, Backend, Leaf, LeafWalk, Root, VersionBounds},
        traverse::store,
        typed::{
            Path, Prefix,
            height::{self, Height, S, Z},
        },
    },
};

/// The local-operations capability of a backend: copy-on-write
/// construction, node identity, and the bulk tree operations, each an
/// overridable seam. See the [module docs](self).
///
/// # Handle custody
///
/// Every node handle a `Store` mints — through construction, fetch, or
/// any seam here — must stay valid until the last clone of that handle
/// drops, however long the holder keeps it: replicas clone roots out of
/// their published state into snapshots, observers, and sessions, and
/// none of those holders re-registers with the backend. A backend whose
/// nodes reference storage it can reclaim must therefore make its handles
/// self-protecting (registration travels inside the handle), and since
/// `Drop` cannot await, release is *deferred*: a dropped handle queues its
/// deregistration for the backend's next transaction rather than
/// performing I/O.
pub trait Store<T: Send + Sync + 'static>: Backend<T, Node<Z>: Leaf<T>> {
    /// Whether [`commit`](Self::commit) records anything durable.
    ///
    /// `false` — the default — promises `commit` is a no-op, and licenses
    /// committers to skip preparing the identity clock it would carry (the
    /// preparation aliases the party, work worth skipping on every
    /// in-memory commit). A backend that overrides `commit` sets this
    /// `true`.
    const PERSISTS: bool = false;

    /// Whether two handles name the same backend allocation.
    ///
    /// The `Arc::ptr_eq` analog: **sufficient, not necessary**, for
    /// structural equality — forked trees share their unchanged subtrees,
    /// so a merge can short-circuit them in `O(1)` before falling back to
    /// the content hash. Meaningful only between handles minted by the
    /// same backend instance; `false` must always fall back to the hash.
    fn same<H: Height>(a: &Self::Node<H>, b: &Self::Node<H>) -> bool;

    /// The child of `parent` at `radix`, or `None`.
    ///
    /// The default filters the [`children`](Backend::children) stream; a
    /// backend whose handles carry the child table resident answers with a
    /// point lookup instead.
    fn child<H>(
        self,
        prefix: Prefix<S<H>>,
        parent: Self::Node<S<H>>,
        radix: u8,
    ) -> impl Future<Output = Result<Option<Self::Node<H>>, Self::Error>> + Send
    where
        H: Height,
        S<H>: Height,
    {
        async move {
            let mut children = pin!(self.children(prefix, parent));
            while let Some(item) = children.next().await {
                let (prefix, child) = item?;
                let (_, at) = prefix.pop();
                // The stream is strictly ascending: past the radix, no
                // match can follow.
                match at.cmp(&radix) {
                    std::cmp::Ordering::Less => continue,
                    std::cmp::Ordering::Equal => return Ok(Some(child)),
                    std::cmp::Ordering::Greater => return Ok(None),
                }
            }
            Ok(None)
        }
    }

    /// Apply one action batch to the (possibly absent) root, returning the
    /// rebuilt root.
    ///
    /// `on_action` fires at most once per *leaf path* the batch touches,
    /// with the join of every version aimed at that path — skipped versions
    /// included. It fires whenever the path held a leaf before or holds one
    /// after; only a path that never existed and stayed empty observes
    /// nothing. The caller joins the observed versions into its causal
    /// ceiling, so a batch whose every action lands on never-existed paths
    /// and deletes nothing advances nothing.
    fn act<F>(
        self,
        root: Option<Self::Node<height::Root>>,
        actions: Vec<(Path, Version, Action<T>)>,
        on_action: F,
    ) -> impl Future<Output = Result<Option<Self::Node<height::Root>>, Self::Error>> + Send
    where
        F: FnMut(&Version) + Send,
    {
        async move { store::act(&self, root, actions, on_action).await }
    }

    /// Merge two trees resident in this backend, honoring deletions by
    /// version dominance (see [`fn@store::join::join`]).
    ///
    /// Both roots must have been minted by this backend instance:
    /// [`same`](Self::same) is meaningful only within one store, and a
    /// tree from elsewhere enters through a mirror session, never here.
    ///
    /// `changed` is set — never cleared — iff the merged result's content
    /// differs from `a`'s, decided exactly by the traversal itself with no
    /// hashing; the full contract is stated at [`fn@store::join::join`].
    fn join(
        self,
        a: Option<Self::Node<height::Root>>,
        b: Option<Self::Node<height::Root>>,
        a_version: &Version,
        b_version: &Version,
        changed: &mut bool,
    ) -> impl Future<Output = Result<Option<Self::Node<height::Root>>, Self::Error>> + Send {
        async move { store::join(&self, a, b, a_version, b_version, changed).await }
    }

    /// Look up the live leaf at `path`, as a bare height-zero handle;
    /// `None` when no live leaf sits there.
    fn get(
        self,
        root: Option<Self::Node<height::Root>>,
        path: Path,
    ) -> impl Future<Output = Result<Option<Self::Node<Z>>, Self::Error>> + Send {
        async move { store::get::get(&self, root, path).await }
    }

    /// The concrete stream [`range`](Self::range) returns: what the read
    /// surfaces ([`Messages`](crate::Messages), the observers) hold across
    /// polls.
    ///
    /// A nameable type rather than an opaque return so each backend picks
    /// its own dispatch: the in-memory backend walks synchronously with no
    /// per-item box or vcall, and a storage-owning backend keeps the boxed
    /// async walk ([`ranged`] is the one-line default body for it).
    type Walk: Stream<Item = Result<(Key, Self::Node<Z>), Self::Error>> + Send + Unpin + 'static;

    /// Stream the live leaves whose versions fall within `bounds`, in
    /// ascending path order, each keyed by its full 32-byte [`Key`].
    ///
    /// Subtrees wholly outside the range are pruned by their resident
    /// version bounds without being fetched.
    fn range(self, root: Option<Self::Node<height::Root>>, bounds: VersionBounds) -> Self::Walk;

    /// Persist the canonical root — and the identity clock that stamps its
    /// versions — atomically.
    ///
    /// A replica calls this at every root-replacing commit, *before*
    /// publishing the new root to observers: for a persistent backend this
    /// is the root-flip transaction, and flipping the root and recording
    /// the clock in one transaction is what keeps the two from diverging
    /// at rest (a stale clock beside a persisted tree would re-mint used
    /// version coordinates on restart). `clock` pairs an alias of the
    /// committing party with the built root's frontier — exactly the
    /// record a restart needs to keep minting past everything stored —
    /// and is `None` only when the replica holds no identity (a
    /// retirement's donated party is in flight). The default does
    /// nothing, and [`PERSISTS`](Self::PERSISTS) tells committers so: a
    /// backend whose tree lives and dies with the process has nothing to
    /// flip.
    ///
    /// The backend must only *serialize* the received clock — never tick
    /// it, join it with another record, or otherwise treat it as a live
    /// identity. It is an alias of the committing party, and the aliasing
    /// contract tolerates exactly one live copy; recording is the one
    /// sanctioned use.
    ///
    /// The recorded clock may lag the live party *subset-ward*: party
    /// growth (reclaiming a bookmarked region, recovering an absorbed
    /// donation) is deliberately lock-free and can land between a
    /// commit's clock capture and its transaction. That lag is an
    /// invariant, not a defect — a crash then costs a benignly leaked
    /// region, never a duplicated one — and the next root flip records
    /// the grown identity. An implementation must not compensate by
    /// consulting or merging any other identity record.
    fn commit(
        &self,
        root: &Root<Self, T>,
        clock: Option<before::Clock>,
        network: crate::Network,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send
    where
        Self: Sized,
    {
        let _ = (root, clock, network);
        async { Ok(()) }
    }

    /// Record the identity clock alone, leaving the stored tree untouched:
    /// the party-*shrink* write.
    ///
    /// A replica donating identity (serving a bootstrap's fork, retiring
    /// its whole party) calls this with the post-shrink clock **before the
    /// donation crosses the wire** — the storage analog of slicing the
    /// donation out of the bookmark. Once the donated region is in the
    /// counterparty's hands, no crash of this process may resurrect it; a
    /// record that still contained the donation would do exactly that on
    /// restart. `None` records that the replica holds no identity (a
    /// retirement's whole-party donation is about to ship); an aborted
    /// donation then re-joins the party *in memory* and the record lags
    /// subset-ward until the next write — the sanctioned direction (see
    /// [`commit`](Self::commit)).
    ///
    /// The alias and staleness obligations of [`commit`](Self::commit)
    /// apply verbatim; the default does nothing, which
    /// [`PERSISTS`](Self::PERSISTS) licenses.
    fn record(
        &self,
        clock: Option<before::Clock>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        let _ = clock;
        async { Ok(()) }
    }

    /// A durability barrier: resolves when every acknowledged
    /// [`commit`](Self::commit) and [`record`](Self::record) is as durable
    /// as this backend gets.
    ///
    /// A replica calls this before *transmitting* own-party versions to a
    /// counterparty: the identity record covering a version must survive a
    /// crash once some other replica holds messages stamped with it, or a
    /// restart could re-mint the coordinate (the record's durability
    /// policy is the backend's — see the transaction contract it builds
    /// on — and this barrier is how a session waits it out). The default
    /// does nothing, correct wherever `commit` is (the in-memory backend,
    /// or a store whose commits are durable when acknowledged).
    fn barrier(&self) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async { Ok(()) }
    }
}

/// The boxed generic walk: the one-line [`Store::range`] body for a backend
/// whose leaves resolve through async fetches.
///
/// Streams the live leaves under `root` whose versions fall within
/// `bounds`, in ascending path order, pruning by resident version bounds —
/// the walk tower shared by every backend that does not override the
/// traversal itself.
pub(crate) fn ranged<S, T>(
    backend: S,
    root: Option<S::Node<height::Root>>,
    bounds: VersionBounds,
) -> LeafWalk<T, S>
where
    S: Store<T>,
    T: Send + Sync + 'static,
{
    Box::pin(async_stream::try_stream! {
        let Some(node) = root else { return };
        let mut leaves =
            store::walk::Walk::walk(&backend, Prefix::new(), node, &bounds, false);
        while let Some(item) = leaves.next().await {
            let (prefix, leaf) = item?;
            yield (Key::from(prefix), leaf);
        }
    })
}
