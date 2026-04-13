//! Bit-twiddling helpers operating on the 256-bit `which` array.

pub(super) const WORDS: usize = 4;
pub(super) const BITS_PER_WORD: u8 = 64;

fn word_bit(idx: u8) -> (usize, u8) {
    ((idx / BITS_PER_WORD) as usize, idx % BITS_PER_WORD)
}

pub(super) fn bit_get(which: &[u64; WORDS], idx: u8) -> bool {
    let (w, b) = word_bit(idx);
    which[w] & (1u64 << b) != 0
}

pub(super) fn bit_set(which: &mut [u64; WORDS], idx: u8) {
    let (w, b) = word_bit(idx);
    which[w] |= 1u64 << b;
}

pub(super) fn bit_clear(which: &mut [u64; WORDS], idx: u8) {
    let (w, b) = word_bit(idx);
    which[w] &= !(1u64 << b);
}

/// Number of bits set in `which` strictly below `idx`.
pub(super) fn position(which: &[u64; WORDS], idx: u8) -> usize {
    let (w, b) = word_bit(idx);
    let mut count = 0usize;
    for word in which.iter().take(w) {
        count += word.count_ones() as usize;
    }
    // b is in 0..64, so (1u64 << b) is well-defined and the subtraction
    // produces a mask of the low `b` bits (zero when b == 0).
    let mask = (1u64 << b).wrapping_sub(1);
    count += (which[w] & mask).count_ones() as usize;
    count
}

/// Total popcount of `which` (the number of set bits across all words).
pub(super) fn popcount(which: &[u64; WORDS]) -> usize {
    which.iter().map(|w| w.count_ones() as usize).sum()
}

/// Mask `which` to retain only bits at indices in `[start, end)`. Both bounds
/// are in `0..=256`. If `start >= end`, the result is all zeros.
pub(super) fn mask_range(which: [u64; WORDS], start: u16, end: u16) -> [u64; WORDS] {
    if start >= end {
        return [0; WORDS];
    }
    let mut r = which;

    // Clear bits below `start` (start is 0..=255 here since start < end <= 256).
    let s = start as u8;
    let sw = (s / BITS_PER_WORD) as usize;
    let sb = s % BITS_PER_WORD;
    for word in r.iter_mut().take(sw) {
        *word = 0;
    }
    let low_mask = (1u64 << sb).wrapping_sub(1);
    r[sw] &= !low_mask;

    // Clear bits at or above `end`. If end == 256, no upper clearing.
    if end < 256 {
        let e = end as u8;
        let ew = (e / BITS_PER_WORD) as usize;
        let eb = e % BITS_PER_WORD;
        let high_mask = !(1u64 << eb).wrapping_sub(1);
        r[ew] &= !high_mask;
        for word in r.iter_mut().skip(ew + 1) {
            *word = 0;
        }
    }

    r
}
