use std::collections::VecDeque;
use std::pin::Pin;
use std::task::{Context, Poll};

use proptest::prelude::*;
use tokio::io::{AsyncRead, AsyncReadExt, ReadBuf};

use super::{CountedRead, Recorder};

/// A transport that delivers a scripted sequence of chunks, one chunk
/// per poll, so the counter sees deliveries at every buffer fill level.
struct Chunked {
    chunks: VecDeque<Vec<u8>>,
}

impl AsyncRead for Chunked {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        // An empty scripted chunk delivers nothing and would read as
        // EOF; skip to the next delivering chunk.
        while self.chunks.front().is_some_and(|chunk| chunk.is_empty()) {
            self.chunks.pop_front();
        }
        if let Some(chunk) = self.chunks.front_mut() {
            let take = chunk.len().min(buf.remaining());
            buf.put_slice(&chunk[..take]);
            chunk.drain(..take);
            if chunk.is_empty() {
                self.chunks.pop_front();
            }
        }
        Poll::Ready(Ok(()))
    }
}

/// The received-byte counter adds exactly the bytes each poll delivers,
/// split deliveries into one partially filled buffer included.
///
/// The delta is measured against the buffer's fill level before the
/// poll, never its absolute level after it: a `read_exact` that fills
/// the same buffer across two polls counts each delivery once.
#[test]
fn counted_read_counts_split_deliveries() {
    let recorder = Recorder::default();
    let mut read = CountedRead::new((&b"ab"[..]).chain(&b"c"[..]), recorder.clone());
    let mut buf = [0u8; 3];
    pollster::block_on(read.read_exact(&mut buf)).expect("three bytes arrive");
    assert_eq!(&buf, b"abc");
    assert_eq!(recorder.snapshot().bytes_received, 3);
}

proptest! {
    /// The received-byte counter equals exactly the bytes delivered, for
    /// any payload under any chunking: however the transport splits the
    /// stream across polls, each delivered byte is counted once.
    #[test]
    fn counted_read_counts_any_chunking(
        data in proptest::collection::vec(any::<u8>(), 0..200),
        cuts in proptest::collection::vec(any::<proptest::sample::Index>(), 0..8),
    ) {
        let mut bounds: Vec<usize> = cuts.iter().map(|cut| cut.index(data.len() + 1)).collect();
        bounds.sort_unstable();
        bounds.dedup();
        let mut chunks = VecDeque::new();
        let mut start = 0;
        for bound in bounds {
            chunks.push_back(data[start..bound].to_vec());
            start = bound;
        }
        chunks.push_back(data[start..].to_vec());

        let recorder = Recorder::default();
        let mut read = CountedRead::new(Chunked { chunks }, recorder.clone());
        let mut buf = vec![0u8; data.len()];
        pollster::block_on(read.read_exact(&mut buf)).expect("every chunk arrives");
        prop_assert_eq!(&buf, &data);
        prop_assert_eq!(recorder.snapshot().bytes_received, data.len() as u64);
    }
}
