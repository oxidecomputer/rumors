//! The CBOR envelope and integrity check for ordered recovery records.
//!
//! ```text
//! 55799([format_version, integrity : bstr .size 32, 24(payload : bstr)])
//! ```
//!
//! The checksum protects the format version and payload bytes. Both writing and
//! verification feed their Serde encodings into SHA3-256: the version item,
//! followed by the tagged payload item. Equivalent spellings of the envelope
//! do not affect integrity. The payload itself is opaque bytes until its hash
//! has been checked; any change to those bytes changes the checksum input.
//!
//! The payload stores networks newest-first and identities oldest-first, with
//! one shared write frontier per network. Serde owns the CBOR encoding at both
//! levels; the record's adapters validate its structure and restore LRU order.

use ciborium::tag::Required;
use serde::{Serialize, de::DeserializeOwned};
use sha3::{Digest, Sha3_256};

use super::error::FormatError;
use super::record::{NetworkRecord, Record};
use super::serde::{Bytes, Complete};
use crate::Network;
use crate::tree::mirror::cbor::{self, TAG_EMBEDDED_ITEM, TAG_SELF_DESCRIBED};

/// The on-disk bookmark format version.
///
/// The current format stores one write frontier per network, separate from
/// identity entries, and preserves network and identity recency. Other version
/// numbers are rejected with [`FormatError::VersionMismatch`].
pub const BOOKMARK_FORMAT_VERSION: u64 = 6;

/// Width of the SHA3-256 integrity hash, in bytes.
const HASH_LEN: usize = 32;

/// The envelope's version, integrity digest, and embedded record bytes.
type Envelope =
    Required<Complete<(u64, Vec<u8>, Required<Vec<u8>, TAG_EMBEDDED_ITEM>)>, TAG_SELF_DESCRIBED>;

/// Hash the version and payload values using their normal Serde encodings.
fn checksum(version: u64, payload: &[u8]) -> [u8; HASH_LEN] {
    let mut hash = Sha3_256::new();
    hash.update(to_vec(&version));
    hash.update(to_vec(&Required::<_, TAG_EMBEDDED_ITEM>(Bytes(payload))));
    hash.finalize().into()
}

/// Frame a payload under the given version, including that version in the hash.
fn frame_as(version: u64, payload: &[u8]) -> Vec<u8> {
    let hash = checksum(version, payload);
    to_vec(&Required::<_, TAG_SELF_DESCRIBED>((
        version,
        Bytes(&hash),
        Required::<_, TAG_EMBEDDED_ITEM>(Bytes(payload)),
    )))
}

/// Protect a payload with the current format version and an integrity hash.
pub(crate) fn frame(payload: &[u8]) -> Vec<u8> {
    frame_as(BOOKMARK_FORMAT_VERSION, payload)
}

/// Decode one complete CBOR value, rejecting bytes after it.
fn from_slice<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, ciborium::de::Error<std::io::Error>> {
    let mut input = bytes;
    let value = ciborium::de::from_reader(&mut input)?;
    if !input.is_empty() {
        return Err(ciborium::de::Error::Semantic(
            Some(bytes.len() - input.len()),
            format!("{} trailing bytes", input.len()),
        ));
    }
    Ok(value)
}

/// Validate the envelope and checksum before returning the record bytes.
pub(crate) fn unframe(bytes: &[u8]) -> Result<Vec<u8>, FormatError> {
    let Required(Complete((version, hash, Required(payload)))) = from_slice::<Envelope>(bytes)
        .map_err(|defect| match defect {
            ciborium::de::Error::Io(ref error)
                if error.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                FormatError::Truncated { len: bytes.len() }
            }
            defect => FormatError::NotABookmark { defect },
        })?;
    if version != BOOKMARK_FORMAT_VERSION {
        return Err(FormatError::VersionMismatch { found: version });
    }
    if hash.as_slice() != checksum(version, &payload) {
        return Err(FormatError::HashMismatch);
    }
    Ok(payload)
}

/// Serialize an in-memory value to CBOR; these bookmark types serialize
/// infallibly.
fn to_vec(value: &impl Serialize) -> Vec<u8> {
    let mut bytes = Vec::new();
    ciborium::ser::into_writer(value, &mut bytes).expect("bookmark serialization into a Vec");
    bytes
}

/// Encode one network with the same Serde adapter used by the complete record.
pub(super) fn encode_network(network: Network, record: &NetworkRecord) -> Vec<u8> {
    to_vec(&super::record::serde::Entry(network, record))
}

/// The complete frame size for an array of already-encoded network records.
pub(crate) fn record_size(networks: usize, encoded_bytes: usize) -> usize {
    let payload = cbor::head_len(networks as u64) + encoded_bytes;
    cbor::head_len(TAG_SELF_DESCRIBED)
        + 1
        + cbor::head_len(BOOKMARK_FORMAT_VERSION)
        + cbor::head_len(HASH_LEN as u64)
        + HASH_LEN
        + cbor::head_len(TAG_EMBEDDED_ITEM)
        + cbor::head_len(payload as u64)
        + payload
}

/// Serialize the ordered record and protect its bytes with an integrity frame.
pub(super) fn encode(record: &Record) -> Vec<u8> {
    frame(&to_vec(record))
}

/// Verify integrity before deserializing any recovery rights.
pub(crate) fn decode(bytes: &[u8]) -> Result<Record, FormatError> {
    from_slice(&unframe(bytes)?).map_err(FormatError::Record)
}

#[cfg(test)]
mod tests;
