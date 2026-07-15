//! Self-delimiting frame decoding.

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
    frame::{
        Frame, QUERY_CHILD_LEN, QUERY_COUNT_BIAS, QUERY_COUNT_LEN, Reaction, WireFrame,
        validate_children,
    },
    signal::{Signal, Speaker, WireSignal},
};

/// Decode one frame from `read`, leaving subsequent bytes untouched.
pub fn decode<T: BorshDeserialize>(
    speaker: Speaker,
    read: &mut impl Read,
) -> Result<WireFrame<T>, DecodeError> {
    let mut byte = [0; WireSignal::ENCODED_LEN];
    read_exact(read, &mut byte, FramePart::Signal)
        .map_err(|kind| DecodeError::direction(speaker, kind))?;
    let signal_byte = *byte.first().expect("a signal occupies one byte");
    let wire = WireSignal::from_byte(signal_byte)
        .map_err(|invalid| DecodeError::stream(speaker, invalid.stream(), invalid.into()))?;
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
    let mut count = [0; QUERY_COUNT_LEN];
    read_exact(read, &mut count, FramePart::QueryCount)?;
    let encoded_count = *count.first().expect("a query count occupies one byte");
    let count = usize::from(encoded_count) + QUERY_COUNT_BIAS;
    let mut listing = vec![0; count * QUERY_CHILD_LEN];
    read_exact(read, &mut listing, FramePart::QueryChildren)?;

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
    Ok(Reaction::Query(children))
}

fn decode_supply<T: BorshDeserialize>(
    read: &mut impl Read,
) -> Result<Reaction<T>, DecodeErrorKind> {
    let mut header = [0; LENGTH_HEADER_LEN];
    read_exact(read, &mut header, FramePart::SupplyLength)?;
    let len = u32::from_be_bytes(header) as usize;
    let mut leaf = vec![0; len];
    read_exact(read, &mut leaf, FramePart::SupplyLeaf)?;

    let mut leaf = leaf.as_slice();
    let version = Version::deserialize(&mut leaf).map_err(DecodeLeafError::Version)?;
    let message = Message::<T>::deserialize(&mut leaf).map_err(DecodeLeafError::Message)?;
    if !leaf.is_empty() {
        return Err(DecodeLeafError::TrailingBytes { count: leaf.len() }.into());
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
            DecodeErrorKind::Truncated {
                missing: part,
                source,
            }
        } else {
            DecodeErrorKind::Read { part, source }
        }
    })
}

#[cfg(test)]
mod tests;
