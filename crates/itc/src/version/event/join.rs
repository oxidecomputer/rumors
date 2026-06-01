use crate::codec::Base;

use crate::version::compare::{EvHeader, EvView, Side};
use crate::version::working::WorkingVersion;

use super::Builder;

/// A step in the threaded two-tree `join` walk. `ret` is the [`Joined`] register (see
/// the module doc). The broadcast rule — an internal side threads/descends, a leaf side
/// re-broadcasts in place — lives once in the [`Side`] helpers (`left`/`right`/`end`),
/// not spelled out per side per arm.
enum JoinJob {
    /// Join the subtrees at these positions, under these path-sum offsets.
    Eval {
        /// Position of `a`'s subtree root.
        a_pos: usize,
        /// Path sum down to `a`'s subtree.
        a_off: Base,
        /// Position of `b`'s subtree root.
        b_pos: usize,
        /// Path sum down to `b`'s subtree.
        b_off: Base,
    },
    /// Left child finished; launch the right child. Each [`Side`] threads its internal
    /// side from `ret` and re-broadcasts its leaf side in place.
    Right {
        /// `a`'s state at this node.
        a: Side,
        /// `b`'s state at this node.
        b: Side,
    },
    /// Right child finished; sink and close the node, reporting its end positions (each
    /// [`Side::end`] picks the threaded child end or the pinned leaf `next`).
    Close {
        /// Output index of the node being closed.
        node: usize,
        /// `a`'s state at this node.
        a: Side,
        /// `b`'s state at this node.
        b: Side,
    },
}

/// The thread register for `join` (see the module doc): the output root a
/// just-finished subtree produced, plus where it ended in each input. An `Eval` arm
/// *writes* it (a leaf directly, or via the `Close` arm folding a node); the deferred
/// `Right`/`Close` frames *read* it.
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
    /// The least upper bound of `self` and `other` (the paper's `join` over event trees),
    /// produced in normal form. Reads either storage form via [`EvView`]; `O(n + m)`.
    ///
    /// The iterative, offset-threaded form of the recursive `oracle::Version::join_off` (the
    /// paper's `join`); read that recursive twin first. The call stack is made explicit on a
    /// `JoinJob` stack, right-child positions are threaded through the [`Joined`] register,
    /// and the leaf/node broadcast rule lives in the [`Side`] helpers.
    pub(crate) fn join(&self, other: &EvView) -> WorkingVersion {
        let (a, b) = (self, other);
        let mut out = Builder::with_capacity(a.node_capacity_bound() + b.node_capacity_bound());
        let mut ret = Joined::default();
        let mut stack = vec![JoinJob::Eval {
            a_pos: 0,
            a_off: Base::ZERO,
            b_pos: 0,
            b_off: Base::ZERO,
        }];
        while let Some(job) = stack.pop() {
            match job {
                JoinJob::Eval {
                    a_pos,
                    a_off,
                    b_pos,
                    b_off,
                } => {
                    let EvHeader {
                        internal: a_internal,
                        base: a_base,
                        next: a_next,
                    } = a.header(a_pos);
                    let EvHeader {
                        internal: b_internal,
                        base: b_base,
                        next: b_next,
                    } = b.header(b_pos);
                    let a_sum = a_off.clone() + a_base;
                    let b_sum = b_off.clone() + b_base;
                    if !a_internal && !b_internal {
                        ret = Joined {
                            out_root: out.leaf(a_sum.max(b_sum)),
                            a_end: a_next,
                            b_end: b_next,
                        };
                        continue;
                    }
                    let node = out.open(Base::ZERO);
                    // At least one side is internal: descend it, broadcast the other leaf.
                    // The [`Side`] helpers carry the one broadcast rule for both children
                    // and the node close.
                    let a_side = Side {
                        internal: a_internal,
                        pos: a_pos,
                        off: a_off,
                        sum: a_sum,
                        next: a_next,
                    };
                    let b_side = Side {
                        internal: b_internal,
                        pos: b_pos,
                        off: b_off,
                        sum: b_sum,
                        next: b_next,
                    };
                    let (left_a_pos, left_a_off) = a_side.left();
                    let (left_b_pos, left_b_off) = b_side.left();
                    stack.push(JoinJob::Close {
                        node,
                        a: a_side.clone(),
                        b: b_side.clone(),
                    });
                    stack.push(JoinJob::Right {
                        a: a_side,
                        b: b_side,
                    });
                    stack.push(JoinJob::Eval {
                        a_pos: left_a_pos,
                        a_off: left_a_off,
                        b_pos: left_b_pos,
                        b_off: left_b_off,
                    });
                }
                JoinJob::Right { a, b } => {
                    let (right_a_pos, right_a_off) = a.right(ret.a_end);
                    let (right_b_pos, right_b_off) = b.right(ret.b_end);
                    stack.push(JoinJob::Eval {
                        a_pos: right_a_pos,
                        a_off: right_a_off,
                        b_pos: right_b_pos,
                        b_off: right_b_off,
                    });
                }
                JoinJob::Close { node, a, b } => {
                    out.close_node(node, ret.out_root);
                    ret = Joined {
                        out_root: node,
                        a_end: a.end(ret.a_end),
                        b_end: b.end(ret.b_end),
                    };
                }
            }
        }
        out.finish()
    }
}
