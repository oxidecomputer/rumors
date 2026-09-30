//! Compact state for parsing a party tree.

use crate::codec::BitStack;

/// The unvisited part of one open party branch.
#[derive(Clone, Copy)]
pub(super) enum Frame {
    /// A two-child branch whose left child is next.
    BothNeedLeft,
    /// A two-child branch whose right child is next.
    BothNeedRight { left_terminal: bool },
    /// A one-child branch whose child is next.
    UnaryNeedChild,
}

/// The parser's open branches, encoded in two bits apiece.
///
/// A deep spine can open one branch per two input bits. Matching that density
/// keeps parser memory proportional to the input rather than storing a Rust
/// enum for every branch.
#[derive(Default)]
pub(super) struct Frames(BitStack);

impl Frames {
    /// Retain one unfinished branch.
    pub(super) fn push(&mut self, frame: Frame) {
        let state = match frame {
            Frame::BothNeedLeft => 0b00,
            Frame::BothNeedRight {
                left_terminal: false,
            } => 0b01,
            Frame::UnaryNeedChild => 0b10,
            Frame::BothNeedRight {
                left_terminal: true,
            } => 0b11,
        };
        self.0.push(state & 0b10 != 0);
        self.0.push(state & 0b01 != 0);
    }

    /// Restore the innermost unfinished branch.
    pub(super) fn pop(&mut self) -> Option<Frame> {
        let low = self.0.pop()?;
        let high = self
            .0
            .pop()
            .expect("a parser frame always occupies two bits");
        Some(match (high, low) {
            (false, false) => Frame::BothNeedLeft,
            (false, true) => Frame::BothNeedRight {
                left_terminal: false,
            },
            (true, false) => Frame::UnaryNeedChild,
            (true, true) => Frame::BothNeedRight {
                left_terminal: true,
            },
        })
    }
}
