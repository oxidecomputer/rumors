//! A party of one backend, presented in another's node vocabulary.
//!
//! The protocol traits key their messages by a single backend, so two parties
//! pair only when they name the same one. [`Converted`] is what pairs a party
//! whose tree lives in `A` with a counterparty that speaks `B`: it wraps the
//! party and re-represents every node crossing it, inbound and outbound.
//!
//! Exactly one side of a session wraps. A homogeneous session names
//! `Converted` nowhere and pays nothing.
//!
//! # Where the faults go
//!
//! Conversion is fallible in both directions, but only one direction has
//! somewhere to say so: a stage's outgoing stream is a [`Responses`], while
//! its incoming stream is a [`Requests`] — structurally non-erroring, because
//! the driver has already lifted the *producer's* errors out of band (see
//! [`divert`](super::super::divert)).
//!
//! So the wrapper reproduces that trick one level down. Each stage opens a
//! one-error slot: an inbound conversion fault lands in the slot and parks the
//! incoming stream, and the stage's own outgoing stream — racing the slot —
//! surfaces the fault as an ordinary outgoing error, where the driver's
//! `divert` picks it up and abandons the session. The terminal stage
//! ([`CompleteInitiator`](protocol::CompleteInitiator)), which has no outgoing
//! stream, races the slot against its output future instead.
//!
//! Parking rather than ending matters for the same reason it does in the
//! driver: end-of-stream means phase completion, and a truncated phase would
//! be misread as a complete one.

use std::marker::PhantomData;
use std::pin::pin;

use async_stream::stream;
use futures::stream::StreamExt;
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::{
    Version,
    tree::typed::{
        Prefix,
        height::{Pred, S, UnderRoot, UnderUnderRoot, Z},
    },
};

use super::super::backend::{Backend, Leaf};
use super::super::message;
use super::super::protocol::{self, BoxResponses, Requests, Responses};
use super::{Convert, Convertible, converted};

use crate::tree::mirror::Error;

/// A wrapped party's error: its own faults first, faults of representing one of
/// its nodes in the counterparty's backend `B` second.
///
/// This is the producer's frame, the same convention [`subtree`](super::subtree)
/// returns in: what the wrapped party failed at, versus what `B` failed to
/// build.
type Fault<A, B, T> = Error<<A as Backend<T>>::Error, <B as Backend<T>>::Error>;

/// A party of `A`'s, speaking `B`'s node vocabulary.
///
/// `Converted` implements every protocol trait at backend `B` that `party`
/// implements at backend `A` — [`Client`](protocol::Client),
/// [`Server`](protocol::Server), [`Peer`](protocol::Peer) and the stages
/// beneath them — so wrapping one side of a session is what makes two
/// backends pair:
///
/// ```ignore
/// let ours = Handshaking::start(Local, root);            // Peer<Local, T>
/// let theirs = Converted::new(                           // Peer<Local, T>
///     Handshaking::start(Persistent, their_root),        //   ..of a Peer<Persistent, T>
///     Persistent,
///     Local,
/// );
/// mirror(ours, theirs).await
/// ```
///
/// Each side keeps its own [`Output`](protocol::Protocol::Output): the wrapper
/// hands back the wrapped party's reconciled root, in `A`'s node types. Only
/// what crosses the wire is re-represented.
///
/// The wrapped party's errors must be its backend's own (`P::Error =
/// A::Error`), which is what the materialized stages promise: the wrapper
/// lifts them into the first position of [`Fault`].
pub struct Converted<P, A, B, T> {
    /// The wrapped party, walking its tree in `A`'s node types.
    party: P,
    /// The handle nodes are exploded through on the way out, and reassembled
    /// through on the way in.
    from: A,
    /// The counterparty's handle: the vocabulary on the wire.
    to: B,
    message: PhantomData<fn() -> T>,
}

impl<P, A, B, T> Converted<P, A, B, T> {
    /// Present `party`, whose tree lives in `from`, to a counterparty speaking
    /// `to`.
    pub fn new(party: P, from: A, to: B) -> Self {
        Converted {
            party,
            from,
            to,
            message: PhantomData,
        }
    }
}

/// Re-represent an incoming stream in the wrapped party's node types,
/// [diverting](super::super::divert) conversion faults into `faults`.
///
/// The stream parks on a fault rather than ending: its consumer reads
/// end-of-stream as phase completion, and the fault has already left through
/// the slot.
fn absorbed<A, B, T, M>(
    wire: B,
    local: A,
    requests: impl Requests<M>,
    faults: Sender<Fault<A, B, T>>,
) -> impl Requests<M::Converted>
where
    A: Backend<T, Node<Z>: Leaf<T>>,
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    M: Convertible<B, A, T> + 'static,
{
    stream! {
        let mut requests = pin!(requests);
        while let Some(message) = requests.next().await {
            match message.convert(&wire, &local).await {
                Ok(message) => yield message,
                Err(fault) => {
                    // The fault arrives in the *wire's* producer frame — `B`
                    // exploded, `A` reassembled — and the wrapper's frame puts
                    // its own party first, so the sides swap on the way out.
                    //
                    // First fault wins; a later one finds the slot claimed.
                    let _ = faults.try_send(fault.flip());
                    std::future::pending::<()>().await;
                }
            }
        }
    }
}

/// Re-represent an outgoing stream in the counterparty's node types, racing it
/// against the inbound faults of the same stage.
///
/// A fault ends the stream: the session is over, and the driver's `divert`
/// has the error. The race is what gives an inbound fault an erroring position
/// to surface through.
fn emitted<A, B, T, M>(
    local: A,
    wire: B,
    messages: impl Responses<M, A::Error>,
    mut faults: Receiver<Fault<A, B, T>>,
) -> impl Responses<M::Converted, Fault<A, B, T>>
where
    A: Backend<T, Node<Z>: Leaf<T>>,
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    M: Convertible<A, B, T> + 'static,
{
    stream! {
        let mut messages = pin!(converted(local, wire, messages));
        loop {
            // Both arms are cancel safe: `next` holds no partial item, and a
            // fault the slot already accepted stays in it.
            tokio::select! {
                message = messages.next() => match message {
                    Some(message) => yield message,
                    None => break,
                },
                Some(fault) = faults.recv() => {
                    yield Err(fault);
                    break;
                }
            }
        }
    }
}

impl<P, A, B, T> protocol::Protocol for Converted<P, A, B, T>
where
    P: protocol::Protocol,
    A: Backend<T>,
    B: Backend<T>,
{
    type Height = P::Height;
    type Output = P::Output;
    type Error = Fault<A, B, T>;
}

// The handshake stages carry versions, not nodes, so they cross unconverted;
// only the wrapped party's own errors lift into the first position.

impl<P, A, B, T> protocol::Connect<B, T> for Converted<P, A, B, T>
where
    P: protocol::Connect<A, T> + protocol::Protocol<Error = A::Error> + Send,
    P::Next: protocol::Protocol<Error = A::Error>,
    A: Backend<T, Node<Z>: Leaf<T>>,
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Next = Converted<P::Next, A, B, T>;

    async fn connect(self) -> Result<(message::Handshake, Self::Next), Self::Error> {
        let Converted {
            party, from, to, ..
        } = self;
        let (handshake, party) = party.connect().await.map_err(Error::Client)?;
        Ok((handshake, Converted::new(party, from, to)))
    }
}

impl<P, A, B, T> protocol::CompleteConnect<B, T> for Converted<P, A, B, T>
where
    P: protocol::CompleteConnect<A, T> + protocol::Protocol<Error = A::Error> + Send,
    P::Next: protocol::Protocol<Error = A::Error>,
    A: Backend<T, Node<Z>: Leaf<T>>,
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Next = Converted<P::Next, A, B, T>;

    async fn complete_connect(self, their_version: Version) -> Result<Self::Next, Self::Error> {
        let Converted {
            party, from, to, ..
        } = self;
        let party = party
            .complete_connect(their_version)
            .await
            .map_err(Error::Client)?;
        Ok(Converted::new(party, from, to))
    }
}

impl<P, A, B, T> protocol::Accept<B, T> for Converted<P, A, B, T>
where
    P: protocol::Accept<A, T> + protocol::Protocol<Error = A::Error> + Send,
    P::Next: protocol::Protocol<Error = A::Error>,
    A: Backend<T, Node<Z>: Leaf<T>>,
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Next = Converted<P::Next, A, B, T>;

    async fn accept(
        self,
        request: message::Handshake,
    ) -> Result<(message::Handshake, Self::Next), Self::Error> {
        let Converted {
            party, from, to, ..
        } = self;
        let (handshake, party) = party.accept(request).await.map_err(Error::Client)?;
        Ok((handshake, Converted::new(party, from, to)))
    }
}

// The opening stages: `Initiate` is a bare root hash and `Opening` a listing of
// hashes, so the first node to cross is the initiator's opening reaction.

impl<P, A, B, T> protocol::Initiator<B, T> for Converted<P, A, B, T>
where
    P: protocol::Initiator<A, T> + protocol::Protocol<Error = A::Error>,
    P::Next: protocol::Protocol<Error = A::Error>,
    A: Backend<T, Node<Z>: Leaf<T>>,
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Next = Converted<P::Next, A, B, T>;

    fn initiator(self) -> (impl Responses<message::Initiate, Self::Error>, Self::Next) {
        let Converted {
            party, from, to, ..
        } = self;
        let (initiate, party) = party.initiator();
        (
            initiate.map(|item| item.map_err(Error::Client)),
            Converted::new(party, from, to),
        )
    }
}

impl<P, A, B, T> protocol::Responder<B, T> for Converted<P, A, B, T>
where
    P: protocol::Responder<A, T> + protocol::Protocol<Error = A::Error>,
    P::Next: protocol::Protocol<Error = A::Error>,
    A: Backend<T, Node<Z>: Leaf<T>>,
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Next = Converted<P::Next, A, B, T>;

    fn responder(
        self,
        requests: impl Requests<message::Initiate>,
    ) -> (impl Responses<message::Opening, Self::Error>, Self::Next) {
        let Converted {
            party, from, to, ..
        } = self;
        let (opening, party) = party.responder(requests);
        (
            opening.map(|item| item.map_err(Error::Client)),
            Converted::new(party, from, to),
        )
    }
}

impl<P, A, B, T> protocol::OpenInitiator<B, T> for Converted<P, A, B, T>
where
    P: protocol::OpenInitiator<A, T> + protocol::Protocol<Error = A::Error>,
    P::Next: protocol::Protocol<Error = A::Error>,
    A: Backend<T, Node<Z>: Leaf<T>>,
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    Converted<P::Next, A, B, T>: protocol::Exchange<B, T>
        + protocol::Protocol<Height = UnderUnderRoot, Output = P::Output, Error = Fault<A, B, T>>,
{
    type Next = Converted<P::Next, A, B, T>;

    fn open_initiator(
        self,
        requests: impl Requests<message::Opening>,
    ) -> (
        BoxResponses<message::Exchanged<B, T, UnderRoot>, Self::Error>,
        Self::Next,
    ) {
        let Converted {
            party, from, to, ..
        } = self;
        // Nothing inbound carries a node, so this stage needs no slot.
        let (opening, party) = party.open_initiator(requests);
        let sending = converted(from.clone(), to.clone(), opening);
        (Box::pin(sending), Converted::new(party, from, to))
    }
}

// The descent: every stage converts in both directions.

impl<P, A, B, T> protocol::Exchange<B, T> for Converted<P, A, B, T>
where
    P: protocol::Exchange<A, T> + protocol::Protocol<Error = A::Error>,
    P::Next: protocol::Protocol<Error = A::Error>,
    P::Height: Convert + Pred<Pred: Convert + Pred>,
    A: Backend<T, Node<Z>: Leaf<T>>,
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    // Discharged at each concrete height by one of the three `AfterExchange`
    // blanket impls; cannot be proven generically by the trait solver.
    //
    // The successor's `Protocol` shape is spelled out because the solver will
    // not normalize through the blanket impl while it is still well-formedness
    // checking `Exchange`'s own `Self::Height: Pred`.
    Converted<P::Next, A, B, T>: protocol::AfterExchange<B, T, <<P::Height as Pred>::Pred as Pred>::Pred>
        + protocol::Protocol<
            Height = <<P::Height as Pred>::Pred as Pred>::Pred,
            Output = P::Output,
            Error = Fault<A, B, T>,
        >,
{
    type Next = Converted<P::Next, A, B, T>;

    fn exchange(
        self,
        requests: impl Requests<message::Exchanged<B, T, Self::Height>>,
    ) -> (
        BoxResponses<message::Exchanged<B, T, <Self::Height as Pred>::Pred>, Self::Error>,
        Self::Next,
    ) {
        let Converted {
            party, from, to, ..
        } = self;
        let (faults, raised) = mpsc::channel(1);
        let requests = absorbed(to.clone(), from.clone(), requests, faults);
        let (walk, party) = party.exchange(requests);
        let sending = emitted(from.clone(), to.clone(), walk, raised);
        (Box::pin(sending), Converted::new(party, from, to))
    }
}

impl<P, A, B, T> protocol::CloseInitiator<B, T> for Converted<P, A, B, T>
where
    P: protocol::CloseInitiator<A, T> + protocol::Protocol<Error = A::Error>,
    P::Next: protocol::Protocol<Error = A::Error>,
    A: Backend<T, Node<Z>: Leaf<T>>,
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    Converted<P::Next, A, B, T>: protocol::CompleteInitiator<B, T>
        + protocol::Protocol<Height = Z, Output = P::Output, Error = Fault<A, B, T>>,
{
    type Next = Converted<P::Next, A, B, T>;

    fn close_initiator(
        self,
        requests: impl Requests<message::Exchanged<B, T, S<S<Z>>>>,
    ) -> (
        BoxResponses<(Prefix<S<Z>>, message::Closing<B, T>), Self::Error>,
        Self::Next,
    ) {
        let Converted {
            party, from, to, ..
        } = self;
        let (faults, raised) = mpsc::channel(1);
        let requests = absorbed(to.clone(), from.clone(), requests, faults);
        let (closing, party) = party.close_initiator(requests);
        let sending = emitted(from.clone(), to.clone(), closing, raised);
        (Box::pin(sending), Converted::new(party, from, to))
    }
}

// The two terminals: the responder still has an outgoing stream to surface an
// inbound fault through; the initiator has only its output future.

impl<P, A, B, T> protocol::CompleteResponder<B, T> for Converted<P, A, B, T>
where
    P: protocol::CompleteResponder<A, T> + protocol::Protocol<Error = A::Error>,
    A: Backend<T, Node<Z>: Leaf<T>>,
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    fn complete_responder(
        self,
        requests: impl Requests<(Prefix<S<Z>>, message::Closing<B, T>)>,
    ) -> (
        BoxResponses<(Prefix<Z>, message::Complete<B, T>), Self::Error>,
        impl Future<Output = Result<Self::Output, Self::Error>> + Send,
    ) {
        let Converted {
            party, from, to, ..
        } = self;
        let (faults, raised) = mpsc::channel(1);
        let requests = absorbed(to.clone(), from.clone(), requests, faults);
        let (complete, settled) = party.complete_responder(requests);
        let sending = emitted(from, to, complete, raised);
        (Box::pin(sending), async move {
            settled.await.map_err(Error::Client)
        })
    }
}

impl<P, A, B, T> protocol::CompleteInitiator<B, T> for Converted<P, A, B, T>
where
    P: protocol::CompleteInitiator<A, T> + protocol::Protocol<Error = A::Error> + Send,
    A: Backend<T, Node<Z>: Leaf<T>>,
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    async fn complete_initiator(
        self,
        requests: impl Requests<(Prefix<Z>, message::Complete<B, T>)>,
    ) -> Result<Self::Output, Self::Error> {
        let Converted {
            party, from, to, ..
        } = self;
        let (faults, mut raised) = mpsc::channel(1);
        let requests = absorbed(to, from, requests, faults);

        // No outgoing stream here: the output future is the only erroring
        // position left, so the slot races it directly. A parked incoming
        // stream would otherwise hang the terminal forever.
        tokio::select! {
            absorbed = party.complete_initiator(requests) => absorbed.map_err(Error::Client),
            Some(fault) = raised.recv() => Err(fault),
        }
    }
}
