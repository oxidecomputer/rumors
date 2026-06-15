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

use crate::Version;

use super::typed::*;
use height::{Height, S, Z};

/// The inductive step of the filter, implemented per [`Height`]: each level
/// prunes by its memoized ceiling/floor before descending.
pub trait Unknown: Height {
    /// Filter this subtree down to the nodes a counterparty at `known` is
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

        // There are two fast paths here:
        //
        // 1. floor concurrent with or > known
        // 2. ceiling <= known
        //
        // We check them in this order because it's expected that the first
        // comparison is *cheaper* (the meet of random versions is likely to be
        // small because it's the greatest-common-ancestor) and because it's
        // more likely to happen *higher* in the tree, *and* because it's the
        // only one of the two comparisons which can early-terminate during the
        // `partial_cmp` (because the `None` verdict can bail early in version
        // comparison). This gives a measurable, if small win in benchmarks, by
        // skipping the second comparison more of the time.

        // If the node's floor is concurrent with or greater than the known
        // version vector, it's definitely unknown (and so are all its children,
        // since they are always in the causal future or present of their
        // parent's floor), so return the node unchanged:
        match node.floor().partial_cmp(known) {
            None | Some(Ordering::Greater) => return Some(node),
            _ => {}
        }

        // If the node's ceiling is causally prior to or at the known version
        // vector, it's already known (and so are all its children, since they
        // are always in the causal past or present of their parent's ceiling),
        // so don't return anything at all:
        if node.ceiling() <= known {
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

        // If the node is causally prior or at the known version vector, it's
        // already known, so don't return anything
        if node.ceiling() <= known {
            return None;
        }

        // Otherwise, the node is causally unknown: return it
        Some(node)
    }
}
