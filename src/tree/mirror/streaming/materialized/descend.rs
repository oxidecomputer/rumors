//! The height-recursive descent: one [`Descending`] stage per exchange round,
//! down to the two terminals.
//!
//! Each stage's walk lives in [`reconcile`]; this module owns the stage state
//! that travels between rounds and the terminal futures that drive the session's
//! accumulated work to its reconciled [`Root`].

use std::pin::pin;

use futures::future::{self, BoxFuture};
use futures::join;
use futures::stream::StreamExt;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio_stream::wrappers::ReceiverStream;

use crate::tree::mirror::streaming::backend::BoxOptionNodeStream;
use crate::tree::mirror::streaming::protocol::BoxResponses;
use crate::{
    Version,
    tree::typed::{
        Prefix,
        height::{self, Height, S, Z},
    },
};

use super::super::backend::{Backend, BoxNodeStream, Leaf, Root};
use super::super::message;
use super::super::protocol::{self, Requests, Responses};
use super::FAN;
use super::unknown::Unknown;
