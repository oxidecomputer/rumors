//! Re-represent nodes from one backend in the node types of another.
//!
//! A node converts by exploding to leaves in the source backend and
//! reassembling in the target, the two halves running concurrently through one
//! [`FAN`]-bounded channel ([`subtree`]).
//!
//! The protocol itself converts nowhere: both parties of a session name one
//! backend, and a homogeneous session pays nothing. This module is what lets a
//! heterogeneous pair meet, by re-representing each node-carrying message.

use std::pin::pin;

use async_stream::try_stream;
use futures::channel::mpsc;
use futures::{SinkExt, StreamExt, stream};

use crate::tree::mirror::streaming::FAN;
use crate::tree::mirror::streaming::backend::{NodeStream, OptionNodeStream};
use crate::tree::mirror::streaming::message::{Close, Complete, Reply};
use crate::tree::typed::{
    Prefix,
    height::{Height, S, Z},
};

use super::Error;
use super::backend::{Backend, BoxNodeStream, Leaf, Node};
use super::message;
use super::protocol::Responses;

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

/// Re-represent a single node of `from`'s in `to`'s node type.
///
/// The node [explodes](Convert::explode) to leaves in `from` while `to`
/// concurrently [reassembles](Convert::assemble) them, the halves joined by a
/// [`FAN`]-bounded leaf channel; the cost is the subtree's size in time and one
/// fan in memory. Errors return in the producer's frame: `from`'s explosion
/// failures in first position, `to`'s reassembly failures in second.
async fn subtree<B, O, T, H>(
    from: &B,
    to: &O,
    prefix: Prefix<H>,
    node: B::Node<H>,
) -> Result<O::Node<H>, Error<B::Error, O::Error>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    O: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Convert,
{
    let (tx, rx) = mpsc::channel::<Result<(Prefix<Z>, O::Node<Z>), O::Error>>(FAN);

    let feed = async move {
        let mut tx = tx;
        let mut leaves = pin!(H::explode(
            from.clone(),
            Box::pin(stream::once(async move { Ok((prefix, node)) })),
        ));
        while let Some(item) = leaves.next().await {
            let (prefix, leaf) = item?;

            // The crossing: a leaf re-represents by value, no backend work.
            let version = leaf.ceiling().clone();
            let message = leaf.message().clone();
            if tx
                .send(Ok((prefix, Leaf::leaf(version, message))))
                .await
                .is_err()
            {
                // The build side stopped pulling: its own failure already
                // ends the conversion, so there is nothing left to feed.
                break;
            }
        }
        Ok::<_, B::Error>(())
    };

    let build = async {
        let mut nodes = pin!(H::assemble(to.clone(), Box::pin(rx)));
        nodes.next().await
    };

    let (fed, built) = futures::future::join(feed, build).await;
    // A feed failure truncates the leaf stream, which explains anything odd
    // downstream of it, so it outranks whatever the build half produced.
    fed.map_err(Error::Client)?;
    built
        .expect("a subtree's leaves reassemble to exactly one node")
        .map(|(_prefix, node)| node)
        .map_err(Error::Server)
}

/// Reassemble an ascending marked child stream into its marked parent level,
/// one complete radix group at a time.
pub(super) fn fold_parents<B, T, H>(
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
