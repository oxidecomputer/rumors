//! Background work accumulated by materialized protocol states.
//!
//! [`Work`] owns every independently runnable pump while the type-level walk
//! advances. [`levels`] contains the phase-specific walks, while [`assembly`]
//! reconstructs their resolved scopes upward. The terminal protocol state
//! drives the accumulated tasks and its final result through one shared
//! fail-fast completion primitive.

use std::pin::pin;

use futures::{Stream, future::BoxFuture};
use tokio_stream::StreamExt;

mod answer;
mod assembly;
mod levels;
mod queues;
mod resolver;

#[cfg(test)]
use super::progress;
use crate::tree::{
    mirror::streaming::{
        Backend, Leaf,
        materialized::{Error, channel::Sender},
        protocol::{BoxResponses, Responses},
        tasks::{complete, park_after_published_error},
    },
    typed::height::{Height, Z},
};

use self::queues::outgoing_responses;

/// Backend and independently runnable tasks retained across protocol phases.
pub struct Work<B, T>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    backend: B,
    tasks: Vec<BoxFuture<'static, Result<(), Error<B::Error>>>>,
    #[cfg(test)]
    trace_id: usize,
}

impl<B, T> Work<B, T>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    /// Construct a new work context.
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            tasks: Vec::new(),
            #[cfg(test)]
            trace_id: progress::new_work(),
        }
    }

    /// Clone the backend for one independently driven task.
    fn backend(&self) -> B {
        self.backend.clone()
    }

    /// Add a task which actively drives a response stream.
    ///
    /// One buffered response is sufficient: whenever the pump blocks, that
    /// response is already available to advance the counterparty and release
    /// the slot. Buffering a fan would retain whole protocol messages without
    /// breaking any additional dependency.
    fn respond<H: Height>(
        &mut self,
        messages: impl Responses<B, T, H, Error<B::Error>>,
    ) -> BoxResponses<B, T, H, Error<B::Error>> {
        let (send, responses) = outgoing_responses();
        self.tasks.push(Box::pin(async move {
            let mut messages = pin!(messages);
            while let Some(item) = messages.next().await {
                let failed = item.is_err();
                if send.send(item).await.is_err() {
                    return Ok(());
                }
                park_after_published_error(failed).await;
            }
            Ok::<(), Error<B::Error>>(())
        }));
        responses
    }

    /// Forward a stream of nodes into an upward return channel.
    fn return_into<H: Height>(
        &mut self,
        returns: Sender<Option<B::Node<H>>>,
        stream: impl Stream<Item = Result<Option<B::Node<H>>, Error<B::Error>>> + Send + 'static,
    ) {
        self.tasks.push(Box::pin(async move {
            let mut stream = pin!(stream);
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

#[cfg(test)]
mod tests;
