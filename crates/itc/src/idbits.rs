//! A read-only cursor over the packed id encoding, shared by the party operations
//! (`split`/`sum`/`is_disjoint`/`contains`) and the event operations (`fill`/`grow`
//! walk the packed id alongside the working event tree).
//!
//! `enc_id(Leaf v) = 0, v` (2 bits); `enc_id(Node l r) = 1, enc_id(l), enc_id(r)`.
//!
//! **Normal-form precondition.** Every `Party` is in canonical normal form (`decode`
//! rejects anything else; every op produces normal form), and so is every subtree of
//! one. Normalization collapses `(0,0) → 0` and `(1,1) → 1`, so in a normal id an
//! empty region is *exactly* the `0` leaf and a full region is *exactly* the `1`
//! leaf, so emptiness/fullness are `O(1)` leaf checks rather than subtree scans:
//! [`is_full`], and an inline `(false, false)` header test for emptiness. Callers
//! must only pass normal-form id bits.

use crate::codec::BitsSlice;
use crate::step;

/// `(is_node, leaf_value, position-just-past-this-node's-header)`. For a node the
/// header is the single flag bit and the left child begins at the returned position;
/// for a leaf the header is the flag plus its value bit.
pub(crate) fn header(bits: &BitsSlice, at: usize) -> (bool, bool, usize) {
    step!();
    if bits[at] {
        (true, false, at + 1)
    } else {
        (false, bits[at + 1], at + 2)
    }
}

/// Position just past the whole subtree rooted at `at`. Iterative: a pending-children
/// counter, never the call stack. (The event-tree analogue, on the `EvView` header
/// shape, is `version::compare::skip`: same algorithm, different node encoding — keep
/// the two in step.)
pub(crate) fn skip(bits: &BitsSlice, mut at: usize) -> usize {
    let mut pending: i64 = 1;
    while pending > 0 {
        let (is_node, _, next) = header(bits, at);
        at = next;
        pending += if is_node { 1 } else { -1 };
    }
    at
}

/// Whether the normal-form subtree at `at` owns everything. `O(1)`: it is full iff it
/// is the `1` leaf (see the module's normal-form precondition).
pub(crate) fn is_full(bits: &BitsSlice, at: usize) -> bool {
    matches!(header(bits, at), (false, true, _))
}
