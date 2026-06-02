use crate::codec::Base;

use crate::version::compare::{EvHeader, EvView};
use crate::version::working::WorkingVersion;

use super::Builder;

/// A built `join` subtree report: the output root a just-finished subtree
/// produced, plus where it ended in each input (so a right sibling resumes
/// without re-scanning).
#[derive(Clone, Copy, Default)]
struct Joined {
    /// Output index of the subtree's root.
    out_root: usize,
    /// Position just past the subtree in `a`.
    a_end: usize,
    /// Position just past the subtree in `b`.
    b_end: usize,
}

impl EvView<'_> {
    /// The least upper bound of `self` and `other` (the paper's `join` over
    /// event trees), produced in normal form. Reads either storage form via
    /// [`EvView`]; `O(n + m)`.
    ///
    /// The recursive, offset-threaded form of `oracle::Version::join_off` (the
    /// paper's `join`), guarded by [`crate::recurse`] so deep trees grow the
    /// stack onto the heap rather than overflowing. The leaf/node broadcast rule
    /// is inlined: an internal side descends (threading its right child from
    /// where the left ended), a leaf side re-broadcasts in place to both of the
    /// other side's children, and each side hands the same offset — its node sum
    /// when internal, its own offset when a leaf — to both children by reference.
    pub(crate) fn join(&self, other: &EvView) -> WorkingVersion {
        let mut walk = JoinWalk {
            a: *self,
            b: *other,
            out: Builder::with_capacity(self.node_capacity_bound() + other.node_capacity_bound()),
        };
        let zero = Base::ZERO;
        walk.rec(0, &zero, 0, &zero, 0);
        walk.out.finish()
    }
}

/// The mutable state of a [`join`](EvView::join) walk: the two views and the
/// single output builder threaded through the recursion. Per-node path-sum
/// offsets stay borrowed (`&Base`); each side's single child offset is shared by
/// reference between its two children, so the walk clones no path sums.
struct JoinWalk<'a> {
    a: EvView<'a>,
    b: EvView<'a>,
    out: Builder,
}

impl JoinWalk<'_> {
    /// Join the aligned subtrees at the given positions and path-sum offsets,
    /// emitting into `out` and routing through the amortized stack-growth guard.
    /// Returns the subtree's [`Joined`] report.
    fn rec(
        &mut self,
        a_pos: usize,
        a_off: &Base,
        b_pos: usize,
        b_off: &Base,
        depth: usize,
    ) -> Joined {
        crate::recurse::guarded(depth, move || {
            let EvHeader {
                internal: a_internal,
                base: a_base,
                next: a_next,
            } = self.a.header(a_pos);
            let EvHeader {
                internal: b_internal,
                base: b_base,
                next: b_next,
            } = self.b.header(b_pos);
            let a_sum = a_off + &a_base;
            let b_sum = b_off + &b_base;
            if !a_internal && !b_internal {
                // Both leaves: the joined leaf is their pointwise maximum.
                return Joined {
                    out_root: self.out.leaf(a_sum.max(b_sum)),
                    a_end: a_next,
                    b_end: b_next,
                };
            }
            // At least one side is internal: descend it, broadcast the other
            // leaf. Each side hands the same offset to both children — its node
            // sum when internal, its own offset when a leaf — by reference. The
            // positions differ: an internal side descends (left at `next`, right
            // threaded from where the left ended), a leaf re-broadcasts in place.
            let node = self.out.open(Base::ZERO);
            let a_child: &Base = if a_internal { &a_sum } else { a_off };
            let b_child: &Base = if b_internal { &b_sum } else { b_off };
            let a_left = if a_internal { a_next } else { a_pos };
            let b_left = if b_internal { b_next } else { b_pos };
            let left = self.rec(a_left, a_child, b_left, b_child, depth + 1);
            let a_right = if a_internal { left.a_end } else { a_pos };
            let b_right = if b_internal { left.b_end } else { b_pos };
            let right = self.rec(a_right, a_child, b_right, b_child, depth + 1);
            self.out.close_node(node, right.out_root);
            // The node ends where each internal side's right subtree ended; a
            // re-broadcast leaf side ends just past its own header.
            Joined {
                out_root: node,
                a_end: if a_internal { right.a_end } else { a_next },
                b_end: if b_internal { right.b_end } else { b_next },
            }
        })
    }
}
