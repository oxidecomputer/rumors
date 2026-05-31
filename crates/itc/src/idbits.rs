//! A read-only cursor over the packed id encoding, shared by the party operations
//! (`split`/`sum`/`is_disjoint`/`contains`) and the event operations (`fill`/`grow`
//! walk the packed id alongside the working event tree).
//!
//! `enc_id(Leaf v) = 0, v` (2 bits); `enc_id(Node l r) = 1, enc_id(l), enc_id(r)`.
//!
//! The bit stream is wrapped in [`IdView`] — a lightweight read-only newtype that
//! parallels the event side's `EvView` — so the cursor operations read as methods on the
//! id (`id.header(at)`, `id.skip(at)`, `id.is_empty(at)`, `id.is_full(at)`).
//!
//! **Normal-form precondition.** Every `Party` is in canonical normal form (`decode`
//! rejects anything else; every op produces normal form), and so is every subtree of
//! one. Normalization collapses `(0,0) → 0` and `(1,1) → 1`, so in a normal id an
//! empty region is *exactly* the `0` leaf and a full region is *exactly* the `1`
//! leaf, so emptiness/fullness are `O(1)` leaf checks rather than subtree scans:
//! [`IdView::is_empty`] and [`IdView::is_full`]. Callers must only pass normal-form id
//! bits.

use crate::codec::BitsSlice;
use crate::step;

/// A read-only view of a packed id bit stream, addressed by a bit offset. The id-side
/// analogue of the event side's `EvView`: the cursor operations
/// (`header`/`skip`/`is_empty`/`is_full`) hang off it as methods. A thin `Copy` wrapper
/// over a borrowed slice, so passing one by value is free.
#[derive(Clone, Copy)]
pub(crate) struct IdView<'a>(pub(crate) &'a BitsSlice);

/// The decoded id-node header at a position. For a node the header is the single flag
/// bit (`node`) and the left child begins at `next`; for a leaf the header is the flag
/// plus its value bit (`val`).
pub(crate) struct IdHeader {
    /// Whether this node is internal (has two children) rather than a leaf.
    pub(crate) node: bool,
    /// A leaf's value bit (`true` = owned, `false` = not owned); meaningless for a node.
    pub(crate) val: bool,
    /// Position just past this node's header (where its left child, if any, begins).
    pub(crate) next: usize,
}

impl<'a> IdView<'a> {
    /// Decode the id-node header at `at`. For a node the header is the single flag bit
    /// and the left child begins at [`IdHeader::next`]; for a leaf the header is the
    /// flag plus its value bit.
    pub(crate) fn header(&self, at: usize) -> IdHeader {
        let bits = self.0;
        step!();
        if bits[at] {
            IdHeader {
                node: true,
                val: false,
                next: at + 1,
            }
        } else {
            IdHeader {
                node: false,
                val: bits[at + 1],
                next: at + 2,
            }
        }
    }

    /// Position just past the whole subtree rooted at `at`. Iterative: a pending-children
    /// counter, never the call stack — see the shared [`skip_subtree`] core. (The
    /// event-tree analogue, on the `EvView` header shape, is `EvView::skip` in
    /// `version::compare`: same algorithm via the same core, different node encoding.)
    pub(crate) fn skip(&self, at: usize) -> usize {
        skip_subtree(at, |pos| {
            let h = self.header(pos);
            (h.node, h.next)
        })
    }

    /// Whether the normal-form subtree at `at` owns nothing. `O(1)`: it is empty iff it
    /// is the `0` leaf (see the module's normal-form precondition).
    pub(crate) fn is_empty(&self, at: usize) -> bool {
        let h = self.header(at);
        !h.node && !h.val
    }

    /// Whether the normal-form subtree at `at` owns everything. `O(1)`: it is full iff it
    /// is the `1` leaf (see the module's normal-form precondition).
    pub(crate) fn is_full(&self, at: usize) -> bool {
        let h = self.header(at);
        !h.node && h.val
    }

    /// The underlying packed bit stream.
    pub(crate) fn bits(&self) -> &'a BitsSlice {
        self.0
    }
}

/// Position just past the whole subtree rooted at `at` of any preorder tree encoding,
/// driven by a caller-supplied header probe. Iterative: a pending-children counter
/// (`+1` per internal node, `-1` per leaf, start at one outstanding child), never the
/// call stack — deep inputs cannot overflow. `header(at)` reports `(is_internal, next)`:
/// whether the node at `at` is internal (so its children follow) and the position just
/// past its header. The single shared spelling of this scan: [`IdView::skip`] runs it on
/// the packed id header shape, `EvView::skip` (in `version::compare`) on the `EvView`
/// event header shape, and `version::event::Builder::copy` inlines the same loop while
/// also emitting the visited nodes.
pub(crate) fn skip_subtree(mut at: usize, mut header: impl FnMut(usize) -> (bool, usize)) -> usize {
    let mut pending: i64 = 1;
    while pending > 0 {
        let (internal, next) = header(at);
        at = next;
        pending += if internal { 1 } else { -1 };
    }
    at
}
