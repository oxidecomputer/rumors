//! Work-owned translation between typed reply streams and wire frames.
//!
//! The shape deliberately follows the materialized `Work`:
//! every outbound encoder becomes an independently runnable task, and each
//! method returns the receiver-side stream or next-phase scope queue fed by
//! that task. No state outside this module handles an internal sender.
//!
//! Three one-slot channels carry acknowledged local questions into decoding,
//! decoded replies outward through [`Work::respond`], and scopes derived from
//! those replies into the next phase. A complete wire reply precedes its local
//! questions; a decoded reply precedes its dependent scopes.

use async_stream::try_stream;
use futures::StreamExt;
use tokio::io::{AsyncRead, AsyncWrite};

use super::Work;
use crate::tree::{
    mirror::streaming::{
        Backend, Leaf, Node,
        channel::Receiver,
        convert::Convert,
        protocol::{BoxResponses, Requests},
        remote::{
            adapter::{Decoded, Scope, decode_leaf_reply, decode_opening, decode_reply},
            codec::{Frame, Stream},
            proxy::Error,
            session::{FrameSender, Incoming, Outgoing},
        },
    },
    typed::height::{Height, S, UnderRoot, UnderUnderRoot, Z},
};

use super::{encode, queues};

impl<B, T, R, W> Work<B, T, R, W>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: borsh::BorshDeserialize + Send + Sync + 'static,
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    /// Decode the remote initiator's distinguished opening question.
    pub fn initiator(
        &mut self,
        mut incoming: tokio_stream::wrappers::ReceiverStream<Frame<T>>,
    ) -> (
        BoxResponses<B, T, UnderRoot, Error<B::Error>>,
        Receiver<Scope<UnderRoot>>,
    ) {
        let (next_scopes, scopes) = queues::next_scopes::<_, UnderRoot>();
        let progress = self.progress;
        let responses = try_stream! {
            let frame = incoming.next().await.ok_or(Error::MissingOpening)?;
            let (reply, scope) = decode_opening(frame).map_err(Error::OpeningDecode)?;
            yield_reply_scopes!(
                progress, UnderRoot, 1;
                yield reply;
                next_scopes => [scope];
            );
            reject_extra(&mut incoming).await?;
        };
        (self.respond(responses), scopes)
    }

    /// Proxy the responder opening and return its lower scope queue.
    pub fn opening_responder(
        &mut self,
        requests: impl Requests<B, T, UnderRoot>,
        mut incoming: tokio_stream::wrappers::ReceiverStream<Frame<T>>,
        outgoing: FrameSender<T>,
    ) -> (
        BoxResponses<B, T, UnderRoot, Error<B::Error>>,
        Receiver<Scope<UnderUnderRoot>>,
    ) {
        let progress = self.progress;
        let (local_questions, mut questions) = queues::local_questions::<_, UnderRoot>();
        self.spawn(encode::opening(
            requests,
            outgoing,
            local_questions,
            progress,
        ));
        let (next_scopes, scopes) = queues::next_scopes::<_, UnderUnderRoot>();
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
    pub fn internal_replies<H>(
        &mut self,
        requests: impl Requests<B, T, S<S<H>>>,
        scopes: Receiver<Scope<S<S<H>>>>,
        mut incoming: tokio_stream::wrappers::ReceiverStream<Frame<T>>,
        outgoing: FrameSender<T>,
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
        let (local_questions, mut questions) = queues::local_questions::<_, S<H>>();
        self.spawn(encode::replies(
            self.backend(),
            requests,
            scopes,
            outgoing,
            local_questions,
            progress,
        ));
        let (next_scopes, scopes) = queues::next_scopes::<_, H>();
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
    pub fn leaf_replies(
        &mut self,
        requests: impl Requests<B, T, S<Z>>,
        scopes: Receiver<Scope<S<Z>>>,
        mut incoming: tokio_stream::wrappers::ReceiverStream<Frame<T>>,
        outgoing: FrameSender<T>,
    ) -> (BoxResponses<B, T, Z, Error<B::Error>>, Receiver<Scope<Z>>) {
        let progress = self.progress;
        let (local_questions, mut questions) = queues::local_questions::<_, Z>();
        self.spawn(encode::replies(
            self.backend(),
            requests,
            scopes,
            outgoing,
            local_questions,
            progress,
        ));
        let (next_scopes, scopes) = queues::next_scopes::<_, Z>();
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
    pub async fn complete_initiator(
        self,
        requests: impl Requests<B, T, Z>,
        scopes: Receiver<Scope<Z>>,
        outgoing: FrameSender<T>,
    ) -> Result<(R, W), Error<B::Error>> {
        let progress = self.progress;
        let finish = encode::terminal(self.backend(), requests, scopes, outgoing, None, progress);
        let ((), read, write) = self.execute(finish).await?;
        Ok((read, write))
    }

    /// Drive the responder's final bidirectional leaf exchange.
    pub fn complete_responder(
        mut self,
        requests: impl Requests<B, T, Z>,
        scopes: Receiver<Scope<Z>>,
        mut incoming: tokio_stream::wrappers::ReceiverStream<Frame<T>>,
        outgoing: FrameSender<T>,
    ) -> (
        BoxResponses<B, T, Z, Error<B::Error>>,
        impl Future<Output = Result<(R, W), Error<B::Error>>> + Send,
    )
    where
        R: Send,
        W: Send,
    {
        let progress = self.progress;
        let (local_questions, mut questions) = queues::local_questions::<_, Z>();
        self.spawn(encode::terminal(
            self.backend(),
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

    /// Close every unused logical stream and wait for the peer to do likewise.
    pub async fn complete_equal(
        self,
        mut incoming: Incoming<T>,
        mut outgoing: Outgoing<T>,
    ) -> Result<(R, W), Error<B::Error>>
    where
        R: Send,
        W: Send,
    {
        let finish = async move {
            for index in 0..Stream::COUNT {
                let stream = Stream::new(index).expect("a session index names a logical stream");
                outgoing.take(stream).finish().await?;
            }
            for index in 0..Stream::COUNT {
                let stream = Stream::new(index).expect("a session index names a logical stream");
                let mut frames = incoming.take(stream);
                reject_extra(&mut frames).await?;
            }
            Ok(())
        };
        let ((), read, write) = self.execute(finish).await?;
        Ok((read, write))
    }
}

/// Require a completed incoming logical stream after all expected replies.
async fn reject_extra<T, E>(
    incoming: &mut tokio_stream::wrappers::ReceiverStream<Frame<T>>,
) -> Result<(), Error<E>> {
    if incoming.next().await.is_some() {
        Err(Error::UnaskedReply)
    } else {
        Ok(())
    }
}
