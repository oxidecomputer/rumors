use std::pin::Pin;
use std::sync::{Arc, mpsc};
use std::task::{Context, Poll};
use std::time::Duration;

use futures::FutureExt;
use proptest::prelude::*;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, DuplexStream, ReadBuf};

use super::{Pool, STREAM_COUNT, io};
use crate::link::routed::header;
use crate::testing::run_to_quiescence;

/// A transport that can return another connection when polled or dropped.
struct CallbackConn {
    /// Carries bytes to and from the test's peer.
    io: DuplexStream,
    /// Runs on the first read after being set.
    on_read: Option<Box<dyn FnOnce() + Send>>,
    /// Runs when this connection is discarded.
    on_drop: Option<Box<dyn FnOnce() + Send>>,
}

impl CallbackConn {
    /// Create a callback-free connection and its in-memory peer.
    fn pair() -> (Self, DuplexStream) {
        let (io, remote) = tokio::io::duplex(64);
        (
            Self {
                io,
                on_read: None,
                on_drop: None,
            },
            remote,
        )
    }
}

impl Drop for CallbackConn {
    /// Run the configured destructor callback, if any.
    fn drop(&mut self) {
        if let Some(callback) = self.on_drop.take() {
            callback();
        }
    }
}

impl AsyncRead for CallbackConn {
    /// Run the read callback once, then poll the underlying connection.
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if let Some(callback) = self.on_read.take() {
            callback();
        }
        Pin::new(&mut self.io).poll_read(cx, buf)
    }
}

impl AsyncWrite for CallbackConn {
    /// Write through to the underlying connection.
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.io).poll_write(cx, buf)
    }

    /// Flush the underlying connection.
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.io).poll_flush(cx)
    }

    /// Close the underlying connection's write half.
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.io).poll_shutdown(cx)
    }
}

/// A mutex deadlock blocks polling itself, so bound it from another thread.
fn completes(test: impl FnOnce() + Send + 'static) {
    let (done, finished) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(test));
        done.send(outcome).unwrap();
    });
    let outcome = finished
        .recv_timeout(Duration::from_secs(10))
        .expect("pool access from the transport callback must finish");
    worker.join().unwrap();
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

/// Verify that a return from a read or drop callback remains usable.
fn check_return_from_callback(on_read: bool) {
    completes(move || {
        let pool = Arc::new(Pool::new());
        let (mut first, first_remote) = CallbackConn::pair();
        let (second, mut second_remote) = CallbackConn::pair();
        pollster::block_on(second_remote.write_all(&[header::READY])).unwrap();
        let weak = Arc::downgrade(&pool);
        let callback = Box::new(move || weak.upgrade().unwrap().put(second));
        if on_read {
            first.on_read = Some(callback);
        } else {
            first.on_drop = Some(callback);
        }
        pool.put(first);
        drop(first_remote);
        assert!(
            pool.take_ready().is_none(),
            "the first connection is closed"
        );
        let mut reused = pool
            .take_ready()
            .expect("the callback returned a ready connection");
        pollster::block_on(async {
            reused.write_all(b"reused").await.unwrap();
            let mut bytes = [0; 6];
            second_remote.read_exact(&mut bytes).await.unwrap();
            assert_eq!(&bytes, b"reused");
        });
    });
}

/// Discarding a closed connection permits its destructor to return another
/// connection to the pool, where it remains usable.
#[test]
fn destructor_can_return_to_pool() {
    check_return_from_callback(false);
}

/// Probing a connection permits transport code to return another connection
/// to the same pool without deadlocking or losing the return.
#[test]
fn read_callback_can_return_to_pool() {
    check_return_from_callback(true);
}

proptest! {
    /// A transport callback can take another ready connection while an outer
    /// probe holds the first; neither call loses or duplicates an entry.
    #[test]
    fn a_probe_allows_another_caller_to_take(count in 2usize..=STREAM_COUNT) {
        completes(move || {
            run_to_quiescence(async {
                let pool = Arc::new(Pool::new());
                let (taken, received) = mpsc::channel();
                let weak = Arc::downgrade(&pool);
                let mut nested = Some(Box::new(move || {
                    let conn = weak.upgrade().unwrap().take_ready()
                        .expect("another ready connection is available");
                    taken.send(conn).unwrap();
                }) as Box<dyn FnOnce() + Send>);
                let mut peers = Vec::new();
                for _ in 0..count {
                    let (mut conn, mut peer) = CallbackConn::pair();
                    conn.on_read = nested.take();
                    peer.write_u8(header::READY).await.unwrap();
                    peers.push(peer);
                    pool.put(conn);
                }
                let mut connections = vec![pool.take_ready().unwrap(), received.try_recv().unwrap()];
                while let Some(conn) = pool.take_ready() {
                    connections.push(conn);
                }
                assert_eq!(connections.len(), peers.len());
                for (mut conn, mut peer) in connections.into_iter().zip(peers) {
                    conn.write_u8(42).await.unwrap();
                    assert_eq!(peer.read_u8().await.unwrap(), 42);
                }
            }).expect("overlapping takes remain independent");
        });
    }

    /// Ready connections each get a turn despite repeated returns, including
    /// a return during probing. Pending connections do not block reuse and
    /// receive service once READY arrives; closed connections are discarded.
    #[test]
    fn reuse_preserves_ready_order(
        states in prop::collection::vec(prop::option::of(any::<bool>()), 0..STREAM_COUNT - 1),
    ) {
        completes(move || {
            run_to_quiescence(async {
                let pool = Arc::new(Pool::new());
                // None closes the peer; live peers either send READY or wait.
                let states: Vec<_> = std::iter::once(Some(true)).chain(states).collect();
                let mut peers = Vec::new();
                let mut connections = Vec::new();
                for state in &states {
                    let (conn, mut peer) = CallbackConn::pair();
                    if *state == Some(true) {
                        peer.write_u8(header::READY).await.unwrap();
                    }
                    connections.push(conn);
                    peers.push(state.map(|_| peer));
                }

                // This return arrives while the oldest connection is being
                // probed, and must not move ahead of older ready entries.
                let (returned, mut peer) = CallbackConn::pair();
                peer.write_u8(header::READY).await.unwrap();
                let returning = peers.len();
                peers.push(Some(peer));
                let weak = Arc::downgrade(&pool);
                connections[0].on_read = Some(Box::new(move || {
                    weak.upgrade().unwrap().put(returned);
                }));
                for conn in connections {
                    pool.put(conn);
                }

                let mut expected: Vec<_> = states.iter().enumerate()
                    .filter_map(|(index, state)| (*state == Some(true)).then_some(index))
                    .chain(std::iter::once(returning))
                    .collect();
                for phase in 0..2 {
                    if phase == 1 {
                        // Once every live connection is ready, none may get
                        // a second turn before all the others get their first.
                        let pending: Vec<_> = states.iter().enumerate()
                            .filter_map(|(index, state)| (*state == Some(false)).then_some(index))
                            .collect();
                        for &index in &pending {
                            peers[index].as_mut().unwrap().write_u8(header::READY).await.unwrap();
                        }
                        expected.extend(pending);
                    }
                    for _ in 0..2 {
                        let mut served = std::collections::BTreeSet::new();
                        for _ in 0..expected.len() {
                            let mut conn = pool.take_ready().expect("a connection is ready");
                            // Identify the selected connection through its peer,
                            // without inspecting the pool's storage.
                            conn.write_u8(42).await.unwrap();
                            let selected = peers.iter_mut().position(|peer| {
                                peer.as_mut().is_some_and(|peer| {
                                    matches!(peer.read_u8().now_or_never(), Some(Ok(42)))
                                })
                            }).expect("the selected connection delivered its byte");
                            assert!(served.insert(selected), "a connection got a second turn too soon");
                            if phase == 0 {
                                assert_eq!(selected, expected[0], "ready entries stay in return order");
                                expected.rotate_left(1);
                            }
                            pool.put(conn);
                            peers[selected].as_mut().unwrap().write_u8(header::READY).await.unwrap();
                        }
                        assert_eq!(served, expected.iter().copied().collect());
                    }
                }
            }).expect("ready connections can be reused without waiting");
        });
    }

    /// Connections being probed still occupy capacity: concurrent returns fill
    /// only the remaining slots, and dropping the pool releases every survivor.
    #[test]
    fn probing_preserves_idle_bound(held in 1usize..=STREAM_COUNT, returned in 1usize..=STREAM_COUNT) {
        completes(move || {
            let pool = Arc::new(Pool::new());
            let mut peers = Vec::new();
            let mut returning = Vec::new();
            for _ in 0..returned {
                let (conn, peer) = CallbackConn::pair();
                returning.push(conn);
                peers.push(peer);
            }
            let (mut first, peer) = CallbackConn::pair();
            peers.push(peer);
            let weak = Arc::downgrade(&pool);
            first.on_read = Some(Box::new(move || {
                let pool = weak.upgrade().unwrap();
                for conn in returning {
                    pool.put(conn);
                }
            }));
            pool.put(first);
            for _ in 1..held {
                let (conn, peer) = CallbackConn::pair();
                pool.put(conn);
                peers.push(peer);
            }
            assert!(pool.take_ready().is_none(), "no peer has sent READY");
            let mut retained = 0;
            for peer in &mut peers {
                match peer.read_u8().now_or_never() {
                    None => retained += 1,
                    Some(Err(error)) => assert_eq!(error.kind(), io::ErrorKind::UnexpectedEof),
                    Some(Ok(_)) => panic!("no payload was sent"),
                }
            }
            assert_eq!(retained, (held + returned).min(STREAM_COUNT));
            drop(pool);
            for mut peer in peers {
                assert_eq!(peer.read_u8().now_or_never().unwrap().unwrap_err().kind(),
                    io::ErrorKind::UnexpectedEof);
            }
        });
    }
}
