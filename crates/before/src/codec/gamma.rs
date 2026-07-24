//! Elias-gamma encoding and decoding of the integers in a
//! [`Version`](crate::Version).
//!
//! The normal form of a [`Version`](crate::Version) guarantees that at least
//! half of the integers in its event tree are zero, and normalization pushes
//! magnitude toward the root, so most stored integers are small even after
//! many events. Elias-gamma encodes a zero in one bit and other integers in
//! bits proportional to the log of their magnitude, so the encoding is close
//! to minimal for this distribution.
//!
//! The trade-off is decode cost on every operation that examines a version.
//! That cost buys a heap-size reduction of one to two orders of magnitude,
//! and [`Batch`](crate::version::Batch) amortizes the decoding across a run
//! of operations. Both directions keep that cost word-scale: the stream is
//! byte-backed, so a whole code is decoded from one 64-bit window
//! ([`decode_int_window`]) and emitted with one store, with per-bit loops as
//! the fallback — and, on decode, the sole arbiter of every reject.

use bitvec::domain::Domain;
use bitvec::field::BitField;
use dashu_int::UBig;

use crate::error::Decode;

use super::{Base, BitCursor, Bits, BitsSlice, SliceCursor};

/// Append `n` as the Elias gamma code of `m = n + 1`: `floor(log2(m))` zero
/// bits, then `m` in `floor(log2(m)) + 1` bits, most-significant first.
///
/// Cost is `2*floor(log2(n+1)) + 1` bits; `0` costs a single bit. Canonical and
/// prefix-free, for an arbitrary-width non-negative `n` (there is no value
/// cap).
pub(crate) fn encode_int(out: &mut Bits, n: &Base) {
    let m = n + 1u32;
    match m.to_u64() {
        // Word case: the mantissa fits a machine word, so append the whole
        // code word-wise — the `2k+1` bits (zeros and all) in one `resize`,
        // then the `k+1` mantissa bits in one `store_be` — instead of one
        // `push` per bit. Byte-identical to the per-bit emit below.
        Some(m) => {
            // m >= 1, so `leading_zeros < 64` and `k = floor(log2(m))` never
            // underflows.
            let k = (u64::BITS - 1 - m.leading_zeros()) as usize;
            let start = out.len();
            out.resize(start + 2 * k + 1, false);
            out[start + k..].store_be::<u64>(m);
        }
        // Wide case (`n >= u64::MAX`): per-bit emit of the wide mantissa.
        None => {
            // m >= 1, so `m.bits() >= 1` and computing `k = floor(log2(m)) =
            // bit_length(m) - 1` never underflows. `k` is a bit count and fits
            // a `u64` even when `m` itself does not.
            let k = m.bits() - 1;
            for _ in 0..k {
                out.push(false);
            }
            // Emit `m` in `k + 1` bits, most-significant first.
            for i in (0..=k).rev() {
                out.push(m.bit(i));
            }
        }
    }
}

/// Read an Elias-gamma-coded integer at `pos`, returning the value and the new
/// position.
///
/// Running past the end is `Truncated`. Decodes an arbitrary-width value (no
/// cap): the unary prefix length `k` is bounded by the available bits, which
/// the `Truncated` checks enforce, so a declared code can never exceed the
/// input.
///
/// Reads word-wise when [`decode_int_window`] can prove the whole code from
/// one window; every other input — including every reject — is decided by the
/// per-bit loop ([`decode_int_from`]), so the two paths accept and reject
/// identically by construction (the routing lives in
/// [`SliceCursor::read_int`](BitCursor::read_int)).
pub(crate) fn decode_int(bits: &BitsSlice, pos: usize) -> Result<(Base, usize), Decode> {
    let mut cursor = SliceCursor::new(bits, pos);
    let base = cursor.read_int()?;
    Ok((base, cursor.position()))
}

/// The number of bits a [`decode_int_window`] window holds.
const WINDOW_BITS: usize = u64::BITS as usize;

/// One-window fast path of the gamma decoder: the value and end position of
/// the code at `pos`, when a single 64-bit window proves the whole code.
///
/// Loads a [`WINDOW_BITS`]-bit big-endian window at `pos`, takes one
/// `leading_zeros` for the whole unary prefix `k`, and shifts the mantissa out
/// of the same window — `O(1)` words per integer instead of ~10 ops per bit.
///
/// Returns `None` — decode nothing, let the caller run the per-bit loop from
/// `pos` instead — whenever the window cannot *prove* a complete code:
///
/// - the slice does not start on a byte boundary of its backing store (no
///   cheap byte view; stored forms always do);
/// - `pos` lies past the end of the stream (the bit loop reports `Truncated`);
/// - the `2k+1`-bit code overruns the window's proven bits, either because the
///   stream ends first (the bit loop reports `Truncated`) or because the code
///   is wider than the window (the bit loop decodes it: its machine-word path
///   reads every `k ≤ 63` mantissa — the `k + 1`-bit mantissa is the value
///   itself and fits `u64` — and only wider codes take the wide fallback).
///
/// The conditions are conservative, never guesses: `Some` is returned only
/// when every bit of the code lies within the window *and* within the stream,
/// so the fallback loop remains the sole arbiter of every reject. Bits between
/// the end of the stream and the end of the window read as zero (the tail
/// byte's dead bits are masked, missing bytes are zero-filled), which only
/// ever *lengthens* the apparent prefix — pushing `2k+1` past the proven
/// bits and into the fallback — never shortens it into a bogus accept.
pub(crate) fn decode_int_window(bits: &BitsSlice, pos: usize) -> Option<(u64, usize)> {
    // Bits of real stream between `pos` and the window's end.
    let proven = bits.len().checked_sub(pos)?.min(WINDOW_BITS);
    let window = load_window(bits, pos)?;
    let k = window.leading_zeros() as usize;
    let code_len = 2 * k + 1;
    if code_len > proven {
        return None;
    }
    // `code_len <= 64` bounds `k <= 31`, so the shift is in range and the
    // `k+1`-bit mantissa `m` (its leading 1 included) fits comfortably.
    let m = window >> (WINDOW_BITS - code_len);
    Some((m - 1, pos + code_len))
}

/// Load a 64-bit big-endian window of `bits` starting at bit `pos`: bit `pos`
/// of the stream in the most significant position, zero past the stream's end.
///
/// `None` when the slice does not begin on a byte boundary of its own backing
/// store, the one shape with no direct byte view. Every decode surface hands
/// in a whole stored stream (offsets travel as `pos`), so this fallback is
/// latent, kept for correctness rather than reached in practice.
fn load_window(bits: &BitsSlice, pos: usize) -> Option<u64> {
    let (body, tail): (&[u8], Option<u8>) = match bits.domain() {
        Domain::Region {
            head: None,
            body,
            tail,
        } => (body, tail.map(|elem| elem.load_value())),
        Domain::Enclave(elem) if elem.head().into_inner() == 0 => (&[], Some(elem.load_value())),
        Domain::Region { head: Some(_), .. } | Domain::Enclave(_) => return None,
    };
    let byte = pos / 8;
    let shift = pos % 8;
    // Gather the (up to) 9 bytes covering bits `pos..pos + 64`: 8 whole bytes
    // plus the partial ninth that a mid-byte `pos` shifts in. Bytes past the
    // stream stay zero — `load_value` masks the tail byte's dead bits, and the
    // buffer zero-fills past the last byte — so phantom bits are always zero.
    let mut buf = [0u8; 9];
    let start = byte.min(body.len());
    let end = (byte + buf.len()).min(body.len());
    buf[..end - start].copy_from_slice(&body[start..end]);
    if let Some(t) = tail {
        // `pos <= bits.len()` (checked by the caller) puts `byte` at or before
        // the tail byte, so the index never underflows.
        let tail_at = body.len();
        if tail_at < byte + buf.len() {
            buf[tail_at - byte] = t;
        }
    }
    let word = u64::from_be_bytes(buf[..8].try_into().expect("buf holds 8 whole bytes"));
    Some(if shift == 0 {
        word
    } else {
        (word << shift) | (u64::from(buf[8]) >> (8 - shift))
    })
}

/// Read one Elias-gamma-coded integer from a sequential bit cursor.
pub(crate) fn decode_int_from<C: BitCursor>(cursor: &mut C) -> Result<Base, Decode>
where
    Decode: From<C::Error>,
{
    let mut k = 0usize;
    while !cursor.read_bit()? {
        // The match (rather than `ok_or`) keeps the error value — `Decode`
        // has drop glue — from being constructed and dropped on every
        // iteration of this per-bit loop; see `codec::cursor::Truncated`.
        k = match k.checked_add(1) {
            Some(k) => k,
            None => return Err(Decode::NotCanonical),
        };
    }

    // Common case: read small codes into a machine integer, then widen once.
    if k < u64::BITS as usize {
        let mut m = 1u64;
        for _ in 0..k {
            m <<= 1;
            if cursor.read_bit()? {
                m |= 1;
            }
        }
        return Ok(Base::from(m - 1));
    }

    // Wide fallback: the leading 1 has already been consumed, and it is the
    // mantissa's top bit, at position `k`; the next `k` stream bits are the
    // mantissa's remaining bits, most-significant first. Setting the top bit
    // first sizes the value's storage once, and each later set writes one
    // limb in place, so the total limb work is linear in the code's bit
    // width and the only allocation is the value itself. A truncated stream
    // still fails at the same `read_bit` position it would reading into an
    // accumulator, so the accept/reject boundary is unchanged.
    let mut m = UBig::ZERO;
    m.set_bit(k);
    for i in (0..k).rev() {
        if cursor.read_bit()? {
            m.set_bit(i);
        }
    }
    // One width-proportional record per wide value: sizing `m`'s storage and
    // decrementing it below each cost one pass over its limbs.
    #[cfg(feature = "limb-meter")]
    super::base::limb_meter::record_wide(&m);
    Ok(Base::from(m - 1u32))
}

/// Skip an Elias-gamma-coded integer at `pos`, returning the position just past
/// it without materializing the integer. Used by topology-only event scans.
///
/// The prefix length alone determines the distance, so the
/// [`decode_int_window`] fast path settles it with one `leading_zeros`; the
/// bit-counting loop below remains the arbiter of everything the window
/// cannot prove, rejects included.
pub(crate) fn skip_int(bits: &BitsSlice, pos: usize) -> Result<usize, Decode> {
    if let Some((_, end)) = decode_int_window(bits, pos) {
        return Ok(end);
    }
    let mut k = 0usize;
    loop {
        let idx = pos + k;
        if idx >= bits.len() {
            return Err(Decode::Truncated);
        }
        if bits[idx] {
            break;
        }
        k += 1;
    }
    let end = pos + (2 * k) + 1;
    if end > bits.len() {
        Err(Decode::Truncated)
    } else {
        // The whole code's width is the scan record: the skip is the
        // topology walks' stand-in for reading the code, so it must price
        // the same bits a read would.
        super::scan::record_bits(end - pos);
        Ok(end)
    }
}
