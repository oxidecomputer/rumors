//! The causal order on event trees: an iterative, offset-threaded `leq` that reads
//! either representation in place (no transcode), plus the derived comparison.
//!
//! `leq(a, b)` decides whether `a`'s event function is `<=` `b`'s pointwise. It is the
//! recursive `leq` of the paper (plan Appendix A) made iterative *and* `O(n + m)`:
//! `a` is always fully descended (every node visited once, no re-scan), while `b`
//! either descends in lockstep (both internal) or is broadcast unchanged to both of
//! `a`'s children (when `b` is a leaf). Right-child positions are **threaded** — the
//! walk reports where each subtree ended, so a sibling resumes there instead of
//! skipping the left subtree. The one place `b` is skipped is under an `a` leaf, which
//! dominates `b`'s whole subtree there; each `b` node is skipped at most once, so the
//! total stays linear. Path sums are threaded as `u64` offsets — the same width as the
//! stored bases and as `ev_join`/`fill`/`grow` and the oracle. They cannot overflow for
//! any real tree: a path sum is the running total of stored bases along a root-to-node
//! path, `tick` adds 1 at a time, and any tree small enough to hold in memory has a
//! total event count far below `u64::MAX`.

use core::cmp::Ordering;

use crate::codec::{decode_int, BitsSlice};
use crate::step;

use super::working::WorkingVersion;

/// A read-only view of an event tree in either storage form, addressed by a position
/// (a bit offset for packed, a node index for working).
pub(crate) enum EvView<'a> {
    Packed(&'a BitsSlice),
    Working(&'a WorkingVersion),
}

impl EvView<'_> {
    /// `(is_internal, base, position-just-past-this-node's-header)`. For packed, the
    /// header is the flag bit plus the gamma-coded base; the left child (if any)
    /// begins at the returned position. For working, a node is one slot.
    pub(super) fn header(&self, at: usize) -> (bool, u64, usize) {
        step!();
        match self {
            EvView::Packed(bits) => {
                let internal = bits[at];
                let (base, next) = decode_int(bits, at + 1).expect("canonical event bits");
                (internal, base, next)
            }
            EvView::Working(work) => (work.topo[at], work.base[at], at + 1),
        }
    }
}

/// Advance past one whole subtree starting at `at`, returning the position after it.
/// Iterative: a pending-children counter, never the call stack.
pub(super) fn skip(view: &EvView, mut at: usize) -> usize {
    let mut pending: i64 = 1;
    while pending > 0 {
        let (internal, _, next) = view.header(at);
        at = next;
        pending += if internal { 1 } else { -1 };
    }
    at
}

/// A step in the threaded `leq` walk.
enum Job {
    /// Compare the subtrees at these positions, under these path-sum offsets.
    Eval {
        ap: usize,
        ao: u64,
        bp: usize,
        bo: u64,
    },
    /// Both-internal node: its left child finished; launch the right child, whose `b`
    /// position is threaded from where the left child's `b` subtree ended.
    RightLockstep { an: u64, bn: u64 },
    /// Broadcast node (`b` is a leaf): launch the right child against the same pinned
    /// `b` leaf, with `a`'s position threaded from the left child's end.
    RightBroadcast { an: u64, bp: usize, bo: u64 },
}

/// Whether `a`'s event function is pointwise `<=` `b`'s. Iterative and `O(n + m)`.
fn leq(a: &EvView, b: &EvView) -> bool {
    // The (a, b) end positions of the most recently completed subtree; a pending right
    // child reads `ret.0` (its `a` start, always threaded) and, in lockstep, `ret.1`.
    let mut ret = (0usize, 0usize);
    let mut stack = vec![Job::Eval {
        ap: 0,
        ao: 0,
        bp: 0,
        bo: 0,
    }];
    while let Some(job) = stack.pop() {
        match job {
            Job::Eval { ap, ao, bp, bo } => {
                let (a_internal, a_base, a_next) = a.header(ap);
                let (b_internal, b_base, b_next) = b.header(bp);
                let an = ao + a_base;
                let bn = bo + b_base;
                if an > bn {
                    return false;
                }
                if !a_internal {
                    // `a` leaf: dominated everywhere below. Report `b`'s subtree end so
                    // the parent can resume — the one bounded lazy-skip of `b`.
                    ret = (a_next, skip(b, bp));
                    continue;
                }
                let a_left = a_next;
                if b_internal {
                    stack.push(Job::RightLockstep { an, bn });
                    stack.push(Job::Eval {
                        ap: a_left,
                        ao: an,
                        bp: b_next,
                        bo: bn,
                    });
                } else {
                    stack.push(Job::RightBroadcast { an, bp, bo });
                    stack.push(Job::Eval {
                        ap: a_left,
                        ao: an,
                        bp,
                        bo,
                    });
                }
            }
            Job::RightLockstep { an, bn } => {
                let (a_left_end, b_left_end) = ret;
                stack.push(Job::Eval {
                    ap: a_left_end,
                    ao: an,
                    bp: b_left_end,
                    bo: bn,
                });
            }
            Job::RightBroadcast { an, bp, bo } => {
                let (a_left_end, _) = ret;
                stack.push(Job::Eval {
                    ap: a_left_end,
                    ao: an,
                    bp,
                    bo,
                });
            }
        }
    }
    true
}

/// The causal order; `None` means concurrent.
pub(crate) fn causal_cmp(a: &EvView, b: &EvView) -> Option<Ordering> {
    match (leq(a, b), leq(b, a)) {
        (true, true) => Some(Ordering::Equal),
        (true, false) => Some(Ordering::Less),
        (false, true) => Some(Ordering::Greater),
        (false, false) => None,
    }
}
