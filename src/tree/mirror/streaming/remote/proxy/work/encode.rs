//! Outbound protocol replies rendered as complete wire replies.
//!
//! Questions are retained until every frame of their containing reply has
//! flushed. Publishing them any earlier could block the encoder before the
//! reply end reaches the remote peer which must answer them.

use std::pin::pin;

use futures::StreamExt;

use crate::tree::{
    mirror::streaming::{
        Backend, Leaf,
        channel::{Receiver, Sender},
        convert::Convert,
        protocol::Requests,
        remote::{
            adapter::{self, Encoded, Scope, encode_opening, encode_reply},
            proxy::{Error, send_or_cancel},
            session::{FrameSender, ReplyFrame},
        },
    },
    typed::height::{Height, S, UnderRoot, Z},
};

use super::progress::Progress;

/// Encode local leaf replies, optionally publishing the leaf questions they ask.
pub async fn terminal<B, T>(
    backend: B,
    requests: impl Requests<B, T, Z>,
    mut scopes: Receiver<Scope<Z>>,
    mut outgoing: FrameSender<T>,
    questions: Option<Sender<Scope<Z>>>,
    progress: Progress,
) -> Result<(), Error<B::Error>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    let mut requests = pin!(requests);
    while let Some(request) = requests.next().await {
        let scope = scopes.recv().await.ok_or(Error::UnaskedLocalReply)?;
        let mut encoded = adapter::encode_leaf_reply(backend.clone(), scope, request);
        let batch = write_reply(&mut outgoing, &mut encoded).await?;
        progress.wire_reply::<Z>(batch.len());
        if let Some(questions) = &questions {
            publish::<_, Z>(questions, batch, progress).await;
        } else if !batch.is_empty() {
            return Err(Error::TerminalQuery);
        }
    }
    finish(scopes, outgoing).await
}

/// Encode non-leaf replies and publish each complete question batch.
pub async fn replies<B, T, H>(
    backend: B,
    requests: impl Requests<B, T, S<H>>,
    mut scopes: Receiver<Scope<S<H>>>,
    mut outgoing: FrameSender<T>,
    questions: Sender<Scope<H>>,
    progress: Progress,
) -> Result<(), Error<B::Error>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
    S<H>: Convert,
    S<S<H>>: Height,
{
    let mut requests = pin!(requests);
    while let Some(request) = requests.next().await {
        let scope = scopes.recv().await.ok_or(Error::UnaskedLocalReply)?;
        let mut encoded = encode_reply(backend.clone(), scope, request);
        let batch = write_reply(&mut outgoing, &mut encoded).await?;
        progress.wire_reply::<H>(batch.len());
        publish::<_, H>(&questions, batch, progress).await;
    }
    finish(scopes, outgoing).await
}

/// Encode and close the local initiator's distinguished opening stream.
pub async fn opening<B, T>(
    requests: impl Requests<B, T, UnderRoot>,
    mut outgoing: FrameSender<T>,
    questions: Sender<Scope<UnderRoot>>,
    progress: Progress,
) -> Result<(), Error<B::Error>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    let mut requests = pin!(requests);
    let request = requests.next().await.ok_or(Error::MissingOpening)?;
    let encoded = encode_opening(request).map_err(Error::OpeningEncode)?;
    let question = write_encoded(&mut outgoing, encoded).await?;
    progress.wire_reply::<UnderRoot>(usize::from(question.is_some()));
    if let Some(question) = question {
        progress.local_question::<UnderRoot>();
        send_or_cancel(&questions, question).await;
    }
    if requests.next().await.is_some() {
        return Err(Error::ExtraOpening);
    }
    outgoing.finish().await?;
    Ok(())
}

/// Flush every frame in one reply and retain its acknowledged questions.
async fn write_reply<T, Q, E>(
    outgoing: &mut FrameSender<T>,
    encoded: &mut (impl futures::Stream<Item = Result<Encoded<T, Q>, adapter::EncodeError<E>>> + Unpin),
) -> Result<Vec<Q>, Error<E>> {
    let mut batch = Vec::new();
    while let Some(frame) = encoded.next().await {
        if let Some(question) = write_encoded(outgoing, frame?).await? {
            batch.push(question);
        }
    }
    Ok(batch)
}

/// Publish one complete reply's questions in their wire order.
async fn publish<Q, H: Height>(questions: &Sender<Q>, batch: Vec<Q>, progress: Progress) {
    for question in batch {
        progress.local_question::<H>();
        send_or_cancel(questions, question).await;
    }
}

/// Reject unanswered scopes, then close the outgoing logical stream.
async fn finish<T, H, E>(
    mut scopes: Receiver<Scope<H>>,
    outgoing: FrameSender<T>,
) -> Result<(), Error<E>>
where
    H: Height,
    S<H>: Height,
{
    if scopes.recv().await.is_some() {
        return Err(Error::UnansweredRemoteQuery);
    }
    outgoing.finish().await?;
    Ok(())
}

/// Flush one adapter frame and release its optional question afterward.
async fn write_encoded<T, Q, E>(
    outgoing: &mut FrameSender<T>,
    encoded: Encoded<T, Q>,
) -> Result<Option<Q>, Error<E>> {
    encoded
        .write_with(|frame| async {
            let frame = ReplyFrame::try_from(frame).map_err(Error::ReplyFrame)?;
            outgoing.frame(frame).await.map_err(Error::Send)
        })
        .await
}
