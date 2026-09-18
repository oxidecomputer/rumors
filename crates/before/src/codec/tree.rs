use crate::error::Decode;

use super::{BitCursor, BitsBuf, BitsView};

/// The unvisited part of an unfinished party node.
#[derive(Clone, Copy)]
enum IdFrame {
    /// A both-present node: the next subtree is its left child.
    BothNeedLeft,
    /// A both-present node whose left child is complete.
    BothNeedRight { left_terminal: bool },
    /// A unary node (left- or right-only): the next subtree is its one child.
    UnaryNeedChild,
}

/// Two bits per open node, matching the two bits each node contributes to the
/// encoded input.
///
/// The parser may have one frame per input node on a deep spine. Packing its
/// four states keeps that auxiliary stack proportional to the bytes that force
/// it, rather than storing a Rust enum beside every two input bits.
#[derive(Default)]
struct IdFrames(BitsBuf);

impl IdFrames {
    /// Push one unfinished-node state.
    fn push(&mut self, frame: IdFrame) {
        let state = match frame {
            IdFrame::BothNeedLeft => 0b00,
            IdFrame::BothNeedRight {
                left_terminal: false,
            } => 0b01,
            IdFrame::UnaryNeedChild => 0b10,
            IdFrame::BothNeedRight {
                left_terminal: true,
            } => 0b11,
        };
        self.0.push_bits(state, 2);
    }

    /// Pop the newest unfinished-node state.
    fn pop(&mut self) -> Option<IdFrame> {
        let low = self.0.pop()?;
        let high = self
            .0
            .pop()
            .expect("a parser frame always occupies two bits");
        Some(match (high, low) {
            (false, false) => IdFrame::BothNeedLeft,
            (false, true) => IdFrame::BothNeedRight {
                left_terminal: false,
            },
            (true, false) => IdFrame::UnaryNeedChild,
            (true, true) => IdFrame::BothNeedRight {
                left_terminal: true,
            },
        })
    }
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
    let mut stack = IdFrames::default();
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
