use bitvec::domain::Domain;
use bitvec::prelude::*;

use crate::error::Decode;

/// The packed storage form: a most-significant-bit-first bit stream over bytes.
///
/// This is the at-rest form of a `Party`/`Version`: the canonical packed
/// preorder bit stream together with its exact live length, in one
/// container. The raw byte slice ([`BitVec::as_raw_slice`]) *is* the wire
/// encoding — the final partial byte's dead bits are zeroed at every
/// storage seam (`zero_dead_bits`) — and the live length is a cached parse
/// product the wire legitimately omits, because the streams are
/// self-delimiting at the bit level.
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

/// The direct byte view of a bit slice that starts on a byte boundary of
/// its backing store: the whole body bytes plus the masked partial tail
/// byte, if any.
///
/// `None` for the one shape with no direct byte view — a slice whose
/// backing-store offset puts live bits behind a partial head element.
/// Every stored stream starts on a byte boundary (offsets travel as bit
/// positions, never as re-sliced heads), so the `None` arm is a caller
/// policy decision, not a reachable production state: the gamma window
/// loader degrades to its bit-addressed fallback, the dsi cursor treats
/// it as a violated precondition. The destructuring lives here once so
/// the two callers cannot drift on the byte-alignment invariant while
/// keeping their deliberately different failure policies.
pub(crate) fn byte_view(bits: &BitsSlice) -> Option<(&[u8], Option<u8>)> {
    match bits.domain() {
        Domain::Region {
            head: None,
            body,
            tail,
        } => Some((body, tail.map(|elem| elem.load_value()))),
        Domain::Enclave(elem) if elem.head().into_inner() == 0 => {
            Some((&[], Some(elem.load_value())))
        }
        Domain::Region { head: Some(_), .. } | Domain::Enclave(_) => None,
    }
}

/// Require that the bits from `pos` onward are exactly the canonical padding: a
/// run of zeros shorter than a byte.
///
/// A canonical encoding zero-pads only the final partial byte (the stored
/// form's raw slice covers exactly the live bits' bytes, dead bits zeroed),
/// so it has at most 7 trailing zero bits; both a nonzero padding bit AND a
/// whole spurious zero byte (`>= 8` trailing bits, even if all zero) are
/// non-canonical. Bounding the length is what makes `decode` injective on
/// bytes — without it, `decode([.., 0x00])` would accept the same value
/// under infinitely many byte strings, re-encoding to a shorter stream than
/// its own input.
pub(crate) fn require_zero_padding(bits: &BitsSlice, pos: usize) -> Result<(), Decode> {
    if bits.len() - pos >= 8 || bits[pos..].any() {
        Err(Decode::TrailingBits)
    } else {
        Ok(())
    }
}
