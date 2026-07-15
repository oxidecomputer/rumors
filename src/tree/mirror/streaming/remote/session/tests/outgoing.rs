//! Outgoing scheduling and physical-write acknowledgement properties.

use std::{
    pin::Pin,
    sync::atomic::Ordering,
    task::{Context, Poll},
};

use futures::poll;
use tokio::io::AsyncWrite;

use super::super::{MuxError, STREAM_COUNT, outgoing as build_outgoing, stream_at};
use super::{RecordingWriter, SPEAKERS, closing_frame};
use crate::tree::mirror::streaming::remote::codec::{
    EncodeErrorKind, Flow, Frame, Origin, Reaction, Speaker, Stream, decode,
};

/// When every stream is ready together, the mux emits complete frames from
/// the leaf-most stream upward and acknowledges every physical write.
#[tokio::test]
async fn prefers_the_bottom_most_ready_stream() {
    for speaker in SPEAKERS {
        let (mux, mut outgoing) = build_outgoing(speaker, RecordingWriter::default());
        let mut senders = (0..STREAM_COUNT)
            .map(|index| outgoing.take(stream_at(index)))
            .collect::<Vec<_>>();
        let mut sending = senders
            .iter_mut()
            .enumerate()
            .map(|(index, sender)| {
                Box::pin(sender.frame(closing_frame::<()>(speaker, stream_at(index))))
            })
            .collect::<Vec<_>>();
        for frame in &mut sending {
            assert!(poll!(frame).is_pending());
        }

        let written = mux.run().await.unwrap().bytes;
        for frame in sending {
            frame.await.unwrap();
        }

        let mut rest = written.as_slice();
        for index in (0..STREAM_COUNT).rev() {
            let stream = stream_at(index);
            assert_eq!(
                decode::<()>(speaker, &mut rest).unwrap(),
                (stream, closing_frame(speaker, stream)),
            );
        }
        assert!(rest.is_empty());
    }
}

/// Cancelling one frame wait cannot leave an acknowledgement that lets the
/// following frame return before its own transport flush.
#[tokio::test]
async fn acknowledgements_are_bound_to_their_exact_frame() {
    let speaker = Speaker::Responder;
    let logical = stream_at(8);
    let writer = RecordingWriter::default();
    let flushes = writer.flushes.clone();
    let (mux, mut outgoing) = build_outgoing(speaker, writer);
    let mut sender = outgoing.take(logical);

    {
        let first = Frame::Reaction(Reaction::<()>::Match, Flow::Continue);
        let mut cancelled = Box::pin(sender.frame(first));
        assert!(poll!(&mut cancelled).is_pending());
    }

    let mut running = Box::pin(mux.run());
    let second = closing_frame::<()>(speaker, logical);
    let mut sending = Box::pin(sender.frame(second));
    tokio::select! {
        result = &mut sending => result.unwrap(),
        result = &mut running => panic!("mux stopped before the second frame: {result:?}"),
    }
    assert_eq!(flushes.load(Ordering::Relaxed), 2);

    let dropped = stream_at(7);
    drop(outgoing.take(dropped));
    assert!(matches!(
        running.await,
        Err(MuxError::SenderDropped { origin })
            if origin == Origin::stream(Speaker::Responder, dropped)
    ));
}

/// A logical producer must finish with a stream end rather than silently
/// disappearing and leaving session completion permanently ambiguous.
#[tokio::test]
async fn reports_a_sender_dropped_before_stream_end() {
    let (mux, mut outgoing) =
        build_outgoing::<_, ()>(Speaker::Initiator, RecordingWriter::default());
    let dropped = stream_at(Stream::MAX as usize);
    drop(outgoing.take(dropped));
    assert!(matches!(
        mux.run().await,
        Err(MuxError::SenderDropped { origin })
            if origin == Origin::stream(Speaker::Initiator, dropped)
    ));
}

/// Writer which accepts every byte but fails the required flush.
struct FlushFailure;

impl AsyncWrite for FlushFailure {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Poll::Ready(Ok(bytes.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Err(std::io::ErrorKind::Other.into()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

/// A queued frame is not acknowledged when its bytes were accepted but its
/// required flush failed, while the mux retains the contextual codec error.
#[tokio::test]
async fn acknowledges_only_after_a_successful_flush() {
    let speaker = Speaker::Initiator;
    let stream = stream_at(8);
    let (mux, mut outgoing) = build_outgoing(speaker, FlushFailure);
    let mut sender = outgoing.take(stream);
    let mut sending = Box::pin(sender.frame(closing_frame::<()>(speaker, stream)));
    assert!(poll!(&mut sending).is_pending());

    let Err(MuxError::Codec(error)) = mux.run().await else {
        panic!("flush failure must retain its codec context");
    };
    assert!(matches!(error.kind, EncodeErrorKind::Flush(_)));
    assert!(sending.await.is_err());
}
