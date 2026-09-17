use crate::error::Decode;

use super::{BitCursor, BitsView};

/// The unvisited part of an unfinished party node.
enum IdFrame {
    /// A both-present node: the next subtree is its left child.
    BothNeedLeft,
    /// A both-present node whose left child is complete.
    BothNeedRight { left_terminal: bool },
    /// A unary node (left- or right-only): the next subtree is its one child.
    UnaryNeedChild,
}

/// Parse one party tree at `pos` and return the first bit after it.
///
/// Each node begins with two child-presence bits. `00` is a terminal; `10` and
/// `01` have one child; `11` has two. Two terminal children are noncanonical
/// because they collapse to one terminal. An explicit stack makes the parser
/// safe for arbitrarily deep inputs.
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

/// Parse and validate one party tree from a sequential bit cursor.
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
