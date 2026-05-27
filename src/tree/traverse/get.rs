use std::sync::Arc;

use itertools::Itertools;

use crate::{tree::key::Key, version::Version};

use super::typed::*;
use height::{Height, Root, S, Z};

#[derive(Clone, Debug)]
pub enum Paths<H = Root>
where
    H: Height,
{
    All,
    Selected(Vec<Path<H>>),
}

/// Perform a batch lookup of paths in the tree, returning a list of versioned,
/// keyed messages which are stored at these paths.
///
/// Values are returned in arbitrary order, not necessarily in the order of the
/// specified paths.
pub fn get<P, T>(node: Option<Node<P, T, Root>>, paths: Paths) -> Vec<(Key, Version<P>, Arc<T>)>
where
    P: Clone + Ord + AsRef<[u8]>,
{
    pollster::block_on(async {
        let mut gotten = Vec::new();
        Get::get(node, Prefix::new(), paths, &mut async |k, v, m| {
            gotten.push((k, v.clone(), m.clone()))
        })
        .await;
        gotten
    })
}

pub trait Get: Height {
    async fn get<P, T>(
        node: Option<Node<P, T, Self>>,
        prefix: Prefix<Self>,
        paths: Paths<Self>,
        with_gotten: &mut impl AsyncFnMut(Key, &Version<P>, &Arc<T>),
    ) where
        P: Clone + Ord + AsRef<[u8]>;
}

impl<H: Get> Get for S<H>
where
    S<H>: Height,
{
    async fn get<P, T>(
        node: Option<Node<P, T, Self>>,
        prefix: Prefix<Self>,
        paths: Paths<Self>,
        with_gotten: &mut impl AsyncFnMut(Key, &Version<P>, &Arc<T>),
    ) where
        P: Clone + Ord + AsRef<[u8]>,
    {
        let Some(node) = node else {
            return;
        };

        if let Paths::Selected(paths) = paths {
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
            let mut children = node.into_children();

            // Recursively look up each radix group in the corresponding child
            for (radix, group) in by_radix.into_iter() {
                let child_paths: Vec<_> = group.map(|(_, path)| path).collect();
                Get::get(
                    children.remove(&radix),
                    prefix.push(radix),
                    Paths::Selected(child_paths),
                    with_gotten,
                )
                .await;
            }
        } else {
            // Get all the paths
            for (radix, child) in node.into_children() {
                Get::get(Some(child), prefix.push(radix), Paths::All, with_gotten).await
            }
        }
    }
}

impl Get for Z {
    async fn get<P, T>(
        node: Option<Node<P, T, Self>>,
        prefix: Prefix<Self>,
        paths: Paths<Self>,
        with_gotten: &mut impl AsyncFnMut(Key, &Version<P>, &Arc<T>),
    ) where
        P: Clone + Ord + AsRef<[u8]>,
    {
        let Some(node) = node else {
            return;
        };

        if let Paths::Selected(paths) = paths
            && paths.is_empty()
        {
            // Do nothing if the path doesn't match
        } else {
            with_gotten(prefix.into(), node.version(), node.message().as_ref()).await;
        }
    }
}
