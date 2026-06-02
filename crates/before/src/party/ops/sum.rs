use crate::codec::Bits;
use crate::idbits::IdView;
use crate::recurse::descend;

use super::build::IdBuilder;

/// A built `sum` subtree report: the subtree's output root, plus where it ended
/// in each input (so a right sibling resumes without re-scanning).
#[derive(Clone, Copy, Default)]
struct Summed {
    /// Output bit position of the subtree's root.
    out_root: usize,
    /// Position just past the subtree in `a`.
    a_end: usize,
    /// Position just past the subtree in `b`.
    b_end: usize,
}

impl IdView<'_> {
    /// Sum `self` and `other` (normal-form ids) — the union of their regions —
    /// producing a normalized id, or `None` if they overlap (share a region, so
    /// no disjoint union exists). This is the single point of overlap
    /// detection: callers (`Party::join`) need not pre-check
    /// [`is_disjoint`](IdView::is_disjoint), since a successful `sum` *is* the
    /// disjointness proof. `O(n + m)`: the both-internal case threads (no
    /// skip); a `0` child copies the other subtree verbatim (work bounded by
    /// the output size).
    ///
    /// The recursive form of `oracle::Party::sum` (the paper's `sum`/`norm`),
    /// guarded by [`crate::recurse`] so deep ids grow the stack onto the heap
    /// rather than overflowing.
    pub(crate) fn sum(&self, other: &IdView) -> Option<Bits> {
        let mut walk = SumWalk {
            a: *self,
            b: *other,
            out: IdBuilder::with_capacity(self.bits().len() + other.bits().len()),
        };
        descend!(0, walk.rec(0, 0, 0))?; // `None` on overlap, discarding the partial output
        Some(walk.out.finish())
    }
}

/// The mutable state of a [`sum`](IdView::sum) walk: the two id views and the
/// single output builder threaded through the recursion.
struct SumWalk<'a> {
    a: IdView<'a>,
    b: IdView<'a>,
    out: IdBuilder,
}

impl SumWalk<'_> {
    /// Sum the subtrees rooted at the given positions, emitting into `out` and
    /// routing through the amortized stack-growth guard. Returns the subtree's
    /// [`Summed`] report, or `None` the instant an overlap is found (unwinding
    /// the whole walk).
    fn rec(&mut self, a_pos: usize, b_pos: usize, depth: usize) -> Option<Summed> {
        let a_hdr = self.a.header(a_pos);
        let b_hdr = self.b.header(b_pos);
        let (a_node, a_next) = (a_hdr.node, a_hdr.next);
        let (b_node, b_next) = (b_hdr.node, b_hdr.next);
        if a_hdr.is_empty() {
            let (out_root, b_end) = self.out.copy(self.b, b_pos); // sum(0, b) = b
            return Some(Summed {
                out_root,
                a_end: a_next,
                b_end,
            });
        }
        if b_hdr.is_empty() {
            let (out_root, a_end) = self.out.copy(self.a, a_pos); // sum(a, 0) = a
            return Some(Summed {
                out_root,
                a_end,
                b_end: b_next,
            });
        }
        if a_node && b_node {
            // Both internal: descend (left now, right threaded from where the
            // left ended), then close the node. Its ends are the right child's
            // (it was threaded last).
            let node = self.out.open();
            let left = descend!(depth + 1, self.rec(a_next, b_next, depth + 1))?;
            let right = descend!(depth + 1, self.rec(left.a_end, left.b_end, depth + 1))?;
            self.out.close_node(node, right.out_root);
            return Some(Summed {
                out_root: node,
                a_end: right.a_end,
                b_end: right.b_end,
            });
        }
        // A `1` (full) leaf meets a nonempty subtree on the other side: the two
        // ids share a region, so there is no disjoint union.
        None
    }
}
