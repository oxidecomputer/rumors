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
    party: &P,
    version: u64,
    actions: Vec<(Path<H>, Action)>,
) -> Option<Node<P, H>>
where
    P: Clone + Hash + Eq + AsRef<[u8]>,
{
    Act::act(node, party, version, actions)
}

// The internal implementation of the traversal as a polymorphic-recursive

pub trait Act: Height {
    fn act<P>(
        node: Option<Node<P, Self>>,
        party: &P,
        version: u64,
        actions: Vec<(Path<Self>, Action)>,
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
        party: &P,
        version: u64,
        actions: Vec<(Path<Self>, Action)>,
    ) -> Option<Node<P, S<H>>>
    where
        P: Clone + Hash + Eq + AsRef<[u8]>,
    {
        // Group the paths by their first element
        let by_radix = actions
            .into_iter()
            .map(|(path, operation)| {
                let (child, path) = path.pop();
                (child, path, operation)
            })
            .sorted_by_key(|(child, _, _)| *child)
            .chunk_by(|(child, _, _)| *child);

        // Explode the node into its children
        let mut existing_children = node.map(|n| n.into_children()).unwrap_or_default();

        // Recursively apply each radix group into the corresponding child of
        // the original node, pulling each existing child out of the original
        // map exploded from the node
        let updated: Vec<_> = by_radix
            .into_iter()
            .filter_map(|(radix, i)| {
                let insertions: Vec<_> = i.map(|(_, path, operation)| (path, operation)).collect();
                Some((
                    radix,
                    Act::act(existing_children.remove(&radix), party, version, insertions)?,
                ))
            })
            .collect();

        // Re-assemble: updated children + untouched existing children
        Node::branch(updated.into_iter().chain(existing_children).collect())
    }
}

impl Act for Z {
    fn act<P>(
        mut node: Option<Node<P, Z>>,
        party: &P,
        version: u64,
        actions: Vec<(Path<Self>, Action)>,
    ) -> Option<Node<P, Z>>
    where
        P: Clone + Hash + Eq + AsRef<[u8]>,
    {
        // Sequentially apply the operations pertaining to this node; the last
        // operation wins
        for (_, operation) in actions {
            node = match operation {
                Action::Delete => None,
                Action::Insert(value) => Some(Node::leaf(party.clone(), version, value)),
            };
        }

        node
    }
}
