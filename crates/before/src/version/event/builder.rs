use crate::codec::{Base, Bits};

use crate::version::compare::{EvNode, EvReader};
use crate::version::working::WorkingVersion;

/// Accumulates the output event tree in preorder. A node's base is written as a
/// placeholder when the node opens and finalized by
/// [`close_node`](Self::close_node) once its children are in place. This is the
/// canonical output path shared by every emitting walk (`join`, `fill`, the
/// [`grow`](super::grow) emit); it is the single place event normalization
/// lives, so callers never re-implement the sink/collapse.
pub(super) struct Builder {
    pub(super) topo: Bits,
    pub(super) base: Vec<Base>,
}

impl Builder {
    pub(super) fn with_capacity(nodes: usize) -> Self {
        Builder {
            topo: Bits::with_capacity(nodes),
            base: Vec::with_capacity(nodes),
        }
    }

    fn len(&self) -> usize {
        self.base.len()
    }

    /// Append a leaf with the given base; return its output position.
    pub(super) fn leaf(&mut self, base: Base) -> Leaf {
        let i = self.len();
        self.topo.push(false);
        self.base.push(base);
        Leaf(i)
    }

    /// Open an internal node with a placeholder base; its children are appended
    /// next, then it is closed (and normalized) with
    /// [`close_node`](Self::close_node). Return the open-node token.
    pub(super) fn open(&mut self, base: Base) -> Node {
        let i = self.len();
        self.topo.push(true);
        self.base.push(base);
        Node(i)
    }

    /// Copy the whole subtree at `src` verbatim (it is already normal form),
    /// advancing `src` past it and returning the output root. Iterative single
    /// pass: the same pending-children scan as the shared
    /// [`idbits::skip_subtree`](crate::idbits::skip_subtree) core, but it emits
    /// each visited node into the output as it goes rather than only computing
    /// the end. A synthetic `Zero` subtree copies as a fresh `Leaf(0)`.
    pub(super) fn copy_reader(&mut self, src: &mut EvReader) -> Slot {
        if matches!(src, EvReader::Zero) {
            return self.leaf(Base::ZERO).into();
        }
        let out_root = self.len();
        let mut pending: i64 = 1;
        while pending > 0 {
            let internal = match src.read() {
                EvNode::Internal(base) => {
                    self.base.push(base);
                    true
                }
                EvNode::Leaf(base) => {
                    self.base.push(base);
                    false
                }
            };
            self.topo.push(internal);
            pending += if internal { 1 } else { -1 };
        }
        Slot(out_root)
    }

    /// Finalize the internal node at `node` whose left child is at `node + 1`
    /// and right child at `right`. Sinks the children's common minimum into the
    /// node's base (`O(1)`) and collapses `(n, m, m)` of two equal leaves to a
    /// single leaf, preserving normal form. The node's root index is unchanged.
    ///
    /// Adjacency precondition for the collapse: it fires only when *both*
    /// children are leaves (the `!self.topo[..]` guards). A leaf occupies
    /// exactly one slot, so the left child is `node + 1` and the right child is
    /// `node + 2 == right` — i.e. `[node, left, right]` are the final three
    /// slots in `topo`/`base`. That is why `truncate(node)` discards exactly
    /// those three and nothing earlier before pushing the single collapsed leaf
    /// in their place.
    pub(super) fn close_node(&mut self, node: Node, right: impl Into<Slot>) -> Slot {
        let node = node.0;
        let right = right.into().0;
        let left = node + 1;
        let m = if self.base[left] <= self.base[right] {
            self.base[left].clone()
        } else {
            self.base[right].clone()
        };
        self.base[node] += &m;
        self.base[left] -= &m;
        self.base[right] -= &m;

        // Collapse only when both children are leaves of equal (post-sink) base.
        if !self.topo[left] && !self.topo[right] && self.base[left] == self.base[right] {
            debug_assert_eq!(
                right,
                node + 2,
                "collapse precondition: both children are adjacent leaves",
            );
            let collapsed = self.base[node].clone(); // the common child base is 0 after the sink
            self.topo.truncate(node);
            self.base.truncate(node);
            return self.leaf(collapsed).into();
        }
        Slot(node)
    }

    /// A leaf whose base is not yet known: emitted as a placeholder now, filled
    /// in by [`resolve`](Self::resolve) once the value it depends on exists.
    /// `fill` needs this for the one case the preorder builder cannot emit in
    /// evaluation order: an id-full *left* child collapses to a max-leaf whose
    /// value depends on its right sibling, but preorder must place the left leaf
    /// first. (An id-full *right* child needs no deferral — its left sibling is
    /// already built when the right leaf is emitted.)
    pub(super) fn deferred_leaf(&mut self) -> DeferredLeaf {
        DeferredLeaf(self.leaf(Base::ZERO).0)
    }

    /// Fill in a [`deferred_leaf`](Self::deferred_leaf)'s base once it is known.
    pub(super) fn resolve_leaf(&mut self, leaf: DeferredLeaf, base: Base) {
        self.base[leaf.0] = base;
    }

    /// The base stored at an output position (e.g. a just-built subtree's root),
    /// for computing a collapsing sibling's value.
    pub(super) fn base_of(&self, at: Slot) -> &Base {
        &self.base[at.0]
    }

    pub(super) fn finish(self) -> WorkingVersion {
        WorkingVersion {
            topo: self.topo,
            base: self.base,
        }
    }
}

/// The root position of an emitted subtree (a leaf, a closed node, or a copied
/// run): usable as a node's right child or to look up a built subtree's base.
#[derive(Clone, Copy)]
pub(super) struct Slot(usize);

/// A leaf's output position. Cheap to copy; unlike [`Node`] it needs no close.
#[derive(Clone, Copy)]
pub(super) struct Leaf(usize);

/// A just-opened internal node, awaiting its children and a
/// [`close_node`](Builder::close_node). `!Clone` and `#[must_use]`: the token
/// must be closed exactly once, and the borrow checker stops it being reused or
/// dropped silently — so an open with no matching close cannot compile.
#[must_use = "an opened node must be closed with close_node"]
pub(super) struct Node(usize);

impl From<Leaf> for Slot {
    fn from(leaf: Leaf) -> Slot {
        Slot(leaf.0)
    }
}

/// A placeholder leaf in a [`Builder`], to be filled by
/// [`Builder::resolve`](Builder::resolve). Carries the output index opaquely so
/// callers cannot poke the base array directly (see
/// [`Builder::deferred_leaf`](Builder::deferred_leaf)).
#[must_use = "deferred leaves must be resolved"]
pub(super) struct DeferredLeaf(usize);
