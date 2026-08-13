//! Exact asynchronous input for the self-delimiting frame grammar.

use std::slice;

use borsh::{BorshDeserialize, io::ErrorKind};
use tokio::io::{AsyncRead, AsyncReadExt};

use super::super::{
    error::{DecodeError, DecodeErrorKind, FramePart},
    frame::{Frame, LeafRun, QUERY_CHILD_LEN, QUERY_COUNT_BIAS, Reaction, WireFrame},
    signal::{Signal, Speaker, Stream},
};
use super::{decode_signal, parse_query};
use crate::tree::{mirror::framing::LENGTH_HEADER_LEN, typed::Hash};

/// Async frame reader over one speaker's transport direction.
///
/// EOF before a signal is a clean direction close and returns `None`. Once a
/// signal arrives, a missing component is a contextual truncation. Variable
/// bodies are read at their declared size and validated exactly once.
pub struct FrameRead<R> {
    speaker: Speaker,
    read: R,
}

impl<R> FrameRead<R> {
    /// Bind `read` to the direction spoken by `speaker`.
    pub fn new(speaker: Speaker, read: R) -> Self {
        Self { speaker, read }
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
    pub async fn frame<T: BorshDeserialize>(
        &mut self,
    ) -> Result<Option<WireFrame<T>>, DecodeError> {
        let Some((stream, signal)) = read_signal(self.speaker, &mut self.read).await? else {
            return Ok(None);
        };
        let frame = AsyncFrameDecoder::new(&mut self.read)
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
}

impl<'a, R: AsyncRead + Unpin> AsyncFrameDecoder<'a, R> {
    fn new(read: &'a mut R) -> Self {
        Self { read }
    }

    async fn body<T: BorshDeserialize>(
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
        let mut run = vec![0; u32::from_be_bytes(header) as usize];
        self.read_exact(&mut run, FramePart::SupplyRun).await?;
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
            .map_err(|source| match source.kind() {
                ErrorKind::UnexpectedEof => DecodeErrorKind::Truncated {
                    missing: part,
                    source,
                },
                _ => DecodeErrorKind::Read { part, source },
            })
    }
}
