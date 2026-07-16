//! Type-level protocol proxy over the multiplexed transport session.
//!
//! [`Handshaking`] hides transport startup behind the same protocol boundary
//! used by an in-process participant. Once the handshake elects wire roles,
//! each typed state owns the one scope queue needed to interpret replies at its
//! height. The reply pumps encode local responses and decode remote responses
//! concurrently so transport backpressure never serializes the two directions.

use crate::tree::mirror::streaming::{channel::Sender, tasks::cancelled};

/// Send one internal item or await cancellation if its consumer has gone.
async fn send_or_cancel<T>(sender: &Sender<T>, value: T) {
    if sender.send(value).await.is_err() {
        cancelled().await;
    }
}

/// Publish a decoded reply before the lower scopes which depend on it.
///
/// The `yield` expression belongs inside the invocation so `async_stream` can
/// lower it before this macro expands. Keeping both phases together prevents a
/// caller from filling the one-slot scope queue while withholding the reply
/// which lets its consumer advance and drain that queue. This is the remote
/// proxy analogue of the materialized implementation's `yield_resolve_query!`.
macro_rules! yield_reply_scopes {
    (
        $progress:expr, $height:ty, $count:expr;
        $yielded:expr;
        $scopes:expr => $next_scopes:expr;
    ) => {{
        $progress.decoded_reply::<$height>($count);
        $yielded;
        for scope in $next_scopes {
            $progress.next_scope::<$height>();
            $crate::tree::mirror::streaming::remote::proxy::send_or_cancel(&$scopes, scope).await;
        }
    }};
}

mod error;
mod start;
mod state;
mod work;

pub use error::Error;
pub use start::Handshaking;
pub use state::Connected;

#[cfg(test)]
mod tests;
