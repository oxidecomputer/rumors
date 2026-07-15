//! Self-delimiting frame decoding.

use borsh::{
    BorshDeserialize,
    io::{ErrorKind, Read},
};

use crate::{
    Version,
    message::Message,
    tree::typed::{Hash, hash::MERKLE_HASH_LEN},
};

use super::{
    error::{DecodeError, DecodeErrorKind, DecodeLeafError, FramePart},
    frame::{Frame, Reaction, WireFrame, validate_children},
    signal::{Signal, Speaker, WireSignal},
};

/// Decode one frame from `read`, leaving subsequent bytes untouched.
pub fn decode<T: BorshDeserialize>(
    speaker: Speaker,
    read: &mut impl Read,
) -> Result<WireFrame<T>, DecodeError> {
    let mut byte = [0];
    read_exact(read, &mut byte, FramePart::Signal)
        .map_err(|kind| DecodeError::direction(speaker, kind))?;
    let wire = WireSignal::from_byte(byte[0]).map_err(|invalid| {
        DecodeError::stream(
            speaker,
            invalid.stream(),
            DecodeErrorKind::UnknownSignal {
                signal: invalid.byte(),
            },
        )
    })?;
    let (stream, signal) = wire.into_parts();

    decode_frame(read, signal)
        .map(|frame| (stream, frame))
        .map_err(|kind| DecodeError::stream(speaker, stream, kind))
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

fn decode_frame<T: BorshDeserialize>(
    read: &mut impl Read,
    signal: Signal,
) -> Result<Frame<T>, DecodeErrorKind> {
    match signal {
        Signal::Match(flow) => Ok(Frame::Reaction(Reaction::Match, flow)),
        Signal::QueryEmpty(flow) => Ok(Frame::Reaction(Reaction::Query(Vec::new()), flow)),
        Signal::Query(flow) => decode_query(read).map(|reaction| Frame::Reaction(reaction, flow)),
        Signal::Supply(flow) => decode_supply(read).map(|reaction| Frame::Reaction(reaction, flow)),
        Signal::End(end) => Ok(Frame::End(end)),
    }
}

fn decode_query<T>(read: &mut impl Read) -> Result<Reaction<T>, DecodeErrorKind> {
    let mut count = [0];
    read_exact(read, &mut count, FramePart::QueryCount)?;
    let count = usize::from(count[0]) + 1;
    let mut listing = vec![0; count * (1 + MERKLE_HASH_LEN)];
    read_exact(read, &mut listing, FramePart::QueryChildren)?;

    let mut children = Vec::with_capacity(count);
    for record in listing.chunks_exact(1 + MERKLE_HASH_LEN) {
        let radix = record[0];
        let mut hash = [0; MERKLE_HASH_LEN];
        hash.copy_from_slice(&record[1..]);
        children.push((radix, Hash(hash)));
    }
    validate_children(&children)?;
    Ok(Reaction::Query(children))
}

fn decode_supply<T: BorshDeserialize>(
    read: &mut impl Read,
) -> Result<Reaction<T>, DecodeErrorKind> {
    let mut header = [0; 4];
    read_exact(read, &mut header, FramePart::SupplyLength)?;
    let len = u32::from_be_bytes(header) as usize;
    let mut leaf = vec![0; len];
    read_exact(read, &mut leaf, FramePart::SupplyLeaf)?;

    let mut leaf = leaf.as_slice();
    let version = Version::deserialize(&mut leaf).map_err(DecodeLeafError::Version)?;
    let message = Message::<T>::deserialize(&mut leaf).map_err(DecodeLeafError::Message)?;
    if !leaf.is_empty() {
        return Err(DecodeLeafError::TrailingBytes.into());
    }
    Ok(Reaction::Supply(version, message))
}

fn read_exact(
    read: &mut impl Read,
    bytes: &mut [u8],
    part: FramePart,
) -> Result<(), DecodeErrorKind> {
    read.read_exact(bytes).map_err(|source| {
        if source.kind() == ErrorKind::UnexpectedEof {
            DecodeErrorKind::Truncated { missing: part }
        } else {
            DecodeErrorKind::Read { part, source }
        }
    })
}

#[cfg(test)]
mod tests;
