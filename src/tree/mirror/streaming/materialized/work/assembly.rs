//! Upward reconstruction of resolved scopes.

use std::pin::pin;

use async_stream::try_stream;
use futures::{Stream, stream};
use tokio_stream::StreamExt;

use super::{Work, queues::assembly_level_returns};
use crate::tree::{
    mirror::streaming::{
        Backend, Leaf,
        materialized::{Error, Resolution, Resolve, channel::Sender},
        tasks::next_or_cancelled,
    },
    typed::height::{Height, S, Z},
};

impl<B, T> Work<B, T>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    /// Assemble one level upward and return its lower-level sender.
    ///
    /// A full fan lets every lower scope enqueue before the parent resolution
    /// containing its [`Resolve::Pending`] slots is published, without relying
    /// on blocked sender futures remaining independently runnable.
    pub fn assemble<H>(
        &mut self,
        returns: Sender<Option<B::Node<S<H>>>>,
        resolutions: impl Stream<Item = Result<Resolution<B, T, H>, Error<B::Error>>> + Send + 'static,
    ) -> Sender<Option<B::Node<H>>>
    where
        H: Height,
        S<H>: Height,
    {
        let (level, level_rx) = assembly_level_returns::<B, T, H>();
        self.return_into(
            returns,
            assemble(self.backend.clone(), resolutions, level_rx),
        );
        level
    }

    /// Assemble leaf resolutions upward with no level beneath them.
    pub fn assemble_leaves(
        &mut self,
        returns: Sender<Option<B::Node<S<Z>>>>,
        resolutions: impl Stream<Item = Result<Resolution<B, T, Z>, Error<B::Error>>> + Send + 'static,
    ) {
        self.return_into(
            returns,
            assemble(self.backend.clone(), resolutions, stream::empty()),
        );
    }
}

/// Complete resolutions into parents, filling `Pending` slots from `level`.
///
/// Pairing is positional: resolutions arrive in query order and `level`
/// carries one item per `Pending` in that same order. An empty resolution
/// reaches [`Backend::parent`] with an empty group and resolves to `None`.
pub(super) fn assemble<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static, H: Height>(
    backend: B,
    resolutions: impl Stream<Item = Result<Resolution<B, T, H>, Error<B::Error>>> + Send,
    level: impl Stream<Item = Result<Option<B::Node<H>>, Error<B::Error>>> + Send,
) -> impl Stream<Item = Result<Option<B::Node<S<H>>>, Error<B::Error>>> + Send
where
    S<H>: Height,
{
    try_stream! {
        let mut level = pin!(level.fuse());
        for await resolved in resolutions {
            let Resolution { prefix, resolved } = resolved?;
            let mut children = Vec::with_capacity(resolved.len());
            for (radix, slot) in resolved {
                children.push((radix, match slot {
                    Resolve::Ready(child) => child,
                    Resolve::Pending => {
                        // A `Pending` is a promise our own stages made. If its
                        // source ends, the session is already aborting; park so
                        // that causal error wins rather than manufacturing one.
                        next_or_cancelled(level.next()).await?
                    }
                }));
            }
            yield backend.clone().parent(prefix, children).await?;
        }
    }
}
