//! The deletion-honoring filter, as a prefix-ordered stream transducer.
//!
//! This is the streaming counterpart of
//! [`traverse::unknown`](crate::tree::traverse::unknown): it prunes a
//! prefix-ordered node stream down to the nodes a counterparty at a given
//! [`Version`] is *missing*, honoring deletions — a subtree causally at or
//! before `known` is already known there (or was deleted there) and drops out,
//! so a deletion propagates by the receiver simply never re-learning the leaf.
//!
//! Unlike the materialized filter, which walks one owned subtree, this version
//! is generic over any [`Backend`] and consumes the tree as a stream. It never
//! materializes more than the [`children`](Backend::children) /
//! [`parents`](Backend::parents) fan of a single recursing node, so it stays
//! constant-memory and reusable across the in-memory and persistent backends
//! alike.
//!
//! Every height returns a boxed [`NodeStream`]. The descent is only 32 deep,
//! but an `impl Stream` return would nest each level's `async_stream` type
//! inside the next; erasing to a trait object at each step keeps that type flat
//! (and its `Send`-ness asserted rather than proven through the whole tower) —
//! the same reason [`Local`](super::super::Local)'s own `children`/`parents` box. An
//! `impl Stream` return here makes the compiler's type balloon past any memory
//! bound.

use std::cmp::Ordering;
use std::pin::Pin;

use async_stream::try_stream;

use crate::Version;
use crate::tree::typed::height::{Height, S, Z};

use super::super::backend::{Backend, Leaf, Node, NodeStream, one};

/// Prune `stream` down to the nodes a counterparty at `known` is missing.
///
/// Order is preserved: the output carries a prefix-ordered subsequence of the
/// input, with recursing subtrees replaced by their pruned selves. A subtree
/// that prunes away entirely (every leaf already known) vanishes.
pub fn unknown<'a, B, T, H>(
    backend: &'a B,
    known: &'a Version,
    stream: impl NodeStream<B, T, H> + 'a,
) -> Pin<Box<dyn NodeStream<B, T, H> + 'a>>
where
    B: Backend<T, Node<Z>: Leaf<T>> + Sync + 'a,
    T: Send + Sync + 'a,
    H: Unknown,
{
    H::unknown(backend, known, Box::pin(stream))
}

/// The inductive step of the streaming filter, implemented per [`Height`].
///
/// Each level classifies a node by its memoized version bounds before
/// descending, reproducing the verdicts of
/// [`traverse::unknown::Unknown`](crate::tree::traverse::unknown::Unknown) node
/// for node.
pub trait Unknown: Height {
    /// Prune `stream` at this height. See [`unknown`].
    fn unknown<'a, B, T>(
        backend: &'a B,
        known: &'a Version,
        stream: Pin<Box<dyn NodeStream<B, T, Self> + 'a>>,
    ) -> Pin<Box<dyn NodeStream<B, T, Self> + 'a>>
    where
        B: Backend<T, Node<Z>: Leaf<T>> + Sync + 'a,
        T: Send + Sync + 'a;
}

impl Unknown for Z {
    fn unknown<'a, B, T>(
        _backend: &'a B,
        known: &'a Version,
        stream: Pin<Box<dyn NodeStream<B, T, Z> + 'a>>,
    ) -> Pin<Box<dyn NodeStream<B, T, Z> + 'a>>
    where
        B: Backend<T, Node<Z>: Leaf<T>> + Sync + 'a,
        T: Send + Sync + 'a,
    {
        // A leaf is known iff its ceiling is causally at or before `known`;
        // a concurrent ceiling compares as `None`, so those survive.
        Box::pin(try_stream! {
            for await item in stream {
                let (prefix, node) = item?;
                let known = matches!(
                    node.ceiling().partial_cmp(known),
                    Some(Ordering::Less | Ordering::Equal)
                );
                if !known {
                    yield (prefix, node);
                }
            }
        })
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
        stream: Pin<Box<dyn NodeStream<B, T, S<H>> + 'a>>,
    ) -> Pin<Box<dyn NodeStream<B, T, S<H>> + 'a>>
    where
        B: Backend<T, Node<Z>: Leaf<T>> + Sync + 'a,
        T: Send + Sync + 'a,
    {
        Box::pin(try_stream! {
            for await item in stream {
                let (prefix, node) = item?;

                // Fast path, checked first because the meet is cheaper and can
                // early-terminate in `partial_cmp`: a floor concurrent with or
                // greater than `known` means the whole subtree is unknown.
                match node.floor().partial_cmp(known) {
                    None | Some(Ordering::Greater) => {
                        yield (prefix, node);
                        continue;
                    }
                    _ => {}
                }

                // A ceiling causally at or before `known` means the whole
                // subtree is already known (or was deleted): drop it.
                if node.ceiling() <= known {
                    continue;
                }

                // Mixed: descend. Explode just this node one level, prune its
                // children, and reassemble. A child list that prunes away
                // entirely collapses to nothing (`parents` emits no node).
                let children = Box::pin(backend.clone().children::<H>(one(prefix, node)));
                let pruned = H::unknown(backend, known, children);
                let parents = backend.clone().parents::<H>(pruned);
                for await parent in parents {
                    yield parent?;
                }
            }
        })
    }
}

#[cfg(test)]
mod tests;
