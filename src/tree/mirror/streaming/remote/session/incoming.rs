//! Incoming frame routing and logical-stream lifecycle validation.

use borsh::BorshDeserialize;
use tokio::{io::AsyncRead, sync::mpsc};
use tokio_stream::wrappers::ReceiverStream;

use super::{STREAM_COUNT, handoffs, stream_at};
use crate::tree::mirror::streaming::remote::codec::{
    DecodeError, End, Frame, FrameRead, Origin, Speaker, Stream,
};

/// Build an incoming demultiplexer and its one-slot per-stream receivers.
///
/// `speaker` is the remote party whose direction `read` carries.
pub fn incoming<R, T>(speaker: Speaker, read: R) -> (Demux<R, T>, Incoming<T>) {
    let (senders, receivers) = handoffs();
    (
        Demux {
            speaker,
            read: FrameRead::new(speaker, read),
            senders: senders.map(Some),
        },
        Incoming {
            receivers: receivers.map(|receiver| Some(ReceiverStream::new(receiver))),
        },
    )
}

/// The receiving ends of the demultiplexed logical streams.
pub struct Incoming<T> {
    receivers: [Option<ReceiverStream<Frame<T>>>; STREAM_COUNT],
}

impl<T> Incoming<T> {
    /// Take the sole receiver for `stream`.
    pub fn take(&mut self, stream: Stream) -> ReceiverStream<Frame<T>> {
        self.receivers[usize::from(stream.index())]
            .take()
            .expect("each incoming logical stream is taken exactly once")
    }
}

/// Routes decoded frames into their logical streams.
///
/// Each sender slot transitions exactly once from open (`Some`) to ended
/// (`None`) after its stream-end control is consumed. A local receiver drop
/// terminates the driver instead of masquerading as a peer stream end.
pub struct Demux<R, T> {
    speaker: Speaker,
    read: FrameRead<R>,
    senders: [Option<mpsc::Sender<Frame<T>>>; STREAM_COUNT],
}

impl<R, T> Demux<R, T>
where
    R: AsyncRead + Unpin,
    T: BorshDeserialize + Send + 'static,
{
    /// Drive frames until all logical streams close, returning the raw reader.
    pub async fn run(mut self) -> Result<R, DemuxError> {
        loop {
            let Some((stream, frame)) = self.read.frame().await.map_err(DemuxError::Codec)? else {
                return Err(DemuxError::PrematureEof {
                    origin: Origin::direction(self.speaker),
                    open: self.open_streams(),
                });
            };
            let index = usize::from(stream.index());
            let Some(send) = &self.senders[index] else {
                return Err(DemuxError::FrameAfterEnd {
                    origin: Origin::stream(self.speaker, stream),
                });
            };
            if matches!(frame, Frame::End(End::Stream)) {
                if send.is_closed() {
                    return Err(DemuxError::ReceiverDropped {
                        origin: Origin::stream(self.speaker, stream),
                    });
                }
                self.senders[index] = None;
                if self.senders.iter().all(Option::is_none) {
                    return Ok(self.read.into_inner());
                }
                continue;
            }
            send.send(frame)
                .await
                .map_err(|_| DemuxError::ReceiverDropped {
                    origin: Origin::stream(self.speaker, stream),
                })?;
        }
    }

    /// Collect the streams which have not yet delivered their stream end.
    fn open_streams(&self) -> Vec<Stream> {
        self.senders
            .iter()
            .enumerate()
            .filter_map(|(index, sender)| sender.as_ref().map(|_| stream_at(index)))
            .collect()
    }
}

/// Failure while receiving and routing peer frames.
#[derive(Debug, thiserror::Error)]
pub enum DemuxError {
    /// The incoming frame codec rejected bytes or the transport failed.
    #[error(transparent)]
    Codec(#[from] DecodeError),
    /// The transport ended while logical streams were still open.
    #[error("{origin}: transport ended with logical streams still open: {open:?}")]
    PrematureEof { origin: Origin, open: Vec<Stream> },
    /// A peer sent another frame after closing its logical stream.
    #[error("{origin}: peer sent a frame after closing the logical stream")]
    FrameAfterEnd { origin: Origin },
    /// The protocol consumer disappeared while its logical stream was open.
    ///
    /// The coordinator must prefer any semantic error which caused this local
    /// cancellation; this variant is only the standalone driver's diagnosis.
    #[error("{origin}: consumer dropped the logical stream")]
    ReceiverDropped { origin: Origin },
}
