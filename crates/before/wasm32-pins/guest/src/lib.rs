//! The 32-bit boundary-pin guest: `before`'s public byte decode doors driven
//! at the sizes where 32-bit position arithmetic has boundaries, compiled to
//! wasm32-unknown-unknown so every pin executes under a genuinely 32-bit
//! `usize` with a full 4 GiB address space.
//!
//! The host (the `wasm32-pins-harness` crate) instantiates this module under
//! wasmtime and calls one export per pinned case. The contract:
//!
//! - Every interesting input is synthesized *inside* the guest (hundreds of
//!   megabytes at the deepest pins), so nothing bulk crosses the host
//!   boundary and the host-side test stays a one-call driver.
//! - An export returns a nonnegative observation on success — the decoded
//!   value's own bit length where one exists, `0` for a plain pass — and a
//!   negative code naming the first failed observation.
//! - A panic anywhere in `before` or its dependencies aborts the guest,
//!   which wasmtime surfaces as a trap: a first-class outcome the harness
//!   pins directly, so trap-versus-value is the red/green axis whenever a
//!   boundary misbehaves.
//!
//! The workspace builds this guest with `overflow-checks = true`: the 32-bit
//! failure class under audit includes silent release-mode wraps, and the
//! checks turn exactly those wraps into observable traps instead of wrong
//! values downstream code would have to detect after the fact.

use before::{Rank, Version};

/// The canonical encoding of a valid single-leaf `Version` padded to exactly
/// `n` bytes: one leaf flag, then the Elias-gamma code of the leaf height
/// `2^k - 1` with `k = 4n - 5`, then the marker byte.
///
/// The layout, bit by bit (positions are MSB-first over the buffer):
///
/// - bit `0`: `1`, the leaf flag ending an empty topology run;
/// - bits `1 ..= k`: the gamma code's `k`-zero prefix;
/// - bit `k + 1`: the mantissa's leading `1` (the value is `m = 2^k`);
/// - bits `k + 2 ..= 2k + 1`: the mantissa's remaining `k` zeros;
/// - bit `2k + 2 = 8n - 8`: the padding marker, alone in the final byte.
///
/// The stream is canonical (a lone leaf has no sibling to collapse with, and
/// its absolute height is a natural), so `Version::decode` must accept it at
/// any `n` the target's memory admits, and the decoded value's
/// `encoded_bits` is exactly `8n - 8`.
fn synth_version(n: usize) -> Vec<u8> {
    assert!(
        n >= 18,
        "the single-wide-leaf layout needs k = 4n - 5 >= 64"
    );
    let k = 4 * n - 5;
    let mut bytes = vec![0u8; n];
    bytes[0] |= 0x80; // the leaf flag
    bytes[(k + 1) / 8] |= 0x80 >> ((k + 1) % 8); // the mantissa's leading 1
    bytes[n - 1] |= 0x80; // the padding marker
    bytes
}

/// Sanity and liveness: a small synthesized version decodes to the expected
/// bit length, its bytes round-trip, and two mutilations reject with typed
/// errors (never a panic). Green at every commit; a red here means the
/// harness or the synthesis is broken, not that a boundary moved.
#[no_mangle]
pub extern "C" fn pin_version_small() -> i64 {
    let bytes = synth_version(64);
    let v = match Version::decode(&bytes[..]) {
        Ok(v) => v,
        Err(_) => return -1,
    };
    if v.encoded_bits() != 8 * 64 - 8 {
        return -2;
    }
    if v.as_bytes() != &bytes[..] {
        return -3;
    }
    // A truncated input must reject as a typed error.
    if Version::decode(&bytes[..63]).is_ok() {
        return -4;
    }
    // A zeroed final byte has no padding marker: typed reject.
    let mut unmarked = bytes;
    unmarked[63] = 0;
    if Version::decode(&unmarked[..]).is_ok() {
        return -5;
    }
    0
}

/// Decode a valid `n`-byte synthesized version and return its
/// `encoded_bits` (always `8n - 8` for the synthesized layout), checking
/// that the stored bytes round-trip the input exactly.
///
/// The harness aims this at the 32-bit boundaries: the last size below
/// `bitvec`'s borrowed-view cap, the first size at it, and the 512 MiB
/// bit-length boundary of `usize` position arithmetic.
#[no_mangle]
pub extern "C" fn pin_version_decode(n_bytes: u64) -> i64 {
    let n = match usize::try_from(n_bytes) {
        Ok(n) => n,
        Err(_) => return -100,
    };
    let bytes = synth_version(n);
    let v = match Version::decode(&bytes[..]) {
        Ok(v) => v,
        Err(_) => return -1,
    };
    if v.as_bytes() != &bytes[..] {
        return -2;
    }
    i64::try_from(v.encoded_bits()).unwrap_or(-3)
}

/// The canonical encoding of the valid rank `(2^(exp - 65) + 1) / 2^exp`:
/// integral part zero, a fraction exactly `exp` expansion bits deep whose
/// set bits are expansion positions `65` and `exp`.
///
/// `exp` must be a positive multiple of 8 (so the final fraction group is
/// flush and the pad is zero) and at least 128. The layout: one `0` header
/// bit (the inverted-delta code of integral `0`), then `exp / 8` fraction
/// groups of nine bits (a `1` continuation, then eight expansion bits),
/// then the closing `0` bit and zero padding to the byte boundary.
///
/// Every byte of this stream is input the decoder must actually read — the
/// fraction's depth is deliberately in-band (counted from bits read, never
/// from a header's claim), so ~`9/64` bytes per expansion bit is the
/// smallest honest trigger for any `exp`-boundary behavior: no crafted
/// stream can reach the exponent seam without materializing this length.
fn synth_rank(exp: u64) -> Vec<u8> {
    assert!(
        exp >= 128 && exp.is_multiple_of(8),
        "the layout wants flush groups"
    );
    let groups = exp / 8;
    let total_bits = 9 * groups + 2; // header + groups + close bit
    let total_bytes = usize::try_from(total_bits.div_ceil(8)).expect("the stream is addressable");

    // The group region is a 72-bit-periodic stream (eight 9-bit groups per
    // period) offset one bit by the header: emit it bytewise as the tile
    // shifted right one bit with carry, then clear everything at and past
    // the close bit and patch the two set expansion bits.
    const TILE: [u8; 9] = [0x80, 0x40, 0x20, 0x10, 0x08, 0x04, 0x02, 0x01, 0x00];
    let mut bytes = vec![0u8; total_bytes];
    for (b, byte) in bytes.iter_mut().enumerate() {
        let hi = if b == 0 { 0 } else { TILE[(b - 1) % 9] };
        *byte = (hi << 7) | (TILE[b % 9] >> 1);
    }
    // Clear the close bit and the padding after it (the tile pattern would
    // keep opening groups past the fraction's end).
    let close = 1 + 9 * groups;
    let close_byte = usize::try_from(close / 8).expect("the stream is addressable");
    let offset = (close % 8) as u32;
    bytes[close_byte] = if offset == 0 {
        0
    } else {
        bytes[close_byte] & (0xFFu8 << (8 - offset))
    };
    for byte in bytes.iter_mut().skip(close_byte + 1) {
        *byte = 0;
    }
    // Set expansion bits 65 and `exp` (1-based from the binary point):
    // expansion bit `e` lives at stream position `1 + 9·((e-1)/8) + 1 +
    // ((e-1)%8)` — its group's continuation bit, then its offset in the
    // group.
    for e in [65, exp] {
        let p = 1 + 9 * ((e - 1) / 8) + 1 + ((e - 1) % 8);
        let byte = usize::try_from(p / 8).expect("the stream is addressable");
        bytes[byte] |= 0x80 >> (p % 8);
    }
    bytes
}

/// Decode a valid synthesized rank whose fraction is exactly `exp`
/// expansion bits deep, then check three exact-order observations against
/// reference ranks: the value sits strictly between zero and one, and
/// equals its own clone.
///
/// The harness aims this at the `u64 -> usize` exponent seam: `exp` just
/// below `2^32` (in `usize` range on wasm32) and at `2^32` (past it). The
/// input is ~`9/64 · exp` bytes — about 604 MB at the seam — which is the
/// smallest honest trigger (`synth_rank` documents why no smaller stream
/// can reach it).
#[no_mangle]
pub extern "C" fn pin_rank_decode(exp: u64) -> i64 {
    let bytes = synth_rank(exp);
    let r = match Rank::decode(&bytes[..]) {
        Ok(r) => r,
        Err(_) => return -1,
    };
    drop(bytes);
    if r <= Rank::ZERO {
        return -2;
    }
    let one = match Version::try_from(1) {
        Ok(v) => v.rank(),
        Err(_) => return -3,
    };
    if r >= one {
        return -4;
    }
    if r != r.clone() {
        return -5;
    }
    0
}
