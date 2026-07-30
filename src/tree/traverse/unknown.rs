//! The deletion-honoring filter: prune a subtree down to what a
//! counterparty at a given version is missing.
//!
//! This traversal is where "redaction leaves no tombstone" is cashed out.
//! A subtree whose version ceiling is contained in the counterparty's
//! version holds nothing it hasn't already seen — including anything it
//! has seen *and deleted* — so the subtree drops out of the answer, and a
//! deletion propagates by the receiver simply never re-learning the leaf.
//! Both the in-memory [`join`](mod@super::join) and the wire
//! [`mirror`](super::mirror) delegate their version filtering here, which
//! is what makes them observationally identical.

use crate::{Version, causally};

use super::typed::*;
use height::{Height, S, Z};

/// The inductive step of the filter, implemented per [`Height`]: each level
/// prunes by its memoized ceiling/floor before descending.
pub trait Unknown: Height {
    /// Filters this subtree down to the nodes a counterparty at `known` is
    /// missing, honoring deletions: a node causally `<=` `known` is already
    /// known there (or was deleted there) and drops out.
    fn unknown<T>(node: Option<Node<T, Self>>, known: &Version) -> Option<Node<T, Self>>
    where
        T: Send + Sync;
}

impl<H: Unknown> Unknown for S<H>
where
    S<H>: Height,
{
    fn unknown<T>(node: Option<Node<T, Self>>, known: &Version) -> Option<Node<T, Self>>
    where
        T: Send + Sync,
    {
        // If the node doesn't exist, we can't return information about it
        let node = node?;

        // One fused walk classifies the node: the counterparty's version
        // is placed against the subtree's memoized `[floor, ceiling]`
        // interval — ordered structurally, since both memos are the meet
        // and join of the same leaf versions, which is what lets the
        // trusted constructor skip a validating comparison per node —
        // and each verdict of the dominance face is one prune decision.
        // The fused walk decodes `known` once where placing the two
        // bounds separately would decode it once per bound, and it keeps
        // the cheap unknown-subtree exit: `floor <= known` refuted is
        // the whole verdict, decided at the first refuting interval
        // (the floor, a meet, is likely the smallest stream, and whole
        // divergent subtrees are the common case high in the tree).
        let interval = causally::Interval::ordered(node.floor(), node.ceiling());
        match interval.dominance_of(known) {
            // `known` does not dominate even the floor (the floor is
            // beyond or beside it): the whole subtree is definitely
            // unknown (children are always in the causal future or
            // present of their parent's floor), so return the node
            // unchanged.
            causally::Dominance::Neither => return Some(node),
            // `known` dominates the ceiling: the whole subtree is
            // already known (children are always in the causal past or
            // present of their parent's ceiling), so don't return
            // anything at all.
            causally::Dominance::Whole => return None,
            // Only the floor is dominated: the subtree is mixed.
            causally::Dominance::StartOnly => {}
        }

        // Recursively process each child, re-assembling only the unknown children
        Node::branch({
            let mut children = Children::default();
            for (radix, child) in node.into_children() {
                if let Some(child) = Unknown::unknown(Some(child), known) {
                    children.insert(radix, child);
                }
            }
            children
        })
    }
}

impl Unknown for Z {
    fn unknown<T>(node: Option<Node<T, Self>>, known: &Version) -> Option<Node<T, Self>>
    where
        T: Send + Sync,
    {
        // If the node doesn't exist, we can't return information about it
        let node = node?;

        // If the leaf's version is within the counterparty's known-at range,
        // it's already known, so don't return anything
        if causally::known_at(known).contains(node.ceiling()) {
            return None;
        }

        // Otherwise, the node is causally unknown: return it
        Some(node)
    }
}

#[cfg(test)]
mod tests;
