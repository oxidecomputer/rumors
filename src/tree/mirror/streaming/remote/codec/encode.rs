//! Canonical frame encoding.

use borsh::{BorshSerialize, io::Write};

use crate::tree::mirror::framing::length_header;

use super::{
    error::{EncodeError, EncodeErrorKind, EncodeLeafError, FramePart},
    frame::{Frame, MAX_QUERY_CHILDREN, QUERY_COUNT_BIAS, Reaction, WireFrame, validate_children},
    signal::{Signal, Speaker, Stream, WireSignal},
};

/// Append `wire`'s canonical representation to `out`.
pub fn encode<T, W: Write>(
    speaker: Speaker,
    wire: &WireFrame<T>,
    out: &mut W,
) -> Result<(), EncodeError> {
    let (stream, frame) = wire;
    encode_frame(frame, *stream, out).map_err(|kind| EncodeError::new(speaker, *stream, kind))
}

fn encode_frame<T, W: Write>(
    frame: &Frame<T>,
    stream: Stream,
    out: &mut W,
) -> Result<(), EncodeErrorKind> {
    match frame {
        Frame::Reaction(Reaction::Match, flow) => {
            write(
                out,
                FramePart::Signal,
                &[WireSignal::new(stream, Signal::Match(*flow)).to_byte()],
            )?;
        }
        Frame::Reaction(Reaction::Query(children), flow) if children.is_empty() => {
            write(
                out,
                FramePart::Signal,
                &[WireSignal::new(stream, Signal::QueryEmpty(*flow)).to_byte()],
            )?;
        }
        Frame::Reaction(Reaction::Query(children), flow) => {
            if children.len() > MAX_QUERY_CHILDREN {
                return Err(EncodeErrorKind::QueryTooWide {
                    count: children.len(),
                });
            }
            let count = u8::try_from(children.len() - QUERY_COUNT_BIAS)
                .expect("a query within the protocol fan has a one-byte count");
            validate_children(children)?;
            write(
                out,
                FramePart::Signal,
                &[WireSignal::new(stream, Signal::Query(*flow)).to_byte()],
            )?;
            write(out, FramePart::QueryCount, &[count])?;
            for (radix, hash) in children {
                write(out, FramePart::QueryChildren, &[*radix])?;
                write(out, FramePart::QueryChildren, hash.as_bytes())?;
            }
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
            write(
                out,
                FramePart::Signal,
                &[WireSignal::new(stream, Signal::Supply(*flow)).to_byte()],
            )?;
            write(out, FramePart::SupplyLength, &header)?;
            version
                .serialize(&mut *out)
                .map_err(EncodeLeafError::Version)?;
            message
                .serialize(&mut *out)
                .map_err(EncodeLeafError::Message)?;
        }
        Frame::End(end) => write(
            out,
            FramePart::Signal,
            &[WireSignal::new(stream, Signal::End(*end)).to_byte()],
        )?,
    }
    Ok(())
}

fn write(out: &mut impl Write, part: FramePart, bytes: &[u8]) -> Result<(), EncodeErrorKind> {
    out.write_all(bytes)
        .map_err(|source| EncodeErrorKind::Write { part, source })
}

#[cfg(test)]
mod tests;
