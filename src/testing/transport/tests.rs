//! Tests for the shared transport adversity wrappers.

use std::io::IoSlice;

use futures::poll;
use proptest::prelude::*;
use tokio::io::{AsyncReadExt, AsyncWrite, AsyncWriteExt, duplex, split};

use super::{IoPlan, Side, wrap_io};
use crate::testing::run_to_quiescence;

/// Flush buffering keeps completed writes invisible to the peer until the
/// corresponding flush is polled.
#[test]
fn flush_buffering_withholds_bytes_until_flush() {
    let (left, right) = duplex(8);
    let (read, write) = split(left);
    let plan = IoPlan {
        hold_until_flush: true,
        ..IoPlan::default()
    };
    let (_read, mut write, report) = wrap_io(Side::Left, plan, read, write);
    let (mut peer_read, _peer_write) = split(right);

    let received = run_to_quiescence(async {
        write.write_all(b"abcd").await.unwrap();

        let mut bytes = [0; 4];
        let mut receive = std::pin::pin!(peer_read.read_exact(&mut bytes));
        assert!(poll!(receive.as_mut()).is_pending());

        write.flush().await.unwrap();
        receive.await.unwrap();
        bytes
    })
    .expect("the buffered transport should remain live");

    assert_eq!(received, *b"abcd");
    let snapshot = report.snapshot();
    assert_eq!(snapshot.writes, 1);
    assert_eq!(snapshot.write_bytes, 4);
    assert_eq!(snapshot.flushes, 1);
}

/// Fragmentation, delays, and flush buffering compose without losing bytes.
#[test]
fn successful_adversity_is_lossless() {
    let (left, right) = duplex(1);
    let (read, write) = split(left);
    let plan = IoPlan {
        read_chunk: 1,
        write_chunk: 2,
        read_delays: vec![1; 8],
        write_delays: vec![1; 8],
        flush_delays: vec![1],
        hold_until_flush: true,
        vectored: false,
        fault: None,
    };
    let (mut read, mut write, report) = wrap_io(Side::Left, plan, read, write);
    let (mut peer_read, mut peer_write) = split(right);
    let (_, received, peer_received) = run_to_quiescence(async {
        futures::join!(
            async {
                write.write_all(b"abcd").await.unwrap();
                write.flush().await.unwrap();
            },
            async {
                let mut bytes = [0; 2];
                read.read_exact(&mut bytes).await.unwrap();
                bytes
            },
            async {
                let mut bytes = [0; 4];
                peer_read.read_exact(&mut bytes).await.unwrap();
                peer_write.write_all(b"xy").await.unwrap();
                peer_write.flush().await.unwrap();
                bytes
            },
        )
    })
    .expect("the closed transport should remain live");
    assert_eq!(received, *b"xy");
    assert_eq!(peer_received, *b"abcd");
    let snapshot = report.snapshot();
    assert_eq!(snapshot.write_bytes, 4);
    assert_eq!(snapshot.read_bytes, 2);
    assert!(snapshot.delayed_polls > 0);
}

proptest! {
    /// A vectored adversarial writer takes up to `write_chunk` bytes across a
    /// vectored write's slices, in order, as one operation.
    ///
    /// A writer that is not vectored takes at most `write_chunk` bytes of
    /// the first nonempty slice, as the trait's default does. Either way the
    /// bytes it takes reach the peer, at once or at the flush.
    #[test]
    fn a_vectored_write_takes_up_to_a_chunk_across_slices(
        slices in proptest::collection::vec(proptest::collection::vec(any::<u8>(), 0..8), 1..5),
        chunk in 1usize..32,
        vectored in any::<bool>(),
        hold_until_flush in any::<bool>(),
    ) {
        let (left, mut peer) = duplex(1024);
        let (read, write) = split(left);
        let plan = IoPlan {
            write_chunk: chunk,
            vectored,
            hold_until_flush,
            ..IoPlan::default()
        };
        let (_read, mut write, report) = wrap_io(Side::Left, plan, read, write);
        prop_assert_eq!(write.is_write_vectored(), vectored);
        let io_slices: Vec<IoSlice<'_>> = slices.iter().map(|slice| IoSlice::new(slice)).collect();
        let (taken, received) = run_to_quiescence(async {
            let taken = write.write_vectored(&io_slices).await.unwrap();
            write.flush().await.unwrap();
            let mut received = vec![0; taken];
            peer.read_exact(&mut received).await.unwrap();
            (taken, received)
        })
        .expect("an in-memory write and flush complete");
        let offered: Vec<u8> = if vectored {
            slices.concat()
        } else {
            slices.iter().find(|slice| !slice.is_empty()).cloned().unwrap_or_default()
        };
        prop_assert_eq!(taken, chunk.min(offered.len()));
        prop_assert_eq!(&received[..], &offered[..taken]);
        prop_assert_eq!(report.snapshot().writes, usize::from(taken > 0));
    }
}
