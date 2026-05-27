use crate::{message::Message, tree::key::Key, version::Version};

use super::typed::*;
use height::{Height, Root, S, Z};
use imbl::OrdMap;
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
    with_unknown: &mut impl FnMut(Key, &Version<P>, &Message<T>),
) -> Option<Node<P, T, Root>>
where
    P: Clone + Ord + AsRef<[u8]>,
{
    pollster::block_on(Unknown::unknown(
        node,
        Prefix::new(),
        known,
        &mut async |k, v, m| with_unknown(k, v, m),
    ))
}

pub trait Unknown: Height {
    async fn unknown<P, T>(
        node: Option<Node<P, T, Self>>,
        prefix: Prefix<Self>,
        known: &Version<P>,
        with_unknown: &mut impl AsyncFnMut(Key, &Version<P>, &Message<T>),
    ) -> Option<Node<P, T, Self>>
    where
        P: Clone + Ord + AsRef<[u8]>;
}

impl<H: Unknown> Unknown for S<H>
where
    S<H>: Height,
{
    async fn unknown<P, T>(
        node: Option<Node<P, T, Self>>,
        prefix: Prefix<Self>,
        known: &Version<P>,
        with_unknown: &mut impl AsyncFnMut(Key, &Version<P>, &Message<T>),
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
        Node::branch({
            let mut children = OrdMap::new();
            for (radix, child) in node.into_children() {
                if let Some(child) =
                    Unknown::unknown(Some(child), prefix.push(radix), known, with_unknown).await
                {
                    children.insert(radix, child);
                }
            }
            children
        })
    }
}

impl Unknown for Z {
    async fn unknown<P, T>(
        node: Option<Node<P, T, Self>>,
        prefix: Prefix,
        known: &Version<P>,
        with_unknown: &mut impl AsyncFnMut(Key, &Version<P>, &Message<T>),
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
        with_unknown(Path::from(prefix).into(), node.version(), node.message()).await;
        Some(node)
    }
}
