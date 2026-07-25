//! Background work accumulated by the remote protocol states.
//!
//! Like the materialized implementation's work context, this stores every
//! independently runnable pump as the type-level schedule advances. The final
//! protocol operation concurrently drives the stored pumps, its own terminal
//! work, the session's accept driver, and the incoming-stream error route.

use std::pin::pin;

use futures::{StreamExt, future::BoxFuture};

use crate::link::Acceptor;
use crate::tree::{
    mirror::streaming::{
        Backend, Leaf,
        protocol::{BoxResponses, Responses},
        remote::{
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
pub struct Work<B, T, R, W, A>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
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

impl<B, T, R, W, A> Work<B, T, R, W, A>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: borsh::BorshDeserialize + Send + Sync + 'static,
    A: Acceptor,
{
    /// Begin accumulating work around an elected physical session.
    pub fn new(
        backend: B,
        window: Window,
        budget: RunBudget,
        peer_version_bytes: u64,
        peer_listing: Vec<(u8, Hash)>,
        physical: Physical<R, W, A>,
    ) -> Self {
        Self {
            backend,
            window,
            budget,
            peer_version_bytes,
            peer_listing,
            physical,
            tasks: Vec::new(),
            progress: Progress::new(),
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

    /// Add a task which actively drives a response stream.
    ///
    /// One buffered response is sufficient: whenever the task blocks, that
    /// response is already available to advance the counterparty and release
    /// the slot. Buffering a fan would retain whole protocol messages without
    /// breaking any additional dependency.
    fn respond<H>(
        &mut self,
        messages: impl Responses<B, T, H, Error<B::Error>>,
    ) -> BoxResponses<B, T, H, Error<B::Error>>
    where
        H: Height,
    {
        let (send, receive) = self::queues::responses::<_, H>();
        self.spawn(async move {
            let mut messages = pin!(messages);
            while let Some(message) = messages.next().await {
                let failed = message.is_err();
                send_or_cancel(&send, message).await;
                park_after_published_error(failed).await;
            }
            Ok(())
        });
        #[cfg(test)]
        let responses = Box::pin(receive);
        #[cfg(not(test))]
        let responses = Box::pin(tokio_stream::wrappers::ReceiverStream::new(receive));
        responses
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
    /// One refinement on protocol *failure*: a supply failure the accept
    /// driver deposited was observed in a strictly earlier poll wave, so a
    /// protocol error found beside it is the dead transport's symptom
    /// (writes to a peer that already tore down, decodes of severed
    /// streams). The terminal then reports the supply failure as the
    /// session's cause, at direction granularity, unless a stream that
    /// provably needed the supply already claimed it.
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
            tokio::select! {
                biased;
                output = &mut protocol => output,
                error = &mut stream_errors => Err(Error::Stream(error)),
                error = &mut accept => Err(Error::Accept(error)),
            }
        };
        match outcome {
            Ok(output) => Ok((output, control_read, control_write)),
            Err(error) => match errors.take_supply_failure() {
                Some(source) => Err(Error::Stream(StreamError::SupplyClosed {
                    origin: Origin::direction(remote),
                    source: Some(source),
                })),
                None => Err(error),
            },
        }
    }
}

#[cfg(test)]
mod tests;
