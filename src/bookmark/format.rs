//! The on-disk framing for a persisted identity record.
//!
//! A [`Bookmark`](super::Bookmark) lends raw byte storage; *this* module owns
//! what those bytes are. A stored record is a single self-describing,
//! self-checking frame, and the whole file parses as one CBOR item — a
//! generic CBOR tool with no rumors knowledge unfolds it completely:
//!
//! ```text
//! 55799([                                   ; self-described CBOR (RFC 8949)
//!     format_version : uint,                ; BOOKMARK_FORMAT_VERSION
//!     integrity      : bstr .size 32,       ; BLAKE3, coverage below
//!     payload        : 24(bstr .cbor map),  ; the record, embedded
//! ])
//! ```
//!
//! The opening self-described tag and the format version reject a foreign or
//! future file *loudly* — a non-bookmark or a format this build does not
//! understand is an error, never a misparse. The integrity hash covers the
//! encoded bytes of the format-version item and of the whole payload item
//! (tag 24 header included): every item of the frame array except the
//! integrity item itself. The frame's fixed opening (the self-described tag
//! and the array header) is outside the hash because any corruption there
//! already fails shape validation before the hash is consulted; a truncated
//! or bit-rotted file is caught before its bytes are ever decoded into a
//! [`Clock`] — the silent-divergence failure mode this crate exists to
//! prevent.
//!
//! The payload rides as an *embedded CBOR data item* (tag 24): the region the
//! hash covers is a CBOR-visible item, not an offset convention, and the
//! record inside is one CBOR map from each 16-byte network identifier to an
//! array of clocks, every clock a [`CLOCK_TAG`]-tagged byte string wrapping
//! its canonical encoding. The tags are written and read here, by the codec —
//! the atom types' serde implementations stay untagged and format-agnostic.
//!
//! The hash is a plain [`blake3`] digest, deliberately *not* the tree's
//! path-identity hash: that type's contract is identity (a leaf's path), a
//! different concern from this one's local, non-adversarial corruption check.
//!
//! The frame is deterministic-encoding CBOR: shortest-form headers
//! everywhere, one spelling per value. The frame's own heads admit only
//! their canonical spelling; the embedded payload is decoded by a general
//! CBOR reader, so its one-spelling property is the encoder's (equal
//! records produce equal files, and the byte-for-byte format pins stay
//! meaningful), not an ingress check.
//!
//! The framing ([`frame`]/[`unframe`]) is kept separate from the record codec
//! ([`encode`]/[`decode`]) so the byte framing can be property-tested over
//! arbitrary payloads, independent of the `!Clone` [`Clock`]s a real record
//! holds.

use std::collections::BTreeMap;

use before::Clock;
use ciborium::value::Value;

use crate::Network;
use crate::tags::CLOCK_TAG;
use crate::tree::mirror::cbor::{self, MAJOR_BSTR, MAJOR_UINT, SELF_DESCRIBED_HEAD};

/// On-disk bookmark format version, the first item of the frame array.
///
/// Version 4 is the fully CBOR-parseable frame: the self-described tag, this
/// version, the integrity hash, and the record as an embedded CBOR item
/// (clocks as [`CLOCK_TAG`]-tagged byte strings wrapping their canonical
/// codec, skyline version coding inside). A file carrying any other version
/// is rejected with [`FormatError::VersionMismatch`] rather than misread;
/// there is no migration path.
pub const BOOKMARK_FORMAT_VERSION: u64 = 4;

/// The frame array's header byte: a definite-length array of three items.
const FRAME_ARRAY: u8 = 0x83;

/// The integrity item's header: a 32-byte byte string.
const INTEGRITY_HEAD: [u8; 2] = [0x58, 0x20];

/// Width of the BLAKE3 integrity hash, in bytes.
const HASH_LEN: usize = 32;

/// The payload item's tag header: tag 24, "encoded CBOR data item".
const EMBEDDED_CBOR: [u8; 2] = [0xd8, 0x18];

/// Which part of the frame's fixed shape failed to parse.
///
/// Carried by [`FormatError::NotABookmark`]: the bytes are present but are not a
/// bookmark frame at the named position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum FrameDefect {
    /// The frame does not open with the CBOR self-described tag.
    ///
    /// The tag says "CBOR", not "rumors": what makes a file a *bookmark*
    /// is the frame shape and the format version behind it.
    #[error("no self-described CBOR tag")]
    SelfDescribedTag,

    /// The tagged item is not the three-item frame array.
    #[error("not the three-item frame array")]
    FrameArray,

    /// The format-version item is not a shortest-form unsigned integer.
    #[error("malformed format-version item")]
    FormatVersion,

    /// The integrity item is not a 32-byte byte string.
    #[error("malformed integrity item")]
    Integrity,

    /// The payload item does not carry the embedded-CBOR tag.
    #[error("payload is not an embedded CBOR item")]
    PayloadTag,

    /// The payload item's byte string header is malformed or non-canonical.
    #[error("malformed payload byte string")]
    PayloadByteString,

    /// Bytes continue past the end of the frame array.
    #[error("trailing bytes after the frame")]
    TrailingBytes,
}

/// Why an intact frame's payload is not the record this codec writes.
///
/// Carried by [`FormatError::Record`]. The frame passed its integrity check,
/// so every variant is a logic error — the bytes are the ones that were
/// written — never corruption.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RecordDefect {
    /// The payload is not parseable as a CBOR item at all.
    #[error("the payload does not parse as CBOR: {0}")]
    Cbor(#[source] ciborium::de::Error<std::io::Error>),

    /// Bytes continue past the payload's single CBOR item.
    #[error("{trailing} trailing bytes after the bookmark record")]
    TrailingBytes {
        /// How many bytes follow the record item.
        trailing: usize,
    },

    /// The record item is not a map.
    #[error("the bookmark record is not a map")]
    NotAMap,

    /// A record key is not a byte string.
    #[error("a record key is not a byte string")]
    KeyNotBytes,

    /// A record key byte string is not a 16-byte network identifier.
    #[error("a network identifier is exactly 16 bytes, found {len}")]
    KeyWidth {
        /// The width of the key actually found.
        len: usize,
    },

    /// A record entry's value is not an array of clocks.
    #[error("a record entry is not an array of clocks")]
    ClocksNotArray,

    /// A stored clock carries no CBOR tag.
    #[error("a stored clock is untagged")]
    ClockUntagged,

    /// A stored clock carries a tag other than the clock tag.
    #[error("a stored clock carries tag {found}, not the clock tag")]
    ClockTag {
        /// The tag number actually found.
        found: u64,
    },

    /// A stored clock's tagged item is not a byte string.
    #[error("a stored clock is not a byte string")]
    ClockNotBytes,

    /// A stored clock byte string fails the strict clock decoder.
    #[error("a stored clock would not decode")]
    Clock(#[source] before::error::Decode),
}

/// Why a stored bookmark could not be turned back into a record.
///
/// Every variant but [`Read`](Self::Read) is a property of the *bytes* — a
/// foreign file, a format this build predates, or corruption — and means the
/// stored identity is unusable, not merely unavailable.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FormatError {
    /// The bytes end inside the frame: a truncated or empty file. (An
    /// *absent* bookmark is reported by [`load`](super::Bookmark::load)
    /// returning `None`, never as an empty frame.)
    #[error("bookmark truncated: the {len} bytes present end inside the frame")]
    Truncated {
        /// How many bytes were actually present.
        len: usize,
    },

    /// The bytes are not a self-described CBOR bookmark frame: this is not a
    /// bookmark.
    #[error("not a rumors bookmark: {defect}")]
    NotABookmark {
        /// Which part of the frame shape failed.
        #[source]
        defect: FrameDefect,
    },

    /// A bookmark frame, but a format version this build does not understand.
    #[error(
        "unsupported bookmark format version {found} (this build writes {BOOKMARK_FORMAT_VERSION})"
    )]
    VersionMismatch {
        /// The format version the file declared.
        found: u64,
    },

    /// The integrity hash does not match the covered items: the file is
    /// corrupt.
    #[error("bookmark integrity hash mismatch: stored record is corrupt")]
    HashMismatch,

    /// The lent reader failed mid-stream, before a frame could be examined.
    #[error("reading the stored bookmark failed: {0}")]
    Read(#[source] std::io::Error),

    /// The frame was well-formed and intact, but its payload is not the
    /// record this codec writes — a logic error, since a matching hash means
    /// the bytes are the ones that were written.
    #[error("the bookmark payload is not a record this codec writes: {0}")]
    Record(#[source] RecordDefect),
}

/// Wrap `payload` in a bookmark frame declaring `version`.
///
/// Split from [`frame`] so the tests can build otherwise-valid frames
/// carrying a rejected version: the hash is computed over whatever version is
/// written, so version rejection is exercised on its own, not shadowed by
/// [`FormatError::HashMismatch`].
fn frame_as(version: u64, payload: &[u8]) -> Vec<u8> {
    // The hash's covered region: the encoded format-version item followed by
    // the encoded payload item (tag 24, byte-string header, payload bytes) —
    // built first, exactly as it will appear in the frame.
    let mut covered = Vec::with_capacity(9 + 2 + 9 + payload.len());
    cbor::write_head(&mut covered, MAJOR_UINT, version);
    let version_item_len = covered.len();
    covered.extend_from_slice(&EMBEDDED_CBOR);
    cbor::write_head(&mut covered, MAJOR_BSTR, payload.len() as u64);
    covered.extend_from_slice(payload);
    let hash = blake3::hash(&covered);

    let mut out = Vec::with_capacity(SELF_DESCRIBED_HEAD.len() + 1 + 2 + HASH_LEN + covered.len());
    out.extend_from_slice(&SELF_DESCRIBED_HEAD);
    out.push(FRAME_ARRAY);
    out.extend_from_slice(&covered[..version_item_len]);
    out.extend_from_slice(&INTEGRITY_HEAD);
    out.extend_from_slice(hash.as_bytes());
    out.extend_from_slice(&covered[version_item_len..]);
    out
}

/// Wrap `payload` in a bookmark frame: the self-described tag, the frame
/// array, the format version, a BLAKE3 hash over the version and payload
/// items, and the payload as an embedded CBOR item.
///
/// The inverse of [`unframe`].
pub(crate) fn frame(payload: &[u8]) -> Vec<u8> {
    frame_as(BOOKMARK_FORMAT_VERSION, payload)
}

/// A strict, position-tracking reader over a candidate frame.
///
/// Running out of bytes is [`FormatError::Truncated`]; bytes that differ from
/// the demanded spelling are [`FormatError::NotABookmark`] with the caller's
/// defect. Together the two carry the totality of the shape check: every byte
/// of the frame is either compared against a fixed spelling, parsed as a
/// shortest-form header, hashed, or payload.
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    /// Take the next `n` bytes, or report the frame truncated.
    fn take(&mut self, n: usize) -> Result<&'a [u8], FormatError> {
        let end = self
            .at
            .checked_add(n)
            .filter(|&end| end <= self.bytes.len())
            .ok_or(FormatError::Truncated {
                len: self.bytes.len(),
            })?;
        let taken = &self.bytes[self.at..end];
        self.at = end;
        Ok(taken)
    }

    /// Demand the exact bytes `spelling` next, else the named `defect`.
    fn expect(&mut self, spelling: &[u8], defect: FrameDefect) -> Result<(), FormatError> {
        if self.take(spelling.len())? != spelling {
            return Err(FormatError::NotABookmark { defect });
        }
        Ok(())
    }

    /// Parse a shortest-form header of the expected `major`, returning its
    /// argument, else the named `defect`.
    ///
    /// The head grammar itself — argument widths, shortest-form
    /// enforcement, indefinite and reserved rejection — is
    /// [`cbor::read_head`]'s, the crate's one canonical implementation.
    /// This adapter keeps only the frame's own concerns: the major gate
    /// first (a header of the wrong major is the caller's defect, whatever
    /// follows it), then truncation as the exact-position
    /// [`FormatError::Truncated`] and every grammar violation as
    /// [`FormatError::NotABookmark`] with the caller's defect.
    fn head(&mut self, major: u8, defect: FrameDefect) -> Result<u64, FormatError> {
        let len = self.bytes.len();
        let mut input = &self.bytes[self.at..];
        let initial = *input.first().ok_or(FormatError::Truncated { len })?;
        if initial >> 5 != major {
            return Err(FormatError::NotABookmark { defect });
        }
        let head = cbor::read_head(&mut input).map_err(|error| match error {
            cbor::HeadError::Truncated => FormatError::Truncated { len },
            _ => FormatError::NotABookmark { defect },
        })?;
        self.at = len - input.len();
        Ok(head.value)
    }
}

/// Validate a bookmark frame and return its payload slice.
///
/// Checks, in order: the self-described tag and frame shape, the format
/// version, then the integrity hash over the version and payload items. The
/// inverse of [`frame`]: `unframe(&frame(p)) == Ok(p)`.
///
/// # Errors
///
/// [`FormatError::Truncated`], [`NotABookmark`](FormatError::NotABookmark),
/// [`VersionMismatch`](FormatError::VersionMismatch), or
/// [`HashMismatch`](FormatError::HashMismatch) — each pinpointing how the
/// bytes failed to be a frame this build can trust.
pub(crate) fn unframe(bytes: &[u8]) -> Result<&[u8], FormatError> {
    let mut reader = Reader { bytes, at: 0 };
    reader.expect(&SELF_DESCRIBED_HEAD, FrameDefect::SelfDescribedTag)?;
    reader.expect(&[FRAME_ARRAY], FrameDefect::FrameArray)?;

    let version_start = reader.at;
    let version = reader.head(MAJOR_UINT, FrameDefect::FormatVersion)?;
    let version_end = reader.at;
    if version != BOOKMARK_FORMAT_VERSION {
        return Err(FormatError::VersionMismatch { found: version });
    }

    reader.expect(&INTEGRITY_HEAD, FrameDefect::Integrity)?;
    let stored_hash = reader.take(HASH_LEN)?;

    let payload_start = reader.at;
    reader.expect(&EMBEDDED_CBOR, FrameDefect::PayloadTag)?;
    let declared = reader.head(MAJOR_BSTR, FrameDefect::PayloadByteString)?;
    if declared > (bytes.len() - reader.at) as u64 {
        return Err(FormatError::Truncated { len: bytes.len() });
    }
    let payload = reader.take(declared as usize).expect("length checked");
    if reader.at != bytes.len() {
        return Err(FormatError::NotABookmark {
            defect: FrameDefect::TrailingBytes,
        });
    }

    let mut hasher = blake3::Hasher::new();
    hasher.update(&bytes[version_start..version_end]);
    hasher.update(&bytes[payload_start..]);
    if hasher.finalize().as_bytes().as_slice() != stored_hash {
        return Err(FormatError::HashMismatch);
    }

    Ok(payload)
}

/// Serialize a record into a complete bookmark frame.
///
/// Spells the record as one CBOR map — network identifier byte strings to
/// arrays of [`CLOCK_TAG`]-tagged clock byte strings — then [`frame`]s it.
/// The inverse of [`decode`].
pub(crate) fn encode(record: &BTreeMap<Network, Vec<Clock>>) -> Vec<u8> {
    let map = Value::Map(
        record
            .iter()
            .map(|(network, clocks)| {
                (
                    Value::Bytes(network.to_bytes().to_vec()),
                    Value::Array(
                        clocks
                            .iter()
                            .map(|clock| {
                                Value::Tag(CLOCK_TAG, Box::new(Value::Bytes(clock.encode())))
                            })
                            .collect(),
                    ),
                )
            })
            .collect(),
    );
    // Encoding to a `Vec` cannot fail: every value is a plain byte string or
    // container, and a `Vec` never fails to extend.
    let mut payload = Vec::new();
    ciborium::ser::into_writer(&map, &mut payload)
        .expect("encoding a record to a Vec is infallible");
    frame(&payload)
}

/// Validate a bookmark frame and deserialize its record.
///
/// [`unframe`]s, then walks the payload, which must be exactly one CBOR map
/// of the shape [`encode`] writes — every clock carrying [`CLOCK_TAG`]. The
/// inverse of [`encode`].
///
/// # Errors
///
/// Any [`unframe`] error, or [`FormatError::Record`] if a frame that passed
/// its integrity check nonetheless held an undecodable payload (a logic
/// error, not corruption).
pub(crate) fn decode(bytes: &[u8]) -> Result<BTreeMap<Network, Vec<Clock>>, FormatError> {
    let payload = unframe(bytes)?;
    walk(payload).map_err(FormatError::Record)
}

/// Walk an unframed payload into the record it spells.
fn walk(payload: &[u8]) -> Result<BTreeMap<Network, Vec<Clock>>, RecordDefect> {
    let mut input = payload;
    let value: Value = ciborium::de::from_reader(&mut input).map_err(RecordDefect::Cbor)?;
    if !input.is_empty() {
        return Err(RecordDefect::TrailingBytes {
            trailing: input.len(),
        });
    }

    let Value::Map(entries) = value else {
        return Err(RecordDefect::NotAMap);
    };
    let mut record = BTreeMap::new();
    for (key, clocks) in entries {
        let Value::Bytes(key) = key else {
            return Err(RecordDefect::KeyNotBytes);
        };
        let network = Network::from_bytes(
            key.as_slice()
                .try_into()
                .map_err(|_| RecordDefect::KeyWidth { len: key.len() })?,
        );
        let Value::Array(clocks) = clocks else {
            return Err(RecordDefect::ClocksNotArray);
        };
        let mut decoded = Vec::with_capacity(clocks.len());
        for clock in clocks {
            let Value::Tag(tag, boxed) = clock else {
                return Err(RecordDefect::ClockUntagged);
            };
            if tag != CLOCK_TAG {
                return Err(RecordDefect::ClockTag { found: tag });
            }
            let Value::Bytes(clock) = *boxed else {
                return Err(RecordDefect::ClockNotBytes);
            };
            decoded.push(Clock::decode(clock.as_slice()).map_err(RecordDefect::Clock)?);
        }
        record.insert(network, decoded);
    }
    Ok(record)
}

#[cfg(test)]
mod tests;
