use std::pin::pin;

use futures::StreamExt as _;

use crate::tree::{
    mirror::streaming::{Backend, Leaf},
    typed::{
        Prefix,
        height::{Height, S, Z},
    },
};

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
