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
pub async fn act<'a, P, T, F>(
    node: Option<Node<P, T, Root>>,
    actions: Vec<(Path, Version<P>, Action<T>)>,
    on_action: F,
) -> Option<Node<P, T, Root>>
where
    P: Clone + Ord + AsRef<[u8]> + 'a,
    T: 'a,
    F: AsyncFnMut(Key, &Version<P>, Option<&Message<T>>) + 'a,
{
    Box::pin(async move {
        let mut on_action = on_action;
        Act::act(node, Prefix::new(), actions, &mut on_action).await
    })
    .await
}

// The internal implementation of the traversal as a polymorphic-recursive

pub trait Act: Height {
    async fn act<P, T>(
        node: Option<Node<P, T, Self>>,
        prefix: Prefix<Self>,
        actions: Vec<(Path<Self>, Version<P>, Action<T>)>,
        on_action: &mut impl AsyncFnMut(Key, &Version<P>, Option<&Message<T>>),
    ) -> Option<Node<P, T, Self>>
    where
        P: Clone + Ord + AsRef<[u8]>;
}

impl<H: Act> Act for S<H>
where
    S<H>: Height,
{
    async fn act<P, T>(
        node: Option<Node<P, T, S<H>>>,
        prefix: Prefix<Self>,
        actions: Vec<(Path<Self>, Version<P>, Action<T>)>,
        on_action: &mut impl AsyncFnMut(Key, &Version<P>, Option<&Message<T>>),
    ) -> Option<Node<P, T, S<H>>>
    where
        P: Clone + Ord + AsRef<[u8]>,
    {
        // Group the paths by their first element
        let by_radix = actions
            .into_iter()
            .map(|(path, version, action)| {
                let (child, path) = path.pop();
                (child, path, version, action)
            })
            .sorted_by_key(|(child, _, _, _)| *child)
            .chunk_by(|(child, _, _, _)| *child);

        // Explode the node into its children
        let mut existing_children = node.map(|n| n.into_children()).unwrap_or_default();

        // Recursively apply each radix group into the corresponding child of
        // the original node, pulling each existing child out of the original
        // map exploded from the node
        let mut updated: Vec<_> = Vec::new();
        for (radix, group) in by_radix.into_iter() {
            // Collect all the actions for this radix group, and mutably
            // pull the existing child out of the parent:
            let actions: Vec<_> = group
                .map(|(_, path, version, action)| (path, version, action))
                .collect();
            let existing_child = existing_children.remove(&radix);

            // Short-circuit when solely trying to delete from a non-existent child:
            if existing_child.is_none()
                && actions
                    .iter()
                    .all(|(_, _, action)| matches!(action, Action::Forget))
            {
                continue;
            }

            // We box the future to avoid having an enormous future due to recursion.
            let recursed = Box::pin(Act::act(
                existing_child,
                prefix.push(radix),
                actions,
                on_action,
            ))
            .await;
            if let Some(child) = recursed {
                updated.push((radix, child));
            }
        }

        // Re-assemble: updated children + untouched existing children.
        Node::branch(updated.into_iter().chain(existing_children).collect())
    }
}

impl Act for Z {
    async fn act<P, T>(
        mut node: Option<Node<P, T, Self>>,
        prefix: Prefix<Self>,
        actions: Vec<(Path<Self>, Version<P>, Action<T>)>,
        on_action: &mut impl AsyncFnMut(Key, &Version<P>, Option<&Message<T>>),
    ) -> Option<Node<P, T, Z>>
    where
        P: Clone + Ord + AsRef<[u8]>,
    {
        let existed_before = node.is_some();
        let mut greatest_version = Version::default();

        // Sequentially apply the operations pertaining to this node; the
        // causally posterior operation wins, with concurrent or equal actions
        // biasing towards the last in the sequence
        for (_, version, action) in actions {
            greatest_version |= version.clone();

            // Skip updates that are strictly causally prior to the current
            // version at this node
            if &version
                < node
                    .as_ref()
                    .map(|n| n.version())
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
