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

/// The materiality of a backend's nodes: [`Material`] nodes carry real
/// Merkle hashes and version bounds, [`Immaterial`] nodes are opaque
/// transport cargo.
///
/// This is the type-level switch [`Backend`] dispatches its node
/// requirements through: the [`Node`] operations' return types project
/// through the backend's [`Materialized`](Backend::Materialized), so a
/// material backend's nodes answer with real [`Version`]s and
/// [`struct@Hash`]es
/// while an immaterial backend's answer with units. Session code that
/// *walks* trees — comparing hashes, pruning by version bounds — demands
/// `Materialized = Material`; everything that merely moves nodes around
/// (the protocol schedule, the drivers, the conversion boundary) accepts
/// either.
///
/// Sealed: exactly these two materialities exist.
pub trait Materiality: sealed::Sealed + 'static {
    /// What this materiality knows about version bounds: [`Version`] when
    /// material, `()` when immaterial.
    type Version;

    /// What this materiality knows about Merkle hashes: [`struct@Hash`]
    /// when material, `()` when immaterial.
    type Hash;
}

/// The materiality of backends whose nodes are inspectable: every node
/// reports its Merkle hash and version bounds.
pub enum Material {}

/// The materiality of backends whose nodes are opaque transport cargo:
/// re-chunkable by prefix, but carrying no intermediate hashes or version
/// bounds.
///
/// This is the shape of a wire party, whose node payloads are framed leaf
/// sequences.
pub enum Immaterial {}

impl Materiality for Material {
    type Version = Version;
    type Hash = Hash;
}

impl Materiality for Immaterial {
    type Version = ();
    type Hash = ();
}

mod sealed {
    pub trait Sealed {}
    impl Sealed for super::Material {}
    impl Sealed for super::Immaterial {}
}

/// The inspection operations of a backend's individual node type, dispatched
/// by materiality.
///
/// The default `M = Material` is the interesting instantiation: real
/// version bounds and hashes, which is what the session's walks require. An
/// [`Immaterial`] impl is three unit returns — it exists so the
/// [`Backend`] GAT bound holds uniformly at every height while promising
/// nothing: an immaterial node's answers are *uninformative*, not
/// unavailable, and only the `Materialized = Material` bound on the session
/// keeps walk-shaped code meaningful.
pub trait Node<M: Materiality = Material> {
    /// The maximum version of any node under this one.
    fn ceiling(&self) -> &M::Version;

    /// The minimum version of any node under this one.
    fn floor(&self) -> &M::Version;

    /// The merkle hash of this node.
    fn hash(&self) -> M::Hash;
}

/// The leaf currency: what crosses between backends at the conversion
/// boundary, and the one node shape every backend — material or not — must
/// represent faithfully.
///
/// Deliberately not a [`Node`] refinement: requiring one would force
/// backend-agnostic code that only touches leaves (the conversion boundary)
/// to name a materiality it doesn't care about. A leaf is exactly a version
/// and a message, and both are real even for a wire party.
pub trait Leaf<T> {
    /// The version at which this leaf's message was incorporated.
    ///
    /// # Contract
    ///
    /// For a [`Material`] backend's leaf this must equal
    /// [`Node::ceiling`] — a leaf's ceiling *is* its version. The two
    /// methods are separate impls with no type-level tie, and the session's
    /// deletion honoring reads `ceiling` while the conversion boundary
    /// reads `version`: letting them disagree silently diverges
    /// reconciliation.
    fn version(&self) -> &Version;

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
/// These are pure key-algebra re-chunking: nothing here requires nodes to
/// carry hashes or version bounds, which is what lets a wire party — whose
/// intermediate "nodes" are framed leaf sequences — implement `Backend` at
/// [`Materialized`](Self::Materialized) `= `[`Immaterial`]. What each layer
/// additionally demands is spelled at its use sites: the conversion
/// boundary asks `Node<Z>: Leaf<T>` (leaves cross by value), and the
/// session's walks ask `Materialized = Material` (they inspect every
/// height).
///
/// A backend value is a cheap cloneable *handle* to its storage — an
/// in-memory tree's is zero-sized, a persistent one's is an `Arc` of its
/// state — never the storage itself. The [session](super::session) machinery
/// clones one handle per concurrently scheduled worker, which is what the
/// supertraits require: handles are shared freely (`Clone`), cross into owned
/// futures (`Send + 'static`), and are borrowed from streams that must
/// themselves be `Send` (`Sync`).
pub trait Backend<T>: Clone + Send + Sync + 'static {
    /// Whether this backend's nodes are inspectable ([`Material`]) or
    /// opaque transport cargo ([`Immaterial`]).
    type Materialized: Materiality;

    /// The type of nodes carrying messages of type `T`, indexed by height `H`.
    ///
    /// The [`Node`] bound dispatches through
    /// [`Materialized`](Self::Materialized), so it holds at every height
    /// for material and immaterial backends alike — the universal property
    /// the session's height-generic walks lean on.
    ///
    /// Nodes are handles too: they ride the [`NodeStream`]s the protocol
    /// threads between its workers (`Send + 'static`), and the session keeps
    /// a node while separately providing its children to the counterparty
    /// (`Clone`).
    type Node<H: Height>: Node<Self::Materialized> + Clone + Send + 'static;

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
    B: Backend<T>,
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
            root: self.root.clone(),
        }
    }
}
