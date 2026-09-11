//! Background work accumulated by materialized protocol states.
//!
//! [`Work`] collects the pumps created by each protocol phase. [`levels`]
//! processes requests; [`assembly`] reconstructs resolved scopes. The terminal
//! state drives all pumps and its final result together. A pump failure cancels
//! the remaining work without waiting for a response consumer to read it.
//!
//! Walks use [`erased`] values so their code is compiled once per backend.
//! Their entry methods erase typed requests; [`Work::respond`] assigns the
//! phase's height to outgoing replies for the typed protocol schedule.

use std::pin::Pin;

use futures::{Stream, StreamExt, future::BoxFuture};

mod answer;
mod assembly;
mod levels;
mod queues;
mod resolver;

pub(super) use resolver::Resolver;

#[cfg(test)]
use super::{progress, transcript};
use crate::tree::{
    mirror::streaming::{
        Backend, Leaf, erased,
        materialized::{Error, channel::Sender},
        protocol::BoxResponses,
        stats::Recorder,
        tasks::complete,
        window::Window,
    },
    typed::height::{Height, Z},
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
    /// Identifies this walk in test traces.
    #[cfg(test)]
    trace_id: usize,
}

/// Accumulate phase work and drive its pumps to completion.
impl<B> Work<B>
where
    B: Backend<Node<Z>: Leaf>,
{
    /// Construct a new work context with the session's pipeline window and
    /// stats recorder.
    pub fn new(backend: B, window: Window, stats: Recorder) -> Self {
        Self {
            backend,
            window,
            stats,
            tasks: Vec::new(),
            #[cfg(test)]
            trace_id: progress::new_work(),
        }
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
    fn respond<H: Height>(
        &mut self,
        messages: impl Stream<Item = Result<erased::Reply<B::Erased>, Error<B::Error>>> + Send + 'static,
    ) -> BoxResponses<B, H, Error<B::Error>> {
        let (send, responses) = outgoing_responses::<B, H>();
        self.tasks.push(Box::pin(pump(
            Box::pin(messages),
            send,
            #[cfg(test)]
            (self.trace_id, H::HEIGHT),
        )));
        Box::pin(responses)
    }

    /// Forward a stream of nodes into an upward return channel.
    fn return_into(
        &mut self,
        returns: Sender<Option<B::Erased>>,
        stream: impl Stream<Item = Result<Option<B::Erased>, Error<B::Error>>> + Send + 'static,
    ) {
        self.tasks.push(Box::pin(async move {
            let mut stream = std::pin::pin!(stream);
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
/// One buffered response is sufficient: whenever the pump blocks, that
/// response is already available to advance the counterparty and release
/// the slot. Buffering a fan would retain whole protocol messages without
/// breaking another dependency.
///
/// Errors return directly to the work executor. Sending them through the reply
/// queue would require a consumer that may already have stopped reading.
async fn pump<E: Send, Err: Send + 'static>(
    mut messages: Pin<Box<dyn Stream<Item = Result<erased::Reply<E>, Error<Err>>> + Send>>,
    send: Sender<Result<erased::Reply<E>, Error<Err>>>,
    #[cfg(test)] (work, height): (usize, usize),
) -> Result<(), Error<Err>> {
    while let Some(reply) = messages.next().await {
        let reply = reply?;
        // Record produced replies in stream order.
        #[cfg(test)]
        transcript::reply(work, height, &reply);
        if send.send(Ok(reply)).await.is_err() {
            return Ok(());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
