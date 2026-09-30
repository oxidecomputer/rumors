//! A compact stack of nonnegative machine-word values.

use super::BitStack;

#[cfg(test)]
mod tests;

/// A stack of `u64` values packed in proportion to their bit width.
///
/// Values and widths occupy parallel bit stacks. A width `w` is stored as one
/// `false` below `w - 1` `true` bits, so it can be decoded from the newest end
/// without an index or a machine word per entry. The complete entry costs
/// `2w` bits. This favors the small deltas, code lengths, and positions held by
/// deep tree walks while still representing every `u64`.
pub(crate) struct PackedU64Stack {
    /// Self-delimiting widths: one `false` below `w - 1` `true`s per entry.
    widths: BitStack,
    /// Value bits, most-significant pushed first so pops read the value
    /// least-significant first.
    value: BitStack,
}

impl PackedU64Stack {
    /// Construct an empty stack.
    pub(crate) fn new() -> Self {
        PackedU64Stack {
            widths: BitStack::new(),
            value: BitStack::new(),
        }
    }

    /// Push a value, including zero.
    pub(crate) fn push(&mut self, value: u64) {
        let width = (u64::BITS - value.leading_zeros()).max(1);
        // Store the value high bit first. Pops then recover its low bits first,
        // which is the order `pop_bits` assembles into a word.
        if width == 64 {
            self.value.push(value >> 63 & 1 == 1);
            self.value.push_bits(value & (u64::MAX >> 1), 63);
        } else {
            self.value.push_bits(value, width);
        }

        // The terminator is pushed first, below the continuation bits, so a
        // pop sees `width - 1` set bits followed by the clear terminator.
        if width == 64 {
            self.widths.push(false);
            self.widths.push_bits(u64::MAX >> 1, 63);
        } else {
            self.widths.push_bits((1u64 << (width - 1)) - 1, width);
        }
    }

    /// Pop the most recently pushed value.
    ///
    /// # Panics
    ///
    /// Panics if the stack is empty.
    pub(crate) fn pop(&mut self) -> u64 {
        // Most widths fit in the two registers inspected by this bounded
        // count. Only the widest values fall back to individual bit pops.
        let quick = self.widths.trailing_ones_capped();
        let width = if quick < 62 {
            self.widths.pop_bits(quick + 1);
            quick + 1
        } else {
            let mut width = 0u32;
            loop {
                let continuation = self.widths.pop().expect("bit stack underflow");
                width += 1;
                if !continuation {
                    break;
                }
            }
            width
        };
        if width == 64 {
            let low = self.value.pop_bits(63);
            let top = u64::from(self.value.pop().expect("bit stack value bits underflow"));
            (top << 63) | low
        } else {
            self.value.pop_bits(width)
        }
    }
}
