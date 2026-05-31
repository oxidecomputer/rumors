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
//! are threaded as arbitrary-precision [`Base`](crate::codec::Base) offsets — the same
//! value type as the stored bases and as `ev_join`/`fill`/`grow`. A path sum is the
//! running total of stored bases along a root-to-node path; an unbounded integer type
//! removes the `u64` overflow class entirely (`decode` admits any normal-form tree,
//! including one whose path sums exceed `u64::MAX`, so a bounded accumulator could
//! wrap and invert the causal order).

use core::cmp::Ordering;

use crate::codec::{decode_int, Base, BitsSlice};
use crate::{idbits, step};

use super::working::WorkingVersion;

/// The decoded event-node header at a position: whether the node is internal, its
/// stored base, and where the next node begins. Returned by [`EvView::header`].
pub(super) struct EvHeader {
    /// Whether this node is internal (has two children) rather than a leaf.
    pub(super) internal: bool,
    /// This node's stored base, an arbitrary-precision [`Base`].
    pub(super) base: Base,
    /// Position just past this node's header (where its left child, if any, begins).
    pub(super) next: usize,
}

/// A read-only view of an event tree in either storage form, addressed by a position
/// (a bit offset for packed, a node index for working). Visibility is uniform
/// `pub(super)` across the trio `EvView`/[`header`](EvView::header)/[`skip`] — used
/// throughout `version/` (compare, event, grow) and nowhere outside it.
pub(super) enum EvView<'a> {
    Packed(&'a BitsSlice),
    Working(&'a WorkingVersion),
}

impl EvView<'_> {
    /// Decode the event-node header at `at`. For packed, the header is the flag bit plus
    /// the gamma-coded base; the left child (if any) begins at [`EvHeader::next`]. For
    /// working, a node is one slot. The base is an arbitrary-precision [`Base`], returned
    /// by value (cloned from the working store).
    pub(super) fn header(&self, at: usize) -> EvHeader {
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
                EvHeader {
                    internal,
                    base,
                    next,
                }
            }
            EvView::Working(work) => EvHeader {
                internal: work.topo[at],
                base: work.base[at].clone(),
                next: at + 1,
            },
        }
    }

    /// Whether the two views are *trivially* equal: the same storage form with
    /// byte-for-byte identical contents. Both forms are always in canonical normal
    /// form (a stored `Version` is canonical; the working form is kept normal by
    /// `event::Builder`), so identical contents is exactly semantic equality — which
    /// lets [`causal_cmp`] settle `Equal` with one length-checked memcmp instead of
    /// the full `O(n + m)` walk and its heap-allocated job stack. A representation
    /// mismatch (one packed, one working) declines to `false` and falls through:
    /// proving equality across forms would mean transcoding one side, no cheaper than
    /// the walk itself.
    pub(super) fn trivially_eq(&self, other: &EvView) -> bool {
        match (self, other) {
            (EvView::Packed(a), EvView::Packed(b)) => a == b,
            (EvView::Working(a), EvView::Working(b)) => a.topo == b.topo && a.base == b.base,
            _ => false,
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
/// Iterative: a pending-children counter, never the call stack — see the shared
/// [`idbits::skip_subtree`](crate::idbits::skip_subtree) core. (The id-tree analogue, on
/// the packed id header shape, is [`idbits::skip`](crate::idbits::skip): same algorithm
/// via the same core, different node encoding.)
pub(super) fn skip(view: &EvView, at: usize) -> usize {
    idbits::skip_subtree(at, |pos| {
        let h = view.header(pos);
        (h.internal, h.next)
    })
}

/// A step in the threaded comparison walk. Positions (`*_pos`) address a node in each
/// tree; offsets (`*_off`) are the path sum down to that node; sums (`*_sum`) are the
/// offset plus the node's own base — the value the node contributes pointwise.
enum Job {
    /// Compare the subtrees at these positions, under these path-sum offsets.
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
    /// Both-internal node: its left child finished; launch the right child, both
    /// positions threaded from where the left child's subtrees ended (read from `ret`)
    /// and offset by the node sums.
    RightLockstep { a_sum: Base, b_sum: Base },
    /// `b` is a leaf broadcast to both of `a`'s children: launch `a`'s right child
    /// (its position threaded from `ret`, offset by `a_sum`) against the same pinned `b`
    /// leaf (its `b_pos`/`b_off` carried here, not threaded).
    RightBroadcastB {
        a_sum: Base,
        b_pos: usize,
        b_off: Base,
    },
    /// `a` is a leaf broadcast to both of `b`'s children: launch `b`'s right child
    /// (its position threaded from `ret`, offset by `b_sum`) against the same pinned `a`
    /// leaf (its `a_pos`/`a_off` carried here, not threaded).
    RightBroadcastA {
        a_pos: usize,
        a_off: Base,
        b_sum: Base,
    },
}

/// The thread register for the comparison walk (the discipline is documented in
/// [`super::event`]'s module doc): the position just past the most-recently-finished
/// subtree in each input. An `Eval` arm *writes* it on deciding a leaf-vs-leaf branch;
/// a deferred `Right*` frame *reads* it to resume a sibling where its left neighbor
/// ended. There is no payload — the comparison accumulates into `le`/`ge`, not here.
#[derive(Clone, Copy, Default)]
struct Ends {
    /// Position just past the finished subtree in `a`.
    a_end: usize,
    /// Position just past the finished subtree in `b`.
    b_end: usize,
}

/// The causal order, computed in one `O(n + m)` pass; `None` means concurrent.
///
/// Tracks `a <= b` (`le`) and `b <= a` (`ge`) together so the two pointwise
/// comparisons share a single traversal instead of running `leq` twice. The walk
/// descends into whichever side is internal — both in lockstep, or the internal one
/// while the leaf side is broadcast unchanged to both its children — so each node of
/// either tree is visited once. Stops early once both directions are excluded.
pub(crate) fn causal_cmp(a: &EvView, b: &EvView) -> Option<Ordering> {
    // Both storage forms are canonical normal form, so identical contents is exactly
    // semantic equality: settle `Equal` with one memcmp before allocating the walk's
    // job stack. Covers every entry point — Version vs Version, Batch vs Batch, and a
    // not-yet-materialized Batch (still packed) against either. (Mixed packed/working
    // forms decline and fall through; see `EvView::trivially_eq`.)
    if a.trivially_eq(b) {
        return Some(Ordering::Equal);
    }
    let mut le = true; // `a <= b` still possible
    let mut ge = true; // `b <= a` still possible

    // A pending right child reads the threaded side(s) from `ret` (the pinned side
    // carries its own position in the broadcast jobs).
    let mut ret = Ends::default();
    let mut stack = vec![Job::Eval {
        a_pos: 0,
        a_off: Base::ZERO,
        b_pos: 0,
        b_off: Base::ZERO,
    }];
    while let Some(job) = stack.pop() {
        match job {
            Job::Eval {
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
                if a_sum > b_sum {
                    le = false;
                }
                if b_sum > a_sum {
                    ge = false;
                }
                if !le && !ge {
                    return None; // concurrent: neither direction can recover
                }
                match (a_internal, b_internal) {
                    // Both leaves: this branch is decided. Report both ends.
                    (false, false) => {
                        ret = Ends {
                            a_end: a_next,
                            b_end: b_next,
                        }
                    }
                    // Both internal: descend in lockstep.
                    (true, true) => {
                        stack.push(Job::RightLockstep {
                            a_sum: a_sum.clone(),
                            b_sum: b_sum.clone(),
                        });
                        stack.push(Job::Eval {
                            a_pos: a_next,
                            a_off: a_sum,
                            b_pos: b_next,
                            b_off: b_sum,
                        });
                    }
                    // `b` leaf broadcast to both of `a`'s children.
                    (true, false) => {
                        stack.push(Job::RightBroadcastB {
                            a_sum: a_sum.clone(),
                            b_pos,
                            b_off: b_off.clone(),
                        });
                        stack.push(Job::Eval {
                            a_pos: a_next,
                            a_off: a_sum,
                            b_pos,
                            b_off,
                        });
                    }
                    // `a` leaf broadcast to both of `b`'s children.
                    (false, true) => {
                        stack.push(Job::RightBroadcastA {
                            a_pos,
                            a_off: a_off.clone(),
                            b_sum: b_sum.clone(),
                        });
                        stack.push(Job::Eval {
                            a_pos,
                            a_off,
                            b_pos: b_next,
                            b_off: b_sum,
                        });
                    }
                }
            }
            Job::RightLockstep { a_sum, b_sum } => {
                stack.push(Job::Eval {
                    a_pos: ret.a_end,
                    a_off: a_sum,
                    b_pos: ret.b_end,
                    b_off: b_sum,
                });
            }
            Job::RightBroadcastB {
                a_sum,
                b_pos,
                b_off,
            } => {
                stack.push(Job::Eval {
                    a_pos: ret.a_end,
                    a_off: a_sum,
                    b_pos,
                    b_off,
                });
            }
            Job::RightBroadcastA {
                a_pos,
                a_off,
                b_sum,
            } => {
                stack.push(Job::Eval {
                    a_pos,
                    a_off,
                    b_pos: ret.b_end,
                    b_off: b_sum,
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
