//! Self-delimiting frame decoding.

#[cfg(test)]
use std::io::{ErrorKind, Read};

use crate::tree::mirror::cbor::{self, HeadReadError, MAJOR_ARRAY, MAJOR_UINT};

mod async_io;

pub use async_io::FrameRead;

#[cfg(test)]
use super::budget::RunBudget;
#[cfg(test)]
use super::frame::{Frame, LeafRun, Reaction, WireFrame};
use super::{
    error::{DecodeError, DecodeErrorKind, FramePart},
    frame::ListingIssue,
    signal::{Signal, Speaker, Stream, WireSignal},
};
#[cfg(test)]
use crate::tree::typed::{Hash, hash::MERKLE_HASH_LEN};

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
        let arity = self
            .arity()
            .map_err(|kind| DecodeError::direction(self.speaker, kind))?;
        let (stream, signal) = self.signal()?;
        let frame = check_arity(signal, arity)
            .and_then(|()| self.body(signal))
            .map_err(|kind| DecodeError::stream(self.speaker, stream, kind))?;
        Ok((stream, frame))
    }

    /// Read the frame's array head; this oracle treats a clean close as a
    /// truncation, since its callers always expect a frame.
    fn arity(&mut self) -> Result<u64, DecodeErrorKind> {
        let head = cbor::read_head_io(self.read)
            .map_err(|e| head_error(FramePart::FrameHead, e))?
            .ok_or_else(|| {
                head_error(
                    FramePart::FrameHead,
                    HeadReadError::Io(ErrorKind::UnexpectedEof.into()),
                )
            })?;
        frame_arity(head)
    }

    fn signal(&mut self) -> Result<(Stream, Signal), DecodeError> {
        let head = cbor::read_head_io(self.read)
            .map_err(|e| head_error(FramePart::Signal, e))
            .and_then(|head| {
                head.ok_or_else(|| {
                    head_error(
                        FramePart::Signal,
                        HeadReadError::Io(ErrorKind::UnexpectedEof.into()),
                    )
                })
            })
            .map_err(|kind| DecodeError::direction(self.speaker, kind))?;
        let code = signal_code(head).map_err(|kind| DecodeError::direction(self.speaker, kind))?;
        decode_signal(self.speaker, code)
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
        let head = self.head(FramePart::QueryChildren)?;
        let mut listing = query_listing(head)?;
        let count = head.value;
        for _ in 0..count {
            let key = self.head(FramePart::QueryChildren)?;
            let radix = listing.key(key).map_err(listing_issue)?;
            let value = self.head(FramePart::QueryChildren)?;
            super::frame::ListingBuilder::value_head(value).map_err(listing_issue)?;
            let mut digest = [0; MERKLE_HASH_LEN];
            self.read_exact(&mut digest, FramePart::QueryChildren)?;
            listing.entry(radix, digest);
        }
        Ok(listing.finish())
    }

    fn supply(&mut self) -> Result<LeafRun, DecodeErrorKind> {
        let tag = self.head(FramePart::SupplyLength)?;
        let body = self.head(FramePart::SupplyLength)?;
        let len = run_head(tag, body)?;
        // The run-budget ingress check, mirroring the async reader's exactly
        // (see `AsyncFrameDecoder::supply` for the memory argument this
        // oracle does not need): an over-budget frame is legal only as one
        // lone record spanning the whole body, decided from the first
        // record's heads alone.
        if !self.budget.covers(len) {
            let budget = self.budget;
            let overbatched = move || DecodeErrorKind::OverbatchedRun {
                declared: super::budget::SUPPLY_FRAME_OVERHEAD.saturating_add(len),
                budget: budget.bytes(),
            };
            // A body too short to hold a record's heads cannot be a lone
            // record: rejected on the declared length alone.
            if len < super::frame::RECORD_TAG_LEN + 1 {
                return Err(overbatched());
            }
            let Some((prefix, record)) = self.record_prefix()? else {
                return Err(overbatched());
            };
            if !super::frame::lone_record_spans(len, record) {
                return Err(overbatched());
            }
            let mut run = vec![0; len];
            run[..prefix.len()].copy_from_slice(&prefix);
            self.read_exact(&mut run[prefix.len()..], FramePart::SupplyRun)?;
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

    /// Read the first record's heads inside an over-budget run, returning
    /// the exact bytes consumed and the record content length; `None` when
    /// they are not a record's heads (over budget, the distinction from
    /// malformed is moot).
    fn record_prefix(&mut self) -> Result<Option<(Vec<u8>, u64)>, DecodeErrorKind> {
        let mut prefix = Vec::new();
        let tag = self.head(FramePart::SupplyRun)?;
        cbor::write_head(&mut prefix, tag.major, tag.value);
        if tag.major != cbor::MAJOR_TAG || tag.value != cbor::TAG_CBOR_SEQUENCE {
            return Ok(None);
        }
        let body = self.head(FramePart::SupplyRun)?;
        cbor::write_head(&mut prefix, body.major, body.value);
        if body.major != cbor::MAJOR_BSTR {
            return Ok(None);
        }
        Ok(Some((prefix, body.value)))
    }

    fn head(&mut self, part: FramePart) -> Result<cbor::Head, DecodeErrorKind> {
        cbor::read_head_io(self.read)
            .map_err(|e| head_error(part, e))?
            .ok_or_else(|| head_error(part, HeadReadError::Io(ErrorKind::UnexpectedEof.into())))
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

/// Validate a frame's array head: a definite array of one or two items.
pub(super) fn frame_arity(head: cbor::Head) -> Result<u64, DecodeErrorKind> {
    if head.major != MAJOR_ARRAY {
        return Err(DecodeErrorKind::FrameShape {
            detail: "frame item is not an array",
        });
    }
    if !(1..=2).contains(&head.value) {
        return Err(DecodeErrorKind::FrameShape {
            detail: "frame array is not one or two items",
        });
    }
    Ok(head.value)
}

/// Validate a signal head: an unsigned int within the dense code space's
/// byte range. Codes above the dense space but within the byte range keep
/// their reserved-value taxonomy downstream.
pub(super) fn signal_code(head: cbor::Head) -> Result<u8, DecodeErrorKind> {
    if head.major != MAJOR_UINT {
        return Err(DecodeErrorKind::Malformed {
            part: FramePart::Signal,
            detail: "signal is not an unsigned int",
        });
    }
    u8::try_from(head.value).map_err(|_| DecodeErrorKind::Malformed {
        part: FramePart::Signal,
        detail: "signal is outside the dense code space",
    })
}

/// Enforce the frame array's length against its signal's body arity.
pub(super) fn check_arity(signal: Signal, arity: u64) -> Result<(), DecodeErrorKind> {
    let expected = match signal {
        Signal::Match(_) | Signal::QueryEmpty(_) | Signal::End(_) => 1,
        Signal::Query(_) | Signal::Supply(_) => 2,
    };
    if arity != expected {
        return Err(DecodeErrorKind::FrameArity {
            expected,
            found: arity,
        });
    }
    Ok(())
}

/// Open a query body: its head must be a nonempty listing map (an empty
/// query travels as its own signal), within the radix space.
pub(super) fn query_listing(
    head: cbor::Head,
) -> Result<super::frame::ListingBuilder, DecodeErrorKind> {
    if head.major != cbor::MAJOR_MAP {
        return Err(DecodeErrorKind::Malformed {
            part: FramePart::QueryChildren,
            detail: "query body is not a listing map",
        });
    }
    if head.value == 0 {
        return Err(DecodeErrorKind::Malformed {
            part: FramePart::QueryChildren,
            detail: "a nonempty query's listing is empty",
        });
    }
    super::frame::ListingBuilder::new(head.value).map_err(listing_issue)
}

/// Open a supply body: the run's embedded-sequence tag and byte-string
/// head, held to the wire's run byte cap.
pub(super) fn run_head(tag: cbor::Head, body: cbor::Head) -> Result<usize, DecodeErrorKind> {
    if tag.major != cbor::MAJOR_TAG || tag.value != cbor::TAG_CBOR_SEQUENCE {
        return Err(DecodeErrorKind::Malformed {
            part: FramePart::SupplyLength,
            detail: "supply body does not open with the embedded-sequence tag",
        });
    }
    if body.major != cbor::MAJOR_BSTR {
        return Err(DecodeErrorKind::Malformed {
            part: FramePart::SupplyLength,
            detail: "supply tag does not wrap a byte string",
        });
    }
    u32::try_from(body.value)
        .map(|len| len as usize)
        .map_err(|_| DecodeErrorKind::Malformed {
            part: FramePart::SupplyLength,
            detail: "supply run exceeds the run byte cap",
        })
}

/// Type a listing-map violation into the frame error taxonomy.
pub(super) fn listing_issue(issue: ListingIssue) -> DecodeErrorKind {
    match issue {
        ListingIssue::Order(order) => DecodeErrorKind::QueryOutOfOrder(order),
        ListingIssue::Head(_) => DecodeErrorKind::Malformed {
            part: FramePart::QueryChildren,
            detail: "listing head is not canonical",
        },
        ListingIssue::Shape(detail) => DecodeErrorKind::Malformed {
            part: FramePart::QueryChildren,
            detail,
        },
        ListingIssue::Truncated => DecodeErrorKind::Malformed {
            part: FramePart::QueryChildren,
            detail: "listing hash bytes are truncated",
        },
    }
}

/// Type a head-read failure by the frame part it interrupted.
pub(super) fn head_error(part: FramePart, error: HeadReadError) -> DecodeErrorKind {
    match error {
        HeadReadError::Io(source) => match source.kind() {
            std::io::ErrorKind::UnexpectedEof => DecodeErrorKind::Truncated {
                missing: part,
                source,
            },
            _ => DecodeErrorKind::Read { part, source },
        },
        HeadReadError::Malformed(head) => DecodeErrorKind::Malformed {
            part,
            detail: head_detail(head),
        },
    }
}

/// Name a deterministic-contract violation for the error taxonomy.
fn head_detail(error: cbor::HeadError) -> &'static str {
    match error {
        cbor::HeadError::Truncated => "truncated head",
        cbor::HeadError::Indefinite => "indefinite-length head",
        cbor::HeadError::Reserved => "reserved head",
        cbor::HeadError::NotShortest => "head not in shortest form",
    }
}

pub(super) fn decode_signal(speaker: Speaker, code: u8) -> Result<(Stream, Signal), DecodeError> {
    let wire = WireSignal::from_byte(speaker, code)
        .map_err(|invalid| DecodeError::stream(speaker, invalid.stream(), invalid.into()))?;
    Ok(wire.into_parts())
}

#[cfg(test)]
mod tests;
