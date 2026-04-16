use std::collections::HashSet;
use std::hash::Hash;
use std::mem;
use std::sync::Arc;

use bytes::Bytes;
use imbl::OrdMap;
use itertools::Itertools;

mod cached;
use cached::Cached;

#[derive(Clone, Debug)]
pub struct Node<P: Hash + Eq> {
    inner: Arc<NodeInner<P>>,
}

#[derive(Clone, Debug)]
struct NodeInner<P: Hash + Eq> {
    /// Compressed path above this node's own branching level, stored with the
    /// deepest byte at index 0 and the shallowest byte at the last index. An
    /// empty prefix means the node is not path-compressed above its level.
    prefix: Vec<u8>,
    /// The cached hash of this node, invalidated when any change occurs in or
    /// beneath it.
    hash: Cached<blake3::Hash>,
    /// The children of this node: either a leaf, or a branch point.
    children: Children<P>,
}

/// The children of a node.
#[derive(Clone, Debug)]
enum Children<P: Hash + Eq> {
    /// A direct leaf, at the true bottom of the tree.
    Leaf(Leaf<P>),
    /// A materialized branch point, with the invariant that there are always >=
    /// 2 branches (or else they should be path-compressed away).
    Branch(Branch<P>),
}

/// A branch in the middle of the tree.
#[derive(Clone, Debug)]
pub struct Branch<P: Hash + Eq> {
    /// The children of this branch.
    children: OrdMap<u8, Node<P>>,
}

impl<P: Clone + Hash + Eq> Default for Branch<P> {
    fn default() -> Self {
        Self {
            children: OrdMap::new(),
        }
    }
}

/// A leaf at the bottom of the tree, holding the value payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Leaf<P: Hash + Eq> {
    /// The party which originally inserted this leaf into the set.
    pub party: P,
    /// That party's local version scalar at the time of insertion.
    pub version: u64,
    /// The value inserted, whose hash is the path in the tree.
    pub value: Bytes,
}

impl<P: Hash + Eq + Clone> Node<P> {
    /// Construct a new branch node from a list of children with distinct
    /// indices (inverse to [`Node::into_children`]).
    pub fn branch(children: OrdMap<u8, Node<P>>) -> Option<Self> {
        match children.len() {
            0 => None,
            1 => {
                let (index, node) = children.into_iter().next().unwrap();
                Some(node.beneath(index))
            }
            _ => Some(Node {
                inner: Arc::new(NodeInner {
                    prefix: Vec::new(),
                    hash: Cached::new(),
                    children: Children::Branch(Branch { children }),
                }),
            }),
        }
    }

    /// Convert a node into a map from child index to child node (inverse to
    /// [`Node::branch`]).
    ///
    /// If `self` is a leaf node, returns `Err(self)`.
    pub fn into_children(mut self) -> Result<OrdMap<u8, Node<P>>, Node<P>> {
        if !self.inner.prefix.is_empty() {
            // Path-compressed: pop the top byte and rewrap self under
            // it. The caller observes self with a shortened prefix,
            // so the cached hash must be invalidated.
            let inner = Arc::make_mut(&mut self.inner);
            inner.hash.invalidate();
            let index = inner.prefix.pop().expect("non-empty prefix");
            Ok(OrdMap::from_iter([(index, self)]))
        } else {
            match &self.inner.children {
                Children::Leaf(_) => Err(self),
                Children::Branch(_) => {
                    // Extract the children map; self is dropped without
                    // the caller observing its mutated state, so hash
                    // invalidation would be wasted work.
                    let inner = Arc::make_mut(&mut self.inner);
                    let Children::Branch(branch) = &mut inner.children else {
                        unreachable!("just matched Branch")
                    };
                    Ok(mem::take(&mut branch.children))
                }
            }
        }
    }

    /// Construct a new leaf node.
    pub fn leaf(party: P, version: u64, value: Bytes) -> Self {
        Node {
            inner: Arc::new(NodeInner {
                prefix: Vec::new(),
                hash: Cached::new(),
                children: Children::Leaf(Leaf {
                    party,
                    version,
                    value,
                }),
            }),
        }
    }

    /// Get a reference to the leaf at this node, if it is a leaf.
    pub fn as_leaf(&self) -> Option<&Leaf<P>> {
        match &self.inner.children {
            Children::Leaf(leaf) => Some(leaf),
            _ => None,
        }
    }

    /// Hash the subtree rooted at this node.
    ///
    /// Hashes are lazily computed and cached until the tree structure changes
    /// to invalidate them.
    ///
    /// The hashing convention: a leaf's "hash" is the distinguished sentinel
    /// `[0xff; 32]`, a branch's is the hash of 256 concatenated child hashes
    /// (with `[0x00; 32]` in empty slots). Hashing should not depend on path
    /// compression; we ensure this by hashing "virtual" nodes as we traverse up
    /// a compressed path.
    pub fn hash(&self) -> blake3::Hash {
        self.inner.hash.get(|| {
            let mut hash: blake3::Hash = match &self.inner.children {
                Children::Leaf(_) => [0xff; 32].into(),
                Children::Branch(branch) => {
                    let mut buf = [0u8; 256 * 32];
                    for (&i, child) in branch.children.iter() {
                        buf[i as usize * 32..][..32]
                            .copy_from_slice(child.hash().as_bytes());
                    }
                    blake3::hash(&buf)
                }
            };
            // Wrap with the compressed prefix bottom-up: prefix[0] is the
            // deepest byte, applied first; prefix[last] is the shallowest byte,
            // applied last (producing the hash at this node's own level).
            //
            // Each virtual branch is 256 × 32 = 8192 bytes: all zero-slots
            // except for the one occupied child. We build the full buffer
            // contiguously to avoid 256 separate tiny update calls per byte.
            for &byte in &self.inner.prefix {
                let mut buf = [0u8; 256 * 32];
                buf[byte as usize * 32..][..32].copy_from_slice(hash.as_bytes());
                hash = blake3::hash(&buf);
            }
            hash
        })
    }

    /// Merge an iterator of nodes of the same height, taking the union of their
    /// leaves. Returns `None` if the iterator is empty.
    ///
    /// If a leaf appears simultaneously in multiple nodes, the last one wins.
    pub fn unions<I>(nodes: I) -> Option<Self>
    where
        I: IntoIterator<Item = Self>,
    {
        let mut iter = nodes.into_iter();

        // Short-circuit 0 or 1 inputs: nothing to merge.
        let node_1 = iter.next()?;
        let Some(node_2) = iter.next() else {
            return Some(node_1);
        };

        // With >= 2 inputs, the first two tell us which level we're at: both
        // decompose (Ok) means a branch level, neither decomposes (Err) means a
        // leaf level, and a mix is a height violation. Subsequent inputs are
        // expected to match the level determined here; the decomposition of
        // each validates that.
        match (node_1.into_children(), node_2.into_children()) {
            (Ok(children_1), Ok(children_2)) => {
                let all_children: Vec<OrdMap<u8, Node<P>>> = [children_1, children_2]
                    .into_iter()
                    .chain(iter.map(|n| match n.into_children() {
                        Ok(c) => c,
                        Err(_) => panic!("structural violation: leaf at same height as branch"),
                    }))
                    .collect();

                // Group all children of all nodes under union by their child
                // index; we will now union all the children with the same child
                // index recursively, and construct a new node to hold all of
                // the results.
                let grouped_by_index = all_children
                    .into_iter()
                    .flatten()
                    .sorted_by(|(x, _), (y, _)| x.cmp(y))
                    .chunk_by(|(index, _)| *index);

                // For each child index, dedup by hash with last-wins
                // (preserving input order) and recurse.
                let mut children = grouped_by_index.into_iter().filter_map(|(child, nodes)| {
                    let mut deduped: Vec<Node<P>> = nodes.map(|(_, n)| n).collect();

                    // In-place dedup by hash with last-wins, preserving input
                    // order: reverse so `retain` sees nodes in reverse input
                    // order, keep the first occurrence in that direction (= the
                    // last occurrence in input order), then reverse back. Input
                    // order must be preserved through the recursion so that
                    // last-wins at metadata-conflicting leaves is decided by
                    // iteration order, not hash order.
                    let mut seen = HashSet::<blake3::Hash>::new();
                    deduped.reverse();
                    deduped.retain(|n| seen.insert(n.hash()));
                    deduped.reverse();

                    Some((child, Node::unions(deduped)?))
                });

                // At least one index group exists because we have >= 2
                // non-empty children maps. One group means path-compress via
                // `beneath`; two or more means build a branch directly.
                let (child_1, node_1) = children.next()?;
                Some(match children.next() {
                    None => node_1.beneath(child_1),
                    Some((child_2, node_2)) => Node {
                        inner: Arc::new(NodeInner {
                            prefix: Vec::new(),
                            hash: Cached::default(),
                            children: Children::Branch(Branch {
                                children: OrdMap::from_iter(
                                    [(child_1, node_1), (child_2, node_2)]
                                        .into_iter()
                                        .chain(children),
                                ),
                            }),
                        }),
                    },
                })
            }
            (Err(_), Err(second_leaf)) => {
                // Leaf level: last wins. The first leaf is superseded by the
                // second and any subsequent leaves.
                [second_leaf]
                    .into_iter()
                    .chain(iter.map(|n| match n.into_children() {
                        Err(leaf) => leaf,
                        Ok(_) => panic!("structural violation: branch at same height as leaf"),
                    }))
                    .last()
            }
            _ => panic!("structural violation: leaf at same height as branch"),
        }
    }

    /// Place a node beneath the given child index, increasing its height by one.
    fn beneath(mut self, index: u8) -> Node<P> {
        let inner = Arc::make_mut(&mut self.inner);
        inner.hash.invalidate();
        inner.prefix.push(index);
        self
    }

    /// Return `true` if no node in the tree violates path compression: every
    /// branch must have at least two children. The empty tree is represented by
    /// the absence of a root, so empty and one-child branches are never valid
    /// anywhere in the tree.
    #[cfg(test)]
    fn is_max_compressed(&self) -> bool {
        match &self.inner.children {
            Children::Leaf(_) => true,
            Children::Branch(branch) => {
                branch.children.len() >= 2 && branch.children.values().all(Self::is_max_compressed)
            }
        }
    }
}

impl<P: Hash + Eq + Clone> Eq for Node<P> {}

impl<P: Hash + Eq + Clone> PartialEq for Node<P> {
    fn eq(&self, other: &Self) -> bool {
        self.hash() == other.hash()
    }
}

#[cfg(test)]
mod tests;
