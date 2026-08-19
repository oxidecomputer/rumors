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
//! primitives so every builder's write work is counted once, in one place. The
//! per-node payload discipline lives in the wrappers: the id builder
//! (`party::ops`) patches presence tags and collapses uniform subtrees to their
//! tag; the skyline builder ([`crate::version::skyline`]) appends leaf delta
//! codes and collapses equal sibling leaves by truncation.
//!
//! # Storage
//!
//! The buffer is the output's whole bytes plus one staging register of
//! fewer than eight trailing bits, so every append is shift arithmetic
//! against the register and lands in the byte vector at byte
//! granularity — bit-granular work costs word ops, not one buffer
//! operation per bit. A verbatim splice aligns its source to the
//! source's own byte boundary (at most seven leading bits through the
//! register) and then copies bytes — a `memcpy` when the register is
//! empty, a two-shift merge per byte otherwise.

use super::code::SMALL_CODE_BITS;
use super::{BitsBuf, BitsView, Code};

/// An append-truncate builder over one packed preorder bit stream.
///
/// The wrapper owning it defines the tree coding; this core owns the
/// buffer, the primitive moves, and the write metering.
///
/// Positions and lengths run at `u64` width on every target, the build
/// side's discipline (the `buf` module doc): the builder is exact to
/// allocatable memory, and byte *indexes* — which fit `usize` because they
/// index an allocated buffer — are converted exactly where a byte is
/// touched.
pub(crate) struct PackedBuilder {
    /// The committed prefix: whole bytes, most-significant bit first.
    bytes: Vec<u8>,
    /// The trailing not-yet-committed bits, value-packed at the low end
    /// (the stream's next bit is the register's most significant live
    /// bit). Always fewer than eight: appends flush whole bytes
    /// greedily.
    staged: u64,
    /// Live bits in `staged`, `0..8`.
    staged_len: u32,
}

impl PackedBuilder {
    /// Create a builder with room for `capacity` bits before reallocation.
    ///
    /// The capacity is a hint: a request past the target's address space
    /// allocates nothing up front, and the buffer still grows to whatever
    /// the appends actually demand.
    pub(crate) fn with_capacity(capacity: u64) -> Self {
        PackedBuilder {
            bytes: Vec::with_capacity(usize::try_from(capacity / 8 + 1).unwrap_or(0)),
            staged: 0,
            staged_len: 0,
        }
    }

    /// The current output length in bits: the position the next append
    /// lands at, and the coordinate [`truncate`](Self::truncate) rolls
    /// back to.
    pub(crate) fn len(&self) -> u64 {
        self.bytes.len() as u64 * 8 + u64::from(self.staged_len)
    }

    /// Append one bit.
    pub(crate) fn push_bit(&mut self, bit: bool) {
        super::scan::record_bits(1);
        self.append_bits(u64::from(bit), 1);
    }

    /// Copy the completed range `start..` back out of the output as a
    /// [`Code`], recording the read.
    ///
    /// Collapse repairs re-anchor a surviving code before truncating the
    /// region it sits in; this is the read half of that repair (the
    /// skyline builder's cascade, on the production join/meet path).
    pub(crate) fn extract_code(&self, start: u64) -> Code {
        let n = self.len() - start;
        super::scan::record_bits_u64(n);
        if n <= SMALL_CODE_BITS {
            return Code::Small {
                bits: self.read_bits(start, n as u32),
                len: n as u8,
            };
        }
        let mut out = BitsBuf::with_capacity(n);
        for i in start..start + n {
            out.push(self.bit_at(i));
        }
        Code::Wide(out)
    }

    /// Append one complete payload code.
    pub(crate) fn push_code(&mut self, code: &Code) {
        match code {
            Code::Small { bits, len } => {
                super::scan::record_bits(usize::from(*len));
                self.append_bits(*bits, u32::from(*len));
            }
            // The splice records its own write.
            Code::Wide(bits) => {
                let src = super::buf::built_view(bits);
                self.splice(src, 0, src.len());
            }
        }
    }

    /// Append `width` zero bits as a header slot to be
    /// [`patch_bit`](Self::patch_bit)ed once the children are known,
    /// returning the slot's position.
    pub(crate) fn reserve(&mut self, width: usize) -> u64 {
        super::scan::record_bits(width);
        let at = self.len();
        let mut remaining = width;
        while remaining > 0 {
            let chunk = remaining.min(32);
            self.append_bits(0, chunk as u32);
            remaining -= chunk;
        }
        at
    }

    /// Write one bit of a reserved header slot in place.
    ///
    /// # Panics
    ///
    /// Panics if `at` is at or past the current output length.
    pub(crate) fn patch_bit(&mut self, at: u64, bit: bool) {
        super::scan::record_bits(1);
        let committed = self.bytes.len() as u64 * 8;
        if at < committed {
            let mask = 1u8 << (7 - at % 8);
            if bit {
                self.bytes[(at / 8) as usize] |= mask;
            } else {
                self.bytes[(at / 8) as usize] &= !mask;
            }
        } else {
            let offset = (at - committed) as u32;
            assert!(
                offset < self.staged_len,
                "patch position {at} is past the output"
            );
            let mask = 1u64 << (self.staged_len - 1 - offset);
            if bit {
                self.staged |= mask;
            } else {
                self.staged &= !mask;
            }
        }
    }

    /// Append the bit range `start..end` of `src`, copied verbatim from an
    /// already-normal source.
    ///
    /// # Panics
    ///
    /// `start..end` must be a range within the view's live length.
    pub(crate) fn splice(&mut self, src: BitsView<'_>, start: u64, end: u64) {
        assert!(
            start <= end && end <= src.len(),
            "spliced range within the view's live length"
        );
        super::scan::record_bits_u64(end - start);
        let mut pos = start;
        // Walk the source up to its next byte boundary (at most seven
        // bits), where the byte copy takes over.
        while pos < end && !pos.is_multiple_of(8) {
            self.append_bits(u64::from(src.bit(pos)), 1);
            pos += 1;
        }
        let whole = (end - pos) / 8;
        if whole > 0 {
            let at = (pos / 8) as usize;
            self.append_bytes(&src.bytes()[at..at + whole as usize]);
            pos += whole * 8;
        }
        if pos < end {
            // Trailing bits (fewer than 8), value-packed from their byte.
            let rem = (end - pos) as u32;
            let byte = src.bytes()[(pos / 8) as usize];
            self.append_bits(u64::from(byte >> (8 - rem)), rem);
        }
    }

    /// Roll the output back to `len` bits, discarding everything after.
    ///
    /// # Panics
    ///
    /// Panics if `len` exceeds the current output length: truncation only
    /// ever shortens.
    pub(crate) fn truncate(&mut self, len: u64) {
        assert!(
            len <= self.len(),
            "builder truncation target {len} exceeds the {} bits written",
            self.len(),
        );
        let whole = len / 8;
        let rem = (len % 8) as u32;
        if whole < self.bytes.len() as u64 {
            self.staged = if rem > 0 {
                u64::from(self.bytes[whole as usize] >> (8 - rem))
            } else {
                0
            };
            self.staged_len = rem;
            self.bytes.truncate(whole as usize);
        } else {
            self.staged >>= self.staged_len - rem;
            self.staged_len = rem;
        }
    }

    /// Take the finished stream: the committed bytes adopted whole, the
    /// staging register flushed as the final (zero-padded) partial byte —
    /// no length-encoding conversion binds the hand-off on any target.
    pub(crate) fn finish(self) -> BitsBuf {
        let bit_len = self.len();
        let mut bytes = self.bytes;
        if self.staged_len > 0 {
            bytes.push((self.staged << (8 - self.staged_len)) as u8);
        }
        BitsBuf::from_raw_parts(bytes, bit_len)
    }

    /// Append `len <= 63` bits, value-packed at the low end of `value`
    /// (bits above `len` must be zero), flushing whole bytes into the
    /// committed prefix.
    fn append_bits(&mut self, value: u64, len: u32) {
        debug_assert!(len <= 63, "appends stage at most 63 bits at once");
        debug_assert!(
            value >> len == 0,
            "append value has bits above its stated width"
        );
        let total = self.staged_len + len;
        if total < 8 {
            self.staged = (self.staged << len) | value;
            self.staged_len = total;
            return;
        }
        // Up to 70 live bits: merge in a double-word register, commit the
        // whole bytes in one extend, keep the remainder staged.
        let acc = (u128::from(self.staged) << len) | u128::from(value);
        let rem = total % 8;
        let whole = (total / 8) as usize;
        let aligned = (acc << (128 - total)).to_be_bytes();
        self.bytes.extend_from_slice(&aligned[..whole]);
        self.staged = (acc as u64) & ((1u64 << rem) - 1);
        self.staged_len = rem;
    }

    /// Append whole bytes: a `memcpy` when the staging register is
    /// empty, a two-shift merge per byte otherwise.
    fn append_bytes(&mut self, body: &[u8]) {
        if self.staged_len == 0 {
            self.bytes.extend_from_slice(body);
            return;
        }
        let r = self.staged_len;
        let mut carry = self.staged as u8;
        for &b in body {
            self.bytes.push((carry << (8 - r)) | (b >> r));
            carry = b & ((1 << r) - 1);
        }
        self.staged = u64::from(carry);
    }

    /// Read `n <= 63` bits at `pos` back out of the output, value-packed
    /// at the low end of the result.
    fn read_bits(&self, pos: u64, n: u32) -> u64 {
        debug_assert!(u64::from(n) <= SMALL_CODE_BITS && pos + u64::from(n) <= self.len());
        let committed = self.bytes.len() as u64 * 8;
        let mut acc = 0u64;
        let mut got = 0u32;
        let mut p = pos;
        while got < n {
            if p < committed {
                let within = (p % 8) as u32;
                let take = (8 - within).min(n - got);
                let byte = self.bytes[(p / 8) as usize];
                let chunk = u64::from(byte >> (8 - within - take)) & ((1u64 << take) - 1);
                acc = (acc << take) | chunk;
                got += take;
                p += u64::from(take);
            } else {
                let offset = (p - committed) as u32;
                let take = n - got;
                let chunk =
                    (self.staged >> (self.staged_len - offset - take)) & ((1u64 << take) - 1);
                acc = (acc << take) | chunk;
                got += take;
                p += u64::from(take);
            }
        }
        acc
    }

    /// The bit at `pos`, read back out of the committed prefix or the
    /// staging register.
    fn bit_at(&self, pos: u64) -> bool {
        let committed = self.bytes.len() as u64 * 8;
        if pos < committed {
            self.bytes[(pos / 8) as usize] >> (7 - pos % 8) & 1 == 1
        } else {
            let offset = (pos - committed) as u32;
            debug_assert!(offset < self.staged_len, "read past the output");
            self.staged >> (self.staged_len - 1 - offset) & 1 == 1
        }
    }
}
