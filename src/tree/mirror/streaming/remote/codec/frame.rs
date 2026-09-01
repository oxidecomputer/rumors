//! Semantic wire frames after signal decoding.

use crate::{
    Version,
    message::{Message, PayloadCodec},
    tree::{
        mirror::cbor::{
            self, HeadError, MAJOR_BSTR, MAJOR_MAP, MAJOR_TAG, MAJOR_UINT, TAG_CBOR_SEQUENCE,
        },
        mirror::framing::LengthOverflow,
        typed::{Hash, hash::MERKLE_HASH_LEN},
    },
};

use super::error::{DecodeLeafError, QueryOrderError};
use super::signal::{End, Flow, Stream};

/// Largest query fan a listing map can carry: one child per radix value.
pub const MAX_QUERY_CHILDREN: usize = 256;

/// Bytes of the byte-string head ahead of one listed Merkle hash.
pub const HASH_HEAD_LEN: usize = cbor::head_len(MERKLE_HASH_LEN as u64);

/// Bytes one listed child occupies as a map entry: its radix key's head,
/// then its hash value's head and digest bytes. Radixes of 24 and above
/// take a two-byte key head; smaller radixes take one.
pub const fn listing_entry_len(radix: u8) -> usize {
    cbor::head_len(radix as u64) + HASH_HEAD_LEN + MERKLE_HASH_LEN
}

/// Head bytes of the embedded-CBOR-sequence tag (63) opening every supply
/// run and every record within one.
pub(super) const RECORD_TAG_LEN: usize = cbor::head_len(TAG_CBOR_SEQUENCE);

/// Head bytes of the version-atom tag ahead of a record's version.
const VERSION_TAG_LEN: usize = cbor::head_len(crate::tags::VERSION_TAG);

/// The body of one complete reaction frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reaction {
    Match,
    Query(Vec<(u8, Hash)>),
    Supply(LeafRun),
}

/// A protocol reaction frame or a boundary-only frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frame {
    /// A reaction and whether another follows in its reply.
    Reaction(Reaction, Flow),
    /// An empty reply or a transport-level stream-end control.
    End(End),
}

/// A frame paired with the logical stream named by its signal.
pub type WireFrame = (Stream, Frame);

/// One supply frame's run of leaf records, held in encoded form.
///
/// A run is a CBOR sequence of one or more records. Each record is an
/// embedded-sequence item — tag 63 wrapping a byte string — whose content
/// is itself a two-item CBOR sequence: the version atom (its own tag
/// wrapping a byte string of the version's canonical encoding) followed by
/// the message's CBOR payload. The record's byte-string head delimits the
/// payload, so the payload travels bare, and the version's framing is what
/// lets the decoder split the two without re-measuring. The run stays
/// encoded on both sides of the wire — the encoder appends records copied
/// from borrowed leaf data ([`push`](Self::push)) and the decoder yields
/// them one at a time ([`records`](Self::records)) — so neither side
/// materializes a decoded vector of leaves per frame; the bound is one
/// run's bytes.
///
/// Construction guarantees record framing: [`push`] rejects a record no
/// run body can carry within the wire's run byte cap, and
/// [`from_encoded`](Self::from_encoded) rejects wire bytes whose record
/// items do not chain exactly to the end in canonical form. A [`records`]
/// iterator therefore never fails structurally, only on a record's
/// content: a version-atom tag that is missing, non-canonical, or cut
/// short by the record's end (the tag's head is hand-parsed and
/// spelling-judged), a version item the general CBOR reader cannot
/// decode behind that tag, a version atom whose content bytes fail the
/// strict [`Version`] decoder (the atom's byte-string head is read by
/// that general reader and not re-judged for spelling), or an
/// application payload that does not decode.
///
/// [`push`]: Self::push
/// [`records`]: Self::records
pub struct LeafRun {
    bytes: Vec<u8>,
}

impl Default for LeafRun {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for LeafRun {
    fn clone(&self) -> Self {
        Self {
            bytes: self.bytes.clone(),
        }
    }
}

impl PartialEq for LeafRun {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes
    }
}

impl Eq for LeafRun {}

impl std::fmt::Debug for LeafRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LeafRun")
            .field("records", &self.record_count())
            .field("encoded_len", &self.encoded_len())
            .finish()
    }
}

impl LeafRun {
    /// Start an empty run; at least one record must be pushed before it may
    /// become a frame.
    pub fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    /// Whether no record has been pushed yet.
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Bytes this run occupies on the wire, excluding the frame head and
    /// the run's own embedded-sequence head.
    pub fn encoded_len(&self) -> usize {
        self.bytes.len()
    }

    /// The exact wire bytes of this run's records.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Bytes one record with these components will occupy in a run.
    ///
    /// Exactly what [`push`](Self::push) writes — the record's
    /// embedded-sequence tag and byte-string head, the version atom's tag
    /// and byte-string framing plus its canonical bytes, and the payload —
    /// pinned against an actual push by `record_len_matches_an_actual_push`.
    /// Saturating: a sum past `usize::MAX` cannot occur for in-memory
    /// slices, and an over-large record is rejected by [`push`](Self::push)
    /// regardless.
    pub fn record_len(version: &Version, message: &Message) -> usize {
        let body = Self::record_body_len(version, message);
        RECORD_TAG_LEN
            .saturating_add(cbor::head_len(body as u64))
            .saturating_add(body)
    }

    /// Bytes of a record's content behind its embedded-sequence head.
    fn record_body_len(version: &Version, message: &Message) -> usize {
        let version = version.as_bytes().len();
        VERSION_TAG_LEN
            .saturating_add(cbor::head_len(version as u64))
            .saturating_add(version)
            .saturating_add(message.as_slice().len())
    }

    /// Append one leaf record from borrowed components.
    ///
    /// # Errors
    ///
    /// Rejects a record no run can carry — one whose whole record item
    /// exceeds the wire's run byte cap — leaving the run untouched.
    pub fn push(&mut self, version: &Version, message: &Message) -> Result<(), LengthOverflow> {
        let body = Self::record_body_len(version, message);
        let item = RECORD_TAG_LEN
            .saturating_add(cbor::head_len(body as u64))
            .saturating_add(body);
        checked_run_len(item)?;
        let version = version.as_bytes();
        let message = message.as_slice();
        self.bytes.reserve(item);
        cbor::write_tag(&mut self.bytes, TAG_CBOR_SEQUENCE);
        cbor::write_head(&mut self.bytes, MAJOR_BSTR, body as u64);
        cbor::write_tag(&mut self.bytes, crate::tags::VERSION_TAG);
        cbor::write_head(&mut self.bytes, MAJOR_BSTR, version.len() as u64);
        self.bytes.extend_from_slice(version);
        self.bytes.extend_from_slice(message);
        Ok(())
    }

    /// Validate wire bytes as a run: nonempty, canonical record items
    /// chaining exactly to the end.
    pub fn from_encoded(bytes: Vec<u8>) -> Result<Self, LeafRunError> {
        if bytes.is_empty() {
            return Err(LeafRunError::Empty);
        }
        let mut rest = bytes.as_slice();
        while !rest.is_empty() {
            let remaining = rest.len();
            let len = match record_head(&mut rest) {
                Ok(len) => len,
                Err(RecordHeadError::Head(source)) => {
                    return Err(LeafRunError::Head { remaining, source });
                }
                Err(RecordHeadError::NotARecord(detail)) => {
                    return Err(LeafRunError::NotARecord { remaining, detail });
                }
            };
            let Ok(len) = usize::try_from(len) else {
                return Err(LeafRunError::NotARecord {
                    remaining,
                    detail: "record exceeds the run byte cap",
                });
            };
            if rest.len() < len {
                return Err(LeafRunError::TruncatedRecord {
                    len,
                    remaining: rest.len(),
                });
            }
            rest = &rest[len..];
        }
        Ok(Self { bytes })
    }

    /// The number of records in this run.
    pub fn record_count(&self) -> usize {
        self.record_slices().count()
    }

    /// Iterate the run's records, decoding each into its canonical pair.
    pub fn records(
        &self,
        codec: PayloadCodec,
    ) -> impl Iterator<Item = Result<(Version, Message), DecodeLeafError>> {
        self.record_slices()
            .map(move |record| parse_record(record, codec))
    }

    /// Split the validated run back into its exact record contents.
    ///
    /// `pub(super)` for the capture renderer, which decodes each
    /// record's version structurally without knowing the leaf type.
    pub(super) fn record_slices(&self) -> RecordSlices<'_> {
        RecordSlices { rest: &self.bytes }
    }
}

/// Iterator over the exact record contents of a structurally valid run.
pub(super) struct RecordSlices<'a> {
    rest: &'a [u8],
}

impl<'a> Iterator for RecordSlices<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if self.rest.is_empty() {
            return None;
        }
        let len = record_head(&mut self.rest).expect("a validated run chains canonical records");
        let (record, rest) = self
            .rest
            .split_at(usize::try_from(len).expect("a validated record fits in memory"));
        self.rest = rest;
        Some(record)
    }
}

/// A record's leading heads were not a canonical embedded-sequence item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RecordHeadError {
    Head(HeadError),
    NotARecord(&'static str),
}

/// Parse one record's leading heads — the embedded-sequence tag and its
/// byte-string head — off the front of `input`, returning the record's
/// content length.
pub(super) fn record_head(input: &mut &[u8]) -> Result<u64, RecordHeadError> {
    let head = cbor::read_head(input).map_err(RecordHeadError::Head)?;
    if head.major != MAJOR_TAG || head.value != TAG_CBOR_SEQUENCE {
        return Err(RecordHeadError::NotARecord(
            "record does not open with the embedded-sequence tag",
        ));
    }
    let head = cbor::read_head(input).map_err(RecordHeadError::Head)?;
    if head.major != MAJOR_BSTR {
        return Err(RecordHeadError::NotARecord(
            "record tag does not wrap a byte string",
        ));
    }
    Ok(head.value)
}

/// Check a run body length against the wire's run byte cap.
///
/// The encoder's boundary: a run the cap rejects was necessarily a single
/// record (the budget saturates below the cap, so a multi-record run never
/// grows here), and [`LeafRun::push`] already rejected any such record —
/// this check is the belt to that suspender, priced identically.
pub(super) fn checked_run_len(len: usize) -> Result<u64, LengthOverflow> {
    // The cap is exactly the u32 range, so the failed conversion is the
    // overflow witness.
    match u32::try_from(len) {
        Ok(len) => Ok(u64::from(len)),
        Err(source) => Err(LengthOverflow { len, source }),
    }
}

/// Whether a run body of `len` bytes is exactly one record: the first
/// record's heads plus the content they declare span the body.
///
/// The lone-record test of the run-budget ingress check, shared by the
/// async reader and the sync oracle so the two decoders draw the
/// over-budget legality boundary identically. A body this predicate
/// rejects may also be structurally malformed; over budget, that
/// distinction is moot — either way the frame is not the one legal
/// overhang — so the check does not refine it further.
pub(super) fn lone_record_spans(len: usize, record_content: u64) -> bool {
    (RECORD_TAG_LEN as u64)
        .saturating_add(cbor::head_len(record_content) as u64)
        .saturating_add(record_content)
        == len as u64
}

/// Decode one exact record content into its canonical pair.
fn parse_record(record: &[u8], codec: PayloadCodec) -> Result<(Version, Message), DecodeLeafError> {
    // The version atom's tag is protocol vocabulary, read here by hand;
    // the byte string behind it and the payload are self-delimiting CBOR
    // values, so the exact record content parses without retrying, and
    // whatever the payload's parse does not consume is trailing.
    fn de_error(e: ciborium::de::Error<std::io::Error>) -> std::io::Error {
        match e {
            ciborium::de::Error::Io(e) => e,
            e => std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()),
        }
    }
    fn invalid(message: &str) -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::InvalidData, message.to_string())
    }
    let mut input = record;
    match cbor::read_head(&mut input) {
        Ok(head) if head.major == MAJOR_TAG && head.value == crate::tags::VERSION_TAG => {}
        Ok(_) => {
            return Err(DecodeLeafError::Version(invalid(
                "supplied version does not carry the version-atom tag",
            )));
        }
        // A record too short to hold the version's tag ran out of bytes,
        // the same class as a version cut mid-encoding.
        Err(HeadError::Truncated) => {
            return Err(DecodeLeafError::Version(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "record ends inside the version atom's tag",
            )));
        }
        Err(e) => return Err(DecodeLeafError::Version(invalid(&e.to_string()))),
    }
    let version: Version =
        ciborium::de::from_reader(&mut input).map_err(|e| DecodeLeafError::Version(de_error(e)))?;
    // The payload codec owns the payload parse, including the
    // exactly-one-value check the record framing otherwise cannot make
    // (the payload runs to the record's end), so trailing bytes surface
    // as its InvalidData.
    let message = Message::from_wire(bytes::Bytes::copy_from_slice(input), codec)
        .map_err(DecodeLeafError::Message)?;
    Ok((version, message))
}

/// A supply run whose record framing is structurally invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LeafRunError {
    /// Every supply frame carries at least one record.
    #[error("a supply run carries no leaf records")]
    Empty,
    /// A record's leading heads are truncated or non-canonical.
    #[error("a leaf record's heads are invalid in the {remaining} bytes left in its run: {source}")]
    Head {
        remaining: usize,
        #[source]
        source: HeadError,
    },
    /// The bytes where a record belongs are some other CBOR item.
    #[error("a {remaining}-byte run tail is not a leaf record: {detail}")]
    NotARecord {
        remaining: usize,
        detail: &'static str,
    },
    /// A record's content overruns the run's declared length.
    #[error("a leaf record of {len} bytes overruns the {remaining} bytes left in its run")]
    TruncatedRecord { len: usize, remaining: usize },
}

/// One structural problem in a child-listing map.
///
/// Every child listing entering from the wire — a query frame's body or
/// the greeting's root-fan listing — passes one structural gate, and
/// this names how a listing failed it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ListingIssue {
    /// A head was truncated, indefinite, reserved, or widened.
    #[error("{0}")]
    Head(HeadError),
    /// An item had the wrong major type, value range, or count.
    #[error("{0}")]
    Shape(&'static str),
    /// The digest bytes behind a value head were cut short.
    #[error("listing hash bytes are truncated")]
    Truncated,
    /// Adjacent keys were not strictly ascending.
    #[error("{0}")]
    Order(QueryOrderError),
}

/// Incrementally validated state of one child-listing map's entries.
///
/// This is the one gate every child listing entering from the wire
/// passes, whichever surface carries it — a query frame's body or the
/// greeting's root-fan listing — and whichever reader drives it (the
/// async decoder, the sync oracle, or the slice parser). The map's
/// deterministic-encoding key order and the wire's canonical child order
/// are one discipline: keys must be strictly ascending radixes, so an
/// equal adjacent pair is rejected exactly like a descent
/// ([`ListingIssue::Order`]).
pub(super) struct ListingBuilder {
    children: Vec<(u8, Hash)>,
    previous: Option<u8>,
}

impl ListingBuilder {
    /// Accept a map head of `count` entries within the radix space.
    pub(super) fn new(count: u64) -> Result<Self, ListingIssue> {
        if count > MAX_QUERY_CHILDREN as u64 {
            return Err(ListingIssue::Shape("listing exceeds the radix space"));
        }
        Ok(Self {
            children: Vec::with_capacity(count as usize),
            previous: None,
        })
    }

    /// Judge one entry's key head: an unsigned radix, strictly above the
    /// last recorded entry's.
    ///
    /// A pure check, so a reader may judge a key as soon as it arrives and
    /// again when its entry is whole; recording the entry
    /// ([`entry`](Self::entry)) is what advances the order.
    pub(super) fn key(&self, head: cbor::Head) -> Result<u8, ListingIssue> {
        if head.major != MAJOR_UINT || head.value > u64::from(u8::MAX) {
            return Err(ListingIssue::Shape("listing key is not a radix"));
        }
        let radix = head.value as u8;
        if let Some(previous) = self.previous
            && previous >= radix
        {
            return Err(ListingIssue::Order(QueryOrderError { previous, radix }));
        }
        Ok(radix)
    }

    /// Accept one entry's value head: a byte string of exactly one digest.
    pub(super) fn value_head(head: cbor::Head) -> Result<(), ListingIssue> {
        if head.major != MAJOR_BSTR || head.value != MERKLE_HASH_LEN as u64 {
            return Err(ListingIssue::Shape("listing value is not a Merkle hash"));
        }
        Ok(())
    }

    /// Record one entry whose key and value heads were accepted; later
    /// keys must ascend past its radix.
    pub(super) fn entry(&mut self, radix: u8, hash: [u8; MERKLE_HASH_LEN]) {
        self.children.push((radix, Hash(hash)));
        self.previous = Some(radix);
    }

    /// Yield the validated children.
    pub(super) fn finish(self) -> Vec<(u8, Hash)> {
        self.children
    }
}

/// Parse one complete child-listing map off the front of `input`,
/// advancing past it.
pub(crate) fn parse_listing_map(input: &mut &[u8]) -> Result<Vec<(u8, Hash)>, ListingIssue> {
    let head = cbor::read_head(input).map_err(ListingIssue::Head)?;
    if head.major != MAJOR_MAP {
        return Err(ListingIssue::Shape("listing is not a map"));
    }
    let count = head.value;
    let mut listing = ListingBuilder::new(count)?;
    for _ in 0..count {
        let key = cbor::read_head(input).map_err(ListingIssue::Head)?;
        let radix = listing.key(key)?;
        let value = cbor::read_head(input).map_err(ListingIssue::Head)?;
        ListingBuilder::value_head(value)?;
        if input.len() < MERKLE_HASH_LEN {
            return Err(ListingIssue::Truncated);
        }
        let (digest, rest) = input.split_at(MERKLE_HASH_LEN);
        *input = rest;
        listing.entry(radix, digest.try_into().expect("split at the digest width"));
    }
    Ok(listing.finish())
}

/// Append one child listing as a canonical map: ascending radix keys,
/// each hash a definite-length byte string.
///
/// The encoder is not a trust boundary — callers guarantee canonical
/// child order — so this writes without revalidating it.
pub(crate) fn write_listing(out: &mut Vec<u8>, children: &[(u8, Hash)]) {
    cbor::write_head(out, MAJOR_MAP, children.len() as u64);
    for (radix, hash) in children {
        cbor::write_head(out, MAJOR_UINT, u64::from(*radix));
        cbor::write_head(out, MAJOR_BSTR, MERKLE_HASH_LEN as u64);
        out.extend_from_slice(hash.as_bytes());
    }
}

/// Bytes a whole child listing occupies as a map: its head plus entries.
#[cfg(test)]
pub fn listing_len(children: &[(u8, Hash)]) -> usize {
    let mut total = cbor::head_len(children.len() as u64);
    for (radix, _) in children {
        total += listing_entry_len(*radix);
    }
    total
}

#[cfg(test)]
mod tests;
