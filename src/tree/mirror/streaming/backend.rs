use std::pin::Pin;

use futures::{Stream, future, stream};

use crate::{
    Version,
    message::Message,
    tree::typed::{
        Hash, Prefix,
        height::{self, Height, S, Z},
    },
};

// The specific backends:
mod local;
pub use local::Local;

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
///
/// A backend value is a cheap cloneable *handle* to its storage — an
/// in-memory tree's is zero-sized, a persistent one's is an `Arc` of its
/// state — never the storage itself. The [session](super::session) machinery
/// clones one handle per concurrently scheduled worker, which is what the
/// supertraits require: handles are shared freely (`Clone`), cross into owned
/// futures (`Send + 'static`), and are borrowed from streams that must
/// themselves be `Send` (`Sync`).
pub trait Backend<T>: Clone + Send + Sync + 'static
where
    Self::Node<Z>: Leaf<T>,
{
    /// The type of nodes carrying messages of type `T`, indexed by height `H`.
    ///
    /// Nodes are handles too: they ride the [`NodeStream`]s the protocol
    /// threads between its workers (`Send + 'static`), and the session keeps
    /// a node while separately providing its children to the counterparty
    /// (`Clone`).
    type Node<H: Height>: Node<Height = H> + Clone + Send + 'static;

    /// The type of errors returned by this backend.
    ///
    /// `Send + 'static` for the same reason as [`Node`](Self::Node): errors
    /// ride the node streams as their failure arm.
    type Error: Send + 'static;

    /// Assemble a stream of children at height `H` into a stream of parents at height `H + 1`.
    ///
    /// This may assume that the children are in strictly increasing prefix order, and it
    /// must produce parents also in strictly increasing prefix order, propagating the
    /// input's errors.
    ///
    /// Takes the handle by value so the returned stream owns it and stays
    /// `'static`; callers clone the handle they keep.
    fn parents<H>(self, children: impl NodeStream<Self, T, H>) -> impl NodeStream<Self, T, S<H>>
    where
        H: Height,
        S<H>: Height;

    /// Disassemble a stream of parents at height `H + 1` into a stream of children at height `H`.
    ///
    /// This may assume that the parents are in strictly increasing prefix order, and it
    /// must produce children also in strictly increasing prefix order, propagating the
    /// input's errors.
    ///
    /// Takes the handle by value so the returned stream owns it and stays
    /// `'static`; callers clone the handle they keep.
    fn children<H>(self, parents: impl NodeStream<Self, T, S<H>>) -> impl NodeStream<Self, T, H>
    where
        H: Height,
        S<H>: Height;
}

/// Type synonym for a fallible [`Stream`] of prefix-keyed nodes represented by a given backend.
pub trait NodeStream<B: Backend<T, Node<Z>: Leaf<T>>, T, H: Height>:
    Stream<Item = Result<(Prefix<H>, B::Node<H>), B::Error>> + Send
{
}
impl<N, B: Backend<T, Node<Z>: Leaf<T>>, T, H: Height> NodeStream<B, T, H> for N where
    N: Stream<Item = Result<(Prefix<H>, B::Node<H>), B::Error>> + Send
{
}

/// A [`NodeStream`] erased to one level of type depth.
///
/// Every height-recursive transducer over node streams boxes at each level:
/// an `impl Stream` threaded through the full height of the tree would nest
/// each level's stream type inside the next and balloon the compiler's types
/// past any bound.
pub(super) type BoxNodeStream<B, T, H> = Pin<Box<dyn NodeStream<B, T, H>>>;

/// A stream of one prefix-keyed node.
///
/// The seed for anything that operates on a single subtree through the
/// stream algebra: exploding it one level via [`Backend::children`], or
/// pruning it via [`unknown`](super::unknown::unknown).
pub(super) fn one<B, T, H>(prefix: Prefix<H>, node: B::Node<H>) -> impl NodeStream<B, T, H>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    H: Height,
{
    stream::once(future::ready(Ok((prefix, node))))
}

/// A backend's whole tree at rest: what a mirror session consumes and
/// produces.
///
/// This is the backend-generic form of [`tree::Root`](crate::tree::Root); the
/// `Local` backend converts between the two with [`From`]. The ceiling rides
/// separately from the root node because the two can disagree: redaction
/// advances a tree's version while removing nodes, so an empty tree still
/// carries the version at which it became empty — which is exactly what
/// deletion honoring compares against on the next reconciliation.
#[derive(Debug)]
pub struct Root<B, T>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
{
    /// The maximum version this tree has incorporated.
    pub ceiling: Version,
    /// The root node, or nothing when the tree is empty.
    pub root: Option<B::Node<height::Root>>,
}

// Manual because the derive would demand `T: Clone`; nodes are cloneable
// handles regardless of the message type they carry.
impl<B, T> Clone for Root<B, T>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
{
    fn clone(&self) -> Self {
        Root {
            ceiling: self.ceiling.clone(),
            root: self.root.clone(),
        }
    }
}
