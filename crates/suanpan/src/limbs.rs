//! The limb view: a wide operand's stored words read as 64-bit limbs.
//!
//! Wide-operand costs are denominated in 64-bit limbs of the operand's
//! value, whatever the backend's storage word width, so digit-touch
//! counts are identical across targets. [`Limbs`] is the one reader
//! that makes the denomination true: it pairs narrower storage words
//! (32-bit on wasm32) into whole limbs, borrowing the stored slice so
//! streaming an operand allocates nothing.

use dashu_int::Word;

use crate::UBig;

/// Stored words per 64-bit limb: 1 where the backend word is 64 bits, 2
/// where it is 32 (wasm32).
///
/// Wide-operand costs are counted in 64-bit limbs, so pairing narrower
/// storage words keeps digit-touch counts identical across targets.
const WORDS_PER_LIMB: usize = (u64::BITS / Word::BITS) as usize;

/// Pack one limb's worth of stored words (the top chunk may be partial).
fn pack_limb(chunk: &[Word]) -> u64 {
    // One face of this cast is a no-op: `Word` is `u64` on 64-bit targets
    // and `u32` on 32-bit ones, and the cast is what compiles on both.
    #[allow(clippy::unnecessary_cast)]
    chunk.iter().enumerate().fold(0u64, |limb, (index, &word)| {
        limb | ((word as u64) << (index as u32 * Word::BITS))
    })
}

/// The 64-bit limbs of a magnitude, least significant first.
///
/// The unit this crate's wide-operand costs are counted in: a wide write
/// pays amortized O(1) digit touches per limb yielded here, whatever the
/// backend's storage word width. Borrows the stored word slice, so
/// iteration allocates nothing; the top limb zero-pads any missing high
/// words. A zero value has no limbs. Double-ended, so
/// most-significant-first consumers reverse it.
///
/// # Complexity
///
/// Construction and each step `O(1)`.
pub struct Limbs<'a> {
    chunks: core::slice::Chunks<'a, Word>,
}

impl<'a> Limbs<'a> {
    /// The limbs of `value`, borrowing its stored words.
    ///
    /// # Complexity
    ///
    /// `O(1)`.
    pub fn new(value: &'a UBig) -> Limbs<'a> {
        Limbs {
            chunks: value.as_words().chunks(WORDS_PER_LIMB),
        }
    }
}

impl Iterator for Limbs<'_> {
    type Item = u64;

    fn next(&mut self) -> Option<u64> {
        self.chunks.next().map(pack_limb)
    }
}

impl DoubleEndedIterator for Limbs<'_> {
    fn next_back(&mut self) -> Option<u64> {
        self.chunks.next_back().map(pack_limb)
    }
}
