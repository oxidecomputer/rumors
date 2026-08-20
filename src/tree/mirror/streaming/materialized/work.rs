//! Background work accumulated by materialized protocol states.
//!
//! [`Work`] owns every independently runnable pump while the type-level walk
//! advances. [`levels`] contains the phase-specific walks, while [`assembly`]
//! reconstructs their resolved scopes upward. The terminal protocol state
//! drives the accumulated tasks and its final result through one shared
//! fail-fast completion primitive.
//!
//! The walks and pumps run on the erased vocabulary (see
//! [`erased`]): one instantiation
//! per backend. The typed surface is the thin boundary the protocol
//! schedule sees — [`Work::respond`]'s [`BoxResponses`] exit re-tags each
//! outgoing reply at its stage's height, and each walk's public method
//! erases its typed request stream on the way in.

use std::pin::Pin;

use futures::{Stream, StreamExt, future::BoxFuture};

mod answer;
mod assembly;
mod levels;
mod queues;
mod resolver;

#[cfg(test)]
use super::{progress, transcript};
use crate::tree::{
    mirror::streaming::{
        Backend, Leaf, erased,
        materialized::{Error, channel::Sender},
        protocol::BoxResponses,
        stats::Recorder,
        tasks::{complete, park_after_published_error},
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
    backend: B,
    /// Per-edge capacity for the recursive query and resolution queues.
    window: Window,
    /// The session's stats recorder: the walks count disputed scopes,
    /// absorbed supplies, and deletion-honoring drops through clones of it.
    stats: Recorder,
    tasks: Vec<BoxFuture<'static, Result<(), Error<B::Error>>>>,
    #[cfg(test)]
    trace_id: usize,
}

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

    /// Add a task which actively drives a response stream, and return the
    /// stream's typed exit: the one point where a walk's erased replies
    /// re-tag at their stage's height.
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

/// Drive one walk's response stream into its outgoing edge.
///
/// One buffered response is sufficient: whenever the pump blocks, that
/// response is already available to advance the counterparty and release
/// the slot. Buffering a fan would retain whole protocol messages without
/// breaking another dependency.
async fn pump<E: Send, Err: Send + 'static>(
    mut messages: Pin<Box<dyn Stream<Item = Result<erased::Reply<E>, Error<Err>>> + Send>>,
    send: Sender<Result<erased::Reply<E>, Error<Err>>>,
    #[cfg(test)] (work, height): (usize, usize),
) -> Result<(), Error<Err>> {
    while let Some(item) = messages.next().await {
        // Capture the payload-erased wire transcript at the pump:
        // per-stream pull order is exactly the wire order.
        #[cfg(test)]
        if let Ok(reply) = &item {
            transcript::reply(work, height, reply);
        }
        let failed = item.is_err();
        if send.send(item).await.is_err() {
            return Ok(());
        }
        park_after_published_error(failed).await;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
