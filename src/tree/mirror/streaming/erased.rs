//! The height-erased seam of the streaming session: the wire vocabulary's
//! erased twin, the typed exits, and the dispatch back into the typed
//! backend surface.
//!
//! The materialized walk's payloads and workers are height-uniform at
//! runtime: nodes erase to one representation per backend
//! ([`Backend::Erased`]), prefixes to their bytes
//! ([`ErasedPrefix`](crate::tree::typed::ErasedPrefix)), and nothing else
//! in a payload ever depended on the height. The walk therefore runs on
//! erased values — one instantiation of its channels, generators, and
//! loops per backend, instead of one per height — while the protocol
//! schedule around it stays fully typed. This module owns the seam
//! between the two:
//!
//! - [`Reply`] and [`Reaction`] are [`message::Reply`]'s and
//!   [`message::Reaction`]'s erased twins, converted exactly at the
//!   schedule boundary: [`erase_reply`] where a typed request stream
//!   enters a walk worker, and [`reply_channel`]'s typed exit where a
//!   worker's responses become the schedule's typed response stream.
//!   Every conversion is a phantom-tag swap over values the program
//!   already holds.
//! - [`ops`] carries erased node operations back into the height-typed
//!   [`Backend`] surface, selecting the type-level height from the one
//!   runtime witness every erased scope carries: its prefix's byte
//!   length.
//!
//! # What the types stop proving, and what catches it instead
//!
//! Outside the walk, pairing a height-5 payload with a height-6 consumer
//! is a compile error, exactly as before: the schedule's typestates and
//! message streams remain height-typed, and this module's two boundary
//! conversions are minted at one height parameter apiece. Inside the
//! walk, height agreement is a runtime-witnessed property: every prefix
//! re-tag debug-asserts its byte length against the claimed height, the
//! [`ops`] dispatch derives its height from that same length (so the
//! coordinate and the witness cannot drift apart), and every channel
//! keeps its [`QueueRole`] height label for
//! the instrumented diagnostics. The behavioral pins — the alternating
//! oracle, the violation and capacity suites, the byte-pinned wire
//! snapshots — exercise exactly these pairings.

use std::pin::Pin;
use std::task::{Context, Poll};

use futures::Stream;
#[cfg(not(test))]
use tokio_stream::wrappers::ReceiverStream;

use crate::tree::{
    mirror::streaming::{
        Backend, Leaf,
        channel::{QueueRole, Receiver, Sender, channel},
        message,
    },
    typed::{
        Hash,
        height::{Height, Z},
    },
};

/// [`message::Reply`] with its height forgotten: what the walk's workers
/// produce and consume.
pub(crate) struct Reply<E> {
    pub replies: Vec<Reaction<E>>,
}

/// [`message::Reaction`] with its height forgotten.
pub(crate) enum Reaction<E> {
    Supply(u8, E),
    Match,
    Query(Vec<(u8, Hash)>),
}

/// Erase one typed reply where a schedule-typed request stream enters a
/// walk worker.
pub(crate) fn erase_reply<B, T, H>(reply: message::Reply<B, T, H>) -> Reply<B::Erased>
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

/// Re-tag one erased reply at the typed exit of [`reply_channel`].
fn assume_reply<B, T, H>(reply: Reply<B::Erased>) -> message::Reply<B, T, H>
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

/// The typed exit of [`reply_channel`]: the erased receiver as a stream
/// of schedule-typed replies. `Err` is the session's error type, passed
/// through untouched.
pub(crate) struct ReplyResultStream<B, T, H, Err>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
{
    inner: ReceiverStreamOf<Result<Reply<B::Erased>, Err>>,
    assume: fn(Result<Reply<B::Erased>, Err>) -> Result<message::Reply<B, T, H>, Err>,
}

impl<B, T, H, Err> Stream for ReplyResultStream<B, T, H, Err>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
    Err: Send,
{
    type Item = Result<message::Reply<B, T, H>, Err>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        Pin::new(&mut this.inner)
            .poll_next(cx)
            .map(|item| item.map(this.assume))
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

/// Mint the outgoing-response edge: an erased sender for the response
/// pump, and the typed response stream the schedule consumes.
///
/// The one edge whose two halves speak different vocabularies — erased
/// in, typed out — and therefore the outgoing half of the walk's typed
/// boundary. Its height parameter fixes the exit's re-tag; the erased
/// sender needs none.
pub(crate) fn reply_channel<B, T, H, Err>(
    role: QueueRole,
    capacity: usize,
) -> (
    Sender<Result<Reply<B::Erased>, Err>>,
    ReplyResultStream<B, T, H, Err>,
)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
    Err: Send,
{
    let (sender, receiver) = channel(role, capacity);
    (
        sender,
        ReplyResultStream {
            inner: receiver_stream(receiver),
            assume: |item| item.map(assume_reply::<B, T, H>),
        },
    )
}

/// Erased node operations dispatched back into the height-typed
/// [`Backend`] surface.
///
/// The backend's traversal operations are generic over type-level
/// heights; an erased caller selects the height at runtime from the one
/// witness every erased scope carries — its prefix's byte length — via a
/// 33-arm match whose arms each instantiate one thin typed call. Deriving
/// the height from the prefix (rather than threading a separate counter)
/// is what keeps the coordinate and the witness structurally inseparable.
pub(crate) mod ops {
    use futures::StreamExt;

    use super::*;
    use crate::tree::{
        mirror::streaming::{
            backend::BoxNodeStream, materialized::children_of as children_of_typed,
        },
        typed::{ErasedPrefix, height::Pred},
    };

    /// Select the type-level height matching a runtime *parent* height.
    ///
    /// `$H` is bound to the parent's height type within `$body`, and
    /// `<$H as Pred>::Pred` names the children's. Each arm monomorphizes
    /// the body once, so bodies must stay thin.
    macro_rules! at_parent_height {
        ($height:expr, $H:ident => $body:expr) => {{
            seq_macro::seq!(N in 1..=32 {
                match $height {
                    0 => unreachable!("a leaf-height node has no children"),
                    #(N => { type $H = crate::tree::typed::height::H~N; $body })*
                    _ => unreachable!("a tree height is 0..=32"),
                }
            })
        }};
    }

    /// Select the type-level height matching a runtime height, leaves
    /// included: [`at_parent_height!`] minus the nonzero premise.
    macro_rules! at_height {
        ($height:expr, $H:ident => $body:expr) => {{
            seq_macro::seq!(N in 0..=32 {
                match $height {
                    #(N => { type $H = crate::tree::typed::height::H~N; $body })*
                    _ => unreachable!("a tree height is 0..=32"),
                }
            })
        }};
    }

    /// Collect one erased node's children, addressed by radix
    /// ([`children_of_typed`], erased).
    ///
    /// `prefix` is the node's own prefix; its length names the height the
    /// node is re-tagged at, so a walk cannot explode a node at any level
    /// other than the one its coordinate claims.
    pub(crate) async fn children_of<B, T>(
        backend: &B,
        prefix: ErasedPrefix,
        node: B::Erased,
    ) -> Result<Vec<(u8, B::Erased)>, B::Error>
    where
        B: Backend<T, Node<Z>: Leaf<T>>,
        T: Send + Sync + 'static,
    {
        at_parent_height!(prefix.height(), H => {
            let children =
                children_of_typed::<B, T, <H as Pred>::Pred>(
                    backend,
                    prefix.assume::<H>(),
                    B::assume::<H>(node),
                )
                .await?;
            Ok(children
                .into_iter()
                .map(|(radix, child)| (radix, B::erase(child)))
                .collect())
        })
    }

    /// Assemble one erased parent node at `prefix` from one radix-keyed
    /// child group ([`Backend::parent`], erased).
    ///
    /// `prefix` is the parent's own prefix, carrying the same
    /// length-is-height witness as [`children_of`].
    pub(crate) async fn parent<B, T>(
        backend: B,
        prefix: ErasedPrefix,
        children: Vec<(u8, Option<B::Erased>)>,
    ) -> Result<Option<B::Erased>, B::Error>
    where
        B: Backend<T, Node<Z>: Leaf<T>>,
        T: Send + Sync + 'static,
    {
        at_parent_height!(prefix.height(), H => {
            let children = children
                .into_iter()
                .map(|(radix, child)| (radix, child.map(B::assume::<<H as Pred>::Pred>)))
                .collect();
            Ok(backend
                .parent::<<H as Pred>::Pred>(prefix.assume::<H>(), children)
                .await?
                .map(B::erase))
        })
    }

    /// Walk every leaf beneath an erased node, in ascending path order
    /// ([`Backend::leaves`], erased).
    ///
    /// The yielded leaves stay typed: `Z` is a single height, so nothing
    /// about a leaf ever needed erasing.
    pub(crate) fn leaves<B, T>(
        backend: B,
        prefix: ErasedPrefix,
        node: B::Erased,
    ) -> BoxNodeStream<'static, B, T, Z>
    where
        B: Backend<T, Node<Z>: Leaf<T>>,
        T: Send + Sync + 'static,
    {
        at_height!(prefix.height(), H => {
            Box::pin(backend.leaves::<H>(prefix.assume::<H>(), B::assume::<H>(node)))
        })
    }

    /// Assemble a strictly ascending leaf stream into erased nodes at
    /// `height`, one per maximal same-prefix run, in run order
    /// ([`Backend::assemble`], erased).
    ///
    /// The one dispatch keyed by an explicit height rather than a prefix:
    /// its input is a whole leaf stream, and the target height is the
    /// consuming scope's — whose prefix length the caller derives it from.
    pub(crate) fn assemble<B, T>(
        backend: B,
        height: usize,
        leaves: BoxNodeStream<'static, B, T, Z>,
    ) -> Pin<Box<dyn Stream<Item = Result<(ErasedPrefix, B::Erased), B::Error>> + Send>>
    where
        B: Backend<T, Node<Z>: Leaf<T>>,
        T: Send + Sync + 'static,
    {
        at_height!(height, H => {
            Box::pin(
                backend
                    .assemble::<H>(leaves)
                    .map(|item| item.map(|(prefix, node)| (prefix.erase(), B::erase(node)))),
            )
        })
    }
}
