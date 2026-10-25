//! The limbs a magnitude read returns, held inline while they fit.
//!
//! A read normalizes the digits it covers and emits each unsigned base-2^32
//! digit from low to high; [`DigitPacker`] pairs them into 64-bit limbs as
//! they arrive. Limbs stay inline until a third one arrives, so a magnitude
//! below `2^128` reads without allocating, and a wider one is written into a
//! single allocation sized to the digits read, with no intermediate digit
//! buffer.

use core::fmt;
use core::ops::Deref;

/// The little-endian 64-bit limbs of a magnitude, as
/// [`signed_magnitude`](super::Accumulator::signed_magnitude) and
/// [`scaled_signed_magnitude`](super::Accumulator::scaled_signed_magnitude)
/// return them.
///
/// `Limbs` dereferences to `[u64]`. The slice has no high zero limb, so it is
/// empty exactly when the magnitude is zero. Up to two limbs, any magnitude
/// below `2^128`, are held inline without allocating; a wider magnitude is
/// held in one allocation.
///
/// ```
/// use suanpan::Accumulator;
///
/// let mut total = Accumulator::new();
/// total.add_shifted_limbs(0, [7, 1]);
/// let (_, limbs) = total.signed_magnitude();
/// assert_eq!(*limbs, [7, 1]);
/// ```
#[derive(Clone)]
pub struct Limbs(Storage);

/// Where the limbs live.
///
/// The inline variants fit beside `Vec`'s own niche, so `Limbs` is no larger
/// than the `Vec` alone: a caller storing many of them pays nothing for the
/// inline cases.
#[derive(Clone)]
enum Storage {
    /// No limbs: the magnitude is zero.
    Zero,
    /// One limb.
    One(u64),
    /// Two limbs, low first.
    Two([u64; 2]),
    /// More limbs than fit inline.
    Spilled(Vec<u64>),
}

#[cfg(target_pointer_width = "64")]
const _: () = assert!(core::mem::size_of::<Limbs>() == core::mem::size_of::<Vec<u64>>());

impl Limbs {
    /// Hold no limbs.
    const fn empty() -> Limbs {
        Limbs(Storage::Zero)
    }

    /// Hold a magnitude below `2^128`, inline.
    pub(crate) fn from_u128(magnitude: u128) -> Limbs {
        let mut limbs = Limbs::empty();
        limbs.push(magnitude as u64, 0);
        limbs.push((magnitude >> 64) as u64, 0);
        limbs.trim();
        limbs
    }

    /// Append a limb, spilling to an allocation of `spill_capacity` limbs, or
    /// more, when the inline storage is full, and growing a full allocation
    /// by exactly one limb.
    fn push(&mut self, limb: u64, spill_capacity: usize) {
        self.0 = match core::mem::replace(&mut self.0, Storage::Zero) {
            Storage::Zero => Storage::One(limb),
            Storage::One(low) => Storage::Two([low, limb]),
            Storage::Two([low, high]) => {
                let mut spilled = Vec::with_capacity(spill_capacity.max(3));
                spilled.extend([low, high, limb]);
                Storage::Spilled(spilled)
            }
            Storage::Spilled(mut limbs) => {
                // Growth comes only from a normalizing read's one carry digit,
                // so one more limb suffices; doubling would retain spare
                // capacity for the result's whole life.
                if limbs.len() == limbs.capacity() {
                    limbs.reserve_exact(1);
                }
                limbs.push(limb);
                Storage::Spilled(limbs)
            }
        };
    }

    /// Drop high zero limbs.
    fn trim(&mut self) {
        match &mut self.0 {
            Storage::Spilled(limbs) => {
                while limbs.last() == Some(&0) {
                    limbs.pop();
                }
            }
            Storage::Two([low, 0]) => self.0 = Storage::One(*low),
            Storage::Zero | Storage::One(_) | Storage::Two(_) => {}
        }
        if let Storage::One(0) = self.0 {
            self.0 = Storage::Zero;
        }
    }

    /// The last limb, which exists while a digit pair is open.
    fn last_mut(&mut self) -> &mut u64 {
        match &mut self.0 {
            Storage::One(limb) | Storage::Two([_, limb]) => limb,
            Storage::Spilled(limbs) => limbs
                .last_mut()
                .expect("a spilled allocation holds at least three limbs"),
            Storage::Zero => {
                unreachable!("a digit pair is open only after its low digit pushed a limb")
            }
        }
    }
}

/// Dereference to the limbs in use.
impl Deref for Limbs {
    type Target = [u64];

    fn deref(&self) -> &[u64] {
        match &self.0 {
            Storage::Zero => &[],
            Storage::One(limb) => core::slice::from_ref(limb),
            Storage::Two(limbs) => limbs,
            Storage::Spilled(limbs) => limbs,
        }
    }
}

/// Borrow the limbs in use.
impl AsRef<[u64]> for Limbs {
    fn as_ref(&self) -> &[u64] {
        self
    }
}

/// Format as the slice of limbs in use, whatever the storage.
impl fmt::Debug for Limbs {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, formatter)
    }
}

/// Collects base-2^32 digits, low to high, into [`Limbs`].
pub(crate) struct DigitPacker {
    /// The limbs packed so far; the last may hold only its low digit.
    limbs: Limbs,
    /// Digits pushed so far.
    digits: usize,
    /// Limbs to allocate if the digits spill past the inline storage.
    spill_capacity: usize,
}

impl DigitPacker {
    /// Begin collecting about `expected_digits` digits, which sizes the
    /// allocation should they spill; more digits grow it one limb at a time.
    pub(crate) fn new(expected_digits: usize) -> DigitPacker {
        DigitPacker {
            limbs: Limbs::empty(),
            digits: 0,
            spill_capacity: expected_digits.div_ceil(2),
        }
    }

    /// Append the next digit above those already pushed.
    pub(crate) fn push(&mut self, digit: u32) {
        if self.digits.is_multiple_of(2) {
            self.limbs.push(u64::from(digit), self.spill_capacity);
        } else {
            *self.limbs.last_mut() |= u64::from(digit) << 32;
        }
        self.digits += 1;
    }

    /// Return the minimal limbs, with high zeros dropped.
    pub(crate) fn finish(mut self) -> Limbs {
        self.limbs.trim();
        self.limbs
    }
}

#[cfg(test)]
mod tests;
