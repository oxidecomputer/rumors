//! A stack of subtree-minimum records, written once and read once.
//!
//! Each distinct live minimum has one record: the supplying leaf's offset
//! from its reference, the number of subtree closes charged to it, and
//! whether it is the first record at its reference. The current minimum is
//! the top record; suspending it pushes the new minimum above it, so a
//! suspended record is never moved or reread until it is settled. Only the
//! top record's close count changes, so it is held explicitly.
//!
//! Offsets live in an [`IntegerStack`], so every record costs space in
//! proportion to its offset's width.

use core::cmp::Ordering;

use super::integers::{IntegerStack, Magnitude};
use crate::bits::stack::{BitStack, PackedU64Stack};

#[cfg(test)]
mod tests;

/// A record removed from the stack for settlement.
pub struct Popped {
    /// The offset's comparison with zero.
    pub sign: Ordering,
    /// The offset's magnitude.
    pub magnitude: Magnitude,
    /// Subtree closes charged to this minimum.
    pub closes: u64,
    /// Whether this was the first record at its reference.
    pub first: bool,
}

/// Live minimum records, the current minimum on top.
pub struct Records {
    /// Each record's offset from its reference.
    offsets: IntegerStack,
    /// Close counts of every record below the top.
    closes: PackedU64Stack,
    /// Whether each record is the first at its reference.
    first: BitStack,
    /// The top record's close count.
    top_closes: u64,
}

impl Records {
    /// Construct an empty stack.
    pub fn new() -> Self {
        Self {
            offsets: IntegerStack::new(),
            closes: PackedU64Stack::new(),
            first: BitStack::new(),
            top_closes: 0,
        }
    }

    /// Push a new current minimum with no closes yet, its offset given as a
    /// comparison with zero and little-endian limbs with no high zero limb.
    pub fn push(&mut self, sign: Ordering, magnitude: &[u64], first: bool) {
        if self.first.len() != 0 {
            self.closes.push(self.top_closes);
        }
        self.top_closes = 0;
        self.offsets.push(sign, magnitude.iter().copied());
        self.first.push(first);
    }

    /// Charge one more subtree close to the current minimum.
    pub fn increment(&mut self) {
        debug_assert!(
            self.first.len() != 0,
            "an armed subtree has a current minimum"
        );
        // Each close consumes one internal node of a version held in memory,
        // so the count stays far below u64::MAX.
        self.top_closes += 1;
    }

    /// Remove the current minimum, exposing the record below it.
    ///
    /// # Panics
    ///
    /// Panics if the stack is empty.
    pub fn pop(&mut self) -> Popped {
        let (sign, magnitude) = self.offsets.pop();
        let closes = self.top_closes;
        let first = self.first.pop().expect("each record has a first flag");
        self.top_closes = if self.first.len() != 0 {
            self.closes.pop()
        } else {
            0
        };
        Popped {
            sign,
            magnitude,
            closes,
            first,
        }
    }
}
