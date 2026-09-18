use std::convert::Infallible;
use std::mem;
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
            convert::Convert,
        },
        typed::{
            self, LeafRun, Path, Prefix,
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
impl<H: Height> Node for typed::Node<H> {
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

    /// Return the largest encoded version bound in the subtree.
    fn version_bytes(&self) -> usize {
        self.version_bytes()
    }
}

/// Read comparison metadata without a height marker.
impl ErasedNode for typed::untyped::Node {
    /// Borrow the subtree's ordered version bounds.
    fn span(&self) -> Span<'_> {
        self.span()
    }

    /// Read the memoized subtree digest.
    fn hash(&self) -> typed::Hash {
        self.hash()
    }

    /// Return the stored leaf count.
    fn len(&self) -> usize {
        self.len()
    }
}

/// Store leaf payloads directly in the in-memory tree.
impl Leaf for typed::Node<Z> {
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

/// The in-memory backend: [`typed::Node`] handles over the crate's own tree.
///
/// Zero-sized — the nodes carry all the state — so the cloneable-handle
/// contract of [`Backend`] is satisfied by `Copy`.
#[derive(Default, Clone, Copy, Debug)]
pub struct Local;

// The handle is pointer-sized, as required by the window's per-reference price.
const _: () = assert!(std::mem::size_of::<typed::Node<Z>>() == std::mem::size_of::<*const ()>());

/// Implement backend operations with the crate's in-memory typed tree.
impl Backend for Local {
    /// Use the typed in-memory node at every height.
    type Node<H: Height> = typed::Node<H>;
    /// Use the untyped node representation shared by every height.
    ///
    /// The typed node is a phantom tag over this value, so erasure and
    /// restoration are field moves.
    type Erased = typed::untyped::Node;
    /// In-memory tree operations cannot fail.
    type Error = Infallible;

    /// Remove a node's phantom height tag.
    fn erase<H: Height>(node: Self::Node<H>) -> Self::Erased {
        node.into_untyped()
    }

    /// Restore a node's phantom height tag.
    fn assume<H: Height>(erased: Self::Erased) -> Self::Node<H> {
        typed::Node::from_untyped(erased)
    }

    /// Price one in-flight node reference as one pointer.
    ///
    /// The `Arc` handle has the same layout at every height. Its children,
    /// hash, and version bounds belong to the tree rather than the session,
    /// so neither argument changes the price.
    fn node_bytes(_children: usize, _version_bound: usize) -> usize {
        std::mem::size_of::<typed::Node<Z>>()
    }

    /// Stream one branch's children in radix order.
    fn children<H>(self, prefix: Prefix<S<H>>, parent: Self::Node<S<H>>) -> impl NodeStream<Self, H>
    where
        H: Height,
        S<H>: Height,
    {
        let children = stream::iter(
            parent
                .child_iter()
                .map(move |(radix, child)| Ok((prefix.push(radix), child))),
        );
        #[cfg(test)]
        return adversarial::stream(children);
        #[cfg(not(test))]
        children
    }

    /// Assemble one in-memory branch from its surviving children.
    fn parent<H>(
        self,
        _prefix: Prefix<S<H>>,
        children: Vec<(u8, Option<Self::Node<H>>)>,
    ) -> impl Future<Output = Result<Option<Self::Node<S<H>>>, Self::Error>> + Send
    where
        H: Height,
        S<H>: Height,
    {
        // A deleted child simply doesn't join the reassembly, and deleting
        // every child deletes the parent: `branch` of the empty set is `None`.
        let parent = future::ready(Ok(typed::Node::branch(
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
        // The default level-by-level explosion pays an allocation per
        // virtual level, which is costly for compressed paths. In-memory
        // nodes walk their own leaves directly, skipping compressed spans.
        let leaves = stream::iter(node.leaves(&prefix).map(Ok));
        #[cfg(test)]
        return adversarial::stream(leaves);
        #[cfg(not(test))]
        leaves
    }

    /// Build each same-prefix leaf run as one in-memory subtree.
    fn assemble<'a, H: Convert>(
        self,
        leaves: BoxNodeStream<'a, Self, Z>,
    ) -> impl NodeStream<Self, H> + 'a {
        // Buffer one maximal same-prefix run in the bulk builder's own entry
        // format, then consume it directly. This avoids both the default
        // level-by-level assembly and a second run-sized conversion buffer.
        // The remaining transient grows with the subtree being inserted and
        // is released as soon as that subtree joins the replica.
        let assembled = try_stream! {
            let mut leaves = pin!(leaves);
            let mut current: Option<Prefix<H>> = None;
            let mut run = LeafRun::new();
            while let Some(item) = leaves.next().await {
                let (prefix, leaf) = item?;
                let target = Prefix::<H>::containing(&Path::from(prefix));
                if current != Some(target)
                    && let Some(finished) = current.replace(target)
                {
                    yield (
                        finished,
                        mem::take(&mut run).build(&finished),
                    );
                }
                run.push(prefix, leaf);
            }
            if let Some(finished) = current {
                yield (finished, run.build(&finished));
            }
        };
        #[cfg(test)]
        return adversarial::stream(assembled);
        #[cfg(not(test))]
        assembled
    }
}

// `tree::Root` is exactly the `Local` instance of the session's generic
// `Root`: the same (ceiling, optional root node) pair, concretely typed.

/// Convert the crate's in-memory root into the backend-generic form.
impl From<tree::Root> for Root<Local> {
    /// Move the ceiling and optional root node without changing them.
    fn from(root: tree::Root) -> Self {
        let tree::Root { ceiling, node } = root;
        Root {
            ceiling,
            root: node,
        }
    }
}

/// Convert the backend-generic in-memory root into the crate's root type.
impl From<Root<Local>> for tree::Root {
    /// Move the ceiling and optional root node without changing them.
    fn from(root: Root<Local>) -> Self {
        let Root { ceiling, root } = root;
        tree::Root {
            ceiling,
            node: root,
        }
    }
}
