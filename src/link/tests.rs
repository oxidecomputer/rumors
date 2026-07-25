//! Contract tests for the in-memory link, run under the deterministic
//! closed-world harness so liveness failures surface as quiescence.

use futures::future::join;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::testing::run_to_quiescence;

use super::{Acceptor, Connector, STREAM_COUNT, memory, memory_with_capacity};

/// The link module's stream bound and the codec's logical stream count are
/// the same protocol constant, stated once in each layer's vocabulary.
#[test]
fn stream_count_matches_the_codec() {
    assert_eq!(
        STREAM_COUNT,
        usize::from(crate::tree::mirror::streaming::remote::codec_stream_count()),
    );
}

/// The control halves form two independent ordered byte pipes: bytes written
/// on each side arrive intact and in order on the other.
#[test]
fn control_carries_bytes_both_ways() {
    let (mut a, mut b) = memory();
    run_to_quiescence(async {
        let ping = async {
            a.control_write.write_all(b"ping").await.unwrap();
            let mut buf = [0u8; 4];
            a.control_read.read_exact(&mut buf).await.unwrap();
            assert_eq!(&buf, b"pong");
        };
        let pong = async {
            let mut buf = [0u8; 4];
            b.control_read.read_exact(&mut buf).await.unwrap();
            assert_eq!(&buf, b"ping");
            b.control_write.write_all(b"pong").await.unwrap();
        };
        join(ping, pong).await;
    })
    .expect("control exchange stays live");
}

/// An opened stream's bytes reach the peer's acceptor as one ordered stream,
/// and dropping the write half surfaces end-of-stream to the reader.
#[test]
fn connect_delivers_an_ordered_half_closing_stream() {
    let (a, mut b) = memory();
    run_to_quiescence(async {
        let send = async {
            let mut tx = a.connector.connect().await.unwrap();
            tx.write_all(b"hello").await.unwrap();
            drop(tx);
        };
        let receive = async {
            let mut rx = b.acceptor.accept().await.unwrap();
            let mut bytes = Vec::new();
            rx.read_to_end(&mut bytes).await.unwrap();
            assert_eq!(bytes, b"hello");
        };
        join(send, receive).await;
    })
    .expect("stream delivery stays live");
}

/// Streams are independent: a stream whose reader never drains does not
/// stop a later stream from opening, transferring, and closing.
#[test]
fn a_stalled_stream_does_not_couple_its_siblings() {
    let (a, mut b) = memory_with_capacity(4);
    run_to_quiescence(async {
        let send = async {
            let mut stalled = a.connector.connect().await.unwrap();
            // Fill the stalled stream's bounded buffer to its brim; its
            // reader never drains it, so more writes would block.
            stalled.write_all(&[0u8; 4]).await.unwrap();
            let mut live = a.connector.connect().await.unwrap();
            live.write_all(b"live").await.unwrap();
            drop(live);
            stalled
        };
        let receive = async {
            let _stalled = b.acceptor.accept().await.unwrap();
            let mut rx = b.acceptor.accept().await.unwrap();
            let mut bytes = Vec::new();
            rx.read_to_end(&mut bytes).await.unwrap();
            assert_eq!(bytes, b"live");
        };
        join(send, receive).await;
    })
    .expect("independent streams stay live beside a stalled one");
}

/// Backpressure is per-stream and receiver-paced: a writer past the buffer
/// capacity blocks until its own reader drains, then completes.
#[test]
fn stream_writes_block_on_their_own_reader() {
    let (a, mut b) = memory_with_capacity(2);
    run_to_quiescence(async {
        let send = async {
            let mut tx = a.connector.connect().await.unwrap();
            tx.write_all(b"abcdef").await.unwrap();
            drop(tx);
        };
        let receive = async {
            let mut rx = b.acceptor.accept().await.unwrap();
            let mut bytes = Vec::new();
            rx.read_to_end(&mut bytes).await.unwrap();
            assert_eq!(bytes, b"abcdef");
        };
        join(send, receive).await;
    })
    .expect("a two-byte window still carries six bytes");
}

/// Dropping one whole link fails the peer's supply cleanly: connect and
/// accept both report transport errors instead of hanging.
#[test]
fn dropping_the_peer_fails_the_supply() {
    let (a, b) = memory();
    drop(b);
    run_to_quiescence(async {
        a.connector
            .connect()
            .await
            .expect_err("connect to a dropped peer fails");
    })
    .expect("failed connect resolves");

    let (a, mut b) = memory();
    drop(a);
    run_to_quiescence(async {
        b.acceptor
            .accept()
            .await
            .expect_err("accept from a dropped peer fails");
    })
    .expect("failed accept resolves");
}

/// Epochs advance one per begun session and wrap: the label tripwire never
/// allocates, only counts. `finish` marks each session's clean end.
#[test]
fn epochs_count_and_wrap() {
    let (mut a, _b) = memory();
    assert_eq!(a.session.begin().expect("fresh link"), 0);
    a.session.finish();
    assert_eq!(a.session.begin().expect("clean boundary"), 1);
    a.session.finish();
    a.session.epoch = u8::MAX;
    assert_eq!(a.session.begin().expect("clean boundary"), u8::MAX);
    a.session.finish();
    assert_eq!(a.session.begin().expect("clean boundary"), 0);
}

/// `begin` latches the poison flag for the open session's whole duration
/// and only `finish` clears it: a session that never finishes leaves the
/// link failing every later `begin` fast, without burning further epochs.
#[test]
fn an_unfinished_session_poisons_every_later_begin() {
    let (mut a, _b) = memory();
    assert!(!a.session.poisoned);
    assert_eq!(a.session.begin().expect("fresh link"), 0);
    assert!(a.session.poisoned, "open sessions hold the latch");

    // The interrupted session never finishes: both later attempts fail
    // fast, and neither advances the counter past the interrupted session.
    for _ in 0..2 {
        assert!(
            matches!(a.session.begin(), Err(crate::Error::LinkPoisoned)),
            "a poisoned link must fail begin fast"
        );
    }
    assert_eq!(a.session.epoch, 1, "failed begins burn no epochs");

    // Clearing the latch (as a funnel does on clean completion) restores
    // the link; the next session takes the next epoch in sequence.
    a.session.finish();
    assert_eq!(a.session.begin().expect("cleared latch"), 1);
}
