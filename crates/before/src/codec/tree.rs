use crate::error::{Decode, Parse};

use super::{BitCursor, BitsView};

/// While building a node bottom-up, what we still need from the stream.
///
/// The parsers keep one frame per unfinished ancestor on an explicit heap `Vec`
/// — as deep as the tree, never the call stack. A terminal parse pushes nothing
/// and allocates nothing; deeper trees pay plain amortized growth, a rounding
/// error against the per-node tag reads.
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
/// Returns the position just past the tree, at the walk's own `u64` width:
/// the byte decode doors walk their whole padded buffer as bits, whose
/// positions can exceed a 32-bit `usize`. Iterative: depth lives on an
/// explicit stack, never the call stack.
///
/// Each node is a 2-bit presence tag (bit 0 = left child follows, bit 1 = right
/// child follows): `00` a terminal, `10`/`01` a unary node, `11` a both-present
/// node. A `0` id is structural absence — a zero presence bit in its parent's
/// tag, no bits of its own — so the grammar has no empty production: input
/// exhausted before a tag completes, the empty input included, is
/// [`Decode::Truncated`], exactly as a byte-starved reader reports it.
pub(crate) fn parse_id(bits: BitsView<'_>, pos: u64) -> Result<u64, Decode> {
    let mut cursor = super::DsiCursor::new_at(bits, pos);
    parse_id_core(&mut cursor)?;
    Ok(cursor.position())
}

/// Parse and validate one id tree from a sequential bit cursor, returning the
/// position just past it.
#[cfg(all(test, feature = "borsh"))]
pub(crate) fn parse_id_from<C: BitCursor>(cursor: &mut C) -> Result<u64, Decode>
where
    Decode: From<C::Error>,
{
    parse_id_core(cursor)?;
    Ok(cursor.position())
}

/// Parse and validate one id tree from a sequential bit cursor: the one
/// grammar body.
///
/// The end position is left to the caller's own [`BitCursor::position`]
/// read.
pub(crate) fn parse_id_core<C: BitCursor>(cursor: &mut C) -> Result<(), Decode>
where
    Decode: From<C::Error>,
{
    let mut stack: Vec<IdFrame> = Vec::new();
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
                None => return Ok(()), // the root is complete
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
/// text grammar legitimately construct (the literal `0`). Whether an anonymous
/// id is *allowed* is the caller's question, answered at the standalone-value
/// gates (`Parse::Anonymous`); the wire grammar never asks it, because no
/// encoder spells the anonymous id on the wire.
pub(crate) fn validate_id(bits: BitsView<'_>) -> Result<(), Parse> {
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
