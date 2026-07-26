use std::io;

use bitvec::domain::Domain;
use bitvec::prelude::*;

use crate::error::Decode;

/// The packed storage form: a most-significant-bit-first bit stream over bytes.
///
/// This is the at-rest form of a `Party`/`Version`: the canonical packed
/// preorder bit stream together with its exact live length, in one
/// container. The raw byte slice ([`BitVec::as_raw_slice`]) *is* the wire
/// encoding — the final partial byte is zero-padded (see
/// [`zero_dead_bits`]) — and the live length is a cached parse product the
/// wire legitimately omits, because the streams are self-delimiting at the
/// bit level.
pub type Bits = BitVec<u8, Msb0>;

/// A borrowed view of the packed storage form.
pub type BitsSlice = BitSlice<u8, Msb0>;

/// Borrow bytes as an MSB-first bit stream without first copying them into a
/// [`Bits`].
pub(crate) fn bytes_as_bits(bytes: &[u8]) -> &BitsSlice {
    bytes.view_bits::<Msb0>()
}

/// Zero the dead bits past the live length, making the packed bytes
/// ([`BitVec::as_raw_slice`]) canonical: byte-equal if and only if the bit
/// content is equal.
///
/// The tree builders write into a reused buffer, and a collapsing node (the
/// party `sum`/`diff` ops, via `IdBuilder::close_node`) `truncate`s it,
/// shrinking the live length while leaving the bits it shed in the final
/// partial byte. `as_raw_slice` exposes those stale bits, so two equal parties
/// built different ways would serialize to different bytes and a joined party
/// could fail to decode on the wire. Calling this before a [`Bits`] becomes a
/// stored `Party`/`Version` restores the canonical-storage invariant that
/// `as_bytes`, [`Hash`](core::hash::Hash), and the borsh wire form rest on.
pub(crate) fn zero_dead_bits(bits: &mut Bits) {
    bits.set_uninitialized(false);
}

/// Byte-level equality of two canonical stored streams: equal live
/// lengths and equal raw bytes.
///
/// Rests on the canonical-raw-slice invariant — [`zero_dead_bits`] at
/// every storage seam — under which raw-byte equality plus live-length
/// equality is exactly bit equality, decided by one `memcmp` instead of
/// a bit-domain-chunked compare (measured 2–54x faster on equal pairs
/// from 23 bits to 32 Kbits, and 5x on a hash-map workload, in the
/// 2026-07 storage-migration probe). The length check is load-bearing:
/// two streams of different live length can share raw bytes (`01` vs
/// `010` are both the byte `0x40`).
pub(crate) fn canonical_eq(a: &Bits, b: &Bits) -> bool {
    debug_assert!(
        dead_bits_are_zero(a) && dead_bits_are_zero(b),
        "canonical_eq compares raw bytes: both operands' dead bits must be zero",
    );
    a.len() == b.len() && a.as_raw_slice() == b.as_raw_slice()
}

/// Byte-level hash of a canonical stored stream: the raw bytes, then the
/// live length.
///
/// [`canonical_eq`]'s hash counterpart — it feeds the hasher exactly the
/// pair that equality compares, so equal values hash equally by
/// construction. Rests on the same canonical-raw-slice invariant, and is
/// an order of magnitude cheaper than hashing bit by bit (same probe as
/// [`canonical_eq`]'s).
pub(crate) fn canonical_hash<H: core::hash::Hasher>(bits: &Bits, state: &mut H) {
    use core::hash::Hash;
    debug_assert!(
        dead_bits_are_zero(bits),
        "canonical_hash reads raw bytes: dead bits must be zero",
    );
    bits.as_raw_slice().hash(state);
    bits.len().hash(state);
}

/// Whether a stored stream's dead bits are zero: the canonical-storage
/// check behind the `as_bytes` debug asserts.
///
/// Only the final partial byte of [`BitVec::as_raw_slice`] can hold dead
/// bits (the slice covers exactly the live bits' bytes), so this is one
/// mask test — `O(1)`, cheap enough to assert on every raw-byte read.
pub(crate) fn dead_bits_are_zero(bits: &Bits) -> bool {
    let live_in_last = bits.len() % 8;
    live_in_last == 0
        || bits
            .as_raw_slice()
            .last()
            .is_none_or(|last| last & (0xFF >> live_in_last) == 0)
}

/// Streams a bit-concatenation of canonical bit slices to a writer, packing
/// MSB-first into bytes and zero-padding the final partial byte — with no
/// intermediate buffer.
///
/// `Clock::encode_to` writes the id stream then the event stream through one of
/// these, so the cross-stream byte (the partial id tail merged with the leading
/// event bits) is produced on the fly rather than via a combined `BitVec`;
/// single-stream `Party`/`Version` go through [`pack_to_writer`].
pub(crate) struct BitWriter<'w, W: io::Write> {
    w: &'w mut W,
    /// The byte under construction: `filled` valid bits in its high positions
    /// (MSB first), the low `8 - filled` bits zero.
    cur: u8,
    /// Number of valid high bits in `cur` (`0..8`).
    filled: u32,
}

impl<'w, W: io::Write> BitWriter<'w, W> {
    pub(crate) fn new(w: &'w mut W) -> Self {
        BitWriter {
            w,
            cur: 0,
            filled: 0,
        }
    }

    /// Append `k` bits (`1..=8`) taken from the high `k` positions of `src` (its
    /// low `8 - k` bits must be zero), MSB-first.
    fn push(&mut self, src: u8, k: u32) -> io::Result<()> {
        debug_assert!(
            (1..=8).contains(&k) && u32::from(src).trailing_zeros() >= 8 - k,
            "push expects {k} live bits in the high positions of {src:#010b}",
        );
        if self.filled + k < 8 {
            self.cur |= src >> self.filled;
            self.filled += k;
            Ok(())
        } else {
            let out = self.cur | (src >> self.filled);
            self.w.write_all(&[out])?;
            // The bits of `src` that did not fit become the next partial byte,
            // shifted to the high positions; the `u16` cast keeps `<< 8` (when
            // `filled == 0`, i.e. a whole byte) from overflowing and clears `cur`.
            self.cur = ((u16::from(src) << (8 - self.filled)) & 0xFF) as u8;
            self.filled = self.filled + k - 8;
            Ok(())
        }
    }

    /// Append a canonical bit slice (MSB-first).
    ///
    /// The slice starts on a byte
    /// boundary in its own backing store (every stored `Party`/`Version` does):
    /// when the writer is itself byte-aligned the whole-byte body is emitted in
    /// one `write_all`; otherwise it is merged byte-by-byte across the boundary.
    pub(crate) fn write(&mut self, bits: &BitsSlice) -> io::Result<()> {
        if bits.is_empty() {
            return Ok(());
        }
        match bits.domain() {
            Domain::Enclave(elem) if elem.head().into_inner() == 0 => {
                self.push(elem.load_value(), bits.len() as u32)
            }
            Domain::Region {
                head: None,
                body,
                tail,
            } => {
                if self.filled == 0 {
                    self.w.write_all(body)?;
                } else {
                    for &b in body {
                        self.push(b, 8)?;
                    }
                }
                if let Some(elem) = tail {
                    self.push(elem.load_value(), (bits.len() % 8) as u32)?;
                }
                Ok(())
            }
            _ => {
                // A source that does not start on a byte boundary — not produced
                // by the stored forms; per-bit fallback keeps the writer correct
                // for any slice without an intermediate buffer.
                for bit in bits.iter().by_vals() {
                    self.push(if bit { 0x80 } else { 0 }, 1)?;
                }
                Ok(())
            }
        }
    }

    /// Flush the final partial byte (zero-padded) if any bits are pending.
    pub(crate) fn finish(self) -> io::Result<()> {
        if self.filled > 0 {
            self.w.write_all(&[self.cur])?;
        }
        Ok(())
    }
}

/// Pack a single canonical bit stream into bytes written to `w`, zero-padding
/// the final partial byte. The single-stream entry to [`BitWriter`] used by
/// `Party`/`Version`'s `encode_to`.
pub(crate) fn pack_to_writer<W: io::Write>(bits: &BitsSlice, w: &mut W) -> io::Result<()> {
    let mut writer = BitWriter::new(w);
    writer.write(bits)?;
    writer.finish()
}

/// Require that the bits from `pos` onward are exactly the canonical padding: a
/// run of zeros shorter than a byte.
///
/// [`pack_to_writer`] only pads the final
/// partial byte, so a canonical stream has at most 7 trailing zero bits; both a
/// nonzero padding bit AND a whole spurious zero byte (`>= 8` trailing bits,
/// even if all zero) are non-canonical. Bounding the length is what makes
/// `decode` injective on bytes — without it, `decode([.., 0x00])` would accept
/// the same value under infinitely many byte strings, re-encoding to a shorter
/// stream than its own input.
pub(crate) fn require_zero_padding(bits: &BitsSlice, pos: usize) -> Result<(), Decode> {
    if bits.len() - pos >= 8 || bits[pos..].any() {
        Err(Decode::TrailingBits)
    } else {
        Ok(())
    }
}
