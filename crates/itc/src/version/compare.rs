//! The causal order on event trees: a single iterative, offset-threaded pass that
//! reads either representation in place (no transcode) and yields the comparison
//! directly.
//!
//! [`causal_cmp`] decides the causal order of `a` and `b` by a pointwise comparison of
//! their event functions, tracking both `a <= b` and `b <= a` in **one** traversal —
//! running the paper's `leq` twice would do double the work. It is the recursive `leq`
//! of the paper (plan Appendix A) made iterative, symmetric, *and* `O(n + m)`: at each
//! aligned node pair the path sums settle the local direction (`an > bn` rules out
//! `a <= b`; `bn > an` rules out `b <= a`), then the walk descends into whichever side
//! is internal, broadcasting the leaf side unchanged to both of the other's children,
//! until both bottom out — so every node of either tree is visited once. Right-child
//! positions are **threaded**: each subtree reports where it ended, so a sibling
//! resumes there instead of re-scanning the left subtree. As soon as both directions
//! are excluded the result is concurrent (`None`) and the walk stops early. Path sums
//! are threaded as `u64` offsets — the same width as the stored bases and as
//! `ev_join`/`fill`/`grow` and the oracle. They cannot overflow for any real tree: a
//! path sum is the running total of stored bases along a root-to-node path, `tick`
//! adds 1 at a time, and any tree small enough to hold in memory has a total event
//! count far below `u64::MAX`.

use core::cmp::Ordering;

use crate::codec::{decode_int, BitsSlice};
use crate::step;

use super::working::WorkingVersion;

/// A read-only view of an event tree in either storage form, addressed by a position
/// (a bit offset for packed, a node index for working). Visibility is uniform
/// `pub(super)` across the trio `EvView`/[`header`](EvView::header)/[`skip`] — used
/// throughout `version/` (compare, event, grow) and nowhere outside it.
pub(super) enum EvView<'a> {
    Packed(&'a BitsSlice),
    Working(&'a WorkingVersion),
}

impl EvView<'_> {
    /// `(is_internal, base, position-just-past-this-node's-header)`. For packed, the
    /// header is the flag bit plus the gamma-coded base; the left child (if any)
    /// begins at the returned position. For working, a node is one slot.
    pub(super) fn header(&self, at: usize) -> (bool, u64, usize) {
        // `grow` uses `super::event::VIRTUAL` (`usize::MAX`) as a sentinel "virtual leaf"
        // position and always guards `ev == VIRTUAL` before any real header read. This
        // turns a slipped guard into a loud panic instead of a silent out-of-bounds /
        // wrong-answer. Defense-in-depth only; debug builds.
        debug_assert!(
            at != super::event::VIRTUAL,
            "EvView::header called on the VIRTUAL sentinel position",
        );
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

    /// An exclusive upper bound on the positions this view addresses: the bit length
    /// for packed, the node count for working. Used to size a dense position-indexed
    /// array (see `grow`'s `Choices`).
    pub(super) fn span(&self) -> usize {
        match self {
            EvView::Packed(bits) => bits.len(),
            EvView::Working(work) => work.base.len(),
        }
    }
}

/// Advance past one whole subtree starting at `at`, returning the position after it.
/// Iterative: a pending-children counter, never the call stack. (The id-tree analogue,
/// on the packed id header shape, is [`idbits::skip`](crate::idbits::skip): same
/// algorithm, different node encoding — keep the two in step.)
pub(super) fn skip(view: &EvView, mut at: usize) -> usize {
    let mut pending: i64 = 1;
    while pending > 0 {
        let (internal, _, next) = view.header(at);
        at = next;
        pending += if internal { 1 } else { -1 };
    }
    at
}

/// A step in the threaded comparison walk.
enum Job {
    /// Compare the subtrees at these positions, under these path-sum offsets.
    Eval {
        ap: usize,
        ao: u64,
        bp: usize,
        bo: u64,
    },
    /// Both-internal node: its left child finished; launch the right child, both
    /// positions threaded from where the left child's subtrees ended.
    RightLockstep { an: u64, bn: u64 },
    /// `b` is a leaf broadcast to both of `a`'s children: launch `a`'s right child
    /// (threaded from the left child's end) against the same pinned `b` leaf.
    RightBroadcastB { an: u64, bp: usize, bo: u64 },
    /// `a` is a leaf broadcast to both of `b`'s children: launch `b`'s right child
    /// (threaded from the left child's end) against the same pinned `a` leaf.
    RightBroadcastA { ap: usize, ao: u64, bn: u64 },
}

/// The causal order, computed in one `O(n + m)` pass; `None` means concurrent.
///
/// Tracks `a <= b` (`le`) and `b <= a` (`ge`) together so the two pointwise
/// comparisons share a single traversal instead of running `leq` twice. The walk
/// descends into whichever side is internal — both in lockstep, or the internal one
/// while the leaf side is broadcast unchanged to both its children — so each node of
/// either tree is visited once. Stops early once both directions are excluded.
pub(crate) fn causal_cmp(a: &EvView, b: &EvView) -> Option<Ordering> {
    let mut le = true; // `a <= b` still possible
    let mut ge = true; // `b <= a` still possible

    // The (a, b) end positions of the most recently completed subtree; a pending right
    // child reads the threaded side(s) from here (the pinned side carries its own
    // position in the broadcast jobs).
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
                    le = false;
                }
                if bn > an {
                    ge = false;
                }
                if !le && !ge {
                    return None; // concurrent: neither direction can recover
                }
                match (a_internal, b_internal) {
                    // Both leaves: this branch is decided. Report both ends.
                    (false, false) => ret = (a_next, b_next),
                    // Both internal: descend in lockstep.
                    (true, true) => {
                        stack.push(Job::RightLockstep { an, bn });
                        stack.push(Job::Eval {
                            ap: a_next,
                            ao: an,
                            bp: b_next,
                            bo: bn,
                        });
                    }
                    // `b` leaf broadcast to both of `a`'s children.
                    (true, false) => {
                        stack.push(Job::RightBroadcastB { an, bp, bo });
                        stack.push(Job::Eval {
                            ap: a_next,
                            ao: an,
                            bp,
                            bo,
                        });
                    }
                    // `a` leaf broadcast to both of `b`'s children.
                    (false, true) => {
                        stack.push(Job::RightBroadcastA { ap, ao, bn });
                        stack.push(Job::Eval {
                            ap,
                            ao,
                            bp: b_next,
                            bo: bn,
                        });
                    }
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
            Job::RightBroadcastB { an, bp, bo } => {
                let (a_left_end, _) = ret;
                stack.push(Job::Eval {
                    ap: a_left_end,
                    ao: an,
                    bp,
                    bo,
                });
            }
            Job::RightBroadcastA { ap, ao, bn } => {
                let (_, b_left_end) = ret;
                stack.push(Job::Eval {
                    ap,
                    ao,
                    bp: b_left_end,
                    bo: bn,
                });
            }
        }
    }

    match (le, ge) {
        (true, true) => Some(Ordering::Equal),
        (true, false) => Some(Ordering::Less),
        (false, true) => Some(Ordering::Greater),
        // Unreachable: both-false returns `None` inside the loop above.
        (false, false) => None,
    }
}
