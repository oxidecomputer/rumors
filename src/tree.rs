use std::hash::Hash;

use bytes::Bytes;

mod typed;

pub use Action::*;

/// A sparse Merkle trie with transparent path compression, whose leaves store
/// versioned blobs of [`Bytes`].
///
/// The tree internally has a branching factor of 256 and a depth of 32; this
/// means that each path into the tree corresponds exactly to the hash of the
/// `Bytes` stored at that position.
///
/// The only possible collisions (absent the astronomically unlikely hash
/// collision) are in the event multiple different parties write the same value.
/// This is resolved at the synchronization protocol level, and is not a concern
/// of the tree structure.
#[derive(Clone, Debug)]
pub struct Tree<P: Clone + Hash + Eq> {
    root: Option<typed::node::Root<P>>,
}

impl<P: Clone + Hash + Eq> Eq for Tree<P> {}

impl<P: Clone + Hash + Eq> PartialEq for Tree<P> {
    fn eq(&self, other: &Self) -> bool {
        self.root == other.root
    }
}

impl<P: Hash + Eq + Clone> Default for Tree<P> {
    fn default() -> Self {
        Self { root: None }
    }
}

/// An action to perform on the tree.
#[derive(Clone, Debug)]
pub enum Action<P> {
    /// Insert some value, tagged at a version by the inserting party.
    Insert {
        /// The party who inserted the value.
        party: P,
        /// Their local version scalar at time of insertion.
        version: u64,
        /// The value itself.
        value: Bytes,
    },
    /// Delete the value corresponding to a hash.
    Delete {
        /// The hash whose value we should delete.
        hash: [u8; 32],
    },
}

impl<P: Clone + Hash + Eq> Tree<P> {
    /// Get the root hash for the tree.
    pub fn hash(&self) -> [u8; 32] {
        // The root hash of an empty tree is "00000..."
        const EMPTY_ROOT_HASH: blake3::Hash = blake3::Hash::from_bytes([0x00; 32]);

        match &self.root {
            None => *EMPTY_ROOT_HASH.as_bytes(),
            Some(root) => *root.hash().as_bytes(),
        }
    }

    /// Get all the values stored at a list of hash paths in the tree.
    pub fn get<I>(&self, paths: I) -> Vec<Bytes>
    where
        I: IntoIterator<Item = [u8; 32]>,
    {
        typed::traverse::get(
            self.root.as_ref(),
            paths.into_iter().map(typed::Path::from).collect(),
        )
    }

    /// Apply the specified actions as a batch to the tree.
    ///
    /// If multiple actions refer to the same leaf of the tree, the last
    /// specified action wins.
    ///
    /// It is more efficient to apply a batch of actions all at once, compared
    /// to applying them one at a time, even though the two are semantically
    /// equivalent. This is because all actions in a batch are applied to the
    /// tree in a single traversal. Theoretically, this gives an O(log n)
    /// speedup relative to one-by-one insertion operations, but since the log
    /// base is 256, in practice this is about 2-3x.
    pub fn act<I>(&mut self, i: I)
    where
        I: IntoIterator<Item = Action<P>>,
    {
        // Convert the specified actions into the action specification required
        // by the inductive traversal of the tree
        let actions = i
            .into_iter()
            .map(|op| match op {
                Delete { hash } => (typed::Path::from(hash), typed::traverse::Action::Delete),
                Insert {
                    party,
                    version,
                    value,
                } => (
                    typed::Path::for_bytes(&value),
                    typed::traverse::Action::Insert {
                        party,
                        version,
                        value,
                    },
                ),
            })
            .collect();

        // Traverse the tree from the root, batch-applying the actions
        self.root = typed::traverse::act(self.root.take(), actions)
    }
}

// #[cfg(test)]
// mod test;

#[cfg(test)]
mod test;
