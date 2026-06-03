use crate::codec::Bits;
use crate::idbits::IdReader;

/// Single-buffer builder for normalized id output. A node is opened before its
/// children are emitted, then closed once the right child is known; if both
/// children are equal leaves, the just-emitted `[node, left, right]` suffix is
/// replaced by the collapsed leaf. This mirrors the event-side `Builder`, but
/// the id payload is only bits.
pub(super) struct IdBuilder {
    bits: Bits,
}

impl IdBuilder {
    pub(super) fn with_capacity(capacity: usize) -> Self {
        IdBuilder {
            bits: Bits::with_capacity(capacity),
        }
    }

    /// Append a leaf with the given value; return its output position.
    pub(super) fn leaf(&mut self, value: bool) -> Leaf {
        let root = self.bits.len();
        self.bits.push(false);
        self.bits.push(value);
        Leaf(root)
    }

    /// Open an internal node; its children are emitted next, then it is closed
    /// (and normalized) with [`close_node`](Self::close_node).
    pub(super) fn open(&mut self) -> Node {
        let root = self.bits.len();
        self.bits.push(true);
        Node(root)
    }

    /// Copy one already-normal source subtree into the output, advancing `src`
    /// past it and returning its new root. The source subtree is copied exactly
    /// once (a verbatim bit-range splice).
    pub(super) fn copy_reader(&mut self, src: &mut IdReader) -> Slot {
        let out_root = self.bits.len();
        let start = src.pos();
        src.skip();
        self.bits
            .extend_from_bitslice(&src.bits()[start..src.pos()]);
        Slot(out_root)
    }

    /// Normalize and close the node opened at `node`, consuming the open-node
    /// token. The left child starts immediately after the node flag; `right` is
    /// the right child's root. If both children are equal leaves, collapse the
    /// just-emitted suffix to a single leaf. Returns the node's root position,
    /// which is unchanged by the collapse.
    pub(super) fn close_node(&mut self, node: Node, right: impl Into<Slot>) -> Slot {
        let node = node.0;
        let right = right.into().0;
        let left = node + 1;
        if !self.bits[left] && !self.bits[right] && self.bits[left + 1] == self.bits[right + 1] {
            debug_assert_eq!(
                right,
                left + 2,
                "equal-leaf children must be adjacent in the output suffix",
            );
            debug_assert_eq!(
                self.bits.len(),
                right + 2,
                "closing node must be the most recently emitted subtree",
            );
            let value = self.bits[left + 1];
            self.bits.truncate(node);
            return self.leaf(value).into();
        }
        Slot(node)
    }

    pub(super) fn finish(self) -> Bits {
        self.bits
    }
}

/// The root position of an emitted subtree (a leaf, a closed node, or a copied
/// run): usable as a node's right child or to refer back to a built subtree.
#[derive(Clone, Copy)]
pub(super) struct Slot(usize);

/// A leaf's output position. Cheap to copy; unlike [`Node`] it needs no close.
#[derive(Clone, Copy)]
pub(super) struct Leaf(usize);

/// A just-opened internal node, awaiting its children and a
/// [`close_node`](IdBuilder::close_node). `!Clone` and `#[must_use]`: the token
/// must be closed exactly once, and the borrow checker stops it being reused or
/// dropped silently — so an open with no matching close cannot compile.
#[must_use = "an opened node must be closed with close_node"]
pub(super) struct Node(usize);

impl From<Leaf> for Slot {
    fn from(leaf: Leaf) -> Slot {
        Slot(leaf.0)
    }
}
