//! Compact control state for the iterative join.

use crate::bits::stack::BitStack;

/// State retained for one open output branch.
pub enum Frame {
    /// Its right child pair remains to be joined.
    PendingRight {
        /// Whether the first operand has a right child.
        a_present: bool,
        /// Whether the second operand has a right child.
        b_present: bool,
    },
    /// Close the branch when its current child completes.
    PendingClose { left_owned: bool },
}

/// The open branches of a join, encoded in two or three bits apiece.
///
/// A pending right child retains the two operand-presence bits needed to resume
/// the walk. A pending close retains only whether the left output was owned,
/// which is the one fact normalization needs. The consuming cursors
/// retain their own positions, so the stack stores no offsets or values.
pub struct Frames(BitStack);

impl Frames {
    /// Create an empty join stack.
    pub fn new() -> Self {
        Self(BitStack::new())
    }

    /// Queue the right child pair while the walk enters the left pair.
    pub fn push_pending_right(&mut self, a_present: bool, b_present: bool) {
        self.0.push(a_present);
        self.0.push(b_present);
        self.0.push(true);
    }

    /// Retain the left child's kind while the right pair completes.
    pub fn push_pending_close(&mut self, left_owned: bool) {
        self.0.push(left_owned);
        self.0.push(false);
    }

    /// Restore the innermost open branch.
    pub fn pop(&mut self) -> Option<Frame> {
        let pending_right = self.0.pop()?;
        if pending_right {
            let b_present = self.0.pop().expect("a pending-right frame is three bits");
            let a_present = self.0.pop().expect("a pending-right frame is three bits");
            Some(Frame::PendingRight {
                a_present,
                b_present,
            })
        } else {
            let left_owned = self.0.pop().expect("a pending-close frame is two bits");
            Some(Frame::PendingClose { left_owned })
        }
    }
}
