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
//! - `"protocol"`: the text `"rumors"` — the protocol magic. The
//!   preamble's self-described CBOR tag announces only "this is CBOR";
//!   this entry is what marks the session as a rumors stream to a
//!   reader holding nothing but the bytes.
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

/// The greeting map's keys, in the deterministic (bytewise lexicographic)
/// order the wire requires.
const KEYS: [&str; 7] = [
    "listing",
    "set_len",
    "version",
    "protocol",
    "max_version_bytes",
    "payload_depth_limit",
    "target_message_size",
];

/// The protocol magic carried by the greeting's `"protocol"` entry.
const PROTOCOL_NAME: &str = "rumors";

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
    cbor::write_head(&mut map, cbor::MAJOR_MAP, KEYS.len() as u64);
    for key in KEYS {
        cbor::write_head(&mut map, MAJOR_TEXT, key.len() as u64);
        map.extend_from_slice(key.as_bytes());
        match key {
            "listing" => write_listing(&mut map, &greeting.listing),
            "set_len" => cbor::write_head(&mut map, MAJOR_UINT, greeting.set_len),
            "version" => {
                let version = greeting.version.as_bytes();
                cbor::write_tag(&mut map, crate::tags::VERSION_TAG);
                cbor::write_head(&mut map, MAJOR_BSTR, version.len() as u64);
                map.extend_from_slice(version);
            }
            "protocol" => {
                cbor::write_head(&mut map, MAJOR_TEXT, PROTOCOL_NAME.len() as u64);
                map.extend_from_slice(PROTOCOL_NAME.as_bytes());
            }
            "max_version_bytes" => {
                cbor::write_head(&mut map, MAJOR_UINT, greeting.max_version_bytes);
            }
            "payload_depth_limit" => {
                cbor::write_head(&mut map, MAJOR_UINT, greeting.payload_depth_limit);
            }
            "target_message_size" => {
                cbor::write_head(&mut map, MAJOR_UINT, greeting.target_message_size);
            }
            _ => unreachable!("the key roster is exhaustive"),
        }
    }
    map
}

/// A greeting that is not canonical rumors CBOR.
///
/// Carried by [`RemoteError::HandshakeDecode`]: the greeting item
/// arrived, but its spelling or content violates the wire's
/// deterministic-encoding contract. The greeting admits one spelling per
/// value, so every variant here is a counterparty bug, never an
/// alternate encoding.
///
/// [`RemoteError::HandshakeDecode`]: super::super::RemoteError::HandshakeDecode
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum GreetingError {
    /// A head was truncated, indefinite, reserved, or widened.
    #[error("greeting head is not canonical: {0}")]
    Head(HeadError),
    /// An item had the wrong major type, value, or position.
    #[error("greeting is malformed: {0}")]
    Shape(&'static str),
    /// The listing map violated a structural rule.
    #[error("greeting listing is malformed: {0}")]
    Listing(ListingIssue),
    /// The listing's keys were not in canonical strictly ascending order.
    #[error(transparent)]
    Order(QueryOrderError),
    /// The version atom's bytes are not one canonical version encoding.
    #[error("greeting version does not decode: {0}")]
    Version(before::error::Decode),
}

/// Parse a greeting map from the embedded byte string's exact content.
pub(crate) fn parse_greeting(bytes: &[u8]) -> Result<Greeting, GreetingError> {
    let mut input = bytes;
    let head = cbor::read_head(&mut input).map_err(GreetingError::Head)?;
    if head.major != cbor::MAJOR_MAP || head.value != KEYS.len() as u64 {
        return Err(GreetingError::Shape(
            "greeting is not a map of one entry per roster key",
        ));
    }
    let mut version = None;
    let mut set_len = None;
    let mut max_version_bytes = None;
    let mut payload_depth_limit = None;
    let mut target_message_size = None;
    let mut listing = None;
    for key in KEYS {
        let head = cbor::read_head(&mut input).map_err(GreetingError::Head)?;
        if head.major != MAJOR_TEXT || head.value != key.len() as u64 {
            return Err(GreetingError::Shape(
                "greeting keys are not the deterministic roster",
            ));
        }
        let Some((text, rest)) = split(input, key.len()) else {
            return Err(GreetingError::Shape("greeting key is truncated"));
        };
        input = rest;
        if text != key.as_bytes() {
            return Err(GreetingError::Shape(
                "greeting keys are not the deterministic roster",
            ));
        }
        match key {
            "listing" => {
                listing = Some(parse_listing_map(&mut input).map_err(|issue| match issue {
                    ListingIssue::Order(order) => GreetingError::Order(order),
                    issue => GreetingError::Listing(issue),
                })?);
            }
            "set_len" => set_len = Some(uint(&mut input, "set_len is not an unsigned int")?),
            "version" => {
                let head = cbor::read_head(&mut input).map_err(GreetingError::Head)?;
                if head.major != MAJOR_TAG || head.value != crate::tags::VERSION_TAG {
                    return Err(GreetingError::Shape(
                        "greeting version does not carry the version-atom tag",
                    ));
                }
                let head = cbor::read_head(&mut input).map_err(GreetingError::Head)?;
                if head.major != MAJOR_BSTR {
                    return Err(GreetingError::Shape(
                        "greeting version tag does not wrap a byte string",
                    ));
                }
                let Ok(len) = usize::try_from(head.value) else {
                    return Err(GreetingError::Shape("greeting version outsizes memory"));
                };
                let Some((atom, rest)) = split(input, len) else {
                    return Err(GreetingError::Shape("greeting version is truncated"));
                };
                input = rest;
                version = Some(Version::decode(atom).map_err(GreetingError::Version)?);
            }
            "protocol" => {
                let head = cbor::read_head(&mut input).map_err(GreetingError::Head)?;
                if head.major != MAJOR_TEXT || head.value != PROTOCOL_NAME.len() as u64 {
                    return Err(GreetingError::Shape("greeting protocol magic is absent"));
                }
                let Some((name, rest)) = split(input, PROTOCOL_NAME.len()) else {
                    return Err(GreetingError::Shape("greeting protocol magic is truncated"));
                };
                input = rest;
                if name != PROTOCOL_NAME.as_bytes() {
                    return Err(GreetingError::Shape(
                        "greeting protocol magic is not \"rumors\"",
                    ));
                }
            }
            "max_version_bytes" => {
                max_version_bytes = Some(uint(
                    &mut input,
                    "max_version_bytes is not an unsigned int",
                )?);
            }
            "payload_depth_limit" => {
                payload_depth_limit = Some(uint(
                    &mut input,
                    "payload_depth_limit is not an unsigned int",
                )?);
            }
            "target_message_size" => {
                target_message_size = Some(uint(
                    &mut input,
                    "target_message_size is not an unsigned int",
                )?);
            }
            _ => unreachable!("the key roster is exhaustive"),
        }
    }
    if !input.is_empty() {
        return Err(GreetingError::Shape("greeting carries trailing bytes"));
    }
    Ok(Greeting {
        version: version.expect("the roster visits version"),
        set_len: set_len.expect("the roster visits set_len"),
        max_version_bytes: max_version_bytes.expect("the roster visits max_version_bytes"),
        payload_depth_limit: payload_depth_limit.expect("the roster visits payload_depth_limit"),
        target_message_size: target_message_size.expect("the roster visits target_message_size"),
        listing: listing.expect("the roster visits listing"),
    })
}

/// Read one unsigned-int value, returning `detail` as the shape
/// diagnostic when the item is not an unsigned int.
fn uint(input: &mut &[u8], detail: &'static str) -> Result<u64, GreetingError> {
    let head = cbor::read_head(input).map_err(GreetingError::Head)?;
    if head.major != MAJOR_UINT {
        return Err(GreetingError::Shape(detail));
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
        return Err(ReadGreetingError::Decode(GreetingError::Shape(
            "greeting does not open with the embedded-item tag",
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
        return Err(ReadGreetingError::Decode(GreetingError::Shape(
            "greeting tag does not wrap a byte string",
        )));
    }
    let Ok(len) = usize::try_from(head.value) else {
        return Err(ReadGreetingError::Decode(GreetingError::Shape(
            "greeting declares an unaddressable length",
        )));
    };
    let bytes = read_payload(read, len)
        .await
        .map_err(ReadGreetingError::Io)?;
    parse_greeting(&bytes).map_err(|e| match e {
        GreetingError::Order(order) => ReadGreetingError::Listing(order),
        e => ReadGreetingError::Decode(e),
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
