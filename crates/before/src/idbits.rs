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

/// A cursor into a packed id tree, or a synthetic full leaf. The id-side
/// analogue of the event side's [`EvReader`](crate::version::compare): it
/// *consumes* — [`read`](IdReader::read) decodes the node at the cursor and
/// advances it in place — so operations thread `&mut` readers and read as the
/// paper's recursive `match`.
///
/// - `At`: a bit offset into the packed `enc_id` stream.
/// - `Full`: a synthetic `1` leaf that consumes nothing — the id-side analogue
///   of [`EvReader::Zero`](crate::version::compare). `grow` hands it to both
///   event children when the id is full (`FullEvNode`), re-presenting the full
///   `1` without duplicating the real cursor.
///
/// **Not `Copy`/`Clone`.** A cursor is single-use: advancing it consumes the
/// stream, so a stale or duplicated cursor — the re-scan footgun that breaks
/// `O(n + m)` — cannot be formed by accident. The only operation that reads a
/// tree twice is `grow` (on the event side); it rebuilds a fresh cursor from the
/// source per pass, so no cursor is ever duplicated.
pub(crate) enum IdReader<'a> {
    At {
        bits: &'a BitsSlice,
        pos: usize,
    },
    /// A synthetic full `1` leaf (see the type doc); reads as [`IdNode::Full`]
    /// and never advances.
    Full,
}

impl<'a> IdReader<'a> {
    /// A reader at the root of `bits`.
    pub(crate) fn root(bits: &'a BitsSlice) -> Self {
        IdReader::At { bits, pos: 0 }
    }

    /// A reader at an explicit bit offset, for resuming a scan at a recorded
    /// subtree position (see `split`'s `build_split`).
    pub(crate) fn at(bits: &'a BitsSlice, pos: usize) -> Self {
        IdReader::At { bits, pos }
    }

    /// Decode the node at this cursor, advancing it just past the header — to
    /// the left child, for an internal node. `Full` reads as [`IdNode::Full`]
    /// and never advances.
    pub(crate) fn read(&mut self) -> IdNode {
        match self {
            IdReader::Full => IdNode::Full,
            IdReader::At { bits, pos } => {
                step!();
                let (node, next) = if bits[*pos] {
                    (IdNode::Internal, *pos + 1)
                } else if bits[*pos + 1] {
                    (IdNode::Full, *pos + 2)
                } else {
                    (IdNode::Empty, *pos + 2)
                };
                *pos = next;
                node
            }
        }
    }

    /// Decode the node at this cursor *without* advancing — a look at the
    /// current node. `fill` uses it to test whether a child is fully owned
    /// before deciding to collapse it (a shortcut) or recurse into it. (Not a
    /// duplication: it reads the node in place, leaving the single cursor where
    /// it was.)
    pub(crate) fn peek(&self) -> IdNode {
        match self {
            IdReader::Full => IdNode::Full,
            IdReader::At { bits, pos } => {
                step!();
                if bits[*pos] {
                    IdNode::Internal
                } else if bits[*pos + 1] {
                    IdNode::Full
                } else {
                    IdNode::Empty
                }
            }
        }
    }

    /// Advance this cursor just past the whole subtree at it (the shared
    /// iterative [`skip_subtree`] scan), for skipping a dominated id subtree.
    /// `Full` is a leaf: nothing to skip.
    pub(crate) fn skip(&mut self) {
        if let IdReader::At { bits, pos } = self {
            let bits = *bits;
            *pos = skip_subtree(*pos, |at| {
                step!();
                if bits[at] {
                    (true, at + 1) // internal: one flag bit
                } else {
                    (false, at + 2) // leaf: flag + value bit
                }
            });
        }
    }

    /// The underlying packed bit stream. Not called on the synthetic `Full`.
    pub(crate) fn bits(&self) -> &'a BitsSlice {
        match self {
            IdReader::At { bits, .. } => bits,
            IdReader::Full => unreachable!("bits() on the synthetic Full id reader"),
        }
    }

    /// This reader's bit offset, for copying a subtree's verbatim bit range or
    /// recording a branch position. Not called on the synthetic `Full`.
    pub(crate) fn pos(&self) -> usize {
        match self {
            IdReader::At { pos, .. } => *pos,
            IdReader::Full => unreachable!("pos() on the synthetic Full id reader"),
        }
    }

    /// This reader's bit offset if it addresses a real tree, or `None` for the
    /// synthetic `Full`. `grow` captures it (before a read advances the cursor)
    /// to key its position-indexed `Route`; the synthetic side of a branch is
    /// never the keying side, so its `None` is never unwrapped.
    pub(crate) fn pos_opt(&self) -> Option<usize> {
        match self {
            IdReader::At { pos, .. } => Some(*pos),
            IdReader::Full => None,
        }
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
