use crate::codec::Base;
use crate::recurse::descend;

use crate::version::compare::EvReader;
use crate::version::working::WorkingVersion;

use super::Builder;

impl<'a> EvReader<'a> {
    /// The least upper bound of `self` and `other` (the paper's `join` over
    /// event trees), produced in normal form. Reads either storage form via
    /// [`EvReader`]; `O(n + m)`.
    ///
    /// The recursive, offset-threaded form of `oracle::Version::join_off` (the
    /// paper's `join`), guarded by [`crate::recurse`] so deep trees grow the
    /// stack onto the heap rather than overflowing. An internal side descends
    /// into its children; a leaf side broadcasts a fresh
    /// [`Zero`](EvReader::Zero) to both of the other side's children; each side
    /// hands its node sum to both children.
    pub(crate) fn join(self, other: EvReader<'a>) -> WorkingVersion {
        let mut walk = JoinWalk {
            out: Builder::with_capacity(self.node_capacity_bound() + other.node_capacity_bound()),
        };
        let zero = Base::ZERO;
        let (mut a, mut b) = (self, other);
        descend!(0, walk.rec(&mut a, &zero, &mut b, &zero, 0));
        walk.out.finish()
    }
}

/// The single output builder of a [`join`](EvReader::join) walk; the `&mut`
/// readers carry the traversal state.
struct JoinWalk {
    out: Builder,
}

impl JoinWalk {
    /// Join the aligned subtrees at the two `&mut` readers and path-sum offsets,
    /// emitting into `out`, advancing each reader past its subtree, and routing
    /// through the amortized stack-growth guard. Returns the output root.
    ///
    /// Reads as the paper's `join`: two leaves join to their pointwise maximum;
    /// otherwise open a node, descend each side (an internal side into its real
    /// children, a leaf side broadcasting a fresh `Zero`), and close — the close
    /// performs the normalizing sink.
    fn rec(
        &mut self,
        a: &mut EvReader,
        a_off: &Base,
        b: &mut EvReader,
        b_off: &Base,
        depth: usize,
    ) -> usize {
        let a_node = a.read();
        let b_node = b.read();
        let a_internal = a_node.is_internal();
        let b_internal = b_node.is_internal();
        let a_sum = a_off + a_node.base();
        let b_sum = b_off + b_node.base();
        if !a_internal && !b_internal {
            // Both leaves: the joined leaf is their pointwise maximum.
            return self.out.leaf(a_sum.max(b_sum));
        }
        // Open a node, descend each side (an internal side hands down its `&mut`
        // cursor; a leaf side broadcasts a fresh `Zero`), then close to sink.
        let node = self.out.open(Base::ZERO);
        let mut a_zero = EvReader::Zero;
        let mut b_zero = EvReader::Zero;
        descend!(
            depth + 1,
            self.rec(
                if a_internal { &mut *a } else { &mut a_zero },
                &a_sum,
                if b_internal { &mut *b } else { &mut b_zero },
                &b_sum,
                depth + 1,
            )
        );
        let right = descend!(
            depth + 1,
            self.rec(
                if a_internal { &mut *a } else { &mut a_zero },
                &a_sum,
                if b_internal { &mut *b } else { &mut b_zero },
                &b_sum,
                depth + 1,
            )
        );
        self.out.close_node(node, right);
        node
    }
}
