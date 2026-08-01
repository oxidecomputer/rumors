//! The backend-generic merge: one simultaneous recursion over two trees
//! resident in the same backend.
//!
//! The generic twin of [`fn@crate::tree::traverse::join`],
//! with the same four-case analysis per node pair. Equal subtrees are
//! pruned by [`equivalent`] — backend identity first, content hash second —
//! before any child is fetched, which is what keeps a small delta against a
//! large shared tree costing work proportional to the delta. One-sided
//! subtrees delegate to the same deletion-honoring
//! [`mod@super::unknown`] filter the mirror uses.
//!
//! Both trees must be resident in the *same* backend instance:
//! [`Store::same`] is meaningful only between handles the same store
//! minted, and `false` merely falls back to the hash comparison. The one
//! production merge site satisfies this by construction — the reconciled
//! root it joins came out of a session started with this replica's own
//! backend handle.

use futures::future::BoxFuture;

use crate::{
    Version,
    tree::{
        backend::{Leaf, Node, Store, children_of},
        traverse::store::unknown::{self, Unknown},
        typed::{
            Prefix,
            height::{self, Height, S, Z},
        },
    },
};

/// Whether two same-store subtrees are structurally equal.
///
/// Backend identity short-circuits (the common case for forked trees,
/// hash-free), and the content hash decides otherwise — equal hash ⟹
/// equal subtree, by content addressing.
pub fn equivalent<B, T, H>(a: &B::Node<H>, b: &B::Node<H>) -> bool
where
    B: Store<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
{
    B::same(a, b) || a.hash() == b.hash()
}

/// Merge two same-store trees rooted at `a` and `b` into one.
///
/// `a_version` / `b_version` are the two roots' version vectors, used to
/// honor deletions: a subtree one side lacks while its version is `<=` that
/// side's vector was deleted there, and is dropped.
///
/// `changed` is set — never cleared — iff the merged result's content
/// differs from `a`'s: some leaf was gained from `b`, or some leaf of `a`
/// was dropped by deletion honoring. The recursion decides this exactly,
/// with no hashing, through the same mechanics as the in-memory
/// [`traverse::join`](fn@crate::tree::traverse::join): a gain is a subtree of
/// `b` surviving the deletion filter where `a` held nothing, and a drop is
/// the filter's own shed observation on `a`'s side. Gains and drops live at
/// distinct content-addressed paths and each is monotone at its path, so
/// they cannot cancel: an untouched flag really means the merged tree is
/// `a`, content-identical, equal root hash.
pub async fn join<B, T>(
    backend: &B,
    a: Option<B::Node<height::Root>>,
    b: Option<B::Node<height::Root>>,
    a_version: &Version,
    b_version: &Version,
    changed: &mut bool,
) -> Result<Option<B::Node<height::Root>>, B::Error>
where
    B: Store<T, Node<Z>: Leaf<T>> + Sync,
    T: Send + Sync + 'static,
{
    Join::join(backend, Prefix::new(), a, b, a_version, b_version, changed).await
}

/// The inductive step of the generic merge, implemented per [`Height`]; see
/// the module docs for the four-case analysis each level performs.
///
/// Each step upholds the [`join`] free function's `changed` contract: set
/// on any gain from `b` or any deletion-honoring drop from `a`, left
/// alone when the result is content-identical to `a`.
pub trait Join: Unknown {
    fn join<'a, B, T>(
        backend: &'a B,
        prefix: Prefix<Self>,
        a: Option<B::Node<Self>>,
        b: Option<B::Node<Self>>,
        a_version: &'a Version,
        b_version: &'a Version,
        changed: &'a mut bool,
    ) -> BoxFuture<'a, Result<Option<B::Node<Self>>, B::Error>>
    where
        B: Store<T, Node<Z>: Leaf<T>> + Sync,
        T: Send + Sync + 'static;
}

impl<H: Join> Join for S<H>
where
    S<H>: Height,
{
    fn join<'a, B, T>(
        backend: &'a B,
        prefix: Prefix<S<H>>,
        a: Option<B::Node<S<H>>>,
        b: Option<B::Node<S<H>>>,
        a_version: &'a Version,
        b_version: &'a Version,
        changed: &'a mut bool,
    ) -> BoxFuture<'a, Result<Option<B::Node<S<H>>>, B::Error>>
    where
        B: Store<T, Node<Z>: Leaf<T>> + Sync,
        T: Send + Sync + 'static,
    {
        Box::pin(async move {
            let (ours, theirs) = match (a, b) {
                (None, None) => return Ok(None),
                // Asymmetric cases: a subtree one side holds and the other
                // lacks. Filter it against the *other* side's version vector
                // to honor deletions: causally-known subtrees the other side
                // lacks were deleted there, and drop out.
                //
                // On our side, the filter only ever *removes* leaves, and its
                // shed observation counts each drop exactly: any observation
                // is a change. On their side, any survivor at all is a gain
                // (we held nothing here).
                (Some(ours), None) => {
                    let mut shed = |dropped| *changed |= dropped > 0;
                    return unknown::unknown(backend, b_version, prefix, ours, &mut shed).await;
                }
                (None, Some(theirs)) => {
                    let gained =
                        unknown::unknown(backend, a_version, prefix, theirs, &mut |_| {}).await?;
                    *changed |= gained.is_some();
                    return Ok(gained);
                }
                (Some(ours), Some(theirs)) => {
                    // Identical subtrees: keep one. Nothing is learned on
                    // either side across an equal subtree.
                    if equivalent::<B, T, _>(&ours, &theirs) {
                        return Ok(Some(ours));
                    }
                    (ours, theirs)
                }
            };

            // Differing subtrees: descend one level, but recurse only into
            // the radixes that actually diverge — every equivalent child
            // carries over verbatim, ours kept.
            let ours = children_of(backend, prefix, ours).await?;
            let theirs = children_of(backend, prefix, theirs).await?;

            let mut group = Vec::with_capacity(ours.len().max(theirs.len()));
            let mut ours = ours.into_iter().peekable();
            let mut theirs = theirs.into_iter().peekable();
            loop {
                // Merge-walk the two ascending fans by radix.
                let (radix, our_child, their_child) = match (ours.peek(), theirs.peek()) {
                    (None, None) => break,
                    (Some((r, _)), None) => (*r, ours.next().map(|(_, c)| c), None),
                    (None, Some((r, _))) => (*r, None, theirs.next().map(|(_, c)| c)),
                    (Some((ro, _)), Some((rt, _))) => {
                        let radix = (*ro).min(*rt);
                        (
                            radix,
                            ours.next_if(|(r, _)| *r == radix).map(|(_, c)| c),
                            theirs.next_if(|(r, _)| *r == radix).map(|(_, c)| c),
                        )
                    }
                };

                if let (Some(our_child), Some(their_child)) = (&our_child, &their_child)
                    && equivalent::<B, T, _>(our_child, their_child)
                {
                    group.push((radix, Some(our_child.clone())));
                    continue;
                }

                let merged = H::join(
                    backend,
                    prefix.push(radix),
                    our_child,
                    their_child,
                    a_version,
                    b_version,
                    &mut *changed,
                )
                .await?;
                group.push((radix, merged));
            }

            backend.clone().parent(prefix, group).await
        })
    }
}

impl Join for Z {
    fn join<'a, B, T>(
        backend: &'a B,
        prefix: Prefix<Z>,
        a: Option<B::Node<Z>>,
        b: Option<B::Node<Z>>,
        a_version: &'a Version,
        b_version: &'a Version,
        changed: &'a mut bool,
    ) -> BoxFuture<'a, Result<Option<B::Node<Z>>, B::Error>>
    where
        B: Store<T, Node<Z>: Leaf<T>> + Sync,
        T: Send + Sync + 'static,
    {
        Box::pin(async move {
            match (a, b) {
                (None, None) => Ok(None),
                // The leaf-level base of the asymmetric arms' change
                // detection: our leaf dropped by deletion honoring is a
                // change, and their leaf surviving the filter is a gain.
                (Some(ours), None) => {
                    let kept =
                        unknown::unknown(backend, b_version, prefix, ours, &mut |_| {}).await?;
                    *changed |= kept.is_none();
                    Ok(kept)
                }
                (None, Some(theirs)) => {
                    let gained =
                        unknown::unknown(backend, a_version, prefix, theirs, &mut |_| {}).await?;
                    *changed |= gained.is_some();
                    Ok(gained)
                }
                // Two leaves at the same path are the same leaf: the path is
                // the content-addressed hash of (version, value), so
                // identical paths carry identical contents. Keep one.
                (Some(ours), Some(_)) => Ok(Some(ours)),
            }
        })
    }
}
