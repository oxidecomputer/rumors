//! A read-only cursor over the packed id encoding, shared by the party
//! operations (`split`/`sum`/`is_disjoint`/`compare`) and the event operations
//! (`fill`/`grow` walk the packed id alongside the working event tree).
//!
//! `enc_id(Leaf v) = 0, v` (2 bits); `enc_id(Node l r) = 1, enc_id(l), enc_id(r)`.
//!
//! The bit stream is wrapped in [`IdReader`] — a consuming cursor that parallels
//! the event side's [`EvReader`](crate::version::compare) — so the operations
//! read as the paper's recursive `match` over [`IdNode`].
//!
//! **Normal-form precondition.** Every `Party` is in canonical normal form
//! (`decode` rejects anything else; every op produces normal form), and so is
//! every subtree of one. Normalization collapses `(0,0) → 0` and `(1,1) → 1`,
//! so in a normal id an empty region is *exactly* the `0` leaf
//! ([`IdNode::Empty`]) and a full region is *exactly* the `1` leaf
//! ([`IdNode::Full`]), so emptiness/fullness are `O(1)` checks on a decoded node
//! rather than subtree scans. Callers must only pass normal-form id bits.

use crate::codec::BitsSlice;
use crate::step;

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

    /// A reader at an explicit bit offset, for resuming a scan at a recorded
    /// subtree position (see `split`'s `build_split`).
    pub(crate) fn at(bits: &'a BitsSlice, pos: usize) -> Self {
        IdReader { bits, pos }
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
        let bits = self.bits;
        let pos = skip_subtree(self.pos, |at| {
            step!();
            if bits[at] {
                (true, at + 1) // internal: one flag bit
            } else {
                (false, at + 2) // leaf: flag + value bit
            }
        });
        IdReader { bits, pos }
    }

    /// The underlying packed bit stream.
    pub(crate) fn bits(self) -> &'a BitsSlice {
        self.bits
    }

    /// This reader's bit offset, for copying a subtree's verbatim bit range.
    pub(crate) fn pos(self) -> usize {
        self.pos
    }
}

/// Position just past the whole subtree rooted at `at` of any preorder tree
/// encoding, driven by a caller-supplied header probe. Iterative: a
/// pending-children counter (`+1` per internal node, `-1` per leaf, start at
/// one outstanding child), never the call stack — deep inputs cannot overflow.
/// `header(at)` reports `(is_internal, next)`: whether the node at `at` is
/// internal (so its children follow) and the position just past its header. The
/// single shared spelling of this scan: [`IdReader::skip`] runs it on the packed
/// id encoding, `EvReader::skip` (in `version::compare`) on the event encoding,
/// and `version::event::Builder::copy_reader` inlines the same loop while also
/// emitting the visited nodes.
pub(crate) fn skip_subtree(mut at: usize, mut header: impl FnMut(usize) -> (bool, usize)) -> usize {
    let mut pending: i64 = 1;
    while pending > 0 {
        let (internal, next) = header(at);
        at = next;
        pending += if internal { 1 } else { -1 };
    }
    at
}
