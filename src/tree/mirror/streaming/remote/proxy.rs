//! Type-level protocol proxy over a link's per-stream transport session.
//!
//! [`Handshaking`] hides transport startup behind the same protocol boundary
//! used by an in-process participant. Once the shared driver dispatches the
//! elected roles, each typed state owns the one scope queue needed to interpret
//! replies at its height. Background tasks encode local responses and decode
//! remote responses concurrently so transport backpressure never serializes the
//! two directions.

use crate::tree::mirror::streaming::{channel::Sender, tasks::cancelled};

/// Send one internal item or await cancellation if its consumer has gone.
///
/// A task may still own a transport stream when its consumer stops. Parking it
/// keeps that stream alive until the session executor cancels all work, rather
/// than presenting the peer with a torn stream during error propagation.
async fn send_or_cancel<T>(sender: &mut Sender<T>, value: T) {
    if sender.send(value).await.is_err() {
        cancelled().await;
    }
}

mod error;
mod start;
mod state;
mod work;

pub use error::Error;
pub use start::Handshaking;
pub use state::Connected;
pub use work::ControlRead;

#[cfg(test)]
mod tests;
