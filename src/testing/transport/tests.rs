//! Tests for the shared transport adversity wrappers.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use futures::poll;
use tokio::io::{AsyncReadExt, AsyncWriteExt, duplex, split};

use super::{IoPlan, Side, reorder_accepts, wrap_io, yield_once};
use crate::link::{Acceptor, Connector, memory};
use crate::testing::run_to_quiescence;

/// The patient wait forms a batch of two and releases it newest-first,
/// counting one inversion.
///
/// The second stream is connected only after the acceptor has held the
/// first and begun yielding, so a drain of only-`Ready` arrivals would
/// release the first alone and count nothing.
#[test]
fn reordering_acceptor_inverts_a_patient_batch() {
    let reordered = Arc::new(AtomicUsize::new(0));
    let (a, b) = memory();
    let mut acceptor = reorder_accepts(a, 2, reordered.clone()).acceptor;
    let connector = b.connector;
    let released = run_to_quiescence(async {
        let (_streams, released) = futures::join!(
            async {
                let (mut first, _done) = connector.connect().await.unwrap();
                first.write_all(b"1").await.unwrap();
                // Let the acceptor take the first arrival and start
                // waiting before the second exists.
                for _ in 0..4 {
                    yield_once().await;
                }
                let (mut second, _done) = connector.connect().await.unwrap();
                second.write_all(b"2").await.unwrap();
                (first, second)
            },
            async {
                let mut released = Vec::new();
                for _ in 0..2 {
                    let (mut rx, _done) = acceptor.accept().await.unwrap();
                    let mut tag = [0u8; 1];
                    rx.read_exact(&mut tag).await.unwrap();
                    released.push(tag[0]);
                }
                released
            },
        );
        released
    })
    .expect("the batch releases and the harness stays live");
    assert_eq!(released, b"21", "the batch of two releases newest-first");
    assert_eq!(
        reordered.load(Ordering::Relaxed),
        1,
        "one batch of two is one recorded inversion"
    );
}

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
