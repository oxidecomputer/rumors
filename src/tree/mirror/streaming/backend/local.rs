use std::convert::Infallible;

use futures::stream::{self, StreamExt};

use crate::{
    Version,
    message::Message,
    tree::{
        self,
        typed::{
            self,
            height::{Height, S, Z},
            node::Children,
        },
    },
};

use super::{Backend, Leaf, Material, Node, NodeStream, Root};

impl<T, H: Height> Node for typed::Node<T, H> {
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

impl<T> Leaf<T> for typed::Node<T, Z> {
    // Delegates to the same inherent `ceiling` the `Node` impl uses,
    // keeping `Leaf::version` and `Node::ceiling` one method in disguise
    // (the coherence contract on `Leaf::version`).
    fn version(&self) -> &Version {
        self.ceiling()
    }

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

impl<T: Send + Sync + 'static> Backend<T> for Local {
    type Materialized = Material;
    type Node<H: Height> = typed::Node<T, H>;
    type Error = Infallible;

    fn parents<H>(self, children: impl NodeStream<Self, T, H>) -> impl NodeStream<Self, T, S<H>>
    where
        H: Height,
        S<H>: Height,
    {
        // Children of a given parent arrive contiguously, so coalesce each run
        // of equal parent prefixes into one branch node: flush the open parent
        // when the prefix changes, then once more when the input ends. `fuse`
        // keeps the poll after that final flush well-defined.
        stream::unfold(
            // Our state is the pair of the children stream (which we'll pull
            // from) and an optional pair of our current prefix and its
            // children.
            (Box::pin(children.fuse()), None::<(_, Children<_, _>)>),
            |(mut children, mut current)| async move {
                // Loop internally to the single output future, pulling children...
                while let Some(Ok((path, child))) = children.next().await
                    && let (prefix, radix) = path.pop()
                {
                    if let Some((current_prefix, current_children)) = &mut current
                        && *current_prefix == prefix
                    {
                        // If the current prefix matches, append to children:
                        current_children.insert(radix, child);
                    } else if let Some((finished_prefix, finished_children)) =
                        current.replace((prefix, Children::from_iter([(radix, child)])))
                        && let Some(finished_parent) = typed::Node::branch(finished_children)
                    {
                        // Otherwise, pull out a finished prefix and children and
                        // construct the corresponding parent output:
                        let output = (finished_prefix, finished_parent);
                        return Some((Ok(output), (children, current)));
                    }
                }

                // When there are no more children in the input stream, flush any remaining
                // single buffered parent:
                current
                    .take()
                    .and_then(|(current_prefix, current_children)| {
                        typed::Node::branch(current_children)
                            .map(|parent| (Ok((current_prefix, parent)), (children, None)))
                    })
            },
        )
    }

    fn children<H>(self, parents: impl NodeStream<Self, T, S<H>>) -> impl NodeStream<Self, T, H>
    where
        H: Height,
        S<H>: Height,
    {
        // We box the stream so that traversing down the whole 32-deep tree
        // does not build a gigantic stream type.
        Box::pin(parents.fuse().flat_map(move |Ok((prefix, node))| {
            stream::iter(
                node.into_children()
                    .into_iter()
                    .map(move |(radix, child)| Ok((prefix.push(radix), child))),
            )
        }))
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
