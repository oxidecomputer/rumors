//! Canonical frame encoding.

use borsh::{BorshSerialize, io::Write};

use crate::{
    Version,
    message::Message,
    tree::{
        mirror::framing::{LENGTH_HEADER_LEN, length_header},
        typed::Hash,
    },
};

mod async_io;

pub use async_io::FrameWrite;

use super::{
    error::{EncodeError, EncodeErrorKind, EncodeLeafError, FramePart},
    frame::{Frame, QUERY_COUNT_BIAS, Reaction, WireFrame},
    signal::{Signal, Speaker, Stream, WireSignal},
};

/// Append `wire`'s canonical representation to `out`.
pub fn encode<T, W: Write>(
    speaker: Speaker,
    wire: &WireFrame<T>,
    out: &mut W,
) -> Result<(), EncodeError> {
    let (stream, frame) = wire;
    FrameEncoding::new(*stream, frame)
        .and_then(|encoding| encoding.write(out))
        .map_err(|kind| EncodeError::new(speaker, *stream, kind))
}

/// A protocol-produced frame split into directly writable pieces.
///
/// The encoder is not a trust boundary: phase placement and query ordering are
/// guaranteed by its callers and checked only when bytes enter from the wire.
/// Construction performs only the representational checks needed before any
/// byte can be emitted.
struct FrameEncoding<'a, T> {
    signal: [u8; WireSignal::ENCODED_LEN],
    body: BodyEncoding<'a, T>,
}

enum BodyEncoding<'a, T> {
    Empty,
    Query {
        count: [u8; 1],
        children: &'a [(u8, Hash)],
    },
    Supply {
        header: [u8; LENGTH_HEADER_LEN],
        version: &'a Version,
        message: &'a Message<T>,
    },
}

impl<'a, T> FrameEncoding<'a, T> {
    fn new(stream: Stream, frame: &'a Frame<T>) -> Result<Self, EncodeErrorKind> {
        let (signal, body) = match frame {
            Frame::Reaction(Reaction::Match, flow) => (Signal::Match(*flow), BodyEncoding::Empty),
            Frame::Reaction(Reaction::Query(children), flow) if children.is_empty() => {
                (Signal::QueryEmpty(*flow), BodyEncoding::Empty)
            }
            Frame::Reaction(Reaction::Query(children), flow) => {
                let count = u8::try_from(children.len() - QUERY_COUNT_BIAS)
                    .expect("a protocol query never exceeds the radix fan");
                (
                    Signal::Query(*flow),
                    BodyEncoding::Query {
                        count: [count],
                        children,
                    },
                )
            }
            Frame::Reaction(Reaction::Supply(version, message), flow) => {
                let version_len = version.as_bytes().len();
                let message_len = message.as_slice().len();
                let len = version_len.checked_add(message_len).ok_or(
                    EncodeErrorKind::SupplyLengthOverflow {
                        version_len,
                        message_len,
                    },
                )?;
                let header = length_header(len)?;
                (
                    Signal::Supply(*flow),
                    BodyEncoding::Supply {
                        header,
                        version,
                        message,
                    },
                )
            }
            Frame::End(end) => (Signal::End(*end), BodyEncoding::Empty),
        };
        let signal = [WireSignal::encode(stream, signal)];
        Ok(Self { signal, body })
    }

    fn write(&self, out: &mut impl Write) -> Result<(), EncodeErrorKind> {
        write(out, FramePart::Signal, &self.signal)?;
        match &self.body {
            BodyEncoding::Empty => {}
            BodyEncoding::Query { count, children } => {
                write(out, FramePart::QueryCount, count)?;
                for (radix, hash) in *children {
                    write(out, FramePart::QueryChildren, std::slice::from_ref(radix))?;
                    write(out, FramePart::QueryChildren, hash.as_bytes())?;
                }
            }
            BodyEncoding::Supply {
                header,
                version,
                message,
            } => {
                write(out, FramePart::SupplyLength, header)?;
                version
                    .serialize(&mut *out)
                    .map_err(EncodeLeafError::Version)?;
                message
                    .serialize(&mut *out)
                    .map_err(EncodeLeafError::Message)?;
            }
        }
        Ok(())
    }
}

fn write(out: &mut impl Write, part: FramePart, bytes: &[u8]) -> Result<(), EncodeErrorKind> {
    out.write_all(bytes)
        .map_err(|source| EncodeErrorKind::Write { part, source })
}

#[cfg(test)]
mod tests;
