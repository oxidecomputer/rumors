//! Height-erased twins of the session's channel payloads, and the typed
//! facades that are the only way in or out of them.
//!
//! Every item the walk's bounded channels carry is height-indexed in the
//! type system but height-uniform at runtime: nodes erase to one
//! representation per backend ([`Backend::Erased`]), prefixes to their
//! bytes ([`ErasedPrefix`]), and nothing else in a payload ever depended
//! on the height. Minting channels of the erased twins therefore costs
//! nothing at runtime — every conversion below is a phantom-tag swap over
//! the value the program already holds — and collapses the channel
//! machinery from one instantiation per height to one per backend.
//!
//! # What the types stop proving, and what catches it instead
//!
//! Outside this module, sending a height-5 payload into a height-6 queue
//! is a compile error, exactly as before: the constructors here pair each
//! erased channel with typed facades ([`TypedSender`], [`TypedReceiver`],
//! [`TypedStream`], [`TypedOkStream`]) minted at one height parameter, so
//! both halves of an edge speak the same height by construction, and a
//! mispairing can only be authored *inside this module* by wiring a
//! constructor's two halves to different conversions. That one-module
//! audit surface is the design's locality argument. At runtime, every
//! prefix re-tag debug-asserts its byte length against the claimed height
//! ([`ErasedPrefix::assume`]), and every channel keeps its
//! [`QueueRole`] height label for the instrumented diagnostics.

use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::Stream;
#[cfg(not(test))]
use tokio_stream::wrappers::ReceiverStream;

use crate::tree::{
    mirror::streaming::{
        Backend, Leaf,
        channel::{QueueRole, Receiver, Sender, channel},
        materialized::{self, Error},
        message,
    },
    typed::{
        ErasedPrefix, Hash,
        height::{Height, S, Z},
    },
};

/// [`message::Reply`] with its height forgotten.
pub(crate) struct Reply<E> {
    pub replies: Vec<Reaction<E>>,
}

/// [`message::Reaction`] with its height forgotten.
pub(crate) enum Reaction<E> {
    Supply(u8, E),
    Match,
    Query(Vec<(u8, Hash)>),
}

/// [`materialized::Query`] with its height forgotten.
pub(crate) struct Query<E> {
    pub prefix: ErasedPrefix,
    pub ours: Vec<(u8, E)>,
}

/// [`materialized::Resolution`] with its height forgotten.
pub(crate) struct Resolution<E> {
    pub prefix: ErasedPrefix,
    pub resolved: Vec<(u8, Resolve<E>)>,
}

/// [`materialized::Resolve`] with its height forgotten.
pub(crate) enum Resolve<E> {
    Ready(Option<E>),
    Pending,
}

/// Shorthand for the erased node representation of one backend.
type ErasedOf<B, T> = <B as Backend<T>>::Erased;

/// The typed halves of the outgoing-response edge
/// ([`reply_channel`]): items are whole replies or the error that ends
/// the stream.
pub(crate) type ReplyResultSender<B, T, H> = TypedSender<
    Result<message::Reply<B, T, H>, Error<<B as Backend<T>>::Error>>,
    Result<Reply<ErasedOf<B, T>>, Error<<B as Backend<T>>::Error>>,
>;
/// The receiving half of [`reply_channel`], as the response stream shape.
pub(crate) type ReplyResultStream<B, T, H> = TypedStream<
    Result<message::Reply<B, T, H>, Error<<B as Backend<T>>::Error>>,
    Result<Reply<ErasedOf<B, T>>, Error<<B as Backend<T>>::Error>>,
>;
/// The typed sending half of a query edge ([`query_channel`]).
pub(crate) type QuerySender<B, T, H> =
    TypedSender<materialized::Query<B, T, H>, Query<ErasedOf<B, T>>>;
/// The typed receiving half of a query edge ([`query_channel`]).
pub(crate) type QueryReceiver<B, T, H> =
    TypedReceiver<materialized::Query<B, T, H>, Query<ErasedOf<B, T>>>;
/// The typed sending half of a return edge ([`return_channel`],
/// [`return_ok_channel`]): one reconciled node per query, in query order.
pub(crate) type ReturnSender<B, T, H> =
    TypedSender<Option<<B as Backend<T>>::Node<H>>, Option<ErasedOf<B, T>>>;
/// The typed receiving half of [`return_channel`].
pub(crate) type ReturnReceiver<B, T, H> =
    TypedReceiver<Option<<B as Backend<T>>::Node<H>>, Option<ErasedOf<B, T>>>;
/// The receiving half of [`return_ok_channel`], as an `Ok`-wrapping stream.
pub(crate) type ReturnOkStream<B, T, H> = TypedOkStream<
    Option<<B as Backend<T>>::Node<H>>,
    Option<ErasedOf<B, T>>,
    Error<<B as Backend<T>>::Error>,
>;
/// The typed sending half of a resolution edge ([`resolution_ok_channel`]).
pub(crate) type ResolutionSender<B, T, H> =
    TypedSender<materialized::Resolution<B, T, H>, Resolution<ErasedOf<B, T>>>;
/// The receiving half of [`resolution_ok_channel`], as an `Ok`-wrapping
/// stream.
pub(crate) type ResolutionOkStream<B, T, H> = TypedOkStream<
    materialized::Resolution<B, T, H>,
    Resolution<ErasedOf<B, T>>,
    Error<<B as Backend<T>>::Error>,
>;

/// Mint the outgoing-response edge: a typed sender and the typed response
/// stream its receiver drains into.
pub(crate) fn reply_channel<B, T, H>(
    role: QueueRole,
    capacity: usize,
) -> (ReplyResultSender<B, T, H>, ReplyResultStream<B, T, H>)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
{
    let (sender, receiver) = channel(role, capacity);
    (
        TypedSender::new(sender, |item: Result<_, _>| {
            item.map(erase_reply::<B, T, H>)
        }),
        TypedStream::new(receiver, |item: Result<_, _>| {
            item.map(assume_reply::<B, T, H>)
        }),
    )
}

/// Mint one query edge at height `H` (the children's height; the scope
/// sits at `S<H>`).
pub(crate) fn query_channel<B, T, H>(
    role: QueueRole,
    capacity: usize,
) -> (QuerySender<B, T, H>, QueryReceiver<B, T, H>)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
    S<H>: Height,
{
    let (sender, receiver) = channel(role, capacity);
    (
        TypedSender::new(sender, erase_query::<B, T, H>),
        TypedReceiver::new(receiver, assume_query::<B, T, H>),
    )
}

/// Mint one return edge at height `H`, received item by item.
pub(crate) fn return_channel<B, T, H>(
    role: QueueRole,
    capacity: usize,
) -> (ReturnSender<B, T, H>, ReturnReceiver<B, T, H>)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
{
    let (sender, receiver) = channel(role, capacity);
    (
        TypedSender::new(sender, erase_return::<B, T, H>),
        TypedReceiver::new(receiver, assume_return::<B, T, H>),
    )
}

/// Mint one return edge at height `H`, received as an `Ok`-wrapping stream.
pub(crate) fn return_ok_channel<B, T, H>(
    role: QueueRole,
    capacity: usize,
) -> (ReturnSender<B, T, H>, ReturnOkStream<B, T, H>)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
{
    let (sender, receiver) = channel(role, capacity);
    (
        TypedSender::new(sender, erase_return::<B, T, H>),
        TypedOkStream::new(receiver, assume_return::<B, T, H>),
    )
}

/// Mint one resolution edge at height `H`, received as an `Ok`-wrapping
/// stream.
pub(crate) fn resolution_ok_channel<B, T, H>(
    role: QueueRole,
    capacity: usize,
) -> (ResolutionSender<B, T, H>, ResolutionOkStream<B, T, H>)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
    S<H>: Height,
{
    let (sender, receiver) = channel(role, capacity);
    (
        TypedSender::new(sender, erase_resolution::<B, T, H>),
        TypedOkStream::new(receiver, assume_resolution::<B, T, H>),
    )
}

fn erase_reply<B, T, H>(reply: message::Reply<B, T, H>) -> Reply<ErasedOf<B, T>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
{
    Reply {
        replies: reply
            .replies
            .into_iter()
            .map(|reaction| match reaction {
                message::Reaction::Supply(radix, node) => Reaction::Supply(radix, B::erase(node)),
                message::Reaction::Match => Reaction::Match,
                message::Reaction::Query(listing) => Reaction::Query(listing),
            })
            .collect(),
    }
}

fn assume_reply<B, T, H>(reply: Reply<ErasedOf<B, T>>) -> message::Reply<B, T, H>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
{
    message::Reply {
        replies: reply
            .replies
            .into_iter()
            .map(|reaction| match reaction {
                Reaction::Supply(radix, node) => message::Reaction::Supply(radix, B::assume(node)),
                Reaction::Match => message::Reaction::Match,
                Reaction::Query(listing) => message::Reaction::Query(listing),
            })
            .collect(),
    }
}

fn erase_query<B, T, H>(query: materialized::Query<B, T, H>) -> Query<ErasedOf<B, T>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
    S<H>: Height,
{
    Query {
        prefix: query.prefix.erase(),
        ours: query
            .ours
            .into_iter()
            .map(|(radix, node)| (radix, B::erase(node)))
            .collect(),
    }
}

fn assume_query<B, T, H>(query: Query<ErasedOf<B, T>>) -> materialized::Query<B, T, H>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
    S<H>: Height,
{
    materialized::Query {
        prefix: query.prefix.assume(),
        ours: query
            .ours
            .into_iter()
            .map(|(radix, node)| (radix, B::assume(node)))
            .collect(),
    }
}

fn erase_resolution<B, T, H>(
    resolution: materialized::Resolution<B, T, H>,
) -> Resolution<ErasedOf<B, T>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
    S<H>: Height,
{
    Resolution {
        prefix: resolution.prefix.erase(),
        resolved: resolution
            .resolved
            .into_iter()
            .map(|(radix, slot)| {
                (
                    radix,
                    match slot {
                        materialized::Resolve::Ready(node) => Resolve::Ready(node.map(B::erase)),
                        materialized::Resolve::Pending => Resolve::Pending,
                    },
                )
            })
            .collect(),
    }
}

fn assume_resolution<B, T, H>(
    resolution: Resolution<ErasedOf<B, T>>,
) -> materialized::Resolution<B, T, H>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
    S<H>: Height,
{
    materialized::Resolution {
        prefix: resolution.prefix.assume(),
        resolved: resolution
            .resolved
            .into_iter()
            .map(|(radix, slot)| {
                (
                    radix,
                    match slot {
                        Resolve::Ready(node) => {
                            materialized::Resolve::Ready(node.map(B::assume::<H>))
                        }
                        Resolve::Pending => materialized::Resolve::Pending,
                    },
                )
            })
            .collect(),
    }
}

fn erase_return<B, T, H>(node: Option<B::Node<H>>) -> Option<ErasedOf<B, T>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
{
    node.map(B::erase)
}

fn assume_return<B, T, H>(node: Option<ErasedOf<B, T>>) -> Option<B::Node<H>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
{
    node.map(B::assume::<H>)
}

// --------------------------------------------------------------------------------
// The typed facades: each pairs one erased channel half with the fixed
// conversion its constructor minted it with.
// --------------------------------------------------------------------------------

/// The typed sending half of an erased channel.
pub(crate) struct TypedSender<M, E> {
    inner: Sender<E>,
    erase: fn(M) -> E,
}

impl<M, E> Clone for TypedSender<M, E> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            erase: self.erase,
        }
    }
}

impl<M, E: Send> TypedSender<M, E> {
    fn new(inner: Sender<E>, erase: fn(M) -> E) -> Self {
        Self { inner, erase }
    }

    /// Send one typed item, erased in place.
    ///
    /// Like the underlying channel's send: an error means the receiving
    /// half is gone, and the producer should wind down.
    pub(crate) async fn send(&self, message: M) -> Result<(), ClosedChannel> {
        self.inner
            .send((self.erase)(message))
            .await
            .map_err(|_| ClosedChannel)
    }
}

/// The receiver of a typed send has hung up; the payload is dropped.
///
/// The typed sender cannot return the underlying
/// [`SendError`](tokio::sync::mpsc::error::SendError) because that hands
/// back the *erased* payload; no caller inspects it — a failed send means
/// "stop producing" on every edge.
#[derive(Debug)]
pub(crate) struct ClosedChannel;

/// The typed receiving half of an erased channel.
pub(crate) struct TypedReceiver<M, E> {
    inner: Receiver<E>,
    assume: fn(E) -> M,
}

impl<M, E: Send> TypedReceiver<M, E> {
    fn new(inner: Receiver<E>, assume: fn(E) -> M) -> Self {
        Self { inner, assume }
    }

    /// Receive one typed item, re-tagged in place.
    pub(crate) async fn recv(&mut self) -> Option<M> {
        self.inner.recv().await.map(self.assume)
    }
}

/// The channel receiver as a stream, uniform across the test and
/// production channel types.
#[cfg(test)]
type ReceiverStreamOf<E> = Receiver<E>;
/// The channel receiver as a stream, uniform across the test and
/// production channel types.
#[cfg(not(test))]
type ReceiverStreamOf<E> = ReceiverStream<E>;

fn receiver_stream<E: Send>(receiver: Receiver<E>) -> ReceiverStreamOf<E> {
    #[cfg(test)]
    {
        receiver
    }
    #[cfg(not(test))]
    {
        ReceiverStream::new(receiver)
    }
}

/// An erased channel's receiving half as a stream of typed items.
pub(crate) struct TypedStream<M, E> {
    inner: ReceiverStreamOf<E>,
    assume: fn(E) -> M,
}

impl<M, E: Send> TypedStream<M, E> {
    fn new(inner: Receiver<E>, assume: fn(E) -> M) -> Self {
        Self {
            inner: receiver_stream(inner),
            assume,
        }
    }
}

impl<M, E: Send> Stream for TypedStream<M, E> {
    type Item = M;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        Pin::new(&mut this.inner)
            .poll_next(cx)
            .map(|item| item.map(this.assume))
    }
}

/// An erased channel's receiving half as a stream of `Ok`-wrapped typed
/// items: the shape the assembly and walk consumers pull from.
pub(crate) struct TypedOkStream<M, E, Err> {
    inner: ReceiverStreamOf<E>,
    assume: fn(E) -> M,
    error: PhantomData<fn() -> Err>,
}

impl<M, E: Send, Err> TypedOkStream<M, E, Err> {
    fn new(inner: Receiver<E>, assume: fn(E) -> M) -> Self {
        Self {
            inner: receiver_stream(inner),
            assume,
            error: PhantomData,
        }
    }
}

impl<M, E: Send, Err> Stream for TypedOkStream<M, E, Err> {
    type Item = Result<M, Err>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        Pin::new(&mut this.inner)
            .poll_next(cx)
            .map(|item| item.map(|item| Ok((this.assume)(item))))
    }
}
