use futures::{StreamExt as _, stream};
#[cfg(not(test))]
use tokio_stream::wrappers::ReceiverStream;

pub use crate::tree::backend::children_of;

use super::channel::{QueueRole, Receiver, Sender, channel};

/// Create a pair of a sender and a receiver stream, where the receiver
/// wraps items in `Ok`.
pub fn ok_channel<T: Send, E>(
    role: QueueRole,
    buffer: usize,
) -> (Sender<T>, OkReceiverStream<T, E>) {
    ok_channel_with(channel(role, buffer))
}

fn ok_channel_with<T: Send, E>(
    (tx, rx): (Sender<T>, Receiver<T>),
) -> (Sender<T>, OkReceiverStream<T, E>) {
    #[cfg(test)]
    {
        (tx, rx.map(Ok))
    }
    #[cfg(not(test))]
    {
        (tx, ReceiverStream::new(rx).map(Ok))
    }
}

/// The type of a receiver stream wrapping items in `Ok`.
#[cfg(test)]
pub type OkReceiverStream<T, E> = stream::Map<Receiver<T>, fn(T) -> Result<T, E>>;
/// The type of a receiver stream wrapping items in `Ok`.
#[cfg(not(test))]
pub type OkReceiverStream<T, E> = stream::Map<ReceiverStream<T>, fn(T) -> Result<T, E>>;
