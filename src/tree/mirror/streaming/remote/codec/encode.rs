//! Canonical frame encoding.

#[cfg(test)]
use std::io::Write;

use crate::counter::widen;
use crate::tree::mirror::cbor::{self, MAJOR_ARRAY, MAJOR_BSTR, MAJOR_UINT, TAG_CBOR_SEQUENCE};

mod async_io;

#[cfg(any(test, feature = "test-internals"))]
pub use async_io::CUTS_TO_GROW;
pub use async_io::FrameWrite;

#[cfg(test)]
use super::{error::EncodeError, frame::WireFrame, signal::Speaker};
use crate::tree::mirror::streaming::byte_set::{Encoded, MAX_ENCODED_LEN};

use super::{
    error::{EncodeErrorKind, FramePart},
    frame::{Frame, LeafRun, Reaction, SUPPLY_HEAD_LEN, listing_len, write_listing},
    signal::{FRAME_OPENER_LEN, Signal, Stream},
};

/// Append `wire`'s canonical representation to `out`.
#[cfg(test)]
pub fn encode<W: Write>(
    speaker: Speaker,
    wire: &WireFrame,
    out: &mut W,
) -> Result<(), EncodeError> {
    let (stream, frame) = wire;
    FrameEncoding::new(*stream, frame)
        .and_then(|encoding| encoding.write(out))
        .map_err(|kind| EncodeError::new(speaker, *stream, kind))
}

/// A run of rendered heads held on the stack.
///
/// Every frame carries a few head bytes of known maximum width; rendering them
/// here, rather than into a heap buffer, keeps the frames a session writes most
/// often — the body-free reactions — free of allocation.
struct Heads<const N: usize> {
    bytes: [u8; N],
    len: usize,
}

impl<const N: usize> Heads<N> {
    const fn new() -> Self {
        Self {
            bytes: [0; N],
            len: 0,
        }
    }

    /// Append the shortest-form head `(major, value)`.
    ///
    /// The capacity is the grammar's maximum for the heads it holds, so
    /// exceeding it is a programmer error, not an input.
    fn put(&mut self, major: u8, value: u64) {
        let head = cbor::render_head(major, value);
        let end = self.len + head.as_slice().len();
        self.bytes[self.len..end].copy_from_slice(head.as_slice());
        self.len = end;
    }

    fn as_slice(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

/// A protocol-produced frame split into directly writable pieces.
///
/// The encoder is not a trust boundary: phase placement, query ordering, and
/// run record framing are guaranteed by its callers and checked only when bytes
/// enter from the wire. Construction performs only the representational checks
/// needed before any byte can be emitted, and renders every head — the fixed
/// ones on the stack — so the write paths move bytes without measuring
/// anything.
struct FrameEncoding<'a> {
    /// The frame's opener: its array head, stream item, and state item.
    head: Heads<FRAME_OPENER_LEN>,
    body: BodyEncoding<'a>,
}

enum BodyEncoding<'a> {
    Empty,
    /// A match's later matches: the byte-string head and the set's encoding,
    /// fully rendered on the stack.
    MatchSet {
        head: Heads<MATCH_SET_HEAD_LEN>,
        set: Encoded,
    },
    /// A nonempty query's child-listing map, fully rendered.
    Listing(Vec<u8>),
    /// A supply run behind its rendered embedded-sequence head.
    Supply {
        head: Heads<SUPPLY_HEAD_LEN>,
        run: &'a LeafRun,
    },
}

impl<'a> FrameEncoding<'a> {
    fn new(stream: Stream, frame: &'a Frame) -> Result<Self, EncodeErrorKind> {
        let (signal, body) = match frame {
            Frame::Reaction(Reaction::Match(later), flow) => {
                let body = match later.get() {
                    None => BodyEncoding::Empty,
                    Some(set) => {
                        let set = set.encoded();
                        let mut head = Heads::new();
                        head.put(MAJOR_BSTR, widen(set.as_bytes().len()));
                        BodyEncoding::MatchSet { head, set }
                    }
                };
                (Signal::Match(*flow), body)
            }
            Frame::Reaction(Reaction::Query(children), flow) if children.is_empty() => {
                (Signal::QueryEmpty(*flow), BodyEncoding::Empty)
            }
            Frame::Reaction(Reaction::Query(children), flow) => {
                let mut listing = Vec::with_capacity(listing_len(children));
                write_listing(&mut listing, children);
                (Signal::Query(*flow), BodyEncoding::Listing(listing))
            }
            Frame::Reaction(Reaction::Supply(run), flow) => {
                let len = super::frame::checked_run_len(run.encoded_len())?;
                let mut head = Heads::new();
                head.put(cbor::MAJOR_TAG, TAG_CBOR_SEQUENCE);
                head.put(MAJOR_BSTR, len);
                (Signal::Supply(*flow), BodyEncoding::Supply { head, run })
            }
            Frame::End(end) => (Signal::End(*end), BodyEncoding::Empty),
        };
        let arity = match &body {
            BodyEncoding::Empty => 2,
            BodyEncoding::MatchSet { .. }
            | BodyEncoding::Listing(_)
            | BodyEncoding::Supply { .. } => 3,
        };
        let mut head = Heads::new();
        head.put(MAJOR_ARRAY, arity);
        head.put(MAJOR_UINT, u64::from(stream.index()));
        head.put(MAJOR_UINT, u64::from(signal.state()));
        Ok(Self { head, body })
    }

    /// The frame's pieces in wire order, each with the part it carries.
    ///
    /// They are the opener, then a match's set head and encoding, the listing,
    /// or the supply run's head and its records. Every piece is nonempty, and
    /// there are at most [`MAX_PIECES`].
    fn pieces(&self) -> impl Iterator<Item = (FramePart, &[u8])> {
        let body = match &self.body {
            BodyEncoding::Empty => [None, None],
            BodyEncoding::MatchSet { head, set } => [
                Some((FramePart::MatchSet, head.as_slice())),
                Some((FramePart::MatchSet, set.as_bytes())),
            ],
            BodyEncoding::Listing(listing) => {
                [Some((FramePart::QueryChildren, &listing[..])), None]
            }
            BodyEncoding::Supply { head, run } => [
                Some((FramePart::SupplyLength, head.as_slice())),
                Some((FramePart::SupplyRun, run.as_bytes())),
            ],
        };
        std::iter::once((FramePart::FrameHead, self.head.as_slice()))
            .chain(body.into_iter().flatten())
    }

    /// The frame's encoded length in bytes: the sum of its pieces.
    fn len(&self) -> usize {
        self.pieces().map(|(_, piece)| piece.len()).sum()
    }

    /// Render the whole frame into one contiguous buffer: byte for byte what
    /// the writers emit, materialized only for an attached observer's one-item
    /// view.
    fn to_vec(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.len());
        for (_, piece) in self.pieces() {
            bytes.extend_from_slice(piece);
        }
        bytes
    }

    #[cfg(test)]
    fn write(&self, out: &mut impl Write) -> Result<(), EncodeErrorKind> {
        for (part, piece) in self.pieces() {
            out.write_all(piece)
                .map_err(|source| EncodeErrorKind::Write { part, source })?;
        }
        Ok(())
    }
}

/// The most pieces one frame has: a supply frame's opener, run head, and
/// records, or a match's opener, set head, and set.
const MAX_PIECES: usize = 3;

/// The widest byte-string head ahead of a match's set: one for the longest
/// encoding.
const MATCH_SET_HEAD_LEN: usize = cbor::head_len(widen(MAX_ENCODED_LEN));

#[cfg(test)]
mod tests;
