//! Semantic wire frames after signal decoding.

use std::marker::PhantomData;

use crate::{
    Version,
    message::Message,
    tree::{
        mirror::framing::{LENGTH_HEADER_LEN, LengthOverflow, length_header},
        typed::{Hash, hash::MERKLE_HASH_LEN},
    },
};

use super::error::{DecodeLeafError, QueryOrderError};
use super::signal::{End, Flow, Stream};

use serde::de::DeserializeOwned;
/// The count byte stores one less than the nonempty query's actual fan.
pub const QUERY_COUNT_BIAS: usize = 1;

/// Largest query fan representable by a count-minus-one byte.
pub const MAX_QUERY_CHILDREN: usize = u8::MAX as usize + QUERY_COUNT_BIAS;

/// Bytes occupied by one query child: its radix followed by its Merkle hash.
pub const QUERY_CHILD_LEN: usize = std::mem::size_of::<u8>() + MERKLE_HASH_LEN;

/// Bytes occupied by the count-minus-one field of a nonempty query.
pub const QUERY_COUNT_LEN: usize = std::mem::size_of::<u8>();

/// Items in the adjacent-child window used to validate strict ordering.
const ADJACENT_CHILD_COUNT: usize = 2;

/// The body of one complete reaction frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reaction<T> {
    Match,
    Query(Vec<(u8, Hash)>),
    Supply(LeafRun<T>),
}

/// A protocol reaction frame or a boundary-only frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frame<T> {
    /// A reaction and whether another follows in its reply.
    Reaction(Reaction<T>, Flow),
    /// An empty reply or a transport-level stream-end control.
    End(End),
}

/// A frame paired with the logical stream named by its signal byte.
pub type WireFrame<T> = (Stream, Frame<T>);

/// One supply frame's run of leaf records, held in encoded form.
///
/// A run is a delimited sequence of one or more `(Version, Message<T>)`
/// records: each record is a [`LENGTH_HEADER_LEN`]-byte big-endian length
/// followed by one CBOR value (a byte string wrapping the version's
/// canonical encoding) and then the message's CBOR payload, back to back —
/// the record header delimits the payload, so it travels bare, and the
/// version's CBOR framing is what lets the decoder split the two without
/// re-measuring. The run stays encoded on both sides of the wire — the encoder
/// appends records copied from borrowed leaf data ([`push`](Self::push)) and
/// the decoder yields them one at a time ([`records`](Self::records)) — so
/// neither side materializes a decoded vector of leaves per frame; the bound
/// is one run's bytes.
///
/// Construction guarantees record framing: [`push`] rejects a record no run
/// body can carry within the wire's `u32` frame header, and
/// [`from_encoded`](Self::from_encoded) rejects wire bytes whose headers do
/// not chain exactly to the end. A [`records`] iterator therefore never
/// fails structurally, only on a record's canonical content.
///
/// [`push`]: Self::push
/// [`records`]: Self::records
pub struct LeafRun<T> {
    bytes: Vec<u8>,
    marker: PhantomData<fn() -> T>,
}

impl<T> Default for LeafRun<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Clone for LeafRun<T> {
    fn clone(&self) -> Self {
        Self {
            bytes: self.bytes.clone(),
            marker: PhantomData,
        }
    }
}

impl<T> PartialEq for LeafRun<T> {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes
    }
}

impl<T> Eq for LeafRun<T> {}

impl<T> std::fmt::Debug for LeafRun<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LeafRun")
            .field("records", &self.record_count())
            .field("encoded_len", &self.encoded_len())
            .finish()
    }
}

impl<T> LeafRun<T> {
    /// Start an empty run; at least one record must be pushed before it may
    /// become a frame.
    pub fn new() -> Self {
        Self {
            bytes: Vec::new(),
            marker: PhantomData,
        }
    }

    /// Whether no record has been pushed yet.
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Bytes this run occupies on the wire, excluding signal and run length.
    pub fn encoded_len(&self) -> usize {
        self.bytes.len()
    }

    /// The exact wire bytes of this run's records.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Bytes one record with these components will occupy in a run.
    ///
    /// Exactly what [`push`](Self::push) writes — the record header, the
    /// version's CBOR byte-string framing plus its canonical bytes, and
    /// the payload — pinned against an actual push by
    /// `record_len_matches_an_actual_push`. Saturating: a sum past
    /// `usize::MAX` cannot occur for in-memory slices, and an over-large
    /// record is rejected by [`push`](Self::push) regardless.
    pub fn record_len(version: &Version, message: &Message<T>) -> usize {
        let version = version.as_bytes().len();
        LENGTH_HEADER_LEN
            .saturating_add(cbor_bytes_header_len(version))
            .saturating_add(version)
            .saturating_add(message.as_slice().len())
    }

    /// Append one leaf record from borrowed components.
    ///
    /// # Errors
    ///
    /// Rejects a record no run can carry — one whose combined encoding plus
    /// its own record header exceeds the `u32` run-body limit — leaving the
    /// run untouched.
    pub fn push(&mut self, version: &Version, message: &Message<T>) -> Result<(), LengthOverflow> {
        let version = version.as_bytes();
        let message = message.as_slice();
        let len = cbor_bytes_header_len(version.len())
            .saturating_add(version.len())
            .saturating_add(message.len());
        let header = checked_record_header(len)?;
        self.bytes.reserve(LENGTH_HEADER_LEN + len);
        self.bytes.extend_from_slice(&header);
        write_cbor_bytes_header(&mut self.bytes, version.len());
        self.bytes.extend_from_slice(version);
        self.bytes.extend_from_slice(message);
        Ok(())
    }

    /// Validate wire bytes as a run: nonempty, headers chaining exactly.
    pub fn from_encoded(bytes: Vec<u8>) -> Result<Self, LeafRunError> {
        if bytes.is_empty() {
            return Err(LeafRunError::Empty);
        }
        let mut rest = bytes.as_slice();
        while !rest.is_empty() {
            if rest.len() < LENGTH_HEADER_LEN {
                return Err(LeafRunError::TruncatedHeader {
                    remaining: rest.len(),
                });
            }
            let (header, body) = rest.split_at(LENGTH_HEADER_LEN);
            let len = record_header(header);
            if body.len() < len {
                return Err(LeafRunError::TruncatedRecord {
                    len,
                    remaining: body.len(),
                });
            }
            rest = &body[len..];
        }
        Ok(Self {
            bytes,
            marker: PhantomData,
        })
    }

    /// The number of records in this run.
    pub fn record_count(&self) -> usize {
        self.record_slices().count()
    }

    /// Iterate the run's records, decoding each into its canonical pair.
    pub fn records(&self) -> impl Iterator<Item = Result<(Version, Message<T>), DecodeLeafError>>
    where
        T: DeserializeOwned,
    {
        self.record_slices().map(parse_record)
    }

    /// Split the validated run back into its exact record slices.
    ///
    /// `pub(super)` for the capture renderer, which decodes each
    /// record's version structurally without knowing the leaf type.
    pub(super) fn record_slices(&self) -> RecordSlices<'_> {
        RecordSlices { rest: &self.bytes }
    }
}

/// Iterator over the exact record bodies of a structurally valid run.
pub(super) struct RecordSlices<'a> {
    rest: &'a [u8],
}

impl<'a> Iterator for RecordSlices<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if self.rest.is_empty() {
            return None;
        }
        let (header, body) = self.rest.split_at(LENGTH_HEADER_LEN);
        let (record, rest) = body.split_at(record_header(header));
        self.rest = rest;
        Some(record)
    }
}

/// The record header for a `len`-byte record, checked against the outer frame.
///
/// A record is only encodable if the smallest run body holding it — the
/// record's bytes plus its own [`LENGTH_HEADER_LEN`]-byte header — fits the
/// wire's `u32` frame header, so the check charges the record header too.
/// [`LeafRun::push`] rejects on this boundary eagerly: an unshippable record
/// fails at record level rather than later at the outer frame.
fn checked_record_header(len: usize) -> Result<[u8; LENGTH_HEADER_LEN], LengthOverflow> {
    length_header(len.saturating_add(LENGTH_HEADER_LEN))?;
    Ok(length_header(len).expect("bounded by the header-charged check above"))
}

/// Read one record header; construction guarantees its width.
fn record_header(header: &[u8]) -> usize {
    u32::from_be_bytes(
        header
            .try_into()
            .expect("a validated run chunks exact record headers"),
    ) as usize
}

/// Decode one exact record body into its canonical pair.
fn parse_record<T: DeserializeOwned>(
    record: &[u8],
) -> Result<(Version, Message<T>), DecodeLeafError> {
    // Both fields are self-delimiting CBOR values, so the exact record
    // body parses without retrying, and whatever the payload's parse does
    // not consume is trailing.
    fn de_error(e: ciborium::de::Error<std::io::Error>) -> std::io::Error {
        match e {
            ciborium::de::Error::Io(e) => e,
            e => std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()),
        }
    }
    let mut input = record;
    let version: Version =
        ciborium::de::from_reader(&mut input).map_err(|e| DecodeLeafError::Version(de_error(e)))?;
    let payload = input;
    let message: T =
        ciborium::de::from_reader(&mut input).map_err(|e| DecodeLeafError::Message(de_error(e)))?;
    if !input.is_empty() {
        return Err(DecodeLeafError::TrailingBytes { count: input.len() });
    }
    let message = Message::from_decoded(message, bytes::Bytes::copy_from_slice(payload));
    Ok((version, message))
}

/// Bytes of the CBOR definite-length byte-string header for a `len`-byte
/// payload: the major-type-2 initial byte, plus the argument's width.
///
/// The dual of [`write_cbor_bytes_header`]; `record_len` prices with one
/// and `push` writes with the other, and the
/// `record_len_matches_an_actual_push` pin holds them together.
fn cbor_bytes_header_len(len: usize) -> usize {
    match len {
        0..=23 => 1,
        24..=0xff => 2,
        0x100..=0xffff => 3,
        0x1_0000..=0xffff_ffff => 5,
        _ => 9,
    }
}

/// Append the CBOR definite-length byte-string header for a `len`-byte
/// payload: exactly what [`ciborium`] emits for `serialize_bytes`.
fn write_cbor_bytes_header(out: &mut Vec<u8>, len: usize) {
    const MAJOR_BYTES: u8 = 2 << 5;
    match len {
        0..=23 => out.push(MAJOR_BYTES | len as u8),
        24..=0xff => out.extend_from_slice(&[MAJOR_BYTES | 24, len as u8]),
        0x100..=0xffff => {
            out.push(MAJOR_BYTES | 25);
            out.extend_from_slice(&(len as u16).to_be_bytes());
        }
        0x1_0000..=0xffff_ffff => {
            out.push(MAJOR_BYTES | 26);
            out.extend_from_slice(&(len as u32).to_be_bytes());
        }
        _ => {
            out.push(MAJOR_BYTES | 27);
            out.extend_from_slice(&(len as u64).to_be_bytes());
        }
    }
}

/// A supply run whose record framing is structurally invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LeafRunError {
    /// Every supply frame carries at least one record.
    #[error("a supply run carries no leaf records")]
    Empty,
    /// A record header overruns the run's declared length.
    #[error("a leaf record header overruns the {remaining} bytes left in its run")]
    TruncatedHeader { remaining: usize },
    /// A record body overruns the run's declared length.
    #[error("a leaf record of {len} bytes overruns the {remaining} bytes left in its run")]
    TruncatedRecord { len: usize, remaining: usize },
}

/// Validate that a radix listing is in canonical order: strictly ascending.
///
/// This is the one gate every child listing entering from the wire passes,
/// whichever surface carries it — a query frame's body or the greeting's
/// root-fan listing. Strictness is the whole invariant: the canonical form
/// admits each radix at most once, so an equal adjacent pair is rejected
/// exactly like a descent.
///
/// # Errors
///
/// The first adjacent non-ascending pair reports both radices as a
/// [`QueryOrderError`].
pub fn validate_children(children: &[(u8, Hash)]) -> Result<(), QueryOrderError> {
    for pair in children.windows(ADJACENT_CHILD_COUNT) {
        let [previous, current] = pair else {
            unreachable!("an adjacent-child window contains exactly two items")
        };
        if previous.0 >= current.0 {
            return Err(QueryOrderError {
                previous: previous.0,
                radix: current.0,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
