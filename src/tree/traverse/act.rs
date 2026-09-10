//! Apply a local batch in path order, retaining nodes that do not change.

use itertools::Itertools;

use crate::{Version, message::Message};

use super::typed::*;
use height::{Height, Root, S, Z};

/// An action to perform at a particular [`Path`].
#[derive(Debug, Clone)]
pub enum Action {
    /// Insert a message at its version-derived path.
    Insert(Message),
    /// Delete the leaf at this path.
    Forget,
}

/// Apply versioned actions, preserving their order within each path.
///
/// Actions at one path must be causally ascending, as
/// [`Tree::act`](crate::tree::Tree::act) guarantees. The observer receives the
/// last applied version at each changed key. A key that starts and ends absent
/// contributes no history. An occupied key whose actions are all skipped
/// reports an empty version: the caller's changed flag remains conservative,
/// but its ceiling does not advance.
///
/// Sorting once makes each recursive group a contiguous slice. Keeping this
/// entry monomorphic also keeps the per-height traversal out of downstream
/// crates' monomorphizations.
///
/// # Panics
///
/// After skipping causally older actions, an insert that disagrees with a
/// resident leaf's version or payload panics. Fresh versions and disjoint
/// parties prevent this in a conforming replica; disagreement indicates version
/// reuse, a crate bug.
pub fn act(
    node: Option<Node<Root>>,
    mut actions: Vec<(Path, Version, Action)>,
    on_action: &mut dyn FnMut(&Version),
) -> Option<Node<Root>> {
    #[cfg(test)]
    crate::tree::panic_injection::fire_if_armed();

    // Stable sorting keeps specification order for actions at the same key.
    actions.sort_by_key(|(path, _, _)| *path);
    let result = Act::act(node, &actions, on_action);
    // Skipped or displaced payloads can have panicking destructors. Release
    // the batch before returning a candidate for the caller to publish.
    drop(actions);
    result
}

/// One height of the batch traversal; slices retain full leaf addresses.
pub trait Act: Height {
    /// Apply the sorted slice belonging to this subtree.
    fn act(
        node: Option<Node<Self>>,
        actions: &[(Path, Version, Action)],
        on_action: &mut dyn FnMut(&Version),
    ) -> Option<Node<Self>>;
}

/// Descend by one radix byte while retaining unchanged subtrees.
impl<H: Act> Act for S<H>
where
    S<H>: Height,
{
    /// Apply each radix group and rebuild only if a child changed.
    fn act(
        node: Option<Node<Self>>,
        actions: &[(Path, Version, Action)],
        on_action: &mut dyn FnMut(&Version),
    ) -> Option<Node<Self>> {
        #[cfg(test)]
        crate::tree::panic_injection::fire_if_armed();

        if actions.is_empty()
            || (node.is_none()
                && actions
                    .iter()
                    .all(|(_, _, action)| matches!(action, Action::Forget)))
        {
            return node;
        }

        // Keep the original handle, including its memos. Expanding a compressed
        // prefix creates temporary nodes, which a no-op must not publish.
        let mut existing = node.clone().map(Node::into_children).unwrap_or_default();
        let mut updated = Vec::new();
        let mut changed = false;
        let radix =
            |(path, _, _): &(Path, Version, Action)| <[u8; 32]>::from(*path)[32 - Self::HEIGHT];
        for group in actions.chunk_by(|a, b| radix(a) == radix(b)) {
            let slot = radix(&group[0]);
            let prior = existing.remove(slot);
            let child = H::act(prior.clone(), group, on_action);
            changed |= match (&prior, &child) {
                (None, None) => false,
                (Some(a), Some(b)) => !a.ptr_eq(b),
                _ => true,
            };
            if let Some(child) = child {
                updated.push((slot, child));
            }
        }

        if !changed {
            return node;
        }
        // The two sorted streams have disjoint radices: each updated slot was
        // removed from `existing`. Merge them without another sort.
        Node::branch(
            updated
                .into_iter()
                .merge_by(existing, |a, b| a.0 < b.0)
                .collect(),
        )
    }
}

/// Resolve the ordered actions at a single leaf address.
impl Act for Z {
    /// Apply one key's actions, storing each insert's own version.
    fn act(
        mut node: Option<Node<Self>>,
        actions: &[(Path, Version, Action)],
        on_action: &mut dyn FnMut(&Version),
    ) -> Option<Node<Self>> {
        let existed_before = node.is_some();
        let mut last_applied = None;
        for (_, version, action) in actions {
            // A conforming local tick exceeds every resident version. Keep
            // this check for internal fixtures and malformed stored state.
            if node.as_ref().is_some_and(|n| version < n.ceiling()) {
                continue;
            }
            last_applied = Some(version);

            if let (Action::Insert(value), Some(existing)) = (action, &node) {
                assert!(
                    existing.ceiling() == version
                        && existing.message().as_slice() == value.as_slice(),
                    "version reuse: an insert landed on a live leaf disagreeing on version or payload",
                );
                continue;
            }
            node = match action {
                Action::Forget => None,
                Action::Insert(value) => Some(Node::leaf(version.clone(), value.clone())),
            };
        }

        // An insert followed by a forget of the same fresh key is invisible
        // outside this batch. Otherwise the last applied version bounds the
        // earlier actions, since each key's actions are causally ascending.
        if existed_before || node.is_some() {
            on_action(last_applied.unwrap_or(&Version::default()));
        }
        node
    }
}
