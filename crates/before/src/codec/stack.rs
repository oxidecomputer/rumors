//! Pop-able stacks held as bits: the word-backed bit stack the deep
//! walks keep their paths and phases on, and the nonnegative-integer
//! stack built over it.
//!
//! Depth is priced in *bits* here — one path bit per open ancestor,
//! `~2·w` bits per stacked `w`-bit integer — the currency the deep-input
//! walks pay their transient in, where a machine word per level would
//! dominate it. The backing is machine words, so every push and pop is
//! shift arithmetic on a register with at most one word spill.

#[cfg(test)]
mod tests;

/// A pop-able stack of bits over machine words.
///
/// The newest bit lives at the low end of the top register; a filled
/// register spills whole into the word vector and refills on the pop
/// that crosses back. Every operation is O(1) with no bit-addressing
/// arithmetic.
pub(crate) struct BitStack {
    /// Completed 64-bit groups below the top register, oldest first.
    words: Vec<u64>,
    /// The newest bits, newest at bit 0; only the low
    /// [`top_len`](Self::top_len) bits are live.
    top: u64,
    /// Live bits in [`top`](Self::top), `0..=64`.
    top_len: u32,
}

impl BitStack {
    pub(crate) fn new() -> Self {
        BitStack {
            words: Vec::new(),
            top: 0,
            top_len: 0,
        }
    }

    /// The stack's height in bits.
    pub(crate) fn len(&self) -> usize {
        self.words.len() * 64 + self.top_len as usize
    }

    /// Push one bit.
    pub(crate) fn push(&mut self, bit: bool) {
        if self.top_len == 64 {
            self.words.push(self.top);
            self.top = 0;
            self.top_len = 0;
        }
        self.top = (self.top << 1) | u64::from(bit);
        self.top_len += 1;
    }

    /// Pop the newest bit.
    pub(crate) fn pop(&mut self) -> Option<bool> {
        if self.top_len == 0 {
            self.top = self.words.pop()?;
            self.top_len = 64;
        }
        let bit = self.top & 1 == 1;
        self.top >>= 1;
        self.top_len -= 1;
        Some(bit)
    }

    /// Overwrite the newest bit.
    ///
    /// # Panics
    ///
    /// Panics if the stack is empty.
    pub(crate) fn set_last(&mut self, bit: bool) {
        if self.top_len > 0 {
            self.top = (self.top & !1) | u64::from(bit);
        } else {
            let word = self.words.last_mut().expect("set_last on an empty stack");
            *word = (*word & !1) | u64::from(bit);
        }
    }

    /// The newest bit, unpopped.
    pub(crate) fn last(&self) -> Option<bool> {
        if self.top_len > 0 {
            Some(self.top & 1 == 1)
        } else {
            self.words.last().map(|w| w & 1 == 1)
        }
    }

    /// Whether every held bit is set (vacuously true when empty).
    pub(crate) fn all_set(&self) -> bool {
        let top_all = match self.top_len {
            0 => true,
            64 => self.top == u64::MAX,
            n => self.top == (1u64 << n) - 1,
        };
        top_all && self.words.iter().all(|&w| w == u64::MAX)
    }
}

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
    unary: BitStack,
    /// Value bits, most-significant pushed first so pops read the value
    /// least-significant first.
    value: BitStack,
}

impl PopStack {
    pub(crate) fn new() -> Self {
        PopStack {
            unary: BitStack::new(),
            value: BitStack::new(),
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
