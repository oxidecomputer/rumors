use std::convert::Infallible;
use std::pin::pin;

use async_stream::try_stream;
use futures::{StreamExt, future, stream};

use before::Span;

use crate::{
    Version,
    message::Message,
    tree::{
        self,
        mirror::streaming::{
            Backend, ErasedNode, Leaf, Node, Root,
            backend::{BoxNodeStream, NodeStream},
            byte_set::ByteSet,
            convert::Convert,
            holding::{Holding, held_by},
        },
        typed::{
            self, ErasedPrefix, Path, Prefix, SortedRun,
            height::{Height, S, Z},
        },
    },
};

#[cfg(test)]
mod adversarial;
#[cfg(test)]
mod tests;
#[cfg(test)]
pub use adversarial::with_schedule;

/// Read aggregate metadata from the in-memory tree's stored memos.
impl<H: Height> Node for typed::View<H> {
    type Backend = Local;
    type Height = H;

    /// Borrow bounds ordered by construction; a leaf's bounds coincide.
    fn span(&self) -> Span<'_> {
        self.span()
    }

    /// Return the stored leaf count.
    fn len(&self) -> usize {
        self.len()
    }

    /// Derive the grandchild count from the stored aggregate and the path.
    fn grandchild_count(&self) -> usize {
        self.grandchild_count()
    }

    /// Return the largest encoded version bound in the subtree.
    fn version_bytes(&self) -> usize {
        self.version_bytes()
    }
}

/// Read comparison metadata without a height marker, viewing the stored
/// node from the coordinate each query names.
impl ErasedNode for typed::ErasedView {
    /// Borrow the subtree's ordered version bounds.
    fn span(&self) -> Span<'_> {
        self.span()
    }

    /// Read the subtree digest at `at` under `key`: a whole node's memo,
    /// computed on first read, or a peeled view's, computed on each read.
    ///
    /// For the session-budget meter, computing it is hashing's own work
    /// ([`Holding::Hashing`]), whichever place's code read it.
    fn hash(&self, at: &ErasedPrefix, key: &typed::HashKey) -> typed::Hash {
        held_by(Holding::Hashing, || self.at(at).hash(key))
    }

    /// Return the stored leaf count.
    fn len(&self) -> usize {
        self.len()
    }

    /// Derive the grandchild count at `at` from the stored aggregate and
    /// the visible path.
    fn grandchild_count(&self, at: &ErasedPrefix) -> usize {
        self.at(at).grandchild_count()
    }

    /// Count the children listing this node at `at` yields, from its
    /// shape.
    fn child_count(&self, at: &ErasedPrefix) -> usize {
        self.at(at).child_count()
    }

    /// Collect the radices listing this node at `at` yields, from its fan
    /// or its visible path, in one pass with no handle cloned.
    fn child_radices(&self, at: &ErasedPrefix) -> ByteSet {
        self.at(at).child_radices().collect()
    }
}

/// Store leaf payloads directly in the in-memory tree.
impl Leaf for typed::View<Z> {
    /// Borrow the stored message.
    fn message(&self) -> &Message {
        self.message()
    }

    /// Construct a leaf without asynchronous storage work.
    ///
    /// The handle owns the payload and the destination tree is already
    /// resident, so construction completes immediately.
    async fn leaf(version: Version, message: Message) -> Result<Self, Infallible> {
        Ok(Self::leaf(version, message))
    }
}

/// The in-memory backend: [`typed::View`] handles over the crate's own
/// tree, each the size of a stored node's handle.
///
/// Zero-sized — the handles carry all the state — so the cloneable-handle
/// contract of [`Backend`] is satisfied by `Copy`. Its nodes are immutable
/// and shared, and it takes no locks, so its operations never wait on other
/// session work.
///
/// Each handle is a view of a stored node from one level of its compressed
/// path, so listing a compressed node's one child shares the node rather
/// than copying it, and listing allocates nothing beyond the handles it
/// yields. Reassembly builds a node from a view only where the view joins a
/// new branch. A handle holds the stored node alone: the level it views
/// the node from is its height, a type at [`Node`](Backend::Node), and the
/// coordinate the session keeps beside it at [`Erased`](Backend::Erased).
#[derive(Default, Clone, Copy, Debug)]
pub struct Local;

/// Implement backend operations with the crate's in-memory typed tree.
impl Backend for Local {
    /// Use the typed in-memory view at every height.
    type Node<H: Height> = typed::View<H>;
    /// Use the erased view: the stored node, shared by every height.
    ///
    /// The typed view is a phantom tag over the same stored node, so
    /// erasure is a field move, and restoration a move, after checking the
    /// coordinate in debug builds.
    type Erased = typed::ErasedView;
    /// In-memory tree operations cannot fail.
    type Error = Infallible;

    /// Remove a node's phantom height tag.
    fn erase<H: Height>(node: Self::Node<H>) -> Self::Erased {
        node.erase()
    }

    /// Restore a node's phantom height tag from its coordinate, checking
    /// the coordinate against the stored node's path bytes in debug builds
    /// ([`typed::View::placed`]).
    fn assume<H: Height>(at: &Prefix<H>, erased: Self::Erased) -> Self::Node<H> {
        typed::View::placed(at, erased)
    }

    /// Stream one branch's children in radix order.
    fn children<H>(self, prefix: Prefix<S<H>>, parent: Self::Node<S<H>>) -> impl NodeStream<Self, H>
    where
        H: Height,
        S<H>: Height,
    {
        let children = stream::iter(
            parent
                .children()
                .map(move |(radix, child)| Ok((prefix.push(radix), child))),
        );
        #[cfg(test)]
        return adversarial::stream(children);
        #[cfg(not(test))]
        children
    }

    /// Assemble one in-memory branch from its surviving children, returning
    /// the original node whole when they are its own.
    fn parent<H>(
        self,
        _prefix: Prefix<S<H>>,
        original: Option<Self::Node<S<H>>>,
        children: Vec<(u8, Option<Self::Node<H>>)>,
    ) -> impl Future<Output = Result<Option<Self::Node<S<H>>>, Self::Error>> + Send
    where
        H: Height,
        S<H>: Height,
    {
        // A deleted child simply doesn't join the reassembly, and deleting
        // every child deletes the parent: `rebuild` of the empty set is
        // `None`.
        let parent = future::ready(Ok(typed::View::rebuild(
            original,
            children
                .into_iter()
                .filter_map(|(radix, child)| Some((radix, child?)))
                .collect(),
        )));
        #[cfg(test)]
        return adversarial::future(parent);
        #[cfg(not(test))]
        parent
    }

    /// Walk the in-memory node's compressed representation directly.
    fn leaves<H: Convert>(
        self,
        prefix: Prefix<H>,
        node: Self::Node<H>,
    ) -> impl NodeStream<Self, Z> {
        // The default explodes level by level, one listing per level of a
        // compressed path. A view walks its stored node's leaves directly,
        // skipping compressed spans.
        let leaves = stream::iter(node.leaves(&prefix).map(Ok));
        #[cfg(test)]
        return adversarial::stream(leaves);
        #[cfg(not(test))]
        leaves
    }

    /// Build each same-prefix leaf run as one in-memory subtree, one leaf
    /// at a time.
    ///
    /// Each run builds over its open right spine ([`SortedRun`]), so a
    /// subtree of any size assembles in transient memory bounded by one fan
    /// per path byte, and each node is built once, with its whole
    /// compressed prefix: one node per branch point and one per leaf. The
    /// default level-by-level fold would instead rebuild a node at every
    /// virtual level a compressed path spans.
    fn assemble<'a, H: Convert>(
        self,
        leaves: BoxNodeStream<'a, Self, Z>,
    ) -> impl NodeStream<Self, H> + 'a {
        let assembled = try_stream! {
            let mut leaves = pin!(leaves);
            let mut run: Option<SortedRun<H>> = None;
            while let Some(item) = leaves.next().await {
                let (path, leaf) = item?;
                let target = Prefix::<H>::containing(&Path::from(path));
                if run.as_ref().is_none_or(|run| run.prefix() != target)
                    && let Some(finished) = run.replace(SortedRun::new(target))
                {
                    yield viewed(finished.finish());
                }
                // A decoded leaf is a whole view of the node `leaf` built,
                // so building its node returns that node.
                run.as_mut()
                    .expect("a run is open once a leaf has arrived")
                    .push(path, leaf.into_node());
            }
            if let Some(finished) = run {
                yield viewed(finished.finish());
            }
        };
        #[cfg(test)]
        return adversarial::stream(Box::pin(assembled));
        #[cfg(not(test))]
        assembled
    }
}

/// View a finished run's subtree whole, at its prefix.
fn viewed<H: Height>((prefix, node): (Prefix<H>, typed::Node<H>)) -> (Prefix<H>, typed::View<H>) {
    (prefix, typed::View::from(node))
}

// `tree::Root` corresponds to the `Local` instance of the session's
// generic `Root`: the same (ceiling, optional root) pair, with the root
// stored as a node rather than viewed.

/// Convert the crate's in-memory root into the backend-generic form.
impl From<tree::Root> for Root<Local> {
    /// Move the ceiling, and view the root node whole.
    fn from(root: tree::Root) -> Self {
        let tree::Root { ceiling, node } = root;
        Root {
            ceiling,
            root: node.map(typed::View::from),
        }
    }
}

/// Convert the backend-generic in-memory root into the crate's root type.
impl From<Root<Local>> for tree::Root {
    /// Move the ceiling, and build the root's node from its view.
    ///
    /// A root has no path above it to hide, so its view is whole, and
    /// building its node returns the stored node itself.
    fn from(root: Root<Local>) -> Self {
        let Root { ceiling, root } = root;
        tree::Root {
            ceiling,
            node: root.map(typed::View::into_node),
        }
    }
}
