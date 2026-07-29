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
        backend::{Backend, Leaf, Root},
        traverse::{
            act::Action,
            store::{self, walk::VersionBounds},
        },
        typed::{
            Path, Prefix,
            height::{self, Height, S, Z},
        },
    },
};

/// The local-operations capability of a backend: copy-on-write
/// construction, node identity, and the bulk tree operations, each an
/// overridable seam. See the [module docs](self).
pub trait Store<T: Send + Sync + 'static>: Backend<T>
where
    Self::Node<Z>: Leaf<T>,
{
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
    /// `on_action` fires once per *effectual* action — a leaf inserted,
    /// replaced, or removed — with that action's version, which is what
    /// lets the caller join versions only for actions that changed the
    /// tree.
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

    /// Stream the live leaves whose versions fall within `bounds`, in
    /// ascending path order, each keyed by its full 32-byte [`Key`].
    ///
    /// Subtrees wholly outside the range are pruned by their resident
    /// version bounds without being fetched.
    fn range(
        self,
        root: Option<Self::Node<height::Root>>,
        bounds: VersionBounds,
    ) -> impl Stream<Item = Result<(Key, Self::Node<Z>), Self::Error>> + Send {
        async_stream::try_stream! {
            let Some(node) = root else { return };
            let backend = self;
            let mut leaves =
                store::walk::Walk::walk(&backend, Prefix::new(), node, &bounds, false);
            while let Some(item) = leaves.next().await {
                let (prefix, leaf) = item?;
                yield (Key::from(prefix), leaf);
            }
        }
    }

    /// Persist the canonical root — and the party clock that stamps its
    /// versions — atomically.
    ///
    /// A replica calls this at every root-replacing commit, *before*
    /// publishing the new root to observers: for a persistent backend this
    /// is the root-flip transaction, and flipping the root and recording
    /// the clock in one transaction is what keeps the two from diverging
    /// at rest (a stale clock beside a persisted tree would re-mint used
    /// version coordinates on restart). The default does nothing: a
    /// backend whose tree lives and dies with the process has nothing to
    /// flip.
    fn commit(
        &self,
        root: &Root<Self, T>,
        party: Option<&before::Party>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send
    where
        Self: Sized,
    {
        let _ = (root, party);
        async { Ok(()) }
    }
}
