use std::{fmt::Debug, iter::Map, marker::PhantomData};

use borsh::{BorshDeserialize, BorshSerialize};
use imbl::{OrdMap, ordmap};

use crate::{message::Message, version::Version};

use super::hash::Hash;
use super::height::{self, Height, S, Z};
use super::levels::{Top, levels};
use super::prefix::Prefix;
use super::untyped;

/// The typed node with a height of 32; the root of the tree.
pub type Root<T> = Node<T, height::Root>;

/// The type of children of a given height.
pub struct Children<T, H: Height> {
    height: PhantomData<fn() -> H>,
    inner: OrdMap<u8, untyped::Node<T>>,
}

impl<T, H: Height> Default for Children<T, H> {
    fn default() -> Self {
        Self {
            height: PhantomData,
            inner: OrdMap::new(),
        }
    }
}

impl<T, H: Height> Children<T, H> {
    fn from_untyped_map(inner: OrdMap<u8, untyped::Node<T>>) -> Self {
        Self {
            height: PhantomData,
            inner,
        }
    }

    fn into_untyped_map(self) -> OrdMap<u8, untyped::Node<T>> {
        self.inner
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn insert(&mut self, radix: u8, child: Node<T, H>) -> Option<Node<T, H>> {
        self.inner
            .insert(radix, child.into_untyped())
            .map(Node::from_untyped)
    }

    pub fn remove(&mut self, radix: &u8) -> Option<Node<T, H>> {
        self.inner.remove(radix).map(Node::from_untyped)
    }

    #[allow(clippy::type_complexity)]
    pub fn diff_owned<'a>(
        &'a self,
        other: &'a Self,
    ) -> impl Iterator<Item = (u8, Option<Node<T, H>>, Option<Node<T, H>>)> + 'a {
        self.inner.diff(&other.inner).map(|item| match item {
            ordmap::DiffItem::Add(&radix, theirs) => {
                (radix, None, Some(Node::from_untyped(theirs.clone())))
            }
            ordmap::DiffItem::Remove(&radix, ours) => {
                (radix, Some(Node::from_untyped(ours.clone())), None)
            }
            ordmap::DiffItem::Update {
                old: (&radix, ours),
                new: (_, theirs),
            } => (
                radix,
                Some(Node::from_untyped(ours.clone())),
                Some(Node::from_untyped(theirs.clone())),
            ),
        })
    }
}

impl<T, H: Height> FromIterator<(u8, Node<T, H>)> for Children<T, H> {
    fn from_iter<I: IntoIterator<Item = (u8, Node<T, H>)>>(iter: I) -> Self {
        Self::from_untyped_map(
            iter.into_iter()
                .map(|(radix, child)| (radix, child.into_untyped()))
                .collect(),
        )
    }
}

fn typed_child<T, H: Height>((radix, inner): (u8, untyped::Node<T>)) -> (u8, Node<T, H>) {
    (radix, Node::from_untyped(inner))
}

impl<T, H: Height> IntoIterator for Children<T, H> {
    type Item = (u8, Node<T, H>);
    type IntoIter = Map<
        <OrdMap<u8, untyped::Node<T>> as IntoIterator>::IntoIter,
        fn((u8, untyped::Node<T>)) -> (u8, Node<T, H>),
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.inner
            .into_iter()
            .map(typed_child::<T, H> as fn((u8, untyped::Node<T>)) -> (u8, Node<T, H>))
    }
}

/// A typed node which enforces the structural validity of the constructed tree
/// at compile-time.
///
/// The height marker is held as `PhantomData<fn() -> H>` rather than
/// `PhantomData<H>`. Function pointers are unconditionally `Send + Sync`,
/// so the auto-trait check on `Node` does not descend into the
/// `S<S<S<...S<Z>...>>>` peano-style height chain. Without this, the
/// `SharedPointer<...>: Send` obligation imposed by `imbl` (which
/// requires its contents to be `Sync`) recursively walks 32 levels of
/// `S<…>: Sync`, even though the type variable `H` itself is purely phantom
/// and never constructs anything that could fail to be `Send`/`Sync`.
#[repr(transparent)]
pub struct Node<T, H: Height> {
    height: PhantomData<fn() -> H>,
    inner: untyped::Node<T>,
}

impl<T, H: Height> Clone for Node<T, H> {
    fn clone(&self) -> Self {
        Self {
            height: self.height,
            inner: self.inner.clone(),
        }
    }
}

impl<T, H> Debug for Node<T, H>
where
    T: Debug,
    H: Height,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl<T, H: Height> Node<T, H> {
    fn from_untyped(inner: untyped::Node<T>) -> Self {
        Self {
            height: PhantomData,
            inner,
        }
    }

    fn into_untyped(self) -> untyped::Node<T> {
        self.inner
    }

    /// Get the ceiling version of this node (the greatest version contained within).
    pub fn ceiling(&self) -> &Version {
        self.inner.ceiling()
    }

    /// Get the floor version of this node (the least version contained within).
    pub fn floor(&self) -> &Version {
        self.inner.floor()
    }

    /// Get the number of leaves under this node.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether this node's content is a single leaf, regardless of any
    /// path-compressed prefix above it. For such a node `version` is also the
    /// meet of its leaves, so a single version comparison decides whether the
    /// whole (compressed) subtree is kept or dropped — no need to explode it.
    pub fn is_leaf(&self) -> bool {
        self.inner.is_leaf()
    }

    /// Lazily walk every leaf of this subtree, given the `prefix` path already
    /// taken to reach it, yielding each leaf's reconstructed [`Key`],
    /// [`Version`] and [`Message`].
    ///
    /// Read-only: it borrows the subtree and leaves it — and its memoized
    /// hash/ceiling/floor — untouched, which is what lets a caller observe every
    /// leaf without the destroy-and-rebuild that [`into_children`] incurs.
    ///
    /// [`into_children`]: Node::into_children
    /// [`Key`]: crate::Key
    pub(crate) fn leaves(&self, prefix: Prefix<H>) -> untyped::Iter<'_, T> {
        untyped::Iter::within(&self.inner, prefix.as_bytes())
    }

    /// Number of path-compressed prefix bytes on this node — i.e., the
    /// count of singleton virtual-branch levels collapsed above the node's
    /// actual content. Zero for a leaf or a non-compressed branch.
    #[cfg(test)]
    pub fn compressed_prefix_len(&self) -> usize {
        self.inner.compressed_prefix_len()
    }

    /// Hash the subtree rooted at this node.
    ///
    /// Hashes are computed lazily on first read and memoized, so the first read
    /// of a freshly-built subtree costs `O(nodes)` and every read thereafter is
    /// an `O(1)` field load.
    ///
    /// The hashing convention (see [`Hash::branch`] and [`Hash::leaf`]): a leaf
    /// hashes to `blake3(LEAF_TAG)`; a branch to `blake3(BRANCH_TAG ‖ r₀ ‖ h₀ ‖
    /// …)` over its children in ascending radix order. Hashing does not depend
    /// on path compression: a one-child branch and a node path-compressed by
    /// one byte produce identical hashes.
    pub fn hash(&self) -> Hash {
        self.inner.hash()
    }
}

impl<T, H: Height> Node<T, S<H>>
where
    S<H>: Height,
{
    /// Construct a new branch node from a map of children (inverse to
    /// [`Node::into_children`]).
    pub fn branch(children: Children<T, H>) -> Option<Self> {
        Some(Node {
            height: PhantomData,
            inner: untyped::Node::branch(children.into_untyped_map())?,
        })
    }

    /// Convert a node into a map from child index to child node (inverse to
    /// [`Node::branch`]).
    pub fn into_children(self) -> Children<T, H> {
        let children = match self.inner.into_children() {
            Ok(children) => children,
            Err(_) => unreachable!("typed nonzero-height node cannot be an uncompressed leaf"),
        };

        Children::from_untyped_map(children)
    }

    /// Wrap `child` (at height `H`) beneath slot `index` of a virtual branch
    /// at height `S<H>`. The result is the typed counterpart of
    /// `untyped::Node::beneath`: it path-compresses a single-child wrap into
    /// the underlying node's prefix without materializing the intervening
    /// branch level.
    pub fn beneath(child: Node<T, H>, index: u8) -> Self {
        Node {
            height: PhantomData,
            inner: child.inner.beneath(index),
        }
    }
}

impl<T> Node<T, Z> {
    /// Construct a new leaf node from a versioned message.
    pub fn leaf(version: Version, message: Message<T>) -> Self {
        Self {
            height: PhantomData,
            inner: untyped::Node::leaf(version, message),
        }
    }

    /// Get a reference to the message at this leaf node.
    pub fn message(&self) -> &Message<T> {
        self.inner
            .as_leaf()
            .expect("typed leaf failed to be a leaf")
    }
}

impl<T> Node<T, height::Root> {
    pub fn levels(node: Option<Root<T>>) -> Top<T> {
        levels(node)
    }

    /// Lazily iterate every live leaf in this root subtree as
    /// `(Key, &Version, &Arc<T>)`. Delegates to the height-agnostic untyped
    /// walk; because this is a height-32 root, every yielded path is a full
    /// 32-byte [`Key`](crate::Key).
    pub fn iter(&self) -> untyped::Iter<'_, T> {
        untyped::Iter::root(&self.inner)
    }

    pub fn root_hash(node: &Option<Root<T>>) -> Hash {
        // An absent root is the empty tree, which hashes as a branch with no
        // children (`blake3(BRANCH_TAG)`), not as the all-zero default.
        node.as_ref()
            .map(|n| n.hash())
            .unwrap_or_else(Hash::empty_root)
    }
}

impl<T, H: Height> Eq for Node<T, H> {}

impl<T, H: Height> PartialEq for Node<T, H> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

// Borsh wire format. Serialization is height-uniform: every typed
// `Node<T, H>` delegates to [`untyped::Node::serialize_to`], which
// emits the in-memory representation directly (prefix length, head bytes,
// then either a leaf body or a `count_minus_two` + children list). No
// leaf-vs-branch tag is needed on the wire — at the receiver, the typed
// height together with the running `prefix_len` names the body's shape.
//
// Deserialization at typed height `H` reads `prefix_len`, then either
// decodes the body directly (when `prefix_len == 0`) or peels one head
// byte and recurses at the next-finer typed height — synthesizing the
// `prefix_len - 1` byte for the inner reader via
// [`borsh::io::Read::chain`]. The recursion bottoms out at the typed
// level matching the structural level of the underlying body: a multi-
// child branch at `S<_>` heights, or a leaf at `Z`.
//
// Multi-child branches always carry at least two children (the path-
// compression invariant); singletons appear on the wire only as
// `prefix_len > 0` and reconstruct through [`Node::beneath`].
//
// The branch decoder builds a typed [`Children`] through its safe `insert`
// API rather than transmuting an `OrdMap<u8, Node<T, H>>`: `Node` carries no
// unsafe code, so the wire decoder stays within the same safe boundary as
// [`Node::branch`].

impl<T, H> BorshSerialize for Node<T, H>
where
    H: Height,
{
    fn serialize<W: borsh::io::Write>(&self, writer: &mut W) -> borsh::io::Result<()> {
        self.inner.serialize_to(writer)
    }
}

impl<T> BorshDeserialize for Node<T, Z>
where
    T: BorshDeserialize,
{
    fn deserialize_reader<R: borsh::io::Read>(reader: &mut R) -> borsh::io::Result<Self> {
        let prefix_len = u8::deserialize_reader(reader)?;
        if prefix_len != 0 {
            return Err(borsh::io::Error::new(
                borsh::io::ErrorKind::InvalidData,
                "leaf height cannot carry a prefix",
            ));
        }
        let version = Version::deserialize_reader(reader)?;
        let message = Message::<T>::deserialize_reader(reader)?;
        Ok(Node::leaf(version, message))
    }
}

impl<T, H> BorshDeserialize for Node<T, S<H>>
where
    T: BorshDeserialize,
    H: Height,
    S<H>: Height,
    Node<T, H>: BorshDeserialize,
{
    fn deserialize_reader<R: borsh::io::Read>(reader: &mut R) -> borsh::io::Result<Self> {
        let prefix_len = u8::deserialize_reader(reader)?;
        if (prefix_len as usize) > <S<H>>::HEIGHT {
            return Err(borsh::io::Error::new(
                borsh::io::ErrorKind::InvalidData,
                "prefix length exceeds typed height",
            ));
        }
        if prefix_len == 0 {
            let count_minus_two = u8::deserialize_reader(reader)?;
            let count = (count_minus_two as usize) + 2;
            if count > 256 {
                return Err(borsh::io::Error::new(
                    borsh::io::ErrorKind::InvalidData,
                    "branch children count exceeds 256",
                ));
            }
            let mut children = Children::<T, H>::default();
            let mut prev: Option<u8> = None;
            for _ in 0..count {
                let radix = u8::deserialize_reader(reader)?;
                if let Some(p) = prev
                    && radix <= p
                {
                    return Err(borsh::io::Error::new(
                        borsh::io::ErrorKind::InvalidData,
                        "branch radices not strictly ascending",
                    ));
                }
                prev = Some(radix);
                let child = Node::<T, H>::deserialize_reader(reader)?;
                children.insert(radix, child);
            }
            Node::branch(children).ok_or_else(|| {
                borsh::io::Error::new(
                    borsh::io::ErrorKind::InvalidData,
                    "branch could not be reconstructed",
                )
            })
        } else {
            let head = u8::deserialize_reader(reader)?;
            // Prepend `prefix_len - 1` to the rest of the stream so the
            // inner typed level reads it as if it were on the wire,
            // synthesizing the singleton-chain recursion without a helper
            // trait.
            let synthesized = [prefix_len - 1];
            let mut chained = borsh::io::Read::chain(synthesized.as_slice(), &mut *reader);
            let inner = Node::<T, H>::deserialize_reader(&mut chained)?;
            Ok(Node::beneath(inner, head))
        }
    }
}
