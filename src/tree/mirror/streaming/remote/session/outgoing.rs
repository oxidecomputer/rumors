//! Bottom-most outgoing scheduling with exact physical-write completion.

use std::{future::poll_fn, pin::Pin, task::Poll};

use tokio::{
    io::AsyncWrite,
    sync::{mpsc, oneshot},
};

use super::{STREAM_COUNT, handoffs, stream_at};
use crate::tree::mirror::streaming::remote::codec::{
    EncodeError, End, Frame, FrameWrite, Origin, Speaker, Stream,
};

/// Build an outgoing multiplexer and its one-slot per-stream senders.
///
/// `speaker` is the local party whose direction `write` carries.
pub fn outgoing<W, T>(speaker: Speaker, write: W) -> (Mux<W, T>, Outgoing<T>) {
    let (senders, receivers) = handoffs();
    let mut index = 0;
    let senders = senders.map(|send| {
        let sender = FrameSender {
            origin: Origin::stream(speaker, stream_at(index)),
            send,
        };
        index += 1;
        Some(sender)
    });
    (
        Mux {
            speaker,
            write: FrameWrite::new(speaker, write),
            receivers: receivers.map(Some),
            remaining: STREAM_COUNT,
        },
        Outgoing { senders },
    )
}

/// One frame whose completion is tied to its own oneshot receipt.
struct WriteRequest<T> {
    frame: Frame<T>,
    written: oneshot::Sender<()>,
}

impl<T> WriteRequest<T> {
    /// Pair a new frame with the sole receipt which can acknowledge it.
    fn new(frame: Frame<T>) -> (Self, WriteReceipt) {
        let (written, receipt) = oneshot::channel();
        (Self { frame, written }, WriteReceipt(receipt))
    }

    /// Write and flush this exact frame before completing its receipt.
    async fn write<W>(self, stream: Stream, write: &mut FrameWrite<W>) -> Result<bool, EncodeError>
    where
        W: AsyncWrite + Unpin,
    {
        let Self { frame, written } = self;
        let closes_stream = frame.end() == Some(End::Stream);
        write.frame(&(stream, frame)).await?;
        let _ = written.send(());
        Ok(closes_stream)
    }
}

/// The cancellation-safe completion handle for one exact write request.
struct WriteReceipt(oneshot::Receiver<()>);

impl WriteReceipt {
    /// Wait until the paired request has been written and flushed.
    async fn wait(self) -> Result<(), oneshot::error::RecvError> {
        self.0.await
    }
}

/// The sending ends of the multiplexed logical streams.
pub struct Outgoing<T> {
    senders: [Option<FrameSender<T>>; STREAM_COUNT],
}

impl<T> Outgoing<T> {
    /// Take the sole sender for `stream`.
    pub fn take(&mut self, stream: Stream) -> FrameSender<T> {
        self.senders[usize::from(stream.index())]
            .take()
            .expect("each outgoing logical stream is taken exactly once")
    }
}

/// A one-slot logical-stream sender with physical-write acknowledgement.
pub struct FrameSender<T> {
    origin: Origin,
    send: mpsc::Sender<WriteRequest<T>>,
}

/// A protocol reply frame, statically excluding stream-end transport control.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplyFrame<T>(Frame<T>);

impl<T> TryFrom<Frame<T>> for ReplyFrame<T> {
    type Error = ReplyFrameError;

    /// Check that a general wire frame belongs to a protocol reply.
    fn try_from(frame: Frame<T>) -> Result<Self, Self::Error> {
        if matches!(frame, Frame::End(End::Stream)) {
            Err(ReplyFrameError::StreamEnd)
        } else {
            Ok(Self(frame))
        }
    }
}

impl<T> From<ReplyFrame<T>> for Frame<T> {
    /// Recover the general wire frame for transport encoding.
    fn from(frame: ReplyFrame<T>) -> Self {
        frame.0
    }
}

/// A general wire frame was transport control rather than a protocol reply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ReplyFrameError {
    /// Stream end is emitted only by the internal `FrameSender::finish` path.
    #[error("stream-end control is not a protocol reply frame")]
    StreamEnd,
}

impl<T> FrameSender<T> {
    /// Enqueue one reply frame and return after it is written and flushed.
    ///
    /// The mutable borrow deliberately permits only one in-flight frame from
    /// this logical producer. The per-frame receipt additionally makes a
    /// cancelled wait impossible to reuse as a later frame's acknowledgement.
    /// Stream-end control is excluded by [`ReplyFrame`] and emitted only by
    /// [`Self::finish`].
    pub async fn frame(&mut self, frame: ReplyFrame<T>) -> Result<(), SendError> {
        self.send(frame.into()).await
    }

    /// End this logical stream after all of its replies have been flushed.
    pub async fn finish(mut self) -> Result<(), SendError> {
        self.send(Frame::End(End::Stream)).await
    }

    /// Enqueue one frame and await its exact physical-write receipt.
    async fn send(&mut self, frame: Frame<T>) -> Result<(), SendError> {
        let (request, receipt) = WriteRequest::new(frame);
        self.send.send(request).await.map_err(|_| SendError {
            origin: self.origin,
        })?;
        receipt.wait().await.map_err(|_| SendError {
            origin: self.origin,
        })
    }
}

/// The session stopped before a queued frame was flushed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("{origin}: session closed before the logical stream flushed its frame")]
pub struct SendError {
    pub origin: Origin,
}

/// Schedules logical streams onto one frame writer.
///
/// Each receiver slot transitions exactly once from open (`Some`) to ended
/// (`None`) after its stream-end control is flushed. A sender disappearing first
/// terminates the driver instead of silently completing that logical stream.
pub struct Mux<W, T> {
    speaker: Speaker,
    write: FrameWrite<W>,
    receivers: [Option<mpsc::Receiver<WriteRequest<T>>>; STREAM_COUNT],
    remaining: usize,
}

impl<W, T> Mux<W, T>
where
    W: AsyncWrite + Unpin,
{
    /// Drive frames until every logical stream ends, returning the raw writer.
    pub async fn run(mut self) -> Result<W, MuxError> {
        loop {
            let (stream, request) = self.next().await?;
            let index = usize::from(stream.index());
            let closes_stream = request
                .write(stream, &mut self.write)
                .await
                .map_err(MuxError::Codec)?;
            if closes_stream {
                self.receivers[index] = None;
                self.remaining -= 1;
                if self.remaining == 0 {
                    return Ok(self.write.into_inner());
                }
            }
        }
    }

    /// Wait for the bottom-most ready stream or a premature producer close.
    async fn next(&mut self) -> Result<(Stream, WriteRequest<T>), MuxError> {
        poll_fn(|cx| {
            // Reverse order is the scheduling policy. Polling every pending
            // receiver registers this task's waker without another arbiter.
            for index in (0..STREAM_COUNT).rev() {
                let Some(receive) = &mut self.receivers[index] else {
                    continue;
                };
                match Pin::new(receive).poll_recv(cx) {
                    Poll::Ready(Some(request)) => {
                        return Poll::Ready(Ok((stream_at(index), request)));
                    }
                    Poll::Ready(None) => {
                        self.receivers[index] = None;
                        return Poll::Ready(Err(MuxError::SenderDropped {
                            origin: Origin::stream(self.speaker, stream_at(index)),
                        }));
                    }
                    Poll::Pending => {}
                }
            }
            Poll::Pending
        })
        .await
    }
}

/// Failure while scheduling and writing local frames.
#[derive(Debug, thiserror::Error)]
pub enum MuxError {
    /// The outgoing frame codec or transport failed.
    #[error(transparent)]
    Codec(#[from] EncodeError),
    /// A local producer disappeared without ending its logical stream.
    ///
    /// The coordinator must prefer any semantic error which caused this local
    /// cancellation; this variant is only the standalone driver's diagnosis.
    #[error("{origin}: producer dropped the logical stream before its end")]
    SenderDropped { origin: Origin },
}
