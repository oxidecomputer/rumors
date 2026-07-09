//! The streaming protocol implemented generically for every materialized
//! backend.
//!
//! Any [`Backend`] holding a real tree — one whose nodes answer
//! [`Node`](super::backend::Node)'s hash and version bounds, and whose leaves
//! are [`Leaf`]s — can be used here,
//! with no further ceremony. The stages speak that backend's node types on
//! both sides of the wire: what a walk emits is what the counterparty reads.

use futures::channel::mpsc::{self, Receiver, Sender};
use futures::future::{self, BoxFuture};
use futures::stream::StreamExt;
use futures::{SinkExt, join};
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
mod dispute;
mod handshake;
mod merge;
mod reconcile;
// `pub(super)` (not plain `mod`) so sibling modules' docs can link into it:
// `backend::one` names `unknown::unknown` as the other consumer of its seeds.
pub(super) mod unknown;

pub use handshake::Handshaking;
