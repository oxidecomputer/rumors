//! Outbound protocol replies rendered as complete wire replies.
//!
//! Questions are retained until every frame of their containing reply has
//! flushed. Publishing them any earlier could block the encoder before the
//! reply end reaches the remote peer which must answer them.

use std::pin::{Pin, pin};

use futures::{Stream, StreamExt};

use crate::link::Connector;
use crate::tree::{
    mirror::streaming::{
        Backend, Leaf,
        channel::{Receiver, Sender},
        convert::Convert,
        message::Reply,
        protocol::Requests,
        remote::{
            adapter::{self, Encoded, Scope, encode_reply, opening_parts},
            codec::RunBudget,
            proxy::{Error, send_or_cancel},
            streams::{ReplyFrame, StreamSender},
        },
    },
    typed::{
        Hash,
        height::{Height, S, UnderRoot, UnderUnderRoot, Z},
    },
};

use super::progress::Progress;

/// Encode local leaf replies, optionally publishing the leaf questions they ask.
pub async fn terminal<B, T, C>(
    backend: B,
    budget: RunBudget,
    requests: impl Requests<B, T, Z>,
    mut scopes: Receiver<Scope<Z>>,
    mut outgoing: StreamSender<C, T>,
    questions: Option<Sender<Scope<Z>>>,
    progress: Progress,
) -> Result<(), Error<B::Error>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    C: Connector,
{
    let mut requests = pin!(requests);
    // Scope-first pairing: dequeuing the scope before awaiting the local
    // reply frees its channel slot one reply earlier, so a K-slot edge
    // admits K truly in-flight scopes (the walk's stage loops make the
    // same choice; see the materialized module docs).
    while let Some(scope) = scopes.recv().await {
        let request = requests.next().await.ok_or(Error::UnansweredRemoteQuery)?;
        let mut encoded = adapter::encode_leaf_reply(backend.clone(), budget, scope, request);
        let batch = write_reply(&mut outgoing, &mut encoded).await?;
        progress.wire_reply::<Z>(batch.len());
        if let Some(questions) = &questions {
            publish::<_, Z>(questions, batch, progress).await;
        } else if !batch.is_empty() {
            return Err(Error::TerminalQuery);
        }
    }
    finish(requests, outgoing).await
}

/// Encode non-leaf replies and publish each complete question batch.
pub async fn replies<B, T, C, H>(
    backend: B,
    budget: RunBudget,
    requests: impl Requests<B, T, S<H>>,
    mut scopes: Receiver<Scope<S<H>>>,
    mut outgoing: StreamSender<C, T>,
    questions: Sender<Scope<H>>,
    progress: Progress,
) -> Result<(), Error<B::Error>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    C: Connector,
    H: Height,
    S<H>: Convert,
    S<S<H>>: Height,
{
    let mut requests = pin!(requests);
    while let Some(scope) = scopes.recv().await {
        let request = requests.next().await.ok_or(Error::UnansweredRemoteQuery)?;
        let mut encoded = encode_reply(backend.clone(), budget, scope, request);
        let batch = write_reply(&mut outgoing, &mut encoded).await?;
        progress.wire_reply::<H>(batch.len());
        publish::<_, H>(&questions, batch, progress).await;
    }
    finish(requests, outgoing).await
}

/// Consume the local initiator's distinguished opening: publish its scope,
/// then write its early supplies.
///
/// The opening *question* writes nothing: its content — the local root-fan
/// listing — already crossed inside the greeting, which flushed before the
/// descent began, so the "wire before internal publication" order is
/// satisfied vacuously and the question publishes immediately, before any
/// supply byte. That order is load-bearing: publishing first keeps the
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
/// never opens, and a session without initiator exclusives keeps today's
/// streamless opening.
pub async fn opening<B, T, C>(
    backend: B,
    budget: RunBudget,
    requests: impl Requests<B, T, UnderRoot>,
    questions: Sender<Scope<UnderRoot>>,
    mut outgoing: StreamSender<C, T>,
    peer_listing: Vec<(u8, Hash)>,
    progress: Progress,
) -> Result<(), Error<B::Error>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    C: Connector,
{
    let mut requests = pin!(requests);
    let request = requests.next().await.ok_or(Error::MissingOpening)?;
    let (listing, supplies) = opening_parts(request).map_err(Error::OpeningEncode)?;
    let question = Scope::opening(&listing);
    progress.wire_reply::<UnderRoot>(1);
    progress.local_question::<UnderRoot>();
    send_or_cancel(&questions, question).await;

    let early = {
        let mut peers = peer_listing.iter().map(|(radix, _)| *radix).peekable();
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
            Reply { replies: supplies },
        );
        let batch: Vec<Scope<UnderUnderRoot>> = write_reply(&mut outgoing, &mut encoded).await?;
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

/// Flush every frame in one reply and retain its acknowledged questions.
async fn write_reply<T, C, Q, E>(
    outgoing: &mut StreamSender<C, T>,
    encoded: &mut (impl futures::Stream<Item = Result<Encoded<T, Q>, adapter::EncodeError<E>>> + Unpin),
) -> Result<Vec<Q>, Error<E>>
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
async fn publish<Q, H: Height>(questions: &Sender<Q>, batch: Vec<Q>, progress: Progress) {
    for question in batch {
        progress.local_question::<H>();
        send_or_cancel(questions, question).await;
    }
}

/// Reject unclaimed local replies, then close the outgoing logical stream.
async fn finish<T, C, R, E>(
    mut requests: Pin<&mut R>,
    outgoing: StreamSender<C, T>,
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
async fn write_encoded<T, C, Q, E>(
    outgoing: &mut StreamSender<C, T>,
    encoded: Encoded<T, Q>,
) -> Result<Option<Q>, Error<E>>
where
    C: Connector,
{
    encoded
        .write_with(|frame| async {
            let frame = ReplyFrame::try_from(frame).map_err(Error::ReplyFrame)?;
            outgoing.frame(frame).await.map_err(Error::Send)
        })
        .await
}
