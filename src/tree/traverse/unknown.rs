use std::cmp::Ordering;

use crate::Version;

use super::typed::*;
use height::{Height, S, Z};

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

        // If the node is causally prior or at the known version vector, it's
        // already known (and so are all its children, since they are always in
        // the causal past or present of their parent), so don't return anything
        if node.ceiling() <= known {
            return None;
        }

        // Keep-whole fast path: if this subtree's meet (the floor, the minimal
        // version among its leaves) is *not* dominated by `known`, then no leaf
        // can be either — any leaf `v` with `v <= known` would force
        // `floor <= v <= known` — so every leaf is unknown and none would be
        // filtered out. The floor is undominated exactly when the comparison is
        // `Greater` or incomparable (`None`); a concurrent floor counts, since a
        // counterparty at `known` is still missing it.
        //
        // The destroy-and-rebuild below would reproduce this subtree
        // identically (with cold memos), so skip it and return it verbatim (an
        // `Arc` move), leaving its memoized hash/ceiling/floor untouched.
        let floor_unknown = matches!(
            node.floor().partial_cmp(known),
            None | Some(Ordering::Greater)
        );
        if floor_unknown {
            return Some(node);
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
