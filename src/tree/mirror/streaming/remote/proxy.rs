//! Type-level protocol proxy over one session's pipe.
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
/// A task's consumer stops only when the session is failing. Parking the task
/// until the session executor cancels all work keeps the error that stopped
/// the consumer the one the executor reports.
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
pub(crate) use work::Parked;

#[cfg(test)]
mod tests;
