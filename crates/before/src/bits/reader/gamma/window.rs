//! The single-word fast path for Elias-gamma decoding.
//!
//! A window is used only when it contains the complete code. Wider or
//! truncated inputs fall back to the sequential reader, which remains the sole
//! authority for rejection.

use super::super::body_tail;

/// Bits held in one decoding window.
const WINDOW_BITS: u64 = u64::BITS as u64;

/// Decode one complete code from the word beginning at `position`.
pub(super) fn decode(bytes: &[u8], len: u64, position: u64) -> Option<(u64, u64)> {
    let (body, tail) = body_tail(bytes, len);
    let proven = len.checked_sub(position)?.min(WINDOW_BITS);
    let word = load(body, tail, position);
    let zeros = u64::from(word.leading_zeros());
    let code_len = 2 * zeros + 1;
    if code_len > proven {
        return None;
    }
    let mantissa = word >> (WINDOW_BITS - code_len);
    Some((mantissa - 1, position + code_len))
}

/// Load 64 big-endian bits at `position`, filling past the stream with zeros.
fn load(body: &[u8], tail: Option<u8>, position: u64) -> u64 {
    let byte = usize::try_from(position / 8).unwrap_or(usize::MAX);
    let shift = (position % 8) as usize;
    let mut bytes = [0u8; 9];

    let start = byte.min(body.len());
    let end = byte.saturating_add(bytes.len()).min(body.len());
    bytes[..end - start].copy_from_slice(&body[start..end]);

    if let Some(tail) = tail {
        let tail_at = body.len();
        if tail_at < byte.saturating_add(bytes.len()) {
            bytes[tail_at - byte] = tail;
        }
    }

    let word = u64::from_be_bytes(bytes[..8].try_into().expect("window has eight bytes"));
    if shift == 0 {
        word
    } else {
        (word << shift) | (u64::from(bytes[8]) >> (8 - shift))
    }
}
