//! A word-backed stack of individual bits.

#[cfg(test)]
mod tests;

/// A stack of bits over machine words.
///
/// The newest bit lives at the low end of the top register; a filled register
/// spills whole into the word vector and refills on the pop that crosses back.
/// Every operation is O(1) with no bit-addressing arithmetic.
#[derive(Default)]
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
    /// An empty stack.
    pub(crate) fn new() -> Self {
        BitStack::default()
    }

    /// The stack's height in bits.
    ///
    /// `u64`, the walks' depth denomination: a stack this deep occupies
    /// real memory (its words), so the height is bounded by allocatable
    /// memory — past a 32-bit `usize` from 512 MiB of stack, and exactly
    /// representable here on every target.
    pub(crate) fn len(&self) -> u64 {
        self.words.len() as u64 * 64 + u64::from(self.top_len)
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

    /// Push `len <= 63` bits at once, oldest at the value's high end — popping
    /// returns them newest-first, exactly as `len` single pushes of the value's
    /// bits from high to low.
    pub(crate) fn push_bits(&mut self, value: u64, len: u32) {
        debug_assert!(len <= 63 && (len == 64 || value >> len == 0));
        let total = self.top_len + len;
        if total <= 64 {
            self.top = if len == 64 {
                value
            } else {
                (self.top << len) | value
            };
            self.top_len = total;
            return;
        }
        let spill = 64 - self.top_len;
        self.words
            .push((self.top << spill) | (value >> (len - spill)));
        self.top = value & ((1u64 << (len - spill)) - 1);
        self.top_len = len - spill;
    }

    /// Pop `len <= 63` bits at once, returned exactly as
    /// [`push_bits`](Self::push_bits) stored them: the inverse, equal to `len`
    /// single pops assembled low bit first.
    ///
    /// # Panics
    ///
    /// Panics if fewer than `len` bits are held.
    pub(crate) fn pop_bits(&mut self, len: u32) -> u64 {
        debug_assert!(len <= 63);
        if len <= self.top_len {
            let value = self.top & ((1u64 << len) - 1);
            self.top >>= len;
            self.top_len -= len;
            return value;
        }
        let low_len = self.top_len;
        let low = self.top;
        let rest = len - low_len;
        self.top = self.words.pop().expect("bit stack underflow");
        self.top_len = 64;
        let high = self.pop_bits(rest);
        (high << low_len) | low
    }

    /// The exact run of set bits at the top of the stack.
    ///
    /// Runs in `O(1 + r / 64)`, where `r` is the returned run length. The
    /// result is `u64`, like [`len`](Self::len), because the run can span the
    /// stack's full height.
    pub(crate) fn trailing_ones(&self) -> u64 {
        let top_run = self.top.trailing_ones().min(self.top_len);
        if top_run < self.top_len {
            return u64::from(top_run);
        }
        let mut run = u64::from(top_run);
        for &word in self.words.iter().rev() {
            let w = word.trailing_ones();
            run += u64::from(w);
            if w < 64 {
                break;
            }
        }
        run
    }

    /// The run of set bits at the top of the stack, capped at 62.
    ///
    /// Reads only the top register and at most one spilled word (a cap under 63
    /// never needs a second): the integer stack's width scan. Bits above the
    /// register's live length are zero by construction, so `trailing_ones`
    /// stops inside the live region or exactly at its edge.
    pub(crate) fn trailing_ones_capped(&self) -> u32 {
        let mut run = self.top.trailing_ones().min(self.top_len);
        if run == self.top_len {
            if let Some(&word) = self.words.last() {
                run += word.trailing_ones();
            }
        }
        run.min(62)
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
