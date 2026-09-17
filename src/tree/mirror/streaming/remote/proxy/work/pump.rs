//! Work-owned translation between typed reply streams and wire frames.
//!
//! The shape deliberately follows the materialized `Work`: every outbound
//! encoder becomes an independently runnable task, and each method returns the
//! receiver-side stream or next-phase scope queue fed by that task. No state
//! outside this module handles an internal sender.
//!
//! Like the walk, the decode loops run on the erased vocabulary — one
//! instantiation per backend and transport — behind thin typed methods
//! that erase the stage's local reply stream on the way in and re-tag the
//! decoded responses at the typed exit (`Work::respond`).
//!
//! Three channels carry the dataflow:
//!
//! - flushed local questions flow into decoding, sized by the session
//!   [`Window`](crate::tree::mirror::streaming::window::Window);
//! - scopes derived from decoded replies flow into the next phase, also
//!   window-sized;
//! - decoded replies flow outward through [`Work::respond`] on a one-slot
//!   edge, an in-order relay that bounds decoded replies in flight at one
//!   per stage.
//!
//! A complete wire reply precedes its local questions; a decoded reply
//! precedes its dependent scopes. Each edge's capacity rationale lives at
//! its constructor in [`queues`].

use async_stream::try_stream;
use futures::Stream;
use tokio::io::AsyncRead;

use super::{ControlRead, Work, encode, opening_supplies::OpeningSupplies, queues};
use crate::{
    link::{Acceptor, Connector},
    tree::{
        mirror::streaming::{
            Backend, Leaf,
            channel::{Receiver, Sender},
            erased::{Reaction, Reply, ops},
            protocol::{BoxResponses, Requests},
            remote::{
                adapter::{
                    DecodeError, Decoded, Scope, decode_leaf_reply, decode_reply, opening_reply,
                },
                proxy::Error,
                streams::{ReceiverFinish, StreamReceiver, StreamSender},
            },
        },
        typed::height::{Height, Root as RootHeight, S, UnderRoot, UnderUnderRoot, Z},
    },
};

/// Connect each protocol stage's encoder, decoder, and dependent scope queues.
impl<B, R, W, A> Work<B, R, W, A>
where
    B: Backend<Node<Z>: Leaf>,
    R: Send,
    W: Send,
    A: Acceptor,
{
    /// Replay the remote initiator's opening question from its greeting.
    ///
    /// The question's content — the remote's root-fan listing — already
    /// crossed inside the greeting, so no wire frame exists at this stage:
    /// the reply and its root scope are synthesized from the retained
    /// listing. The initiator-direction opening stream carries the remote's
    /// early supplies instead; the first descending transition claims and
    /// decodes it against the root-level requests it answers, so the root
    /// merge-join below never waits on the supply bulk.
    pub fn initiator(&mut self) -> (BoxResponses<B, UnderRoot, Error<B::Error>>, Receiver<Scope>) {
        let (next_scopes, scopes) =
            queues::next_scopes(UnderRoot::HEIGHT, self.window.capacity(UnderRoot::HEIGHT));
        let progress = self.progress;
        let listing = std::mem::take(&mut self.peer_listing);
        let responses = try_stream! {
            let (reply, scope) = opening_reply::<B::Erased>(listing);
            yield_reply_scopes!(
                progress, RootHeight::HEIGHT, 1;
                yield reply;
                next_scopes => [scope];
            );
        };
        (self.respond::<UnderRoot>(responses), scopes)
    }

    /// Proxy the responder opening and return its lower scope queue.
    pub fn opening_responder<C: Connector>(
        &mut self,
        requests: impl Requests<B, UnderRoot>,
        incoming: StreamReceiver,
        outgoing: StreamSender<C>,
    ) -> (BoxResponses<B, UnderRoot, Error<B::Error>>, Receiver<Scope>) {
        let requests = requests.erase();
        let (local_questions, questions) =
            queues::local_questions(UnderRoot::HEIGHT, self.window.capacity(UnderRoot::HEIGHT));
        let peer_listing = std::mem::take(&mut self.peer_listing);
        self.spawn(encode::opening(
            self.backend(),
            self.budget,
            requests,
            local_questions,
            outgoing,
            peer_listing,
            self.progress,
        ));
        let (next_scopes, scopes) = queues::next_scopes(
            UnderUnderRoot::HEIGHT,
            self.window.capacity(UnderUnderRoot::HEIGHT),
        );
        let responses = self.decode_pump(
            questions,
            incoming,
            next_scopes,
            None,
            UnderUnderRoot::HEIGHT,
        );
        (self.respond::<UnderRoot>(responses), scopes)
    }

    /// Proxy one ordinary two-height transition and return its lower scopes.
    ///
    /// `opening_supplies` is armed only on the first initiator-representing
    /// transition: there, each of the local responder's root-level requests
    /// pairs its (empty) wire reply with the whole node the remote supplied at
    /// the opening, exploded into the per-child supplies the walk absorbs — the
    /// same reply shape a wire-borne answer would have carried.
    pub fn internal_replies<C: Connector, H>(
        &mut self,
        requests: impl Requests<B, S<S<H>>>,
        scopes: Receiver<Scope>,
        incoming: StreamReceiver,
        outgoing: StreamSender<C>,
        opening_supplies: Option<StreamReceiver>,
    ) -> (BoxResponses<B, S<H>, Error<B::Error>>, Receiver<Scope>)
    where
        H: Height,
        S<H>: Height,
        S<S<H>>: Height,
    {
        let requests = requests.erase();
        let (local_questions, questions) =
            queues::local_questions(<S<H>>::HEIGHT, self.window.capacity(<S<H>>::HEIGHT));
        self.spawn(encode::replies(
            self.backend(),
            self.budget,
            requests,
            scopes,
            outgoing,
            local_questions,
            self.progress,
            <S<H>>::HEIGHT,
        ));
        let (next_scopes, scopes) = queues::next_scopes(H::HEIGHT, self.window.capacity(H::HEIGHT));
        let responses = self.decode_pump(
            questions,
            incoming,
            next_scopes,
            opening_supplies,
            H::HEIGHT,
        );
        (self.respond::<S<H>>(responses), scopes)
    }

    /// The decode loop shared by the responder opening and every internal
    /// transition: pair each flushed local question with its decoded wire
    /// reply, publishing the reply before the lower scopes derived from it.
    ///
    /// `height` is the derived scopes' height. `opening_supplies` arms the
    /// opening-supply pairing (see
    /// [`internal_replies`](Self::internal_replies)); the opening
    /// responder and every deeper stage pass `None`.
    fn decode_pump(
        &mut self,
        mut questions: Receiver<Scope>,
        mut incoming: StreamReceiver,
        next_scopes: Sender<Scope>,
        opening_supplies: Option<StreamReceiver>,
        height: usize,
    ) -> impl Stream<Item = Result<Reply<B::Erased>, Error<B::Error>>> + Send + 'static + use<B, R, W, A>
    {
        let progress = self.progress;
        let backend = self.backend();
        let version_bytes = self.peer_version_bytes;
        let ledger = self.peer_supplies.clone();
        let codec = self.codec;
        try_stream! {
            let mut opening = opening_supplies.map(|receiver| {
                OpeningSupplies::<B>::new(version_bytes, ledger.clone(), receiver, codec)
            });
            while let Some(scope) = questions.recv().await {
                // A root-level request's content crossed at the opening. Its
                // ordinary pairing reply arrives empty; after decoding that
                // reply, supplement it with the opening stream's node, if
                // pruning left one.
                let opening_supply = (opening.is_some() && scope.is_request()).then(|| {
                    let parent = scope.parent();
                    let (root, radix) = parent.pop();
                    (parent, root, radix)
                });
                let Decoded {
                    mut reply,
                    questions,
                } = decode_reply::<B, _>(
                    backend.clone(),
                    version_bytes,
                    ledger.clone(),
                    scope,
                    &mut incoming,
                    codec,
                )
                .await?;
                let opening_node = match (opening.as_mut(), opening_supply) {
                    (Some(opening), Some((parent, root, radix))) => opening
                        .advance_to(&backend, root, radix)
                        .await?
                        .map(|node| (parent, node)),
                    _ => None,
                };
                if let Some((parent, node)) = opening_node {
                    let children = ops::children_of(&backend, parent, node)
                        .await
                        .map_err(|error| Error::Decode(DecodeError::Backend(error)))?;
                    reply.reactions.extend(
                        children
                            .into_iter()
                            .map(|(radix, child)| Reaction::Supply(radix, child)),
                    );
                }
                yield_reply_scopes!(
                    progress, height + 1, questions.len();
                    yield reply;
                    next_scopes => questions;
                );
            }
            if let Some(opening) = &mut opening {
                opening.finish().await?;
            }
            reject_extra(&mut incoming).await?;
        }
    }

    /// Proxy the leaf-parent transition and return its terminal leaf scopes.
    pub fn leaf_replies<C: Connector>(
        &mut self,
        requests: impl Requests<B, S<Z>>,
        scopes: Receiver<Scope>,
        incoming: StreamReceiver,
        outgoing: StreamSender<C>,
    ) -> (BoxResponses<B, Z, Error<B::Error>>, Receiver<Scope>) {
        let requests = requests.erase();
        let (local_questions, questions) =
            queues::local_questions(Z::HEIGHT, self.window.capacity(Z::HEIGHT));
        self.spawn(encode::replies(
            self.backend(),
            self.budget,
            requests,
            scopes,
            outgoing,
            local_questions,
            self.progress,
            Z::HEIGHT,
        ));
        let (next_scopes, scopes) = queues::next_scopes(Z::HEIGHT, self.window.capacity(Z::HEIGHT));
        let responses = self.leaf_decode_pump(questions, incoming, Some(next_scopes));
        (self.respond::<Z>(responses), scopes)
    }

    /// Decode leaf replies, optionally publishing their dependent scopes.
    ///
    /// Ordinary leaf replies pass a scope sender. The responder's terminal
    /// stream passes `None`; the wire grammar excludes query reactions there.
    fn leaf_decode_pump(
        &mut self,
        mut questions: Receiver<Scope>,
        mut incoming: StreamReceiver,
        next_scopes: Option<Sender<Scope>>,
    ) -> impl Stream<Item = Result<Reply<B::Erased>, Error<B::Error>>> + Send + 'static + use<B, R, W, A>
    {
        let progress = self.progress;
        let backend = self.backend();
        let version_bytes = self.peer_version_bytes;
        let ledger = self.peer_supplies.clone();
        let codec = self.codec;
        try_stream! {
            while let Some(scope) = questions.recv().await {
                let Decoded { reply, questions } = decode_leaf_reply(
                    backend.clone(),
                    version_bytes,
                    ledger.clone(),
                    scope,
                    &mut incoming,
                    codec,
                )
                .await?;
                if let Some(next_scopes) = &next_scopes {
                    yield_reply_scopes!(
                        progress, Z::HEIGHT, questions.len();
                        yield reply;
                        next_scopes => questions;
                    );
                } else {
                    progress.decoded_reply(Z::HEIGHT, 0);
                    yield reply;
                }
            }
            reject_extra(&mut incoming).await?;
        }
    }

    /// Drive the final local answers for a remote initiator to completion.
    pub async fn complete_initiator<C: Connector>(
        self,
        requests: impl Requests<B, Z>,
        scopes: Receiver<Scope>,
        outgoing: StreamSender<C>,
    ) -> Result<(ControlRead<R>, W), Error<B::Error>>
    where
        R: AsyncRead + Unpin,
    {
        let requests = requests.erase();
        let finish = encode::terminal(
            self.backend(),
            self.budget,
            requests,
            scopes,
            outgoing,
            None,
            self.progress,
        );
        let ((), read, write) = self.execute(finish).await?;
        Ok((read, write))
    }

    /// Drive the responder's final bidirectional leaf exchange.
    pub fn complete_responder<C: Connector>(
        mut self,
        requests: impl Requests<B, Z>,
        scopes: Receiver<Scope>,
        incoming: StreamReceiver,
        outgoing: StreamSender<C>,
    ) -> (
        BoxResponses<B, Z, Error<B::Error>>,
        impl Future<Output = Result<(ControlRead<R>, W), Error<B::Error>>> + Send,
    )
    where
        R: AsyncRead + Unpin + Send,
        W: Send,
        A: Send,
    {
        let requests = requests.erase();
        let (local_questions, questions) =
            queues::local_questions(Z::HEIGHT, self.window.capacity(Z::HEIGHT));
        self.spawn(encode::terminal(
            self.backend(),
            self.budget,
            requests,
            scopes,
            outgoing,
            Some(local_questions),
            self.progress,
        ));
        let responses = self.leaf_decode_pump(questions, incoming, None);
        let responses = self.respond::<Z>(responses);
        let completion = async move {
            let ((), read, write) = self.execute(async { Ok(()) }).await?;
            Ok((read, write))
        };
        (responses, completion)
    }
}

/// Require a finished incoming logical stream after all expected replies.
///
/// A stream that was never claimed — its level asked no question — is
/// finished vacuously; a claimed stream must have delivered its end control
/// with no reply to spare.
async fn reject_extra<E>(incoming: &mut StreamReceiver) -> Result<(), Error<E>> {
    match incoming.finish().await {
        ReceiverFinish::Clean => Ok(()),
        ReceiverFinish::ExtraReply => Err(Error::UnaskedReply),
    }
}
