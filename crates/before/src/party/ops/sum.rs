use crate::codec::Bits;
use crate::idbits::{IdNode, IdReader};
use crate::recurse::descend;

use super::build::IdBuilder;

/// A built `sum` subtree report: the subtree's output root, plus readers past
/// it in each input (so a right sibling resumes without re-scanning).
struct Summed<'a> {
    /// Output bit position of the subtree's root.
    out_root: usize,
    /// Reader just past the subtree in `a`.
    a_end: IdReader<'a>,
    /// Reader just past the subtree in `b`.
    b_end: IdReader<'a>,
}

impl IdReader<'_> {
    /// Sum `self` and `other` (normal-form ids) — the union of their regions —
    /// producing a normalized id, or `None` if they overlap (share a region, so
    /// no disjoint union exists). This is the single point of overlap
    /// detection: callers (`Party::join`) need not pre-check
    /// [`is_disjoint`](IdReader::is_disjoint), since a successful `sum` *is* the
    /// disjointness proof. `O(n + m)`: the both-internal case threads (no
    /// skip); a `0` child copies the other subtree verbatim (work bounded by
    /// the output size).
    ///
    /// The recursive form of `oracle::Party::sum` (the paper's `sum`/`norm`),
    /// guarded by [`crate::recurse`] so deep ids grow the stack onto the heap
    /// rather than overflowing.
    pub(crate) fn sum(self, other: IdReader) -> Option<Bits> {
        let mut walk = SumWalk {
            // Conservative: the disjoint union has at most as many bits as both
            // inputs combined; normalization (collapsing `(v, v)` leaves) only
            // shrinks it. No tighter bound is cheap without doing the sum.
            out: IdBuilder::with_capacity(self.bits().len() + other.bits().len()),
        };
        descend!(0, walk.rec(self, other, 0))?; // `None` on overlap, discarding the partial output
        Some(walk.out.finish())
    }
}

/// The single output builder of a [`sum`](IdReader::sum) walk; the readers carry
/// the traversal state.
struct SumWalk {
    out: IdBuilder,
}

impl SumWalk {
    /// Sum the subtrees at the two readers, emitting into `out` and routing
    /// through the amortized stack-growth guard. Returns the subtree's
    /// [`Summed`] report, or `None` the instant an overlap is found (unwinding
    /// the whole walk). Reads as a match on the two id nodes: `sum(0, b) = b`,
    /// `sum(a, 0) = a` (copy the nonempty side), two nodes recurse and normalize
    /// on close, and a full side over a nonempty other is an overlap.
    fn rec<'a>(&mut self, a: IdReader<'a>, b: IdReader<'a>, depth: usize) -> Option<Summed<'a>> {
        let (a_node, a_after) = a.read();
        let (b_node, b_after) = b.read();
        match (a_node, b_node) {
            (IdNode::Empty, _) => {
                let (out_root, b_end) = self.out.copy_reader(b); // sum(0, b) = b
                Some(Summed {
                    out_root,
                    a_end: a_after,
                    b_end,
                })
            }
            (_, IdNode::Empty) => {
                let (out_root, a_end) = self.out.copy_reader(a); // sum(a, 0) = a
                Some(Summed {
                    out_root,
                    a_end,
                    b_end: b_after,
                })
            }
            // A `1` (full) leaf meets a nonempty subtree: the two ids share a
            // region, so there is no disjoint union.
            (IdNode::Full, _) | (_, IdNode::Full) => None,
            // Both internal: descend (left now, right threaded from where the
            // left ended), then close the node, which normalizes.
            (IdNode::Internal, IdNode::Internal) => {
                let node = self.out.open();
                let left = descend!(depth + 1, self.rec(a_after, b_after, depth + 1))?;
                let right = descend!(depth + 1, self.rec(left.a_end, left.b_end, depth + 1))?;
                self.out.close_node(node, right.out_root);
                Some(Summed {
                    out_root: node,
                    a_end: right.a_end,
                    b_end: right.b_end,
                })
            }
        }
    }
}
