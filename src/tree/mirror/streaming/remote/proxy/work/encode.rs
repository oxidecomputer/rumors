//! Outbound protocol replies rendered as wire replies, chunk by chunk.
//!
//! Questions are retained until every frame of their containing reply is in
//! the multiplexer's channel. Publishing them any earlier could block the
//! encoder before the reply's end reaches the remote peer, which must answer
//! them. Publishing does not wait for the multiplexer to write the frames:
//! the decoder that consumes a record waits for it only when a reply to it
//! has arrived, by which time the encoder is past the reply's last frame.
//!
//! Every encoder here consumes the erased vocabulary. The [`Work`](super::Work)
//! method that spawns it erases and boxes its typed request stream, so each
//! encoder body instantiates once per backend. The progress trace receives the
//! stage height as a runtime label.

use std::pin::Pin;

use futures::{Stream, StreamExt};

use crate::tree::{
    mirror::streaming::{
        Backend, Leaf,
        channel::{Receiver, WindowSender},
        holding::{Holding, held_by},
        ledger::{Charge, Ledger, queued, shared},
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

use super::progress::Progress;

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
        let batch = write_reply(&mut outgoing, &mut requests, encoder).await?;
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
        let batch = write_reply(&mut outgoing, &mut requests, encoder).await?;
        progress.wire_reply(question_height, batch.len());
        publish(&mut questions, batch, progress, question_height).await;
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
    loop {
        let chunk = requests.next().await.ok_or(Error::MissingOpening)?;
        reactions.extend(chunk.reactions);
        if chunk.last {
            break;
        }
    }
    let listing = opening_listing(reactions).map_err(Error::OpeningEncode)?;
    progress.wire_reply(UnderRoot::HEIGHT, 1);
    progress.local_question(UnderRoot::HEIGHT);
    questions.publish(Scope::opening(&listing, responder)).await;
    if requests.next().await.is_some() {
        return Err(Error::ExtraOpening);
    }
    Ok(())
}

/// Hand every frame of one reply, its chunks taken from `requests` up to
/// the last, to the multiplexer through `encoder`, and retain the reply's
/// questions.
async fn write_reply<B>(
    outgoing: &mut Outgoing,
    requests: &mut BoxRequests<B::Erased>,
    mut encoder: ReplyEncoder<B>,
) -> Result<Vec<Scope>, Error<B::Error>>
where
    B: Backend<Node<Z>: Leaf>,
{
    let mut batch = Vec::new();
    loop {
        let chunk = requests.next().await.ok_or(Error::UnansweredRemoteQuery)?;
        let last = chunk.last;
        let mut frames = encoder.chunk(chunk);
        while let Some(frame) = frames.next().await {
            if let Some(question) = write_encoded(outgoing, frame?).await? {
                held_by(Holding::QuestionRecords, || batch.push(question));
            }
        }
        if last {
            return Ok(batch);
        }
    }
}

/// Publish one complete reply's questions in their wire order.
async fn publish(questions: &mut Records, batch: Vec<Scope>, progress: Progress, height: usize) {
    for question in batch {
        progress.local_question(height);
        questions.publish(question).await;
    }
}

/// Where an encoder publishes the records of the questions its replies
/// ask, each charged to the endpoint's ledger for what it holds until the
/// take has read it.
pub struct Records {
    /// The decoder's queue of records.
    sender: WindowSender<(Scope, Charge)>,
    /// The endpoint's ledger.
    ledger: Ledger,
}

impl Records {
    /// Publish records through `sender`, charging them to `ledger`.
    pub fn new(sender: WindowSender<(Scope, Charge)>, ledger: Ledger) -> Self {
        Self { sender, ledger }
    }

    /// Publish one question's record, or await cancellation if its consumer
    /// has gone.
    ///
    /// The record waits in the decoder's queue, then in the allocation the
    /// decoder shares with the take until the take reads its reply's last
    /// chunk, so its charge covers the larger of the two.
    async fn publish(&mut self, question: Scope) {
        let record = queued::<(Scope, Charge)>().max(shared::<Scope>());
        let bytes = record + question.children_bytes();
        let charge = self.ledger.charge(Holding::QuestionRecords, bytes);
        if self.sender.send((question, charge)).await.is_err() {
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

/// Hand one adapter frame to the multiplexer and release its optional
/// question afterward.
async fn write_encoded<E>(
    outgoing: &mut Outgoing,
    encoded: Encoded,
) -> Result<Option<Scope>, Error<E>> {
    encoded
        .write_with(|frame: ReplyFrame| async {
            outgoing.frame(frame).await;
            Ok(())
        })
        .await
}
