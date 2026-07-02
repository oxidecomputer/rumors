//! Re-represent nodes from one backend in the node types of another.
//!
//! [`convert`] re-represents a whole prefix-ordered [`NodeStream`];
//! [`Convertible`] re-represents one wire message, converting the node its
//! `providing` payload carries (every other message kind crosses backends
//! unchanged). The latter is what the in-process driver's party boundary maps
//! over — and what a wire transport does implicitly when it serializes one
//! side's nodes and deserializes them into the other's.
//!
//! Only nodes cross the conversion; errors never do. Just as a wire transport
//! carries no representation of the counterparty's failures — a peer that
//! errors simply stops sending — a source-side failure here is handed to a
//! `divert` callback and the stream ends. Target-side failures are the output
//! stream's own: reassembling the re-represented subtree is the target
//! backend's work, and its errors flow in-band. Keeping each error on its own
//! side is what lets the driver run two backends with unrelated error types.

use std::pin::pin;

use futures::{StreamExt, future};

use crate::tree::typed::{
    Prefix,
    height::{Height, S, Z},
};

use super::backend::{Backend, BoxNodeStream, Leaf, Node, NodeStream, one};
use super::message;

/// Convert a `stream` of `from`'s nodes at height `H` into the equivalent
/// stream of `to`'s nodes.
///
/// Source-side stream errors are passed to `divert` and end the output
/// stream; target-side errors surface in-band.
pub fn convert<B, C, T, H>(
    from: B,
    to: C,
    divert: impl Fn(B::Error) + Clone + Send + 'static,
    stream: impl NodeStream<B, T, H> + 'static,
) -> impl NodeStream<C, T, H>
where
    B: Backend<T>,
    C: Backend<T>,
    B::Node<Z>: Leaf<T>,
    C::Node<Z>: Leaf<T>,
    T: Send + Sync + 'static,
    H: Convert,
{
    H::convert(from, to, divert, stream)
}

/// A height at which a [`NodeStream`] can be re-represented across backends.
pub trait Convert: Height {
    /// Re-represent `stream`, a prefix-ordered stream of `from`'s nodes at this
    /// height, as the equivalent stream of `to`'s nodes.
    ///
    /// Order is preserved: the output carries the same prefixes in the same
    /// strictly-increasing order as the input. Takes the handles by value so
    /// the returned stream owns them and stays `'static` (the module-wide
    /// convention; see [`Backend::children`]).
    fn convert<B, C, T>(
        from: B,
        to: C,
        divert: impl Fn(B::Error) + Clone + Send + 'static,
        stream: impl NodeStream<B, T, Self> + 'static,
    ) -> impl NodeStream<C, T, Self>
    where
        B: Backend<T>,
        C: Backend<T>,
        B::Node<Z>: Leaf<T>,
        C::Node<Z>: Leaf<T>,
        T: Send + Sync + 'static;
}

impl Convert for Z {
    fn convert<B, C, T>(
        _from: B,
        _to: C,
        divert: impl Fn(B::Error) + Clone + Send + 'static,
        stream: impl NodeStream<B, T, Z> + 'static,
    ) -> impl NodeStream<C, T, Z>
    where
        B: Backend<T>,
        C: Backend<T>,
        B::Node<Z>: Leaf<T>,
        C::Node<Z>: Leaf<T>,
        T: Send + Sync + 'static,
    {
        // `scan` ends the stream at the first source-side error: downstream
        // sees clean early termination, and the error itself travels only
        // through `divert`.
        stream.scan((), move |(), item| {
            future::ready(match item {
                Ok((prefix, leaf)) => {
                    // A leaf's ceiling and floor are both equal to its version:
                    let version = leaf.ceiling().clone();
                    let message = leaf.message().clone();
                    Some(Ok::<_, C::Error>((prefix, Leaf::leaf(version, message))))
                }
                Err(error) => {
                    divert(error);
                    None
                }
            })
        })
    }
}

impl<H> Convert for S<H>
where
    H: Convert,
    S<H>: Height,
{
    fn convert<B, C, T>(
        from: B,
        to: C,
        divert: impl Fn(B::Error) + Clone + Send + 'static,
        stream: impl NodeStream<B, T, S<H>> + 'static,
    ) -> impl NodeStream<C, T, S<H>>
    where
        B: Backend<T>,
        C: Backend<T>,
        B::Node<Z>: Leaf<T>,
        C::Node<Z>: Leaf<T>,
        T: Send + Sync + 'static,
    {
        // Boxed on both sides of the recursion, or the full-height
        // instantiation nests seventeen levels of combinators into one type
        // (see [`BoxNodeStream`]).
        let children: BoxNodeStream<B, T, H> = Box::pin(from.clone().children::<H>(stream));
        let converted: BoxNodeStream<C, T, H> =
            Box::pin(H::convert::<B, C, T>(from, to.clone(), divert, children));
        to.parents::<H>(converted)
    }
}

/// Re-represent a single node of `from`'s in `to`'s node type.
///
/// The [`Convert`] machinery works on whole prefix-ordered streams; a wire
/// message carries a single subtree, so this drains the converted [`one`]
/// stream. The cost is the subtree's size: the node is exploded to leaves in
/// `from` and reassembled in `to`.
///
/// Returns `Ok(None)` when a source-side error cut the conversion short: the
/// error itself went to `divert`, and the one-node stream came up empty.
async fn subtree<B, C, T, H>(
    from: &B,
    to: &C,
    divert: impl Fn(B::Error) + Clone + Send + 'static,
    prefix: Prefix<H>,
    node: B::Node<H>,
) -> Result<Option<C::Node<H>>, C::Error>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    C: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Convert,
{
    let mut converted = pin!(convert(from.clone(), to.clone(), divert, one(prefix, node)));
    match converted.next().await {
        Some(item) => item.map(|(_prefix, node)| Some(node)),
        None => Ok(None),
    }
}

/// A wire message re-representable across backends.
///
/// Only `providing` payloads carry nodes; every other message kind crosses
/// unchanged. As with [`convert`], source-side errors go to `divert` (the
/// message is then lost with the conversion, hence `Ok(None)`); target-side
/// errors return in-band.
pub(super) trait Convertible<B, C, T>: Sized + Send
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    C: Backend<T, Node<Z>: Leaf<T>>,
{
    /// `Self`, in `to`'s node vocabulary.
    type Converted: Send + 'static;

    /// Re-represent this message in `to`'s node types.
    fn convert(
        self,
        from: &B,
        to: &C,
        divert: impl Fn(B::Error) + Clone + Send + 'static,
    ) -> impl Future<Output = Result<Option<Self::Converted>, C::Error>> + Send;
}

impl<B, C, T> Convertible<B, C, T> for message::Initiate
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    C: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Converted = message::Initiate;

    async fn convert(
        self,
        _from: &B,
        _to: &C,
        _divert: impl Fn(B::Error) + Clone + Send + 'static,
    ) -> Result<Option<Self>, C::Error> {
        Ok(Some(self))
    }
}

impl<B, C, T> Convertible<B, C, T> for message::Opening
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    C: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Converted = message::Opening;

    async fn convert(
        self,
        _from: &B,
        _to: &C,
        _divert: impl Fn(B::Error) + Clone + Send + 'static,
    ) -> Result<Option<Self>, C::Error> {
        Ok(Some(self))
    }
}

impl<B, C, T, H> Convertible<B, C, T> for message::Exchange<B, T, H>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    C: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
    S<H>: Convert,
{
    type Converted = message::Exchange<C, T, H>;

    async fn convert(
        self,
        from: &B,
        to: &C,
        divert: impl Fn(B::Error) + Clone + Send + 'static,
    ) -> Result<Option<Self::Converted>, C::Error> {
        Ok(match self {
            message::Exchange::Providing(message::Providing { prefix, node }) => {
                subtree(from, to, divert, prefix, node)
                    .await?
                    .map(|node| message::Exchange::Providing(message::Providing { prefix, node }))
            }
            message::Exchange::Requested(requested) => {
                Some(message::Exchange::Requested(requested))
            }
            message::Exchange::Uncertain(uncertain) => {
                Some(message::Exchange::Uncertain(uncertain))
            }
        })
    }
}

impl<B, C, T> Convertible<B, C, T> for message::Closing<B, T>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    C: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Converted = message::Closing<C, T>;

    async fn convert(
        self,
        from: &B,
        to: &C,
        divert: impl Fn(B::Error) + Clone + Send + 'static,
    ) -> Result<Option<Self::Converted>, C::Error> {
        Ok(match self {
            message::Closing::Providing(message::Providing { prefix, node }) => {
                subtree(from, to, divert, prefix, node)
                    .await?
                    .map(|node| message::Closing::Providing(message::Providing { prefix, node }))
            }
            message::Closing::Requested(requested) => Some(message::Closing::Requested(requested)),
        })
    }
}

impl<B, C, T> Convertible<B, C, T> for message::Complete<B, T>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    C: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Converted = message::Complete<C, T>;

    async fn convert(
        self,
        from: &B,
        to: &C,
        divert: impl Fn(B::Error) + Clone + Send + 'static,
    ) -> Result<Option<Self::Converted>, C::Error> {
        Ok(match self {
            message::Complete::Providing(message::Providing { prefix, node }) => {
                subtree(from, to, divert, prefix, node)
                    .await?
                    .map(|node| message::Complete::Providing(message::Providing { prefix, node }))
            }
        })
    }
}
