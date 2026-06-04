//! A direct, in-memory merge of two trees by a single simultaneous recursion
//! over both, inductive over the height.
//!
//! This is the local-only counterpart to the [`mirror`](super::mirror)
//! protocol: where the mirror reconciles two replicas by exchanging messages
//! (and so must serialize, run a zipper, and build the union on *both* sides),
//! `join` walks the two trees in lockstep in one process and builds the merged
//! union *once*. It is **observationally identical** to mirroring two local
//! trees: it produces the same merged [`Root`](crate::tree::Root) and fires the
//! same callbacks, by *delegating* all version-filtering and leaf observation to
//! the same [`Unknown`] traversal the mirror uses.
//!
//! For each pair of nodes at a path the recursion distinguishes four cases:
//!
//! - **neither side has it**: nothing.
//! - **only one side has it**: hand the whole subtree to [`Unknown::unknown`],
//!   filtered against the *other* side's version vector. Survivors are the
//!   subtree the other side learns; anything causally `<=` the other side's
//!   version was deleted there (the version vector is the entire deletion
//!   mechanism — there are no tombstones) and is dropped.
//! - **both have it, hashes equal**: the subtrees are identical (content
//!   addressing makes equal hash ⟹ equal content, versions included), so keep
//!   one verbatim and observe nothing.
//! - **both have it, hashes differ**: explode both one level and recurse per
//!   child over the union of their radixes, reassembling with [`Node::branch`]
//!   (which re-compresses singletons and recomputes the joined branch version).
//!
//! All callback firing therefore happens inside the [`Unknown`] delegations at
//! the asymmetric frontier; the lockstep recursion itself is pure structural
//! routing. Callbacks fire in ascending-[`Key`](crate::tree::Key) order (a
//! leaf's key is its full path and children iterate by radix), which is
//! deterministic but — like the mirror's order — not part of any public
//! contract.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use futures::future::Either;
use imbl::OrdMap;

use crate::{message::Message, tree::key::Key, version::Version};

use super::typed::*;
use super::unknown::Unknown;
use height::{Height, Root, S, Z};
use prefix::Prefix;

/// Merge two trees rooted at `a` and `b` into one, observing each side's gains.
///
/// `a_version` / `b_version` are the two roots' version vectors, used to honor
/// deletions (a node one side lacks while its version is `<=` that side's vector
/// was deleted there, and is dropped). `on_recv` fires for each leaf `a` gains
/// from `b`; `on_send` for each leaf `b` would gain from `a`. Either callback
/// may be [`None`], in which case its observations are skipped (the filtering
/// still runs).
///
/// Type-erased via `Pin<Box<dyn Future>>` for the same reason as
/// [`super::act::act`]: to keep the deep height chain out of callers' auto-trait
/// layout queries.
#[allow(clippy::too_many_arguments)]
pub async fn join<'a, T, R, RFut, W, WFut>(
    a: Option<Node<T, Root>>,
    b: Option<Node<T, Root>>,
    a_version: &Version,
    b_version: &Version,
    on_recv: Option<R>,
    on_send: Option<W>,
) -> Option<Node<T, Root>>
where
    T: Send + Sync + 'a,
    R: FnMut(Key, &Version, &Arc<T>) -> RFut + Send + 'a,
    RFut: Future<Output = ()> + Send + 'a,
    W: FnMut(Key, &Version, &Arc<T>) -> WFut + Send + 'a,
    WFut: Future<Output = ()> + Send + 'a,
{
    Box::pin(async move {
        let mut on_recv = on_recv;
        let mut on_send = on_send;
        Join::join(
            a,
            b,
            Prefix::new(),
            a_version,
            b_version,
            &mut on_recv,
            &mut on_send,
        )
        .await
    })
    .await
}

pub trait Join: Unknown {
    // As with [`Unknown`] / [`super::act::Act`]: declared `-> impl Future +
    // Send` (not `async fn`) so the recursive `Box::pin` at the `S<H>` step can
    // coerce to `Pin<Box<dyn Future + Send + '_>>`, keeping the auto-trait check
    // shallow enough for downstream crates' default `recursion_limit`.
    #[allow(clippy::too_many_arguments)]
    fn join<T, R, RFut, W, WFut>(
        a: Option<Node<T, Self>>,
        b: Option<Node<T, Self>>,
        prefix: Prefix<Self>,
        a_version: &Version,
        b_version: &Version,
        on_recv: &mut Option<R>,
        on_send: &mut Option<W>,
    ) -> impl Future<Output = Option<Node<T, Self>>> + Send
    where
        T: Send + Sync,
        R: FnMut(Key, &Version, &Arc<T>) -> RFut + Send,
        RFut: Future<Output = ()> + Send,
        W: FnMut(Key, &Version, &Arc<T>) -> WFut + Send,
        WFut: Future<Output = ()> + Send;
}

impl<H: Join> Join for S<H>
where
    S<H>: Height + Unknown,
{
    async fn join<T, R, RFut, W, WFut>(
        a: Option<Node<T, S<H>>>,
        b: Option<Node<T, S<H>>>,
        prefix: Prefix<S<H>>,
        a_version: &Version,
        b_version: &Version,
        on_recv: &mut Option<R>,
        on_send: &mut Option<W>,
    ) -> Option<Node<T, S<H>>>
    where
        T: Send + Sync,
        R: FnMut(Key, &Version, &Arc<T>) -> RFut + Send,
        RFut: Future<Output = ()> + Send,
        W: FnMut(Key, &Version, &Arc<T>) -> WFut + Send,
        WFut: Future<Output = ()> + Send,
    {
        match (a, b) {
            (None, None) => None,
            // Only we have it: filter against their version and report what
            // *they* learn (`on_send`); causally-known subtrees they deleted
            // drop out.
            (Some(ours), None) => {
                Unknown::unknown(
                    Some(ours),
                    prefix,
                    b_version,
                    &mut |k, v, m: &Message<T>| match on_send.as_mut() {
                        Some(on_send) => Either::Left(on_send(k, v, m.as_ref())),
                        None => Either::Right(std::future::ready(())),
                    },
                )
                .await
            }
            // Only they have it: filter against our version and report what
            // *we* learn (`on_recv`).
            (None, Some(theirs)) => {
                Unknown::unknown(
                    Some(theirs),
                    prefix,
                    a_version,
                    &mut |k, v, m: &Message<T>| match on_recv.as_mut() {
                        Some(on_recv) => Either::Left(on_recv(k, v, m.as_ref())),
                        None => Either::Right(std::future::ready(())),
                    },
                )
                .await
            }
            (Some(ours), Some(theirs)) => {
                // Identical subtrees: keep one, observe nothing. Equal hash ⟹
                // equal content (content addressing), so there is nothing to
                // learn on either side.
                if ours.hash() == theirs.hash() {
                    return Some(ours);
                }

                // Differing subtrees: descend both one level and recurse per
                // child over the union of radixes.
                let mut ours = ours.into_children();
                let mut theirs = theirs.into_children();
                let radixes: std::collections::BTreeSet<u8> =
                    ours.keys().chain(theirs.keys()).copied().collect();

                let mut merged: OrdMap<u8, Node<T, H>> = OrdMap::new();
                for radix in radixes {
                    let our_child = ours.remove(&radix);
                    let their_child = theirs.remove(&radix);

                    // Box-and-Send-erase the recursive future; see the matching
                    // comment in `act.rs`.
                    #[allow(clippy::type_complexity)]
                    let fut: Pin<
                        Box<dyn Future<Output = Option<Node<T, H>>> + Send + '_>,
                    > = Box::pin(Join::join(
                        our_child,
                        their_child,
                        prefix.push(radix),
                        a_version,
                        b_version,
                        on_recv,
                        on_send,
                    ));
                    if let Some(child) = fut.await {
                        merged.insert(radix, child);
                    }
                }

                Node::branch(merged)
            }
        }
    }
}

impl Join for Z {
    async fn join<T, R, RFut, W, WFut>(
        a: Option<Node<T, Z>>,
        b: Option<Node<T, Z>>,
        prefix: Prefix<Z>,
        a_version: &Version,
        b_version: &Version,
        on_recv: &mut Option<R>,
        on_send: &mut Option<W>,
    ) -> Option<Node<T, Z>>
    where
        T: Send + Sync,
        R: FnMut(Key, &Version, &Arc<T>) -> RFut + Send,
        RFut: Future<Output = ()> + Send,
        W: FnMut(Key, &Version, &Arc<T>) -> WFut + Send,
        WFut: Future<Output = ()> + Send,
    {
        match (a, b) {
            (None, None) => None,
            (Some(ours), None) => {
                Unknown::unknown(
                    Some(ours),
                    prefix,
                    b_version,
                    &mut |k, v, m: &Message<T>| match on_send.as_mut() {
                        Some(on_send) => Either::Left(on_send(k, v, m.as_ref())),
                        None => Either::Right(std::future::ready(())),
                    },
                )
                .await
            }
            (None, Some(theirs)) => {
                Unknown::unknown(
                    Some(theirs),
                    prefix,
                    a_version,
                    &mut |k, v, m: &Message<T>| match on_recv.as_mut() {
                        Some(on_recv) => Either::Left(on_recv(k, v, m.as_ref())),
                        None => Either::Right(std::future::ready(())),
                    },
                )
                .await
            }
            // Two leaves at the same path are the same leaf: the path *is*
            // `blake3(version ‖ value)`, so identical paths carry identical
            // (version, value). Keep one; observe nothing.
            (Some(ours), Some(theirs)) => {
                debug_assert_eq!(
                    ours.hash(),
                    theirs.hash(),
                    "two leaves at the same path must be identical",
                );
                let _ = theirs;
                Some(ours)
            }
        }
    }
}

#[cfg(test)]
mod test;
