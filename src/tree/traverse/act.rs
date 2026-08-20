use itertools::Itertools;

use crate::{Version, message::Message};

use super::typed::*;
use height::{Height, Root, S, Z};

/// An action to perform at a particular [`Path`].
#[derive(Debug, Clone)]
pub enum Action {
    /// Insert a value tagged by a version at a party.
    Insert(Message),
    /// Delete a value at this path.
    Forget,
}

/// Performs a sequence of actions (insertions or deletions) on `node`.
///
/// `on_action` fires once per *effectual* action — a leaf inserted, replaced,
/// or removed — with that action's version. A forget of a leaf that never
/// existed observes nothing, which is what lets the caller join versions only
/// for actions that changed the tree.
///
/// `actions` arrives materialized, and the walk's signature is deliberately
/// monomorphic — a concrete `Vec` and a `&mut dyn FnMut` observer — so the
/// entire per-height apply tower compiles once, into this crate: a generic
/// entry would re-instantiate all of it (radix grouping included) in every
/// downstream crate that applies a batch.
///
/// # Panics
///
/// Panics if an insert lands on a live leaf disagreeing with it on
/// version or payload: version reuse. No input reaches that state —
/// every production insert carries a freshly minted version (a fresh
/// tick strictly dominates the ceiling bounding every live leaf, and
/// party linearity keeps regions disjoint), and no wire-derived leaf
/// passes through this walk — so the panic marks a bug in this crate,
/// never an environmental failure.
pub fn act(
    node: Option<Node<Root>>,
    actions: Vec<(Path, Version, Action)>,
    on_action: &mut dyn FnMut(&Version),
) -> Option<Node<Root>> {
    // Test-only unwind source for the panic-atomicity pins: this walk is
    // the unwind-source region of `Tree::react`'s commit section, and its entry
    // burns the first fuse step (each branch-level step below burns one
    // more).
    #[cfg(test)]
    crate::tree::panic_injection::fire_if_armed();

    Act::act(node, actions, &mut |v| on_action(v))
}

/// The inductive step of the batch-apply, implemented per [`Height`]: the
/// internal form of the [`act`] free function as a polymorphic-recursive
/// trait.
///
/// Each height implements one step, and the recursion is a plain
/// (synchronous) call one height down (always instantiated at `I = Vec<…>`,
/// the per-radix group the branch level collects).
pub trait Act: Height {
    fn act<F, I>(node: Option<Node<Self>>, actions: I, on_action: &mut F) -> Option<Node<Self>>
    where
        F: FnMut(&Version),
        I: IntoIterator<Item = (Path<Self>, Version, Action)>;
}

impl<H: Act> Act for S<H>
where
    S<H>: Height,
{
    fn act<F, I>(node: Option<Node<S<H>>>, actions: I, on_action: &mut F) -> Option<Node<S<H>>>
    where
        F: FnMut(&Version),
        I: IntoIterator<Item = (Path<Self>, Version, Action)>,
    {
        // Test-only unwind source, continued: every branch-level apply step
        // burns one fuse step, so a fuse armed past the entry unwinds only
        // after earlier steps completed real apply work.
        #[cfg(test)]
        crate::tree::panic_injection::fire_if_armed();

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
            // Materialize the group: the recursion takes each level's
            // actions as a concrete `Vec` (the monomorphic signature the
            // once-compiled tower rests on), and the short-circuit below
            // inspects the actions without consuming them.
            let actions: Vec<_> = group
                .map(|(_, path, version, action)| (path, version, action))
                .collect();

            // Mutably pull the existing child out of the parent:
            let existing_child = existing_children.remove(radix);

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
    fn act<F, I>(mut node: Option<Node<Self>>, actions: I, on_action: &mut F) -> Option<Node<Z>>
    where
        F: FnMut(&Version),
        I: IntoIterator<Item = (Path<Self>, Version, Action)>,
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

            // Paths are version-derived, so an insert landing on a live
            // leaf claims a version the tree already binds. Verify identity
            // instead of assuming it: a byte-identical pair is the same
            // send twice (keep the resident leaf), and any mismatch is
            // version reuse — no input reaches it (fresh ticks strictly
            // dominate the ceiling; party linearity keeps regions
            // disjoint; no wire-derived leaf passes through this walk), so
            // the assert marks a crate bug before anything commits.
            if let (Action::Insert(value), Some(existing)) = (&action, &node) {
                assert!(
                    *existing.ceiling() == version
                        && existing.message().as_slice() == value.as_slice(),
                    "version reuse: an insert landed on a live leaf disagreeing on version or payload",
                );
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
