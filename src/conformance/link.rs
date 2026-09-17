//! Conformance checks for caller-built [`Link`] implementations.
//!
//! The checks exercise the [link contract](crate::link): independent control
//! and data traffic, concurrent opens, stream completion and abort, and
//! cancellation of incoming delivery. Both ends are tested, including through
//! cloned connectors and with control and data traffic active together.
//!
//! # Using the suite
//!
//! Supply a connected-pair factory and a deadline factory:
//!
//! ```
//! # tokio::runtime::Builder::new_current_thread().enable_time().build().unwrap().block_on(async {
//! rumors::conformance::link::check(
//!     async || rumors::link::memory(),
//!     || tokio::time::sleep(std::time::Duration::from_secs(30)),
//! ).await;
//! # });
//! ```
//!
//! Each check starts a fresh deadline before constructing its pair. If the
//! deadline completes, the check is cancelled and panics with its name. The
//! focused checks take the same two arguments as [`check`].
//!
//! The caller supplies the clock and executor; this suite requires no runtime.
//! With a deterministic driver that detects stalled futures, use
//! [`std::future::pending`] as the deadline factory. Deadlines are cooperative:
//! they cannot interrupt transport code that never returns from a poll.
//!
//! # What the suite cannot see
//!
//! Passing checks is evidence for the tested traffic and schedules. These
//! obligations still need review of the implementation and its configuration:
//!
//! - **Buffer bounds.** Pressure probes may miss coupling hidden behind large
//!   buffers. They do not require a writer to reach backpressure, so an
//!   unbounded implementation can pass. Control exchanges write
//!   [`CONTROL_DUPLEX_FILL`] bytes each way; data pressure lasts until the
//!   accompanying traffic finishes. Check the complete buffering path and any
//!   shared pool against the [link contract](crate::link#pooled-flow-control).
//! - **Failure handling and security.** A connected pair does not expose
//!   authentication, authorization, encryption, replay protection, or transport
//!   fault injection. The suite also cannot validate a routing deadline or
//!   fairness among links sharing a listener. Those need transport-specific
//!   tests. Errors from `connect` and `accept` must still indicate transport
//!   failure as required by their contracts.

use std::future::Future;
use std::io;
use std::pin::pin;

mod contention;
pub use contention::check_control_data_independence;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll};

use futures::future::{Either, join, join_all, select};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use super::timed;
use crate::link::{Acceptor, Connector, Done, Link, STREAM_COUNT, yield_once};
use crate::{Peer, Rumors};

/// Bytes used to probe stream delivery without assuming any capacity.
const PROBE: &[u8] = b"rumors-conformance-probe";

/// Control probe payload in the a-to-b direction.
///
/// The two directions carry distinct, equal-length payloads so a wiring
/// that loops a control half back onto its own side fails the byte
/// assertion loudly instead of only hanging on a read that never resolves.
const CONTROL_PROBE_AB: &[u8] = b"rumors-conformance-control:a>b";

/// Control probe payload in the b-to-a direction; see [`CONTROL_PROBE_AB`].
const CONTROL_PROBE_BA: &[u8] = b"rumors-conformance-control:b>a";

/// Bytes each direction exchanges in the control-duplex probe.
///
/// Buffers that absorb this entire exchange can hide direction coupling.
/// The size keeps the check affordable even with one-byte stream windows.
pub const CONTROL_DUPLEX_FILL: usize = 32 * 1024;

/// Direction tag folded into the control-duplex payload a writes to b.
const CONTROL_DUPLEX_TAG_AB: u8 = b'a';

/// Direction tag folded into the control-duplex payload b writes to a.
///
/// Distinct from [`CONTROL_DUPLEX_TAG_AB`] so looped-back or cross-wired
/// control halves fail the byte assertion loudly rather than only hanging.
const CONTROL_DUPLEX_TAG_BA: u8 = b'b';

/// First byte of the independence probe's stalled stream.
///
/// Streams are classified in-band because the contract lets an acceptor
/// yield them in any order: assuming the stalled stream arrives first would
/// hang the check against a conforming reordering acceptor.
const STALLED_TAG: u8 = b'S';

/// First byte of every live stream in the independence probe.
const LIVE_TAG: u8 = b'L';

/// Encode a probe stream's position in its one-byte in-band label.
fn stream_tag(index: usize) -> u8 {
    u8::try_from(index).expect("STREAM_COUNT fits in one label byte")
}

/// Bytes per write the stalled stream's writer keeps issuing while the live
/// complement runs, so per-stream buffering that hides cross-stream
/// coupling fills while the probe is still watching.
const STALL_FILL: &[u8] = &[STALLED_TAG; 512];

/// Cancelled accepts allowed to return no stream before collecting normally.
///
/// This bounds the sampled schedule, not delivery time. Normal accepts still
/// require every stream to arrive, so slow arrivals do not fail a healthy link.
const CANCEL_DROP_PATIENCE: usize = 32;

/// Payloads per side in the shallow session check.
///
/// The check requires data streams to open in both directions. Tree depth
/// determines how many; [`check_concurrency`] separately exercises the limit.
const SESSION_PAYLOADS: u64 = 48;

// Keep Link's four transport parameters explicit in these public signatures:
// rustdoc then shows which halves each focused check actually uses, without a
// public alias trait that would merely move the same bounds out of sight.
/// Run the whole conformance suite against fresh pairs from `pair`.
///
/// Each check calls `deadline` once, then constructs and tests a fresh pair.
/// The deadline covers both construction and the check's work. Every check
/// exercises both ends, including asymmetric connectors and acceptors.
///
/// # Panics
///
/// On a contract violation or expired deadline. A deadline failure names the
/// check that timed out.
pub async fn check<CRa, CWa, Ca, Aa, CRb, CWb, Cb, Ab, D>(
    mut pair: impl AsyncFnMut() -> (Link<CRa, CWa, Ca, Aa>, Link<CRb, CWb, Cb, Ab>),
    mut deadline: impl FnMut() -> D,
) where
    CRa: AsyncRead + Unpin + Send,
    CWa: AsyncWrite + Unpin + Send,
    Ca: Connector,
    Aa: Acceptor,
    CRb: AsyncRead + Unpin + Send,
    CWb: AsyncWrite + Unpin + Send,
    Cb: Connector,
    Ab: Acceptor,
    D: Future<Output = ()>,
{
    check_control(&mut pair, &mut deadline).await;
    check_control_duplex(&mut pair, &mut deadline).await;
    check_control_data_independence(&mut pair, &mut deadline).await;
    check_streams(&mut pair, &mut deadline).await;
    check_independence(&mut pair, &mut deadline).await;
    check_concurrency(&mut pair, &mut deadline).await;
    check_accept_cancellation(&mut pair, &mut deadline).await;
    check_sessions(&mut pair, &mut deadline).await;
}

/// The control halves deliver each direction's bytes intact and in order.
///
/// Distinct payloads in each direction detect bytes delivered to the wrong end.
///
/// The deadline covers pair construction and this check.
pub async fn check_control<CRa, CWa, Ca, Aa, CRb, CWb, Cb, Ab, D>(
    pair: impl AsyncFnOnce() -> (Link<CRa, CWa, Ca, Aa>, Link<CRb, CWb, Cb, Ab>),
    deadline: impl FnOnce() -> D,
) where
    CRa: AsyncRead + Unpin,
    CWa: AsyncWrite + Unpin,
    CRb: AsyncRead + Unpin,
    CWb: AsyncWrite + Unpin,
    D: Future<Output = ()>,
{
    timed("check_control", deadline(), async {
        let (mut a, mut b) = pair().await;
        let (a_read, a_write) = (&mut a.control_read, &mut a.control_write);
        let (b_read, b_write) = (&mut b.control_read, &mut b.control_write);
        let ping = async {
            a_write
                .write_all(CONTROL_PROBE_AB)
                .await
                .expect("contract: control writes succeed while the peer link lives");
            a_write.flush().await.expect("contract: control flush");
            let mut bytes = vec![0u8; CONTROL_PROBE_BA.len()];
            a_read
                .read_exact(&mut bytes)
                .await
                .expect("contract: control delivers the peer's bytes");
            assert_eq!(
                bytes, CONTROL_PROBE_BA,
                "contract: control delivers the peer's bytes in order, not this side's own",
            );
        };
        let pong = async {
            let mut bytes = vec![0u8; CONTROL_PROBE_AB.len()];
            b_read
                .read_exact(&mut bytes)
                .await
                .expect("contract: control delivers the peer's bytes");
            assert_eq!(
                bytes, CONTROL_PROBE_AB,
                "contract: control delivers the peer's bytes in order, not this side's own",
            );
            b_write
                .write_all(CONTROL_PROBE_BA)
                .await
                .expect("contract: control writes succeed while the peer link lives");
            b_write.flush().await.expect("contract: control flush");
        };
        join(ping, pong).await;
    })
    .await;
}

/// Both control directions must progress during simultaneous writes and reads.
///
/// Each side exchanges [`CONTROL_DUPLEX_FILL`] bytes, which must exceed the
/// transport's buffering to expose coupling. If each side's read waits on its
/// blocked write, the check cannot finish before its deadline.
///
/// The deadline covers pair construction and this check.
pub async fn check_control_duplex<CRa, CWa, Ca, Aa, CRb, CWb, Cb, Ab, D>(
    pair: impl AsyncFnOnce() -> (Link<CRa, CWa, Ca, Aa>, Link<CRb, CWb, Cb, Ab>),
    deadline: impl FnOnce() -> D,
) where
    CRa: AsyncRead + Unpin,
    CWa: AsyncWrite + Unpin,
    CRb: AsyncRead + Unpin,
    CWb: AsyncWrite + Unpin,
    D: Future<Output = ()>,
{
    timed("check_control_duplex", deadline(), async {
        let (mut a, mut b) = pair().await;
        let a_side = duplex_exchange(
            &mut a.control_read,
            &mut a.control_write,
            CONTROL_DUPLEX_TAG_AB,
            CONTROL_DUPLEX_TAG_BA,
        );
        let b_side = duplex_exchange(
            &mut b.control_read,
            &mut b.control_write,
            CONTROL_DUPLEX_TAG_BA,
            CONTROL_DUPLEX_TAG_AB,
        );
        join(a_side, b_side).await;
    })
    .await;
}

/// One side of [`check_control_duplex`]: write this side's fill while
/// concurrently draining the peer's.
///
/// Both sides write a buffer-exceeding payload concurrently, and each
/// side's read must drain the peer's payload while its own write is still
/// in flight: the exact shape of the protocol's oversized control
/// exchanges.
async fn duplex_exchange<R: AsyncRead + Unpin, W: AsyncWrite + Unpin>(
    read: &mut R,
    write: &mut W,
    write_tag: u8,
    read_tag: u8,
) {
    let send = async move {
        write
            .write_all(&duplex_fill(write_tag))
            .await
            .expect("contract: a backpressured control write blocks rather than failing");
        write.flush().await.expect("contract: control flush");
    };
    let receive = async move {
        let mut bytes = vec![0u8; CONTROL_DUPLEX_FILL];
        read.read_exact(&mut bytes)
            .await
            .expect("contract: control reads progress while this side's write is blocked");
        assert_eq!(
            bytes,
            duplex_fill(read_tag),
            "contract: the control stream delivers the peer's exact bytes in order",
        );
    };
    join(send, receive).await;
}

/// The control-duplex payload for one direction: position-dependent bytes
/// folded with the direction tag, so reordered, misrouted, or looped-back
/// bytes fail the equality assertion instead of only hanging.
fn duplex_fill(tag: u8) -> Vec<u8> {
    (0..CONTROL_DUPLEX_FILL).map(|i| (i as u8) ^ tag).collect()
}

/// Streams preserve accepted bytes on completion and abort, even without a flush.
///
/// Abort must deliver EOF after those bytes. Completion must preserve the
/// stream supply: later streams still flow while either end of an earlier
/// stream has yet to complete. Both directions are exercised.
///
/// The deadline covers pair construction and this check.
pub async fn check_streams<CRa, CWa, Ca, Aa, CRb, CWb, Cb, Ab, D>(
    pair: impl AsyncFnOnce() -> (Link<CRa, CWa, Ca, Aa>, Link<CRb, CWb, Cb, Ab>),
    deadline: impl FnOnce() -> D,
) where
    Ca: Connector,
    Aa: Acceptor,
    Cb: Connector,
    Ab: Acceptor,
    D: Future<Output = ()>,
{
    timed("check_streams", deadline(), async {
        let (mut a, mut b) = pair().await;
        probe_stream(&a.connector, &mut b.acceptor).await;
        probe_stream(&b.connector, &mut a.acceptor).await;
        probe_completed_streams(&a.connector, &mut b.acceptor).await;
        probe_completed_streams(&b.connector, &mut a.acceptor).await;
        for receiver_first in [false, true] {
            join(
                probe_delayed_completion(
                    &a.connector,
                    &mut b.acceptor,
                    receiver_first,
                    STREAM_COUNT + 1,
                ),
                probe_delayed_completion(
                    &b.connector,
                    &mut a.acceptor,
                    receiver_first,
                    STREAM_COUNT + 1,
                ),
            )
            .await;
        }
    })
    .await;
}

/// One direction of [`check_streams`]: a single stream, delivered exactly
/// and ended by the abort (the halves dropped, their handles unused).
async fn probe_stream<C: Connector, A: Acceptor>(connector: &C, acceptor: &mut A) {
    let send = async {
        let (mut tx, done) = connector
            .connect()
            .await
            .expect("contract: connect succeeds while the peer link lives");
        tx.write_all(PROBE).await.expect("contract: stream write");
        drop((tx, done));
    };
    let receive = async {
        let (mut rx, done) = acceptor
            .accept()
            .await
            .expect("contract: an opened stream is accepted");
        let mut bytes = Vec::new();
        rx.read_to_end(&mut bytes)
            .await
            .expect("contract: an abort surfaces as end-of-stream");
        assert_eq!(
            bytes, PROBE,
            "contract: a stream delivers its exact bytes in order",
        );
        drop((rx, done));
    };
    join(send, receive).await;
}

/// The completion leg of [`check_streams`]: two consecutive streams,
/// each ended by completing the write half at its final byte.
///
/// The receiver reads exactly the probe and completes its half there,
/// never probing for end-of-stream: the contract's completion clause
/// leaves what follows the data transport-defined (nothing at all, on
/// an instantiation that recovers the connection). The second stream
/// proves the supply survives the first one's completion, wherever its
/// connection went.
async fn probe_completed_streams<C: Connector, A: Acceptor>(connector: &C, acceptor: &mut A) {
    for sequence in 0..2u8 {
        let payload = [sequence; PROBE.len()];
        let send = async {
            let (mut tx, done) = connector
                .connect()
                .await
                .expect("contract: connect succeeds while the peer link lives");
            tx.write_all(&payload)
                .await
                .expect("contract: stream write");
            done.complete(tx);
        };
        let receive = async {
            let (mut rx, done) = acceptor
                .accept()
                .await
                .expect("contract: an opened stream is accepted");
            let mut bytes = vec![0u8; PROBE.len()];
            rx.read_exact(&mut bytes)
                .await
                .expect("contract: a completed stream delivers its bytes");
            assert_eq!(
                bytes, payload,
                "contract: a stream delivers its exact bytes in order",
            );
            done.complete(rx);
        };
        join(send, receive).await;
    }
}

/// Completing either half must not make later streams wait for the other half.
async fn probe_delayed_completion<C: Connector, A: Acceptor>(
    connector: &C,
    acceptor: &mut A,
    receiver_first: bool,
    rounds: usize,
) {
    let (tx, rx) = join(connector.connect(), acceptor.accept()).await;
    let (mut tx, tx_done) = tx.expect("contract: connect");
    let (mut rx, rx_done) = rx.expect("contract: accept");
    let (sent, received) = join(
        async {
            tx.write_u8(0).await?;
            tx.flush().await
        },
        rx.read_u8(),
    )
    .await;
    sent.expect("contract: stream write");
    assert_eq!(received.expect("contract: stream read"), 0);
    // Return one end while retaining the other. A reusing transport must
    // leave this connection alone and allow other streams to proceed.
    let held = if receiver_first {
        rx_done.complete(rx);
        Either::Left((tx, tx_done))
    } else {
        tx_done.complete(tx);
        Either::Right((rx, rx_done))
    };
    for round in 0..rounds {
        let payload = (round + 1) as u8;
        let send = async {
            let (mut tx, done) = connector
                .connect()
                .await
                .expect("contract: independent open");
            tx.write_u8(payload).await.expect("contract: stream write");
            tx.flush().await.expect("contract: stream flush");
            done.complete(tx);
        };
        let receive = async {
            let (mut rx, done) = acceptor
                .accept()
                .await
                .expect("contract: independent accept");
            assert_eq!(
                rx.read_u8().await.expect("contract: independent delivery"),
                payload,
                "contract: completion preserves stream boundaries"
            );
            done.complete(rx);
        };
        join(send, receive).await;
    }
    match held {
        Either::Left((tx, done)) => done.complete(tx),
        Either::Right((rx, done)) => done.complete(rx),
    }
}

/// Unread streams must not block other streams.
///
/// Both directions are tested with one unread stream and then with all but
/// one stream unread. The latter can exhaust a shared buffer pool that a
/// single stream cannot fill. All remaining streams must deliver and close.
/// Coupling hidden behind larger buffers may pass; see the module docs.
///
/// The deadline covers pair construction and this check.
pub async fn check_independence<CRa, CWa, Ca, Aa, CRb, CWb, Cb, Ab, D>(
    pair: impl AsyncFnOnce() -> (Link<CRa, CWa, Ca, Aa>, Link<CRb, CWb, Cb, Ab>),
    deadline: impl FnOnce() -> D,
) where
    Ca: Connector,
    Aa: Acceptor,
    Cb: Connector,
    Ab: Acceptor,
    D: Future<Output = ()>,
{
    timed("check_independence", deadline(), async {
        let (mut a, mut b) = pair().await;
        for stalled in [1, STALLED_COMPLEMENT] {
            probe_independence(&a.connector, &mut b.acceptor, stalled).await;
            probe_independence(&b.connector, &mut a.acceptor, stalled).await;
        }
    })
    .await;
}

/// One direction of [`check_independence`] with `stalled` unread streams.
///
/// Each stalled stream is held unread past an identifying first byte and
/// pressured while the remaining streams flow. Running this with one stalled
/// stream exposes direct coupling; running it with all but one stalled exposes
/// shared pools that one stream cannot fill. Tags identify streams because an
/// acceptor may return them in any order.
async fn probe_independence<C: Connector, A: Acceptor>(
    connector: &C,
    acceptor: &mut A,
    stalled_count: usize,
) {
    assert!((1..STREAM_COUNT).contains(&stalled_count));
    let send = async {
        let mut stalled = Vec::with_capacity(stalled_count);
        for _ in 0..stalled_count {
            let (mut tx, _) = connector
                .connect()
                .await
                .expect("contract: connect succeeds");
            tx.write_all(&[STALLED_TAG])
                .await
                .expect("contract: one byte lands within any legal window");
            tx.flush().await.expect("contract: stream flush");
            stalled.push(tx);
        }
        let pressure = join_all(stalled.iter_mut().map(press));
        let live = async {
            for _ in stalled_count..STREAM_COUNT {
                let (mut tx, _) = connector.connect().await.expect("contract: connect");
                tx.write_all(&[LIVE_TAG])
                    .await
                    .expect("contract: stream write");
                tx.write_all(PROBE).await.expect("contract: stream write");
                tx.flush().await.expect("contract: stream flush");
                drop(tx);
            }
        };
        match select(pin!(pressure), pin!(live)).await {
            Either::Left(_) => unreachable!("contract: pressure never completes"),
            Either::Right(((), _)) => {}
        }
    };
    let receive = async {
        let mut held = Vec::with_capacity(stalled_count);
        let mut live_seen = 0usize;
        for _ in 0..STREAM_COUNT {
            let (mut rx, _) = acceptor
                .accept()
                .await
                .expect("contract: later streams are accepted beside a stalled one");
            let mut tag = [0u8; 1];
            rx.read_exact(&mut tag)
                .await
                .expect("contract: every stream's first byte is delivered");
            match tag[0] {
                STALLED_TAG => {
                    held.push(rx);
                }
                LIVE_TAG => {
                    let mut bytes = Vec::new();
                    rx.read_to_end(&mut bytes)
                        .await
                        .expect("contract: later streams deliver beside a stalled one");
                    assert_eq!(bytes, PROBE, "contract: exact bytes on every stream");
                    live_seen += 1;
                }
                other => {
                    panic!("contract: a stream delivered a byte it was never sent: {other:#04x}")
                }
            }
        }
        assert_eq!(
            live_seen,
            STREAM_COUNT - stalled_count,
            "contract: every live stream is delivered beside stalled streams",
        );
        assert_eq!(
            held.len(),
            stalled_count,
            "contract: every stalled stream is delivered once",
        );
        // Retain unread receivers until the live complement ends and drops the
        // pressure futures; releasing them earlier could hide coupling.
        held
    };
    join(send, receive).await;
}

/// Stalled streams the pooled half of the independence check holds unread
/// at once: every data stream but the one that must keep flowing.
const STALLED_COMPLEMENT: usize = STREAM_COUNT - 1;

/// Keep writing to an unread stream, yielding if the transport keeps accepting.
async fn press<W: AsyncWrite + Unpin>(write: &mut W) {
    loop {
        write
            .write_all(STALL_FILL)
            .await
            .expect("contract: backpressure blocks instead of failing");
        write.flush().await.expect("contract: stream flush");
        yield_once().await;
    }
}

/// All data streams can stay open and flow in both directions at once.
///
/// Opens run both sequentially and concurrently, through the connector and
/// its clone. The last stream must finish while earlier receivers stay idle.
/// Backpressure is exercised only to the extent described in the module docs.
///
/// The deadline covers pair construction and this check.
pub async fn check_concurrency<CRa, CWa, Ca, Aa, CRb, CWb, Cb, Ab, D>(
    pair: impl AsyncFnOnce() -> (Link<CRa, CWa, Ca, Aa>, Link<CRb, CWb, Cb, Ab>),
    deadline: impl FnOnce() -> D,
) where
    Ca: Connector,
    Aa: Acceptor,
    Cb: Connector,
    Ab: Acceptor,
    D: Future<Output = ()>,
{
    timed("check_concurrency", deadline(), async {
        let (mut a, mut b) = pair().await;
        for concurrent in [false, true] {
            join(
                probe_concurrency(&a.connector, &mut b.acceptor, concurrent),
                probe_concurrency(&b.connector, &mut a.acceptor, concurrent),
            )
            .await;
        }
    })
    .await;
}

/// Open and identify a full set of streams without relying on arrival order.
async fn open_streams<C: Connector, A: Acceptor>(
    connector: &C,
    acceptor: &mut A,
    concurrent: bool,
) -> (Vec<C::Tx>, Vec<A::Rx>) {
    let cloned = connector.clone();
    let open = |index: usize| {
        let connector = if index.is_multiple_of(2) {
            connector
        } else {
            &cloned
        };
        async move {
            let (mut tx, _) = connector
                .connect()
                .await
                .expect("contract: concurrent connect");
            tx.write_u8(stream_tag(index))
                .await
                .expect("contract: stream write");
            tx.flush().await.expect("contract: stream flush");
            tx
        }
    };
    let send = async {
        if concurrent {
            join_all((0..STREAM_COUNT).map(open)).await
        } else {
            let mut held = Vec::with_capacity(STREAM_COUNT);
            for index in 0..STREAM_COUNT {
                held.push(open(index).await);
            }
            held
        }
    };
    let receive = async {
        let mut held: Vec<Option<A::Rx>> =
            std::iter::repeat_with(|| None).take(STREAM_COUNT).collect();
        for _ in 0..STREAM_COUNT {
            let (mut rx, _) = acceptor
                .accept()
                .await
                .expect("contract: every opened stream arrives");
            let index = rx.read_u8().await.expect("contract: stream label");
            let slot = held
                .get_mut(usize::from(index))
                .expect("contract: a stream delivered an index it was never sent");
            assert!(
                slot.replace(rx).is_none(),
                "contract: a stream arrived twice"
            );
        }
        held.into_iter()
            .map(|rx| rx.expect("every stream arrived"))
            .collect()
    };
    join(send, receive).await
}

/// Drain the youngest stream before its siblings, which remain open and unread.
async fn probe_concurrency<C: Connector, A: Acceptor>(
    connector: &C,
    acceptor: &mut A,
    concurrent: bool,
) {
    let (writers, mut readers) = open_streams(connector, acceptor, concurrent).await;
    let send = join_all(writers.into_iter().map(|mut tx| async move {
        tx.write_all(PROBE).await.expect("contract: stream write");
        tx.flush().await.expect("contract: stream flush");
    }));
    let receive = async {
        readers.rotate_right(1);
        for mut rx in readers {
            let mut bytes = Vec::new();
            rx.read_to_end(&mut bytes)
                .await
                .expect("contract: independent stream delivery");
            assert_eq!(bytes, PROBE, "contract: exact bytes on every stream");
        }
    };
    join(send, receive).await;
}

/// Cancelling an accept must preserve delivery for a later accept.
///
/// In both directions, pending accepts are dropped before arrivals and after
/// one, two, four, or eight polls during delivery, with up to [`STREAM_COUNT`]
/// streams in flight. Opens and accepts run together, so the check does not
/// require a connect backlog. A lost stream leaves a later accept waiting even
/// if dropping its reader also makes the sender's write fail. Internal loss
/// windows outside these schedules may go unseen.
///
/// The deadline covers pair construction and this check.
pub async fn check_accept_cancellation<CRa, CWa, Ca, Aa, CRb, CWb, Cb, Ab, D>(
    pair: impl AsyncFnOnce() -> (Link<CRa, CWa, Ca, Aa>, Link<CRb, CWb, Cb, Ab>),
    deadline: impl FnOnce() -> D,
) where
    Ca: Connector,
    Aa: Acceptor,
    Cb: Connector,
    Ab: Acceptor,
    D: Future<Output = ()>,
{
    timed("check_accept_cancellation", deadline(), async {
        let (mut a, mut b) = pair().await;
        for (polls, deliveries) in [(1, 2), (2, 3), (4, STREAM_COUNT), (8, STREAM_COUNT)] {
            join(
                probe_cancellation(&a.connector, &mut b.acceptor, polls, deliveries),
                probe_cancellation(&b.connector, &mut a.acceptor, polls, deliveries),
            )
            .await;
        }
    })
    .await;
}

/// Cancel accepts after `polls` polls while streams are arriving.
///
/// A first delivery establishes that streams can reach the receiver before
/// cancellation starts. Each cancelled future retains its state across polls;
/// the final collecting accepts must recover every stream.
async fn probe_cancellation<C: Connector, A: Acceptor>(
    connector: &C,
    acceptor: &mut A,
    polls: usize,
    deliveries: usize,
) {
    {
        // Poll a pending accept once, then drop it before anything arrives:
        // the trivial case of the teardown shape.
        let mut pending = pin!(acceptor.accept());
        let waker = futures::task::noop_waker();
        let mut cx = Context::from_waker(&waker);
        assert!(
            pending.as_mut().poll(&mut cx).is_pending(),
            "no stream was opened yet",
        );
    }
    // The receiver polls accepts while streams connect. This lets rendezvous
    // transports pass without giving the probe an unadvertised backlog, while
    // still dropping accepts throughout delivery.
    let (connected, in_flight) = futures::channel::oneshot::channel();
    let send = async {
        // Writes begin only after every open finishes. An accept completed
        // during a rendezvous open is retained until this point, so no reader
        // waits for bytes that the sender cannot start writing yet.
        let mut streams = Vec::with_capacity(deliveries);
        for _ in 0..deliveries {
            let (tx, _) = connector.connect().await.expect("contract: connect");
            streams.push(tx);
        }
        let _ = connected.send(());
        // The writes run concurrently: the receiver drains streams in
        // whatever order its acceptor yields them, and sequential writes
        // through small windows would deadlock the probe against a
        // conforming reordering acceptor.
        join_all(
            streams
                .into_iter()
                .enumerate()
                .map(|(index, mut tx)| async move {
                    // The collecting accepts are the cancellation oracle. A
                    // broken pipe here may be the symptom of the acceptor
                    // dropping its reader, so reporting it as a write defect
                    // would blame the wrong half of the link.
                    if tx
                        .write_all(&[stream_tag(index); PROBE.len()])
                        .await
                        .is_ok()
                    {
                        let _ = tx.flush().await;
                    }
                    drop(tx);
                }),
        )
        .await;
    };
    let receive = async {
        let mut seen = vec![false; deliveries];
        let mut accepted = Vec::with_capacity(deliveries);
        let mut in_flight = in_flight;

        // Race each bounded accept attempt against completion of all opens.
        // Ready accepts are retained; pending ones are dropped by `select`.
        // A rendezvous connector can therefore finish through the ready cases,
        // while a cancel-unsafe acceptor loses a stream in a pending case.
        loop {
            let attempt = pin!(poll_accept(acceptor, polls));
            match select(in_flight, attempt).await {
                Either::Left((connected, _pending_accept)) => {
                    connected.expect("the sending half signals after connecting");
                    break;
                }
                Either::Right((arrival, connecting)) => {
                    in_flight = connecting;
                    if let Some(arrival) = arrival {
                        accepted.push(
                            arrival.expect("contract: accept succeeds while the peer link lives"),
                        );
                    }
                    yield_once().await;
                }
            }
        }

        for (rx, _) in accepted.drain(..) {
            receive_cancelled(rx, &mut seen).await;
        }
        let mut delivered = seen.iter().filter(|seen| **seen).count();
        // Keep each accept across several polls before cancelling it. A
        // dequeue reached after the first poll must be cancellation-safe too.
        let mut patience = CANCEL_DROP_PATIENCE;
        while delivered < deliveries {
            match poll_accept(acceptor, polls).await {
                Some(rx) => {
                    let (rx, _) = rx.expect("contract: accept succeeds while the peer link lives");
                    receive_cancelled(rx, &mut seen).await;
                    delivered += 1;
                }
                None => {
                    let Some(remaining) = patience.checked_sub(1) else {
                        break;
                    };
                    patience = remaining;
                    yield_once().await;
                }
            }
        }
        // Every delivery must now surface from real accepts, however many
        // waits were dropped above.
        while delivered < deliveries {
            let (rx, _) = acceptor
                .accept()
                .await
                .expect("contract: a delivery in flight across a dropped accept still arrives");
            receive_cancelled(rx, &mut seen).await;
            delivered += 1;
        }
    };
    join(send, receive).await;
}

/// Poll one accept up to `polls` times, returning `None` when it is cancelled.
async fn poll_accept<A: Acceptor>(
    acceptor: &mut A,
    polls: usize,
) -> Option<io::Result<(A::Rx, Done<A::Rx>)>> {
    assert!(polls > 0, "a cancellation attempt must poll at least once");
    let mut accept = pin!(acceptor.accept());
    for attempt in 0..polls {
        let ready = std::future::poll_fn(|cx| {
            Poll::Ready(match accept.as_mut().poll(cx) {
                Poll::Ready(rx) => Some(rx),
                Poll::Pending => None,
            })
        })
        .await;
        if ready.is_some() {
            return ready;
        }
        if attempt + 1 < polls {
            yield_once().await;
        }
    }
    None
}

/// Check one delivered stream's identity and bytes after accept cancellation.
async fn receive_cancelled<R: AsyncRead + Unpin>(mut rx: R, seen: &mut [bool]) {
    let mut bytes = Vec::new();
    rx.read_to_end(&mut bytes)
        .await
        .expect("contract: delivery after cancellation");
    assert_eq!(
        bytes.len(),
        PROBE.len(),
        "contract: exact bytes after cancellation"
    );
    let index = bytes[0];
    assert!(
        bytes.iter().all(|byte| *byte == index),
        "contract: exact bytes after cancellation"
    );
    let seen = seen
        .get_mut(usize::from(index))
        .expect("contract: stream identity after cancellation");
    assert!(
        !std::mem::replace(seen, true),
        "contract: cancellation duplicated a stream"
    );
}

/// A connector that counts successful opens.
///
/// The session check wraps both ends with one, so its final assertion can
/// prove the sized divergence really opened data streams in each direction
/// instead of assuming [`SESSION_PAYLOADS`] stays large enough as the
/// protocol evolves.
struct CountingConnector<C> {
    /// The transport under test.
    inner: C,
    /// Opens shared by every clone used in the session.
    opened: Arc<AtomicUsize>,
}

impl<C: Clone> Clone for CountingConnector<C> {
    /// Keep cloned session handles in the same census.
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            opened: self.opened.clone(),
        }
    }
}

impl<C: Connector> Connector for CountingConnector<C> {
    /// The underlying transport writer.
    type Tx = C::Tx;

    /// Count a successful open without changing its completion behavior.
    async fn connect(&self) -> io::Result<(Self::Tx, Done<Self::Tx>)> {
        let pair = self.inner.connect().await?;
        self.opened.fetch_add(1, Ordering::Relaxed);
        Ok(pair)
    }
}

/// Wrap a link's connector so successful opens count into `opened`,
/// preserving every other part, the session state included.
fn counting<CR, CW, C, A>(
    link: Link<CR, CW, C, A>,
    opened: Arc<AtomicUsize>,
) -> Link<CR, CW, CountingConnector<C>, A>
where
    CR: AsyncRead + Unpin + Send,
    CW: AsyncWrite + Unpin + Send,
    C: Connector,
    A: Acceptor,
{
    link.map_transport(|control_read, control_write, connector, acceptor| {
        (
            control_read,
            control_write,
            CountingConnector {
                inner: connector,
                opened,
            },
            acceptor,
        )
    })
}

/// Full protocol sessions converge over the pair.
///
/// Bootstrap, reconcile distinct messages, and gossip again over the same
/// link pair. The replicas must reach identical snapshots, and each direction
/// must open data streams during the sessions. This can expose failures that
/// the focused transport checks miss.
///
/// The deadline covers pair construction and this check.
pub async fn check_sessions<CRa, CWa, Ca, Aa, CRb, CWb, Cb, Ab, D>(
    pair: impl AsyncFnOnce() -> (Link<CRa, CWa, Ca, Aa>, Link<CRb, CWb, Cb, Ab>),
    deadline: impl FnOnce() -> D,
) where
    CRa: AsyncRead + Unpin + Send,
    CWa: AsyncWrite + Unpin + Send,
    Ca: Connector,
    Aa: Acceptor,
    CRb: AsyncRead + Unpin + Send,
    CWb: AsyncWrite + Unpin + Send,
    Cb: Connector,
    Ab: Acceptor,
    D: Future<Output = ()>,
{
    timed("check_sessions", deadline(), async {
        let (a, b) = pair().await;
        let a_opened = Arc::new(AtomicUsize::new(0));
        let b_opened = Arc::new(AtomicUsize::new(0));
        let mut a = counting(a, a_opened.clone());
        let mut b = counting(b, b_opened.clone());
        let seed: Rumors<u64> = Peer::seed().into_rumors();
        // Session one: bootstrap the far side into the near side's universe.
        let (served, joined) = join(
            seed.gossip_once(&mut a),
            Peer::<u64>::bootstrap().join(&mut b),
        )
        .await;
        served.expect("contract: the bootstrap-serving session completes");
        let crate::Joined::Joined { peer: newcomer } = joined else {
            panic!("contract: the seed serves the bootstrap: {joined:?}");
        };
        let newcomer = newcomer.into_rumors();

        // Give each side data to contribute to the shallow reconciliation.
        seed.send_all(0..SESSION_PAYLOADS)
            .expect("flat payloads are within any depth limit");
        newcomer
            .send_all(SESSION_PAYLOADS..2 * SESSION_PAYLOADS)
            .expect("flat payloads are within any depth limit");

        // Session two: reconcile the divergence; session three: converge as a
        // no-op. Serialized on the same links, so the epoch counting and
        // per-session stream lifecycle are exercised across sessions.
        for _ in 0..2 {
            let (near, far) = join(seed.gossip_once(&mut a), newcomer.gossip_once(&mut b)).await;
            near.expect("contract: gossip completes over the link");
            far.expect("contract: gossip completes over the link");
        }
        assert_eq!(
            seed.snapshot().len(),
            (2 * SESSION_PAYLOADS) as usize,
            "reconciliation over the link converged",
        );
        assert_eq!(
            seed.snapshot(),
            newcomer.snapshot(),
            "contract: reconciliation over the link converged on the same set",
        );
        for (side, opened) in [("a", &a_opened), ("b", &b_opened)] {
            let opened = opened.load(Ordering::Relaxed);
            assert!(
                opened >= 1,
                "the session check opened no data streams on side {side}: \
             reconciliation rode the control stream alone, so the end-to-end \
             check does not exercise the connector and acceptor in-session",
            );
        }
    })
    .await;
}

#[cfg(test)]
mod tests;
