use std::convert::Infallible;

use borsh::{BorshDeserialize, BorshSerialize};
use futures::stream::{self, StreamExt};

use super::{LeafStream, Node, NodeStream, Storage};
use crate::{
    Version,
    tree::typed::{
        self, Prefix,
        height::{Height, S, Z},
        node::Children,
    },
};

impl<T, H: Height> Node for typed::Node<T, H> {
    fn hash(&self) -> typed::Hash {
        self.hash()
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn ceiling(&self) -> &Version {
        self.ceiling()
    }

    fn floor(&self) -> &Version {
        self.floor()
    }
}

pub struct Local;

impl Storage for Local {
    type Node<T, H: Height> = typed::Node<T, H>;
    type Error<T, H: Height> = Infallible;

    fn branches<T, H>(children: impl NodeStream<Self, T, H>) -> impl NodeStream<Self, T, S<H>>
    where
        T: Send + Sync,
        H: Height,
        S<H>: Height,
    {
        // Children of a given parent arrive contiguously, so coalesce each run
        // of equal parent prefixes into one branch node: flush the open parent
        // when the prefix changes, then once more when the input ends. `fuse`
        // keeps the poll after that final flush well-defined.
        stream::unfold(
            (
                Box::pin(children.fuse()),
                Option::<(Prefix<S<H>>, Children<T, H>)>::None,
            ),
            |(mut children, mut open)| async move {
                while let Some(Ok((path, child))) = children.next().await {
                    let (prefix, radix) = path.pop();
                    if matches!(&open, Some((open_prefix, _)) if *open_prefix == prefix) {
                        open.as_mut().unwrap().1.insert(radix, child);
                    } else if let Some((prefix, pending)) =
                        open.replace((prefix, Children::from_iter([(radix, child)])))
                        && let Some(parent) = typed::Node::branch(pending)
                    {
                        return Some((Ok((prefix, parent)), (children, open)));
                    }
                }
                open.take().and_then(|(prefix, pending)| {
                    typed::Node::branch(pending)
                        .map(|parent| (Ok((prefix, parent)), (children, None)))
                })
            },
        )
    }

    fn children<T, H>(parents: impl NodeStream<Self, T, S<H>>) -> impl NodeStream<Self, T, H>
    where
        T: Send + Sync,
        H: Height,
        S<H>: Height,
    {
        parents.flat_map(move |Ok((prefix, node))| {
            stream::iter(
                node.into_children()
                    .into_iter()
                    .map(move |(radix, child)| Ok((prefix.push(radix), child))),
            )
        })
    }

    fn leaves<T>(leaves: impl LeafStream<Self, T>) -> impl NodeStream<Self, T, Z>
    where
        T: BorshSerialize + Send + Sync,
    {
        leaves.map(|Ok((version, message))| {
            Ok((
                typed::Path::for_leaf(&version, message.bytes()).into(),
                typed::Node::leaf(version, message),
            ))
        })
    }
}
