//! Background work accumulated by the remote protocol states.
//!
//! Like the materialized implementation's work context, this stores every
//! independently runnable pump as the type-level schedule advances. The final
//! protocol operation concurrently drives the stored pumps, its own terminal
//! work, the session's accept driver, and the incoming-stream error route.

use crate::message::PayloadCodec;
use std::pin::{Pin, pin};

use futures::{Stream, StreamExt, future::BoxFuture};

use crate::link::Acceptor;
use crate::tree::{
    mirror::streaming::{
        Backend, Leaf,
        channel::{QueueKind, QueueRole, Sender},
        erased,
        materialized::SupplyLedger,
        protocol::BoxResponses,
        remote::{
            adapter::{DecodeError, EncodeError},
            codec::{Origin, RunBudget, Speaker},
            proxy::{Error, send_or_cancel},
            streams::{AcceptDriver, FirstStreamError, StreamError},
        },
        tasks::{complete, park_after_published_error},
        window::Window,
    },
    typed::{
        Hash,
        height::{Height, Z},
    },
};

use self::progress::Progress;

mod encode;
pub(super) mod progress;
mod pump;
mod queues;

/// Deferred reply pumps and the physical session which drives them.
pub struct Work<B, R, W, A>
where
    B: Backend<Node<Z>: Leaf>,
    A: Acceptor,
{
    backend: B,
    /// Per-edge capacity for the proxy's question and scope queues.
    window: Window,
    /// Byte budget for each outgoing supply run.
    budget: RunBudget,
    /// The remote greeting's `max_version_bytes` declaration, enforced
    /// against every supplied version this session decodes.
    peer_version_bytes: u64,
    /// The remote greeting's `set_len` declaration as a session-total
    /// supply allowance: every leaf record this session decodes charges
    /// it before the payload takes backend custody.
    peer_supplies: SupplyLedger,
    /// The peer's payload codec: the typed ingress every supplied
    /// leaf record decodes through (see [`PayloadCodec`]).
    ///
    /// [`PayloadCodec`]: crate::message::PayloadCodec
    codec: PayloadCodec,
    /// The remote greeting's root-fan listing, consumed by whichever role
    /// the election assigns.
    ///
    /// [`initiator`](Self::initiator) replays it as the remote's opening
    /// question; [`opening_responder`](Self::opening_responder) merges the
    /// local opening's listing against it to decide whether the
    /// early-supply stream opens.
    peer_listing: Vec<(u8, Hash)>,
    physical: Physical<R, W, A>,
    tasks: Vec<BoxFuture<'static, Result<(), Error<B::Error>>>>,
    progress: Progress,
}

/// The session's transport residue: the control halves it must hand back,
/// the accept driver routing incoming streams, and the error route through
/// which those streams report.
pub struct Physical<R, W, A>
where
    A: Acceptor,
{
    pub control_read: R,
    pub control_write: W,
    /// The remote elected speaker: the direction whose failures the
    /// terminal attributes when no single stream can be named.
    pub remote: Speaker,
    pub accept: AcceptDriver<A>,
    pub errors: FirstStreamError,
}

impl<B, R, W, A> Work<B, R, W, A>
where
    B: Backend<Node<Z>: Leaf>,
    A: Acceptor,
{
    /// Begin accumulating work around an elected physical session.
    #[allow(clippy::too_many_arguments)] // The argument list is the session's
    // greeting-derived configuration, one premise per argument.
    pub fn new(
        backend: B,
        window: Window,
        budget: RunBudget,
        peer_version_bytes: u64,
        peer_set_len: u64,
        peer_listing: Vec<(u8, Hash)>,
        physical: Physical<R, W, A>,
        codec: PayloadCodec,
    ) -> Self {
        Self {
            backend,
            window,
            budget,
            peer_version_bytes,
            peer_supplies: SupplyLedger::new(peer_set_len),
            peer_listing,
            physical,
            tasks: Vec::new(),
            progress: Progress::new(),
            codec,
        }
    }

    /// Clone the backend for one independently-driven task.
    fn backend(&self) -> B {
        self.backend.clone()
    }

    /// Add one independently runnable protocol task.
    fn spawn(&mut self, task: impl Future<Output = Result<(), Error<B::Error>>> + Send + 'static) {
        self.tasks.push(Box::pin(task));
    }

    /// Add a task which actively drives a response stream, and return the
    /// stream's typed exit: the one point where the proxy's decoded erased
    /// replies re-tag at their stage's height.
    fn respond<H>(
        &mut self,
        messages: impl Stream<Item = Result<erased::Reply<B::Erased>, Error<B::Error>>> + Send + 'static,
    ) -> BoxResponses<B, H, Error<B::Error>>
    where
        H: Height,
    {
        // One buffered response is sufficient: whenever the pump blocks,
        // that response is already available to advance the counterparty
        // and release the slot. Buffering a fan would retain whole
        // protocol messages without breaking any additional dependency.
        let (send, responses) = erased::reply_channel::<B, H, Error<B::Error>>(
            QueueRole::new(QueueKind::ProxyResponses, H::HEIGHT),
            1,
        );
        self.spawn(pump(Box::pin(messages), send));
        Box::pin(responses)
    }

    /// Drive all accumulated pumps, the terminal operation, and the session's
    /// stream supply to completion.
    ///
    /// The protocol schedule is assembled synchronously before this point:
    /// nothing — pumps, decode streams, the materialized walk, the accept
    /// driver — is polled until this select. The accept driver therefore
    /// starts with the first pump poll, which is why lazy claiming cannot
    /// strand an earlier level: no claim is ever awaited before the driver
    /// that fills it is running.
    ///
    /// Poll order is deliberate: the protocol is observed first, so a
    /// completed reconciliation wins over an accept-side anomaly discovered
    /// in the same poll, and a protocol fault is reported as the cause it
    /// is. The accept driver and the incoming error route resolve only to
    /// errors, so neither can preempt a completion.
    ///
    /// One refinement on protocol *failure*: a dead stream supply is
    /// reported as the session's cause even when one of its consequences
    /// (a write to a peer that already tore down, a decode of a severed
    /// stream) wins the selection. The accept driver deposits the supply
    /// failure's I/O detail in the same poll that observes it, and this
    /// terminal is the deposit's sole consumer, so a consequence caused by
    /// this process's own cut always finds the deposit already in place.
    /// On a real transport the peer's cut arrives from outside, so the
    /// supply failure and a consequence can become ready in the same wave
    /// with the consequence ahead in the biased order; one final poll of
    /// the accept driver then flushes the ready failure into the slot.
    /// That poll never waits — the driver either deposits and parks or is
    /// pending — so the session still imposes no deadline of its own (the
    /// link contract's liveness posture). The failure is surfaced at the
    /// finest granularity available: the selected error or a queued
    /// [`StreamError::SupplyClosed`] the biased order never received
    /// names the stream that provably needed the supply; the deposit
    /// alone is attributed at direction granularity. Typed backend errors
    /// are exempt from the outranking: the local store failing is
    /// independent of the transport, so a dead supply cannot have caused
    /// it, and it surfaces as itself (the attribution contract on
    /// [`Error`]).
    async fn execute<O>(
        self,
        finish: impl Future<Output = Result<O, Error<B::Error>>> + Send,
    ) -> Result<(O, R, W), Error<B::Error>> {
        let Self {
            physical, tasks, ..
        } = self;
        let Physical {
            control_read,
            control_write,
            remote,
            accept,
            mut errors,
        } = physical;
        let outcome = {
            let mut protocol = pin!(Box::pin(complete(tasks, finish)));
            let mut accept = pin!(accept.run());
            let mut stream_errors = pin!(errors.first());
            let outcome = tokio::select! {
                biased;
                output = &mut protocol => output,
                error = &mut stream_errors => Err(Error::Stream(error)),
                error = &mut accept => Err(Error::Accept(error)),
            };
            match &outcome {
                // A violation resolved the accept arm: the driver is
                // complete and must not be polled again, and a violating
                // driver never deposited (it returns instead of parking).
                Ok(_) | Err(Error::Accept(_)) => {}
                Err(_) => {
                    // Flush a supply failure that became ready in the
                    // selected wave but sat behind the biased order; a
                    // single poll either deposits-and-parks or returns
                    // pending, never waits.
                    let _ = futures::poll!(accept.as_mut());
                }
            }
            outcome
        };
        match outcome {
            Ok(output) => Ok((output, control_read, control_write)),
            // The causal supply failure outranks its own symptoms wherever
            // it landed: the selected report or a queued `SupplyClosed` the
            // biased poll never received (stream granularity), else the
            // deposit still in its slot (direction granularity), else the
            // protocol error really is the cause.
            Err(Error::Stream(StreamError::SupplyClosed { origin, source })) => {
                Err(Error::Stream(StreamError::SupplyClosed {
                    origin,
                    source: source.or_else(|| errors.take_supply_failure()),
                }))
            }
            // A typed backend error is the local store's own failure: the
            // supply cannot have caused it, so it is never outranked — it
            // surfaces from the failing operation itself, as `Error`'s
            // attribution contract promises.
            Err(
                error @ (Error::Encode(EncodeError::Backend(_))
                | Error::Decode(DecodeError::Backend(_))),
            ) => Err(error),
            Err(error) => match errors.queued_supply_closed() {
                Some(supply) => Err(Error::Stream(supply)),
                None => match errors.take_supply_failure() {
                    Some(source) => Err(Error::Stream(StreamError::SupplyClosed {
                        origin: Origin::direction(remote),
                        source: Some(source),
                    })),
                    None => Err(error),
                },
            },
        }
    }
}

/// Drive one decoded response stream into its outgoing relay edge.
async fn pump<E: Send, Err: Send + 'static>(
    mut messages: Pin<Box<dyn Stream<Item = Result<erased::Reply<E>, Error<Err>>> + Send>>,
    send: Sender<Result<erased::Reply<E>, Error<Err>>>,
) -> Result<(), Error<Err>> {
    while let Some(message) = messages.next().await {
        let failed = message.is_err();
        send_or_cancel(&send, message).await;
        park_after_published_error(failed).await;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
