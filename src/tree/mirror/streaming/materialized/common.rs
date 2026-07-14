use std::pin::pin;

use futures::{StreamExt as _, stream};
#[cfg(not(test))]
use tokio_stream::wrappers::ReceiverStream;

use crate::tree::{
    mirror::streaming::{Backend, Leaf},
    typed::{
        Prefix,
        height::{Height, S, Z},
    },
};

use super::channel::{Receiver, Sender, channel};

/// Collect one node's children, addressed by radix.
pub async fn children_of<B, T, H>(
    backend: &B,
    prefix: Prefix<S<H>>,
    node: B::Node<S<H>>,
) -> Result<Vec<(u8, B::Node<H>)>, B::Error>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
    S<H>: Height,
{
    let mut children = pin!(backend.clone().children(prefix, node));
    let mut fan = Vec::new();
    while let Some(item) = children.next().await {
        let (prefix, child) = item?;
        let (_, radix) = prefix.pop();
        fan.push((radix, child));
    }
    Ok(fan)
}

/// Create a pair of a sender and a receiver stream, where the receiver
/// wraps items in `Ok`.
pub fn ok_channel<T: Send, E>(buffer: usize) -> (Sender<T>, OkReceiverStream<T, E>) {
    ok_channel_with(channel(buffer))
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
