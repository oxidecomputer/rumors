//! Fixed-memory multiplexing over one ordered transport per direction.
//!
//! [`incoming()`] builds the stream-lifecycle boundary: decoded frames enter one
//! of 17 one-slot queues, and a full queue stops the sole reader so pressure
//! reaches the transport. [`outgoing()`] builds the scheduler: it chooses the
//! bottom-most ready stream and acknowledges each frame only after its bytes
//! are written and flushed. The codec remains the byte-validation boundary.

use tokio::sync::mpsc;

use super::codec::Stream;

mod incoming;
mod outgoing;

pub use incoming::{Demux, DemuxError, Incoming, incoming};
pub use outgoing::{FrameSender, Mux, MuxError, Outgoing, SendError, outgoing};

/// Number of logical streams multiplexed in one transport direction.
const STREAM_COUNT: usize = Stream::COUNT as usize;

/// Capacity of each readiness handoff between a logical stream and the mux.
const HANDOFF_CAPACITY: usize = 1;

/// Allocate the fixed one-slot handoff pair for every logical stream.
fn handoffs<T>() -> (
    [mpsc::Sender<T>; STREAM_COUNT],
    [mpsc::Receiver<T>; STREAM_COUNT],
) {
    let mut senders = Vec::with_capacity(STREAM_COUNT);
    let receivers = std::array::from_fn(|_| {
        let (send, receive) = mpsc::channel(HANDOFF_CAPACITY);
        senders.push(send);
        receive
    });
    let senders = senders
        .try_into()
        .unwrap_or_else(|_| unreachable!("one sender exists for every stream"));
    (senders, receivers)
}

/// Convert an array position back into its validated logical stream id.
fn stream_at(index: usize) -> Stream {
    Stream::new(u8::try_from(index).expect("a stream index fits in one byte"))
        .expect("a session index names a logical stream")
}

#[cfg(test)]
mod tests;
