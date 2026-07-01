//! Re-represent a node stream from one backend in the node types of another.

use futures::StreamExt;

use crate::tree::typed::height::{Height, S, Z};

use super::backend::{Backend, Leaf, Node, NodeStream};

/// Convert a `stream` of `from`'s nodes at height `H` into the equivalent
/// stream of `to`'s nodes.
pub fn convert<B, C, T, H>(
    from: &B,
    to: &C,
    stream: impl NodeStream<B, T, H>,
) -> impl NodeStream<C, T, H>
where
    B: Backend<T>,
    C: Backend<T>,
    B::Node<Z>: Leaf<T>,
    C::Node<Z>: Leaf<T>,
    C::Error: From<B::Error>,
    H: Convert,
{
    H::convert(from, to, stream)
}

/// A height at which a [`NodeStream`] can be re-represented across backends.
pub trait Convert: Height {
    /// Re-represent `stream`, a prefix-ordered stream of `from`'s nodes at this
    /// height, as the equivalent stream of `to`'s nodes.
    ///
    /// Order is preserved: the output carries the same prefixes in the same
    /// strictly-increasing order as the input.
    fn convert<B, C, T>(
        from: &B,
        to: &C,
        stream: impl NodeStream<B, T, Self>,
    ) -> impl NodeStream<C, T, Self>
    where
        B: Backend<T>,
        C: Backend<T>,
        B::Node<Z>: Leaf<T>,
        C::Node<Z>: Leaf<T>,
        C::Error: From<B::Error>;
}

impl Convert for Z {
    fn convert<B, C, T>(
        _from: &B,
        _to: &C,
        stream: impl NodeStream<B, T, Z>,
    ) -> impl NodeStream<C, T, Z>
    where
        B: Backend<T>,
        C: Backend<T>,
        B::Node<Z>: Leaf<T>,
        C::Node<Z>: Leaf<T>,
        C::Error: From<B::Error>,
    {
        stream.map(|item| -> Result<_, C::Error> {
            let (prefix, leaf) = item?;
            // A leaf's ceiling and floor are both equal to its version:
            let version = leaf.ceiling().clone();
            let message = leaf.message().clone();
            Ok((prefix, Leaf::leaf(version, message)))
        })
    }
}

impl<H> Convert for S<H>
where
    H: Convert,
    S<H>: Height,
{
    fn convert<B, C, T>(
        from: &B,
        to: &C,
        stream: impl NodeStream<B, T, S<H>>,
    ) -> impl NodeStream<C, T, S<H>>
    where
        B: Backend<T>,
        C: Backend<T>,
        B::Node<Z>: Leaf<T>,
        C::Node<Z>: Leaf<T>,
        C::Error: From<B::Error>,
    {
        let children = from.clone().children::<H>(stream);
        let converted = H::convert::<B, C, T>(from, to, children);
        to.clone().parents::<H>(converted)
    }
}
