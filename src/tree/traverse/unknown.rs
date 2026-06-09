use std::cmp::Ordering;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::{message::Message, tree::key::Key, version::Version};

use super::typed::*;
use height::{Height, S, Z};
use prefix::Prefix;

/// Adapt a `&Arc<T>` observer into the `&Message<T>` callback [`Unknown`] fires,
/// so callers that hold the public `Arc`-shaped callback can pass it straight
/// in.
///
/// The return-position `impl FnMut(..)` pins a higher-ranked signature, which is
/// what lets the adapted callback flow through the `Option<&mut F>` parameter
/// without a "not general enough" lifetime error (the same wrinkle the sync
/// layer hits at the async boundary).
pub(crate) fn from_arc<T, F, Fut>(
    callback: &mut F,
) -> impl FnMut(Key, &Version, &Message<T>) -> Fut + '_
where
    F: FnMut(Key, &Version, &Arc<T>) -> Fut,
{
    move |k, v, m: &Message<T>| callback(k, v, m.as_ref())
}

/// Perform a batch lookup in the tree by version vector, reporting every
/// versioned leaf (with its path) that is *unknown* relative to the
/// specified version.
///
/// The unknown set is the set of leaves a counterparty holding this version
/// vector must receive for its tree to become a (non-strict) superset of
/// this one.
#[cfg(test)]
pub fn unknown<T>(
    node: Option<Node<T, height::Root>>,
    known: &Version,
    with_unknown: &mut (impl FnMut(Key, &Version, &Message<T>) + Send),
) -> Option<Node<T, height::Root>>
where
    T: Send + Sync,
{
    let mut wrapper = Some(|k, v: &Version, m: &Message<T>| {
        with_unknown(k, v, m);
        std::future::ready(())
    });
    pollster::block_on(Unknown::unknown(node, Prefix::new(), known, &mut wrapper))
}

pub trait Unknown: Height {
    // Declared as `-> impl Future + Send` (rather than `async fn`) so that
    // implementors produce `Send` futures. The recursive `Box::pin` inside
    // the inductive `Unknown::<S<H>>::unknown` body coerces to
    // `Pin<Box<dyn Future + Send + '_>>`; the coercion requires the source
    // state machine to be `Send`, which is what these `Send + Sync` /
    // `Send` bounds discharge.
    //
    // `with_unknown` is [`Option`]al: [`None`] means "filter, but don't
    // observe", which both removes the callers' need to wrap a maybe-absent
    // callback and unlocks the keep-whole fast path below.
    fn unknown<T, F, Fut>(
        node: Option<Node<T, Self>>,
        prefix: Prefix<Self>,
        known: &Version,
        with_unknown: &mut Option<F>,
    ) -> impl Future<Output = Option<Node<T, Self>>> + Send
    where
        T: Send + Sync,
        F: FnMut(Key, &Version, &Message<T>) -> Fut + Send,
        Fut: Future<Output = ()> + Send;
}

impl<H: Unknown> Unknown for S<H>
where
    S<H>: Height,
{
    async fn unknown<T, F, Fut>(
        node: Option<Node<T, Self>>,
        prefix: Prefix<Self>,
        known: &Version,
        with_unknown: &mut Option<F>,
    ) -> Option<Node<T, Self>>
    where
        T: Send + Sync,
        F: FnMut(Key, &Version, &Message<T>) -> Fut + Send,
        Fut: Future<Output = ()> + Send,
    {
        // If the node doesn't exist, we can't return information about it
        let node = node?;

        // If the node is causally prior or at the known version vector, it's
        // already known (and so are all its children, since they are always in
        // the causal past or present of their parent), so don't return anything
        if node.ceiling() <= known {
            return None;
        }

        // Keep-whole fast path: if this subtree's meet (the floor, the minimal
        // version among its leaves) is *not* dominated by `known`, then no leaf
        // can be either — any leaf `v` with `v <= known` would force
        // `floor <= v <= known` — so every leaf is unknown and none would be
        // filtered out. The floor is undominated exactly when the comparison is
        // `Greater` or incomparable (`None`); a concurrent floor counts, since a
        // counterparty at `known` is still missing it.
        //
        // Either way the destroy-and-rebuild below would reproduce this subtree
        // identically (with cold memos), so skip it: if anyone is observing,
        // fire the callback for every leaf via a read-only leaf walk — which
        // leaves the subtree and its memoized hash/ceiling/floor untouched —
        // then return it verbatim (an `Arc` move).
        let floor_unknown = matches!(
            node.floor().partial_cmp(known),
            None | Some(Ordering::Greater)
        );
        if floor_unknown {
            if let Some(with_unknown) = with_unknown.as_mut() {
                for (key, version, message) in node.leaves(prefix) {
                    with_unknown(key, version, message).await;
                }
            }
            return Some(node);
        }

        // Recursively process each child, re-assembling only the unknown children
        Node::branch({
            let mut children = Children::default();
            for (radix, child) in node.into_children() {
                // Box-and-Send-erase the recursive future; see the matching
                // comment in `act.rs`.
                #[allow(clippy::type_complexity)]
                let fut: Pin<
                    Box<dyn Future<Output = Option<Node<T, H>>> + Send + '_>,
                > = Box::pin(Unknown::unknown(
                    Some(child),
                    prefix.push(radix),
                    known,
                    with_unknown,
                ));
                let recursed = fut.await;
                if let Some(child) = recursed {
                    children.insert(radix, child);
                }
            }
            children
        })
    }
}

impl Unknown for Z {
    async fn unknown<T, F, Fut>(
        node: Option<Node<T, Self>>,
        prefix: Prefix,
        known: &Version,
        with_unknown: &mut Option<F>,
    ) -> Option<Node<T, Self>>
    where
        T: Send + Sync,
        F: FnMut(Key, &Version, &Message<T>) -> Fut + Send,
        Fut: Future<Output = ()> + Send,
    {
        // If the node doesn't exist, we can't return information about it
        let node = node?;

        // If the node is causally prior or at the known version vector, it's
        // already known, so don't return anything
        if node.ceiling() <= known {
            return None;
        }

        // Otherwise, the node is causally unknown, so observe it (if anyone is
        // listening) and return its information
        if let Some(with_unknown) = with_unknown.as_mut() {
            with_unknown(Path::from(prefix).into(), node.ceiling(), node.message()).await;
        }
        Some(node)
    }
}
