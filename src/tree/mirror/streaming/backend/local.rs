use std::convert::Infallible;

use futures::{future, stream};

use crate::{
    Version,
    message::Message,
    tree::{
        self,
        mirror::streaming::{Backend, Leaf, Node, Root, backend::NodeStream},
        typed::{
            self, Prefix,
            height::{Height, S, Z},
        },
    },
};

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

impl<T: Send + Sync + 'static> Backend<T> for Local {
    type Node<H: Height> = typed::Node<T, H>;
    type Error = Infallible;

    fn children<H>(
        self,
        prefix: Prefix<S<H>>,
        parent: Self::Node<S<H>>,
    ) -> impl NodeStream<Self, T, H>
    where
        H: Height,
        S<H>: Height,
    {
        stream::iter(
            parent
                .into_children()
                .into_iter()
                .map(move |(radix, child)| Ok((prefix.push(radix), child))),
        )
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
        future::ready(Ok(typed::Node::branch(
            children
                .into_iter()
                .filter_map(|(radix, child)| Some((radix, child?)))
                .collect(),
        )))
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
