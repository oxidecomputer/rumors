//! A pop-able stack of nonnegative integers held as bits.

use super::bits::BitsMut;

#[cfg(test)]
mod tests;

/// A pop-able stack of nonnegative integers held as bits.
///
/// Each entry costs `2·w` bits for a `w`-bit value: the value's bits on
/// one stack, and `w` in pop-able unary — a terminator under `w − 1`
/// continuation bits — on the other. A stack with one entry per open
/// ancestor therefore prices depth in *bits*, the currency the deep-input
/// walks already pay their path and phase stacks in, where a machine word
/// per level would dominate the transient. Consumers keep entries narrow
/// by construction — deltas from a register, code lengths — so an entry
/// typically costs a few bits.
pub(crate) struct PopStack {
    /// Width markers: for each entry, one `false` under `w − 1` `true`s.
    unary: BitsMut,
    /// Value bits, most-significant pushed first so pops read the value
    /// least-significant first.
    value: BitsMut,
}

impl PopStack {
    pub(crate) fn new() -> Self {
        PopStack {
            unary: BitsMut::new(),
            value: BitsMut::new(),
        }
    }

    /// Push a value (zero included: it stores one value bit).
    pub(crate) fn push(&mut self, v: u64) {
        let width = (u64::BITS - v.leading_zeros()).max(1);
        for i in (0..width).rev() {
            self.value.push(v >> i & 1 == 1);
        }
        self.unary.push(false);
        for _ in 1..width {
            self.unary.push(true);
        }
    }

    /// Pop the most recently pushed value.
    ///
    /// # Panics
    ///
    /// Panics if the stack is empty.
    pub(crate) fn pop(&mut self) -> u64 {
        let mut width = 0u32;
        loop {
            let continuation = self.unary.pop().expect("bit stack underflow");
            width += 1;
            if !continuation {
                break;
            }
        }
        let mut v = 0u64;
        for i in 0..width {
            if self.value.pop().expect("bit stack value bits underflow") {
                v |= 1 << i;
            }
        }
        v
    }
}
