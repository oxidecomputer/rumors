use crate::Version;

use super::*;

/// An action to perform at a particular [`Path`].
pub enum Action {
    /// Insert a value tagged by a version at a party.
    Insert(Bytes),
    /// Delete a value at this path.
    Delete,
}

/// Perform a sequence of actions (insertions or deletions) on this node.
pub fn act<P, H: Act>(
    node: Option<Node<P, H>>,
    actions: Vec<(Path<H>, &Version<P>, Action)>,
) -> Option<Node<P, H>>
where
    P: Clone + Hash + Eq + AsRef<[u8]>,
{
    Act::act(node, actions)
}

// The internal implementation of the traversal as a polymorphic-recursive

pub trait Act: Height {
    fn act<P>(
        node: Option<Node<P, Self>>,
        actions: Vec<(Path<Self>, &Version<P>, Action)>,
    ) -> Option<Node<P, Self>>
    where
        P: Clone + Hash + Eq + AsRef<[u8]>;
}

impl<H: Act> Act for S<H>
where
    S<H>: Height,
{
    fn act<P>(
        node: Option<Node<P, S<H>>>,
        actions: Vec<(Path<Self>, &Version<P>, Action)>,
    ) -> Option<Node<P, S<H>>>
    where
        P: Clone + Hash + Eq + AsRef<[u8]>,
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
                        .all(|(_, _, action)| matches!(action, Action::Delete))
                {
                    return None;
                }

                // Recursively apply the actions to the existing child (if any)
                // or its absent slot (if missing):
                let child = Act::act(existing_child, actions)?;
                Some((radix, child))
            })
            .collect();

        // Re-assemble: updated children + untouched existing children
        Node::branch(updated.into_iter().chain(existing_children).collect())
    }
}

impl Act for Z {
    fn act<P>(
        mut node: Option<Node<P, Z>>,
        actions: Vec<(Path<Self>, &Version<P>, Action)>,
    ) -> Option<Node<P, Z>>
    where
        P: Clone + Hash + Eq + AsRef<[u8]>,
    {
        // Sequentially apply the operations pertaining to this node; the
        // causally posterior operation wins, with concurrent or equal actions
        // biasing towards the last in the sequence
        for (_, version, action) in actions {
            // Skip updates that are strictly causally prior to the current
            // version at this node
            if version
                < node
                    .as_ref()
                    .map(|n| n.version())
                    .unwrap_or(&Version::default())
            {
                continue;
            }

            node = match action {
                Action::Delete => None,
                Action::Insert(value) => Some(Node::leaf(version.clone(), value)),
            };
        }

        node
    }
}
