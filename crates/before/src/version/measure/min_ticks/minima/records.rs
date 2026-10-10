//! A stack of subtree-minimum records, written once and read once.
//!
//! Each distinct live minimum has one record: the supplying leaf's offset
//! from its reference, the number of subtree closes charged to it, and
//! whether it is the first record at its reference. The current minimum is
//! the top record; suspending it pushes the new minimum above it, so a
//! suspended record is never moved or reread until it is settled. Only the
//! top record's close count changes, so it is held explicitly.
//!
//! Every field is stored at its own width, so the stack has no capacity
//! threshold: an offset of `w` bits costs about `2w` bits whatever `w` is.

use num_bigint::{BigInt, BigUint, Sign};

use crate::bits::stack::{BitStack, PackedU64Stack};

#[cfg(test)]
mod tests;

/// A record removed from the stack for settlement.
pub struct Popped {
    /// The supplying leaf's offset from its reference.
    pub offset: BigInt,
    /// Subtree closes charged to this minimum.
    pub closes: u64,
    /// Whether this was the first record at its reference.
    pub first: bool,
}

/// Live minimum records, the current minimum on top.
pub struct Records {
    /// Offset magnitude limbs, most significant pushed first, then a header
    /// `limb count << 1 | negative`.
    offsets: PackedU64Stack,
    /// Close counts of every record below the top.
    closes: PackedU64Stack,
    /// Whether each record is the first at its reference.
    first: BitStack,
    /// The top record's close count.
    top_closes: u64,
    /// Records held.
    len: u64,
}

impl Records {
    /// Construct an empty stack.
    pub fn new() -> Self {
        Self {
            offsets: PackedU64Stack::new(),
            closes: PackedU64Stack::new(),
            first: BitStack::new(),
            top_closes: 0,
            len: 0,
        }
    }

    /// Push a new current minimum with no closes yet.
    pub fn push(&mut self, offset: &BigInt, first: bool) {
        if self.len != 0 {
            self.closes.push(self.top_closes);
        }
        self.top_closes = 0;
        let (sign, limbs) = offset.to_u64_digits();
        for &limb in limbs.iter().rev() {
            self.offsets.push(limb);
        }
        let count = u64::try_from(limbs.len()).expect("a limb count fits u64");
        self.offsets
            .push(count << 1 | u64::from(sign == Sign::Minus));
        self.first.push(first);
        self.len += 1;
    }

    /// Charge one more subtree close to the current minimum.
    pub fn increment(&mut self) {
        debug_assert!(self.len != 0, "an armed subtree has a current minimum");
        // Each close consumes one internal node of the input, so the count
        // cannot outgrow a u64.
        self.top_closes += 1;
    }

    /// Remove the current minimum, exposing the record below it.
    pub fn pop(&mut self) -> Popped {
        let header = self.offsets.pop();
        let count = header >> 1;
        let mut digits = Vec::new();
        for _ in 0..count {
            let limb = self.offsets.pop();
            digits.push(limb as u32);
            digits.push((limb >> 32) as u32);
        }
        let sign = if header & 1 == 1 {
            Sign::Minus
        } else {
            Sign::Plus
        };
        let offset = BigInt::from_biguint(sign, BigUint::new(digits));
        let closes = self.top_closes;
        let first = self.first.pop().expect("each record has a first flag");
        self.len -= 1;
        self.top_closes = if self.len != 0 { self.closes.pop() } else { 0 };
        Popped {
            offset,
            closes,
            first,
        }
    }
}
