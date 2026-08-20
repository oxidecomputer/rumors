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
//! The walk runs on erased nodes, its level named by its prefix's byte
//! length: one instantiation per backend, where a height-typed recursion
//! would mint one per level. Each recursive call boxes its future
//! ([`BoxFuture`]) exactly as the typed tower did — the type stays flat —
//! and the depth is bounded by the prefix's remaining height, at most 32,
//! so the recursion is stack-safe by construction rather than by input
//! goodwill.

use futures::future::{BoxFuture, FutureExt};

use before::Dominance;

use crate::{
    Version, causally,
    tree::{
        mirror::streaming::{Backend, ErasedNode, Leaf, erased::ops, stats::Recorder},
        typed::{ErasedPrefix, height::Z},
    },
};

/// True iff a node's whole subtree is causally at or before `version`: a
/// counterparty at that version either has everything under it or deleted
/// it — either way, nothing under it needs to travel.
///
/// A concurrent ceiling is beyond the known-at range and is *not* known:
/// it carries history the counterparty has never seen.
pub(super) fn known(node: &impl ErasedNode, version: &Version) -> bool {
    causally::before(version).contains(node.span().hi())
}

/// Classify a subtree from its memoized version bounds without
/// descending: how much of the node's `[floor, ceiling]` span the
/// counterparty's version dominates *is* the knowledge verdict.
///
/// [`Before`](Dominance::Before) — the floor is beyond or
/// beside `known` — means the whole subtree is unknown;
/// [`After`](Dominance::After) — the ceiling is within
/// `known`'s past — means it is all already known; and
/// [`Between`](Dominance::Between) means mixed, so the
/// caller descends.
///
/// The backend hands out its own stored bounds ([`ErasedNode::span`]) and
/// the span answers in one fused walk that decodes `known` once, where
/// placing the two bounds separately would decode it once per bound;
/// the span's ordering is the backend's construction-time obligation,
/// so no validating comparison is paid per classification. The cheap
/// unknown-subtree exit survives the fusion: `floor <= known` refuted
/// is the whole verdict, decided at the first refuting interval.
fn knowledge(node: &impl ErasedNode, known: &Version) -> Dominance {
    node.span().dominance(known)
}

/// Prune one subtree to what a counterparty at `known` is missing, honoring
/// deletions; `None` when nothing under it is missing.
///
/// Every leaf the filter drops is a deletion honored locally, so each
/// verdict site credits the drop to `stats` as
/// [`messages_shed`](crate::SessionStats::messages_shed): one per dropped
/// leaf, a whole subtree's exact leaf count when the cached version bounds
/// prune it without descending.
pub(super) fn unknown<'a, B>(
    backend: &'a B,
    known: &'a Version,
    prefix: ErasedPrefix,
    node: B::Erased,
    stats: &'a Recorder,
) -> BoxFuture<'a, Result<Option<B::Erased>, B::Error>>
where
    B: Backend<Node<Z>: Leaf> + Sync,
{
    async move {
        if prefix.height() == 0 {
            // A leaf is known iff its ceiling is causally at or before
            // `known`; a concurrent ceiling is beyond the known-at range,
            // so those survive.
            let verdict = Some(node).filter(|node| !self::known(node, known));
            if verdict.is_none() {
                stats.shed(1);
            }
            return Ok(verdict);
        }

        match knowledge(&node, known) {
            // Wholly unknown: the whole subtree travels.
            Dominance::Before => return Ok(Some(node)),
            // Wholly known: nothing under the node needs to travel.
            Dominance::After => {
                stats.shed(node.len() as u64);
                return Ok(None);
            }
            Dominance::Between => {}
        }

        // Mixed: descend. Explode just this node one level, prune its
        // children, and reassemble the survivors from the pruned radix
        // group — `None` entries are the children that pruned away. A group
        // that prunes away entirely reassembles to `None`, reporting the
        // whole node known one level up.
        let children = ops::children_of(backend, prefix, node).await?;
        let mut group = Vec::with_capacity(children.len());
        for (radix, child) in children {
            let survivor = unknown(backend, known, prefix.push(radix), child, stats).await?;
            group.push((radix, survivor));
        }
        ops::parent(backend.clone(), prefix, group).await
    }
    .boxed()
}

/// The top of the recursion, exposed: prune one subtree and report both the
/// surviving parent and its surviving children.
///
/// Reporting the children lets an answerer emit them as `Supply` reactions
/// without re-querying the prefix it just explored (the one-query-per-prefix
/// invariant; see [`super`]).
pub(super) async fn unknown_providing<B>(
    backend: &B,
    known: &Version,
    prefix: ErasedPrefix,
    node: B::Erased,
    stats: &Recorder,
) -> Result<(Option<B::Erased>, Vec<(u8, B::Erased)>), B::Error>
where
    B: Backend<Node<Z>: Leaf> + Sync,
{
    match knowledge(&node, known) {
        // Wholly unknown: the whole subtree travels.
        Dominance::Before => {
            let children = ops::children_of(backend, prefix, node.clone()).await?;
            return Ok((Some(node), children));
        }
        // Wholly known: nothing under the node needs to travel.
        Dominance::After => {
            stats.shed(node.len() as u64);
            return Ok((None, Vec::new()));
        }
        Dominance::Between => {}
    }

    // Mixed: prune the children one by one; the surviving group is both the
    // provision list and the material `parent` rebuilds the survivor from.
    let children = ops::children_of(backend, prefix, node).await?;
    let mut group = Vec::with_capacity(children.len());
    let mut survivors = Vec::new();
    for (radix, child) in children {
        let survivor = unknown(backend, known, prefix.push(radix), child, stats).await?;
        if let Some(survivor) = &survivor {
            survivors.push((radix, survivor.clone()));
        }
        group.push((radix, survivor));
    }
    Ok((
        ops::parent(backend.clone(), prefix, group).await?,
        survivors,
    ))
}

#[cfg(test)]
mod tests;
