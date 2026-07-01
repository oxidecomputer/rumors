use futures::Stream;

use crate::{
    Version,
    message::Message,
    tree::typed::{
        Hash, Prefix,
        height::{Height, S, Z},
    },
};

// The specific backends:
mod local;
pub use local::{Handshaking, Local};

/// The fundamental operations required by a backend's individual node type.
pub trait Node {
    /// The height of this node.
    type Height: Height;

    /// The maximum version of any node under this one.
    fn ceiling(&self) -> &Version;

    /// The minimum version of any node under this one.
    fn floor(&self) -> &Version;

    /// The number of leaves under this node.
    fn len(&self) -> usize;

    /// The merkle hash of this node.
    fn hash(&self) -> Hash;
}

pub trait Leaf<T>: Node<Height = Z> {
    /// The message stored at this leaf node.
    fn message(&self) -> &Message<T>;

    /// Construct a leaf node.
    fn leaf(version: Version, message: Message<T>) -> Self;
}

/// The fundamental operations required by a backend to the protocol.
///
/// A backend must know how to assemble and disassemble its own node types in a prefix-ordered
/// streaming fashion.
pub trait Backend<T>
where
    Self::Node<Z>: Leaf<T>,
{
    /// The type of nodes carrying messages of type `T`, indexed by height `H`.
    ///
    /// Nodes are [`Send`] and `'static`: they ride the [`NodeStream`]s the
    /// protocol threads between stages, which buffer items in channels drained
    /// by independently scheduled (and so owned, `'static`-boxed) pump
    /// futures.
    type Node<H: Height>: Node<Height = H> + Send + 'static;

    /// The type of errors returned by this backend.
    ///
    /// [`Send`] and `'static` for the same reason as [`Node`](Self::Node):
    /// errors ride the node streams as their failure arm.
    type Error: Send + 'static;

    /// Assemble a stream of children at height `H` into a stream of parents at height `H + 1`.
    ///
    /// This may assume that the children are in strictly increasing prefix order, and it
    /// must produce parents also in strictly increasing prefix order.
    fn parents<H>(&self, children: impl NodeStream<Self, T, H>) -> impl NodeStream<Self, T, S<H>>
    where
        H: Height,
        S<H>: Height;

    /// Disassemble a stream of parents at height `H + 1` into a stream of children at height `H`.
    ///
    /// This may assume that the parents are in strictly increasing prefix order, and it
    /// must produce children also in strictly increasing prefix order.
    fn children<H>(&self, parents: impl NodeStream<Self, T, S<H>>) -> impl NodeStream<Self, T, H>
    where
        H: Height,
        S<H>: Height;
}

/// Type synonym for a fallible [`Stream`] of prefix-keyed nodes represented by a given backend.
pub trait NodeStream<B: Backend<T, Node<Z>: Leaf<T>> + ?Sized, T, H: Height>:
    Stream<Item = Result<(Prefix<H>, B::Node<H>), B::Error>> + Send
{
}
impl<N, B: Backend<T, Node<Z>: Leaf<T>> + ?Sized, T, H: Height> NodeStream<B, T, H> for N where
    N: Stream<Item = Result<(Prefix<H>, B::Node<H>), B::Error>> + Send
{
}
