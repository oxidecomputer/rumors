//! The append-truncate builder core shared by the packed preorder emitters.
//!
//! Every packed tree in the crate — id trees on 2-bit presence tags,
//! skyline event streams on 1-bit topology flags plus leaf payload codes —
//! is written by the same small move set over one output bit buffer:
//!
//! - **append**: push flag bits and payload codes at the end;
//! - **reserve/patch**: hold a fixed-width header slot open while the
//!   children are emitted, then write its final bits in place;
//! - **copy-splice**: extend the output with a verbatim bit range copied
//!   from an already-normal source stream;
//! - **truncate**: roll the output back to a recorded position, which is
//!   the whole of normalization on these streams — nothing an emitter
//!   writes is ever widened in place, so every repair is subtractive.
//!
//! This module is that move set, with the packed-stream write meter
//! ([`scan::record_bits`](crate::codec::scan)) applied uniformly at the
//! primitives so every builder's write work is counted once, in one place.
//! The per-node payload discipline lives in the wrappers: the id builder
//! (`party::ops`) patches presence tags and collapses uniform subtrees to
//! their tag; the skyline builder ([`crate::version::skyline`]) appends
//! leaf delta codes and collapses equal sibling leaves by truncation.

use super::{Bits, BitsSlice};

/// An append-truncate builder over one packed preorder bit stream.
///
/// The wrapper owning it defines the tree coding; this core owns the
/// buffer, the primitive moves, and the write metering.
pub(crate) struct PackedBuilder {
    bits: Bits,
}

impl PackedBuilder {
    /// Create a builder with room for `capacity` bits before reallocation.
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        PackedBuilder {
            bits: Bits::with_capacity(capacity),
        }
    }

    /// The current output length in bits: the position the next append
    /// lands at, and the coordinate [`truncate`](Self::truncate) rolls
    /// back to.
    // Reached only from the skyline builder, which compiles under `test`
    // and `meter`; still type-checked in every build.
    #[cfg_attr(not(any(test, feature = "meter")), allow(dead_code))]
    pub(crate) fn len(&self) -> usize {
        self.bits.len()
    }

    /// Append one bit.
    pub(crate) fn push_bit(&mut self, bit: bool) {
        super::scan::record_bits(1);
        self.bits.push(bit);
    }

    /// Copy the completed range `start..` back out of the output,
    /// recording the read.
    ///
    /// Collapse repairs re-anchor a surviving code before truncating the
    /// region it sits in; this is the read half of that repair.
    // Reached only from the skyline builder, which compiles under `test`
    // and `meter`; still type-checked in every build.
    #[cfg_attr(not(any(test, feature = "meter")), allow(dead_code))]
    pub(crate) fn extract(&self, start: usize) -> Bits {
        super::scan::record_bits(self.bits.len() - start);
        self.bits[start..].to_bitvec()
    }

    /// Append `width` zero bits as a header slot to be
    /// [`patch_bit`](Self::patch_bit)ed once the children are known,
    /// returning the slot's position.
    pub(crate) fn reserve(&mut self, width: usize) -> usize {
        super::scan::record_bits(width);
        let at = self.bits.len();
        for _ in 0..width {
            self.bits.push(false);
        }
        at
    }

    /// Write one bit of a reserved header slot in place.
    ///
    /// # Panics
    ///
    /// Panics if `at` is at or past the current output length.
    pub(crate) fn patch_bit(&mut self, at: usize, bit: bool) {
        super::scan::record_bits(1);
        self.bits.set(at, bit);
    }

    /// Append a verbatim bit range copied from an already-normal source.
    pub(crate) fn splice(&mut self, src: &BitsSlice) {
        super::scan::record_bits(src.len());
        self.bits.extend_from_bitslice(src);
    }

    /// Roll the output back to `len` bits, discarding everything after.
    ///
    /// # Panics
    ///
    /// Panics if `len` exceeds the current output length: truncation only
    /// ever shortens.
    pub(crate) fn truncate(&mut self, len: usize) {
        assert!(
            len <= self.bits.len(),
            "builder truncation target {len} exceeds the {} bits written",
            self.bits.len(),
        );
        self.bits.truncate(len);
    }

    /// Take the finished stream.
    pub(crate) fn finish(self) -> Bits {
        self.bits
    }
}
