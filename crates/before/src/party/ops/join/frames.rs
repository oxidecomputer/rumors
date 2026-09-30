//! Compact control state for the iterative join.

use crate::codec::BitStack;

/// State retained for one open output branch.
pub(super) enum Frame {
    /// Its right child pair remains to be joined.
    PendingRight { ar: bool, br: bool },
    /// Close the branch when its current child completes.
    PendingClose { left_terminal: bool },
}

/// The open branches of a join, encoded in two or three bits apiece.
///
/// A pending right child retains the two operand-presence bits needed to resume
/// the walk. A pending close retains only whether the left output was a
/// terminal, which is the one fact normalization needs. The consuming cursors
/// retain their own positions, so the stack stores no offsets or values.
pub(super) struct Frames(BitStack);

impl Frames {
    /// Create an empty join stack.
    pub(super) fn new() -> Self {
        Self(BitStack::new())
    }

    /// Queue the right child pair while the walk enters the left pair.
    pub(super) fn push_pending_right(&mut self, ar: bool, br: bool) {
        self.0.push(ar);
        self.0.push(br);
        self.0.push(true);
    }

    /// Retain the left child's kind while the right pair completes.
    pub(super) fn push_pending_close(&mut self, left_terminal: bool) {
        self.0.push(left_terminal);
        self.0.push(false);
    }

    /// Restore the innermost open branch.
    pub(super) fn pop(&mut self) -> Option<Frame> {
        let pending_right = self.0.pop()?;
        if pending_right {
            let br = self.0.pop().expect("a pending-right frame is three bits");
            let ar = self.0.pop().expect("a pending-right frame is three bits");
            Some(Frame::PendingRight { ar, br })
        } else {
            let left_terminal = self.0.pop().expect("a pending-close frame is two bits");
            Some(Frame::PendingClose { left_terminal })
        }
    }
}
