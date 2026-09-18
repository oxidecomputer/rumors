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
        erased::Reply,
        protocol::BoxRequests,
        remote::{
            adapter::{self, Encoded, Scope, encode_reply, opening_parts},
            codec::{ReplyFrame, RunBudget},
            proxy::{Error, send_or_cancel},
            streams::StreamSender,
        },
    },
    typed::{
        Hash,
        height::{Height, UnderRoot, Z},
    },
};

use super::progress::Progress;

/// Encode local leaf replies, optionally publishing the leaf questions they ask.
pub async fn terminal<B, C>(
    backend: B,
    budget: RunBudget,
    mut requests: BoxRequests<B::Erased>,
    mut scopes: Receiver<Scope>,
    mut outgoing: StreamSender<C>,
    questions: Option<Sender<Scope>>,
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
        if let Some(questions) = &questions {
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
    questions: Sender<Scope>,
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
        publish(&questions, batch, progress, question_height).await;
    }
    finish(requests, outgoing).await
}

/// Consume the local initiator's opening: publish its scope, then write its
/// early supplies.
///
/// The opening *question* writes nothing: its content — the local root-fan
/// listing — already crossed inside the greeting, which flushed before the
/// descent began, so the "wire before internal publication" order is
/// satisfied vacuously and the question publishes immediately, before any
/// supply byte. That order is required: publishing first keeps the
/// responder's root reply decodable while the supply bulk is still
/// flushing against link backpressure, so the disputed descent never waits
/// behind it.
///
/// The trailing early supplies — the initiator's exclusive root children —
/// cross as one supplies-only reply on the initiator-direction opening
/// stream. The stream opens exactly when the *early set* (local listing
/// radices absent from the peer's) is nonempty: the set is what the
/// responder recomputes from the same two listings, so it must learn "all
/// pruned away" from an empty reply rather than wait on a stream that
/// never opens. A session without initiator exclusives opens no
/// initiator-direction stream.
pub async fn opening<B, C>(
    backend: B,
    budget: RunBudget,
    mut requests: BoxRequests<B::Erased>,
    questions: Sender<Scope>,
    mut outgoing: StreamSender<C>,
    remote_listing: Vec<(u8, Hash)>,
    progress: Progress,
) -> Result<(), Error<B::Error>>
where
    B: Backend<Node<Z>: Leaf>,
    C: Connector,
{
    let request = requests.next().await.ok_or(Error::MissingOpening)?;
    let (listing, supplies) = opening_parts(request).map_err(Error::OpeningEncode)?;
    let question = Scope::opening(&listing);
    progress.wire_reply(UnderRoot::HEIGHT, 1);
    progress.local_question(UnderRoot::HEIGHT);
    send_or_cancel(&questions, question).await;

    let early = {
        let mut peers = remote_listing.iter().map(|(radix, _)| *radix).peekable();
        listing.iter().any(|(radix, _)| {
            while peers.next_if(|peer| peer < radix).is_some() {}
            peers.peek() != Some(radix)
        })
    };
    if early {
        let mut encoded = encode_reply(
            backend,
            budget,
            Scope::opening(&[]),
            Reply {
                reactions: supplies,
            },
        );
        let batch: Vec<Scope> = write_reply(&mut outgoing, &mut encoded).await?;
        debug_assert!(batch.is_empty(), "opening supplies ask no question");
    } else {
        debug_assert!(
            supplies.is_empty(),
            "an empty early set admits no early supplies"
        );
    }
    outgoing.finish().await.map_err(Error::Send)?;
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
async fn publish(questions: &Sender<Scope>, batch: Vec<Scope>, progress: Progress, height: usize) {
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
