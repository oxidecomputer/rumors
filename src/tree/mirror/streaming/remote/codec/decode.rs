//! Self-delimiting frame decoding.

#[cfg(test)]
use std::slice;

use borsh::BorshDeserialize;
#[cfg(test)]
use borsh::io::{ErrorKind, Read};

#[cfg(test)]
use crate::tree::mirror::framing::LENGTH_HEADER_LEN;
use crate::{
    Version,
    message::Message,
    tree::typed::{Hash, hash::MERKLE_HASH_LEN},
};

mod async_io;

pub use async_io::FrameRead;

#[cfg(test)]
use super::{
    error::FramePart,
    frame::{Frame, QUERY_COUNT_BIAS, Reaction, WireFrame},
};
use super::{
    error::{DecodeError, DecodeErrorKind, DecodeLeafError},
    frame::{QUERY_CHILD_LEN, validate_children},
    signal::{Signal, Speaker, Stream, WireSignal},
};

/// Decode one frame from `read`, leaving subsequent bytes untouched.
#[cfg(test)]
pub fn decode<T: BorshDeserialize>(
    speaker: Speaker,
    read: &mut impl Read,
) -> Result<WireFrame<T>, DecodeError> {
    FrameDecoder::new(speaker, read).decode()
}

/// Decode exactly one frame from a slice, rejecting bytes after it.
#[cfg(test)]
pub fn decode_exact<T: BorshDeserialize>(
    speaker: Speaker,
    input: &[u8],
) -> Result<WireFrame<T>, DecodeError> {
    let mut rest = input;
    let (stream, frame) = decode(speaker, &mut rest)?;
    if rest.is_empty() {
        Ok((stream, frame))
    } else {
        Err(DecodeError::stream(
            speaker,
            stream,
            DecodeErrorKind::TrailingBytes { count: rest.len() },
        ))
    }
}

/// Frame reader that adds protocol context as soon as the signal reveals it.
#[cfg(test)]
struct FrameDecoder<'a, R> {
    speaker: Speaker,
    read: &'a mut R,
}

#[cfg(test)]
impl<'a, R: Read> FrameDecoder<'a, R> {
    fn new(speaker: Speaker, read: &'a mut R) -> Self {
        Self { speaker, read }
    }

    fn decode<T: BorshDeserialize>(mut self) -> Result<WireFrame<T>, DecodeError> {
        let (stream, signal) = self.signal()?;
        let frame = self
            .body(signal)
            .map_err(|kind| DecodeError::stream(self.speaker, stream, kind))?;
        Ok((stream, frame))
    }

    fn signal(&mut self) -> Result<(Stream, Signal), DecodeError> {
        let byte = self
            .byte(FramePart::Signal)
            .map_err(|kind| DecodeError::direction(self.speaker, kind))?;
        decode_signal(self.speaker, byte)
    }

    fn body<T: BorshDeserialize>(&mut self, signal: Signal) -> Result<Frame<T>, DecodeErrorKind> {
        let frame = match signal {
            Signal::Match(flow) => Frame::Reaction(Reaction::Match, flow),
            Signal::QueryEmpty(flow) => Frame::Reaction(Reaction::Query(Vec::new()), flow),
            Signal::Query(flow) => Frame::Reaction(Reaction::Query(self.query()?), flow),
            Signal::Supply(flow) => {
                let (version, message) = self.supply()?;
                Frame::Reaction(Reaction::Supply(version, message), flow)
            }
            Signal::End(end) => Frame::End(end),
        };
        Ok(frame)
    }

    fn query(&mut self) -> Result<Vec<(u8, Hash)>, DecodeErrorKind> {
        let count = usize::from(self.byte(FramePart::QueryCount)?) + QUERY_COUNT_BIAS;
        // Preserve one bulk read for the whole listing rather than one call per child.
        let mut listing = vec![0; count * QUERY_CHILD_LEN];
        self.read_exact(&mut listing, FramePart::QueryChildren)?;

        parse_query(&listing)
    }

    fn supply<T: BorshDeserialize>(&mut self) -> Result<(Version, Message<T>), DecodeErrorKind> {
        let mut header = [0; LENGTH_HEADER_LEN];
        self.read_exact(&mut header, FramePart::SupplyLength)?;
        let mut leaf = vec![0; u32::from_be_bytes(header) as usize];
        self.read_exact(&mut leaf, FramePart::SupplyLeaf)?;

        parse_supply(&leaf)
    }

    fn byte(&mut self, part: FramePart) -> Result<u8, DecodeErrorKind> {
        let mut byte = 0;
        self.read_exact(slice::from_mut(&mut byte), part)?;
        Ok(byte)
    }

    fn read_exact(&mut self, bytes: &mut [u8], part: FramePart) -> Result<(), DecodeErrorKind> {
        self.read
            .read_exact(bytes)
            .map_err(|source| match source.kind() {
                ErrorKind::UnexpectedEof => DecodeErrorKind::Truncated {
                    missing: part,
                    source,
                },
                _ => DecodeErrorKind::Read { part, source },
            })
    }
}

fn decode_signal(speaker: Speaker, byte: u8) -> Result<(Stream, Signal), DecodeError> {
    let wire = WireSignal::from_byte(speaker, byte)
        .map_err(|invalid| DecodeError::stream(speaker, invalid.stream(), invalid.into()))?;
    Ok(wire.into_parts())
}

fn parse_query(listing: &[u8]) -> Result<Vec<(u8, Hash)>, DecodeErrorKind> {
    let mut children = Vec::with_capacity(listing.len() / QUERY_CHILD_LEN);
    for record in listing.chunks_exact(QUERY_CHILD_LEN) {
        let (&radix, encoded_hash) = record
            .split_first()
            .expect("a query child record contains its radix");
        let mut hash = [0; MERKLE_HASH_LEN];
        hash.copy_from_slice(encoded_hash);
        children.push((radix, Hash(hash)));
    }
    validate_children(&children)?;
    Ok(children)
}

fn parse_supply<T: BorshDeserialize>(
    leaf: &[u8],
) -> Result<(Version, Message<T>), DecodeErrorKind> {
    // The exact body makes both Borsh values a single, non-retrying parse.
    let mut input = leaf;
    let version = Version::deserialize(&mut input).map_err(DecodeLeafError::Version)?;
    let message = Message::deserialize(&mut input).map_err(DecodeLeafError::Message)?;
    if !input.is_empty() {
        return Err(DecodeLeafError::TrailingBytes { count: input.len() }.into());
    }
    Ok((version, message))
}

#[cfg(test)]
mod tests;
