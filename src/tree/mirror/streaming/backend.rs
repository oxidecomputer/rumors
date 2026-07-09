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

/// A backend value is a cheap cloneable *handle* to its storage.
///
/// A backend must know how to assemble and disassemble its own node types in a
/// prefix-ordered streaming fashion. These operations are pure re-chunking of
/// keys: nothing here requires nodes to carry hashes or version bounds, which
/// is what lets a remote party, whose intermediate "nodes" are framed leaf
/// sequences, implement `Backend` at [`Materialized`](Self::Materialized) `=
/// `[`Immaterial`].
pub trait Backend<T>: Clone + Send + Sync + 'static {
    /// The type of nodes carrying messages of type `T`, indexed by height `H`.
    ///
    /// The [`Node`] bound dispatches through
    /// [`Materialized`](Self::Materialized), so it holds at every height for
    /// material and immaterial backends alike.
    type Node<H: Height>: Node + Clone + Send + 'static;

    /// The type of errors returned by this backend.
    type Error: Send + 'static;

    /// Assemble a stream of children at height `H` into a stream of parents at
    /// height `H + 1`.
    ///
    /// This may assume that the children are in strictly increasing prefix
    /// order, and it must produce parents also in strictly increasing prefix
    /// order, propagating the input's errors.
    fn parents<H>(self, children: impl NodeStream<Self, T, H>) -> impl NodeStream<Self, T, S<H>>
    where
        H: Height,
        S<H>: Height;

    /// Disassemble a stream of parents at height `H + 1` into a stream of
    /// children at height `H`.
    ///
    /// This may assume that the parents are in strictly increasing prefix
    /// order, and it must produce children also in strictly increasing prefix
    /// order, propagating the input's errors.
    fn children<H>(self, parents: impl NodeStream<Self, T, S<H>>) -> impl NodeStream<Self, T, H>
    where
        H: Height,
        S<H>: Height;
}

/// The inspection operations of a backend's individual node type, dispatched
/// by materiality.
///
/// The default `M = Material` is the interesting instantiation: real version
/// bounds and hashes, which is what the session's walks require. An
/// [`Immaterial`] impl is three unit returns; it exists so the [`Backend`] GAT
/// bound holds uniformly at every height while promising nothing.
pub trait Node {
    /// The maximum version of any node under this one.
    fn ceiling(&self) -> &Version;

    /// The minimum version of any node under this one.
    fn floor(&self) -> &Version;

    /// The merkle hash of this node.
    fn hash(&self) -> Hash;
}

/// What crosses between backends at the conversion boundary, and the one node
/// shape every backend must represent faithfully.
pub trait Leaf<T> {
    /// The version at which this leaf's message was incorporated.
    ///
    /// # Contract
    ///
    /// For a [`Material`] backend's leaf this must equal [`Node::ceiling`] and
    /// [`Node::floor`]: a leaf's ceiling and floor *are* its version.
    fn version(&self) -> &Version;

    /// The message stored at this leaf node.
    fn message(&self) -> &Message<T>;

    /// Construct a leaf node.
    fn leaf(version: Version, message: Message<T>) -> Self;
}

/// Type synonym for one prefix-keyed node of a backend: the item of a
/// [`NodeStream`], and what the session's internal channels carry.
pub(super) type Keyed<B, T, H> = (Prefix<H>, <B as Backend<T>>::Node<H>);

/// Type synonym for a fallible [`Stream`] of prefix-keyed nodes represented by
/// a given backend.
pub trait NodeStream<B: Backend<T>, T, H: Height>:
    Stream<Item = Result<(Prefix<H>, B::Node<H>), B::Error>> + Send
{
}
impl<N, B: Backend<T>, T, H: Height> NodeStream<B, T, H> for N where
    N: Stream<Item = Result<(Prefix<H>, B::Node<H>), B::Error>> + Send
{
}

/// A [`NodeStream`] erased to one level of type depth.
///
/// Every height-recursive transducer over node streams boxes at each level: an
/// `impl Stream` threaded through the full height of the tree would nest each
/// level's stream type inside the next and balloon the compiler's types past
/// any bound.
pub(super) type BoxNodeStream<B, T, H> = Pin<Box<dyn NodeStream<B, T, H>>>;

/// A stream of one prefix-keyed node.
///
/// The seed for anything that operates on a single subtree through the stream
/// algebra: exploding it one level via [`Backend::children`], or pruning it via
/// [`unknown`](super::materialized::unknown::unknown).
pub(super) fn one<B, T, H>(prefix: Prefix<H>, node: B::Node<H>) -> impl NodeStream<B, T, H>
where
    B: Backend<T>,
    H: Height,
{
    stream::once(async move { Ok((prefix, node)) })
}

/// A backend's whole tree at rest: what a mirror session consumes and produces.
///
/// This is the backend-generic form of [`tree::Root`](crate::tree::Root); the
/// `Local` backend converts between the two with [`From`].
#[derive(Debug)]
pub struct Root<B, T>
where
    B: Backend<T>,
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
    B: Backend<T>,
{
    fn clone(&self) -> Self {
        Root {
            ceiling: self.ceiling.clone(),
            root: self.root.as_ref().map(|r| r.clone()),
        }
    }
}
