use std::{hash::Hash, sync::Arc};

use bytes::Bytes;

mod traverse;
mod typed;

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
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Tree {
    party: Bytes,
    version: Version,
    root: Option<typed::node::Root>,
}

/// An identifier for a unique item in the tree.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct Id(pub [u8; 32]);

impl From<[u8; 32]> for Id {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl From<Id> for [u8; 32] {
    fn from(id: Id) -> Self {
        id.0
    }
}

impl From<typed::Path> for Id {
    fn from(path: typed::Path) -> Self {
        <[u8; 32]>::from(path).into()
    }
}

impl From<Id> for typed::Path {
    fn from(id: Id) -> Self {
        typed::Path::from(id.0)
    }
}

/// An action to perform on the tree, locally.
#[derive(Clone, Debug)]
pub enum Action {
    /// Insert some value, tagged at the current version by your own party.
    Insert(Bytes),
    /// Delete the value corresponding to a hash.
    Delete(Id),
}

/// An action to replay on the tree, originating from another party.
#[derive(Clone, Debug)]
pub enum Reaction {
    /// Insert some value, tagged at a version by the inserting party.
    Insert(Id, Bytes),
    /// Delete the value corresponding to a hash.
    Delete(Id),
}

impl Tree {
    /// Create a new tree which represents the perspective of the given party.
    pub fn for_party(party: impl AsRef<[u8]>) -> Tree {
        Tree {
            party: Bytes::copy_from_slice(&blake3::hash(party.as_ref()).as_bytes()[..]),
            version: Version::default(),
            root: None,
        }
    }

    /// Get the version for the tree.
    pub fn version(&self) -> &Version {
        &self.version
    }

    /// Get the pre-hashed local party identifier this tree was created for.
    pub fn party(&self) -> &Bytes {
        &self.party
    }

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
        I: IntoIterator<Item = Id>,
    {
        if let Some(root) = &self.root {
            traverse::get(
                Some(&root),
                paths
                    .into_iter()
                    .map(|i| i.0)
                    .map(typed::Path::from)
                    .collect(),
            )
        } else {
            Vec::new()
        }
    }

    /// Get all the values in this tree which are unknown relative to the given
    /// version vector.
    pub fn unknown(&self, version: Version) -> Vec<(Id, Version, Bytes)> {
        traverse::unknown(self.root.as_ref(), &version)
            .into_iter()
            .map(|(i, v, b)| (i.into(), v, b))
            .collect()
    }

    /// Apply the specified actions as a batch to the tree, incrementing its
    /// internal version vector.
    ///
    /// If multiple actions refer to the same leaf of the tree, the last
    /// specified action wins.
    ///
    /// It is more efficient to apply a batch of actions all at once, compared
    /// to applying them one at a time. This is because all actions in a batch
    /// are applied to the tree in a single traversal. Theoretically, this gives
    /// an O(log n) speedup relative to one-by-one insertion operations, but
    /// since the log base is 256, in practice this is about 2-3x.
    ///
    /// While [`Tree::react`] is associative, this function is not: each batch
    /// receives a unique incrementing version, tracked internally.
    pub fn act<I>(&mut self, i: I)
    where
        I: IntoIterator<Item = Action>,
    {
        // Get the tree's current version, incrementing the local scalar by one.
        let mut new_version = self.version().clone();
        new_version.event(&self.party);

        // Get the local party.
        let party = self.party.clone();

        // Now apply all the actions in this batch with an identical version,
        // delegating to the logic in `react`:
        let reactions = i.into_iter().map(|action| {
            // Convert unversioned, unlocalized actions into `Reaction`s
            // which are independent of our local party and current version:
            let reaction = match action {
                Action::Delete(hash) => Reaction::Delete(hash),
                Action::Insert(value) => Reaction::Insert(
                    typed::Path::for_leaf(&party, new_version.for_party(&party), &value).into(),
                    value,
                ),
            };
            (&new_version, reaction)
        });
        self.react(reactions);
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
        I: IntoIterator<Item = (&'a Version, Reaction)>,
    {
        // Convert the specified actions into the action specification required
        // by the inductive traversal of the tree
        let actions = i
            .into_iter()
            .map(|(version, op)| {
                // Mark that we have seen this operation's version:
                self.version |= version.clone();

                match op {
                    Reaction::Delete(hash) => {
                        (typed::Path::from(hash), version, traverse::Action::Delete)
                    }
                    Reaction::Insert(hash, value) => (
                        typed::Path::from(hash),
                        version,
                        traverse::Action::Insert(value),
                    ),
                }
            })
            .collect();

        // Traverse the tree from the root, batch-applying the actions
        self.root = traverse::act(self.root.take(), actions);
    }
}

#[cfg(test)]
mod test;
