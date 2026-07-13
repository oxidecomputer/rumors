use std::pin::{Pin, pin};

use async_stream::try_stream;
use futures::stream::StreamExt;
use futures::{Stream, stream};

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
/// A backend speaks two singular operations, one per direction: [`children`]
/// explodes one node into its child stream, and [`parent`] assembles one node
/// from one radix-keyed child group. Everything level-shaped — coalescing an
/// ascending child stream into parent groups — lives above the trait in
/// [`fold_parents`], so a backend never sees more than one node's fan at once
/// and carries no cross-parent ordering obligations.
///
/// [`children`]: Backend::children
/// [`parent`]: Backend::parent
pub trait Backend<T>: Clone + Send + Sync + 'static
where
    Self::Node<Z>: Leaf<T>,
{
    /// The type of nodes carrying messages of type `T`, indexed by height `H`.
    type Node<H: Height>: Node<T, Height = H, Backend = Self> + Clone + Send + 'static;

    /// The type of errors returned by this backend.
    type Error: Send + 'static;

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

    /// Assemble one parent node at `prefix` from one radix-keyed child group.
    ///
    /// The group is the parent's entire child set, in strictly increasing
    /// radix order, at least one entry. A `None` entry is an explicit child
    /// *deletion*: the child does not join the parent, and the backend may
    /// drop whatever it stores beneath that radix. A `None` return means no
    /// child survived — the parent itself is deleted — which the caller
    /// propagates as a `None` entry one level up, cascading deletion to
    /// parents whose entire child set was deleted. Given at least one real
    /// child, construction always yields a parent.
    fn parent<H>(
        self,
        prefix: Prefix<S<H>>,
        children: Vec<(u8, Option<Self::Node<H>>)>,
    ) -> impl Future<Output = Result<Option<Self::Node<S<H>>>, Self::Error>> + Send
    where
        H: Height,
        S<H>: Height;
}

/// Reassemble an ascending marked child stream into its marked parent level,
/// one complete radix group at a time.
///
/// The stream is *marked* (see [`merge`](super::materialized)'s module
/// docs): real children interleave with watermarks, `(k, None)` items
/// asserting nothing keyed at or below `k` follows. Children of a given
/// parent arrive contiguously, so each run of equal parent prefixes
/// coalesces into one group, flushed through [`Backend::parent`] when the
/// prefix changes, when the input ends, or when a watermark covers the open
/// parent — the trigger that cannot wait for the next real child, because in
/// a one-sided region none is coming. A watermark never joins a group (it is
/// not a delete entry — that vocabulary belongs to [`Backend::parent`]'s
/// callers in `unknown`); it translates up a height: a watermark at the
/// maximal child `q·0xff` guarantees `q` itself, any other child watermark
/// guarantees `q`'s predecessor, and the translated watermark is emitted
/// only where it advances past the last yielded key (a real item implies its
/// own key's passage, so echoes carry nothing). Groups therefore stay
/// all-real and non-empty and construction always yields a parent; a `None`
/// from the backend is a contract violation (debug builds panic, release
/// builds drop the parent).
pub(super) fn fold_parents<B, T, H>(
    backend: B,
    children: impl OptionNodeStream<B, T, H>,
) -> impl OptionNodeStream<B, T, S<H>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
    S<H>: Height,
{
    /// Flush a completed group, if any, into its parent.
    async fn flush<B, T, H>(
        backend: &B,
        finished: Option<(Prefix<S<H>>, Vec<(u8, Option<B::Node<H>>)>)>,
    ) -> Result<Option<(Prefix<S<H>>, B::Node<S<H>>)>, B::Error>
    where
        B: Backend<T, Node<Z>: Leaf<T>>,
        T: Send + Sync + 'static,
        H: Height,
        S<H>: Height,
    {
        let Some((prefix, group)) = finished else {
            return Ok(None);
        };
        let parent = backend.clone().parent(prefix, group).await?;
        debug_assert!(
            parent.is_some(),
            "an all-real child group failed to construct its parent",
        );
        Ok(parent.map(|parent| (prefix, parent)))
    }

    try_stream! {
        let mut children = pin!(children);
        let mut open: Option<(_, Vec<_>)> = None;
        // The last key yielded, real or watermark. Real yields ascend by
        // construction (their input does); watermark yields are gated on
        // advancing it, which is also what suppresses an echo of a key a
        // flush just carried.
        let mut last: Option<Prefix<S<H>>> = None;
        while let Some(item) = children.next().await {
            let (path, child) = item?;
            let (prefix, radix) = path.pop();
            match child {
                Some(child) => match &mut open {
                    Some((current, group)) if *current == prefix => {
                        group.push((radix, Some(child)));
                    }
                    _ => {
                        let finished = open.replace((prefix, vec![(radix, Some(child))]));
                        if let Some((flushed, parent)) = flush(&backend, finished).await? {
                            last = Some(flushed);
                            yield (flushed, Some(parent));
                        }
                    }
                },
                None => {
                    // Translate the watermark one level up. At the maximal
                    // child no sibling can follow, so the parent itself is
                    // covered; below it, later siblings may still come, so
                    // the guarantee steps back to the parent's predecessor —
                    // and at all-zeros there is nothing below to speak of.
                    let guarantee = if radix == 0xff { Some(prefix) } else { prefix.pred() };
                    let Some(guarantee) = guarantee else { continue };
                    // The covered open parent's child run is complete even
                    // though no prefix change or input end has said so.
                    let covered = open.as_ref().is_some_and(|(current, _)| *current <= guarantee);
                    let finished = if covered { open.take() } else { None };
                    if let Some((flushed, parent)) = flush(&backend, finished).await? {
                        last = Some(flushed);
                        yield (flushed, Some(parent));
                    }
                    if last.is_none_or(|l| guarantee > l) {
                        last = Some(guarantee);
                        yield (guarantee, None);
                    }
                }
            }
        }
        if let Some((flushed, parent)) = flush(&backend, open.take()).await? {
            yield (flushed, Some(parent));
        }
    }
}

/// The inspection operations of a backend's individual node type.
pub trait Node<T> {
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
pub trait Leaf<T>: Node<T> {
    /// The message stored at this leaf node.
    fn message(&self) -> &Message<T>;

    /// Construct a leaf node.
    fn leaf(version: Version, message: Message<T>) -> Self;
}

/// Type synonym for a fallible [`Stream`] of prefix-keyed nodes represented by
/// a given backend.
pub trait NodeStream<B: Backend<T, Node<Z>: Leaf<T>>, T, H: Height>:
    Stream<Item = Result<(Prefix<H>, B::Node<H>), B::Error>> + Send
{
}
impl<N, B: Backend<T, Node<Z>: Leaf<T>>, T, H: Height> NodeStream<B, T, H> for N where
    N: Stream<Item = Result<(Prefix<H>, B::Node<H>), B::Error>> + Send
{
}

/// A [`NodeStream`] erased to one level of type depth.
pub(super) type BoxNodeStream<'a, B, T, H> = Pin<Box<dyn NodeStream<B, T, H> + 'a>>;

/// Type synonym for a fallible [`Stream`] of prefix-keyed optional nodes
/// represented by a given backend.
pub trait OptionNodeStream<B: Backend<T, Node<Z>: Leaf<T>>, T, H: Height>:
    Stream<Item = Result<(Prefix<H>, Option<B::Node<H>>), B::Error>> + Send
{
}
impl<N, B: Backend<T, Node<Z>: Leaf<T>>, T, H: Height> OptionNodeStream<B, T, H> for N where
    N: Stream<Item = Result<(Prefix<H>, Option<B::Node<H>>), B::Error>> + Send
{
}

/// An [`OptionNodeStream`] erased to one level of type depth.
pub type BoxOptionNodeStream<'a, B, T, H> = Pin<Box<dyn OptionNodeStream<B, T, H> + 'a>>;

/// A stream of one prefix-keyed node: the seed for the stream transducers
/// that recurse over whole subtrees.
pub(super) fn one<B, T, H>(prefix: Prefix<H>, node: B::Node<H>) -> impl NodeStream<B, T, H>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
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

#[cfg(test)]
mod tests;
