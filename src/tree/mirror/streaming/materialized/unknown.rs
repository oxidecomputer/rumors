//! The deletion-honoring filter: prune one subtree to what a counterparty
//! is missing.
//!
//! This is the streaming counterpart of
//! [`traverse::unknown`](crate::tree::traverse::unknown): it prunes a single
//! node down to what a counterparty at a given [`Version`] is *missing*,
//! honoring deletions. A subtree causally at or before `known` is already known
//! there (or was deleted there) and drops out, so a deletion propagates by the
//! receiver simply never re-learning the leaf.
//!
//! Unlike the materialized filter, which walks one owned subtree, this version
//! is generic over any [`Backend`]. It never materializes more than the
//! [`children`](Backend::children) / [`parent`](Backend::parent) fan of a
//! single recursing node, so it stays constant-memory and reusable across the
//! in-memory and persistent backends alike.
//!
//! Every level returns a [`BoxFuture`]. The descent is only 32 deep, but an
//! `impl Future` return would nest each level's `async` type inside the next;
//! erasing to a trait object at each step keeps that type flat (and its
//! `Send`-ness asserted rather than proven through the whole tower). An `impl
//! Future` return here makes the compiler's type balloon exponentially.

use std::cmp::Ordering;

use futures::future::{self, BoxFuture, FutureExt};

use crate::{
    Version,
    tree::{
        mirror::streaming::{Backend, Leaf, Node, materialized::children_of},
        typed::{
            Prefix,
            height::{Height, S, Z},
        },
    },
};

/// True iff a node's whole subtree is causally at or before `version`: a
/// counterparty at that version either has everything under it or deleted
/// it — either way, nothing under it needs to travel.
///
/// A concurrent ceiling compares as `None` and is *not* known: it carries
/// history the counterparty has never seen.
pub(super) fn known<T: Send + Sync + 'static>(node: &impl Node<T>, version: &Version) -> bool {
    matches!(
        node.ceiling().partial_cmp(version),
        Some(Ordering::Less | Ordering::Equal)
    )
}

enum Knowledge {
    Unknown,
    Known,
    Mixed,
}

/// Classify a subtree from its memoized version bounds without descending.
fn knowledge<T: Send + Sync + 'static>(node: &impl Node<T>, known: &Version) -> Knowledge {
    // Fast path, checked first because the meet is cheaper and can
    // early-terminate in `partial_cmp`: a floor concurrent with or greater than
    // `known` means the whole subtree is unknown.
    match node.floor().partial_cmp(known) {
        None | Some(Ordering::Greater) => Knowledge::Unknown,
        // Slower path, checked second: version comparison here can't
        // early-terminate.
        _ if node.ceiling() <= known => Knowledge::Known,
        // If neither is true, we have to descend.
        _ => Knowledge::Mixed,
    }
}

/// Prune one subtree to what a counterparty at `known` is missing, honoring
/// deletions; `None` when nothing under it is missing.
pub(super) fn unknown<'a, B, T, H>(
    backend: &'a B,
    known: &'a Version,
    prefix: Prefix<H>,
    node: B::Node<H>,
) -> BoxFuture<'a, Result<Option<B::Node<H>>, B::Error>>
where
    B: Backend<T, Node<Z>: Leaf<T>> + Sync,
    T: Send + Sync + 'static,
    H: Unknown,
{
    H::unknown(backend, known, prefix, node)
}

/// The top of the recursion, exposed: prune one subtree and report both the
/// surviving parent and its surviving children.
///
/// Reporting the children lets an answerer emit them as `Supply` reactions
/// without re-querying the prefix it just explored (the one-query-per-prefix
/// invariant; see [`super`]).
pub(super) async fn unknown_providing<B, T, H>(
    backend: &B,
    known: &Version,
    prefix: Prefix<S<H>>,
    node: B::Node<S<H>>,
) -> Result<(Option<B::Node<S<H>>>, Vec<(u8, B::Node<H>)>), B::Error>
where
    B: Backend<T, Node<Z>: Leaf<T>> + Sync,
    T: Send + Sync + 'static,
    H: Unknown,
    S<H>: Height,
{
    match knowledge(&node, known) {
        Knowledge::Unknown => {
            let children = children_of(backend, prefix, node.clone()).await?;
            return Ok((Some(node), children));
        }
        Knowledge::Known => return Ok((None, Vec::new())),
        Knowledge::Mixed => {}
    }

    // Mixed: prune the children one by one; the surviving group is both the
    // provision list and the material `parent` rebuilds the survivor from.
    let children = children_of(backend, prefix, node).await?;
    let mut group = Vec::with_capacity(children.len());
    let mut survivors = Vec::new();
    for (radix, child) in children {
        let survivor = H::unknown(backend, known, prefix.push(radix), child).await?;
        if let Some(survivor) = &survivor {
            survivors.push((radix, survivor.clone()));
        }
        group.push((radix, survivor));
    }
    Ok((backend.clone().parent(prefix, group).await?, survivors))
}

/// The inductive step of the streaming filter, implemented per [`Height`].
///
/// Each level classifies a node by its memoized version bounds before
/// descending, reproducing the verdicts of
/// [`traverse::unknown::Unknown`](crate::tree::traverse::unknown::Unknown)
/// node for node.
pub trait Unknown: Height {
    /// Prune one node at this height. See [`unknown`].
    fn unknown<'a, B, T>(
        backend: &'a B,
        known: &'a Version,
        prefix: Prefix<Self>,
        node: B::Node<Self>,
    ) -> BoxFuture<'a, Result<Option<B::Node<Self>>, B::Error>>
    where
        B: Backend<T, Node<Z>: Leaf<T>> + Sync,
        T: Send + Sync + 'static;
}

impl Unknown for Z {
    fn unknown<'a, B, T>(
        _backend: &'a B,
        known: &'a Version,
        _prefix: Prefix<Z>,
        node: B::Node<Z>,
    ) -> BoxFuture<'a, Result<Option<B::Node<Z>>, B::Error>>
    where
        B: Backend<T, Node<Z>: Leaf<T>> + Sync,
        T: Send + Sync + 'static,
    {
        // A leaf is known iff its ceiling is causally at or before `known`;
        // a concurrent ceiling compares as `None`, so those survive.
        let verdict = Some(node).filter(|node| !self::known(node, known));
        future::ready(Ok(verdict)).boxed()
    }
}

impl<H> Unknown for S<H>
where
    H: Unknown,
    S<H>: Height,
{
    fn unknown<'a, B, T>(
        backend: &'a B,
        known: &'a Version,
        prefix: Prefix<S<H>>,
        node: B::Node<S<H>>,
    ) -> BoxFuture<'a, Result<Option<B::Node<S<H>>>, B::Error>>
    where
        B: Backend<T, Node<Z>: Leaf<T>> + Sync,
        T: Send + Sync + 'static,
    {
        Box::pin(async move {
            match knowledge(&node, known) {
                Knowledge::Unknown => return Ok(Some(node)),
                Knowledge::Known => return Ok(None),
                Knowledge::Mixed => {}
            }

            // Mixed: descend. Explode just this node one level, prune its
            // children, and reassemble the survivors from the pruned radix
            // group — `None` entries are the children that pruned away. A group
            // that prunes away entirely reassembles to `None`, reporting the
            // whole node known one level up.
            let children = children_of(backend, prefix, node).await?;
            let mut group = Vec::with_capacity(children.len());
            for (radix, child) in children {
                let survivor = H::unknown(backend, known, prefix.push(radix), child).await?;
                group.push((radix, survivor));
            }
            backend.clone().parent(prefix, group).await
        })
    }
}

#[cfg(test)]
mod tests;
