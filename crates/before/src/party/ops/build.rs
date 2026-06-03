use crate::codec::{Bits, BitsSlice};
use crate::idbits::IdReader;

/// A leaf id stream: `0, v`.
pub(super) fn id_leaf(v: bool) -> Bits {
    let mut out = Bits::with_capacity(2);
    out.push(false);
    out.push(v);
    out
}

/// `norm((l, r))`: build a node, collapsing `(0,0) -> 0` and `(1,1) -> 1`. A
/// 2-bit stream is exactly a leaf, so equal-valued leaf children collapse.
pub(super) fn id_node(l: &BitsSlice, r: &BitsSlice) -> Bits {
    if l.len() == 2 && r.len() == 2 && l[1] == r[1] {
        return id_leaf(l[1]);
    }
    let mut out = Bits::with_capacity(1 + l.len() + r.len());
    out.push(true);
    out.extend_from_bitslice(l);
    out.extend_from_bitslice(r);
    out
}

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

    pub(super) fn leaf(&mut self, value: bool) -> usize {
        let root = self.bits.len();
        self.bits.push(false);
        self.bits.push(value);
        root
    }

    pub(super) fn open(&mut self) -> usize {
        let root = self.bits.len();
        self.bits.push(true);
        root
    }

    /// Copy one already-normal source subtree into the output, returning its
    /// new root and a reader just past it. The source subtree is copied exactly
    /// once.
    pub(super) fn copy_reader<'a>(&mut self, src: IdReader<'a>) -> (usize, IdReader<'a>) {
        let out_root = self.bits.len();
        let end = src.skip();
        self.bits
            .extend_from_bitslice(&src.bits()[src.pos()..end.pos()]);
        (out_root, end)
    }

    /// Normalize the node opened at `node`. The left child starts immediately
    /// after the node flag; `right` is the right child's root. If both children
    /// are equal leaves, collapse the just-emitted suffix to a single leaf. The
    /// root position remains `node`.
    pub(super) fn close_node(&mut self, node: usize, right: usize) {
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
            self.leaf(value);
        }
    }

    pub(super) fn finish(self) -> Bits {
        self.bits
    }
}
