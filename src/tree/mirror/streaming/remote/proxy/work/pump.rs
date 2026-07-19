//! Work-owned translation between typed reply streams and wire frames.
//!
//! The shape deliberately follows the materialized `Work`: every outbound
//! encoder becomes an independently runnable task, and each method returns the
//! receiver-side stream or next-phase scope queue fed by that task. No state
//! outside this module handles an internal sender.
//!
//! Three one-slot channels carry acknowledged local questions into decoding,
//! decoded replies outward through [`Work::respond`], and scopes derived from
//! those replies into the next phase. A complete wire reply precedes its local
//! questions; a decoded reply precedes its dependent scopes.

use async_stream::try_stream;

use super::Work;
use crate::link::{Acceptor, Connector};
use crate::tree::{
    mirror::streaming::{
        Backend, Leaf,
        channel::Receiver,
        convert::Convert,
        protocol::{BoxResponses, Requests},
        remote::{
            adapter::{Decoded, Scope, decode_leaf_reply, decode_reply, opening_reply},
            proxy::Error,
            streams::{ReceiverFinish, StreamReceiver, StreamSender},
        },
    },
    typed::height::{Height, S, UnderRoot, UnderUnderRoot, Z},
};

use super::{encode, queues};

impl<B, T, R, W, A> Work<B, T, R, W, A>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: borsh::BorshDeserialize + Send + Sync + 'static,
    R: Send,
    W: Send,
    A: Acceptor,
{
    /// Replay the remote initiator's opening question from its greeting.
    ///
    /// The question's content — the remote's root-fan listing — already
    /// crossed inside the greeting, so no wire frame exists at this stage:
    /// the reply and its root scope are synthesized from the retained
    /// listing, and the initiator-direction opening stream is never claimed.
    pub fn initiator(
        &mut self,
    ) -> (
        BoxResponses<B, T, UnderRoot, Error<B::Error>>,
        Receiver<Scope<UnderRoot>>,
    ) {
        let (next_scopes, scopes) = queues::next_scopes::<_, UnderRoot>(self.window.scopes());
        let progress = self.progress;
        let listing = std::mem::take(&mut self.peer_listing);
        let responses = try_stream! {
            let (reply, scope) = opening_reply(listing);
            yield_reply_scopes!(
                progress, UnderRoot, 1;
                yield reply;
                next_scopes => [scope];
            );
        };
        (self.respond(responses), scopes)
    }

    /// Proxy the responder opening and return its lower scope queue.
    pub fn opening_responder(
        &mut self,
        requests: impl Requests<B, T, UnderRoot>,
        mut incoming: StreamReceiver<A::Rx, T>,
    ) -> (
        BoxResponses<B, T, UnderRoot, Error<B::Error>>,
        Receiver<Scope<UnderUnderRoot>>,
    ) {
        let progress = self.progress;
        let (local_questions, mut questions) =
            queues::local_questions::<_, UnderRoot>(self.window.scopes());
        self.spawn(encode::opening(requests, local_questions, progress));
        let (next_scopes, scopes) = queues::next_scopes::<_, UnderUnderRoot>(self.window.scopes());
        let backend = self.backend();
        let responses = try_stream! {
            while let Some(scope) = questions.recv().await {
                let Decoded { reply, questions } =
                    decode_reply::<B, T, UnderUnderRoot, _>(
                        backend.clone(), scope, &mut incoming,
                    ).await?;
                yield_reply_scopes!(
                    progress, UnderUnderRoot, questions.len();
                    yield reply;
                    next_scopes => questions;
                );
            }
            reject_extra(&mut incoming).await?;
        };
        (self.respond(responses), scopes)
    }

    /// Proxy one ordinary two-height transition and return its lower scopes.
    pub fn internal_replies<C: Connector, H>(
        &mut self,
        requests: impl Requests<B, T, S<S<H>>>,
        scopes: Receiver<Scope<S<S<H>>>>,
        mut incoming: StreamReceiver<A::Rx, T>,
        outgoing: StreamSender<C, T>,
    ) -> (
        BoxResponses<B, T, S<H>, Error<B::Error>>,
        Receiver<Scope<H>>,
    )
    where
        H: Height,
        S<H>: Convert,
        S<S<H>>: Convert,
        S<S<S<H>>>: Height,
    {
        let progress = self.progress;
        let (local_questions, mut questions) =
            queues::local_questions::<_, S<H>>(self.window.scopes());
        self.spawn(encode::replies(
            self.backend(),
            self.budget,
            requests,
            scopes,
            outgoing,
            local_questions,
            progress,
        ));
        let (next_scopes, scopes) = queues::next_scopes::<_, H>(self.window.scopes());
        let backend = self.backend();
        let responses = try_stream! {
            while let Some(scope) = questions.recv().await {
                let Decoded { reply, questions } = decode_reply::<B, T, H, _>(
                    backend.clone(), scope, &mut incoming,
                ).await?;
                yield_reply_scopes!(
                    progress, H, questions.len();
                    yield reply;
                    next_scopes => questions;
                );
            }
            reject_extra(&mut incoming).await?;
        };
        (self.respond(responses), scopes)
    }

    /// Proxy the leaf-parent transition and return its terminal leaf scopes.
    pub fn leaf_replies<C: Connector>(
        &mut self,
        requests: impl Requests<B, T, S<Z>>,
        scopes: Receiver<Scope<S<Z>>>,
        mut incoming: StreamReceiver<A::Rx, T>,
        outgoing: StreamSender<C, T>,
    ) -> (BoxResponses<B, T, Z, Error<B::Error>>, Receiver<Scope<Z>>) {
        let progress = self.progress;
        let (local_questions, mut questions) =
            queues::local_questions::<_, Z>(self.window.scopes());
        self.spawn(encode::replies(
            self.backend(),
            self.budget,
            requests,
            scopes,
            outgoing,
            local_questions,
            progress,
        ));
        let (next_scopes, scopes) = queues::next_scopes::<_, Z>(self.window.scopes());
        let backend = self.backend();
        let responses = try_stream! {
            while let Some(scope) = questions.recv().await {
                let Decoded { reply, questions } = decode_leaf_reply(
                    backend.clone(), scope, &mut incoming,
                ).await?;
                yield_reply_scopes!(
                    progress, Z, questions.len();
                    yield reply;
                    next_scopes => questions;
                );
            }
            reject_extra(&mut incoming).await?;
        };
        (self.respond(responses), scopes)
    }

    /// Drive the final local answers for a remote initiator to completion.
    pub async fn complete_initiator<C: Connector>(
        self,
        requests: impl Requests<B, T, Z>,
        scopes: Receiver<Scope<Z>>,
        outgoing: StreamSender<C, T>,
    ) -> Result<(R, W), Error<B::Error>> {
        let progress = self.progress;
        let finish = encode::terminal(
            self.backend(),
            self.budget,
            requests,
            scopes,
            outgoing,
            None,
            progress,
        );
        let ((), read, write) = self.execute(finish).await?;
        Ok((read, write))
    }

    /// Drive the responder's final bidirectional leaf exchange.
    pub fn complete_responder<C: Connector>(
        mut self,
        requests: impl Requests<B, T, Z>,
        scopes: Receiver<Scope<Z>>,
        mut incoming: StreamReceiver<A::Rx, T>,
        outgoing: StreamSender<C, T>,
    ) -> (
        BoxResponses<B, T, Z, Error<B::Error>>,
        impl Future<Output = Result<(R, W), Error<B::Error>>> + Send,
    )
    where
        R: Send,
        W: Send,
        A: Send,
    {
        let progress = self.progress;
        let (local_questions, mut questions) =
            queues::local_questions::<_, Z>(self.window.scopes());
        self.spawn(encode::terminal(
            self.backend(),
            self.budget,
            requests,
            scopes,
            outgoing,
            Some(local_questions),
            progress,
        ));
        let backend = self.backend();
        let responses = try_stream! {
            while let Some(scope) = questions.recv().await {
                let Decoded { reply, questions } = decode_leaf_reply(
                    backend.clone(), scope, &mut incoming,
                ).await?;
                if !questions.is_empty() {
                    Err(Error::TerminalQuery)?;
                }
                progress.decoded_reply::<Z>(0);
                yield reply;
            }
            reject_extra(&mut incoming).await?;
        };
        let responses = self.respond(responses);
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
async fn reject_extra<Rx, T, E>(incoming: &mut StreamReceiver<Rx, T>) -> Result<(), Error<E>>
where
    Rx: tokio::io::AsyncRead + Unpin + Send + 'static,
    T: borsh::BorshDeserialize + Send + Sync + 'static,
{
    match incoming.finish().await {
        ReceiverFinish::Clean => Ok(()),
        ReceiverFinish::ExtraReply => Err(Error::UnaskedReply),
    }
}
