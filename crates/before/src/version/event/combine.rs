use crate::codec::Base;
use crate::recurse::descend;

use crate::version::compare::EvReader;
use crate::version::working::WorkingVersion;

use super::{Builder, Slot};

/// How two aligned leaf values combine pointwise. The join (least upper bound)
/// takes their maximum; the meet (greatest lower bound) their minimum. This is
/// the *only* thing that distinguishes the two walks — the broadcast of a leaf
/// to both of a node's children, and the normalizing sink, are identical — so
/// the walk is shared and the lattice direction is this one function.
type LeafOp = fn(Base, Base) -> Base;

impl<'a> EvReader<'a> {
    /// The least upper bound of `self` and `other` (the paper's `join` over
    /// event trees), produced in normal form. Reads either storage form via
    /// [`EvReader`]; `O(n + m)`. The recursive, offset-threaded form of
    /// `oracle::Version::join_off`: aligned leaves join to their pointwise
    /// maximum.
    pub(crate) fn join(self, other: EvReader<'a>) -> WorkingVersion {
        self.combine(other, Ord::max)
    }

    /// The greatest lower bound of `self` and `other` (the meet over event
    /// trees), produced in normal form. The order-theoretic dual of
    /// [`join`](Self::join): aligned leaves meet to their pointwise *minimum*,
    /// every other step identical. Reads either storage form via [`EvReader`];
    /// `O(n + m)`. The recursive, offset-threaded form of
    /// `oracle::Version::meet_off`.
    pub(crate) fn meet(self, other: EvReader<'a>) -> WorkingVersion {
        self.combine(other, Ord::min)
    }

    /// The shared pointwise combine of [`join`](Self::join) and
    /// [`meet`](Self::meet), parameterized by the leaf combiner `leaf_op`
    /// (`max` for the join, `min` for the meet). Guarded by [`crate::recurse`]
    /// so deep trees grow the stack onto the heap rather than overflowing. An
    /// internal side descends into its children; a leaf side broadcasts a fresh
    /// [`Zero`](EvReader::Zero) to both of the other side's children; each side
    /// hands its node sum to both children.
    fn combine(self, other: EvReader<'a>, leaf_op: LeafOp) -> WorkingVersion {
        let mut walk = CombineWalk {
            out: Builder::with_capacity(self.node_capacity_bound() + other.node_capacity_bound()),
            leaf_op,
        };
        let zero = Base::ZERO;
        let (mut a, mut b) = (self, other);
        descend!(0, walk.rec(&mut a, &zero, &mut b, &zero, 0));
        walk.out.finish()
    }
}

/// The single output builder of a [`combine`](EvReader::combine) walk plus the
/// leaf combiner that fixes the lattice direction; the `&mut` readers carry the
/// traversal state.
struct CombineWalk {
    out: Builder,
    leaf_op: LeafOp,
}

impl CombineWalk {
    /// Combine the aligned subtrees at the two `&mut` readers and path-sum
    /// offsets, emitting into `out`, advancing each reader past its subtree, and
    /// routing through the amortized stack-growth guard. Returns the output root.
    ///
    /// Reads as the paper's `join` (and unwritten meet): two leaves combine to
    /// `leaf_op` of their path sums (`max` for the join, `min` for the meet);
    /// otherwise open a node, descend each side (an internal side into its real
    /// children, a leaf side broadcasting a fresh `Zero`), and close — the
    /// close performs the normalizing sink.
    fn rec(
        &mut self,
        a: &mut EvReader,
        a_off: &Base,
        b: &mut EvReader,
        b_off: &Base,
        depth: usize,
    ) -> Slot {
        let a_node = a.read();
        let b_node = b.read();
        let a_internal = a_node.is_internal();
        let b_internal = b_node.is_internal();
        let a_sum = a_off + a_node.base();
        let b_sum = b_off + b_node.base();
        if !a_internal && !b_internal {
            // Both leaves: the combined leaf is `leaf_op` of their path sums.
            return self.out.leaf((self.leaf_op)(a_sum, b_sum)).into();
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
        self.out.close_node(node, right)
    }
}
