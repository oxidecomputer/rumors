//! The backend-generic deletion-honoring filter: prune one subtree to what
//! a counterparty at a given version is missing.
//!
//! The generic twin of [`traverse::unknown`](crate::tree::traverse::unknown),
//! reproducing its verdicts node for node from the resident version bounds
//! every backend handle carries. Both the local [`fn@super::join::join`] and
//! the streaming mirror delegate their version filtering here, which is
//! what keeps them observationally identical.
//!
//! The filter never materializes more than the
//! [`children`](Backend::children) / [`parent`](Backend::parent) fan of a
//! single recursing node, so it stays constant-memory over any backend.
//!
//! `shed` observes every leaf the filter drops — one call per dropped leaf,
//! a whole subtree's exact leaf count when the resident bounds prune it
//! without descending — so a session can credit deletion honoring to its
//! stats while a local merge passes a no-op.

use futures::future::{self, BoxFuture, FutureExt as _};

use crate::{
    Version, causally,
    tree::{
        backend::{Backend, Leaf, Node, children_of},
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
/// A concurrent ceiling is beyond the known-at range and is *not* known:
/// it carries history the counterparty has never seen.
pub fn known<T: Send + Sync + 'static>(node: &impl Node<T>, version: &Version) -> bool {
    causally::known_at(version).contains(node.span().join())
}

/// Classify a subtree from its resident version bounds without
/// descending: how much of the node's `[floor, ceiling]` span the
/// counterparty's version dominates *is* the knowledge verdict.
///
/// [`Before`](causally::Dominance::Before) — the floor is beyond or
/// beside `known` — means the whole subtree is unknown;
/// [`After`](causally::Dominance::After) — the ceiling is within
/// `known`'s past — means it is all already known; and
/// [`Between`](causally::Dominance::Between) means mixed, so the
/// caller descends.
///
/// The backend hands out its own stored bounds ([`Node::span`]) and the
/// span answers in one fused walk that decodes `known` once, where
/// placing the two bounds separately would decode it once per bound;
/// the span's ordering is the backend's construction-time obligation,
/// so no validating comparison is paid per classification. The cheap
/// unknown-subtree exit survives the fusion: `floor <= known` refuted
/// is the whole verdict, decided at the first refuting interval.
pub fn knowledge<T: Send + Sync + 'static>(
    node: &impl Node<T>,
    known: &Version,
) -> causally::Dominance {
    node.span().dominance_of(known)
}

/// Prune one subtree to what a counterparty at `known` is missing, honoring
/// deletions; `None` when nothing under it is missing.
pub fn unknown<'a, B, T, H, F>(
    backend: &'a B,
    known: &'a Version,
    prefix: Prefix<H>,
    node: B::Node<H>,
    shed: &'a mut F,
) -> BoxFuture<'a, Result<Option<B::Node<H>>, B::Error>>
where
    B: Backend<T, Node<Z>: Leaf<T>> + Sync,
    T: Send + Sync + 'static,
    H: Unknown,
    F: FnMut(u64) + Send,
{
    H::unknown(backend, known, prefix, node, shed)
}

/// The inductive step of the generic filter, implemented per [`Height`].
///
/// Each level classifies a node by its resident version bounds before
/// descending, reproducing the verdicts of
/// [`traverse::unknown::Unknown`](crate::tree::traverse::unknown::Unknown)
/// node for node.
pub trait Unknown: Height {
    /// Prune one node at this height. See [`unknown`].
    fn unknown<'a, B, T, F>(
        backend: &'a B,
        known: &'a Version,
        prefix: Prefix<Self>,
        node: B::Node<Self>,
        shed: &'a mut F,
    ) -> BoxFuture<'a, Result<Option<B::Node<Self>>, B::Error>>
    where
        B: Backend<T, Node<Z>: Leaf<T>> + Sync,
        T: Send + Sync + 'static,
        F: FnMut(u64) + Send;
}

impl Unknown for Z {
    fn unknown<'a, B, T, F>(
        _backend: &'a B,
        known: &'a Version,
        _prefix: Prefix<Z>,
        node: B::Node<Z>,
        shed: &'a mut F,
    ) -> BoxFuture<'a, Result<Option<B::Node<Z>>, B::Error>>
    where
        B: Backend<T, Node<Z>: Leaf<T>> + Sync,
        T: Send + Sync + 'static,
        F: FnMut(u64) + Send,
    {
        // A leaf is known iff its ceiling is causally at or before `known`;
        // a concurrent ceiling is beyond the known-at range, so those survive.
        let verdict = Some(node).filter(|node| !self::known(node, known));
        if verdict.is_none() {
            shed(1);
        }
        future::ready(Ok(verdict)).boxed()
    }
}

impl<H> Unknown for S<H>
where
    H: Unknown,
    S<H>: Height,
{
    fn unknown<'a, B, T, F>(
        backend: &'a B,
        known: &'a Version,
        prefix: Prefix<S<H>>,
        node: B::Node<S<H>>,
        shed: &'a mut F,
    ) -> BoxFuture<'a, Result<Option<B::Node<S<H>>>, B::Error>>
    where
        B: Backend<T, Node<Z>: Leaf<T>> + Sync,
        T: Send + Sync + 'static,
        F: FnMut(u64) + Send,
    {
        Box::pin(async move {
            match knowledge(&node, known) {
                // Wholly unknown: the whole subtree travels.
                causally::Dominance::Before => return Ok(Some(node)),
                // Wholly known: nothing under the node needs to travel.
                causally::Dominance::After => {
                    shed(node.len() as u64);
                    return Ok(None);
                }
                causally::Dominance::Between => {}
            }

            // Mixed: descend. Explode just this node one level, prune its
            // children, and reassemble the survivors from the pruned radix
            // group — `None` entries are the children that pruned away. A group
            // that prunes away entirely reassembles to `None`, reporting the
            // whole node known one level up.
            let children = children_of(backend, prefix, node).await?;
            let mut group = Vec::with_capacity(children.len());
            for (radix, child) in children {
                let survivor =
                    H::unknown(backend, known, prefix.push(radix), child, &mut *shed).await?;
                group.push((radix, survivor));
            }
            backend.clone().parent(prefix, group).await
        })
    }
}
