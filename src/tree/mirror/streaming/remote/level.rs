//! One level's wire vocabulary, and the adapters between typed protocol
//! message streams and the level's transport stream.
//!
//! Every stream of the protocol schedule is a *level*, and each level rides
//! its own transport stream as a sequence of self-delimiting borsh
//! [`Item`]s: message headers ride whole ([`Requested`](Item::Requested),
//! [`Uncertain`](Item::Uncertain)), while a `providing` subtree rides as
//! its leaf run ([`Leaf`](Item::Leaf)… [`End`](Item::End); see [`codec`]).
//! A level's end — phase completion — is its stream's own end-of-file: no
//! marker rides in-band, and closing the write half is how a sender fins.
//!
//! Flow control is deliberately absent here: it is the *transport's*
//! contract. Each level's stream must be independently flow-controlled
//! (with a window of at least one fan), which is exactly what stream-native
//! transports (QUIC, one-connection-per-level TCP, an HTTP/2 adapter)
//! already provide; given that contract, the session is deadlock-free and
//! fixed-memory for the same reasons the materialized topology is.
//!
//! The adapters are the proxy's two directions around one level:
//!
//! - [`forward_exchanges`] (and its closing/complete/root variants) is the
//!   *encode* side: it drains the typed protocol messages the local party
//!   feeds the proxy into the level's [`Outgoing`] stream, then fins it.
//! - [`exchanges`] (and its variants) is the *decode* side: the proxy's
//!   returned response stream, parsing [`Incoming`] items back into typed
//!   protocol messages and reassembling leaf runs through the local
//!   party's backend.
//!
//! Decoding validates what the walks assume: message prefixes strictly
//! ascend within a level, and only the item kinds the level's message type
//! can express appear on it.

use std::marker::PhantomData;
use std::pin::pin;

use async_stream::try_stream;
use borsh::{BorshDeserialize, BorshSerialize};
use futures::StreamExt;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::Version;
use crate::message::Message;
use crate::tree::typed::{
    Hash, Prefix,
    height::{Height, S, Z},
};

use super::super::backend::{Backend, Leaf};
use super::super::convert::Convert;
use super::super::message;
use super::super::protocol::{Requests, Responses};
use super::codec::{self, DecodeError, Leaves};
use super::{Error, Violation, WireError};

/// The producer-frame error of the proxy's decoded response streams: wire
/// faults first, the reassembling backend's own faults second (the same
/// frame as [`OutputError`](super::super::protocol::OutputError)).
pub(super) type Fault<E> = crate::tree::mirror::Error<Error<E>, E>;

/// One bounded wire item of a node-carrying level.
///
/// `H` keys the level's prefixes; the borsh form leans on that (a
/// [`Prefix`] travels as exactly its `32 - H` bytes, no length). Kinds a
/// given level's message type cannot express are rejected by its decode
/// adapter, not representable-but-ignored.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Item<T, H: Height> {
    /// The sender lacks the subtree at the prefix: it asks the counterparty
    /// to provide.
    Requested(Prefix<H>),
    /// The sender disputes the subtree at the prefix: its children's hashes,
    /// ascending by radix.
    Uncertain(Prefix<H>, Vec<(u8, Hash)>),
    /// One leaf of the subtree currently being provided; the first leaf
    /// after a header (or another run's end) opens a new run.
    Leaf(Version, Message<T>),
    /// The end of the current leaf run (never sent at height `0`, where a
    /// run is statically one leaf).
    End,
}

// Manual borsh rather than a derive: the derive would demand `T:
// BorshSerialize` for serialization, which `Message<T>` deliberately does
// not need (it ships its cached bytes). Deserialization is strict — an
// `uncertain` listing must be nonempty, at most a fan, and strictly
// ascending by radix, exactly as every honest walk produces it — so
// malformed listings die at the parse instead of inside a merge-join.

impl<T, H: Height> BorshSerialize for Item<T, H> {
    fn serialize<W: borsh::io::Write>(&self, writer: &mut W) -> borsh::io::Result<()> {
        match self {
            Item::Requested(prefix) => {
                0u8.serialize(writer)?;
                prefix.serialize(writer)
            }
            Item::Uncertain(prefix, children) => {
                1u8.serialize(writer)?;
                prefix.serialize(writer)?;
                children.serialize(writer)
            }
            Item::Leaf(version, message) => {
                2u8.serialize(writer)?;
                version.serialize(writer)?;
                message.serialize(writer)
            }
            Item::End => 3u8.serialize(writer),
        }
    }
}

impl<T: BorshDeserialize, H: Height> BorshDeserialize for Item<T, H> {
    fn deserialize_reader<R: borsh::io::Read>(reader: &mut R) -> borsh::io::Result<Self> {
        let malformed =
            |message: &str| borsh::io::Error::new(borsh::io::ErrorKind::InvalidData, message);
        match u8::deserialize_reader(reader)? {
            0 => Ok(Item::Requested(Prefix::deserialize_reader(reader)?)),
            1 => {
                let prefix = Prefix::deserialize_reader(reader)?;
                let children = Vec::<(u8, Hash)>::deserialize_reader(reader)?;
                if children.is_empty() || children.len() > 256 {
                    return Err(malformed("uncertain listing has no or too many children"));
                }
                if !children.windows(2).all(|pair| pair[0].0 < pair[1].0) {
                    return Err(malformed(
                        "uncertain listing radices not strictly ascending",
                    ));
                }
                Ok(Item::Uncertain(prefix, children))
            }
            2 => Ok(Item::Leaf(
                Version::deserialize_reader(reader)?,
                Message::deserialize_reader(reader)?,
            )),
            3 => Ok(Item::End),
            tag => Err(borsh::io::Error::new(
                borsh::io::ErrorKind::InvalidData,
                format!("invalid item tag {tag:#04x}"),
            )),
        }
    }
}

/// The receiving half of one level: typed items parsed incrementally off
/// the level's transport stream, plus one item of replay.
///
/// The replay slot is how a leaf run hands off: the adapter loop pulls an
/// item to see what kind of message comes next, and a [`Leaf`](Item::Leaf)
/// means "a run opened here" — it goes back in the slot and the whole
/// [`Incoming`] lends itself to [`codec::decode`] as the run's [`Leaves`]
/// source.
pub(super) struct Incoming<R, I> {
    stream: R,
    /// Bytes read but not yet parsed; `parsed` of them belong to items
    /// already returned.
    buffer: Vec<u8>,
    parsed: usize,
    replayed: Option<I>,
    _item: PhantomData<fn() -> I>,
}

impl<R, I> Incoming<R, I>
where
    R: AsyncRead + Unpin + Send,
    I: BorshDeserialize,
{
    pub(super) fn new(stream: R) -> Self {
        Incoming {
            stream,
            buffer: Vec::new(),
            parsed: 0,
            replayed: None,
            _item: PhantomData,
        }
    }

    /// The next item, or `None` at the level's end (phase completion).
    ///
    /// Items are borsh, which is self-delimiting, so parsing needs no
    /// framing: parse what the buffer holds, read more when it holds a
    /// partial item. End-of-stream *between* items is the level's fin;
    /// end-of-stream inside one is a truncated session.
    pub(super) async fn next<E>(&mut self) -> Result<Option<I>, Error<E>> {
        if let Some(item) = self.replayed.take() {
            return Ok(Some(item));
        }
        loop {
            if self.parsed < self.buffer.len() {
                let mut cursor = Exhaustible {
                    bytes: &self.buffer[self.parsed..],
                    exhausted: false,
                };
                let unparsed = cursor.bytes.len();
                match I::deserialize_reader(&mut cursor) {
                    Ok(item) => {
                        self.parsed += unparsed - cursor.bytes.len();
                        return Ok(Some(item));
                    }
                    // A parse that touched the buffer's end wanted bytes the
                    // stream still owes: read on. The signal must be
                    // positional — borsh re-dresses end-of-input errors as
                    // `InvalidData`, indistinguishable from malformation by
                    // kind alone. (A malformed item lying about a length
                    // reads to end-of-stream and dies as truncation there.)
                    Err(_) if cursor.exhausted => {}
                    Err(error) => return Err(Error::Io(error)),
                }
            }
            self.buffer.drain(..self.parsed);
            self.parsed = 0;
            if self
                .stream
                .read_buf(&mut self.buffer)
                .await
                .map_err(Error::Io)?
                == 0
            {
                return if self.buffer.is_empty() {
                    Ok(None)
                } else {
                    Err(Error::Violation(Violation::Truncated))
                };
            }
        }
    }

    /// Put one item back; the next [`next`](Self::next) returns it again.
    fn replay(&mut self, item: I) {
        debug_assert!(self.replayed.is_none(), "at most one item of replay");
        self.replayed = Some(item);
    }
}

/// A slice reader that remembers whether a read ever ran it dry: the
/// positional partial-item signal [`Incoming::next`] relies on.
struct Exhaustible<'a> {
    bytes: &'a [u8],
    exhausted: bool,
}

impl borsh::io::Read for Exhaustible<'_> {
    fn read(&mut self, buf: &mut [u8]) -> borsh::io::Result<usize> {
        let read = borsh::io::Read::read(&mut self.bytes, buf)?;
        if read < buf.len() {
            self.exhausted = true;
        }
        Ok(read)
    }
}

/// Lending an [`Incoming`] to [`codec::decode`]: leaf items are the run,
/// [`End`](Item::End) is its terminator, and anything else — including the
/// level finning mid-run — is a violation.
impl<R, T, H> Leaves<T> for Incoming<R, Item<T, H>>
where
    R: AsyncRead + Unpin + Send,
    T: BorshDeserialize + Send + Sync,
    H: Height,
{
    async fn next(&mut self) -> Result<Option<(Version, Message<T>)>, WireError> {
        match Incoming::next(self).await? {
            Some(Item::Leaf(version, message)) => Ok(Some((version, message))),
            Some(Item::End) => Ok(None),
            Some(Item::Requested(..) | Item::Uncertain(..)) => {
                Err(WireError::Violation(Violation::UnexpectedItem))
            }
            None => Err(WireError::Violation(Violation::Truncated)),
        }
    }
}

/// The sending half of one level: typed items onto the level's transport
/// stream.
///
/// Each item writes whole and unbuffered — timely per-item delivery is
/// what the session's lockstep paces itself by, and batching below the
/// item is the transport's business.
pub(super) struct Outgoing<W> {
    stream: W,
}

impl<W> Outgoing<W>
where
    W: AsyncWrite + Unpin + Send,
{
    pub(super) fn new(stream: W) -> Self {
        Outgoing { stream }
    }

    /// Send one item.
    pub(super) async fn send<I: BorshSerialize>(&mut self, item: &I) -> std::io::Result<()> {
        let bytes = borsh::to_vec(item).expect("serializing a wire item into a Vec cannot fail");
        self.stream.write_all(&bytes).await
    }

    /// Fin the level: flush and shut the stream's write half down, so the
    /// counterparty observes end-of-file as phase completion.
    pub(super) async fn finish(mut self) -> std::io::Result<()> {
        self.stream.shutdown().await
    }
}

/// Forward one exchange level's requests onto its stream, then fin it.
///
/// The returned future is stage work: it drains the typed messages the
/// local party feeds the proxy into wire items, exploding each provided
/// subtree into its leaf run through `backend`. Transport failures surface
/// as [`Error::Io`]; the session's terminal collects them.
pub(super) async fn forward_exchanges<B, T, H, W>(
    backend: B,
    requests: impl Requests<message::Exchanged<B, T, H>>,
    mut outgoing: Outgoing<W>,
) -> Result<(), Error<B::Error>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Convert,
    W: AsyncWrite + Unpin + Send,
{
    let mut requests = pin!(requests);
    while let Some((prefix, exchange)) = requests.next().await {
        match exchange {
            message::Exchange::Requested => outgoing
                .send(&Item::<T, H>::Requested(prefix))
                .await
                .map_err(Error::Io)?,
            message::Exchange::Uncertain(children) => outgoing
                .send(&Item::<T, H>::Uncertain(prefix, children))
                .await
                .map_err(Error::Io)?,
            message::Exchange::Providing(node) => {
                run(&backend, prefix, node, &mut outgoing).await?
            }
        }
    }
    outgoing.finish().await.map_err(Error::Io)
}

/// Forward the closing level's requests onto its stream (the initiator's
/// leaf-parent round: [`forward_exchanges`] minus `uncertain`), then fin
/// it.
pub(super) async fn forward_closing<B, T, W>(
    backend: B,
    requests: impl Requests<(Prefix<S<Z>>, message::Closing<B, T>)>,
    mut outgoing: Outgoing<W>,
) -> Result<(), Error<B::Error>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    W: AsyncWrite + Unpin + Send,
{
    let mut requests = pin!(requests);
    while let Some((prefix, closing)) = requests.next().await {
        match closing {
            message::Closing::Requested => outgoing
                .send(&Item::<T, S<Z>>::Requested(prefix))
                .await
                .map_err(Error::Io)?,
            message::Closing::Providing(node) => run(&backend, prefix, node, &mut outgoing).await?,
        }
    }
    outgoing.finish().await.map_err(Error::Io)
}

/// Forward the complete level's requests onto its stream (the responder's
/// final word: bare single-leaf runs), then fin it.
pub(super) async fn forward_complete<B, T, W>(
    backend: B,
    requests: impl Requests<(Prefix<Z>, message::Complete<B, T>)>,
    mut outgoing: Outgoing<W>,
) -> Result<(), Error<B::Error>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    W: AsyncWrite + Unpin + Send,
{
    let mut requests = pin!(requests);
    while let Some((prefix, message::Complete::Providing(node))) = requests.next().await {
        run(&backend, prefix, node, &mut outgoing).await?;
    }
    outgoing.finish().await.map_err(Error::Io)
}

/// Forward the initiate level: the root hash, bare on its stream, then the
/// fin.
pub(super) async fn forward_initiate<W>(
    requests: impl Requests<message::Initiate>,
    mut outgoing: Outgoing<W>,
) -> std::io::Result<()>
where
    W: AsyncWrite + Unpin + Send,
{
    let mut requests = pin!(requests);
    while let Some(message::Initiate::Uncertain(hash)) = requests.next().await {
        outgoing.send(&hash).await?;
    }
    outgoing.finish().await
}

/// Forward the opening level: the responder's root-child listing, bare on
/// its stream, then the fin.
pub(super) async fn forward_opening<W>(
    requests: impl Requests<message::Opening>,
    mut outgoing: Outgoing<W>,
) -> std::io::Result<()>
where
    W: AsyncWrite + Unpin + Send,
{
    let mut requests = pin!(requests);
    while let Some(message::Opening::Uncertain(children)) = requests.next().await {
        outgoing.send(&children).await?;
    }
    outgoing.finish().await
}

/// Ship one provided subtree as its leaf run: the leaves, then — above
/// height `0`, where the run's length is not statically one — the
/// terminating [`End`](Item::End).
async fn run<B, T, H, W>(
    backend: &B,
    prefix: Prefix<H>,
    node: B::Node<H>,
    outgoing: &mut Outgoing<W>,
) -> Result<(), Error<B::Error>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Convert,
    W: AsyncWrite + Unpin + Send,
{
    let mut leaves = pin!(codec::encode(backend.clone(), prefix, node));
    while let Some(leaf) = leaves.next().await {
        let (version, message) = leaf?;
        outgoing
            .send(&Item::<T, H>::Leaf(version, message))
            .await
            .map_err(Error::Io)?;
    }
    if H::HEIGHT == 0 {
        return Ok(());
    }
    outgoing.send(&Item::<T, H>::End).await.map_err(Error::Io)
}

/// Enforce that a level's message prefixes strictly ascend: the merge-join
/// contract every walk rests on.
fn ascending<H: Height>(last: &mut Option<Prefix<H>>, next: Prefix<H>) -> Result<(), Violation> {
    if last.replace(next).is_some_and(|last| last >= next) {
        return Err(Violation::MessageOrder);
    }
    Ok(())
}

/// Decode one exchange level off its stream: the proxy's response stream
/// at height `H`.
///
/// Header items map straight to their messages; a leaf item opens a run,
/// which reassembles through `backend` into a `providing` node placed at
/// its derived prefix (see [`codec::decode`]). The stream's end-of-file is
/// the level's clean end: phase complete, stream over.
pub(super) fn exchanges<B, T, H, R>(
    backend: B,
    mut incoming: Incoming<R, Item<T, H>>,
) -> impl Responses<message::Exchanged<B, T, H>, Fault<B::Error>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: BorshDeserialize + Send + Sync + 'static,
    H: Convert,
    R: AsyncRead + Unpin + Send + 'static,
{
    try_stream! {
        let mut last = None;
        loop {
            match incoming.next().await? {
                None => return,
                Some(Item::Requested(prefix)) => {
                    ascending(&mut last, prefix).map_err(Error::Violation)?;
                    yield (prefix, message::Exchange::Requested);
                }
                Some(Item::Uncertain(prefix, children)) => {
                    ascending(&mut last, prefix).map_err(Error::Violation)?;
                    yield (prefix, message::Exchange::Uncertain(children));
                }
                Some(leaf @ Item::Leaf(..)) => {
                    incoming.replay(leaf);
                    let (prefix, node) = codec::decode(&backend, &mut incoming)
                        .await
                        .map_err(|fault: DecodeError<B::Error>| fault.map_client(WireError::widen))?;
                    ascending(&mut last, prefix).map_err(Error::Violation)?;
                    yield (prefix, message::Exchange::Providing(node));
                }
                Some(Item::End) => Err(Error::Violation(Violation::UnexpectedItem))?,
            }
        }
    }
}

/// Decode the closing level off its stream: [`exchanges`] minus
/// `uncertain`, which the closing message cannot express.
pub(super) fn closing<B, T, R>(
    backend: B,
    mut incoming: Incoming<R, Item<T, S<Z>>>,
) -> impl Responses<(Prefix<S<Z>>, message::Closing<B, T>), Fault<B::Error>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: BorshDeserialize + Send + Sync + 'static,
    R: AsyncRead + Unpin + Send + 'static,
{
    try_stream! {
        let mut last = None;
        loop {
            match incoming.next().await? {
                None => return,
                Some(Item::Requested(prefix)) => {
                    ascending(&mut last, prefix).map_err(Error::Violation)?;
                    yield (prefix, message::Closing::Requested);
                }
                Some(leaf @ Item::Leaf(..)) => {
                    incoming.replay(leaf);
                    let (prefix, node) = codec::decode(&backend, &mut incoming)
                        .await
                        .map_err(|fault: DecodeError<B::Error>| fault.map_client(WireError::widen))?;
                    ascending(&mut last, prefix).map_err(Error::Violation)?;
                    yield (prefix, message::Closing::Providing(node));
                }
                Some(Item::Uncertain(..) | Item::End) => {
                    Err(Error::Violation(Violation::UnexpectedItem))?
                }
            }
        }
    }
}

/// Decode the complete level off its stream: bare single-leaf runs, one
/// message each.
pub(super) fn complete<B, T, R>(
    backend: B,
    mut incoming: Incoming<R, Item<T, Z>>,
) -> impl Responses<(Prefix<Z>, message::Complete<B, T>), Fault<B::Error>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: BorshDeserialize + Send + Sync + 'static,
    R: AsyncRead + Unpin + Send + 'static,
{
    try_stream! {
        let mut last = None;
        loop {
            match incoming.next().await? {
                None => return,
                Some(leaf @ Item::Leaf(..)) => {
                    incoming.replay(leaf);
                    // Height 0: the run is statically one leaf; no `End`.
                    let (prefix, node) = codec::decode(&backend, &mut incoming)
                        .await
                        .map_err(|fault: DecodeError<B::Error>| fault.map_client(WireError::widen))?;
                    ascending(&mut last, prefix).map_err(Error::Violation)?;
                    yield (prefix, message::Complete::Providing(node));
                }
                Some(Item::Requested(..) | Item::Uncertain(..) | Item::End) => {
                    Err(Error::Violation(Violation::UnexpectedItem))?
                }
            }
        }
    }
}

/// Decode the initiate level off its stream: at most one bare root hash.
pub(super) fn initiate<E, R>(
    mut incoming: Incoming<R, Hash>,
) -> impl Responses<message::Initiate, Error<E>>
where
    E: Send + 'static,
    R: AsyncRead + Unpin + Send + 'static,
{
    try_stream! {
        while let Some(hash) = incoming.next().await? {
            yield message::Initiate::Uncertain(hash);
        }
    }
}

/// Decode the opening level off its stream: at most one bare root-child
/// listing.
pub(super) fn opening<E, R>(
    mut incoming: Incoming<R, Vec<(u8, Hash)>>,
) -> impl Responses<message::Opening, Error<E>>
where
    E: Send + 'static,
    R: AsyncRead + Unpin + Send + 'static,
{
    try_stream! {
        while let Some(children) = incoming.next().await? {
            yield message::Opening::Uncertain(children);
        }
    }
}

#[cfg(test)]
mod tests;
