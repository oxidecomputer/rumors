use crate::codec::Bits;
use crate::idbits::IdView;

use super::build::IdBuilder;

/// A step in the threaded `sum` build. `a_pos`/`b_pos` are bit offsets into the
/// two id streams.
enum SumJob {
    /// Sum the subtrees rooted at these positions.
    Eval { a_pos: usize, b_pos: usize },
    /// Left finished; launch the right child from its end (read off `ret`).
    Right,
    /// Both children built; combine them into a normalized node.
    Combine {
        /// Output bit position of the node being closed.
        node: usize,
    },
}

/// A built `sum` subtree report — the register analogue for `sum`: the
/// subtree's output root, plus where it ended in each input. `Eval` writes one
/// for a copied side; `Right` reads it to launch the sibling; `Combine`
/// rewrites it with the joined node's report after finalizing that node in the
/// output builder.
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
    /// The iterative form of the recursive `oracle::Party::sum` (the paper's
    /// `sum`/`norm`); read that recursive twin first, then this is the same
    /// algorithm with the call stack made explicit on the `SumJob` stack.
    pub(crate) fn sum(&self, other: &IdView) -> Option<Bits> {
        let (a, b) = (*self, *other);
        let mut out = IdBuilder::with_capacity(a.bits().len() + b.bits().len());
        let mut ret = Summed::default();
        let mut stack = vec![SumJob::Eval { a_pos: 0, b_pos: 0 }];
        while let Some(job) = stack.pop() {
            match job {
                SumJob::Eval { a_pos, b_pos } => {
                    let a_hdr = a.header(a_pos);
                    let b_hdr = b.header(b_pos);
                    let (a_node, a_next) = (a_hdr.node, a_hdr.next);
                    let (b_node, b_next) = (b_hdr.node, b_hdr.next);
                    if a_hdr.is_empty() {
                        let (out_root, b_end) = out.copy(b, b_pos); // sum(0, b) = b
                        ret = Summed {
                            out_root,
                            a_end: a_next,
                            b_end,
                        };
                    } else if b_hdr.is_empty() {
                        let (out_root, a_end) = out.copy(a, a_pos); // sum(a, 0) = a
                        ret = Summed {
                            out_root,
                            a_end,
                            b_end: b_next,
                        };
                    } else if a_node && b_node {
                        let node = out.open();
                        stack.push(SumJob::Combine { node });
                        stack.push(SumJob::Right);
                        stack.push(SumJob::Eval {
                            a_pos: a_next,
                            b_pos: b_next,
                        }); // left
                    } else {
                        // A `1` (full) leaf meets a nonempty subtree on the other side: the
                        // two ids share a region, so there is no disjoint union.
                        return None;
                    }
                }
                SumJob::Right => {
                    let left = ret;
                    stack.push(SumJob::Eval {
                        a_pos: left.a_end,
                        b_pos: left.b_end,
                    });
                }
                SumJob::Combine { node } => {
                    let right = ret;
                    out.close_node(node, right.out_root);
                    // The node's ends are the right child's (it was threaded last).
                    ret = Summed {
                        out_root: node,
                        a_end: right.a_end,
                        b_end: right.b_end,
                    };
                }
            }
        }
        Some(out.finish())
    }
}
