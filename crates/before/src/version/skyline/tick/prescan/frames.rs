//! Suspended traversal state for a lookahead scan.

use crate::codec::{BitStack, PopStack};

use super::super::frames::Position;

/// Work to resume after the current child or sibling range closes.
pub(super) enum Frame {
    /// An owned-left branch whose sibling minimum is being computed.
    Lookahead,
    /// An ordinary node whose left child's range is in flight.
    AwaitLeft,
    /// An ordinary node whose right child's range is in flight.
    AwaitRight,
}

/// Suspended ancestors of the range currently being scanned.
///
/// Three bit stacks hold the frame kind, phase, and right-child presence.
/// Lookahead frames also carry a memo slot. Those slots increase in stream
/// order, so [`Position`] stores their differences compactly.
pub(super) struct Frames {
    /// Per frame: a lookahead (true) or an ordinary branch (false).
    lookahead: BitStack,
    /// Ordinary frames: awaiting the left (false) or right (true) child's
    /// range. Lookahead frames: false, unread.
    phase: BitStack,
    /// Ordinary frames: whether the right child is present. Lookahead frames:
    /// false, unread.
    right_present: BitStack,
    /// Per lookahead frame: the memo-slot delta against a monotone register.
    values: PopStack,
    /// The top lookahead's memo slot; reservations run in stream order.
    slots: Position,
}

impl Frames {
    /// Construct an empty traversal stack.
    pub(super) fn new() -> Self {
        Self {
            lookahead: BitStack::new(),
            phase: BitStack::new(),
            right_present: BitStack::new(),
            values: PopStack::new(),
            slots: Position::new(),
        }
    }

    /// The top frame's kind, if any frame is open.
    pub(super) fn top(&self) -> Option<Frame> {
        let lookahead = self.lookahead.last()?;
        Some(if lookahead {
            Frame::Lookahead
        } else if self
            .phase
            .last()
            .expect("kind and phase stacks hold one bit per frame")
        {
            Frame::AwaitRight
        } else {
            Frame::AwaitLeft
        })
    }

    /// Whether the top ordinary frame has a right child.
    pub(super) fn right_present(&self) -> bool {
        self.right_present
            .last()
            .expect("an open frame records right-child presence")
    }

    /// Suspend an ordinary node while its left child is scanned.
    pub(super) fn push_node(&mut self, right_present: bool) {
        self.lookahead.push(false);
        self.phase.push(false);
        self.right_present.push(right_present);
    }

    /// Suspend a lookahead while its later sibling is scanned.
    pub(super) fn push_lookahead(&mut self, slot: usize) {
        self.slots.push(&mut self.values, slot as u64);
        self.lookahead.push(true);
        self.phase.push(false);
        self.right_present.push(false);
    }

    /// Mark that the top ordinary frame is now waiting for its right child.
    pub(super) fn await_right(&mut self) {
        debug_assert!(
            self.phase.last() == Some(false) && self.lookahead.last() == Some(false),
            "only a node waiting for its left child can advance"
        );
        self.phase.set_last(true);
    }

    /// Close an ordinary node's frame.
    pub(super) fn pop_node(&mut self) {
        self.lookahead.pop();
        self.phase.pop();
        self.right_present.pop();
    }

    /// Close a lookahead frame and return its memo slot.
    pub(super) fn pop_lookahead(&mut self) -> usize {
        self.lookahead.pop();
        self.phase.pop();
        self.right_present.pop();
        // The value came from a memo index and therefore fits `usize`.
        self.slots.pop(&mut self.values) as usize
    }
}
