//! Pop-able stacks held as bits: the word-backed bit stack the deep walks keep
//! their paths and phases on, and the nonnegative-integer stack built over it.
//!
//! Depth is priced in *bits* here — one path bit per open ancestor, `~2·w` bits
//! per stacked `w`-bit integer — the currency the deep-input walks pay their
//! transient in, where a machine word per level would dominate it. The backing
//! is machine words, so every push and pop is shift arithmetic on a register
//! with at most one word spill.

#[cfg(test)]
mod tests;

/// A pop-able stack of bits over machine words.
///
/// The newest bit lives at the low end of the top register; a filled register
/// spills whole into the word vector and refills on the pop that crosses back.
/// Every operation is O(1) with no bit-addressing arithmetic.
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
    ///
    /// `words.len() * 64` fits `usize` on every target: a walk holds a few
    /// bits per open ancestor here, depth is bounded by the walked stream's
    /// bit length, and stored streams cap at `usize::MAX >> 3` bits (the
    /// borrowed-view encoding) — so the height stays multiple binary orders
    /// of magnitude below any `usize` wrap, even on 32-bit targets.
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

    /// Push `len <= 63` bits at once, oldest at the value's high end — popping
    /// returns them newest-first, exactly as `len` single pushes of the value's
    /// bits from high to low.
    fn push_bits(&mut self, value: u64, len: u32) {
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
    /// [`push_bits`](Self::push_bits) packed them: the inverse, equal to `len`
    /// single pops assembled low bit first.
    ///
    /// # Panics
    ///
    /// Panics if fewer than `len` bits are held.
    fn pop_bits(&mut self, len: u32) -> u64 {
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
    /// One word read per 64 bits of the run: the cost is the run the caller is
    /// about to pop (or has decided not to), never the whole stack.
    pub(crate) fn trailing_ones(&self) -> usize {
        let top_run = self.top.trailing_ones().min(self.top_len);
        if top_run < self.top_len {
            return top_run as usize;
        }
        let mut run = top_run as usize;
        for &word in self.words.iter().rev() {
            let w = word.trailing_ones();
            run += w as usize;
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
    fn trailing_ones_capped(&self) -> u32 {
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

/// A pop-able stack of nonnegative integers held as bits.
///
/// Each entry costs `2·w` bits for a `w`-bit value: the value's bits on one
/// stack, and `w` in pop-able unary — a terminator under `w − 1` continuation
/// bits — on the other. A stack with one entry per open ancestor therefore
/// prices depth in *bits*, the currency the deep-input walks already pay their
/// path and phase stacks in, where a machine word per level would dominate the
/// transient. Consumers keep entries narrow by construction — deltas from a
/// register, code lengths — so an entry typically costs a few bits.
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
        // The value's bits, high first; pops then read it back low first, which
        // is the packing `pop_bits` returns whole. The width marker is `width -
        // 1` continuation `true`s over one `false` terminator: newest-first
        // pops read the `true`s, so they sit above the terminator on the stack.
        if width == 64 {
            self.value.push(v >> 63 & 1 == 1);
            self.value.push_bits(v & (u64::MAX >> 1), 63);
        } else {
            self.value.push_bits(v, width);
        }
        if width == 64 {
            self.unary.push(false);
            self.unary.push_bits(u64::MAX >> 1, 63);
        } else {
            self.unary.push_bits((1u64 << (width - 1)) - 1, width);
        }
    }

    /// Pop the most recently pushed value.
    ///
    /// # Panics
    ///
    /// Panics if the stack is empty.
    pub(crate) fn pop(&mut self) -> u64 {
        // The width scan: count continuation bits in the registers, falling
        // back to single pops past the batched cap.
        let quick = self.unary.trailing_ones_capped();
        let width = if quick < 62 {
            self.unary.pop_bits(quick + 1);
            quick + 1
        } else {
            let mut width = 0u32;
            loop {
                let continuation = self.unary.pop().expect("bit stack underflow");
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
