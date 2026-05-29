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
//! leaf. [`is_empty`]/[`is_full`] rely on this to answer in `O(1)` — a leaf check —
//! rather than scanning a subtree. Callers must only pass normal-form id bits.

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
/// counter, never the call stack.
pub(crate) fn skip(bits: &BitsSlice, mut at: usize) -> usize {
    let mut pending: i64 = 1;
    while pending > 0 {
        let (is_node, _, next) = header(bits, at);
        at = next;
        pending += if is_node { 1 } else { -1 };
    }
    at
}

/// Whether the normal-form subtree at `at` owns nothing. `O(1)`: it is empty iff it is
/// the `0` leaf (see the module's normal-form precondition).
// Consumed by the event operations (`fill`/`grow`, Phase 5), which walk the packed id.
#[allow(dead_code)]
pub(crate) fn is_empty(bits: &BitsSlice, at: usize) -> bool {
    matches!(header(bits, at), (false, false, _))
}

/// Whether the normal-form subtree at `at` owns everything. `O(1)`: it is full iff it
/// is the `1` leaf (see the module's normal-form precondition).
#[allow(dead_code)]
pub(crate) fn is_full(bits: &BitsSlice, at: usize) -> bool {
    matches!(header(bits, at), (false, true, _))
}
