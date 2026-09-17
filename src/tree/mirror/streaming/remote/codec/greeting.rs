//! The V2 greeting's wire spelling.
//!
//! One control-stream item: an embedded-CBOR-item tag (24) wrapping a
//! byte string whose content is a text-keyed map. The embedding is what
//! keeps the control stream's reader trivial — the byte string's head
//! declares the whole greeting's length up front, so no incremental map
//! walk happens against the transport — while a generic tool unwraps
//! tag 24 as part of the standard vocabulary and sees the map.
//!
//! The map's keys ride in CBOR deterministic order (bytewise
//! lexicographic over their encodings), and the decoder requires exactly
//! this key set in exactly that order: one spelling per greeting.
//!
//! - `"listing"`: the sender's root-fan listing, the same
//!   `{radix: hash}` map spelling a query frame carries.
//! - `"set_len"`: the sender's declared set size.
//! - `"version"`: the sender's causal version — the version-atom tag
//!   wrapping a byte string of the version's canonical encoding.
//! - `"max_version_bytes"`: the sender's version-size bound.
//! - `"payload_depth_limit"`: the sender's payload nesting-depth limit,
//!   which the counterparty's must equal for the session to proceed.
//! - `"target_message_size"`: the sender's supply-run byte target.

use crate::{
    Version,
    tree::mirror::cbor::{
        self, HeadError, MAJOR_BSTR, MAJOR_TAG, MAJOR_TEXT, MAJOR_UINT, TAG_EMBEDDED_ITEM,
    },
    tree::mirror::streaming::message::Greeting,
};

use super::error::QueryOrderError;
use super::frame::{ListingIssue, parse_listing_map, write_listing};

/// The greeting fields, in the deterministic order the wire requires.
const FIELDS: [GreetingField; 6] = [
    GreetingField::Listing,
    GreetingField::SetLen,
    GreetingField::Version,
    GreetingField::MaxVersionBytes,
    GreetingField::PayloadDepthLimit,
    GreetingField::TargetMessageSize,
];

/// Render one greeting as its complete control-stream item:
/// tag 24 wrapping a byte string of the greeting map.
pub(crate) fn encode_greeting(greeting: &Greeting) -> Vec<u8> {
    let map = greeting_map(greeting);
    let mut item = Vec::with_capacity(
        cbor::head_len(TAG_EMBEDDED_ITEM) + cbor::head_len(map.len() as u64) + map.len(),
    );
    cbor::write_tag(&mut item, TAG_EMBEDDED_ITEM);
    cbor::write_head(&mut item, MAJOR_BSTR, map.len() as u64);
    item.extend_from_slice(&map);
    item
}

/// Render the greeting map alone.
fn greeting_map(greeting: &Greeting) -> Vec<u8> {
    let mut map = Vec::new();
    cbor::write_head(&mut map, cbor::MAJOR_MAP, FIELDS.len() as u64);
    write_key(&mut map, GreetingField::Listing);
    write_listing(&mut map, &greeting.listing);
    write_key(&mut map, GreetingField::SetLen);
    cbor::write_head(&mut map, MAJOR_UINT, greeting.set_len);
    write_key(&mut map, GreetingField::Version);
    let version = greeting.version.as_bytes();
    cbor::write_tag(&mut map, crate::tags::VERSION_TAG);
    cbor::write_head(&mut map, MAJOR_BSTR, version.len() as u64);
    map.extend_from_slice(version);
    write_key(&mut map, GreetingField::MaxVersionBytes);
    cbor::write_head(&mut map, MAJOR_UINT, greeting.max_version_bytes);
    write_key(&mut map, GreetingField::PayloadDepthLimit);
    cbor::write_head(&mut map, MAJOR_UINT, greeting.payload_depth_limit);
    write_key(&mut map, GreetingField::TargetMessageSize);
    cbor::write_head(&mut map, MAJOR_UINT, greeting.target_message_size);
    map
}

/// Append one field's text key to a greeting map.
fn write_key(out: &mut Vec<u8>, field: GreetingField) {
    let key = field.name();
    cbor::write_head(out, MAJOR_TEXT, key.len() as u64);
    out.extend_from_slice(key.as_bytes());
}

/// One field in the greeting map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GreetingField {
    /// The sender's root-fan listing.
    Listing,
    /// The sender's live message count.
    SetLen,
    /// The sender's causal version.
    Version,
    /// The sender's largest version encoding.
    MaxVersionBytes,
    /// The sender's payload nesting limit.
    PayloadDepthLimit,
    /// The sender's supply-run byte target.
    TargetMessageSize,
}

impl GreetingField {
    /// Return this field's wire key.
    const fn name(self) -> &'static str {
        match self {
            Self::Listing => "listing",
            Self::SetLen => "set_len",
            Self::Version => "version",
            Self::MaxVersionBytes => "max_version_bytes",
            Self::PayloadDepthLimit => "payload_depth_limit",
            Self::TargetMessageSize => "target_message_size",
        }
    }
}

/// Render a greeting field as its wire key.
impl std::fmt::Display for GreetingField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// Why the next greeting key did not name the expected field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum GreetingKeyError {
    /// The key's CBOR head has the wrong major type or byte length.
    #[error("received CBOR head {0:?}, expected a text key of this field's length")]
    Head(cbor::Head),
    /// The key's head declares more text bytes than remain.
    #[error("only {available} of its {declared} declared text bytes remain")]
    Truncated {
        /// Text bytes declared by the key's head.
        declared: usize,
        /// Text bytes still present in the greeting.
        available: usize,
    },
    /// The key has the expected length but different bytes.
    #[error("received different text of the expected length")]
    Spelling,
}

/// A structural defect in a complete greeting item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum GreetingStructureError {
    /// The embedded content does not begin with the fixed-size greeting map.
    #[error("map head is {actual:?}; expected a map with {expected} fields")]
    Map {
        /// Number of fields in the greeting vocabulary.
        expected: usize,
        /// Head found where the map should begin.
        actual: cbor::Head,
    },
    /// The next map key does not name the field required at this position.
    #[error("expected the {expected} key next, but {issue}")]
    Key {
        /// Field whose key belongs at this position.
        expected: GreetingField,
        /// How the received key differs.
        issue: GreetingKeyError,
    },
    /// A numeric field is encoded as another CBOR major type.
    #[error("{field} value has head {actual:?}; expected an unsigned integer")]
    Unsigned {
        /// Field whose value was being decoded.
        field: GreetingField,
        /// Head found where its unsigned integer belongs.
        actual: cbor::Head,
    },
    /// The version value does not begin with the version-atom tag.
    #[error("version value has tag head {actual:?}; expected tag {expected}")]
    VersionTag {
        /// Version-atom tag number required by the protocol.
        expected: u64,
        /// Head found where the tag belongs.
        actual: cbor::Head,
    },
    /// The version tag does not wrap a byte string.
    #[error("version atom has head {actual:?}; expected a byte string")]
    VersionBytes {
        /// Head found where the byte string belongs.
        actual: cbor::Head,
    },
    /// The version byte string cannot be addressed on this platform.
    #[error("version atom declares {declared} bytes, which do not fit in memory")]
    VersionTooLarge {
        /// Byte length declared by the version atom.
        declared: u64,
    },
    /// The greeting ends inside the version byte string.
    #[error("version atom declares {declared} bytes, but only {available} remain")]
    VersionTruncated {
        /// Byte length declared by the version atom.
        declared: usize,
        /// Bytes available after its head.
        available: usize,
    },
    /// Bytes remain after all greeting fields have been decoded.
    #[error("{remaining} trailing bytes follow the greeting map")]
    Trailing {
        /// Bytes remaining after the final field.
        remaining: usize,
    },
    /// The control-stream item does not begin with the embedded-item tag.
    #[error("item has tag head {actual:?}; expected tag {expected}")]
    ItemTag {
        /// Embedded-item tag number required by the protocol.
        expected: u64,
        /// Head found where the tag belongs.
        actual: cbor::Head,
    },
    /// The embedded-item tag does not wrap a byte string.
    #[error("embedded item has head {actual:?}; expected a byte string")]
    ItemBytes {
        /// Head found where the byte string belongs.
        actual: cbor::Head,
    },
    /// The greeting item's declared body cannot be addressed on this platform.
    #[error("greeting item declares {declared} bytes, which do not fit in memory")]
    ItemTooLarge {
        /// Byte length declared by the item.
        declared: u64,
    },
}

/// A greeting whose structure or encoding violates the wire format.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum GreetingError {
    /// A head was truncated, indefinite, reserved, or widened.
    #[error("greeting head is not canonical: {0}")]
    Head(HeadError),
    /// The greeting's fixed structure is malformed.
    #[error("greeting is malformed: {0}")]
    Structure(GreetingStructureError),
    /// The listing map violated a structural rule.
    #[error("greeting listing is malformed: {0}")]
    Listing(ListingIssue),
    /// The version atom's bytes are not one canonical version encoding.
    #[error("greeting version does not decode: {0}")]
    Version(before::error::Decode),
}

/// Parse a greeting map from the embedded byte string's exact content.
pub(crate) fn parse_greeting(bytes: &[u8]) -> Result<Greeting, GreetingError> {
    let mut input = bytes;
    let head = cbor::read_head(&mut input).map_err(GreetingError::Head)?;
    if head.major != cbor::MAJOR_MAP || head.value != FIELDS.len() as u64 {
        return Err(GreetingError::Structure(GreetingStructureError::Map {
            expected: FIELDS.len(),
            actual: head,
        }));
    }
    expect_key(&mut input, GreetingField::Listing)?;
    let listing = parse_listing_map(&mut input).map_err(GreetingError::Listing)?;
    expect_key(&mut input, GreetingField::SetLen)?;
    let set_len = uint(&mut input, GreetingField::SetLen)?;
    expect_key(&mut input, GreetingField::Version)?;
    let head = cbor::read_head(&mut input).map_err(GreetingError::Head)?;
    if head.major != MAJOR_TAG || head.value != crate::tags::VERSION_TAG {
        return Err(GreetingError::Structure(
            GreetingStructureError::VersionTag {
                expected: crate::tags::VERSION_TAG,
                actual: head,
            },
        ));
    }
    let head = cbor::read_head(&mut input).map_err(GreetingError::Head)?;
    if head.major != MAJOR_BSTR {
        return Err(GreetingError::Structure(
            GreetingStructureError::VersionBytes { actual: head },
        ));
    }
    let Ok(len) = usize::try_from(head.value) else {
        return Err(GreetingError::Structure(
            GreetingStructureError::VersionTooLarge {
                declared: head.value,
            },
        ));
    };
    let Some((atom, rest)) = split(input, len) else {
        return Err(GreetingError::Structure(
            GreetingStructureError::VersionTruncated {
                declared: len,
                available: input.len(),
            },
        ));
    };
    input = rest;
    let version = Version::decode(atom).map_err(GreetingError::Version)?;
    expect_key(&mut input, GreetingField::MaxVersionBytes)?;
    let max_version_bytes = uint(&mut input, GreetingField::MaxVersionBytes)?;
    expect_key(&mut input, GreetingField::PayloadDepthLimit)?;
    let payload_depth_limit = uint(&mut input, GreetingField::PayloadDepthLimit)?;
    expect_key(&mut input, GreetingField::TargetMessageSize)?;
    let target_message_size = uint(&mut input, GreetingField::TargetMessageSize)?;
    if !input.is_empty() {
        return Err(GreetingError::Structure(GreetingStructureError::Trailing {
            remaining: input.len(),
        }));
    }
    Ok(Greeting {
        version,
        set_len,
        max_version_bytes,
        payload_depth_limit,
        target_message_size,
        listing,
    })
}

/// Consume the next greeting key, requiring `expected` exactly.
fn expect_key(input: &mut &[u8], expected: GreetingField) -> Result<(), GreetingError> {
    let name = expected.name();
    let head = cbor::read_head(input).map_err(GreetingError::Head)?;
    if head.major != MAJOR_TEXT || head.value != name.len() as u64 {
        return Err(GreetingError::Structure(GreetingStructureError::Key {
            expected,
            issue: GreetingKeyError::Head(head),
        }));
    }
    let Some((text, rest)) = split(input, name.len()) else {
        return Err(GreetingError::Structure(GreetingStructureError::Key {
            expected,
            issue: GreetingKeyError::Truncated {
                declared: name.len(),
                available: input.len(),
            },
        }));
    };
    *input = rest;
    if text != name.as_bytes() {
        return Err(GreetingError::Structure(GreetingStructureError::Key {
            expected,
            issue: GreetingKeyError::Spelling,
        }));
    }
    Ok(())
}

/// Read one unsigned integer for `field`.
fn uint(input: &mut &[u8], field: GreetingField) -> Result<u64, GreetingError> {
    let head = cbor::read_head(input).map_err(GreetingError::Head)?;
    if head.major != MAJOR_UINT {
        return Err(GreetingError::Structure(GreetingStructureError::Unsigned {
            field,
            actual: head,
        }));
    }
    Ok(head.value)
}

/// Split `len` leading bytes off `input`, or `None` when it is shorter.
fn split(input: &[u8], len: usize) -> Option<(&[u8], &[u8])> {
    (input.len() >= len).then(|| input.split_at(len))
}

/// Read one complete greeting item from the control stream.
///
/// Transport failures pass through as `Err(Ok-side io)`; a malformed or
/// non-canonical greeting is a typed [`GreetingError`], except a
/// non-canonical listing order, surfaced separately so the handshake can
/// report it as the codec's own violation class.
pub(crate) async fn read_greeting<R>(read: &mut R) -> Result<Greeting, ReadGreetingError>
where
    R: tokio::io::AsyncRead + Unpin,
{
    use crate::tree::mirror::framing::read_payload;
    let head = cbor::read_head_async(read)
        .await
        .map_err(head_read_error)?
        .ok_or_else(|| {
            ReadGreetingError::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "peer closed before its greeting",
            ))
        })?;
    if head.major != MAJOR_TAG || head.value != TAG_EMBEDDED_ITEM {
        return Err(ReadGreetingError::Decode(GreetingError::Structure(
            GreetingStructureError::ItemTag {
                expected: TAG_EMBEDDED_ITEM,
                actual: head,
            },
        )));
    }
    let head = cbor::read_head_async(read)
        .await
        .map_err(head_read_error)?
        .ok_or_else(|| {
            ReadGreetingError::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "peer closed inside its greeting",
            ))
        })?;
    if head.major != MAJOR_BSTR {
        return Err(ReadGreetingError::Decode(GreetingError::Structure(
            GreetingStructureError::ItemBytes { actual: head },
        )));
    }
    let Ok(len) = usize::try_from(head.value) else {
        return Err(ReadGreetingError::Decode(GreetingError::Structure(
            GreetingStructureError::ItemTooLarge {
                declared: head.value,
            },
        )));
    };
    let bytes = read_payload(read, len)
        .await
        .map_err(ReadGreetingError::Io)?;
    parse_greeting(&bytes).map_err(|error| match error {
        GreetingError::Listing(ListingIssue::Order(order)) => ReadGreetingError::Listing(order),
        error => ReadGreetingError::Decode(error),
    })
}

/// How reading a greeting from the control stream failed.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ReadGreetingError {
    /// The transport failed before the greeting arrived whole.
    #[error(transparent)]
    Io(std::io::Error),
    /// The greeting arrived but is not canonical rumors CBOR.
    #[error(transparent)]
    Decode(GreetingError),
    /// The greeting's listing violated canonical child order.
    #[error(transparent)]
    Listing(QueryOrderError),
}

fn head_read_error(e: cbor::HeadReadError) -> ReadGreetingError {
    match e {
        cbor::HeadReadError::Io(io) => ReadGreetingError::Io(io),
        cbor::HeadReadError::Malformed(head) => {
            ReadGreetingError::Decode(GreetingError::Head(head))
        }
    }
}

#[cfg(test)]
mod tests;
