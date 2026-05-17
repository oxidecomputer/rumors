use crate::{message::Message, tree::key::Key, version::Version};

use super::typed::*;
use height::{Height, Root, S, Z};
use prefix::Prefix;

/// Perform a batch lookup in the tree by version vector, returning a list of
/// [`Bytes`] and their accompanying paths for all versioned leaves which are
/// *unknown* relative to the specified version.
///
/// The unknown set is the set of leaves necessary to communicate to a
/// counterparty who has this version vector, so that their tree will become a
/// (non-strict) superset of yours.
pub fn unknown<P, T>(
    node: Option<Node<P, T, Root>>,
    known: &Version<P>,
    with_unknown: &mut impl FnMut(&Version<P>, Key, &Message<T>),
) -> Option<Node<P, T, Root>>
where
    P: Clone + Ord + AsRef<[u8]>,
{
    Unknown::unknown(node, Prefix::new(), known, with_unknown)
}

pub trait Unknown: Height {
    fn unknown<P, T>(
        node: Option<Node<P, T, Self>>,
        prefix: Prefix<Self>,
        known: &Version<P>,
        with_unknown: &mut impl FnMut(&Version<P>, Key, &Message<T>),
    ) -> Option<Node<P, T, Self>>
    where
        P: Clone + Ord + AsRef<[u8]>;
}

impl<H: Unknown> Unknown for S<H>
where
    S<H>: Height,
{
    fn unknown<P, T>(
        node: Option<Node<P, T, Self>>,
        prefix: Prefix<Self>,
        known: &Version<P>,
        with_unknown: &mut impl FnMut(&Version<P>, Key, &Message<T>),
    ) -> Option<Node<P, T, Self>>
    where
        P: Clone + Ord + AsRef<[u8]>,
    {
        // If the node doesn't exist, we can't return information about it
        let node = node?;

        // If the node is causally prior or at the known version vector, it's
        // already known (and so are all its children, since they are always in
        // the causal past or present of their parent), so don't return anything
        if node.version() <= known {
            return None;
        }

        // Recursively process each child, re-assembling only the unknown children
        Node::branch(
            node.into_children()
                .into_iter()
                .flat_map(|(radix, child)| {
                    Unknown::unknown(Some(child), prefix.push(radix), known, with_unknown)
                        .map(|child| (radix, child))
                })
                .collect(),
        )
    }
}

impl Unknown for Z {
    fn unknown<P, T>(
        node: Option<Node<P, T, Self>>,
        prefix: Prefix,
        known: &Version<P>,
        with_unknown: &mut impl FnMut(&Version<P>, Key, &Message<T>),
    ) -> Option<Node<P, T, Self>>
    where
        P: Clone + Ord + AsRef<[u8]>,
    {
        // If the node doesn't exist, we can't return information about it
        let node = node?;

        // If the node is causally prior or at the known version vector, it's
        // already known, so don't return anything
        if node.version() <= known {
            return None;
        }

        // Otherwise, the node is causally unknown, so return its information
        with_unknown(node.version(), Path::from(prefix).into(), node.message());
        Some(node)
    }
}
