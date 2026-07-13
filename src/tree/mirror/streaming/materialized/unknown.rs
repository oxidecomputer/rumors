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
//! [`parent`](Backend::parent) fan of a single recursing node, so it stays
//! constant-memory and reusable across the in-memory and persistent backends
//! alike.
//!
//! Every height returns a [`BoxOptionNodeStream`]. The descent is only 32
//! deep, but an `impl Stream` return would nest each level's `async_stream`
//! type inside the next; erasing to a trait object at each step keeps that
//! type flat (and its `Send`-ness asserted rather than proven through the
//! whole tower). An `impl Stream` return here makes the compiler's type
//! balloon past any memory bound.

use std::cmp::Ordering;
use std::pin::Pin;

use async_stream::try_stream;
use futures::Stream;
use tokio_stream::StreamExt;

use crate::Version;
use crate::tree::mirror::streaming::backend::{BoxNodeStream, BoxOptionNodeStream};
use crate::tree::typed::Prefix;
use crate::tree::typed::height::{Height, S, Z};

use super::super::backend::{Backend, Leaf, Node, NodeStream};

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

/// Prune `stream` down to the nodes a counterparty at `known` is missing.
///
/// Order is preserved: the output carries a prefix-ordered subsequence of the
/// input, with recursing subtrees replaced by their pruned selves. A subtree
/// that prunes away entirely (every leaf already known) vanishes.
pub fn unknown<'a, B, T, H>(
    backend: &'a B,
    known: &'a Version,
    stream: impl NodeStream<B, T, H> + 'a,
) -> impl NodeStream<B, T, H> + 'a
where
    B: Backend<T, Node<Z>: Leaf<T>> + Sync + 'a,
    T: Send + Sync + 'static,
    H: Unknown,
{
    H::unknown(backend, known, Box::pin(stream)).filter_map(|result| match result {
        Ok((prefix, node)) => node.map(|node| Ok((prefix, node))),
        Err(e) => Some(Err(e)),
    })
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
        stream: BoxNodeStream<'a, B, T, Self>,
    ) -> BoxOptionNodeStream<'a, B, T, Self>
    where
        B: Backend<T, Node<Z>: Leaf<T>> + Sync + 'a,
        T: Send + Sync + 'a;
}

impl Unknown for Z {
    fn unknown<'a, B, T>(
        _backend: &'a B,
        known: &'a Version,
        stream: BoxNodeStream<'a, B, T, Z>,
    ) -> BoxOptionNodeStream<'a, B, T, Z>
    where
        B: Backend<T, Node<Z>: Leaf<T>> + Sync + 'a,
        T: Send + Sync + 'a,
    {
        // A leaf is known iff its ceiling is causally at or before `known`;
        // a concurrent ceiling compares as `None`, so those survive.
        Box::pin(try_stream! {
            for await item in stream {
                let (prefix, node) = item?;
                let keep = !self::known(&node, known);

                // Unconditionally report the node, filtering out known ones to None.
                yield (prefix, Some(node).filter(|_| keep));
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
        stream: BoxNodeStream<'a, B, T, S<H>>,
    ) -> BoxOptionNodeStream<'a, B, T, S<H>>
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
                        yield (prefix, Some(node));
                        continue;
                    }
                    _ => {}
                }

                // A ceiling causally at or before `known` means the whole
                // subtree is already known (or was deleted): drop it.
                if node.ceiling() <= known {
                    yield (prefix, None);
                    continue;
                }

                // Mixed: descend. Explode just this node one level, prune its
                // children, and reassemble the survivors from the pruned radix
                // group — `None` entries are the children that pruned away. A
                // group that prunes away entirely reassembles to `None`,
                // reporting the whole node known one level up.
                let children = Box::pin(backend.clone().children::<H>(prefix, node));
                let mut group = Vec::new();
                for await verdict in H::unknown(backend, known, children) {
                    let (child_prefix, child) = verdict?;
                    let (_, radix) = child_prefix.pop();
                    group.push((radix, child));
                }
                yield (prefix, backend.clone().parent::<H>(prefix, group).await?);
            }
        })
    }
}

#[cfg(test)]
mod tests;
