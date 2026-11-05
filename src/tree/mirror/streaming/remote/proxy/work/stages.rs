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
//! - local questions, each published once the frame carrying it is in the
//!   multiplexer's channel, flow into decoding through an unbounded queue;
//! - decoded replies park, chunk by chunk as their reactions decode, until
//!   their stage takes them, each chunk charged its part of the room its
//!   question set aside, and parking never waits, so decoding never waits
//!   on the walk;
//! - at each take, the scopes the chunk's questions create flow into the
//!   next phase, before the stage receives the chunk.
//!
//! Each question's frame reaches the multiplexer before its record
//! publishes. Each edge's capacity rationale lives at its constructor in
//! [`queues`].

use std::sync::Arc;

use futures::StreamExt;
use tokio::io::{AsyncBufRead, AsyncWrite};

use super::{Derived, Ingress, Outstanding, Parked, Work, encode, queues};
use crate::network::NetworkKeys;
use crate::tree::{
    mirror::streaming::{
        Backend, Leaf,
        channel::{Receiver, Sender, TrySendError},
        erased,
        holding::{Holding, held_by, holding},
        ledger::Charge,
        protocol::{BoxResponses, Requests},
        remote::{
            adapter::{LeafEdge, ReplyLevel, decode_reply, opening_reply},
            codec::Frame,
            pipe::PipeRead,
            proxy::{Error, send_or_cancel},
        },
        tasks::{TaskRole, cancelled},
    },
    typed::height::{Height, Root as RootHeight, S, UnderRoot, UnderUnderRoot, Z},
};

/// The decode loop over one session's shared ingress state.
impl<B> Ingress<B>
where
    B: Backend<Node<Z>: Leaf>,
{
    /// Pair each published local question with its wire reply at `level`,
    /// and park the reply's chunks in `parking` as they decode, the first
    /// carrying the question's record.
    ///
    /// The decoder reads the record in place, and the record then travels
    /// with its reply, so the take derives the reply's questions from the
    /// very record the decoder validated it against. Every reply crosses one
    /// [`LeafEdge`], which the pump owns for its whole run.
    ///
    /// The decoder charges each chunk its own part of the room the question
    /// set aside for its reply's chunks, which returns once the chunk's
    /// stage has used it, and returns the rest of the room as soon as the
    /// reply shows it cannot be used ([`decode_reply`]). So the room a
    /// reply parked behind a busy stage holds is what its chunks hold and
    /// what it can still deliver, not what its bound allowed.
    ///
    /// Parking never waits, so decoding never waits on the walk: its queue
    /// holds whatever arrives, all of it within the room the questions it
    /// answers set aside. A closed queue means the stage is gone, and the
    /// pump awaits cancellation as [`send_or_cancel`] does. Decode errors
    /// return to the executor without waiting for the stage.
    pub(super) async fn decode_and_park(
        self,
        level: ReplyLevel,
        mut questions: Receiver<Outstanding>,
        mut incoming: Receiver<Frame>,
        mut parking: Sender<Parked<B::Erased>>,
    ) -> Result<(), Error<B::Error>> {
        let mut asked = false;
        let mut edge = LeafEdge::new();
        while let Some(Outstanding {
            scope,
            record,
            room,
        }) = questions.recv().await
        {
            asked = true;
            // The record's shared allocation is the record's own, charged
            // with it.
            let answered = held_by(Holding::QuestionRecords, || Arc::new(scope));
            let mut opens = Some((answered.clone(), record));
            let mut gone = false;
            let mut deliver = |chunk: erased::Reply<B::Erased>| {
                let parked = Parked {
                    chunk,
                    opens: opens.take(),
                };
                match parking.try_send(parked) {
                    Ok(()) => {}
                    Err(TrySendError::Closed(_)) => gone = true,
                    Err(TrySendError::Full(_)) => {
                        unreachable!("parking holds `UNBOUNDED` chunks, more than a session makes")
                    }
                }
            };
            decode_reply(
                &mut edge,
                self.backend.clone(),
                self.version_bytes,
                self.ledger.clone(),
                level,
                &answered,
                &mut incoming,
                self.codec,
                self.keys.as_deref().map(NetworkKeys::path),
                room,
                &mut deliver,
            )
            .await?;
            if gone {
                cancelled().await;
            }
        }
        reject_extra(asked, &mut incoming).await
    }
}

/// Connect each protocol stage's encoder, decoder, and dependent scope queues.
impl<B, R, W> Work<B, R, W>
where
    B: Backend<Node<Z>: Leaf>,
    R: Send,
    W: Send,
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
    pub fn initiator(
        &mut self,
    ) -> (
        BoxResponses<B, UnderRoot, Error<B::Error>>,
        Receiver<Derived>,
    ) {
        let (mut next_scopes, scopes) = queues::next_scopes(UnderRoot::HEIGHT);
        let progress = self.progress;
        let listing = std::mem::take(&mut self.remote_listing);
        let responses = async_stream::stream! {
            let (reply, scope) = opening_reply::<B::Erased>(listing);
            progress.taken(RootHeight::HEIGHT, true, 1);
            progress.next_scope(RootHeight::HEIGHT);
            // The opening is the peer's question, asked in its greeting, and
            // reserves nothing on this side: its record rides uncharged, as
            // one of the encoders' working buffers.
            let scope = Derived { scope, charge: Charge::default() };
            send_or_cancel(&mut next_scopes, scope).await;
            yield Ok(erased::assume_reply::<B, UnderRoot>(reply));
        };
        (Box::pin(responses), scopes)
    }

    /// Proxy the responder opening and return its lower scope queue.
    pub fn opening_responder(
        &mut self,
        requests: impl Requests<B, UnderRoot>,
    ) -> (
        BoxResponses<B, UnderRoot, Error<B::Error>>,
        Receiver<Derived>,
    ) {
        let requests = requests.erase();
        let (local_questions, questions) = queues::local_questions(UnderRoot::HEIGHT);
        self.spawn(TaskRole::Encoder.paced(holding(
            Holding::Wire,
            encode::opening::<B>(
                requests,
                encode::Records::new(local_questions),
                self.ingress.root_grandchildren,
                self.progress,
            ),
        )));
        let (next_scopes, scopes) = queues::next_scopes(UnderUnderRoot::HEIGHT);
        let responses = self.respond::<UnderRoot>(questions, Some(next_scopes), self.progress);
        (responses, scopes)
    }

    /// Proxy one ordinary two-height transition and return its lower scopes.
    pub fn internal_replies<H>(
        &mut self,
        requests: impl Requests<B, S<S<H>>>,
        scopes: Receiver<Derived>,
    ) -> (BoxResponses<B, S<H>, Error<B::Error>>, Receiver<Derived>)
    where
        H: Height,
        S<H>: Height,
        S<S<H>>: Height,
    {
        let requests = requests.erase();
        let outgoing = self.outgoing::<S<S<H>>>();
        let (local_questions, questions) = queues::local_questions(<S<H>>::HEIGHT);
        self.spawn(TaskRole::Encoder.paced(holding(
            Holding::Wire,
            encode::replies(
                self.backend(),
                self.budget,
                requests,
                scopes,
                outgoing,
                encode::Records::new(local_questions),
                self.progress,
                <S<H>>::HEIGHT,
            ),
        )));
        let (next_scopes, scopes) = queues::next_scopes(H::HEIGHT);
        let responses = self.respond::<S<H>>(questions, Some(next_scopes), self.progress);
        (responses, scopes)
    }

    /// Proxy the leaf-parent transition and return its terminal leaf scopes.
    pub fn leaf_replies(
        &mut self,
        requests: impl Requests<B, S<Z>>,
        scopes: Receiver<Derived>,
    ) -> (BoxResponses<B, Z, Error<B::Error>>, Receiver<Derived>) {
        let requests = requests.erase();
        let outgoing = self.outgoing::<S<Z>>();
        let (local_questions, questions) = queues::local_questions(Z::HEIGHT);
        self.spawn(TaskRole::Encoder.paced(holding(
            Holding::Wire,
            encode::replies(
                self.backend(),
                self.budget,
                requests,
                scopes,
                outgoing,
                encode::Records::new(local_questions),
                self.progress,
                Z::HEIGHT,
            ),
        )));
        let (next_scopes, scopes) = queues::next_scopes(Z::HEIGHT);
        let responses = self.respond::<Z>(questions, Some(next_scopes), self.progress);
        (responses, scopes)
    }

    /// Drive the final local answers for a remote initiator to completion.
    pub async fn complete_initiator(
        mut self,
        requests: impl Requests<B, Z>,
        scopes: Receiver<Derived>,
    ) -> Result<(PipeRead<R>, W), Error<B::Error>>
    where
        R: AsyncBufRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        let requests = requests.erase();
        let outgoing = self.outgoing::<Z>();
        let finish = holding(
            Holding::Wire,
            encode::terminal(
                self.backend(),
                self.budget,
                requests,
                scopes,
                outgoing,
                None,
                self.progress,
            ),
        );
        let ((), read, write) = self.execute(finish).await?;
        Ok((read, write))
    }

    /// Drive the responder's final bidirectional leaf exchange.
    pub fn complete_responder(
        mut self,
        requests: impl Requests<B, Z>,
        scopes: Receiver<Derived>,
    ) -> (
        BoxResponses<B, Z, Error<B::Error>>,
        impl Future<Output = Result<(PipeRead<R>, W), Error<B::Error>>> + Send,
    )
    where
        R: AsyncBufRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        let requests = requests.erase();
        let outgoing = self.outgoing::<Z>();
        let (local_questions, questions) = queues::local_questions(Z::HEIGHT);
        self.spawn(TaskRole::Encoder.paced(holding(
            Holding::Wire,
            encode::terminal(
                self.backend(),
                self.budget,
                requests,
                scopes,
                outgoing,
                Some(encode::Records::new(local_questions)),
                self.progress.terminal(),
            ),
        )));
        let responses = self.respond::<Z>(questions, None, self.progress.terminal());
        let completion = async move {
            let ((), read, write) = self.execute(async { Ok(()) }).await?;
            Ok((read, write))
        };
        (responses, completion)
    }
}

/// Require a finished incoming logical stream after all expected replies.
///
/// A level that asked no question expects nothing on its stream: a
/// conforming peer sends no frame and no end control there, so awaiting one
/// would wait forever. Any frame on it answers nothing, so the decoder
/// closes the stream's channel, which refuses later frames at the
/// demultiplexer, and reports a frame already waiting. A level that asked
/// must find its stream ended with no reply to spare.
async fn reject_extra<E>(asked: bool, incoming: &mut Receiver<Frame>) -> Result<(), Error<E>> {
    if !asked {
        return match incoming.close_and_try_recv() {
            None => Ok(()),
            Some(_) => Err(Error::UnaskedReply),
        };
    }
    match incoming.next().await {
        None => Ok(()),
        Some(_) => Err(Error::UnaskedReply),
    }
}

#[cfg(test)]
mod tests;
