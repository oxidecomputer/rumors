//! A read-only cursor over the packed id encoding, shared by the party
//! operations (`split`/`sum`/`is_disjoint`/`compare`) and the event operations
//! (`fill`/`grow` walk the packed id alongside the working event tree).
//!
//! Each node is a 2-bit tag whose bits report **child presence**: bit 0 is "a
//! left child follows", bit 1 is "a right child follows". So `00` is a terminal
//! (the owned `1` leaf, no children), `10` a left-only node `(i, 0)`, `01` a
//! right-only node `(0, i)`, and `11` a node with both children `(i1, i2)`.
//! Present children follow the tag in preorder; an absent child occupies no
//! bits. The empty `0` id is the empty bit stream.
//!
//! This is the pruned form of the id grammar `i ::= 0 | 1 | (i1, i2)`: a `0` is
//! never a node, only the *absence* of a child, so `(0, 0)` is unrepresentable.
//! The one canonicity rule left to enforce is "no node with two terminal
//! children" (that is `(1, 1)`); see [`crate::codec`]'s `parse_id`.
//!
//! The bit stream is wrapped in [`IdReader`] — a consuming cursor that parallels
//! the event side's [`EvReader`](crate::version::compare) — so the operations
//! read as the paper's recursive `match` over [`IdNode`].
//!
//! **Normal-form precondition.** Every `Party` is in canonical normal form
//! (`decode` rejects anything else; every op produces normal form), and so is
//! every subtree of one. Because `0` is structural absence and `(1, 1)` cannot
//! appear, an empty region is *exactly* an absent child ([`IdNode::Empty`], only
//! ever yielded by a synthetic reader) and a full region is *exactly* a terminal
//! ([`IdNode::Full`]), so emptiness/fullness are `O(1)` checks on a decoded node
//! rather than subtree scans. Callers must only pass normal-form id bits.

use crate::codec::BitsSlice;
use crate::step;

/// A decoded id node: the empty `0` leaf, the full `1` leaf, or an internal
/// node tagged with which of its children are present.
///
/// The id-side analogue of the event side's `EvNode` — the clean shape the
/// operations recurse on (the paper's id grammar `i ::= 0 | 1 | (i1, i2)`).
///
/// `Empty` is never decoded from the stream — a `0` occupies no bits — so it
/// arises only from the synthetic [`IdReader::Empty`], handed in for an absent
/// child. `Internal` always has at least one present child (a node with neither
/// would be `(0, 0)`, which collapses to `0`).
#[derive(Clone, Copy)]
pub(crate) enum IdNode {
    /// The `0` leaf: an unowned region. Only ever from a synthetic reader.
    Empty,
    /// The `1` leaf: a fully-owned region (a terminal, tag `00`).
    Full,
    /// An internal node; its present children follow in the stream, left then
    /// right. At least one of `left`/`right` is set.
    Internal { left: bool, right: bool },
}

/// A cursor into a packed id tree, or a synthetic leaf.
///
/// The id-side analogue of the event side's
/// [`EvReader`](crate::version::compare): it *consumes* —
/// [`read`](IdReader::read) decodes the node at the cursor and advances it in
/// place — so operations thread `&mut` readers and read as the paper's
/// recursive `match`.
///
/// - `At`: a bit offset into the packed id stream.
/// - `Full`: a synthetic terminal that consumes nothing — the id-side analogue
///   of [`EvReader::Zero`](crate::version::compare). `grow` hands it to both
///   event children when the id is full (`FullEvNode`), re-presenting the full
///   `1` without duplicating the real cursor.
/// - `Empty`: a synthetic `0` leaf that consumes nothing. Because a `0` is
///   absence rather than a node, every walk that descends into an absent child
///   hands one of these in its place, so the `(Empty, …)` match arms fire
///   exactly as they did when `0` was a real leaf in the stream.
///
/// Not `Copy` or `Clone`: a cursor is single-use. Advancing it consumes the
/// stream, so a stale or duplicated cursor (a re-scan, which would break the
/// `O(n + m)` bound) cannot be formed by accident. The only operation that
/// reads a tree twice is `grow` (on the event side), which rebuilds a fresh
/// cursor from the source per pass.
pub(crate) enum IdReader<'a> {
    At {
        bits: &'a BitsSlice,
        pos: usize,
    },
    /// A synthetic full `1` leaf (see the type doc); reads as [`IdNode::Full`]
    /// and never advances.
    Full,
    /// A synthetic empty `0` leaf (see the type doc); reads as [`IdNode::Empty`]
    /// and never advances.
    Empty,
}

impl<'a> IdReader<'a> {
    /// A reader at the root of `bits`. Empty bits are the anonymous `0` id, so
    /// they read as the synthetic [`Empty`](IdReader::Empty) leaf.
    pub(crate) fn root(bits: &'a BitsSlice) -> Self {
        if bits.is_empty() {
            IdReader::Empty
        } else {
            IdReader::At { bits, pos: 0 }
        }
    }

    /// A reader at an explicit bit offset, for resuming a scan at a recorded
    /// subtree position (see `split`'s `build_split`).
    pub(crate) fn at(bits: &'a BitsSlice, pos: usize) -> Self {
        IdReader::At { bits, pos }
    }

    /// Decode the 2-bit tag at `pos`: `(left_present, right_present)`. Neither
    /// present is the terminal (`Full`); otherwise an internal node.
    #[inline]
    fn tag(bits: &BitsSlice, pos: usize) -> IdNode {
        let left = bits[pos];
        let right = bits[pos + 1];
        if !left && !right {
            IdNode::Full
        } else {
            IdNode::Internal { left, right }
        }
    }

    /// Decode the node at this cursor, advancing it just past the 2-bit tag — to
    /// the first present child, for an internal node. `Full`/`Empty` read as
    /// their synthetic nodes and never advance.
    pub(crate) fn read(&mut self) -> IdNode {
        match self {
            IdReader::Full => IdNode::Full,
            IdReader::Empty => IdNode::Empty,
            IdReader::At { bits, pos } => {
                step!();
                let node = Self::tag(bits, *pos);
                *pos += 2;
                node
            }
        }
    }

    /// Decode the node at this cursor *without* advancing — a look at the
    /// current node.
    ///
    /// `fill` uses it to test whether a child is fully owned before deciding to
    /// collapse it (a shortcut) or recurse into it. (Not a duplication: it reads
    /// the node in place, leaving the single cursor where it was.)
    pub(crate) fn peek(&self) -> IdNode {
        match self {
            IdReader::Full => IdNode::Full,
            IdReader::Empty => IdNode::Empty,
            IdReader::At { bits, pos } => {
                step!();
                Self::tag(bits, *pos)
            }
        }
    }

    /// Advance this cursor just past the whole subtree at it (the shared
    /// iterative [`skip_subtree`] scan), for skipping a dominated id subtree.
    /// `Full`/`Empty` are leaves: nothing to skip.
    pub(crate) fn skip(&mut self) {
        if let IdReader::At { bits, pos } = self {
            let bits = *bits;
            *pos = skip_subtree(*pos, |at| {
                step!();
                // Children present = the two tag bits; the tag is 2 bits wide.
                let children = usize::from(bits[at]) + usize::from(bits[at + 1]);
                (children, at + 2)
            });
        }
    }

    /// The underlying packed bit stream, or the empty slice for a synthetic
    /// reader (which addresses no bits).
    ///
    /// Used for `sum`/`diff` capacity hints, where an anonymous (`0`) operand is
    /// a synthetic [`Empty`](IdReader::Empty) contributing zero bits.
    pub(crate) fn bits(&self) -> &'a BitsSlice {
        match self {
            IdReader::At { bits, .. } => bits,
            IdReader::Full | IdReader::Empty => BitsSlice::empty(),
        }
    }

    /// This reader's bit offset, for copying a subtree's verbatim bit range or
    /// recording a branch position. Not called on a synthetic reader.
    pub(crate) fn pos(&self) -> usize {
        match self {
            IdReader::At { pos, .. } => *pos,
            IdReader::Full | IdReader::Empty => {
                unreachable!("pos() on a synthetic id reader")
            }
        }
    }

    /// This reader's bit offset if it addresses a real tree, or `None` for a
    /// synthetic reader.
    ///
    /// `grow` captures it (before a read advances the cursor) to key its
    /// position-indexed `Route`; the synthetic side of a branch is never the
    /// keying side, so its `None` is never unwrapped.
    pub(crate) fn pos_opt(&self) -> Option<usize> {
        match self {
            IdReader::At { pos, .. } => Some(*pos),
            IdReader::Full | IdReader::Empty => None,
        }
    }
}

/// Position just past the whole subtree rooted at `at` of any preorder tree
/// encoding, driven by a caller-supplied header probe.
///
/// Iterative: a pending-children counter, never the call stack — deep inputs
/// cannot overflow. `header(at)` reports `(child_count, next)`: how many children
/// the node at `at` has (so they follow) and the position just past its header.
/// The counter starts with one subtree outstanding and, per node, spends one
/// (the node itself) and adds its children; the subtree ends when nothing is
/// outstanding. This admits unary nodes (one child, net zero), which the id
/// encoding uses; a full binary encoding only ever reports `0` or `2`.
///
/// The single shared spelling of this scan: [`IdReader::skip`] runs it on the
/// packed id encoding, `EvReader::skip` (in `version::compare`) on the event
/// encoding, and `version::event::Builder::copy_reader` inlines the same loop
/// while also emitting the visited nodes.
pub(crate) fn skip_subtree(
    mut at: usize,
    mut header: impl FnMut(usize) -> (usize, usize),
) -> usize {
    let mut pending: i64 = 1;
    while pending > 0 {
        let (children, next) = header(at);
        at = next;
        pending += children as i64 - 1;
    }
    at
}
