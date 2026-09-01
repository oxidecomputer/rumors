//! Canonical frame encoding.

#[cfg(test)]
use std::io::Write;

use crate::tree::mirror::cbor::{self, MAJOR_ARRAY, MAJOR_BSTR, MAJOR_UINT, TAG_CBOR_SEQUENCE};

mod async_io;

pub use async_io::FrameWrite;

use super::{
    error::EncodeErrorKind,
    frame::{Frame, LeafRun, Reaction, listing_len, write_listing},
    signal::{Signal, Stream, WireSignal},
};
#[cfg(test)]
use super::{
    error::{EncodeError, FramePart},
    frame::WireFrame,
    signal::Speaker,
};

/// Bytes a frame head occupies at its widest: the array head of a one- or
/// two-item frame, then the widest signal head.
const FRAME_HEAD_LEN: usize = cbor::head_len(2) + WireSignal::MAX_ENCODED_LEN;

/// Bytes a supply body's heads occupy at their widest: the
/// embedded-sequence tag, then the byte-string head of a run at the
/// wire's run byte cap.
const SUPPLY_HEAD_LEN: usize = cbor::head_len(TAG_CBOR_SEQUENCE) + cbor::head_len(u32::MAX as u64);

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
/// Every frame carries a few head bytes of known maximum width; rendering
/// them here, rather than into a heap buffer, keeps the frames a session
/// writes most often — the body-free reactions — free of allocation.
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
/// run record framing are guaranteed by its callers and checked only when
/// bytes enter from the wire. Construction performs only the
/// representational checks needed before any byte can be emitted, and
/// renders every head — the fixed ones on the stack — so the write paths
/// move bytes without measuring anything.
struct FrameEncoding<'a> {
    /// The frame's array head and signal head.
    head: Heads<FRAME_HEAD_LEN>,
    body: BodyEncoding<'a>,
}

enum BodyEncoding<'a> {
    Empty,
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
            Frame::Reaction(Reaction::Match, flow) => (Signal::Match(*flow), BodyEncoding::Empty),
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
            BodyEncoding::Empty => 1,
            BodyEncoding::Listing(_) | BodyEncoding::Supply { .. } => 2,
        };
        let mut head = Heads::new();
        head.put(MAJOR_ARRAY, arity);
        head.put(MAJOR_UINT, u64::from(WireSignal::encode(stream, signal)));
        Ok(Self { head, body })
    }

    /// Render the whole frame into one contiguous buffer: byte for byte
    /// what the piece-wise writers emit, materialized only for an
    /// attached observer's one-item view.
    fn to_vec(&self) -> Vec<u8> {
        let body_len = match &self.body {
            BodyEncoding::Empty => 0,
            BodyEncoding::Listing(listing) => listing.len(),
            BodyEncoding::Supply { head, run } => head.as_slice().len() + run.as_bytes().len(),
        };
        let mut bytes = Vec::with_capacity(self.head.as_slice().len() + body_len);
        bytes.extend_from_slice(self.head.as_slice());
        match &self.body {
            BodyEncoding::Empty => {}
            BodyEncoding::Listing(listing) => bytes.extend_from_slice(listing),
            BodyEncoding::Supply { head, run } => {
                bytes.extend_from_slice(head.as_slice());
                bytes.extend_from_slice(run.as_bytes());
            }
        }
        bytes
    }

    #[cfg(test)]
    fn write(&self, out: &mut impl Write) -> Result<(), EncodeErrorKind> {
        write(out, FramePart::FrameHead, self.head.as_slice())?;
        match &self.body {
            BodyEncoding::Empty => {}
            BodyEncoding::Listing(listing) => {
                write(out, FramePart::QueryChildren, listing)?;
            }
            BodyEncoding::Supply { head, run } => {
                write(out, FramePart::SupplyLength, head.as_slice())?;
                write(out, FramePart::SupplyRun, run.as_bytes())?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
fn write(out: &mut impl Write, part: FramePart, bytes: &[u8]) -> Result<(), EncodeErrorKind> {
    out.write_all(bytes)
        .map_err(|source| EncodeErrorKind::Write { part, source })
}

#[cfg(test)]
mod tests;
