//! A one-byte-at-a-time `Entry` API over [`Node`], analogous to
//! [`std::collections::btree_map::Entry`], that makes path compression
//! transparent to callers.
//!
//! Callers obtain an [`OccupiedEntry`] from [`Node::root_entry`], match on its
//! variant ([`InteriorEntry`] for "you can still descend", [`LeafEntry`] for
//! "you are at a value"), and either step further with
//! [`InteriorEntry::descend`] or read/replace via the leaf accessors. A
//! [`VacantEntry`] is produced whenever a descent reaches an empty slot, and
//! the [`VacantEntry::insert_leaf`] method handles any prefix splitting needed
//! to splice in a new leaf.
//!
//! The surface area is total: every method has a meaningful result for every
//! well-formed entry it can be called on. The internal enums (`Interior` and
//! `Vacant`) carry exactly the borrows each variant needs, so no method ever
//! has to match on a `Children` variant it does not own.
//!
//! All tree-walk algorithms live outside this module and are written in terms
//! of these primitives.

use std::collections::BTreeMap;
use std::collections::btree_map;
use std::mem;
use std::sync::Arc;

use bytes::Bytes;

use super::cached_hash::CachedHash;
use super::{Children, Leaf, Node};

/// Either an existing position in the tree or an empty slot at which a leaf can
/// be inserted. Returned from [`InteriorEntry::descend`].
pub enum Entry<'a, P> {
    Occupied(OccupiedEntry<'a, P>),
    Vacant(VacantEntry<'a, P>),
}

/// A position currently filled by either an interior (non-terminal) subtree or
/// a terminal leaf. Splitting these into separate variants lets
/// [`InteriorEntry::descend`] be callable only where further descent makes
/// sense, and the leaf accessors only where a leaf actually exists.
pub enum OccupiedEntry<'a, P> {
    Interior(InteriorEntry<'a, P>),
    Leaf(LeafEntry<'a, P>),
}

/// A position from which descent can continue, either by consuming another byte
/// of the current node's compressed prefix or by dispatching through its
/// branching map. The internal representation tracks which of those two
/// situations we are in, so neither operation needs to inspect a [`Children`]
/// variant it does not own.
pub struct InteriorEntry<'a, P> {
    inner: Interior<'a, P>,
}

enum Interior<'a, P> {
    /// Inside a compressed prefix. `depth` bytes have already been virtually
    /// consumed from the shallow end and `depth < node.prefix.len()`.
    InPrefix { node: &'a mut Node<P>, depth: usize },
    /// At the actual branching level of a node whose `children` is a `Branch`.
    /// The map and the cached hash of the owning node are held directly, so
    /// descent needs no further `Children` inspection.
    AtBranch {
        map: &'a mut BTreeMap<u8, Arc<Node<P>>>,
        parent_hash: &'a mut CachedHash,
    },
}

/// A position at a terminal leaf. The whole logical path to this entry has been
/// consumed and the leaf's data can be read or modified, but no further descent
/// is possible. The leaf reference is held directly, without any `Children`
/// indirection.
pub struct LeafEntry<'a, P> {
    leaf: &'a mut Leaf<P>,
}

/// A position with no current child. Committing via
/// [`VacantEntry::insert_leaf`] either splits the surrounding node's compressed
/// prefix (when the vacancy sits inside it) or appends to the branching map
/// (when it sits at the branching level). The internal representation tracks
/// which case applies.
pub struct VacantEntry<'a, P> {
    inner: Vacant<'a, P>,
}

enum Vacant<'a, P> {
    /// Vacancy inside a compressed prefix: the next prefix byte at this virtual
    /// level disagrees with the dispatch byte. Committing requires splitting
    /// the node's prefix.
    InPrefix {
        node: &'a mut Node<P>,
        depth: usize,
        byte: u8,
    },
    /// Vacancy at the branching level: the dispatch byte is missing from the
    /// branching map. Committing inserts directly into the held
    /// `btree_map::VacantEntry`, with no `Children` inspection needed.
    InBranch {
        map_vacant: btree_map::VacantEntry<'a, u8, Arc<Node<P>>>,
        parent_hash: &'a mut CachedHash,
    },
}

impl<P> Node<P> {
    /// Open an [`OccupiedEntry`] at this node's outermost position, before any
    /// descent. The variant returned reflects whether the node is already at a
    /// terminal leaf (`prefix == []` and `children` is a `Leaf`) or still has
    /// structure to descend into.
    pub fn walk(&mut self) -> OccupiedEntry<'_, P> {
        classify(self, 0)
    }
}

impl<P> InteriorEntry<'_, P> {
    /// Mark the surrounding node's cached hash as dirty. Algorithms that may
    /// modify the subtree at or below this entry should call this before
    /// descending further; the cache will be repopulated on the next
    /// `Node::hash()` call.
    pub fn invalidate_hash(&mut self) {
        match &mut self.inner {
            Interior::InPrefix { node, .. } => node.hash.reset(),
            Interior::AtBranch { parent_hash, .. } => parent_hash.reset(),
        }
    }
}

impl<'a, P: Clone> InteriorEntry<'a, P> {
    /// Step one logical level down by dispatching on `byte`.
    ///
    /// Inside the compressed prefix, the byte is matched against the next
    /// prefix byte; a match returns a deeper [`Occupied`](Entry::Occupied)
    /// entry, and a mismatch returns a [`Vacant`](Entry::Vacant) entry that
    /// will split the prefix on commit. At the branching level, the byte is
    /// looked up in the map: a hit returns the appropriate `OccupiedEntry` for
    /// the child, and a miss returns a `VacantEntry` that will splice into the
    /// map on commit.
    pub fn child(&mut self, byte: u8) -> Entry<'_, P> {
        match &mut self.inner {
            Interior::InPrefix { node, depth } => {
                let expected = node.prefix[node.prefix.len() - 1 - *depth];
                if expected == byte {
                    Entry::Occupied(classify(node, *depth + 1))
                } else {
                    Entry::Vacant(VacantEntry {
                        inner: Vacant::InPrefix {
                            node,
                            depth: *depth,
                            byte,
                        },
                    })
                }
            }
            Interior::AtBranch { map, parent_hash } => match map.entry(byte) {
                btree_map::Entry::Occupied(occupied) => {
                    let arc = occupied.into_mut();
                    Entry::Occupied(classify(Arc::make_mut(arc), 0))
                }
                btree_map::Entry::Vacant(map_vacant) => Entry::Vacant(VacantEntry {
                    inner: Vacant::InBranch {
                        map_vacant,
                        parent_hash,
                    },
                }),
            },
        }
    }
}

impl<P> OccupiedEntry<'_, P> {
    /// Mark the surrounding node's cached hash as dirty, no-op for a terminal
    /// leaf (whose hash depends only on its position, not on the stored leaf
    /// data).
    pub fn invalidate_hash(&mut self) {
        match self {
            OccupiedEntry::Interior(interior) => interior.invalidate_hash(),
            OccupiedEntry::Leaf(_) => {}
        }
    }
}

impl<'a, P> LeafEntry<'a, P> {
    /// Borrow the leaf data immutably.
    pub fn leaf(&self) -> &Leaf<P> {
        self.leaf
    }

    /// Borrow the leaf data mutably for in-place modification. The leaf's
    /// merkle hash depends only on its position in the tree, not on the stored
    /// party/version/value, so updating those fields does not invalidate any
    /// cached hashes.
    pub fn leaf_mut(&mut self) -> &mut Leaf<P> {
        self.leaf
    }
}

impl<'a, P: Clone> VacantEntry<'a, P> {
    /// Splice a new leaf into the tree at this vacant slot, returning a mutable
    /// reference to the freshly inserted leaf data.
    ///
    /// If the vacancy sits inside a compressed prefix, split the prefix so that
    /// a new outer branch absorbs the shared top bytes and the two children
    /// (the old subtree with a shorter prefix, and the new leaf with
    /// `remaining_path` as its compressed prefix) sit on the two sides of the
    /// divergence. Otherwise the new leaf is appended to the branching map.
    pub fn insert_leaf(
        self,
        remaining_path: Vec<u8>,
        party: P,
        version: u64,
        value: Bytes,
    ) -> &'a mut Leaf<P> {
        let new_leaf_node = Node {
            prefix: remaining_path,
            hash: CachedHash::default(),
            children: Children::Leaf(Leaf {
                party,
                version,
                value,
            }),
        };

        match self.inner {
            Vacant::InPrefix { node, depth, byte } => {
                // Divergence inside the compressed prefix: split. The shared
                // top bytes (deepest-first) become the outer branch's own
                // compressed prefix; the old subtree keeps only the bytes
                // strictly below the divergence.
                let div_idx = node.prefix.len() - 1 - depth;
                let mut old = mem::take(node);
                let old_div_byte = old.prefix[div_idx];
                let shared: Vec<u8> = old.prefix[div_idx + 1..].to_vec();
                old.prefix.truncate(div_idx);
                old.hash.reset();

                let mut map = BTreeMap::new();
                map.insert(old_div_byte, Arc::new(old));
                map.insert(byte, Arc::new(new_leaf_node));

                *node = Node {
                    prefix: shared,
                    hash: CachedHash::default(),
                    children: Children::Branch(map),
                };

                // Hand back a mutable reference to the leaf data we wrote. We
                // just constructed the new branch with `byte` mapped to a
                // Leaf-children node, so the navigation is total.
                let map = match &mut node.children {
                    Children::Branch(map) => map,
                    Children::Leaf(_) => unreachable!("just constructed Branch"),
                };
                let arc = map
                    .get_mut(&byte)
                    .expect("byte was just inserted into the new branch");
                match &mut Arc::make_mut(arc).children {
                    Children::Leaf(leaf) => leaf,
                    Children::Branch(_) => unreachable!("just constructed Leaf"),
                }
            }
            Vacant::InBranch {
                map_vacant,
                parent_hash,
            } => {
                let arc = map_vacant.insert(Arc::new(new_leaf_node));
                parent_hash.reset();
                match &mut Arc::make_mut(arc).children {
                    Children::Leaf(leaf) => leaf,
                    Children::Branch(_) => unreachable!("just constructed Leaf"),
                }
            }
        }
    }

    /// The dispatch byte that produced this vacancy.
    pub fn byte(&self) -> u8 {
        match &self.inner {
            Vacant::InPrefix { byte, .. } => *byte,
            Vacant::InBranch { map_vacant, .. } => *map_vacant.key(),
        }
    }
}

/// Classify a `(node, depth)` pair into the appropriate `OccupiedEntry`
/// variant, decomposing the node into the disjoint sub-borrows each variant
/// requires. The returned entry's invariants therefore hold by construction.
fn classify<'a, P>(node: &'a mut Node<P>, depth: usize) -> OccupiedEntry<'a, P> {
    if depth < node.prefix.len() {
        return OccupiedEntry::Interior(InteriorEntry {
            inner: Interior::InPrefix { node, depth },
        });
    }

    // depth == node.prefix.len(): split the node into disjoint &mut hash and
    // &mut children borrows so the Branch arm can hand both to
    // `Interior::AtBranch` without re-matching on `Children` later.
    let Node { hash, children, .. } = node;
    match children {
        Children::Leaf(leaf) => OccupiedEntry::Leaf(LeafEntry { leaf }),
        Children::Branch(map) => OccupiedEntry::Interior(InteriorEntry {
            inner: Interior::AtBranch {
                map,
                parent_hash: hash,
            },
        }),
    }
}
