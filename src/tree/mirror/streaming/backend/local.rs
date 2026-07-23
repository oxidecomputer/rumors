use std::convert::Infallible;
use std::mem;
use std::pin::pin;

use async_stream::try_stream;
use futures::{StreamExt, future, stream};

use crate::{
    Version,
    message::Message,
    tree::{
        self,
        mirror::streaming::{
            Backend, Leaf, Node, Root,
            backend::{BoxNodeStream, NodeStream},
            convert::Convert,
        },
        typed::{
            self, Path, Prefix,
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

impl<T: Send + Sync + 'static, H: Height> Node<T> for typed::Node<T, H> {
    type Backend = Local;
    type Height = H;

    fn hash(&self) -> typed::Hash {
        self.hash()
    }

    fn ceiling(&self) -> &Version {
        self.ceiling()
    }

    fn floor(&self) -> &Version {
        self.floor()
    }
}

impl<T: Send + Sync + 'static> Leaf<T> for typed::Node<T, Z> {
    fn message(&self) -> &Message<T> {
        self.message()
    }

    fn leaf(version: Version, message: Message<T>) -> Self {
        Self::leaf(version, message)
    }
}

/// The in-memory backend: [`typed::Node`] handles over the crate's own tree.
///
/// Zero-sized — the nodes carry all the state — so the cloneable-handle
/// contract of [`Backend`] is satisfied by `Copy`.
#[derive(Default, Clone, Copy, Debug)]
pub struct Local;

impl Local {
    /// One in-flight `Local` reference costs one pointer, at every fan
    /// and version bound.
    ///
    /// A node is an `Arc` handle into the session-resident tree — its
    /// children, hash memo, and version bounds live in the tree, shared,
    /// not per-session — verified against the node type below; the
    /// height-typed veneer is `repr(transparent)` over that handle, so
    /// every height costs the same.
    pub(crate) fn node_bytes(_children: usize, _version_bound: usize) -> usize {
        std::mem::size_of::<typed::Node<(), Z>>()
    }
}

/// The handle really is pointer-sized: the window's per-reference price
/// rests on it.
const _: () =
    assert!(std::mem::size_of::<typed::Node<(), Z>>() == std::mem::size_of::<*const ()>());

impl<T: Send + Sync + 'static> Backend<T> for Local {
    type Node<H: Height> = typed::Node<T, H>;
    type Error = Infallible;

    fn node_bytes(children: usize, version_bound: usize) -> usize {
        Local::node_bytes(children, version_bound)
    }

    fn children<H>(
        self,
        prefix: Prefix<S<H>>,
        parent: Self::Node<S<H>>,
    ) -> impl NodeStream<Self, T, H>
    where
        H: Height,
        S<H>: Height,
    {
        let children = stream::iter(
            parent
                .into_children()
                .into_iter()
                .map(move |(radix, child)| Ok((prefix.push(radix), child))),
        );
        #[cfg(test)]
        return adversarial::stream(adversarial::Role::Children { height: H::HEIGHT }, children);
        #[cfg(not(test))]
        children
    }

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
        return adversarial::future(
            adversarial::Role::Parent {
                height: <S<H>>::HEIGHT,
            },
            parent,
        );
        #[cfg(not(test))]
        parent
    }

    fn leaves<H: Convert>(
        self,
        prefix: Prefix<H>,
        node: Self::Node<H>,
    ) -> impl NodeStream<Self, T, Z> {
        // The default level-by-level explosion pays an allocation per
        // *virtual* level — ruinous for path-compressed spines. In-memory
        // nodes walk their own leaves directly, skipping compressed spans.
        let leaves = stream::iter(node.leaves(&prefix).map(Ok));
        #[cfg(test)]
        return adversarial::stream(adversarial::Role::Children { height: H::HEIGHT }, leaves);
        #[cfg(not(test))]
        leaves
    }

    fn assemble<'a, H: Convert>(
        self,
        leaves: BoxNodeStream<'a, Self, T, Z>,
    ) -> impl NodeStream<Self, T, H> + 'a {
        // The bulk counterpart of `leaves`: buffer each maximal
        // same-prefix run and build its subtree in one pass, rather than
        // folding it up one virtual level at a time. The buffered run is
        // transient state for a subtree this in-memory backend is about to
        // hold whole anyway, so the streaming session's memory story is
        // unchanged.
        let assembled = try_stream! {
            let mut leaves = pin!(leaves);
            let mut current: Option<Prefix<H>> = None;
            let mut run: Vec<(Prefix<Z>, typed::Node<T, Z>)> = Vec::new();
            while let Some(item) = leaves.next().await {
                let (prefix, leaf) = item?;
                let target = Prefix::<H>::containing(&Path::from(prefix));
                if current != Some(target)
                    && let Some(finished) = current.replace(target)
                {
                    yield (
                        finished,
                        typed::Node::from_sorted_leaves(&finished, mem::take(&mut run)),
                    );
                }
                run.push((prefix, leaf));
            }
            if let Some(finished) = current {
                yield (finished, typed::Node::from_sorted_leaves(&finished, run));
            }
        };
        #[cfg(test)]
        return adversarial::stream(adversarial::Role::Parent { height: H::HEIGHT }, assembled);
        #[cfg(not(test))]
        assembled
    }
}

// `tree::Root` is exactly the `Local` instance of the session's generic
// `Root`: the same (ceiling, optional root node) pair, concretely typed.

impl<T: Send + Sync + 'static> From<tree::Root<T>> for Root<Local, T> {
    fn from(root: tree::Root<T>) -> Self {
        let tree::Root { ceiling, root } = root;
        Root { ceiling, root }
    }
}

impl<T: Send + Sync + 'static> From<Root<Local, T>> for tree::Root<T> {
    fn from(root: Root<Local, T>) -> Self {
        let Root { ceiling, root } = root;
        tree::Root { ceiling, root }
    }
}
