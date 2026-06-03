use std::future::Future;
use std::pin::Pin;
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
pub fn get<T>(node: Option<Node<T, Root>>, paths: Paths) -> Vec<(Key, Version, Arc<T>)>
where
    T: Send + Sync,
{
    pollster::block_on(async {
        let mut gotten = Vec::new();
        Get::get(
            node,
            Prefix::new(),
            paths,
            &mut |k, v: &Version, m: &Arc<T>| {
                gotten.push((k, v.clone(), m.clone()));
                std::future::ready(())
            },
        )
        .await;
        gotten
    })
}

pub trait Get: Height {
    // Declared as `-> impl Future + Send` (rather than `async fn`) so that
    // implementors produce `Send` futures. The recursive `Box::pin` inside
    // the inductive `Get::<S<H>>::get` body coerces to
    // `Pin<Box<dyn Future + Send + '_>>`; the coercion requires the source
    // state machine to be `Send`, which is what the `Send + Sync` bounds on
    // `P`, `T` and the `Send` bounds on `F`, `Fut` discharge.
    fn get<T, F, Fut>(
        node: Option<Node<T, Self>>,
        prefix: Prefix<Self>,
        paths: Paths<Self>,
        with_gotten: &mut F,
    ) -> impl Future<Output = ()> + Send
    where
        T: Send + Sync,
        F: FnMut(Key, &Version, &Arc<T>) -> Fut + Send,
        Fut: Future<Output = ()> + Send;
}

impl<H: Get> Get for S<H>
where
    S<H>: Height,
{
    async fn get<T, F, Fut>(
        node: Option<Node<T, Self>>,
        prefix: Prefix<Self>,
        paths: Paths<Self>,
        with_gotten: &mut F,
    ) where
        T: Send + Sync,
        F: FnMut(Key, &Version, &Arc<T>) -> Fut + Send,
        Fut: Future<Output = ()> + Send,
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
                // Box-and-Send-erase the recursive future; see the matching
                // comment in `act.rs`.
                let fut: Pin<Box<dyn Future<Output = ()> + Send + '_>> = Box::pin(Get::get(
                    children.remove(&radix),
                    prefix.push(radix),
                    Paths::Selected(child_paths),
                    with_gotten,
                ));
                fut.await;
            }
        } else {
            // Get all the paths
            for (radix, child) in node.into_children() {
                let fut: Pin<Box<dyn Future<Output = ()> + Send + '_>> = Box::pin(Get::get(
                    Some(child),
                    prefix.push(radix),
                    Paths::All,
                    with_gotten,
                ));
                fut.await;
            }
        }
    }
}

impl Get for Z {
    async fn get<T, F, Fut>(
        node: Option<Node<T, Self>>,
        prefix: Prefix<Self>,
        paths: Paths<Self>,
        with_gotten: &mut F,
    ) where
        T: Send + Sync,
        F: FnMut(Key, &Version, &Arc<T>) -> Fut + Send,
        Fut: Future<Output = ()> + Send,
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
