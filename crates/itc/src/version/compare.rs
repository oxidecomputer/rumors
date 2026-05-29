//! The causal order on event trees: an iterative, offset-threaded `leq` that reads
//! either representation in place (no transcode), plus the derived comparison.
//!
//! `leq(a, b)` decides whether `a`'s event function is `<=` `b`'s pointwise. It is
//! the recursive `leq` of the paper (plan Appendix A) made iterative: `self` always
//! descends (so the walk terminates), while `other` either descends in lockstep
//! (both internal) or is broadcast unchanged to both of `self`'s children (when
//! `other` is a leaf). Path sums are threaded as offsets; `u128` offsets cannot
//! overflow for any in-memory tree.

use core::cmp::Ordering;

use crate::codec::{decode_int, BitsSlice};

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
    fn header(&self, at: usize) -> (bool, u64, usize) {
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
fn skip(view: &EvView, mut at: usize) -> usize {
    let mut pending: i64 = 1;
    while pending > 0 {
        let (internal, _, next) = view.header(at);
        at = next;
        pending += if internal { 1 } else { -1 };
    }
    at
}

/// Whether `a`'s event function is pointwise `<=` `b`'s. Iterative; both views are
/// walked from their roots (position `0`).
fn leq(a: &EvView, b: &EvView) -> bool {
    // (a-position, b-position, a-offset, b-offset)
    let mut stack: Vec<(usize, usize, u128, u128)> = vec![(0, 0, 0, 0)];
    while let Some((ap, bp, ao, bo)) = stack.pop() {
        let (a_internal, a_base, a_next) = a.header(ap);
        let (b_internal, b_base, b_next) = b.header(bp);
        let an = ao + a_base as u128;
        let bn = bo + b_base as u128;
        if an > bn {
            return false;
        }
        if !a_internal {
            continue; // a leaf with an <= bn is dominated everywhere below it
        }
        let a_left = a_next;
        let a_right = skip(a, a_left);
        if b_internal {
            let b_left = b_next;
            let b_right = skip(b, b_left);
            stack.push((a_left, b_left, an, bn));
            stack.push((a_right, b_right, an, bn));
        } else {
            // b is a leaf: broadcast it (unchanged) to both of a's children.
            stack.push((a_left, bp, an, bo));
            stack.push((a_right, bp, an, bo));
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
