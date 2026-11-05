//! Background work accumulated by the remote protocol states.
//!
//! [`Work`] owns the translation tasks created as the typestate advances,
//! and the session's pipe. [`stages`] wires each transition, [`encode`]
//! writes local replies, and [`queues`] owns the internal channel
//! capacities. The terminal operation drives these tasks together with the
//! pipe's multiplexer and demultiplexer.

use crate::tree::mirror::streaming::ledger::{Charge, Ledger};
use crate::tree::mirror::streaming::message::{Grandchildren, Listed};
use std::pin::pin;
use std::sync::Arc;
use std::task::Poll;

use futures::future::BoxFuture;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::{
    message::PayloadCodec,
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
                adapter::{ReplyLevel, Scope, derive_questions},
                codec::{Frame, RunBudget, Speaker, Stream as LogicalStream},
                pipe::{self, Demultiplexer, Multiplexer, Outgoing, PipeError, PipeRead},
                proxy::{Error, send_or_cancel},
            },
            stats::Recorder,
            tasks::complete,
            window::Window,
        },
        typed::height::{Height, Z},
    },
};

use self::progress::Progress;

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
    ) -> Self {
        Self {
            backend,
            version_bytes,
            ledger: SupplyLedger::new(set_len),
            root_grandchildren,
            codec,
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
    /// The session window, which sizes the proxy's question queues;
    /// parking holds whatever arrives and the scope queues a fan plus one,
    /// whatever the window.
    window: Window,
    /// Byte budget for each outgoing supply run, also enforced on every
    /// incoming one.
    budget: RunBudget,
    /// The remote greeting's root-fan listing, which
    /// [`initiator`](Self::initiator) replays as the remote's opening
    /// question when the remote initiates.
    remote_listing: Vec<Listed>,
    /// The endpoint's ledger, which the records of the questions this side
    /// sends are charged to.
    ledger: Ledger,
    /// The pipe and the tasks that connect logical streams to it.
    physical: Physical<R, W>,
    /// Tasks driven concurrently when the protocol reaches its terminal step.
    tasks: Vec<BoxFuture<'static, Result<(), Error<B::Error>>>>,
    /// The session's stats recorder: the pipe's frame counts, and the
    /// window stalls of the proxy's window-sized question queues.
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
    // Each argument is one part of the elected session's state, handed over
    // once; bundling them would only rename the arity.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        ingress: Ingress<B>,
        window: Window,
        budget: RunBudget,
        remote_listing: Vec<Listed>,
        ledger: Ledger,
        physical: Physical<R, W>,
        stats: Recorder,
        observe: SessionHandle,
    ) -> Self {
        Self {
            ingress,
            window,
            budget,
            remote_listing,
            ledger,
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
        questions: Receiver<(Scope, Charge)>,
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
        self.spawn(holding(
            Holding::ParkedReplies,
            self.ingress()
                .decode_and_park(level, questions, incoming, parking),
        ));
        Box::pin(holding_stream(
            Holding::QuestionRecords,
            async_stream::stream! {
                let mut taking = None;
                while let Some(Parked { chunk, opens }) = parked.recv().await {
                    let opening = opens.is_some();
                    if let Some((answered, record)) = opens {
                        taking = Some(Taking { answered, taken: 0, _record: record });
                    }
                    let Some(open) = &mut taking else {
                        unreachable!("the pump parks the record with each reply's first chunk");
                    };
                    let mut scopes = 0;
                    if let Some(next_scopes) = &mut publish {
                        let questions = match derive_questions(
                            level,
                            &open.answered,
                            &mut open.taken,
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

    /// Drive the protocol and the pipe together.
    ///
    /// The protocol, which here includes the multiplexer writing every
    /// local frame, is polled first. The demultiplexer reports only
    /// failures; it runs until the protocol completes and is then dropped
    /// between frames, its lookahead returned with the read half for the
    /// closing exchange.
    ///
    /// Once the protocol completes, one last poll of the demultiplexer
    /// collects a failure it had ready. The peer's departure there does not
    /// fail the completed reconciliation: the closing exchange reports it
    /// in its own phase, and a read error stays in the returned read half
    /// for that exchange's first read. Any other failure, or a frame the demultiplexer had
    /// begun but not delivered, is a frame no question invited, so it fails
    /// the session. When the protocol fails because a write failed, a
    /// violation the demultiplexer had ready explains the failure better,
    /// since a peer that detected a problem may have closed the
    /// connection.
    async fn execute<O>(
        self,
        finish: impl Future<Output = Result<O, Error<B::Error>>> + Send,
    ) -> Result<(O, PipeRead<R>, W), Error<B::Error>>
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
    {
        let Self {
            ingress: _,
            window: _,
            budget,
            remote_listing: _,
            ledger: _,
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
            demultiplexer,
        } = physical;
        let mut read = PipeRead::new(read);
        let mut departure = None;
        let outcome = {
            let send = holding(
                Holding::Wire,
                multiplexer.run(&mut write, remote.other(), stats.clone(), &observe),
            );
            let protocol = futures::future::try_join(complete(tasks, finish), async {
                send.await.map_err(|error| Error::Pipe(error.into()))
            });
            let mut protocol = pin!(protocol);
            let mut receive = pin!(holding(
                Holding::Wire,
                demultiplexer.run(&mut read, remote, budget, stats, &observe),
            ));
            let outcome = tokio::select! {
                biased;
                output = &mut protocol => output.map(|(output, ())| output),
                error = &mut receive => Err(Error::Pipe(error)),
            };
            // One last poll collects a failure the demultiplexer had ready;
            // it never waits.
            match outcome {
                Ok(output) => match futures::poll!(receive.as_mut()) {
                    Poll::Ready(PipeError::Departed(source)) => {
                        departure = Some(source);
                        Ok(output)
                    }
                    Poll::Pending => Ok(output),
                    Poll::Ready(error) => Err(Error::Pipe(error)),
                },
                Err(error @ Error::Pipe(PipeError::Encode(_))) => {
                    match futures::poll!(receive.as_mut()) {
                        Poll::Ready(violation) if violation.is_violation() => {
                            Err(Error::Pipe(violation))
                        }
                        Poll::Ready(_) | Poll::Pending => Err(error),
                    }
                }
                Err(error) => Err(error),
            }
        };
        let output = outcome?;
        if let Some(source) = departure {
            read.keep_failure(source);
        }
        pipe::check_drained(&read, remote).map_err(Error::Pipe)?;
        Ok((output, read, write))
    }
}

/// Find the logical stream assigned to one speaker and reply height.
fn stream_at<H: Height>(speaker: Speaker) -> LogicalStream {
    LogicalStream::at_height(speaker, H::HEIGHT)
        .expect("the typestate binds each speaker only at heights on its parity")
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

/// The reply the take is part way through: the record it answers, the
/// record's positions its chunks have taken, and the record's charge,
/// which returns once the take has read the reply's last chunk.
struct Taking {
    /// The record of the question the reply answers.
    answered: Arc<Scope>,
    /// The positions the reply's taken chunks took.
    taken: usize,
    /// The record's charge.
    _record: Charge,
}

#[cfg(test)]
mod tests;
