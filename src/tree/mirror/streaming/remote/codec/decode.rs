//! Self-delimiting frame decoding.

use std::slice;

use borsh::{
    BorshDeserialize,
    io::{ErrorKind, Read},
};

use crate::{
    Version,
    message::Message,
    tree::{
        mirror::framing::LENGTH_HEADER_LEN,
        typed::{Hash, hash::MERKLE_HASH_LEN},
    },
};

use super::{
    error::{DecodeError, DecodeErrorKind, DecodeLeafError, FramePart},
    frame::{Frame, QUERY_CHILD_LEN, QUERY_COUNT_BIAS, Reaction, WireFrame, validate_children},
    signal::{Signal, Speaker, Stream, WireSignal},
};

/// Decode one frame from `read`, leaving subsequent bytes untouched.
pub fn decode<T: BorshDeserialize>(
    speaker: Speaker,
    read: &mut impl Read,
) -> Result<WireFrame<T>, DecodeError> {
    FrameDecoder::new(speaker, read).decode()
}

/// Decode exactly one frame from a slice, rejecting bytes after it.
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
struct FrameDecoder<'a, R> {
    speaker: Speaker,
    read: &'a mut R,
}

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
        let wire = WireSignal::from_byte(self.speaker, byte).map_err(|invalid| {
            DecodeError::stream(self.speaker, invalid.stream(), invalid.into())
        })?;
        Ok(wire.into_parts())
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

        let mut children = Vec::with_capacity(count);
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

    fn supply<T: BorshDeserialize>(&mut self) -> Result<(Version, Message<T>), DecodeErrorKind> {
        let mut header = [0; LENGTH_HEADER_LEN];
        self.read_exact(&mut header, FramePart::SupplyLength)?;
        let mut leaf = vec![0; u32::from_be_bytes(header) as usize];
        self.read_exact(&mut leaf, FramePart::SupplyLeaf)?;

        // The exact body makes both Borsh values a single, non-retrying parse.
        let mut input = leaf.as_slice();
        let version = Version::deserialize(&mut input).map_err(DecodeLeafError::Version)?;
        let message = Message::deserialize(&mut input).map_err(DecodeLeafError::Message)?;
        if !input.is_empty() {
            return Err(DecodeLeafError::TrailingBytes { count: input.len() }.into());
        }
        Ok((version, message))
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

#[cfg(test)]
mod tests;
