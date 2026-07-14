use std::pin::Pin;

use futures::{Stream, stream};

use crate::{
    Version,
    message::Message,
    tree::{
        mirror::streaming::convert::Convert,
        typed::{
            Hash, Prefix,
            height::{self, Height, S, Z},
        },
    },
};

// The specific backends:
mod local;
pub use local::Local;

/// A backend value is a cheap cloneable *handle* to its storage.
pub trait Backend<T: Send + Sync + 'static>: Clone + Send + Sync + 'static
where
    Self::Node<Z>: Leaf<T>,
{
    /// The type of nodes carrying messages of type `T`, indexed by height `H`.
    type Node<H: Height>: Node<T, Height = H, Backend = Self> + Clone + Send + 'static;

    /// The type of errors returned by this backend.
    type Error: Send + 'static;

    /// Assemble one parent node at `prefix` from one radix-keyed child group.
    ///
    /// The group is the parent's entire child set, in strictly increasing radix
    /// order. A `None` entry is an explicit child *deletion*: the child does
    /// not join the parent, and the backend may drop whatever it stores beneath
    /// that radix. A `None` return means no child survived, should propagate as
    /// a `None` entry one level up, cascading deletion to parents whose entire
    /// child set was deleted. The group may also be empty outright — a scope
    /// that resolved to nothing at all, such as the pruned-to-nothing reply to
    /// a request — and resolves to `None` the same way. Given at least one
    /// real child, construction should always yield a parent.
    fn parent<H>(
        self,
        prefix: Prefix<S<H>>,
        children: Vec<(u8, Option<Self::Node<H>>)>,
    ) -> impl Future<Output = Result<Option<Self::Node<S<H>>>, Self::Error>> + Send
    where
        H: Height,
        S<H>: Height;

    /// Explode one parent node at `prefix` into its children, one height down.
    ///
    /// The children are produced in strictly increasing prefix order, each
    /// keyed by the parent's prefix extended with the child's radix.
    fn children<H>(
        self,
        prefix: Prefix<S<H>>,
        parent: Self::Node<S<H>>,
    ) -> impl NodeStream<Self, T, H>
    where
        H: Height,
        S<H>: Height;

    /// Get the leaves of a node directly.
    ///
    /// By default, this is implemented as a streaming recursive traversal of
    /// the node's children, but some backends may be able to obtain this more
    /// efficiently.
    fn leaves<H: Convert>(
        self,
        prefix: Prefix<H>,
        node: Self::Node<H>,
    ) -> impl NodeStream<Self, T, Z> {
        H::explode(
            self,
            Box::pin(stream::once(async move { Ok((prefix, node)) })),
        )
    }
}

/// The inspection operations of a backend's individual node type.
pub trait Node<T: Send + Sync + 'static> {
    /// The backend to which this node belongs.
    type Backend: Backend<T, Node<Z>: Leaf<T>, Node<Self::Height> = Self>;

    /// The height of the node above the leaf level.
    type Height: Height;

    /// The maximum version of any node under this one.
    fn ceiling(&self) -> &Version;

    /// The minimum version of any node under this one.
    fn floor(&self) -> &Version;

    /// The merkle hash of this node.
    fn hash(&self) -> Hash;
}

/// What crosses between backends at the conversion boundary, and the one node
/// shape every backend must represent faithfully.
pub trait Leaf<T: Send + Sync + 'static>: Node<T> {
    /// The message stored at this leaf node.
    fn message(&self) -> &Message<T>;

    /// Construct a leaf node.
    fn leaf(version: Version, message: Message<T>) -> Self;
}

/// Type synonym for a fallible [`Stream`] of prefix-keyed nodes represented by
/// a given backend.
pub trait NodeStream<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static, H: Height>:
    Stream<Item = Result<(Prefix<H>, B::Node<H>), B::Error>> + Send
{
}
impl<N, B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static, H: Height> NodeStream<B, T, H>
    for N
where
    N: Stream<Item = Result<(Prefix<H>, B::Node<H>), B::Error>> + Send,
{
}

/// A [`NodeStream`] erased to one level of type depth.
pub(super) type BoxNodeStream<'a, B, T, H> = Pin<Box<dyn NodeStream<B, T, H> + 'a>>;

/// A backend's whole tree at rest: what a mirror session consumes and produces.
///
/// This is the backend-generic form of [`tree::Root`](crate::tree::Root); the
/// `Local` backend converts between the two with [`From`].
#[derive(Debug)]
pub struct Root<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static> {
    /// The maximum version this tree has incorporated.
    pub ceiling: Version,
    /// The root node, or nothing when the tree is empty.
    pub root: Option<B::Node<height::Root>>,
}

// Manual because the derive would demand `T: Clone`; nodes are cloneable
// handles regardless of the message type they carry.
impl<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static> Clone for Root<B, T> {
    fn clone(&self) -> Self {
        Root {
            ceiling: self.ceiling.clone(),
            root: self.root.clone(),
        }
    }
}
