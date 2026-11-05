//! Outbound protocol replies rendered as wire replies, chunk by chunk.
//!
//! Each question's record publishes right after the frame carrying it is in
//! the multiplexer's channel, so the decoder holds the record before the
//! peer can answer, even while the rest of the reply waits for its chunks.
//! Publishing never waits: the record queue is unbounded, so the encoder
//! never stops partway through a reply on the decoder's account. Nor does it
//! wait for the multiplexer to write the frames: the decoder that consumes a
//! record waits for it only when a reply to it has arrived, by which time
//! the encoder is past the frame carrying its question.
//!
//! Every encoder here consumes the erased vocabulary. The [`Work`](super::Work)
//! method that spawns it erases and boxes its typed request stream, so each
//! encoder body instantiates once per backend. The progress trace receives the
//! stage height as a runtime label.

use std::pin::{Pin, pin};

use futures::{Stream, StreamExt};

use crate::tree::{
    mirror::streaming::{
        Backend, Leaf,
        channel::{Receiver, Sender},
        ledger::Charge,
        message::Grandchildren,
        protocol::BoxRequests,
        remote::{
            adapter::{Encoded, ReplyEncoder, Scope, opening_listing},
            codec::{ReplyFrame, RunBudget},
            pipe::Outgoing,
            proxy::Error,
        },
        tasks::cancelled,
    },
    typed::height::{Height, UnderRoot, Z},
};

use super::{Outstanding, progress::Progress};

/// Encode local leaf replies, optionally publishing the leaf questions they ask.
pub async fn terminal<B>(
    backend: B,
    budget: RunBudget,
    mut requests: BoxRequests<B::Erased>,
    mut scopes: Receiver<Scope>,
    mut outgoing: Outgoing,
    mut questions: Option<Records>,
    progress: Progress,
) -> Result<(), Error<B::Error>>
where
    B: Backend<Node<Z>: Leaf>,
{
    // Scope-first pairing: dequeuing the scope before awaiting the local
    // reply frees its channel slot one reply earlier, so a K-slot edge
    // admits K truly in-flight scopes (the walk's stage loops make the
    // same choice; see the materialized module docs).
    while let Some(scope) = scopes.recv().await {
        let encoder = ReplyEncoder::leaf(backend.clone(), budget, scope);
        // Only replies which can ask questions participate in the trace.
        // The initiator's final answers have no publication dependency.
        write_reply(
            &mut outgoing,
            &mut requests,
            encoder,
            questions.as_mut(),
            progress,
            Z::HEIGHT,
        )
        .await?;
    }
    finish(requests, outgoing).await
}

/// Encode non-leaf replies, publishing each question after its frame.
///
/// `question_height` is the derived questions' height, one under the
/// replies this encoder renders.
// The argument list is the stage's dataflow, one edge per argument;
// bundling edges into a struct would only rename the arity.
#[allow(clippy::too_many_arguments)]
pub async fn replies<B>(
    backend: B,
    budget: RunBudget,
    mut requests: BoxRequests<B::Erased>,
    mut scopes: Receiver<Scope>,
    mut outgoing: Outgoing,
    mut questions: Records,
    progress: Progress,
    question_height: usize,
) -> Result<(), Error<B::Error>>
where
    B: Backend<Node<Z>: Leaf>,
{
    while let Some(scope) = scopes.recv().await {
        let encoder = ReplyEncoder::branch(backend.clone(), budget, scope);
        write_reply(
            &mut outgoing,
            &mut requests,
            encoder,
            Some(&mut questions),
            progress,
            question_height,
        )
        .await?;
    }
    finish(requests, outgoing).await
}

/// Consume the local initiator's opening and publish its scope, bounded by
/// the `responder`'s grandchild count of its root.
///
/// The opening writes nothing: its one query's content, the local root-fan
/// listing, already crossed inside the greeting, which flushed before the
/// descent began. The "wire before internal publication" order is therefore
/// satisfied vacuously, and the question publishes at once.
pub async fn opening<B>(
    mut requests: BoxRequests<B::Erased>,
    mut questions: Records,
    responder: Grandchildren,
    progress: Progress,
) -> Result<(), Error<B::Error>>
where
    B: Backend<Node<Z>: Leaf>,
{
    let mut reactions = Vec::new();
    let (mut record, mut room) = (Charge::default(), Charge::default());
    loop {
        let mut chunk = requests.next().await.ok_or(Error::MissingOpening)?;
        if let Some((asked, parking)) = chunk.carried.take_question() {
            record.absorb(asked);
            room.absorb(parking);
        }
        reactions.extend(chunk.reactions);
        if chunk.last {
            break;
        }
    }
    let listing = opening_listing(reactions).map_err(Error::OpeningEncode)?;
    progress.wire_frame(UnderRoot::HEIGHT, 1);
    progress.local_question(UnderRoot::HEIGHT);
    questions
        .publish(Scope::opening(&listing, responder), record, room)
        .await;
    if requests.next().await.is_some() {
        return Err(Error::ExtraOpening);
    }
    Ok(())
}

/// Hand every frame of one reply, its chunks taken from `requests` up to
/// the last, to the multiplexer through `encoder`, publishing each
/// question's record to `questions`, at `height`, right after its frame.
///
/// A reply that asks a question where `questions` is absent, the
/// initiator's final answers, fails the session.
async fn write_reply<B>(
    outgoing: &mut Outgoing,
    requests: &mut BoxRequests<B::Erased>,
    mut encoder: ReplyEncoder<B>,
    mut questions: Option<&mut Records>,
    progress: Progress,
    height: usize,
) -> Result<(), Error<B::Error>>
where
    B: Backend<Node<Z>: Leaf>,
{
    loop {
        let mut chunk = requests.next().await.ok_or(Error::UnansweredRemoteQuery)?;
        let last = chunk.last;
        let mut asked = chunk.carried.take_question();
        let mut frames = pin!(encoder.chunk(chunk));
        while let Some(frame) = frames.next().await {
            let Some(question) = write_encoded(outgoing, frame?).await? else {
                continue;
            };
            let Some(questions) = questions.as_deref_mut() else {
                return Err(Error::TerminalQuery);
            };
            progress.wire_frame(height, 1);
            progress.local_question(height);
            let (record, room) = asked.take().unwrap_or_default();
            questions.publish(question, record, room).await;
        }
        if last {
            return Ok(());
        }
    }
}

/// Where an encoder publishes the records of the questions its replies
/// ask.
pub struct Records {
    /// The decoder's queue of records.
    sender: Sender<Outstanding>,
}

impl Records {
    /// Publish records through `sender`.
    pub fn new(sender: Sender<Outstanding>) -> Self {
        Self { sender }
    }

    /// Publish one question's record, `scope`, with its charge and the
    /// room its reply parks in, or await cancellation if the record's
    /// consumer has gone.
    ///
    /// The walk built `record` and `room` out of the question's grant, and
    /// the chunk carrying the question handed them over.
    async fn publish(&mut self, scope: Scope, record: Charge, room: Charge) {
        let outstanding = Outstanding {
            scope,
            record,
            room,
        };
        if self.sender.send(outstanding).await.is_err() {
            cancelled().await;
        }
    }
}

/// Reject unclaimed local replies, then close the outgoing logical stream.
async fn finish<R, E>(mut requests: Pin<Box<R>>, outgoing: Outgoing) -> Result<(), Error<E>>
where
    R: Stream + ?Sized,
{
    if requests.next().await.is_some() {
        return Err(Error::UnaskedLocalReply);
    }
    outgoing.finish().await;
    Ok(())
}

/// Hand one adapter frame, with the charge it carries, to the multiplexer
/// and release its optional question afterward.
async fn write_encoded<E>(
    outgoing: &mut Outgoing,
    encoded: Encoded,
) -> Result<Option<Scope>, Error<E>> {
    encoded
        .write_with(|frame: ReplyFrame, charge: Charge| async {
            outgoing.frame(frame, charge).await;
            Ok(())
        })
        .await
}
