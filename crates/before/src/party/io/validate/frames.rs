//! Compact state for validating a party tree.

use crate::bits::stack::BitStack;

/// The unvisited part of one open branch.
#[derive(Clone, Copy)]
pub enum Frame {
    /// A two-child branch whose left child is next.
    BothNeedLeft,
    /// A two-child branch whose right child is next.
    BothNeedRight { left_owned: bool },
    /// A one-child branch whose only child is next.
    OneChild,
}

/// Open branches, encoded in two bits apiece.
///
/// A deep party can open one branch per two input bits. Matching that density
/// avoids amplifying such an input into one machine-word frame per branch.
#[derive(Default)]
pub struct Frames(BitStack);

impl Frames {
    /// Retain one unfinished branch.
    pub fn push(&mut self, frame: Frame) {
        let state = match frame {
            Frame::BothNeedLeft => 0b00,
            Frame::BothNeedRight { left_owned: false } => 0b01,
            Frame::OneChild => 0b10,
            Frame::BothNeedRight { left_owned: true } => 0b11,
        };
        self.0.push(state & 0b10 != 0);
        self.0.push(state & 0b01 != 0);
    }

    /// Restore the innermost unfinished branch.
    pub fn pop(&mut self) -> Option<Frame> {
        let low = self.0.pop()?;
        let high = self
            .0
            .pop()
            .expect("a validation frame always occupies two bits");
        Some(match (high, low) {
            (false, false) => Frame::BothNeedLeft,
            (false, true) => Frame::BothNeedRight { left_owned: false },
            (true, false) => Frame::OneChild,
            (true, true) => Frame::BothNeedRight { left_owned: true },
        })
    }
}
