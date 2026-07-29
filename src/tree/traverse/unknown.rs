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

use std::cmp::Ordering;

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

        // Both fast paths place a node bound against the counterparty's
        // known-at range (everything causally contained in `known`):
        //
        // 1. floor beyond the range (concurrent with or > known)
        // 2. ceiling within the range (<= known)
        //
        // We check them in this order because it's expected that the first
        // comparison is *cheaper* (the meet of random versions is likely to be
        // small because it's the greatest-common-ancestor) and because it's
        // more likely to happen *higher* in the tree, *and* because it's the
        // only one of the two comparisons which can early-terminate during the
        // placement (a floor concurrent to `known` is decided at the first
        // opposing interval). This gives a measurable, if small win in
        // benchmarks, by skipping the second comparison more of the time.
        let known_at = causally::known_at(known);

        // If the node's floor is beyond the known range, the whole subtree is
        // definitely unknown (children are always in the causal future or
        // present of their parent's floor), so return the node unchanged:
        if known_at.placement_of(node.floor()) == Ordering::Greater {
            return Some(node);
        }

        // If the node's ceiling is within the known range, the whole subtree
        // is already known (children are always in the causal past or present
        // of their parent's ceiling), so don't return anything at all:
        if known_at.contains(node.ceiling()) {
            return None;
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
