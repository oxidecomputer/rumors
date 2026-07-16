//! Background work accumulated by the remote protocol states.
//!
//! Like the materialized implementation's work context, this stores every
//! independently runnable pump as the type-level schedule advances. The final
//! protocol operation drives all stored pumps, the final operation, and both
//! physical transport directions concurrently.

use std::pin::pin;

use futures::{StreamExt, future::BoxFuture};
use tokio::io::{AsyncRead, AsyncWrite};

use crate::tree::{
    mirror::streaming::{
        Backend, Leaf,
        protocol::{BoxResponses, Responses},
        remote::{
            proxy::{Error, send_or_cancel},
            session::{DriveError, Drivers},
        },
        tasks::{complete, park_after_published_error},
    },
    typed::height::{Height, Z},
};

use self::progress::Progress;

mod encode;
pub(super) mod progress;
mod pump;
mod queues;

/// Deferred reply pumps and the physical session which drives them.
pub struct Work<B, T, R, W>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    backend: B,
    drivers: Drivers<R, W, T>,
    tasks: Vec<BoxFuture<'static, Result<(), Error<B::Error>>>>,
    progress: Progress,
}

impl<B, T, R, W> Work<B, T, R, W>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: borsh::BorshDeserialize + Send + Sync + 'static,
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    /// Begin accumulating work around an elected physical session.
    pub fn new(backend: B, drivers: Drivers<R, W, T>) -> Self {
        Self {
            backend,
            drivers,
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

    /// Drive all accumulated pumps, the terminal operation, and the transport.
    async fn execute<O>(
        self,
        finish: impl Future<Output = Result<O, Error<B::Error>>> + Send,
    ) -> Result<(O, R, W), Error<B::Error>> {
        let Self { drivers, tasks, .. } = self;
        let protocol = Box::pin(complete(tasks, finish));
        drivers.run(protocol).await.map_err(|error| match error {
            DriveError::Protocol(error) => error,
            DriveError::Incoming(error) => Error::Incoming(error),
            DriveError::Outgoing(error) => Error::Outgoing(error),
        })
    }
}

#[cfg(test)]
mod tests;
