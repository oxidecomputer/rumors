use crate::codec::Bits;
use crate::idbits::{IdNode, IdReader};
use crate::recurse::descend;

use super::build::{Built, IdBuilder};

impl IdReader<'_> {
    /// Sum `self` and `other` (normal-form ids) — the union of their regions —
    /// producing a normalized id, or `None` if they overlap (share a region, so
    /// no disjoint union exists).
    ///
    /// This is the single point of overlap
    /// detection: callers (`Party::join`) need not pre-check
    /// [`is_disjoint`](IdReader::is_disjoint), since a successful `sum` *is* the
    /// disjointness proof. `O(n + m)`: the both-internal case threads (no
    /// skip); a `0` child copies the other subtree verbatim (work bounded by
    /// the output size).
    ///
    /// The recursive form of `oracle::Party::sum` (the paper's `sum`/`norm`),
    /// guarded by [`crate::recurse`] so deep ids grow the stack onto the heap
    /// rather than overflowing.
    pub(crate) fn sum(mut self, mut other: IdReader) -> Option<Bits> {
        let mut walk = SumWalk {
            // Conservative: the disjoint union has at most as many bits as both
            // inputs combined; normalization (collapsing `(v, v)` leaves) only
            // shrinks it. No tighter bound is cheap without doing the sum.
            out: IdBuilder::with_capacity(self.bits().len() + other.bits().len()),
        };
        descend!(0, walk.rec(&mut self, &mut other, 0))?; // `None` on overlap, discarding the partial output
        Some(walk.out.finish())
    }
}

/// The single output builder of a [`sum`](IdReader::sum) walk; the `&mut`
/// readers carry the traversal state.
struct SumWalk {
    out: IdBuilder,
}

impl SumWalk {
    /// Sum the subtrees at the two `&mut` readers, emitting into `out`,
    /// advancing both readers past their subtrees, and routing through the
    /// amortized stack-growth guard.
    ///
    /// Returns the output, or `None` the instant
    /// an overlap is found (unwinding the whole walk). Reads as a match on the
    /// two id nodes: `sum(0, b) = b`, `sum(a, 0) = a` (copy the nonempty side,
    /// skip the empty one), two nodes recurse and normalize on close, and a full
    /// side over a nonempty other is an overlap.
    ///
    /// The nodes are [`peek`](IdReader::peek)ed, not read: the copied side must
    /// stay unconsumed so `copy_reader` can splice its whole subtree.
    fn rec(&mut self, a: &mut IdReader, b: &mut IdReader, depth: usize) -> Option<Built> {
        match (a.peek(), b.peek()) {
            (IdNode::Empty, _) => {
                a.skip(); // sum(0, b) = b: skip the `0`, copy b
                Some(self.out.copy_reader(b))
            }
            (_, IdNode::Empty) => {
                let out_root = self.out.copy_reader(a); // sum(a, 0) = a
                b.skip(); // skip the `0`
                Some(out_root)
            }
            // A `1` (full) leaf meets a nonempty subtree: the two ids share a
            // region, so there is no disjoint union.
            (IdNode::Full, _) | (_, IdNode::Full) => None,
            // Both internal: consume the node headers, sum each child pair
            // (threading the real cursor into present children, a synthetic
            // `Empty` into absent ones), then close the node, which normalizes.
            (
                IdNode::Internal {
                    left: al,
                    right: ar,
                },
                IdNode::Internal {
                    left: bl,
                    right: br,
                },
            ) => {
                a.read();
                b.read();
                let node = self.out.open();
                let left = self.child(a, al, b, bl, depth)?;
                let right = self.child(a, ar, b, br, depth)?;
                Some(self.out.close_node(node, left, right))
            }
        }
    }

    /// Sum one child pair: thread the real cursor where the child is present, a
    /// synthetic [`Empty`](IdReader::Empty) where it is absent, so the
    /// `(Empty, …)` arms fire for a pruned `0` child exactly as for a stored one.
    fn child(
        &mut self,
        a: &mut IdReader,
        a_present: bool,
        b: &mut IdReader,
        b_present: bool,
        depth: usize,
    ) -> Option<Built> {
        let mut empty_a = IdReader::Empty;
        let mut empty_b = IdReader::Empty;
        let ca = if a_present { a } else { &mut empty_a };
        let cb = if b_present { b } else { &mut empty_b };
        descend!(depth + 1, self.rec(ca, cb, depth + 1))
    }
}
