//! Shared harness for the two physical session directions.

use std::{
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll},
};

use tokio::io::AsyncWrite;

use super::super::*;
use crate::tree::mirror::streaming::remote::codec::{
    End, Flow, Frame, Reaction, Speaker, Stream, WireFrame,
};

mod coordinator;
mod incoming;
mod outgoing;

const SPEAKERS: [Speaker; 2] = [Speaker::Initiator, Speaker::Responder];

/// Produce the phase-valid frame which closes one test stream.
fn closing_frame<T>(speaker: Speaker, stream: Stream) -> Frame<T> {
    if speaker == Speaker::Initiator && stream.index() == 0 {
        Frame::Reaction(Reaction::Query(Vec::new()), Flow::End(End::Stream))
    } else {
        Frame::End(End::Stream)
    }
}

/// Concatenate canonical frames into one physical transport direction.
fn encoded<T>(speaker: Speaker, frames: impl IntoIterator<Item = WireFrame<T>>) -> Vec<u8> {
    let mut bytes = Vec::new();
    for frame in frames {
        crate::tree::mirror::streaming::remote::codec::encode(speaker, &frame, &mut bytes).unwrap();
    }
    bytes
}

/// Async writer which records bytes and the number of completed flushes.
#[derive(Debug, Default)]
struct RecordingWriter {
    bytes: Vec<u8>,
    flushes: Arc<AtomicUsize>,
}

impl AsyncWrite for RecordingWriter {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        self.bytes.extend_from_slice(bytes);
        Poll::Ready(Ok(bytes.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.flushes.fetch_add(1, Ordering::Relaxed);
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}
