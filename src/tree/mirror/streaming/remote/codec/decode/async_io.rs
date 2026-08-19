//! Exact asynchronous input for the self-delimiting frame grammar.

use std::slice;

use std::io::ErrorKind;
use tokio::io::{AsyncRead, AsyncReadExt};

use super::super::{
    budget::RunBudget,
    error::{DecodeError, DecodeErrorKind, FramePart},
    frame::{Frame, LeafRun, QUERY_CHILD_LEN, QUERY_COUNT_BIAS, Reaction, WireFrame},
    signal::{Signal, Speaker, Stream},
};
use super::{decode_signal, lone_record_spans, parse_query};
use crate::tree::{
    mirror::framing::{LENGTH_HEADER_LEN, read_payload, resume_payload},
    typed::Hash,
};

use serde::de::DeserializeOwned;
/// Async frame reader over one speaker's transport direction.
///
/// EOF before a signal is a clean direction close and returns `None`. Once a
/// signal arrives, a missing component is a contextual truncation. Variable
/// bodies are read at their declared size and validated exactly once, with
/// supply bodies additionally held to the session's run budget before they
/// are buffered ([`DecodeErrorKind::OverbatchedRun`]).
pub struct FrameRead<R> {
    speaker: Speaker,
    /// The session's negotiated run budget, enforced on every supply frame
    /// this direction delivers.
    budget: RunBudget,
    read: R,
}

impl<R> FrameRead<R> {
    /// Bind `read` to the direction spoken by `speaker`, enforcing `budget`
    /// on the supply frames it delivers.
    pub fn new(speaker: Speaker, budget: RunBudget, read: R) -> Self {
        Self {
            speaker,
            budget,
            read,
        }
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
    /// as a signal. Either retain the in-flight future across polls until
    /// it resolves, or read nothing further from this direction after a
    /// cancellation.
    pub async fn frame<T: DeserializeOwned>(
        &mut self,
    ) -> Result<Option<WireFrame<T>>, DecodeError> {
        let Some((stream, signal)) = read_signal(self.speaker, &mut self.read).await? else {
            return Ok(None);
        };
        let frame = AsyncFrameDecoder::new(&mut self.read, self.budget)
            .body(signal)
            .await
            .map_err(|kind| DecodeError::stream(self.speaker, stream, kind))?;
        Ok(Some((stream, frame)))
    }
}

async fn read_signal(
    speaker: Speaker,
    read: &mut (impl AsyncRead + Unpin),
) -> Result<Option<(Stream, Signal)>, DecodeError> {
    let mut byte = 0;
    match read.read(slice::from_mut(&mut byte)).await {
        Ok(0) => Ok(None),
        Ok(1) => decode_signal(speaker, byte).map(Some),
        Ok(_) => unreachable!("a one-byte async read returns at most one byte"),
        Err(source) => Err(DecodeError::direction(
            speaker,
            DecodeErrorKind::Read {
                part: FramePart::Signal,
                source,
            },
        )),
    }
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

    async fn body<T: DeserializeOwned>(
        &mut self,
        signal: Signal,
    ) -> Result<Frame<T>, DecodeErrorKind> {
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
        let count = usize::from(self.byte(FramePart::QueryCount).await?) + QUERY_COUNT_BIAS;
        let mut listing = vec![0; count * QUERY_CHILD_LEN];
        self.read_exact(&mut listing, FramePart::QueryChildren)
            .await?;
        parse_query(&listing)
    }

    async fn supply<T>(&mut self) -> Result<LeafRun<T>, DecodeErrorKind> {
        let mut header = [0; LENGTH_HEADER_LEN];
        self.read_exact(&mut header, FramePart::SupplyLength)
            .await?;
        let len = u32::from_be_bytes(header) as usize;
        let run = if self.budget.covers(len) {
            read_payload(self.read, len)
                .await
                .map_err(|source| classify(FramePart::SupplyRun, source))?
        } else {
            // The frame outsizes the budget the peer's encoder flushes
            // within, so the one shape an honest encoder can still have
            // produced is a single record spanning the whole body (the
            // minimum-one-record overhang). That is decidable from the
            // first record's length header alone, so nothing beyond it is
            // read until the frame is known legal: a violating frame is
            // rejected before its body is buffered, keeping the decode
            // inside the memory envelope the budget priced. A body too
            // short to hold a record header cannot be a lone record and is
            // rejected on the declared length alone.
            let budget = self.budget;
            let overbatched = move || DecodeErrorKind::OverbatchedRun {
                declared: super::super::budget::SUPPLY_FRAME_OVERHEAD.saturating_add(len),
                budget: budget.bytes(),
            };
            if len < LENGTH_HEADER_LEN {
                return Err(overbatched());
            }
            let mut first = [0; LENGTH_HEADER_LEN];
            self.read_exact(&mut first, FramePart::SupplyRun).await?;
            let record = u32::from_be_bytes(first) as usize;
            if !lone_record_spans(len, record) {
                return Err(overbatched());
            }
            // Legal lone record: resume the body read behind the header
            // already consumed, in the same single buffer.
            resume_payload(self.read, first.to_vec(), len)
                .await
                .map_err(|source| classify(FramePart::SupplyRun, source))?
        };
        Ok(LeafRun::from_encoded(run)?)
    }

    async fn byte(&mut self, part: FramePart) -> Result<u8, DecodeErrorKind> {
        let mut byte = 0;
        self.read_exact(slice::from_mut(&mut byte), part).await?;
        Ok(byte)
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
