//! Canonical frame encoding.

#[cfg(test)]
use std::io::Write;

use crate::tree::mirror::cbor::{self, MAJOR_ARRAY, MAJOR_BSTR, MAJOR_UINT, TAG_CBOR_SEQUENCE};

mod async_io;

pub use async_io::FrameWrite;

use super::{
    error::EncodeErrorKind,
    frame::{Frame, LeafRun, Reaction, write_listing},
    signal::{Signal, Stream, WireSignal},
};
#[cfg(test)]
use super::{
    error::{EncodeError, FramePart},
    frame::WireFrame,
    signal::Speaker,
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

/// A protocol-produced frame split into directly writable pieces.
///
/// The encoder is not a trust boundary: phase placement, query ordering, and
/// run record framing are guaranteed by its callers and checked only when
/// bytes enter from the wire. Construction performs only the
/// representational checks needed before any byte can be emitted, and
/// renders every head — so the write paths move bytes without measuring
/// anything.
struct FrameEncoding<'a> {
    /// The frame's array head and signal head.
    head: Vec<u8>,
    body: BodyEncoding<'a>,
}

enum BodyEncoding<'a> {
    Empty,
    /// A nonempty query's child-listing map, fully rendered.
    Listing(Vec<u8>),
    /// A supply run behind its rendered embedded-sequence head.
    Supply {
        head: Vec<u8>,
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
                let mut listing = Vec::new();
                write_listing(&mut listing, children);
                (Signal::Query(*flow), BodyEncoding::Listing(listing))
            }
            Frame::Reaction(Reaction::Supply(run), flow) => {
                let len = super::frame::checked_run_len(run.encoded_len())?;
                let mut head = Vec::new();
                cbor::write_tag(&mut head, TAG_CBOR_SEQUENCE);
                cbor::write_head(&mut head, MAJOR_BSTR, len);
                (Signal::Supply(*flow), BodyEncoding::Supply { head, run })
            }
            Frame::End(end) => (Signal::End(*end), BodyEncoding::Empty),
        };
        let arity = match &body {
            BodyEncoding::Empty => 1,
            BodyEncoding::Listing(_) | BodyEncoding::Supply { .. } => 2,
        };
        let mut head = Vec::new();
        cbor::write_head(&mut head, MAJOR_ARRAY, arity);
        cbor::write_head(
            &mut head,
            MAJOR_UINT,
            u64::from(WireSignal::encode(stream, signal)),
        );
        Ok(Self { head, body })
    }

    #[cfg(test)]
    fn write(&self, out: &mut impl Write) -> Result<(), EncodeErrorKind> {
        write(out, FramePart::FrameHead, &self.head)?;
        match &self.body {
            BodyEncoding::Empty => {}
            BodyEncoding::Listing(listing) => {
                write(out, FramePart::QueryChildren, listing)?;
            }
            BodyEncoding::Supply { head, run } => {
                write(out, FramePart::SupplyLength, head)?;
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
