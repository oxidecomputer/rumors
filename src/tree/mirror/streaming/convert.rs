//! Re-represent nodes from one backend in the node types of another.
//!
//! A node converts by exploding to leaves in the source backend and
//! reassembling in the target.
//!
//! The protocol itself converts nowhere: both parties of a session name one
//! backend, and a homogeneous session pays nothing. This module is what lets a
//! heterogeneous pair meet, by re-representing each node-carrying message.

use std::pin::pin;

use async_stream::try_stream;
use futures::StreamExt;

use crate::tree::{
    mirror::streaming::{
        Backend, Leaf,
        backend::{BoxNodeStream, NodeStream},
    },
    typed::{
        Prefix,
        height::{Height, S, Z},
    },
};

/// A height whose subtrees convert across backends: they explode to the leaf
/// stream beneath them and reassemble from one.
pub trait Convert: Height {
    /// Disassemble a stream of `backend`'s nodes at this height into the
    /// prefix-ordered stream of every leaf beneath them.
    fn explode<B, T>(backend: B, stream: BoxNodeStream<B, T, Self>) -> BoxNodeStream<B, T, Z>
    where
        B: Backend<T, Node<Z>: Leaf<T>>,
        T: Send + Sync + 'static;

    /// Assemble a prefix-ordered leaf stream into the stream of `backend`'s
    /// nodes at this height.
    fn assemble<B, T>(backend: B, leaves: BoxNodeStream<B, T, Z>) -> BoxNodeStream<B, T, Self>
    where
        B: Backend<T, Node<Z>: Leaf<T>>,
        T: Send + Sync + 'static;
}

impl Convert for Z {
    fn explode<B, T>(_backend: B, stream: BoxNodeStream<B, T, Z>) -> BoxNodeStream<B, T, Z>
    where
        B: Backend<T, Node<Z>: Leaf<T>>,
        T: Send + Sync + 'static,
    {
        stream
    }

    fn assemble<B, T>(_backend: B, leaves: BoxNodeStream<B, T, Z>) -> BoxNodeStream<B, T, Z>
    where
        B: Backend<T, Node<Z>: Leaf<T>>,
        T: Send + Sync + 'static,
    {
        leaves
    }
}

impl<H> Convert for S<H>
where
    H: Convert,
    S<H>: Height,
{
    fn explode<B, T>(backend: B, stream: BoxNodeStream<B, T, S<H>>) -> BoxNodeStream<B, T, Z>
    where
        B: Backend<T, Node<Z>: Leaf<T>>,
        T: Send + Sync + 'static,
    {
        let exploded = backend.clone();
        let below: BoxNodeStream<B, T, H> = Box::pin(try_stream! {
            for await item in stream {
                let (prefix, node) = item?;
                for await child in exploded.clone().children::<H>(prefix, node) {
                    yield child?;
                }
            }
        });
        H::explode(backend, below)
    }

    fn assemble<B, T>(backend: B, leaves: BoxNodeStream<B, T, Z>) -> BoxNodeStream<B, T, S<H>>
    where
        B: Backend<T, Node<Z>: Leaf<T>>,
        T: Send + Sync + 'static,
    {
        let below = H::assemble(backend.clone(), leaves);
        let folded = fold_parents(backend, below);
        Box::pin(try_stream! {
            let mut folded = pin!(folded);
            while let Some(item) = folded.next().await {
                let (prefix, node) = item?;
                yield (prefix, node);
            }
        })
    }
}

/// Reassemble an ascending child stream into its parent level, one complete
/// radix group at a time: a group flushes when the prefix changes or the
/// input ends.
fn fold_parents<B, T, H>(
    backend: B,
    children: impl NodeStream<B, T, H>,
) -> impl NodeStream<B, T, S<H>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
    S<H>: Height,
{
    /// Flush a completed group, if any, into its parent.
    async fn flush<B, T, H>(
        backend: &B,
        finished: Option<(Prefix<S<H>>, Vec<(u8, Option<B::Node<H>>)>)>,
    ) -> Result<Option<(Prefix<S<H>>, B::Node<S<H>>)>, B::Error>
    where
        B: Backend<T, Node<Z>: Leaf<T>>,
        T: Send + Sync + 'static,
        H: Height,
        S<H>: Height,
    {
        let Some((prefix, group)) = finished else {
            return Ok(None);
        };
        let parent = backend.clone().parent(prefix, group).await?;
        Ok(parent.map(|parent| (prefix, parent)))
    }

    try_stream! {
        let mut children = pin!(children);
        let mut open: Option<(_, Vec<_>)> = None;
        while let Some(item) = children.next().await {
            let (path, child) = item?;
            let (prefix, radix) = path.pop();
            match &mut open {
                Some((current, group)) if *current == prefix => {
                    group.push((radix, Some(child)));
                }
                _ => {
                    let finished = open.replace((prefix, vec![(radix, Some(child))]));
                    if let Some((flushed, parent)) = flush(&backend, finished).await? {
                        yield (flushed, parent);
                    }
                }
            }
        }
        if let Some((flushed, parent)) = flush(&backend, open.take()).await? {
            yield (flushed, parent);
        }
    }
}

#[cfg(test)]
mod tests;
