//! Compact suspended state for the iterative tick walk.

use crate::bits::stack::{BitStack, PackedU64Stack};

use super::route::Cost;

/// Work still owed by one open version branch.
pub enum Frame {
    /// A branch whose later minimum was computed by lookahead.
    Lookahead,
    /// A branch waiting for its left child.
    AwaitLeft,
    /// A branch waiting for its right child.
    AwaitRight,
}

/// A monotone position stored as deltas on a shared value stack.
pub struct Position {
    current: u64,
}

impl Position {
    /// Start before the first position.
    pub fn new() -> Self {
        Self { current: 0 }
    }

    /// Retain an advancing position.
    pub fn push(&mut self, values: &mut PackedU64Stack, position: u64) {
        debug_assert!(position >= self.current, "positions only advance");
        values.push(position - self.current);
        self.current = position;
    }

    /// Restore the preceding position and return the current one.
    pub fn pop(&mut self, values: &mut PackedU64Stack) -> u64 {
        let position = self.current;
        self.current -= values.pop();
        position
    }
}

/// The tick walk's open branches.
///
/// Each branch contributes three control bits. Route positions and deferred
/// child costs use compact integers on a shared stack, so a deep version does
/// not allocate one Rust frame per level.
pub struct Frames {
    /// Whether each frame represents a pre-scanned lookahead.
    lookahead: BitStack,
    /// Whether an ordinary frame is waiting for its right child.
    phase: BitStack,
    /// Right-child presence, or whether a lookahead began the pre-scan.
    aux: BitStack,
    /// Position deltas and deferred costs.
    values: PackedU64Stack,
    /// Route position of the top frame.
    keys: Position,
}

impl Frames {
    /// Create an empty frame stack.
    pub fn new() -> Self {
        Self {
            lookahead: BitStack::new(),
            phase: BitStack::new(),
            aux: BitStack::new(),
            values: PackedU64Stack::new(),
            keys: Position::new(),
        }
    }

    /// Number of open branches.
    pub fn len(&self) -> u64 {
        self.lookahead.len()
    }

    /// Work owed by the innermost branch.
    pub fn top(&self) -> Option<Frame> {
        let lookahead = self.lookahead.last()?;
        Some(if lookahead {
            Frame::Lookahead
        } else if self.phase.last().expect("each frame carries one phase bit") {
            Frame::AwaitRight
        } else {
            Frame::AwaitLeft
        })
    }

    /// Whether the top branch has a right child or began the outer scan.
    pub fn aux_top(&self) -> bool {
        self.aux
            .last()
            .expect("each frame carries one auxiliary bit")
    }

    /// Suspend a branch while walking its left child.
    pub fn push_node(&mut self, key: u64, right: bool) {
        self.keys.push(&mut self.values, key);
        self.lookahead.push(false);
        self.phase.push(false);
        self.aux.push(right);
    }

    /// Suspend a pre-scanned lookahead while walking its sibling.
    pub fn push_lookahead(&mut self, key: u64, outermost: bool) {
        self.keys.push(&mut self.values, key);
        self.lookahead.push(true);
        self.phase.push(false);
        self.aux.push(outermost);
    }

    /// Retain the left cost and begin the right child.
    pub fn flip_to_await_right(&mut self, left: Cost) {
        debug_assert!(
            self.phase.last() == Some(false) && self.lookahead.last() == Some(false),
            "only a branch waiting for its left child can advance"
        );
        self.phase.set_last(true);
        let (expansions, depth) = left.stack_words();
        self.values.push(expansions);
        self.values.push(depth);
    }

    /// Close a branch and restore its route position.
    pub fn pop_key(&mut self) -> u64 {
        self.lookahead.pop();
        self.phase.pop();
        self.aux.pop();
        self.keys.pop(&mut self.values)
    }

    /// Close a branch whose right side was resolved without descent.
    pub fn pop_await_left(&mut self) -> u64 {
        self.pop_key()
    }

    /// Close a branch after both children have completed.
    pub fn pop_await_right(&mut self) -> (u64, Cost) {
        let depth = self.values.pop();
        let expansions = self.values.pop();
        (self.pop_key(), Cost::from_stack_words(expansions, depth))
    }

    /// Close a pre-scanned lookahead.
    pub fn pop_lookahead(&mut self) -> (u64, bool) {
        let outermost = self.aux_top();
        (self.pop_key(), outermost)
    }
}
