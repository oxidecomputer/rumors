//! The streaming protocol implemented generically for every materialized
//! backend.
//!
//! Any [`Backend`] holding a real tree — one whose nodes answer
//! [`Node`](super::backend::Node)'s hash and version bounds, and whose leaves
//! are [`Leaf`]s — can be used here, with no further ceremony. The stages speak
//! that backend's node types on both sides of the wire: what a walk emits is
//! what the counterparty reads.

// Where we're going, we need to write some Complex Types.
#![allow(clippy::type_complexity)]

use async_stream::try_stream;
use futures::channel::mpsc::{self, Receiver, Sender};
use futures::future::{self, BoxFuture};
use futures::stream::StreamExt;
use futures::{SinkExt, Stream, join};
use std::pin::pin;

use crate::tree::mirror::streaming::FAN;
use crate::{
    Version,
    tree::typed::{
        Prefix,
        height::{self, Height, S, Z},
    },
};

use super::backend::{Backend, Leaf, NodeStream, Root};
use super::protocol::Responses;

mod descend;
mod handshake;
pub(super) mod unknown;

pub use handshake::Handshaking;

#[derive(Debug, thiserror::Error)]
pub enum Error<E> {
    #[error(transparent)]
    Backend(#[from] E),
    #[error(transparent)]
    Violation(Violation),
}

/// The ways a counterparty can misbehave: exactly the semantic faults
/// only this side can detect, because they depend on our questions and
/// our tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Violation {
    /// A reply arrived with no query outstanding.
    #[error("reply received for unknown query")]
    UnaskedReply,
    /// The reply stream ended while questions were outstanding.
    #[error("no reply to outstanding query")]
    UnansweredQuery,
    /// The reply ended before reacting to every listed child.
    #[error("reply failed to cover every listed radix")]
    UnfinishedReply,
    /// A positional `Match` after every held child has been answered.
    #[error("reply attempted to match unknown child")]
    UnexpectedMatch,
    /// A positional `Query` after every held child has been answered.
    #[error("reply attempted to query unknown child")]
    UnexpectedQuery,
    /// A `Supply` whose radix lands on an already-held child.
    #[error("reply attempted to supply a child that is already known")]
    UnexpectedSupply,
    /// A `Supply` whose radix violates the implicit ordering of children.
    #[error("reply attempted to supply a child out of order")]
    InvalidSupply,
}

/// A pending query, which we will resolve by a remote reply.
pub struct Query<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static, H: Height>
where
    S<H>: Height,
{
    /// The prefix at which the resolved node will sit.
    pub prefix: Prefix<S<H>>,
    /// Our children of the node (empty if we don't have it at all).
    pub ours: Vec<(u8, B::Node<H>)>,
}

pub struct Resolution<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static, H: Height>
where
    S<H>: Height,
{
    /// The prefix at which the resolved node will sit.
    prefix: Prefix<S<H>>,
    /// The possibly-resolved children of the node.
    resolved: Vec<(u8, Resolve<B, T, H>)>,
}

pub enum Resolve<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static, H: Height> {
    /// Resolved at the current level: kept, absorbed, or pruned (`None` = gone;
    /// flows into `Backend::parent` as its deletion vocabulary).
    Ready(Option<B::Node<H>>),
    /// Resolved elsewhere: filled by the hole stream's next item.
    Pending,
}

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
                    Resolve::Pending =>
                        level.next().await
                            .expect("level stream ended early")?

                }));
            }
            yield backend.clone().parent(prefix, children).await?;
        }
    }
}
