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
    Get::get(node, paths)
}

pub trait Get: Height {
    fn get<P, T>(node: Option<&Node<P, T, Self>>, paths: Vec<Path<Self>>) -> Vec<Message<T>>
    where
        T: Clone,
        P: Clone + Ord + AsRef<[u8]>;
}

impl<H: Get> Get for S<H>
where
    S<H>: Height,
{
    fn get<P, T>(node: Option<&Node<P, T, Self>>, paths: Vec<Path<Self>>) -> Vec<Message<T>>
    where
        T: Clone,
        P: Clone + Ord + AsRef<[u8]>,
    {
        let Some(node) = node else {
            return Vec::new();
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
        by_radix
            .into_iter()
            .flat_map(|(radix, group)| {
                let child_paths: Vec<_> = group.map(|(_, path)| path).collect();
                Get::get(children.get(&radix), child_paths)
            })
            .collect()
    }
}

impl Get for Z {
    fn get<P, T>(node: Option<&Node<P, T, Self>>, paths: Vec<Path<Self>>) -> Vec<Message<T>>
    where
        T: Clone,
        P: Clone + Ord + AsRef<[u8]>,
    {
        let Some(node) = node else {
            return Vec::new();
        };

        let leaf = node.value().clone();
        if paths.is_empty() {
            vec![]
        } else {
            vec![leaf.clone()]
        }
    }
}
