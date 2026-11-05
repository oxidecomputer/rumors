//! Background work accumulated by materialized protocol states.
//!
//! [`Work`] collects the pumps created by each protocol phase. [`levels`]
//! drives the phase-specific walks, [`answer`] compares child listings,
//! [`resolver`] pairs replies with outstanding queries, [`assembly`]
//! reconstructs resolved scopes, and [`queues`] owns channel capacities. The
//! terminal state drives every pump and its final result together. A pump
//! failure cancels the remaining work without waiting for a response consumer.
//!
//! Walks use [`erased`] values so their code is compiled once per backend.
//! Their entry methods erase typed requests; [`Work::respond`] assigns the
//! phase's height to outgoing replies for the typed protocol schedule.

use std::pin::{Pin, pin};

use futures::{Stream, StreamExt, future::BoxFuture};

mod answer;
mod assembly;
mod charges;
mod levels;
mod queues;
mod resolver;

pub(crate) use charges::parent_resolution;
pub(super) use resolver::Resolver;

/// One leaf request in its queue: the leaf's path, with the charge for the
/// request and the room its reply sets aside.
pub(crate) type LeafRequest = (Prefix<Z>, Charge);

use super::progress::Progress;
use crate::tree::{
    mirror::streaming::{
        Backend, Leaf,
        channel::Sender,
        erased,
        holding::{Holding, holding_stream},
        ledger::{Charge, Ledger},
        materialized::Error,
        protocol::BoxResponses,
        stats::Recorder,
        tasks::complete,
        window::Window,
    },
    typed::{
        Prefix,
        height::{Height, Z},
    },
};

use self::queues::outgoing_responses;

/// Backend and independently runnable tasks retained across protocol phases.
pub struct Work<B>
where
    B: Backend<Node<Z>: Leaf>,
{
    /// The store used to read and reconstruct nodes.
    backend: B,
    /// Per-edge capacity for the recursive query and resolution queues.
    window: Window,
    /// Shared session statistics updated by the walks.
    stats: Recorder,
    /// Pumps driven together when the protocol reaches its terminal step.
    tasks: Vec<BoxFuture<'static, Result<(), Error<B::Error>>>>,
    /// Records progress-critical publications for tests.
    progress: Progress,
    /// The endpoint's ledger, which every item the walk builds is charged
    /// to.
    ledger: Ledger,
}

/// Accumulate phase work and drive its pumps to completion.
impl<B> Work<B>
where
    B: Backend<Node<Z>: Leaf>,
{
    /// Construct a new work context with the session's pipeline window and
    /// stats recorder.
    pub fn new(backend: B, window: Window, stats: Recorder, ledger: Ledger) -> Self {
        let progress = Progress::new();
        Self {
            backend,
            window,
            stats,
            tasks: Vec::new(),
            progress,
            ledger,
        }
    }

    /// Clone the endpoint's ledger for one independently driven task.
    fn ledger(&self) -> Ledger {
        self.ledger.clone()
    }

    /// Clone the backend for one independently driven task.
    fn backend(&self) -> B {
        self.backend.clone()
    }

    /// Clone the stats recorder for one independently driven task.
    pub(super) fn stats(&self) -> Recorder {
        self.stats.clone()
    }

    /// Drive a response stream independently and expose its replies at height `H`.
    ///
    /// The stream is a stage's walk, whose allocations outside the places it
    /// marks itself are its resolutions' for the session-budget meter
    /// ([`holding`](crate::tree::mirror::streaming::holding)).
    fn respond<H: Height>(
        &mut self,
        messages: impl Stream<Item = Result<erased::Reply<B::Erased>, Error<B::Error>>> + Send + 'static,
    ) -> BoxResponses<B, H, Error<B::Error>> {
        let (send, responses) = outgoing_responses::<B, H>();
        self.tasks.push(Box::pin(pump(
            Box::pin(holding_stream(Holding::Resolutions, messages)),
            send,
            self.progress,
            H::HEIGHT,
        )));
        Box::pin(responses)
    }

    /// Forward a stream of nodes into an upward return channel.
    fn return_into(
        &mut self,
        mut returns: Sender<Option<B::Erased>>,
        stream: impl Stream<Item = Result<Option<B::Erased>, Error<B::Error>>> + Send + 'static,
    ) {
        self.tasks.push(Box::pin(async move {
            let mut stream = pin!(holding_stream(Holding::Assembly, stream));
            while let Some(item) = stream.next().await {
                if returns.send(item?).await.is_err() {
                    return Ok(());
                }
            }
            Ok(())
        }));
    }

    /// Drive every registered task and the terminal output to completion.
    pub async fn execute<O>(
        self,
        finish: BoxFuture<'static, Result<O, Error<B::Error>>>,
    ) -> Result<O, Error<B::Error>> {
        complete(self.tasks, finish).await
    }
}

/// Forward one walk's replies to the response queue.
///
/// Queue capacity and its progress argument live at
/// [`queues::outgoing_responses`].
///
/// Errors return directly to the work executor. Sending them through the reply
/// queue would require a consumer that may already have stopped reading.
async fn pump<E: Send, Err: Send + 'static>(
    mut messages: Pin<Box<dyn Stream<Item = Result<erased::Reply<E>, Error<Err>>> + Send>>,
    mut send: Sender<Result<erased::Reply<E>, Error<Err>>>,
    progress: Progress,
    height: usize,
) -> Result<(), Error<Err>> {
    while let Some(reply) = messages.next().await {
        let reply = reply?;
        progress.reply(height, &reply);
        if send.send(Ok(reply)).await.is_err() {
            return Ok(());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
