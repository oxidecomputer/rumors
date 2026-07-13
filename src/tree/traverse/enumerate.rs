//! Enumerate all the leaves of a node, lazily.

use crate::{Version, message::Message};

use super::typed::*;
use height::{Height, S, Z};

/// The inductive step of the filter, implemented per [`Height`]: each level
/// prunes by its memoized ceiling/floor before descending.
pub trait Enumerate: Height {
    /// Filter this subtree down to the nodes a counterparty at `known` is
    /// missing, honoring deletions: a node causally `<=` `known` is already
    /// known there (or was deleted there) and drops out.
    fn enumerate<'a, T>(
        prefix: Prefix<Self>,
        node: Node<T, Self>,
    ) -> Box<dyn Iterator<Item = (Prefix, Version, Message<T>)> + 'a>
    where
        T: Send + Sync + 'a;
}

impl<H: Enumerate> Enumerate for S<H>
where
    S<H>: Height,
{
    fn enumerate<'a, T>(
        prefix: Prefix<Self>,
        node: Node<T, Self>,
    ) -> Box<dyn Iterator<Item = (Prefix, Version, Message<T>)> + 'a>
    where
        T: Send + Sync + 'a,
    {
        Box::new(
            node.into_children()
                .into_iter()
                .flat_map(move |(radix, child)| H::enumerate(prefix.push(radix), child)),
        )
    }
}

impl Enumerate for Z {
    fn enumerate<'a, T>(
        prefix: Prefix<Self>,
        node: Node<T, Self>,
    ) -> Box<dyn Iterator<Item = (Prefix, Version, Message<T>)> + 'a>
    where
        T: Send + Sync + 'a,
    {
        Box::new(std::iter::once((
            prefix,
            node.ceiling().clone(),
            node.message().clone(),
        )))
    }
}
