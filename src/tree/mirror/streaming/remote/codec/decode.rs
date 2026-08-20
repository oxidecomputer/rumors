//! Self-delimiting frame decoding.

#[cfg(test)]
use std::slice;

#[cfg(test)]
use std::io::{ErrorKind, Read};

use crate::tree::mirror::framing::LENGTH_HEADER_LEN;
use crate::tree::typed::{Hash, hash::MERKLE_HASH_LEN};

mod async_io;

pub use async_io::FrameRead;

#[cfg(test)]
use super::budget::RunBudget;
#[cfg(test)]
use super::{
    error::FramePart,
    frame::{Frame, LeafRun, QUERY_COUNT_BIAS, Reaction, WireFrame},
};
use super::{
    error::{DecodeError, DecodeErrorKind},
    frame::{QUERY_CHILD_LEN, validate_children},
    signal::{Signal, Speaker, Stream, WireSignal},
};

/// Decode one frame from `read`, leaving subsequent bytes untouched.
#[cfg(test)]
pub fn decode(
    speaker: Speaker,
    budget: RunBudget,
    read: &mut impl Read,
) -> Result<WireFrame, DecodeError> {
    FrameDecoder::new(speaker, budget, read).decode()
}

/// Decode exactly one frame from a slice, rejecting bytes after it.
#[cfg(test)]
pub fn decode_exact(
    speaker: Speaker,
    budget: RunBudget,
    input: &[u8],
) -> Result<WireFrame, DecodeError> {
    let mut rest = input;
    let (stream, frame) = decode(speaker, budget, &mut rest)?;
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
    /// The session's run budget, gating supply-body buffering.
    budget: RunBudget,
    read: &'a mut R,
}

#[cfg(test)]
impl<'a, R: Read> FrameDecoder<'a, R> {
    fn new(speaker: Speaker, budget: RunBudget, read: &'a mut R) -> Self {
        Self {
            speaker,
            budget,
            read,
        }
    }

    fn decode(mut self) -> Result<WireFrame, DecodeError> {
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

    fn body(&mut self, signal: Signal) -> Result<Frame, DecodeErrorKind> {
        let frame = match signal {
            Signal::Match(flow) => Frame::Reaction(Reaction::Match, flow),
            Signal::QueryEmpty(flow) => Frame::Reaction(Reaction::Query(Vec::new()), flow),
            Signal::Query(flow) => Frame::Reaction(Reaction::Query(self.query()?), flow),
            Signal::Supply(flow) => Frame::Reaction(Reaction::Supply(self.supply()?), flow),
            Signal::End(end) => Frame::End(end),
        };
        Ok(frame)
    }

    fn query(&mut self) -> Result<Vec<(u8, Hash)>, DecodeErrorKind> {
        let count = usize::from(self.byte(FramePart::QueryCount)?) + QUERY_COUNT_BIAS;
        // One bulk read for the whole listing rather than one call per child.
        let mut listing = vec![0; count * QUERY_CHILD_LEN];
        self.read_exact(&mut listing, FramePart::QueryChildren)?;

        parse_query(&listing)
    }

    fn supply(&mut self) -> Result<LeafRun, DecodeErrorKind> {
        let mut header = [0; LENGTH_HEADER_LEN];
        self.read_exact(&mut header, FramePart::SupplyLength)?;
        let len = u32::from_be_bytes(header) as usize;
        // The run-budget ingress check, mirroring the async reader's exactly
        // (see `AsyncFrameDecoder::supply` for the memory argument this
        // oracle does not need): an over-budget frame is legal only as one
        // lone record spanning the whole body, decided from the first
        // record's length header alone.
        if !self.budget.covers(len) {
            let budget = self.budget;
            let overbatched = move || DecodeErrorKind::OverbatchedRun {
                declared: super::budget::SUPPLY_FRAME_OVERHEAD.saturating_add(len),
                budget: budget.bytes(),
            };
            if len < LENGTH_HEADER_LEN {
                return Err(overbatched());
            }
            let mut first = [0; LENGTH_HEADER_LEN];
            self.read_exact(&mut first, FramePart::SupplyRun)?;
            let record = u32::from_be_bytes(first) as usize;
            if !lone_record_spans(len, record) {
                return Err(overbatched());
            }
            let mut run = vec![0; len];
            run[..LENGTH_HEADER_LEN].copy_from_slice(&first);
            self.read_exact(&mut run[LENGTH_HEADER_LEN..], FramePart::SupplyRun)?;
            return Ok(LeafRun::from_encoded(run)?);
        }
        // This oracle deliberately reads the whole declared body at once so
        // it stays maximally simple; the async reader chunks its reads, and
        // the framing differential proptest carries payload identity across
        // the two shapes.
        let mut run = vec![0; len];
        self.read_exact(&mut run, FramePart::SupplyRun)?;

        Ok(LeafRun::from_encoded(run)?)
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

/// Whether a run body of `len` bytes is exactly one record: the first
/// record's header plus the record it declares span the body.
///
/// The lone-record test of the run-budget ingress check, shared by the
/// async reader and the sync oracle so the two decoders draw the
/// over-budget legality boundary identically. A body this predicate
/// rejects may also be structurally malformed; over budget, that
/// distinction is moot — either way the frame is not the one legal
/// overhang — so the check does not refine it further.
fn lone_record_spans(len: usize, first_record_len: usize) -> bool {
    LENGTH_HEADER_LEN.saturating_add(first_record_len) == len
}

fn decode_signal(speaker: Speaker, byte: u8) -> Result<(Stream, Signal), DecodeError> {
    let wire = WireSignal::from_byte(speaker, byte)
        .map_err(|invalid| DecodeError::stream(speaker, invalid.stream(), invalid.into()))?;
    Ok(wire.into_parts())
}

/// `pub(super)` for the capture renderer, which decodes captured query
/// children through the same canonical path (order validation included).
pub(super) fn parse_query(listing: &[u8]) -> Result<Vec<(u8, Hash)>, DecodeErrorKind> {
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

#[cfg(test)]
mod tests;
