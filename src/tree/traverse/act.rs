use std::future::Future;
use std::pin::Pin;

use itertools::Itertools;

use crate::{Key, message::Message, version::Version};

use super::typed::*;
use height::{Height, Root, S, Z};

/// An action to perform at a particular [`Path`].
#[derive(Debug, Clone)]
pub enum Action<T> {
    /// Insert a value tagged by a version at a party.
    Insert(Message<T>),
    /// Delete a value at this path.
    Forget,
}

/// Perform a sequence of actions (insertions or deletions) on this node.
///
/// Type-erased via `Pin<Box<dyn Future>>` so the deep `S<S<…<Z>>>` chain
/// produced by recursive `Act` trait dispatch doesn't leak into the
/// layout queries of every public API that drives the tree — otherwise
/// downstream crates would need to bump their `recursion_limit`.
pub async fn act<'a, T, F, Fut>(
    node: Option<Node<T, Root>>,
    actions: Vec<(Path, Version, Action<T>)>,
    on_action: F,
) -> Option<Node<T, Root>>
where
    T: Send + Sync + 'a,
    F: FnMut(Key, &Version, Option<&Message<T>>) -> Fut + Send + 'a,
    Fut: Future<Output = ()> + Send + 'a,
{
    Box::pin(async move {
        let mut on_action = on_action;
        Act::act(node, Prefix::new(), actions, &mut on_action).await
    })
    .await
}

// The internal implementation of the traversal as a polymorphic-recursive

pub trait Act: Height {
    // Declared as `-> impl Future + Send` rather than `async fn` so that
    // implementors produce `Send` futures. The recursive `Box::pin` inside
    // the inductive `Act::<S<H>>::act` body coerces to `Pin<Box<dyn Future +
    // Send + '_>>`, and that coercion requires the source state machine to
    // be `Send`. The accompanying `Send + Sync` bounds on `P` and `T`, and
    // `Send` bounds on `F` and `Fut`, are what let the auto-trait check at
    // that coercion site succeed.
    fn act<T, F, Fut>(
        node: Option<Node<T, Self>>,
        prefix: Prefix<Self>,
        actions: Vec<(Path<Self>, Version, Action<T>)>,
        on_action: &mut F,
    ) -> impl Future<Output = Option<Node<T, Self>>> + Send
    where
        T: Send + Sync,
        F: FnMut(Key, &Version, Option<&Message<T>>) -> Fut + Send,
        Fut: Future<Output = ()> + Send;
}

impl<H: Act> Act for S<H>
where
    S<H>: Height,
{
    async fn act<T, F, Fut>(
        node: Option<Node<T, S<H>>>,
        prefix: Prefix<Self>,
        actions: Vec<(Path<Self>, Version, Action<T>)>,
        on_action: &mut F,
    ) -> Option<Node<T, S<H>>>
    where
        T: Send + Sync,
        F: FnMut(Key, &Version, Option<&Message<T>>) -> Fut + Send,
        Fut: Future<Output = ()> + Send,
    {
        // Group the paths by their first element. We collect eagerly into a
        // `Vec<(radix, group)>` (rather than holding the lazy `ChunkBy`
        // iterator across the recursive await below): `itertools::ChunkBy`
        // uses interior `RefCell`/`Cell` state, which is `!Sync` and would
        // make the surrounding async fn's future `!Send` for callers that
        // want to `tokio::spawn` it on a multi-threaded runtime.
        #[allow(clippy::type_complexity)]
        let by_radix: Vec<(u8, Vec<(Path<H>, Version, Action<T>)>)> = actions
            .into_iter()
            .map(|(path, version, action)| {
                let (child, path) = path.pop();
                (child, path, version, action)
            })
            .sorted_by_key(|(child, _, _, _)| *child)
            .chunk_by(|(child, _, _, _)| *child)
            .into_iter()
            .map(|(radix, group)| {
                (
                    radix,
                    group
                        .map(|(_, path, version, action)| (path, version, action))
                        .collect(),
                )
            })
            .collect();

        // Explode the node into its children
        let mut existing_children = node.map(|n| n.into_children()).unwrap_or_default();

        // Recursively apply each radix group into the corresponding child of
        // the original node, pulling each existing child out of the original
        // map exploded from the node
        let mut updated: Vec<_> = Vec::new();
        for (radix, actions) in by_radix {
            // Mutably pull the existing child out of the parent:
            let existing_child = existing_children.remove(&radix);

            // Short-circuit when solely trying to delete from a non-existent child:
            if existing_child.is_none()
                && actions
                    .iter()
                    .all(|(_, _, action)| matches!(action, Action::Forget))
            {
                continue;
            }

            // Box-and-Send-erase the recursive future. The dyn coercion
            // discharges the inner state machine's auto-trait check here,
            // inside the lib's `#![recursion_limit = "256"]`. Without the
            // coercion the source type is `Pin<Box<impl Future>>` and the outer
            // poll's auto-trait walk descends into the inner state machine,
            // recursing once per height and tripping downstream crates' default
            // `recursion_limit = 128`. With the coercion the outer walk sees
            // only `Pin<Box<dyn Future + Send>>`, which is trivially `Send`
            // regardless of what's inside.
            #[allow(clippy::type_complexity)]
            let recursed: Pin<
                Box<dyn Future<Output = Option<Node<T, H>>> + Send + '_>,
            > = Box::pin(Act::act(
                existing_child,
                prefix.push(radix),
                actions,
                on_action,
            ));
            let recursed = recursed.await;
            if let Some(child) = recursed {
                updated.push((radix, child));
            }
        }

        // Re-assemble: updated children + untouched existing children.
        Node::branch(updated.into_iter().chain(existing_children).collect())
    }
}

impl Act for Z {
    async fn act<T, F, Fut>(
        mut node: Option<Node<T, Self>>,
        prefix: Prefix<Self>,
        actions: Vec<(Path<Self>, Version, Action<T>)>,
        on_action: &mut F,
    ) -> Option<Node<T, Z>>
    where
        T: Send + Sync,
        F: FnMut(Key, &Version, Option<&Message<T>>) -> Fut + Send,
        Fut: Future<Output = ()> + Send,
    {
        let existed_before = node.is_some();
        let mut greatest_version = Version::default();

        // Sequentially apply the operations pertaining to this node; the
        // causally posterior operation wins, with concurrent or equal actions
        // biasing towards the last in the sequence
        for (_, version, action) in actions {
            // Join by reference: `version` is still needed for the causality
            // comparison just below, and the join doesn't consume it.
            greatest_version |= &version;

            // Skip updates that are strictly causally prior to the current
            // version at this node
            if version
                < node
                    .as_ref()
                    .map(|n| n.ceiling())
                    .unwrap_or(&Version::default())
            {
                continue;
            }

            // Set the node
            node = match action {
                Action::Forget => None,
                Action::Insert(value) => Some(Node::leaf(greatest_version.clone(), value)),
            };
        }

        // Log the action, provided that the net action wasn't nil
        match (existed_before, &node) {
            // The node stayed empty
            (false, None) => {}
            (_, node) => {
                on_action(
                    prefix.into(),
                    &greatest_version,
                    node.as_ref().map(|n| n.message()),
                )
                .await
            }
        }

        node
    }
}
