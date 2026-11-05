//! Background work accumulated by the remote protocol states.
//!
//! [`Work`] owns the translation tasks created as the typestate advances,
//! and the session's pipe. [`stages`] wires each transition, [`encode`]
//! writes local replies, and [`queues`] owns the internal channel
//! capacities. The terminal operation drives these tasks together with the
//! pipe's multiplexer and demultiplexer, as members of one task set
//! ([`drive`]).

use crate::tree::mirror::streaming::ledger::Charge;
use crate::tree::mirror::streaming::message::{Grandchildren, Listed};
use std::sync::Arc;

use futures::{TryFutureExt, future::BoxFuture};
use tokio::io::{AsyncBufRead, AsyncWrite};

use crate::{
    message::PayloadCodec,
    network::NetworkKeys,
    observe::SessionHandle,
    tree::{
        mirror::streaming::{
            Backend, Leaf,
            channel::{Receiver, Sender},
            erased,
            holding::{Holding, holding, holding_stream},
            materialized::SupplyLedger,
            protocol::BoxResponses,
            remote::{
                adapter::{Replay, ReplyLevel, Scope, derive_questions},
                codec::{Frame, RunBudget, Speaker, Stream as LogicalStream},
                pipe::{self, Demultiplexer, Multiplexer, Outgoing, PipeRead},
                proxy::{Error, send_or_cancel},
            },
            stats::Recorder,
            tasks::TaskRole,
        },
        typed::height::{Height, Z},
    },
};

use self::progress::Progress;

mod drive;
mod encode;
pub(super) mod progress;
mod queues;
mod stages;

/// Backend and validation state shared by every incoming reply decoder.
#[derive(Clone)]
pub(super) struct Ingress<B>
where
    B: Backend<Node<Z>: Leaf>,
{
    /// The store used to reconstruct supplied nodes.
    backend: B,
    /// The peer's declared maximum encoded version size.
    version_bytes: u64,
    /// The peer's remaining declared supply allowance.
    ledger: SupplyLedger,
    /// The peer's declared grandchild count of its root, which bounds its
    /// reply to the local opening question when it responds.
    root_grandchildren: Grandchildren,
    /// The codec used to decode supplied messages.
    codec: PayloadCodec,
    /// The network's keys, whose path key places supplied leaves; `None`
    /// admits no supply.
    ///
    /// Shared, because the keyed hash states are a few hundred bytes and
    /// this handle moves through every greeting state into the session's
    /// future.
    keys: Option<Arc<NetworkKeys>>,
}

impl<B> Ingress<B>
where
    B: Backend<Node<Z>: Leaf>,
{
    /// Combine the backend with the peer's greeting-derived decode limits.
    pub(super) fn new(
        backend: B,
        version_bytes: u64,
        set_len: u64,
        root_grandchildren: Grandchildren,
        codec: PayloadCodec,
        keys: Option<Arc<NetworkKeys>>,
    ) -> Self {
        Self {
            backend,
            version_bytes,
            ledger: SupplyLedger::new(set_len),
            root_grandchildren,
            codec,
            keys,
        }
    }
}

/// Deferred translation tasks and the physical session which drives them.
pub struct Work<B, R, W>
where
    B: Backend<Node<Z>: Leaf>,
{
    /// Backend and validation state for incoming replies.
    ingress: Ingress<B>,
    /// Byte budget for each outgoing supply run, also enforced on every
    /// incoming one.
    budget: RunBudget,
    /// The remote greeting's root-fan listing, which
    /// [`initiator`](Self::initiator) replays as the remote's opening
    /// question when the remote initiates.
    remote_listing: Vec<Listed>,
    /// The pipe and the tasks that connect logical streams to it.
    physical: Physical<R, W>,
    /// Tasks driven concurrently when the protocol reaches its terminal step.
    tasks: Vec<BoxFuture<'static, Result<(), Error<B::Error>>>>,
    /// The session's stats recorder: the pipe's frame counts.
    stats: Recorder,
    /// The session's observation handle, through which the pipe delivers
    /// each frame to its logical stream's observer.
    observe: SessionHandle,
    /// Records reply and scope publication order in tests.
    progress: Progress,
}

/// The pipe's two halves and the logical streams bound to them.
pub struct Physical<R, W> {
    /// The pipe's read half, read by the demultiplexer during
    /// reconciliation and returned for the closing exchange.
    pub read: R,
    /// The pipe's write half, written by the multiplexer during
    /// reconciliation and returned for the closing exchange.
    pub write: W,
    /// The remote elected speaker, whose logical streams arrive here.
    pub remote: Speaker,
    /// The local logical streams, bound as their encoders are created.
    pub multiplexer: Multiplexer,
    /// The remote logical streams, bound as their decoders are created.
    pub demultiplexer: Demultiplexer,
}

/// Accumulate protocol tasks and coordinate them with the transport.
impl<B, R, W> Work<B, R, W>
where
    B: Backend<Node<Z>: Leaf>,
{
    /// Begin accumulating work around an elected physical session.
    pub fn new(
        ingress: Ingress<B>,
        budget: RunBudget,
        remote_listing: Vec<Listed>,
        physical: Physical<R, W>,
        stats: Recorder,
        observe: SessionHandle,
    ) -> Self {
        Self {
            ingress,
            budget,
            remote_listing,
            physical,
            tasks: Vec::new(),
            stats,
            observe,
            progress: Progress::new(),
        }
    }

    /// Clone the backend for one independently-driven task.
    fn backend(&self) -> B {
        self.ingress.backend.clone()
    }

    /// Clone the state shared by incoming reply decoders.
    fn ingress(&self) -> Ingress<B> {
        self.ingress.clone()
    }

    /// Add one independently runnable protocol task.
    fn spawn(&mut self, task: impl Future<Output = Result<(), Error<B::Error>>> + Send + 'static) {
        self.tasks.push(Box::pin(task));
    }

    /// Bind the remote logical stream carrying replies at height `H`.
    fn incoming<H: Height>(&mut self) -> Receiver<Frame> {
        let remote = self.physical.remote;
        self.physical
            .demultiplexer
            .lane(stream_at::<H>(remote), H::HEIGHT)
    }

    /// Bind the local logical stream carrying replies at height `H`.
    fn outgoing<H: Height>(&mut self) -> Outgoing {
        let local = self.physical.remote.other();
        self.physical
            .multiplexer
            .lane(stream_at::<H>(local), H::HEIGHT)
    }

    /// Decode the remote replies at height `H`, park them, and expose them
    /// to the stage that takes them.
    ///
    /// Each reply is decoded against the record of the local question it
    /// answers, taken from `questions` in order, by an independent task
    /// that parks each chunk of it as its reactions decode
    /// ([`Ingress::decode_and_park`]), so decoding never waits on the walk.
    /// When the stage takes a parked chunk, the returned stream derives the
    /// scopes the chunk's questions create, at the level of height `H`, and
    /// publishes them through `publish`, then hands the chunk over. The
    /// responder's terminal replies ask nothing and publish nothing. The
    /// typed height alone chooses the [`ReplyLevel`], in decoding and in the
    /// take, so the two cannot disagree on it.
    fn respond<H>(
        &mut self,
        questions: Receiver<Outstanding>,
        mut publish: Option<Sender<Scope>>,
        progress: Progress,
    ) -> BoxResponses<B, H, Error<B::Error>>
    where
        H: Height,
    {
        let level = ReplyLevel::at::<H>();
        let incoming = self.incoming::<H>();
        let (parking, mut parked) = queues::parked_replies(H::HEIGHT);
        // The pump decodes into parking, so for the session-budget meter
        // its work is the parked replies' unless it marks otherwise; the
        // take derives question records.
        self.spawn(
            TaskRole::Decoder { height: H::HEIGHT }.paced(holding(
                Holding::ParkedReplies,
                self.ingress()
                    .decode_and_park(level, questions, incoming, parking),
            )),
        );
        Box::pin(holding_stream(
            Holding::QuestionRecords,
            async_stream::stream! {
                let mut taking = None;
                while let Some(Parked { chunk, opens }) = parked.recv().await {
                    let opening = opens.is_some();
                    if let Some((answered, record)) = opens {
                        taking = Some(Taking { answered, replay: Replay::default(), _record: record });
                    }
                    let Some(open) = &mut taking else {
                        unreachable!("the pump parks the record with each reply's first chunk");
                    };
                    let mut scopes = 0;
                    if let Some(next_scopes) = &mut publish {
                        let questions = match derive_questions(
                            level,
                            &open.answered,
                            &mut open.replay,
                            &chunk.reactions,
                        ) {
                            Ok(questions) => questions,
                            Err(error) => {
                                yield Err(Error::TakeDerivation(error));
                                return;
                            }
                        };
                        scopes = questions.len();
                        progress.taken(H::HEIGHT, opening, scopes);
                        for question in questions {
                            progress.next_scope(H::HEIGHT);
                            send_or_cancel(next_scopes, question).await;
                        }
                    } else {
                        progress.taken(H::HEIGHT, opening, scopes);
                    }
                    if chunk.last {
                        taking = None;
                    }
                    yield Ok(erased::assume_reply::<B, H>(chunk));
                }
            },
        ))
    }

    /// Drive the protocol and the pipe together, as members of one task set
    /// ([`drive`]), whose module docs give the rules that turn their
    /// failures into the session's outcome.
    ///
    /// The demultiplexer reports only failures; it runs until the protocol
    /// completes and is then dropped between frames, leaving every byte it
    /// has not consumed in the read half's buffer for the closing exchange.
    /// A departure it reported
    /// does not fail a completed reconciliation: a read error stays in the
    /// returned read half for the closing exchange's first read. A frame
    /// the demultiplexer had begun but not delivered is a frame no question
    /// invited, so it fails the session.
    ///
    /// This boxes `finish` and takes the work apart before building the
    /// future it returns, which therefore holds each part once: an `async
    /// fn` would keep its arguments, the terminal future among them, for
    /// its whole run.
    fn execute<O>(
        self,
        finish: impl Future<Output = Result<O, Error<B::Error>>> + Send,
    ) -> impl Future<Output = Result<(O, PipeRead<R>, W), Error<B::Error>>>
    where
        R: AsyncBufRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        let finish = Box::pin(finish);
        let Self {
            ingress: _,
            budget,
            remote_listing: _,
            physical,
            tasks,
            stats,
            observe,
            progress: _,
        } = self;
        let Physical {
            read,
            mut write,
            remote,
            multiplexer,
            mut demultiplexer,
        } = physical;
        let mut read = PipeRead::new(read);
        async move {
            let send = holding(
                Holding::Wire,
                multiplexer.run(&mut write, remote.other(), stats.clone(), &observe),
            )
            .map_err(|error| Error::Pipe(error.into()));
            let receive = holding(
                Holding::Wire,
                demultiplexer.run(&mut read, remote, budget, stats, &observe),
            );
            let (output, departure) =
                drive::drive(tasks, finish, Box::pin(send), Box::pin(receive)).await?;
            if let Some(source) = departure {
                read.keep_failure(source);
            }
            pipe::check_drained(&read, remote).map_err(Error::Pipe)?;
            Ok((output, read, write))
        }
    }
}

/// Find the logical stream assigned to one speaker and reply height.
fn stream_at<H: Height>(speaker: Speaker) -> LogicalStream {
    LogicalStream::at_height(speaker, H::HEIGHT)
        .expect("the typestate binds each speaker only at heights on its parity")
}

/// A question this side asked, from its encoder's publication until the
/// decoder has decoded its reply.
///
/// The question's chunk carried its record's charge and its reply's room to
/// the encoder ([`Carried::Question`]), which publishes them here with the
/// record once the question's frame is in the multiplexer's channel.
///
/// [`Carried::Question`]: crate::tree::mirror::streaming::message::Carried::Question
pub(crate) struct Outstanding {
    /// The record the question's reply is validated against.
    pub(crate) scope: Scope,
    /// The charge for the record, which returns once the take has read the
    /// reply's last chunk.
    pub(crate) record: Charge,
    /// The room the reply's chunks take while they park: the decoder
    /// charges each chunk its part out of it and returns the rest once the
    /// reply is decoded.
    pub(crate) room: Charge,
}

/// A decoded chunk of a reply, parked until its stage takes it.
pub(crate) struct Parked<E> {
    /// The chunk.
    chunk: erased::Reply<E>,
    /// On a reply's first chunk, the record of the local question the reply
    /// answers, from which the take derives the scopes the reply's own
    /// questions create, and the record's charge.
    opens: Option<(Arc<Scope>, Charge)>,
}

/// The reply the take is part way through: the record it answers, how far
/// its taken chunks replayed against the record, and the record's charge,
/// which returns once the take has read the reply's last chunk.
struct Taking {
    /// The record of the question the reply answers.
    answered: Arc<Scope>,
    /// How far the reply's taken chunks replayed against the record.
    replay: Replay,
    /// The record's charge.
    _record: Charge,
}

#[cfg(test)]
mod tests;
