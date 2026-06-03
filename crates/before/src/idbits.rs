//! A read-only cursor over the packed id encoding, shared by the party
//! operations (`split`/`sum`/`is_disjoint`/`compare`) and the event operations
//! (`fill`/`grow` walk the packed id alongside the working event tree).
//!
//! `enc_id(Leaf v) = 0, v` (2 bits); `enc_id(Node l r) = 1, enc_id(l), enc_id(r)`.
//!
//! The bit stream is wrapped in [`IdView`] — a lightweight read-only newtype
//! that parallels the event side's `EvView` — so the cursor operations read as
//! methods on the id (`id.header(at)`, `id.skip(at)`).
//!
//! **Normal-form precondition.** Every `Party` is in canonical normal form
//! (`decode` rejects anything else; every op produces normal form), and so is
//! every subtree of one. Normalization collapses `(0,0) → 0` and `(1,1) → 1`,
//! so in a normal id an empty region is *exactly* the `0` leaf and a full
//! region is *exactly* the `1` leaf, so emptiness/fullness are `O(1)` checks on
//! an already-decoded [`IdHeader`] rather than subtree scans. Callers must only
//! pass normal-form id bits.

use crate::codec::BitsSlice;
use crate::step;

/// A read-only view of a packed id bit stream, addressed by a bit offset. The
/// id-side analogue of the event side's `EvView`: cursor operations hang off it
/// as methods. A thin `Copy` wrapper over a borrowed slice, so passing one by
/// value is free.
#[derive(Clone, Copy)]
pub(crate) struct IdView<'a>(pub(crate) &'a BitsSlice);

/// The decoded id-node header at a position. For a node the header is the
/// single flag bit (`node`) and the left child begins at `next`; for a leaf the
/// header is the flag plus its value bit (`val`).
pub(crate) struct IdHeader {
    /// Whether this node is internal (has two children) rather than a leaf.
    pub(crate) node: bool,
    /// A leaf's value bit (`true` = owned, `false` = not owned); meaningless for a node.
    pub(crate) val: bool,
    /// Position just past this node's header (where its left child, if any, begins).
    pub(crate) next: usize,
}

impl IdHeader {
    /// Whether this already-decoded normal-form header is the empty `0` leaf.
    pub(crate) fn is_empty(&self) -> bool {
        !self.node && !self.val
    }

    /// Whether this already-decoded normal-form header is the full `1` leaf.
    pub(crate) fn is_full(&self) -> bool {
        !self.node && self.val
    }
}

impl<'a> IdView<'a> {
    /// Decode the id-node header at `at`. For a node the header is the single
    /// flag bit and the left child begins at [`IdHeader::next`]; for a leaf the
    /// header is the flag plus its value bit.
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

    /// Position just past the whole subtree rooted at `at`. Iterative: a
    /// pending-children counter, never the call stack — see the shared
    /// [`skip_subtree`] core. (The event-tree analogue, on the `EvView` header
    /// shape, is `EvView::skip` in `version::compare`: same algorithm via the
    /// same core, different node encoding.)
    pub(crate) fn skip(&self, at: usize) -> usize {
        skip_subtree(at, |pos| {
            let h = self.header(pos);
            (h.node, h.next)
        })
    }

    /// The underlying packed bit stream.
    pub(crate) fn bits(&self) -> &'a BitsSlice {
        self.0
    }
}

/// A decoded id node: the empty `0` leaf, the full `1` leaf, or an internal
/// node. The id-side analogue of the event side's `EvNode` — a clean three-way
/// for the `match (id, ev)` shape the operations recurse on (the paper's id
/// grammar `i ::= 0 | 1 | (i1, i2)`).
pub(crate) enum IdNode {
    /// The `0` leaf: an unowned region.
    Empty,
    /// The `1` leaf: a fully-owned region.
    Full,
    /// An internal node `(i1, i2)`; its children follow in the stream.
    Internal,
}

/// A cursor into a packed id tree: a position in the bit stream. The id-side
/// analogue of the event side's [`EvReader`](crate::version::compare). Where
/// [`IdView`] exposes a positional `header(at)`, `IdReader` *consumes* — its
/// [`read`](IdReader::read) advances past the node it decodes — so operations
/// thread readers instead of bare bit offsets and read as the paper's recursive
/// `match`. A thin `Copy` handle over a borrowed slice.
#[derive(Clone, Copy)]
pub(crate) struct IdReader<'a> {
    bits: &'a BitsSlice,
    pos: usize,
}

impl<'a> IdReader<'a> {
    /// A reader at the root of `bits`.
    pub(crate) fn root(bits: &'a BitsSlice) -> Self {
        IdReader { bits, pos: 0 }
    }

    /// Decode this node, returning it together with a reader positioned just
    /// past the header — at the left child, for an internal node.
    pub(crate) fn read(self) -> (IdNode, IdReader<'a>) {
        step!();
        let (node, next) = if self.bits[self.pos] {
            (IdNode::Internal, self.pos + 1)
        } else if self.bits[self.pos + 1] {
            (IdNode::Full, self.pos + 2)
        } else {
            (IdNode::Empty, self.pos + 2)
        };
        (
            node,
            IdReader {
                bits: self.bits,
                pos: next,
            },
        )
    }

    /// A reader positioned just past this whole subtree (the shared iterative
    /// [`skip_subtree`] scan), for skipping a dominated id subtree.
    pub(crate) fn skip(self) -> IdReader<'a> {
        IdReader {
            bits: self.bits,
            pos: IdView(self.bits).skip(self.pos),
        }
    }
}

/// Position just past the whole subtree rooted at `at` of any preorder tree
/// encoding, driven by a caller-supplied header probe. Iterative: a
/// pending-children counter (`+1` per internal node, `-1` per leaf, start at
/// one outstanding child), never the call stack — deep inputs cannot overflow.
/// `header(at)` reports `(is_internal, next)`: whether the node at `at` is
/// internal (so its children follow) and the position just past its header. The
/// single shared spelling of this scan: [`IdView::skip`] runs it on the packed
/// id header shape, `EvView::skip` (in `version::compare`) on the `EvView`
/// event header shape, and `version::event::Builder::copy` inlines the same
/// loop while also emitting the visited nodes.
pub(crate) fn skip_subtree(mut at: usize, mut header: impl FnMut(usize) -> (bool, usize)) -> usize {
    let mut pending: i64 = 1;
    while pending > 0 {
        let (internal, next) = header(at);
        at = next;
        pending += if internal { 1 } else { -1 };
    }
    at
}
