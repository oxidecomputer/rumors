//! A LIFO stack of signed integers, each written once and read once.
//!
//! A magnitude of at most [`PACKED_BITS`] bits is packed at its exact bit
//! length into a shared bit stack, behind a self-delimiting header of about
//! twice its length's own bit length. A wider magnitude gets an allocation of
//! its own, exactly as long as its limbs. Packing keeps the many narrow
//! entries free of per-entry allocations; the separate allocation keeps a
//! wide entry from making the shared stack reallocate, which would briefly
//! hold the stack's old and new storage together. Either way an entry costs
//! space proportional to its width, and a pop returns the entry without
//! copying a wide magnitude.

use crate::bits::stack::{BitStack, PackedU64Stack};

#[cfg(test)]
mod tests;

/// The widest magnitude, in bits, packed into the shared bit stack.
///
/// A wider magnitude is a value whose input already paid for over 256 bits
/// of height change, enough to cover its own allocation's fixed cost.
pub const PACKED_BITS: u64 = 256;

/// Limbs of a packed magnitude at most.
const PACKED_LIMBS: usize = 4;

/// Bits per stored piece of a packed magnitude.
const PIECE_BITS: u64 = 32;

/// Mask selecting one piece.
const PIECE_MASK: u64 = (1 << PIECE_BITS) - 1;

/// A popped magnitude, as little-endian limbs with no high zero limb.
pub enum Magnitude {
    /// A magnitude of at most [`PACKED_BITS`] bits, unpacked onto the stack.
    Packed {
        /// The limbs, of which the first `len` are live.
        limbs: [u64; PACKED_LIMBS],
        /// Live limbs.
        len: usize,
    },
    /// A wider magnitude, in the allocation it was pushed into.
    Separate(Box<[u64]>),
}

impl Magnitude {
    /// The magnitude's limbs.
    pub fn limbs(&self) -> &[u64] {
        match self {
            Magnitude::Packed { limbs, len } => &limbs[..*len],
            Magnitude::Separate(limbs) => limbs,
        }
    }
}

/// Signed integers, newest on top.
pub struct IntegerStack {
    /// One header per entry: `bit length << 1 | negative`.
    headers: PackedU64Stack,
    /// Packed magnitude bits, most significant piece pushed first, so that a
    /// pop reads the least significant piece first.
    bits: BitStack,
    /// Magnitudes wider than [`PACKED_BITS`], one allocation each.
    separate: Vec<Box<[u64]>>,
}

impl IntegerStack {
    /// Construct an empty stack.
    pub fn new() -> Self {
        Self {
            headers: PackedU64Stack::new(),
            bits: BitStack::new(),
            separate: Vec::new(),
        }
    }

    /// Push `±magnitude`, given as little-endian limbs with no high zero limb.
    pub fn push<L>(&mut self, negative: bool, magnitude: L)
    where
        L: ExactSizeIterator<Item = u64>,
    {
        let bit_len;
        if magnitude.len() > PACKED_LIMBS {
            let limbs: Box<[u64]> = magnitude.collect();
            bit_len = bit_length(&limbs);
            self.separate.push(limbs);
        } else {
            let mut limbs = [0u64; PACKED_LIMBS];
            let mut len = 0;
            for limb in magnitude {
                limbs[len] = limb;
                len += 1;
            }
            let limbs = &limbs[..len];
            bit_len = bit_length(limbs);
            let pieces = bit_len.div_ceil(PIECE_BITS);
            for (index, &limb) in limbs.iter().enumerate().rev() {
                for half in [1u64, 0] {
                    let piece = 2 * index as u64 + half;
                    if piece < pieces {
                        let width = piece_width(piece, pieces, bit_len);
                        self.bits
                            .push_bits((limb >> (PIECE_BITS * half)) & PIECE_MASK, width);
                    }
                }
            }
        }
        // The magnitude is held in memory, so its bit length is far below 2^63
        // and the shift keeps every bit.
        self.headers.push(bit_len << 1 | u64::from(negative));
    }

    /// Pop the newest entry, returning whether it is negative and its
    /// magnitude.
    ///
    /// # Panics
    ///
    /// Panics if the stack is empty.
    pub fn pop(&mut self) -> (bool, Magnitude) {
        let header = self.headers.pop();
        let negative = header & 1 == 1;
        let bit_len = header >> 1;
        if bit_len > PACKED_BITS {
            let limbs = self
                .separate
                .pop()
                .expect("each wide header has a separate magnitude");
            return (negative, Magnitude::Separate(limbs));
        }
        let pieces = bit_len.div_ceil(PIECE_BITS);
        let mut limbs = [0u64; PACKED_LIMBS];
        for piece in 0..pieces {
            let value = self.bits.pop_bits(piece_width(piece, pieces, bit_len));
            limbs[(piece / 2) as usize] |= value << (PIECE_BITS * (piece % 2));
        }
        let len = pieces.div_ceil(2) as usize;
        (negative, Magnitude::Packed { limbs, len })
    }
}

/// The bit length of little-endian limbs with no high zero limb.
fn bit_length(limbs: &[u64]) -> u64 {
    debug_assert!(
        limbs.last() != Some(&0),
        "a magnitude has no high zero limb"
    );
    match limbs.last() {
        Some(&top) => 64 * (limbs.len() as u64 - 1) + u64::from(u64::BITS - top.leading_zeros()),
        None => 0,
    }
}

/// The width of piece `piece` of `pieces` in a magnitude of `bit_len` bits.
fn piece_width(piece: u64, pieces: u64, bit_len: u64) -> u32 {
    let width = if piece + 1 == pieces {
        bit_len - PIECE_BITS * piece
    } else {
        PIECE_BITS
    };
    u32::try_from(width).expect("a piece is at most 32 bits")
}
