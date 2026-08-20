//! Canonical CBOR head primitives shared by the hand-written wire codecs.
//!
//! The V2 wire and the session surfaces around it (preamble, greeting,
//! party hand-off, stream labels, epilogue) spell every structure as
//! deterministic-encoding CBOR: shortest-form heads everywhere, definite
//! lengths only, one spelling per value. That contract is what keeps the
//! byte-pinning snapshot discipline meaningful — a value has exactly one
//! encoding, so a snapshot pins semantics, not an encoder's whim — and it
//! is enforced on ingress: [`read_head`] and [`read_head_async`] reject a
//! head that is indefinite, reserved, or wider than its value requires.
//!
//! This module owns only the *head* grammar (RFC 8949 §3: the initial
//! byte's major type and its argument). What follows a head — payload
//! bytes, nested items, tag content — belongs to the codec reading it;
//! each codec validates the majors and values it expects and prices its
//! own lengths. Writers here emit exactly what the readers accept, and
//! the round-trip property tests in this module hold the two together.

use tokio::io::{AsyncRead, AsyncReadExt};

/// Major type of an unsigned integer item.
pub(crate) const MAJOR_UINT: u8 = 0;

/// Major type of a definite-length byte string.
pub(crate) const MAJOR_BSTR: u8 = 2;

/// Major type of a definite-length text string.
pub(crate) const MAJOR_TEXT: u8 = 3;

/// Major type of a definite-length array.
pub(crate) const MAJOR_ARRAY: u8 = 4;

/// Major type of a definite-length map.
pub(crate) const MAJOR_MAP: u8 = 5;

/// Major type of a tag.
pub(crate) const MAJOR_TAG: u8 = 6;

/// Tag number for an embedded CBOR sequence in a byte string (RFC 9277).
pub(crate) const TAG_CBOR_SEQUENCE: u64 = 63;

/// Tag number for an embedded CBOR data item in a byte string (RFC 8949).
pub(crate) const TAG_EMBEDDED_ITEM: u64 = 24;

/// Tag number for self-described CBOR (RFC 8949 §3.4.6): CBOR's own
/// magic, opening the V2 preamble and the stored bookmark.
pub(crate) const TAG_SELF_DESCRIBED: u64 = 55799;

/// Bytes the shortest-form head for `value` occupies, any major type.
pub(crate) const fn head_len(value: u64) -> usize {
    match value {
        0..=23 => 1,
        24..=0xff => 2,
        0x100..=0xffff => 3,
        0x1_0000..=0xffff_ffff => 5,
        _ => 9,
    }
}

/// Append the shortest-form head `(major, value)` to `out`.
pub(crate) fn write_head(out: &mut Vec<u8>, major: u8, value: u64) {
    debug_assert!(major < 8, "a CBOR major type is three bits");
    let major = major << 5;
    match value {
        0..=23 => out.push(major | value as u8),
        24..=0xff => out.extend_from_slice(&[major | 24, value as u8]),
        0x100..=0xffff => {
            out.push(major | 25);
            out.extend_from_slice(&(value as u16).to_be_bytes());
        }
        0x1_0000..=0xffff_ffff => {
            out.push(major | 26);
            out.extend_from_slice(&(value as u32).to_be_bytes());
        }
        _ => {
            out.push(major | 27);
            out.extend_from_slice(&value.to_be_bytes());
        }
    }
}

/// Append the head of a tag item to `out`.
pub(crate) fn write_tag(out: &mut Vec<u8>, tag: u64) {
    write_head(out, MAJOR_TAG, tag);
}

/// One decoded head: the item's major type and its argument (a value,
/// length, count, or tag number, by major type).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Head {
    pub major: u8,
    pub value: u64,
}

/// A head violating the wire's deterministic-encoding contract, or no
/// head at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum HeadError {
    /// The input ended inside the head.
    #[error("input ends inside a CBOR head")]
    Truncated,
    /// An indefinite-length head; the wire is definite-length only.
    #[error("indefinite-length CBOR is not canonical")]
    Indefinite,
    /// A reserved additional-information value (28 through 30).
    #[error("CBOR head uses a reserved additional-information value")]
    Reserved,
    /// A wider argument encoding than the value requires.
    #[error("CBOR head is not in shortest form")]
    NotShortest,
}

/// Read one canonical head off the front of `input`, advancing past it.
pub(crate) fn read_head(input: &mut &[u8]) -> Result<Head, HeadError> {
    let (&initial, rest) = input.split_first().ok_or(HeadError::Truncated)?;
    let major = initial >> 5;
    let info = initial & 0x1f;
    let (value, rest) = match info {
        0..=23 => (u64::from(info), rest),
        24 => {
            let (&byte, rest) = rest.split_first().ok_or(HeadError::Truncated)?;
            (u64::from(byte), rest)
        }
        25 => {
            let (bytes, rest) = split_argument::<2>(rest)?;
            (u64::from(u16::from_be_bytes(bytes)), rest)
        }
        26 => {
            let (bytes, rest) = split_argument::<4>(rest)?;
            (u64::from(u32::from_be_bytes(bytes)), rest)
        }
        27 => {
            let (bytes, rest) = split_argument::<8>(rest)?;
            (u64::from_be_bytes(bytes), rest)
        }
        28..=30 => return Err(HeadError::Reserved),
        _ => return Err(HeadError::Indefinite),
    };
    let width = 1 + (input.len() - rest.len() - 1);
    if width != head_len(value) {
        return Err(HeadError::NotShortest);
    }
    *input = rest;
    Ok(Head { major, value })
}

/// Split a fixed-width head argument off `rest`.
fn split_argument<const N: usize>(rest: &[u8]) -> Result<([u8; N], &[u8]), HeadError> {
    if rest.len() < N {
        return Err(HeadError::Truncated);
    }
    let (bytes, rest) = rest.split_at(N);
    Ok((bytes.try_into().expect("split at the argument width"), rest))
}

/// How reading a head from a live transport failed.
#[derive(Debug, thiserror::Error)]
pub(crate) enum HeadReadError {
    /// The transport failed (end-of-stream inside the head included).
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// The head arrived whole but violates the deterministic contract.
    #[error(transparent)]
    Malformed(HeadError),
}

/// Read one canonical head from `read`.
///
/// A clean end-of-stream *before the first byte* returns `Ok(None)`; an
/// end-of-stream inside the head is an
/// [`UnexpectedEof`](std::io::ErrorKind::UnexpectedEof) I/O error. Not
/// cancel safe: a dropped future may have consumed part of the head.
pub(crate) async fn read_head_async<R: AsyncRead + Unpin + ?Sized>(
    read: &mut R,
) -> Result<Option<Head>, HeadReadError> {
    let mut initial = 0u8;
    match read.read(std::slice::from_mut(&mut initial)).await? {
        0 => return Ok(None),
        1 => {}
        _ => unreachable!("a one-byte read returns at most one byte"),
    }
    let extension = extension_len(initial)?;
    let mut bytes = [0u8; 9];
    bytes[0] = initial;
    read.read_exact(&mut bytes[1..1 + extension]).await?;
    let mut input = &bytes[..1 + extension];
    read_head(&mut input)
        .map(Some)
        .map_err(HeadReadError::Malformed)
}

/// Read one canonical head from a synchronous reader.
///
/// The blocking twin of [`read_head_async`], with the same clean-close
/// and error contract; the sync codec oracle reads through this so the
/// two ingress paths share one head grammar.
#[cfg(test)]
pub(crate) fn read_head_io<R: std::io::Read + ?Sized>(
    read: &mut R,
) -> Result<Option<Head>, HeadReadError> {
    let mut initial = 0u8;
    match read.read(std::slice::from_mut(&mut initial))? {
        0 => return Ok(None),
        1 => {}
        _ => unreachable!("a one-byte read returns at most one byte"),
    }
    let extension = extension_len(initial)?;
    let mut bytes = [0u8; 9];
    bytes[0] = initial;
    read.read_exact(&mut bytes[1..1 + extension])?;
    let mut input = &bytes[..1 + extension];
    read_head(&mut input)
        .map(Some)
        .map_err(HeadReadError::Malformed)
}

/// Bytes of head argument following an initial byte, before reading them.
fn extension_len(initial: u8) -> Result<usize, HeadReadError> {
    match initial & 0x1f {
        0..=23 => Ok(0),
        24 => Ok(1),
        25 => Ok(2),
        26 => Ok(4),
        27 => Ok(8),
        28..=30 => Err(HeadReadError::Malformed(HeadError::Reserved)),
        _ => Err(HeadReadError::Malformed(HeadError::Indefinite)),
    }
}

#[cfg(test)]
mod tests;
