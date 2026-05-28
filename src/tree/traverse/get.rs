use std::future::Future;
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
        Get::get(node, Prefix::new(), paths, &mut |k, v: &Version<P>, m: &Arc<T>| {
            gotten.push((k, v.clone(), m.clone()));
            std::future::ready(())
        })
        .await;
        gotten
    })
}

pub trait Get: Height {
    async fn get<P, T, F, Fut>(
        node: Option<Node<P, T, Self>>,
        prefix: Prefix<Self>,
        paths: Paths<Self>,
        with_gotten: &mut F,
    ) where
        P: Clone + Ord + AsRef<[u8]>,
        F: FnMut(Key, &Version<P>, &Arc<T>) -> Fut,
        Fut: Future<Output = ()>;
}

impl<H: Get> Get for S<H>
where
    S<H>: Height,
{
    async fn get<P, T, F, Fut>(
        node: Option<Node<P, T, Self>>,
        prefix: Prefix<Self>,
        paths: Paths<Self>,
        with_gotten: &mut F,
    ) where
        P: Clone + Ord + AsRef<[u8]>,
        F: FnMut(Key, &Version<P>, &Arc<T>) -> Fut,
        Fut: Future<Output = ()>,
    {
        let Some(node) = node else {
            return;
        };

        if let Paths::Selected(paths) = paths {
            // Group the paths by their first element. Collected eagerly into
            // an owned `Vec` before the recursive await loop: `ChunkBy`'s
            // interior `RefCell`/`Cell` would otherwise make the surrounding
            // `async fn`'s state machine `!Send`. See `act.rs` for the same
            // pattern (and for the boxed-recursion comment that applies here
            // too).
            let by_radix: Vec<(u8, Vec<_>)> = paths
                .into_iter()
                .map(|path| {
                    let (child, path) = path.pop();
                    (child, path)
                })
                .sorted_by_key(|(child, _)| *child)
                .chunk_by(|(child, _)| *child)
                .into_iter()
                .map(|(radix, group)| (radix, group.map(|(_, path)| path).collect()))
                .collect();

            // Decompose the node into its children
            let mut children = node.into_children();

            for (radix, child_paths) in by_radix {
                Box::pin(Get::get(
                    children.remove(&radix),
                    prefix.push(radix),
                    Paths::Selected(child_paths),
                    with_gotten,
                ))
                .await;
            }
        } else {
            // Get all the paths
            for (radix, child) in node.into_children() {
                Box::pin(Get::get(
                    Some(child),
                    prefix.push(radix),
                    Paths::All,
                    with_gotten,
                ))
                .await;
            }
        }
    }
}

impl Get for Z {
    async fn get<P, T, F, Fut>(
        node: Option<Node<P, T, Self>>,
        prefix: Prefix<Self>,
        paths: Paths<Self>,
        with_gotten: &mut F,
    ) where
        P: Clone + Ord + AsRef<[u8]>,
        F: FnMut(Key, &Version<P>, &Arc<T>) -> Fut,
        Fut: Future<Output = ()>,
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
