use std::mem;

use borsh::{BorshDeserialize, BorshSerialize};
use bytes::Bytes;

mod key;
mod traverse;
mod typed;

#[cfg(test)]
mod arb;

use crate::{
    message::Message,
    tree::{
        traverse::Paths,
        typed::{Hash, Node},
    },
    version::Version,
};

pub use key::Key;

pub use traverse::mirror;

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
#[derive(Debug, Eq, PartialEq)]
pub struct Tree<T> {
    party: Bytes,
    pub(crate) root: Root<Bytes, T>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Root<P: Clone + Ord + AsRef<[u8]>, T> {
    version: Version<P>,
    root: Option<typed::node::Root<P, T>>,
}

impl<P: Clone + Ord + AsRef<[u8]>, T> From<Root<P, T>> for Option<typed::node::Root<P, T>> {
    fn from(value: Root<P, T>) -> Self {
        value.root
    }
}

impl<P: Clone + Ord + AsRef<[u8]>, T> Clone for Root<P, T> {
    fn clone(&self) -> Self {
        Self {
            version: self.version.clone(),
            root: self.root.clone(),
        }
    }
}

impl<T> Clone for Tree<T> {
    fn clone(&self) -> Self {
        Self {
            party: self.party.clone(),
            root: self.root.clone(),
        }
    }
}

/// An action to perform on the tree, locally.
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub enum Action<T> {
    /// Insert some value, tagged at the current version by your own party.
    Insert(Message<T>),
    /// Forget the value corresponding to a hash.
    Forget(Key),
}

impl<T> Tree<T> {
    /// Create a new tree which represents the perspective of the given party.
    pub fn for_party(party: impl AsRef<[u8]>) -> Self {
        Tree {
            party: Bytes::copy_from_slice(&Hash::of(party.as_ref()).as_bytes()[..]),
            root: Root {
                version: Version::default(),
                root: None,
            },
        }
    }

    /// Get the version for the tree.
    pub fn version(&self) -> Version {
        self.root.version.clone()
    }

    /// Get the pre-hashed local party identifier this tree was created for.
    pub fn party(&self) -> &Bytes {
        &self.party
    }

    /// Get the root hash for the tree.
    pub fn hash(&self) -> [u8; 32] {
        Node::root_hash(&self.root.clone().into()).into()
    }

    /// Get all the values stored at a list of hash paths in the tree.
    pub fn get<I>(&self, paths: I) -> Vec<(Version, Key, Message<T>)>
    where
        I: IntoIterator<Item = Key>,
    {
        if let Some(root) = &self.root.root {
            traverse::get(
                Some(root.clone()),
                Paths::Selected(
                    paths
                        .into_iter()
                        .map(|i| i.0)
                        .map(typed::Path::from)
                        .collect(),
                ),
            )
        } else {
            Vec::new()
        }
    }

    /// Get all the values in this tree which are unknown relative to the given
    /// version vector.
    pub fn unknown(&self, version: Version) -> Vec<(Version, Key, Message<T>)> {
        let mut unknown = Vec::new();
        traverse::unknown(self.root.clone().into(), &version, &mut |v, k, m| {
            unknown.push((v.clone(), k, m.clone()))
        });
        unknown
    }

    /// Apply the specified actions as a batch to the tree, incrementing its
    /// internal version vector.
    ///
    /// If multiple actions refer to the same leaf of the tree, the last
    /// specified action wins.
    ///
    /// Upon insertion or deletion, the corresponding [`Reaction`] to the
    /// specified [`Action`] is provided to the given closure, so that the
    /// caller can (at their discretion) inspect the items inserted/forgotten
    /// from the tree. These [`Reaction`]s can be replayed on another tree using
    /// [`Tree::react`] to identical effect.
    ///
    /// It is more efficient to apply a batch of actions all at once, compared
    /// to applying them one at a time. This is because all actions in a batch
    /// are applied to the tree in a single traversal. Theoretically, this gives
    /// an O(log n) speedup relative to one-by-one insertion operations, but
    /// since the log base is 256, in practice this is about 2-3x.
    ///
    /// While [`Tree::react`] is associative, this function is not: each batch
    /// receives a unique incrementing version, tracked internally.
    pub fn act<I, O>(&mut self, actions: I, mut react: O)
    where
        I: IntoIterator<Item = Action<T>>,
        O: FnMut(&Version, Key, &Option<Message<T>>),
    {
        // Get the tree's current version, incrementing the local scalar by one.
        let mut new_version = self.version().clone();
        new_version.event(&self.party);

        // Get the local party.
        let party = self.party.clone();

        // Now apply all the actions in this batch with an identical version,
        // delegating to the logic in `react`:
        let reactions = actions.into_iter().map(|action| {
            // Convert unversioned, unlocalized actions into `Reaction`s
            // which are independent of our local party and current version:
            let (key, value) = match action {
                Action::Forget(hash) => (hash, None),
                Action::Insert(value) => (
                    typed::Path::for_leaf(&party, new_version.for_party(&party), value.bytes())
                        .into(),
                    Some(value),
                ),
            };
            react(&new_version, key, &value);
            (new_version.clone(), key, value)
        });
        self.react(reactions);
    }

    /// Apply the specified *versioned* actions as a batch to the tree without
    /// incrementing its internal version vector. In the specified iterator,
    /// `Some(message)` indicates an insert, and `None` indicates that the key
    /// should be forgotten.
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
    pub fn react<M, I>(&mut self, reactions: I)
    where
        M: Into<Option<Message<T>>>,
        I: IntoIterator<Item = (Version, Key, M)>,
    {
        let mut tree_version = self.version();

        // Convert the specified actions into the action specification required
        // by the inductive traversal of the tree
        let actions = reactions
            .into_iter()
            .map(|(version, key, message)| {
                // Join the version on all operations: forget and insert
                tree_version |= version.clone();
                match message.into() {
                    None => (typed::Path::from(key), version, traverse::Action::Forget),
                    Some(value) => (
                        typed::Path::from(key),
                        version,
                        traverse::Action::Insert(value),
                    ),
                }
            })
            .collect();

        // Traverse the tree from the root, batch-applying the actions
        self.root.root = traverse::act(self.root.root.take().into(), actions);
        self.root.version = tree_version;
    }
}

#[cfg(test)]
mod test;
