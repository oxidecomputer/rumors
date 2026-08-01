//! The deletion-honoring filter, session-flavored: the generic
//! [`traverse::store::unknown`](crate::tree::traverse::store::unknown)
//! tower with every dropped leaf credited to the session's stats.
//!
//! The filter itself — verdicts, fast paths, constant-memory descent —
//! lives in the generic tower, shared with the local merge; this module
//! binds its shed observer to the session [`Recorder`], so each leaf the
//! filter drops is counted as
//! [`messages_shed`](crate::SessionStats::messages_shed): one per dropped
//! leaf, a whole subtree's exact leaf count when the resident version
//! bounds prune it without descending.

use futures::future::BoxFuture;

pub use crate::tree::traverse::store::unknown::{Unknown, known};

use crate::{
    Version, causally,
    tree::{
        backend::children_of,
        mirror::streaming::{Backend, Leaf, stats::Recorder},
        traverse::store::unknown,
        typed::{
            Prefix,
            height::{Height, S, Z},
        },
    },
};

/// Prune one subtree to what a counterparty at `known` is missing, honoring
/// deletions; `None` when nothing under it is missing. Drops are credited
/// to `stats`.
pub(super) fn unknown<'a, B, T, H>(
    backend: &'a B,
    known: &'a Version,
    prefix: Prefix<H>,
    node: B::Node<H>,
    stats: &'a Recorder,
) -> BoxFuture<'a, Result<Option<B::Node<H>>, B::Error>>
where
    B: Backend<T, Node<Z>: Leaf<T>> + Sync,
    T: Send + Sync + 'static,
    H: Unknown,
{
    Box::pin(async move {
        let mut shed = |dropped| stats.shed(dropped);
        unknown::unknown(backend, known, prefix, node, &mut shed).await
    })
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
    stats: &Recorder,
) -> Result<(Option<B::Node<S<H>>>, Vec<(u8, B::Node<H>)>), B::Error>
where
    B: Backend<T, Node<Z>: Leaf<T>> + Sync,
    T: Send + Sync + 'static,
    H: Unknown,
    S<H>: Height,
{
    use crate::tree::backend::Node as _;

    match unknown::knowledge(&node, known) {
        // Wholly unknown: the whole subtree travels.
        causally::Dominance::Before => {
            let children = children_of(backend, prefix, node.clone()).await?;
            return Ok((Some(node), children));
        }
        // Wholly known: nothing under the node needs to travel.
        causally::Dominance::After => {
            stats.shed(node.len() as u64);
            return Ok((None, Vec::new()));
        }
        causally::Dominance::Between => {}
    }

    // Mixed: prune the children one by one; the surviving group is both the
    // provision list and the material `parent` rebuilds the survivor from.
    let children = children_of(backend, prefix, node).await?;
    let mut group = Vec::with_capacity(children.len());
    let mut survivors = Vec::new();
    let mut shed = |dropped| stats.shed(dropped);
    for (radix, child) in children {
        let survivor = H::unknown(backend, known, prefix.push(radix), child, &mut shed).await?;
        if let Some(survivor) = &survivor {
            survivors.push((radix, survivor.clone()));
        }
        group.push((radix, survivor));
    }
    Ok((backend.clone().parent(prefix, group).await?, survivors))
}

#[cfg(test)]
mod tests;
