use crate::codec::Base;
use crate::recurse::descend;

use crate::version::compare::{EvReader, Side};
use crate::version::working::WorkingVersion;

use super::Builder;

impl<'a> EvReader<'a> {
    /// The least upper bound of `self` and `other` (the paper's `join` over
    /// event trees), produced in normal form. Reads either storage form via
    /// [`EvReader`]; `O(n + m)`.
    ///
    /// The recursive, offset-threaded form of `oracle::Version::join_off` (the
    /// paper's `join`), guarded by [`crate::recurse`] so deep trees grow the
    /// stack onto the heap rather than overflowing. The leaf/node broadcast is
    /// the shared [`Side`] helper: an internal side descends, a leaf side
    /// broadcasts a [`Zero`](EvReader::Zero) to both of the other side's
    /// children, and each side hands its node sum to both children.
    pub(crate) fn join(self, other: EvReader<'a>) -> WorkingVersion {
        let mut walk = JoinWalk {
            out: Builder::with_capacity(self.node_capacity_bound() + other.node_capacity_bound()),
        };
        let zero = Base::ZERO;
        descend!(0, walk.rec(self, &zero, other, &zero, 0));
        walk.out.finish()
    }
}

/// The single output builder of a [`join`](EvReader::join) walk; the readers
/// carry the traversal state.
struct JoinWalk {
    out: Builder,
}

/// A built `join` subtree: the output root it produced, plus the readers past
/// each input (so a right sibling resumes without re-scanning).
struct Joined<'a> {
    out_root: usize,
    a_end: EvReader<'a>,
    b_end: EvReader<'a>,
}

impl JoinWalk {
    /// Join the aligned subtrees at the two readers and path-sum offsets,
    /// emitting into `out` and routing through the amortized stack-growth guard.
    ///
    /// Reads as the paper's `join`: two leaves join to their pointwise maximum;
    /// otherwise open a node, descend each side (via [`Side`], an internal side
    /// into its children, a leaf side broadcasting `Zero`), and close — the
    /// close performs the normalizing sink.
    fn rec<'a>(
        &mut self,
        a: EvReader<'a>,
        a_off: &Base,
        b: EvReader<'a>,
        b_off: &Base,
        depth: usize,
    ) -> Joined<'a> {
        let (a_node, a_after) = a.read();
        let (b_node, b_after) = b.read();
        let a_sum = a_off + a_node.base();
        let b_sum = b_off + b_node.base();
        if !a_node.is_internal() && !b_node.is_internal() {
            // Both leaves: the joined leaf is their pointwise maximum.
            return Joined {
                out_root: self.out.leaf(a_sum.max(b_sum)),
                a_end: a_after,
                b_end: b_after,
            };
        }
        let sa = Side::new(&a_node, a_after, a_off);
        let sb = Side::new(&b_node, b_after, b_off);
        let node = self.out.open(Base::ZERO);
        let left = descend!(
            depth + 1,
            self.rec(sa.left(), &sa.sum, sb.left(), &sb.sum, depth + 1)
        );
        let right = descend!(
            depth + 1,
            self.rec(
                sa.right(left.a_end),
                &sa.sum,
                sb.right(left.b_end),
                &sb.sum,
                depth + 1
            )
        );
        self.out.close_node(node, right.out_root);
        Joined {
            out_root: node,
            a_end: sa.end(right.a_end),
            b_end: sb.end(right.b_end),
        }
    }
}
