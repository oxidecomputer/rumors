use itertools::Itertools;

use crate::Message;

use super::typed::*;
use height::{Height, Root, S, Z};

/// Perform a batch lookup of paths in the tree, returning a list of [`Bytes`]
/// which are stored at these paths.
///
/// Values are returned in arbitrary order, not necessarily in the order of the
/// specified paths.
pub fn get<P, T>(node: Option<&Node<P, T, Root>>, paths: Vec<Path>) -> Vec<Message<T>>
where
    T: Clone,
    P: Clone + Ord + AsRef<[u8]>,
{
    let mut gotten = Vec::new();
    Get::get(node, paths, &mut |message| gotten.push(message.clone()));
    gotten
}

pub trait Get: Height {
    fn get<P, T>(
        node: Option<&Node<P, T, Self>>,
        paths: Vec<Path<Self>>,
        with_gotten: &mut impl FnMut(&Message<T>),
    ) where
        T: Clone,
        P: Clone + Ord + AsRef<[u8]>;
}

impl<H: Get> Get for S<H>
where
    S<H>: Height,
{
    fn get<P, T>(
        node: Option<&Node<P, T, Self>>,
        paths: Vec<Path<Self>>,
        with_gotten: &mut impl FnMut(&Message<T>),
    ) where
        T: Clone,
        P: Clone + Ord + AsRef<[u8]>,
    {
        let Some(node) = node else {
            return;
        };

        // Group the paths by their first element
        let by_radix = paths
            .into_iter()
            .map(|path| {
                let (child, path) = path.pop();
                (child, path)
            })
            .sorted_by_key(|(child, _)| *child)
            .chunk_by(|(child, _)| *child);

        // Decompose the node into its children
        let children = node.clone().into_children();

        // Recursively look up each radix group in the corresponding child
        for (radix, group) in by_radix.into_iter() {
            let child_paths: Vec<_> = group.map(|(_, path)| path).collect();
            Get::get(children.get(&radix), child_paths, with_gotten);
        }
    }
}

impl Get for Z {
    fn get<P, T>(
        node: Option<&Node<P, T, Self>>,
        paths: Vec<Path<Self>>,
        with_gotten: &mut impl FnMut(&Message<T>),
    ) where
        T: Clone,
        P: Clone + Ord + AsRef<[u8]>,
    {
        let Some(node) = node else {
            return;
        };

        let leaf = node.value();
        if paths.is_empty() {
            return;
        } else {
            with_gotten(leaf);
        }
    }
}
