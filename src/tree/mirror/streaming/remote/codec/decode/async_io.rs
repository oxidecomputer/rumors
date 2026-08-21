//! Exact asynchronous input for the self-delimiting frame grammar.

use std::io::ErrorKind;
use tokio::io::{AsyncRead, AsyncReadExt};

use crate::observe::{CaptureRead, StreamObserver};

use super::super::{
    budget::RunBudget,
    error::{DecodeError, DecodeErrorKind, FramePart},
    frame::{Frame, LeafRun, ListingBuilder, Reaction, WireFrame},
    signal::{Signal, Speaker},
};
use super::{
    check_arity, decode_signal, frame_arity, head_error, listing_issue, query_listing, run_head,
    signal_code,
};
use crate::tree::{
    mirror::cbor,
    mirror::framing::{read_payload, resume_payload},
    typed::{Hash, hash::MERKLE_HASH_LEN},
};

/// Async frame reader over one speaker's transport direction.
///
/// EOF before a frame's array head is a clean direction close and returns
/// `None`. Once that head arrives, a missing component is a contextual
/// truncation. Variable bodies are read at their declared size and
/// validated exactly once, with supply bodies additionally held to the
/// session's run budget before they are buffered
/// ([`DecodeErrorKind::OverbatchedRun`]).
pub struct FrameRead<R> {
    speaker: Speaker,
    /// The session's negotiated run budget, enforced on every supply frame
    /// this direction delivers.
    budget: RunBudget,
    read: R,
    /// The directed stream's observer, if any: handed each accepted
    /// frame's exact wire bytes, and costing one branch when absent.
    observe: Option<Box<dyn StreamObserver>>,
}

impl<R> FrameRead<R> {
    /// Bind `read` to the direction spoken by `speaker`, enforcing `budget`
    /// on the supply frames it delivers.
    pub fn new(speaker: Speaker, budget: RunBudget, read: R) -> Self {
        Self {
            speaker,
            budget,
            read,
            observe: None,
        }
    }

    /// Deliver every accepted frame to `observe`, when one is attached.
    pub fn observed(mut self, observe: Option<Box<dyn StreamObserver>>) -> Self {
        self.observe = observe;
        self
    }

    /// Recover the transport half. The reader buffers nothing (every
    /// read is exact), so between frames the half rests exactly at a
    /// frame boundary.
    pub fn into_inner(self) -> R {
        self.read
    }
}

impl<R: AsyncRead + Unpin> FrameRead<R> {
    /// Read and decode one frame without consuming any byte of the next.
    ///
    /// # Cancel safety
    ///
    /// Not cancel safe. A dropped `frame` future may already have consumed
    /// part of a frame — the exact reads do not give bytes back — leaving
    /// the direction mid-frame, where the next call would parse body bytes
    /// as a frame head. Either retain the in-flight future across polls
    /// until it resolves, or read nothing further from this direction after
    /// a cancellation.
    pub async fn frame(&mut self) -> Result<Option<WireFrame>, DecodeError> {
        match &mut self.observe {
            None => read_frame(&mut self.read, self.speaker, self.budget).await,
            Some(observe) => {
                // Retain the consumed bytes so the observer sees the
                // frame's true wire spelling, never a re-encoding. Only
                // an accepted whole frame is delivered: a clean close
                // consumed nothing, and an error leaves a fragment.
                let mut capture = CaptureRead::new(&mut self.read);
                let result = read_frame(&mut capture, self.speaker, self.budget).await;
                if let Ok(Some(_)) = &result {
                    observe.message(capture.bytes());
                }
                result
            }
        }
    }
}

/// Read and decode one frame from `read`; the contract is
/// [`FrameRead::frame`]'s.
async fn read_frame<R: AsyncRead + Unpin>(
    read: &mut R,
    speaker: Speaker,
    budget: RunBudget,
) -> Result<Option<WireFrame>, DecodeError> {
    let Some(head) = cbor::read_head_async(&mut *read)
        .await
        .map_err(|e| head_error(FramePart::FrameHead, e))
        .map_err(|kind| DecodeError::direction(speaker, kind))?
    else {
        return Ok(None);
    };
    let mut decoder = AsyncFrameDecoder::new(read, budget);
    let arity = frame_arity(head).map_err(|kind| DecodeError::direction(speaker, kind))?;
    let signal_head = decoder
        .head(FramePart::Signal)
        .await
        .map_err(|kind| DecodeError::direction(speaker, kind))?;
    let code = signal_code(signal_head).map_err(|kind| DecodeError::direction(speaker, kind))?;
    let (stream, signal) = decode_signal(speaker, code)?;
    let frame = async {
        check_arity(signal, arity)?;
        decoder.body(signal).await
    }
    .await
    .map_err(|kind| DecodeError::stream(speaker, stream, kind))?;
    Ok(Some((stream, frame)))
}

/// Reads a body after its signal has established the frame grammar.
struct AsyncFrameDecoder<'a, R> {
    read: &'a mut R,
    /// The session's run budget, gating supply-body buffering.
    budget: RunBudget,
}

impl<'a, R: AsyncRead + Unpin> AsyncFrameDecoder<'a, R> {
    fn new(read: &'a mut R, budget: RunBudget) -> Self {
        Self { read, budget }
    }

    async fn body(&mut self, signal: Signal) -> Result<Frame, DecodeErrorKind> {
        let frame = match signal {
            Signal::Match(flow) => Frame::Reaction(Reaction::Match, flow),
            Signal::QueryEmpty(flow) => Frame::Reaction(Reaction::Query(Vec::new()), flow),
            Signal::Query(flow) => Frame::Reaction(Reaction::Query(self.query().await?), flow),
            Signal::Supply(flow) => Frame::Reaction(Reaction::Supply(self.supply().await?), flow),
            Signal::End(end) => Frame::End(end),
        };
        Ok(frame)
    }

    async fn query(&mut self) -> Result<Vec<(u8, Hash)>, DecodeErrorKind> {
        let head = self.head(FramePart::QueryChildren).await?;
        let mut listing = query_listing(head)?;
        for _ in 0..head.value {
            let key = self.head(FramePart::QueryChildren).await?;
            let radix = listing.key(key).map_err(listing_issue)?;
            let value = self.head(FramePart::QueryChildren).await?;
            ListingBuilder::value_head(value).map_err(listing_issue)?;
            let mut digest = [0; MERKLE_HASH_LEN];
            self.read_exact(&mut digest, FramePart::QueryChildren)
                .await?;
            listing.entry(radix, digest);
        }
        Ok(listing.finish())
    }

    async fn supply(&mut self) -> Result<LeafRun, DecodeErrorKind> {
        let tag = self.head(FramePart::SupplyLength).await?;
        let body = self.head(FramePart::SupplyLength).await?;
        let len = run_head(tag, body)?;
        let run = if self.budget.covers(len) {
            read_payload(self.read, len)
                .await
                .map_err(|source| classify(FramePart::SupplyRun, source))?
        } else {
            // The frame outsizes the budget the peer's encoder flushes
            // within, so the one shape an honest encoder can still have
            // produced is a single record spanning the whole body (the
            // minimum-one-record overhang). That is decidable from the
            // first record's heads alone, so nothing beyond them is
            // read until the frame is known legal: a violating frame is
            // rejected before its body is buffered, keeping the decode
            // inside the memory envelope the budget priced. A body too
            // short to hold a record's heads cannot be a lone record and
            // is rejected on the declared length alone.
            let budget = self.budget;
            let overbatched = move || DecodeErrorKind::OverbatchedRun {
                declared: super::super::budget::SUPPLY_FRAME_OVERHEAD.saturating_add(len),
                budget: budget.bytes(),
            };
            if len < super::super::frame::RECORD_TAG_LEN + 1 {
                return Err(overbatched());
            }
            let Some((prefix, record)) = self.record_prefix().await? else {
                return Err(overbatched());
            };
            if !super::super::frame::lone_record_spans(len, record) {
                return Err(overbatched());
            }
            // Legal lone record: resume the body read behind the heads
            // already consumed, in the same single buffer.
            resume_payload(self.read, prefix, len)
                .await
                .map_err(|source| classify(FramePart::SupplyRun, source))?
        };
        Ok(LeafRun::from_encoded(run)?)
    }

    /// Read the first record's heads inside an over-budget run, returning
    /// the exact bytes consumed and the record content length.
    ///
    /// `None` when they are not a record's heads: over budget, the
    /// distinction from malformed is moot — either way the frame is not
    /// the legal overhang.
    async fn record_prefix(&mut self) -> Result<Option<(Vec<u8>, u64)>, DecodeErrorKind> {
        let mut prefix = Vec::new();
        let tag = self.head(FramePart::SupplyRun).await?;
        cbor::write_head(&mut prefix, tag.major, tag.value);
        if tag.major != cbor::MAJOR_TAG || tag.value != cbor::TAG_CBOR_SEQUENCE {
            return Ok(None);
        }
        let body = self.head(FramePart::SupplyRun).await?;
        cbor::write_head(&mut prefix, body.major, body.value);
        if body.major != cbor::MAJOR_BSTR {
            return Ok(None);
        }
        Ok(Some((prefix, body.value)))
    }

    async fn head(&mut self, part: FramePart) -> Result<cbor::Head, DecodeErrorKind> {
        cbor::read_head_async(self.read)
            .await
            .map_err(|e| head_error(part, e))?
            .ok_or_else(|| {
                head_error(
                    part,
                    cbor::HeadReadError::Io(ErrorKind::UnexpectedEof.into()),
                )
            })
    }

    async fn read_exact(
        &mut self,
        bytes: &mut [u8],
        part: FramePart,
    ) -> Result<(), DecodeErrorKind> {
        self.read
            .read_exact(bytes)
            .await
            .map(|_| ())
            .map_err(|source| classify(part, source))
    }
}

/// Type an I/O failure by the frame part it interrupted: end-of-stream is a
/// contextual truncation, anything else a plain read failure.
fn classify(part: FramePart, source: std::io::Error) -> DecodeErrorKind {
    match source.kind() {
        ErrorKind::UnexpectedEof => DecodeErrorKind::Truncated {
            missing: part,
            source,
        },
        _ => DecodeErrorKind::Read { part, source },
    }
}
