//! Outbound protocol replies rendered as complete wire replies.
//!
//! Questions are retained until every frame of their containing reply has
//! flushed. Publishing them any earlier could block the encoder before the
//! reply end reaches the remote peer which must answer them.
//!
//! Every encoder here consumes the erased vocabulary. The [`Work`](super::Work)
//! method that spawns it erases and boxes its typed request stream, so each
//! encoder body instantiates once per backend. The progress trace receives the
//! stage height as a runtime label.

use std::pin::Pin;

use futures::{Stream, StreamExt};

use crate::link::Connector;
use crate::tree::{
    mirror::streaming::{
        Backend, Leaf,
        channel::{Receiver, Sender},
        protocol::BoxRequests,
        remote::{
            adapter::{self, Encoded, Scope, encode_reply, opening_listing},
            codec::{ReplyFrame, RunBudget},
            proxy::{Error, send_or_cancel},
            streams::StreamSender,
        },
    },
    typed::height::{Height, UnderRoot, Z},
};

use super::progress::Progress;

/// Encode local leaf replies, optionally publishing the leaf questions they ask.
pub async fn terminal<B, C>(
    backend: B,
    budget: RunBudget,
    mut requests: BoxRequests<B::Erased>,
    mut scopes: Receiver<Scope>,
    mut outgoing: StreamSender<C>,
    mut questions: Option<Sender<Scope>>,
    progress: Progress,
) -> Result<(), Error<B::Error>>
where
    B: Backend<Node<Z>: Leaf>,
    C: Connector,
{
    // Scope-first pairing: dequeuing the scope before awaiting the local
    // reply frees its channel slot one reply earlier, so a K-slot edge
    // admits K truly in-flight scopes (the walk's stage loops make the
    // same choice; see the materialized module docs).
    while let Some(scope) = scopes.recv().await {
        let request = requests.next().await.ok_or(Error::UnansweredRemoteQuery)?;
        let mut encoded = adapter::encode_leaf_reply(backend.clone(), budget, scope, request);
        let batch = write_reply(&mut outgoing, &mut encoded).await?;
        if let Some(questions) = &mut questions {
            // Only replies which can ask questions participate in this trace.
            // The initiator's final answers have no publication dependency.
            progress.wire_reply(Z::HEIGHT, batch.len());
            publish(questions, batch, progress, Z::HEIGHT).await;
        } else if !batch.is_empty() {
            return Err(Error::TerminalQuery);
        }
    }
    finish(requests, outgoing).await
}

/// Encode non-leaf replies and publish each complete question batch.
///
/// `question_height` is the derived questions' height, one under the
/// replies this encoder renders.
// The argument list is the stage's dataflow, one edge per argument;
// bundling edges into a struct would only rename the arity.
#[allow(clippy::too_many_arguments)]
pub async fn replies<B, C>(
    backend: B,
    budget: RunBudget,
    mut requests: BoxRequests<B::Erased>,
    mut scopes: Receiver<Scope>,
    mut outgoing: StreamSender<C>,
    mut questions: Sender<Scope>,
    progress: Progress,
    question_height: usize,
) -> Result<(), Error<B::Error>>
where
    B: Backend<Node<Z>: Leaf>,
    C: Connector,
{
    while let Some(scope) = scopes.recv().await {
        let request = requests.next().await.ok_or(Error::UnansweredRemoteQuery)?;
        let mut encoded = encode_reply(backend.clone(), budget, scope, request);
        let batch = write_reply(&mut outgoing, &mut encoded).await?;
        progress.wire_reply(question_height, batch.len());
        publish(&mut questions, batch, progress, question_height).await;
    }
    finish(requests, outgoing).await
}

/// Consume the local initiator's opening and publish its scope.
///
/// The opening writes nothing: its one query's content, the local root-fan
/// listing, already crossed inside the greeting, which flushed before the
/// descent began. The "wire before internal publication" order is therefore
/// satisfied vacuously, and the question publishes at once.
pub async fn opening<B>(
    mut requests: BoxRequests<B::Erased>,
    mut questions: Sender<Scope>,
    progress: Progress,
) -> Result<(), Error<B::Error>>
where
    B: Backend<Node<Z>: Leaf>,
{
    let request = requests.next().await.ok_or(Error::MissingOpening)?;
    let listing = opening_listing(request).map_err(Error::OpeningEncode)?;
    progress.wire_reply(UnderRoot::HEIGHT, 1);
    progress.local_question(UnderRoot::HEIGHT);
    send_or_cancel(&mut questions, Scope::opening(&listing)).await;
    if requests.next().await.is_some() {
        return Err(Error::ExtraOpening);
    }
    Ok(())
}

/// Flush every frame in one reply and retain its flushed questions.
async fn write_reply<C, E>(
    outgoing: &mut StreamSender<C>,
    encoded: &mut (impl Stream<Item = Result<Encoded, adapter::EncodeError<E>>> + Unpin),
) -> Result<Vec<Scope>, Error<E>>
where
    C: Connector,
{
    let mut batch = Vec::new();
    while let Some(frame) = encoded.next().await {
        if let Some(question) = write_encoded(outgoing, frame?).await? {
            batch.push(question);
        }
    }
    Ok(batch)
}

/// Publish one complete reply's questions in their wire order.
async fn publish(
    questions: &mut Sender<Scope>,
    batch: Vec<Scope>,
    progress: Progress,
    height: usize,
) {
    for question in batch {
        progress.local_question(height);
        send_or_cancel(questions, question).await;
    }
}

/// Reject unclaimed local replies, then close the outgoing logical stream.
async fn finish<C, R, E>(
    mut requests: Pin<Box<R>>,
    outgoing: StreamSender<C>,
) -> Result<(), Error<E>>
where
    C: Connector,
    R: Stream + ?Sized,
{
    if requests.next().await.is_some() {
        return Err(Error::UnaskedLocalReply);
    }
    outgoing.finish().await?;
    Ok(())
}

/// Flush one adapter frame and release its optional question afterward.
async fn write_encoded<C, E>(
    outgoing: &mut StreamSender<C>,
    encoded: Encoded,
) -> Result<Option<Scope>, Error<E>>
where
    C: Connector,
{
    encoded
        .write_with(|frame: ReplyFrame| async { outgoing.frame(frame).await.map_err(Error::Send) })
        .await
}
