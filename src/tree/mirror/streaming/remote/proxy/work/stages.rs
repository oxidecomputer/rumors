//! Work-owned translation between typed reply streams and wire frames.
//!
//! The shape deliberately follows the materialized `Work`: every outbound
//! encoder becomes an independently runnable task, and each method returns the
//! receiver-side stream or next-phase scope queue fed by that task. The
//! typestates in [`state`](super::super::state) never hold an internal sender.
//!
//! Like the walk, the decode loop runs on the erased vocabulary — one
//! instantiation per backend and transport — behind thin typed methods
//! that erase the stage's local reply stream on the way in and re-tag the
//! decoded responses at the typed exit (`Work::respond`).
//!
//! Three channels carry the dataflow:
//!
//! - flushed local questions flow into decoding, sized by the session
//!   [`Window`](crate::tree::mirror::streaming::window::Window);
//! - decoded replies park until their stage takes them, with room for every
//!   reply this side can have invited, so decoding never waits on the walk;
//! - at each take, the scopes the reply's questions create flow into the
//!   next phase, before the stage receives the reply.
//!
//! A complete wire reply precedes its local questions. Each edge's capacity
//! rationale lives at its constructor in [`queues`].

use async_stream::try_stream;
use futures::Stream;
use tokio::io::AsyncRead;

use super::{ControlRead, Ingress, Parked, Work, encode, queues};
use crate::{
    link::{Acceptor, Connector},
    tree::{
        mirror::streaming::{
            Backend, Leaf,
            channel::Receiver,
            erased,
            protocol::{BoxResponses, Requests},
            remote::{
                adapter::{LeafEdge, ReplyLevel, Scope, decode_reply, opening_reply},
                proxy::{Error, send_or_cancel},
                streams::{ReceiverFinish, StreamReceiver, StreamSender},
            },
        },
        typed::height::{Height, Root as RootHeight, S, UnderRoot, UnderUnderRoot, Z},
    },
};

/// The decode loop over one session's shared ingress state.
impl<B> Ingress<B>
where
    B: Backend<Node<Z>: Leaf>,
{
    /// Pair each flushed local question with its decoded wire reply at
    /// `level`, and yield the reply with the question's record for parking.
    ///
    /// The decoder reads the record in place, and the record then travels
    /// with its reply, so the take derives the reply's questions from the
    /// very record the decoder validated it against. Every reply crosses one
    /// [`LeafEdge`], which the pump owns for its whole run.
    pub(super) fn decode_pump(
        self,
        level: ReplyLevel,
        mut questions: Receiver<Scope>,
        mut incoming: StreamReceiver,
    ) -> impl Stream<Item = Result<Parked<B::Erased>, Error<B::Error>>> + Send + 'static {
        try_stream! {
            let mut edge = LeafEdge::new();
            while let Some(answered) = questions.recv().await {
                let reply = decode_reply(
                    &mut edge,
                    self.backend.clone(),
                    self.version_bytes,
                    self.ledger.clone(),
                    level,
                    &answered,
                    &mut incoming,
                    self.codec,
                )
                .await?;
                yield Parked { reply, answered };
            }
            reject_extra(&mut incoming).await?;
        }
    }
}

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
    /// listing.
    ///
    /// Nothing is decoded, so nothing parks: like any take, the replayed
    /// opening publishes its one scope before the stage receives it.
    pub fn initiator(&mut self) -> (BoxResponses<B, UnderRoot, Error<B::Error>>, Receiver<Scope>) {
        let (mut next_scopes, scopes) = queues::next_scopes(UnderRoot::HEIGHT);
        let progress = self.progress;
        let listing = std::mem::take(&mut self.remote_listing);
        let responses = async_stream::stream! {
            let (reply, scope) = opening_reply::<B::Erased>(listing);
            progress.taken_reply(RootHeight::HEIGHT, 1);
            progress.next_scope(RootHeight::HEIGHT);
            send_or_cancel(&mut next_scopes, scope).await;
            yield Ok(erased::assume_reply::<B, UnderRoot>(reply));
        };
        (Box::pin(responses), scopes)
    }

    /// Proxy the responder opening and return its lower scope queue.
    pub fn opening_responder(
        &mut self,
        requests: impl Requests<B, UnderRoot>,
        incoming: StreamReceiver,
    ) -> (BoxResponses<B, UnderRoot, Error<B::Error>>, Receiver<Scope>) {
        let requests = requests.erase();
        let (local_questions, questions) = queues::local_questions(
            UnderRoot::HEIGHT,
            self.window.capacity(UnderRoot::HEIGHT),
            &self.stats,
        );
        self.spawn(encode::opening::<B>(
            requests,
            local_questions,
            self.progress,
        ));
        let (next_scopes, scopes) = queues::next_scopes(UnderUnderRoot::HEIGHT);
        let responses =
            self.respond::<UnderRoot>(questions, incoming, Some(next_scopes), self.progress);
        (responses, scopes)
    }

    /// Proxy one ordinary two-height transition and return its lower scopes.
    pub fn internal_replies<C: Connector, H>(
        &mut self,
        requests: impl Requests<B, S<S<H>>>,
        scopes: Receiver<Scope>,
        incoming: StreamReceiver,
        outgoing: StreamSender<C>,
    ) -> (BoxResponses<B, S<H>, Error<B::Error>>, Receiver<Scope>)
    where
        H: Height,
        S<H>: Height,
        S<S<H>>: Height,
    {
        let requests = requests.erase();
        let (local_questions, questions) = queues::local_questions(
            <S<H>>::HEIGHT,
            self.window.capacity(<S<H>>::HEIGHT),
            &self.stats,
        );
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
        let (next_scopes, scopes) = queues::next_scopes(H::HEIGHT);
        let responses = self.respond::<S<H>>(questions, incoming, Some(next_scopes), self.progress);
        (responses, scopes)
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
            queues::local_questions(Z::HEIGHT, self.window.capacity(Z::HEIGHT), &self.stats);
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
        let (next_scopes, scopes) = queues::next_scopes(Z::HEIGHT);
        let responses = self.respond::<Z>(questions, incoming, Some(next_scopes), self.progress);
        (responses, scopes)
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
            queues::local_questions(Z::HEIGHT, self.window.capacity(Z::HEIGHT), &self.stats);
        self.spawn(encode::terminal(
            self.backend(),
            self.budget,
            requests,
            scopes,
            outgoing,
            Some(local_questions),
            self.progress.terminal(),
        ));
        let responses = self.respond::<Z>(questions, incoming, None, self.progress.terminal());
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
