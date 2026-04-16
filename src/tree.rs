use std::{hash::Hash, mem};

use bytes::Bytes;

mod typed;

pub use Action::*;

use crate::Version;

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
pub struct Tree<P: Clone + Hash + Eq + AsRef<[u8]> = Bytes> {
    party: P,
    inner: Inner<P>,
}

#[derive(Clone, Debug)]
enum Inner<P: Clone + Eq + Hash + AsRef<[u8]>> {
    Root(typed::node::Root<P>),
    Empty(Version<P>),
}

impl<P: Clone + Eq + Hash + AsRef<[u8]>> Inner<P> {
    fn take_root(&mut self) -> Option<typed::node::Root<P>> {
        match self {
            Inner::Root(root) => {
                let empty = Inner::Empty(root.version().clone());
                let Inner::Root(root) = mem::replace(self, empty) else {
                    unreachable!("tree was root");
                };
                Some(root)
            }
            Inner::Empty(_) => None,
        }
    }
}

/// An action to perform on the tree.
#[derive(Clone, Debug)]
pub enum Action {
    /// Insert some value, tagged at a version by the inserting party.
    Insert(Bytes),
    /// Delete the value corresponding to a hash.
    Delete([u8; 32]),
}

impl<P: Clone + Hash + Eq + AsRef<[u8]>> Tree<P> {
    /// Create a new tree which represents the perspective of the given party.
    pub fn for_party(party: P) -> Tree<P> {
        Tree {
            party,
            inner: Inner::Empty(Version::default()),
        }
    }

    /// Find out which party created this tree.
    pub fn party(&self) -> &P {
        &self.party
    }

    /// Get the version for the tree.
    pub fn version(&self) -> &Version<P> {
        match &self.inner {
            Inner::Root(root) => root.version(),
            Inner::Empty(version) => version,
        }
    }

    /// Get the root hash for the tree.
    pub fn hash(&self) -> [u8; 32] {
        // The root hash of an empty tree is "00000..."
        const EMPTY_ROOT_HASH: blake3::Hash = blake3::Hash::from_bytes([0x00; 32]);

        match &self.inner {
            Inner::Empty(_) => *EMPTY_ROOT_HASH.as_bytes(),
            Inner::Root(root) => *root.hash().as_bytes(),
        }
    }

    /// Get all the values stored at a list of hash paths in the tree.
    pub fn get<I>(&self, paths: I) -> Vec<Bytes>
    where
        I: IntoIterator<Item = [u8; 32]>,
    {
        if let Inner::Root(root) = &self.inner {
            typed::traverse::get(
                Some(&root),
                paths.into_iter().map(typed::Path::from).collect(),
            )
        } else {
            Vec::new()
        }
    }

    /// Apply the specified actions as a batch to the tree, incrementing its
    /// internal version vector.
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
        I: IntoIterator<Item = Action>,
    {
        // Get the tree's current version, incrementing the local scalar by one.
        let mut version = self.version().clone();
        version.event(&self.party);

        // Now apply all the actions in this batch with an identical version,
        // delegating to the logic in `react`:
        self.react(i.into_iter().map(|action| (&version, action)));
    }

    /// Apply the specified *versioned* actions as a batch to the tree without
    /// incrementing its internal version vector.
    ///
    /// If multiple actions refer to the same leaf of the tree, the causally
    /// latest action wins, with order of specification breaking concurrency and
    /// version ties. Because each item is keyed by (party, version, hash), if
    /// each party only manipulates their *own* tree using [`Tree::act`], these
    /// conflicts are impossible.
    ///
    /// It is more efficient to apply a batch of actions all at once, compared
    /// to applying them one at a time, even though the two are semantically
    /// equivalent. This is because all actions in a batch are applied to the
    /// tree in a single traversal. Theoretically, this gives an O(log n)
    /// speedup relative to one-by-one insertion operations, but since the log
    /// base is 256, in practice this is about 2-3x.
    pub fn react<'a, I>(&mut self, i: I)
    where
        P: 'a,
        I: IntoIterator<Item = (&'a Version<P>, Action)>,
    {
        // Get the tree's current version.
        let mut root_version = self.version().clone();

        // Convert the specified actions into the action specification required
        // by the inductive traversal of the tree
        let actions = i
            .into_iter()
            .map(|(version, op)| {
                // While traversing the actions, get the greatest version of any
                // of them; we'll set the root version to this if the end result
                // is an empty tree:
                root_version |= version.clone();

                match op {
                    Delete(hash) => (
                        typed::Path::from(hash),
                        version,
                        typed::traverse::Action::Delete,
                    ),
                    Insert(value) => (
                        typed::Path::for_leaf(&self.party, version.for_party(&self.party), &value),
                        version,
                        typed::traverse::Action::Insert(value),
                    ),
                }
            })
            .collect();

        // Traverse the tree from the root, batch-applying the actions
        self.inner = match typed::traverse::act(self.inner.take_root(), actions) {
            Some(root) => Inner::Root(root),
            None => Inner::Empty(root_version.clone()),
        }
    }
}

// Equality is defined specially for trees, comparing root hashes if both have
// children, and comparing versions only if both are empty. Because root hashes
// incorporate the (party, scalar version, bytes) of the leaves, it should be
// the case that two trees are equal *exactly when* they represent the same
// causal history and point in time.

impl<P: Clone + Hash + Eq + AsRef<[u8]>> Eq for Tree<P> {}

impl<P: Clone + Hash + Eq + AsRef<[u8]>> PartialEq for Tree<P> {
    fn eq(&self, other: &Self) -> bool {
        use Inner::*;
        match (&self.inner, &other.inner) {
            (Root(_), Empty(_)) | (Empty(_), Root(_)) => false,
            (Empty(v), Empty(w)) => v == w,
            (Root(l), Root(r)) => l == r,
        }
    }
}

#[cfg(test)]
mod test;
