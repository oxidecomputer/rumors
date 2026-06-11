use itertools::Itertools;

use crate::{message::Message, version::Version};

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
/// `on_action` fires once per *effectual* action — a leaf inserted, replaced,
/// or removed — with that action's version. A forget of a leaf that never
/// existed observes nothing, which is what lets the caller join versions only
/// for actions that changed the tree.
///
/// `actions` is consumed lazily: the only materialization is the radix sort
/// at each branch level, so callers can feed a `map` chain straight in.
pub fn act<T, F, I>(
    node: Option<Node<T, Root>>,
    actions: I,
    mut on_action: F,
) -> Option<Node<T, Root>>
where
    T: Send + Sync,
    F: FnMut(&Version),
    I: IntoIterator<Item = (Path, Version, Action<T>)>,
{
    Act::act(node, actions, &mut on_action)
}

// The internal implementation of the traversal as a polymorphic-recursive
// trait: each height implements one inductive step, and the recursion is a
// plain (synchronous) call one height down (always instantiated at
// `I = Vec<…>`, the per-radix group the branch level collects).

pub trait Act: Height {
    fn act<T, F, I>(
        node: Option<Node<T, Self>>,
        actions: I,
        on_action: &mut F,
    ) -> Option<Node<T, Self>>
    where
        T: Send + Sync,
        F: FnMut(&Version),
        I: IntoIterator<Item = (Path<Self>, Version, Action<T>)>;
}

impl<H: Act> Act for S<H>
where
    S<H>: Height,
{
    fn act<T, F, I>(
        node: Option<Node<T, S<H>>>,
        actions: I,
        on_action: &mut F,
    ) -> Option<Node<T, S<H>>>
    where
        T: Send + Sync,
        F: FnMut(&Version),
        I: IntoIterator<Item = (Path<Self>, Version, Action<T>)>,
    {
        // Group the paths by their first element. Each group is consumed (and
        // its tail of the path collected) before the recursion below runs, so
        // the lazy `ChunkBy` borrow never overlaps it.
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
        for (radix, group) in &by_radix {
            // This collect is load-bearing: it type-erases the group before
            // the recursion. `Act` is instantiated once per `Height` level,
            // so a lazy iterator here would weave this level's iterator type
            // (closures capturing `I` and all) into the next level's `I`; the
            // type compounds across all 32 levels and monomorphization
            // explodes at codegen — tens of GiB of rustc memory in every
            // downstream crate that links this one. `Vec` resets `I` to the
            // same flat type at every level. It also lets the short-circuit
            // below inspect the actions without consuming them.
            let actions: Vec<_> = group
                .map(|(_, path, version, action)| (path, version, action))
                .collect();

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

            if let Some(child) = Act::act(existing_child, actions, on_action) {
                updated.push((radix, child));
            }
        }

        // Re-assemble: updated children + untouched existing children.
        Node::branch(updated.into_iter().chain(existing_children).collect())
    }
}

impl Act for Z {
    fn act<T, F, I>(
        mut node: Option<Node<T, Self>>,
        actions: I,
        on_action: &mut F,
    ) -> Option<Node<T, Z>>
    where
        T: Send + Sync,
        F: FnMut(&Version),
        I: IntoIterator<Item = (Path<Self>, Version, Action<T>)>,
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

        // Observe the action, provided that the net action wasn't nil
        match (existed_before, &node) {
            // The node stayed empty
            (false, None) => {}
            _ => on_action(&greatest_version),
        }

        node
    }
}
