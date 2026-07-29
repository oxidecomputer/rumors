//! The backend-generic point lookup: one root-to-leaf descent, one
//! [`Store::child`] fetch per level.

use futures::future::{self, BoxFuture, FutureExt as _};

use crate::tree::{
    backend::{Leaf, Store},
    typed::{
        Path, Prefix,
        height::{self, Height, S, Z},
    },
};

/// Look up the live leaf at `path` beneath the (possibly absent) root;
/// `None` when no live leaf sits there.
pub async fn get<B, T>(
    backend: &B,
    root: Option<B::Node<height::Root>>,
    path: Path,
) -> Result<Option<B::Node<Z>>, B::Error>
where
    B: Store<T, Node<Z>: Leaf<T>> + Sync,
    T: Send + Sync + 'static,
{
    match root {
        None => Ok(None),
        Some(node) => Get::get(backend, Prefix::new(), node, path).await,
    }
}

/// The inductive step of the generic point lookup, implemented per
/// [`Height`].
pub trait Get: Height {
    fn get<'a, B, T>(
        backend: &'a B,
        prefix: Prefix<Self>,
        node: B::Node<Self>,
        path: Path<Self>,
    ) -> BoxFuture<'a, Result<Option<B::Node<Z>>, B::Error>>
    where
        B: Store<T, Node<Z>: Leaf<T>> + Sync,
        T: Send + Sync + 'static;
}

impl Get for Z {
    fn get<'a, B, T>(
        _backend: &'a B,
        _prefix: Prefix<Z>,
        node: B::Node<Z>,
        _path: Path<Z>,
    ) -> BoxFuture<'a, Result<Option<B::Node<Z>>, B::Error>>
    where
        B: Store<T, Node<Z>: Leaf<T>> + Sync,
        T: Send + Sync + 'static,
    {
        // The path is exhausted exactly at leaf height: this is the leaf.
        future::ready(Ok(Some(node))).boxed()
    }
}

impl<H: Get> Get for S<H>
where
    S<H>: Height,
{
    fn get<'a, B, T>(
        backend: &'a B,
        prefix: Prefix<S<H>>,
        node: B::Node<S<H>>,
        path: Path<S<H>>,
    ) -> BoxFuture<'a, Result<Option<B::Node<Z>>, B::Error>>
    where
        B: Store<T, Node<Z>: Leaf<T>> + Sync,
        T: Send + Sync + 'static,
    {
        Box::pin(async move {
            let (radix, rest) = path.pop();
            match backend.clone().child(prefix, node, radix).await? {
                None => Ok(None),
                Some(child) => H::get(backend, prefix.push(radix), child, rest).await,
            }
        })
    }
}
