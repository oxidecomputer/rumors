use itertools::Itertools;

use crate::{Message, Version};

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
pub fn act<P, T>(
    node: Option<Node<P, T, Root>>,
    actions: Vec<(Path, Version<P>, Action<T>)>,
) -> Option<Node<P, T, Root>>
where
    T: Clone,
    P: Clone + Ord + AsRef<[u8]>,
{
    Act::act(node, actions)
}

// The internal implementation of the traversal as a polymorphic-recursive

pub trait Act: Height {
    fn act<P, T>(
        node: Option<Node<P, T, Self>>,
        actions: Vec<(Path<Self>, Version<P>, Action<T>)>,
    ) -> Option<Node<P, T, Self>>
    where
        T: Clone,
        P: Clone + Ord + AsRef<[u8]>;
}

impl<H: Act> Act for S<H>
where
    S<H>: Height,
{
    fn act<P, T>(
        node: Option<Node<P, T, S<H>>>,
        actions: Vec<(Path<Self>, Version<P>, Action<T>)>,
    ) -> Option<Node<P, T, S<H>>>
    where
        T: Clone,
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
        let updated: Vec<_> = by_radix
            .into_iter()
            .filter_map(|(radix, i)| {
                // Collect all the actions for this radix group, and mutably
                // pull the existing child out of the parent:
                let actions: Vec<_> = i
                    .map(|(_, path, version, action)| (path, version, action))
                    .collect();
                let existing_child = existing_children.remove(&radix);

                // Short-circuit when solely trying to delete from a non-existent child:
                if existing_child.is_none()
                    && actions
                        .iter()
                        .all(|(_, _, action)| matches!(action, Action::Forget))
                {
                    return None;
                }

                // Recursively apply the actions to the existing child (if any)
                // or its absent slot (if missing):
                let child = Act::act(existing_child, actions)?;
                Some((radix, child))
            })
            .collect();

        // Re-assemble: updated children + untouched existing children.
        Node::branch(updated.into_iter().chain(existing_children).collect())
    }
}

impl Act for Z {
    fn act<P, T>(
        mut node: Option<Node<P, T, Z>>,
        actions: Vec<(Path<Self>, Version<P>, Action<T>)>,
    ) -> Option<Node<P, T, Z>>
    where
        T: Clone,
        P: Clone + Ord + AsRef<[u8]>,
    {
        // Sequentially apply the operations pertaining to this node; the
        // causally posterior operation wins, with concurrent or equal actions
        // biasing towards the last in the sequence
        for (_, version, action) in actions {
            // Skip updates that are strictly causally prior to the current
            // version at this node
            if &version
                < &node
                    .as_ref()
                    .map(|n| n.version())
                    .unwrap_or(&Version::default())
            {
                continue;
            }

            node = match action {
                Action::Forget => None,
                Action::Insert(value) => Some(Node::leaf(version.clone(), value)),
            };
        }

        node
    }
}
