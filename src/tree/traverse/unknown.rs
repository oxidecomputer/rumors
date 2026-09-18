//! The deletion-honoring filter: prune a subtree down to what a
//! counterparty at a given version is missing.
//!
//! Redaction leaves no tombstone in the tree. A subtree whose version ceiling
//! is contained in the counterparty's version holds nothing it hasn't already
//! seen — including anything it has seen *and deleted* — so the subtree drops
//! out of the answer, and a deletion propagates by the receiver simply never
//! re-learning the leaf. The in-memory [`join`](mod@super::join) uses this
//! filter for one-sided subtrees. The streaming mirror implements the same rule
//! over its backend interface; differential tests check that the two
//! reconciliations agree.

use before::Dominance;

use crate::{Version, causally};

use super::typed::*;
use height::{Height, S, Z};

/// The inductive step of the filter, implemented per [`Height`]: each level
/// prunes by its memoized ceiling/floor before descending.
pub trait Unknown: Height {
    /// Filters this subtree down to the nodes a counterparty at `known` is
    /// missing, honoring deletions: a node causally `<=` `known` is already
    /// known there (or was deleted there) and drops out.
    fn unknown(node: Option<Node<Self>>, known: &Version) -> Option<Node<Self>>;
}

impl<H: Unknown> Unknown for S<H>
where
    S<H>: Height,
{
    fn unknown(node: Option<Node<Self>>, known: &Version) -> Option<Node<Self>> {
        let node = node?;

        // Classify the whole subtree from its memoized bounds. This decodes
        // `known` once and avoids descending when every leaf has the same
        // answer.
        match node.span().dominance(known) {
            // `known` does not dominate even the floor (the floor is
            // beyond or beside it): the whole subtree is definitely
            // unknown (children are always in the causal future or
            // present of their parent's floor), so return the node
            // unchanged.
            Dominance::Before => return Some(node),
            // `known` dominates the ceiling: the whole subtree is
            // already known (children are always in the causal past or
            // present of their parent's ceiling), so don't return
            // anything at all.
            Dominance::After => return None,
            // Only the floor is dominated: the subtree is mixed.
            Dominance::Between => {}
        }

        Node::branch(
            node.into_children()
                .into_iter()
                .filter_map(|(radix, child)| {
                    Unknown::unknown(Some(child), known).map(|child| (radix, child))
                })
                .collect(),
        )
    }
}

impl Unknown for Z {
    fn unknown(node: Option<Node<Self>>, known: &Version) -> Option<Node<Self>> {
        let node = node?;

        // A contained version was either retained or redacted there. In either
        // case, the counterparty does not need this leaf again.
        if causally::before(known).contains(node.ceiling()) {
            return None;
        }

        Some(node)
    }
}

#[cfg(test)]
mod tests;
