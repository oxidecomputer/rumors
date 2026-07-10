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
use futures::{SinkExt, StreamExt};

use crate::tree::mirror::streaming::FAN;
use crate::tree::mirror::streaming::message::{Closing, Complete, Exchange};
use crate::tree::typed::{
    Prefix,
    height::{Height, S, Z},
};

use super::Error;
use super::backend::{Backend, BoxNodeStream, Leaf, Node, fold_parents, one};
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
        // Explode each node of the level singularly, in order: children of
        // distinct parents concatenate into the level below, still ascending.
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
        Box::pin(fold_parents(backend, below))
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
        let mut leaves = pin!(H::explode(from.clone(), Box::pin(one(prefix, node))));
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

/// A wire message re-representable across backends.
///
/// Only `providing` payloads carry nodes; every other message kind crosses
/// unchanged. Errors come back in the producer's frame, as with [`subtree`].
pub(super) trait Convertible<B, O, T>: Sized + Send
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    O: Backend<T, Node<Z>: Leaf<T>>,
{
    /// `Self`, in `to`'s node vocabulary.
    type Converted: Send + 'static;

    /// Re-represent this message in `to`'s node types.
    fn convert(
        self,
        from: &B,
        to: &O,
    ) -> impl Future<Output = Result<Self::Converted, Error<B::Error, O::Error>>> + Send;
}

/// Re-represent a whole outgoing stream in `to`'s node vocabulary.
///
/// This is the adapter the node-carrying protocol methods wrap their walks
/// in: the walk runs entirely in the session's own backend `B` and errors in
/// `B::Error`; the adapter [converts](Convertible) each message into `to`'s
/// node types and lifts the walk's own errors into the first position of the
/// producer-frame [`Error`] sum, where `to`'s reassembly failures occupy
/// the second.
pub(super) fn converted<B, O, T, M>(
    from: B,
    to: O,
    messages: impl Responses<M, B::Error>,
) -> impl Responses<M::Converted, Error<B::Error, O::Error>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    O: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    M: Convertible<B, O, T>,
{
    try_stream! {
        let mut messages = pin!(messages);
        while let Some(item) = messages.next().await {
            // The `?` lifts the walk's own error into the sum's first
            // position, through the one asymmetric `From` impl on `Error`.
            let message = item?;
            yield message.convert(&from, &to).await?;
        }
    }
}

impl<B, O, T> Convertible<B, O, T> for message::Opening
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    O: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Converted = message::Opening;

    async fn convert(self, _from: &B, _to: &O) -> Result<Self, Error<B::Error, O::Error>> {
        Ok(self)
    }
}

impl<B, O, T, H> Convertible<B, O, T> for Exchange<B, T, H>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    O: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Convert,
{
    type Converted = Exchange<O, T, H>;

    async fn convert(self, from: &B, to: &O) -> Result<Self::Converted, Error<B::Error, O::Error>> {
        Ok(match self {
            Exchange::Providing(prefix, node) => {
                Exchange::Providing(prefix, subtree(from, to, prefix, node).await?)
            }
            Exchange::Matched => Exchange::Matched,
            Exchange::Requested => Exchange::Requested,
            Exchange::Uncertain(children) => Exchange::Uncertain(children),
        })
    }
}

impl<B, O, T> Convertible<B, O, T> for Closing<B, T>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    O: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Converted = Closing<O, T>;

    async fn convert(self, from: &B, to: &O) -> Result<Self::Converted, Error<B::Error, O::Error>> {
        Ok(match self {
            Closing::Providing(prefix, node) => {
                Closing::Providing(prefix, subtree(from, to, prefix, node).await?)
            }
            Closing::Matched => Closing::Matched,
            Closing::Requested => Closing::Requested,
        })
    }
}

impl<B, O, T> Convertible<B, O, T> for Complete<B, T>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    O: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Converted = Complete<O, T>;

    async fn convert(self, from: &B, to: &O) -> Result<Self::Converted, Error<B::Error, O::Error>> {
        let Complete::Providing(prefix, node) = self;
        Ok(Complete::Providing(
            prefix,
            subtree(from, to, prefix, node).await?,
        ))
    }
}
