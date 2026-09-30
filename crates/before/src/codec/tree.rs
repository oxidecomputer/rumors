//! Validates party trees and locates the end of preorder subtrees.
//!
//! Validation tracks whether completed children are terminals, rejecting a
//! branch whose two owned terminals should have collapsed into one. Two bits
//! per unfinished branch suffice for that check. Subtree skipping needs only a
//! count of nodes still owed and leaves validation to the caller.

use crate::error::Decode;

mod frames;

use super::{BitCursor, BitsView};
use frames::{Frame, Frames};

/// Parse one party tree at `pos` and return the first bit after it.
///
/// Each node begins with two child-presence bits. `00` is a terminal; `10` and
/// `01` have one child; `11` has two. Two terminal children are noncanonical
/// because they collapse to one terminal. An explicit stack makes the parser
/// safe for arbitrarily deep inputs.
pub(crate) fn parse_party(bits: BitsView<'_>, pos: u64) -> Result<u64, Decode> {
    let mut cursor = super::DsiCursor::new_at(bits, pos);
    parse_party_core(&mut cursor)?;
    Ok(cursor.position())
}

/// Parse and validate one party tree from a sequential bit cursor, returning the
/// position just past it.
#[cfg(all(test, feature = "borsh"))]
pub(crate) fn parse_party_from<C: BitCursor>(cursor: &mut C) -> Result<u64, Decode>
where
    Decode: From<C::Error>,
{
    parse_party_core(cursor)?;
    Ok(cursor.position())
}

/// Parse and validate one party tree from a sequential bit cursor.
pub(crate) fn parse_party_core<C: BitCursor>(cursor: &mut C) -> Result<(), Decode>
where
    Decode: From<C::Error>,
{
    let mut stack = Frames::default();
    loop {
        let left = cursor.read_bit()?;
        let right = cursor.read_bit()?;

        // `summary` is whether the just-completed subtree is a terminal — the
        // only fact a parent needs to reject two terminal children.
        let mut summary = match (left, right) {
            (true, true) => {
                stack.push(Frame::BothNeedLeft);
                continue; // descend into the left child
            }
            (true, false) | (false, true) => {
                stack.push(Frame::UnaryNeedChild);
                continue; // descend into the one present child
            }
            (false, false) => true, // a terminal
        };

        // Attach the completed subtree to its parent, possibly completing it too.
        loop {
            match stack.pop() {
                None => return Ok(()), // the root is complete
                Some(Frame::BothNeedLeft) => {
                    stack.push(Frame::BothNeedRight {
                        left_terminal: summary,
                    });
                    break; // go parse the right child
                }
                Some(Frame::BothNeedRight { left_terminal }) => {
                    if left_terminal && summary {
                        return Err(Decode::NotCanonical); // two collapsible terminals
                    }
                    summary = false; // this branch cannot be a terminal child
                }
                Some(Frame::UnaryNeedChild) => {
                    summary = false; // a one-child branch is never terminal
                }
            }
        }
    }
}

/// Return the position after one preorder subtree.
///
/// `header` returns the number of children at a position and the position after
/// that node's header. The pending count makes the traversal iterative and
/// works for trees with any fixed or variable arity.
pub(crate) fn skip_subtree(mut at: u64, mut header: impl FnMut(u64) -> (u64, u64)) -> u64 {
    let mut pending: i64 = 1;
    while pending > 0 {
        let (children, next) = header(at);
        at = next;
        pending += children as i64 - 1;
    }
    at
}
