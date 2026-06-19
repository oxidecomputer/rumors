//! The on-disk framing for a persisted identity record.
//!
//! A [`Bookmark`](super::Bookmark) lends raw byte storage; *this* module owns
//! what those bytes are. A stored record is a single self-describing,
//! self-checking frame:
//!
//! ```text
//! [ magic   : 14 bytes = b"RUMORSBOOKMARK"
//! | version :  2 bytes  (big-endian u16, BOOKMARK_FORMAT_VERSION)
//! | hash    : 32 bytes  BLAKE3(magic ‖ version ‖ payload)
//! | payload :  N bytes  borsh(BTreeMap<Network, Vec<Clock>>) ]
//! ```
//!
//! The magic and version tag reject a foreign or future file *loudly* — a
//! non-bookmark or a format this build does not understand is an error, never a
//! misparse. The hash covers the whole frame body, so a truncated or bit-rotted
//! file is caught before its bytes are ever borsh-decoded into a [`Clock`] — the
//! silent-divergence failure mode this crate exists to prevent.
//!
//! The hash is a plain [`blake3`] digest, deliberately *not* the tree's
//! content-addressing hash: that type's contract is identity (a leaf's path), a
//! different concern from this one's local, non-adversarial corruption check.
//!
//! The framing ([`frame`]/[`unframe`]) is kept separate from the record codec
//! ([`encode`]/[`decode`]) so the byte framing can be property-tested over
//! arbitrary payloads, independent of the `!Clone` [`Clock`]s a real record
//! holds.

use std::collections::BTreeMap;

use before::Clock;

use crate::Network;

/// Magic bytes that open every persisted bookmark frame.
///
/// Distinct from the gossip wire's [`PROTOCOL_MAGIC`](crate::PROTOCOL_MAGIC):
/// the on-disk format and the wire protocol version independently, so bumping
/// one never forces the other.
pub const BOOKMARK_MAGIC: [u8; 14] = *b"RUMORSBOOKMARK";

/// On-disk bookmark format version, following [`BOOKMARK_MAGIC`].
///
/// Bumped whenever the frame layout or payload encoding changes. A file
/// carrying any other version is rejected with
/// [`FormatError::VersionMismatch`] rather than misread.
pub const BOOKMARK_FORMAT_VERSION: u16 = 1;

/// Byte offset of the version field within a frame.
const VERSION_OFFSET: usize = BOOKMARK_MAGIC.len();
/// Byte offset of the integrity hash within a frame.
const HASH_OFFSET: usize = VERSION_OFFSET + 2;
/// Width of the BLAKE3 integrity hash, in bytes.
const HASH_LEN: usize = 32;
/// Byte offset of the borsh payload within a frame: the end of the fixed header.
const PAYLOAD_OFFSET: usize = HASH_OFFSET + HASH_LEN;
/// Total fixed-header width: magic, version, and hash, before the payload.
const HEADER_LEN: usize = PAYLOAD_OFFSET;

/// Why a stored bookmark could not be turned back into a record.
///
/// Every variant but [`Read`](Self::Read) is a property of the *bytes* — a
/// foreign file, a format this build predates, or corruption — and means the
/// stored identity is unusable, not merely unavailable.
#[derive(Debug, thiserror::Error)]
pub enum FormatError {
    /// Fewer bytes than the fixed header: a truncated or empty file. (An
    /// *absent* bookmark is reported by [`load`](super::Bookmark::load)
    /// returning `None`, never as an empty frame.)
    #[error("bookmark too short: {len} bytes (need at least {})", HEADER_LEN)]
    Truncated {
        /// How many bytes were actually present.
        len: usize,
    },

    /// The leading bytes are not [`BOOKMARK_MAGIC`]: this is not a bookmark.
    #[error("not a rumors bookmark: unexpected magic bytes")]
    BadMagic {
        /// The magic bytes actually found.
        found: [u8; BOOKMARK_MAGIC.len()],
    },

    /// A bookmark, but a format version this build does not understand.
    #[error(
        "unsupported bookmark format version {found} (this build writes {BOOKMARK_FORMAT_VERSION})"
    )]
    VersionMismatch {
        /// The format version the file declared.
        found: u16,
    },

    /// The integrity hash does not match the body: the file is corrupt.
    #[error("bookmark integrity hash mismatch: stored record is corrupt")]
    HashMismatch,

    /// The lent reader failed mid-stream, before a frame could be examined.
    #[error("reading the stored bookmark failed: {0}")]
    Read(#[source] std::io::Error),

    /// The frame was well-formed and intact, but its payload would not decode —
    /// a logic error, since a matching hash means the bytes are the ones that
    /// were written.
    #[error("decoding the bookmark payload failed: {0}")]
    Decode(#[source] std::io::Error),
}

/// Wrap `payload` in a bookmark frame: prepend the magic and version tag and a
/// BLAKE3 hash over `magic ‖ version ‖ payload`.
///
/// The inverse of [`unframe`].
pub(crate) fn frame(payload: &[u8]) -> Vec<u8> {
    let version = BOOKMARK_FORMAT_VERSION.to_be_bytes();

    let mut hasher = blake3::Hasher::new();
    hasher.update(&BOOKMARK_MAGIC);
    hasher.update(&version);
    hasher.update(payload);
    let hash = hasher.finalize();

    let mut out = Vec::with_capacity(HEADER_LEN + payload.len());
    out.extend_from_slice(&BOOKMARK_MAGIC);
    out.extend_from_slice(&version);
    out.extend_from_slice(hash.as_bytes());
    out.extend_from_slice(payload);
    out
}

/// Validate a bookmark frame and return its payload slice.
///
/// Checks, in order: length against the fixed header, magic, version, then the
/// integrity hash. The inverse of [`frame`]: `unframe(&frame(p)) == Ok(p)`.
///
/// # Errors
///
/// [`FormatError::Truncated`], [`BadMagic`](FormatError::BadMagic),
/// [`VersionMismatch`](FormatError::VersionMismatch), or
/// [`HashMismatch`](FormatError::HashMismatch) — each pinpointing how the bytes
/// failed to be a frame this build can trust.
pub(crate) fn unframe(bytes: &[u8]) -> Result<&[u8], FormatError> {
    if bytes.len() < HEADER_LEN {
        return Err(FormatError::Truncated { len: bytes.len() });
    }

    let magic: [u8; BOOKMARK_MAGIC.len()] = bytes[..VERSION_OFFSET]
        .try_into()
        .expect("magic is checked");
    if magic != BOOKMARK_MAGIC {
        return Err(FormatError::BadMagic { found: magic });
    }

    let version = u16::from_be_bytes(
        bytes[VERSION_OFFSET..HASH_OFFSET]
            .try_into()
            .expect("two version bytes"),
    );
    if version != BOOKMARK_FORMAT_VERSION {
        return Err(FormatError::VersionMismatch { found: version });
    }

    let stored_hash = &bytes[HASH_OFFSET..PAYLOAD_OFFSET];
    let payload = &bytes[PAYLOAD_OFFSET..];

    let mut hasher = blake3::Hasher::new();
    hasher.update(&BOOKMARK_MAGIC);
    hasher.update(&version.to_be_bytes());
    hasher.update(payload);
    if hasher.finalize().as_bytes().as_slice() != stored_hash {
        return Err(FormatError::HashMismatch);
    }

    Ok(payload)
}

/// Serialize a record into a complete bookmark frame.
///
/// Borsh-encodes the record, then [`frame`]s it. The inverse of [`decode`].
pub(crate) fn encode(record: &BTreeMap<Network, Vec<Clock>>) -> Vec<u8> {
    // Encoding to a `Vec` cannot fail: borsh only errors on a failing writer,
    // and a `Vec` never fails to extend.
    let payload = borsh::to_vec(record).expect("encoding a record to a Vec is infallible");
    frame(&payload)
}

/// Validate a bookmark frame and deserialize its record.
///
/// [`unframe`]s, then borsh-decodes the payload. The inverse of [`encode`].
///
/// # Errors
///
/// Any [`unframe`] error, or [`FormatError::Decode`] if a frame that passed its
/// integrity check nonetheless held an undecodable payload (a logic error, not
/// corruption).
pub(crate) fn decode(bytes: &[u8]) -> Result<BTreeMap<Network, Vec<Clock>>, FormatError> {
    let payload = unframe(bytes)?;
    borsh::from_slice(payload).map_err(FormatError::Decode)
}

#[cfg(test)]
mod tests;
