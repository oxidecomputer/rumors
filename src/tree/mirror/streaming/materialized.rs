//! The streaming protocol implemented generically for every materialized
//! backend.
//!
//! Any [`Backend`] whose `Materialized = Material` automatically can be used
//! here, with no further ceremony.

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
mod unknown;

pub use handshake::Handshaking;
