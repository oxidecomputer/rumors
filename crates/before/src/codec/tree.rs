use smallvec::SmallVec;

use crate::error::{Decode, Parse};

use super::{BitCursor, BitsSlice, SliceCursor};

/// Inline capacity, in frames, of the parsers' explicit stacks.
///
/// One frame per level of unfinished ancestors, so the stack is as deep as
/// the tree; real id and event trees stay well under 16 levels, so a parse
/// normally touches no heap at all — worth having because the wire path runs
/// one parse per decoded `Version` (~10k per gossip session), so a fresh
/// `Vec` per call would put an allocation on every decoded version rather
/// than none. Deeper trees spill to the heap transparently; depth still
/// never lands on the call stack.
pub(crate) const PARSE_STACK_INLINE: usize = 16;

/// While building a node bottom-up, what we still need from the stream.
enum IdFrame {
    /// A both-present node: the next subtree is its left child.
    BothNeedLeft,
    /// A both-present node whose left child is parsed (a terminal? — needed for
    /// the `(1, 1)` check); the next subtree is its right child.
    BothNeedRight { left_terminal: bool },
    /// A unary node (left- or right-only): the next subtree is its one child.
    UnaryNeedChild,
}

/// Parse one packed id tree at `pos`, validating id normal form (no node with
/// two terminal children, that is `(1, 1)`).
///
/// Returns the position just past the tree. Iterative: depth lives on an
/// explicit stack, never the call stack.
///
/// Each node is a 2-bit presence tag (bit 0 = left child follows, bit 1 = right
/// child follows): `00` a terminal, `10`/`01` a unary node, `11` a both-present
/// node. A `0` id is structural absence — a zero presence bit in its parent's
/// tag, no bits of its own — so the grammar has no empty production: input
/// exhausted before a tag completes, the empty input included, is
/// [`Decode::Truncated`], exactly as a byte-starved reader reports it.
pub(crate) fn parse_id(bits: &BitsSlice, pos: usize) -> Result<usize, Decode> {
    let mut cursor = SliceCursor::new(bits, pos);
    parse_id_from(&mut cursor)
}

/// Parse and validate one id tree from a sequential bit cursor.
pub(crate) fn parse_id_from<C: BitCursor>(cursor: &mut C) -> Result<usize, Decode>
where
    Decode: From<C::Error>,
{
    let mut stack: SmallVec<[IdFrame; PARSE_STACK_INLINE]> = SmallVec::new();
    loop {
        let left = cursor.read_bit()?;
        let right = cursor.read_bit()?;

        // `summary` is whether the just-completed subtree is a terminal — the
        // only fact a parent needs, to reject `(1, 1)`.
        let mut summary = match (left, right) {
            (true, true) => {
                stack.push(IdFrame::BothNeedLeft);
                continue; // descend into the left child
            }
            (true, false) | (false, true) => {
                stack.push(IdFrame::UnaryNeedChild);
                continue; // descend into the one present child
            }
            (false, false) => true, // a terminal
        };

        // Attach the completed subtree to its parent, possibly completing it too.
        loop {
            match stack.pop() {
                None => return Ok(cursor.position()), // the root is complete
                Some(IdFrame::BothNeedLeft) => {
                    stack.push(IdFrame::BothNeedRight {
                        left_terminal: summary,
                    });
                    break; // go parse the right child
                }
                Some(IdFrame::BothNeedRight { left_terminal }) => {
                    if left_terminal && summary {
                        return Err(Decode::NotCanonical); // collapsible (1, 1)
                    }
                    summary = false; // this node is internal to its own parent
                }
                Some(IdFrame::UnaryNeedChild) => {
                    summary = false; // a unary node is internal, never a terminal
                }
            }
        }
    }
}

/// Confirm a freshly built id bit stream is exactly one canonical-normal-form
/// tree. Wraps [`parse_id`] (the single source of truth for id normal form),
/// mapping its outcome onto [`Parse`].
///
/// The empty stream is accepted here, unlike at [`parse_id`]: it is the
/// in-memory normal form of the anonymous `0` id, which the builders and the
/// text grammar legitimately construct (the literal `0`). Whether an
/// anonymous id is *allowed* is the caller's question, answered at the
/// standalone-value gates (`Parse::Anonymous`); the wire grammar never asks
/// it, because no encoder spells the anonymous id on the wire.
pub(crate) fn validate_id(bits: &BitsSlice) -> Result<(), Parse> {
    if bits.is_empty() {
        return Ok(());
    }
    match parse_id(bits, 0) {
        Ok(end) if end == bits.len() => Ok(()),
        Ok(_) => Err(Parse::Syntax),
        Err(Decode::NotCanonical) => Err(Parse::NotCanonical),
        Err(_) => Err(Parse::Syntax),
    }
}
