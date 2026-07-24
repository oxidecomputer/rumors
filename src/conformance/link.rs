//! Conformance checks for caller-built [`Link`] implementations.
//!
//! The gossip protocol's deadlock-freedom argument rests on the [link
//! contract](crate::link): a full-duplex control stream, independent
//! receiver-paced streams, half-close, and accept-cancellation tolerance.
//! This crate validates its own in-memory instantiation with these checks;
//! a deployment that builds its own `Link` — over QUIC, TCP, or anything
//! else — should validate it the same way.
//!
//! # Using the suite
//!
//! Provide a factory that mints a *fresh, connected* pair of link ends per
//! call, then run [`check`]:
//!
//! ```
//! # tokio::runtime::Builder::new_current_thread().build().unwrap().block_on(async {
//! rumors::conformance::link::check(async || rumors::link::memory()).await;
//! # });
//! ```
//!
//! Every check panics with the violated clause on failure, so the suite
//! drops into any test harness. Checks run on the caller's executor: a
//! closed-world in-memory link can run under a deterministic single-poll
//! driver, while a link over real sockets runs under its runtime. **Run
//! under a timeout**: the contract's liveness clauses fail as hangs, not as
//! return values, and only the surrounding harness can bound them.
//!
//! # What the suite cannot see
//!
//! A black box bounds what any probe can establish; a pass leaves each of
//! these partly the implementation's own obligation:
//!
//! - **Bounded buffering.** The coupling probes expose dependence by
//!   writing until hidden buffers fill: the independence probe pressures a
//!   stalled stream the whole time the live complement takes to finish,
//!   and the control-duplex probe writes [`CONTROL_DUPLEX_FILL`] bytes
//!   each way. Coupling concealed behind more buffering than a probe
//!   writes — in the limit, an implementation that never backpressures at
//!   all — passes anyway.
//! - **Cancellation mid-delivery.** The probe drops `accept` futures only
//!   after a first delivery has genuinely surfaced, so an acceptor that
//!   internally dequeues and then awaits is caught whenever that dequeue
//!   is reachable within the probe's patience. An acceptor that resolves
//!   on its first poll is never caught waiting, and one whose internal
//!   dequeue arises only under timings outside the probed window still
//!   passes.
//! - **Concurrency under sparse arrival.** The concurrency probe holds a
//!   full complement of open streams and requires the last to open and
//!   flow past its backpressured elders, but the clause's sparse
//!   mid-session arrival pattern is the protocol's own; a supply that
//!   misbehaves only under some arrival cadence the probe does not
//!   produce passes anyway.
//! - **Failure classification.** The contract restricts `connect`/`accept`
//!   errors to transport failure. A healthy link never errs, so the suite
//!   never sees one to classify.

use std::io;
use std::pin::pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll};

use futures::future::{Either, join, join_all, select};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::link::{Acceptor, Connector, Link, LinkParts, STREAM_COUNT};
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

/// Bytes each side writes concurrently in the control-duplex probe.
///
/// The fill must overrun the transport's control buffering, so a carrier
/// that couples the two directions is forced to wedge instead of absorbing
/// the whole exchange: the probe proves direction independence only up to
/// this much hidden buffering (see the module docs). 32 KiB is four times
/// the in-memory reference's buffer and past common transport defaults,
/// while staying affordable for a deterministic single-poll driver at
/// one-byte windows.
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

/// Bytes per write the stalled stream's writer keeps issuing while the live
/// complement runs, so per-stream buffering that hides cross-stream
/// coupling fills while the probe is still watching.
const STALL_FILL: &[u8] = &[STALLED_TAG; 512];

/// Streams the sender puts in flight for the cancellation probe.
///
/// The first bridges arrival — collected by a real accept, proving
/// deliveries are surfacing — and the remainder is what the poll-and-drop
/// cycles catch a lossy acceptor holding.
const CANCELLED_DELIVERIES: usize = 2;

/// Cooperative yields the cancellation probe spends between poll-drop
/// cycles once the first delivery has genuinely arrived.
///
/// The first delivery is collected with a real accept before any cycle
/// runs, so the cycles start inside the delivery window and the budget
/// only spans the jitter between two concurrently written streams.
/// Expiring is safe: the collecting accepts still require every delivery,
/// so expiry weakens the probe (admitted in the module docs) rather than
/// failing a conforming link.
const CANCEL_DROP_PATIENCE: usize = 32;

/// Payload count per side in the session check, sized to open data streams
/// in both directions and run reconciliation's full lifecycle over the
/// pair.
///
/// Stream count follows the reconciled tree's depth, not the payload
/// count: content-addressed keys keep a corpus this size one or two levels
/// deep, opening one or two streams per direction. The check's final
/// assertion pins exactly what the sizing buys — every direction opened at
/// least one data stream in-session — so it cannot rot silently; the
/// many-streams regime is [`check_concurrency`]'s job.
const SESSION_PAYLOADS: u64 = 48;

/// Run the whole conformance suite against fresh pairs from `pair`.
///
/// Each check consumes one fresh pair; `pair` is called once per check, and
/// every focused check probes both directions of its pair, so an asymmetric
/// implementation is validated on each side's connector and acceptor. See
/// the [module docs](self) for executor and timeout requirements.
///
/// # Panics
///
/// On the first violated contract clause, with a description of the clause.
pub async fn check<CRa, CWa, Ca, Aa, CRb, CWb, Cb, Ab>(
    mut pair: impl AsyncFnMut() -> (Link<CRa, CWa, Ca, Aa>, Link<CRb, CWb, Cb, Ab>),
) where
    CRa: AsyncRead + Unpin + Send,
    CWa: AsyncWrite + Unpin + Send,
    Ca: Connector,
    Aa: Acceptor,
    CRb: AsyncRead + Unpin + Send,
    CWb: AsyncWrite + Unpin + Send,
    Cb: Connector,
    Ab: Acceptor,
{
    let (a, b) = pair().await;
    check_control(a, b).await;
    let (a, b) = pair().await;
    check_control_duplex(a, b).await;
    let (a, b) = pair().await;
    check_streams(a, b).await;
    let (a, b) = pair().await;
    check_independence(a, b).await;
    let (a, b) = pair().await;
    check_concurrency(a, b).await;
    let (a, b) = pair().await;
    check_accept_cancellation(a, b).await;
    let (a, b) = pair().await;
    check_sessions(a, b).await;
}

/// The control halves form two independent ordered byte pipes.
///
/// Each direction carries its own distinct probe bytes, so a wiring that
/// loops a side's control write back to its own read fails the byte
/// assertion instead of surfacing only as a hang on the other side.
pub async fn check_control<CRa, CWa, Ca, Aa, CRb, CWb, Cb, Ab>(
    a: Link<CRa, CWa, Ca, Aa>,
    b: Link<CRb, CWb, Cb, Ab>,
) where
    CRa: AsyncRead + Unpin + Send,
    CWa: AsyncWrite + Unpin + Send,
    Ca: Connector,
    Aa: Acceptor,
    CRb: AsyncRead + Unpin + Send,
    CWb: AsyncWrite + Unpin + Send,
    Cb: Connector,
    Ab: Acceptor,
{
    let mut a = a.into_parts();
    let mut b = b.into_parts();
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
}

/// The control stream's two directions are independent full-duplex pipes:
/// both sides write a buffer-exceeding payload concurrently, and each
/// side's read drains the peer's payload while its own write is still in
/// flight.
///
/// This is the control-duplex clause. The protocol's largest control
/// frames (the greeting, the epilogue) are exchanged as concurrent
/// write-and-read on both ends because such a frame may exceed any
/// buffer; a carrier that couples the directions — a half-duplex turn
/// protocol, a shared lock across read and write, a read that waits for
/// this side's own write to drain — wedges with both writers blocked and
/// neither reader progressing, and fails here as a hang. Inherently
/// bidirectional, so one pass probes both directions.
pub async fn check_control_duplex<CRa, CWa, Ca, Aa, CRb, CWb, Cb, Ab>(
    a: Link<CRa, CWa, Ca, Aa>,
    b: Link<CRb, CWb, Cb, Ab>,
) where
    CRa: AsyncRead + Unpin + Send,
    CWa: AsyncWrite + Unpin + Send,
    Ca: Connector,
    Aa: Acceptor,
    CRb: AsyncRead + Unpin + Send,
    CWb: AsyncWrite + Unpin + Send,
    Cb: Connector,
    Ab: Acceptor,
{
    let mut a = a.into_parts();
    let mut b = b.into_parts();
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
}

/// One side of [`check_control_duplex`]: write this side's fill while
/// concurrently draining the peer's.
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

/// An opened stream delivers its exact bytes to the peer's acceptor, and the
/// writer's drop surfaces as end-of-stream after the final byte.
///
/// Probed in both directions: each side's connector against the other
/// side's acceptor.
pub async fn check_streams<CRa, CWa, Ca, Aa, CRb, CWb, Cb, Ab>(
    a: Link<CRa, CWa, Ca, Aa>,
    b: Link<CRb, CWb, Cb, Ab>,
) where
    CRa: AsyncRead + Unpin + Send,
    CWa: AsyncWrite + Unpin + Send,
    Ca: Connector,
    Aa: Acceptor,
    CRb: AsyncRead + Unpin + Send,
    CWb: AsyncWrite + Unpin + Send,
    Cb: Connector,
    Ab: Acceptor,
{
    let mut a = a.into_parts();
    let mut b = b.into_parts();
    probe_stream(&a.connector, &mut b.acceptor).await;
    probe_stream(&b.connector, &mut a.acceptor).await;
}

/// One direction of [`check_streams`]: a single stream, delivered exactly.
async fn probe_stream<C: Connector, A: Acceptor>(connector: &C, acceptor: &mut A) {
    let send = async {
        let mut tx = connector
            .connect()
            .await
            .expect("contract: connect succeeds while the peer link lives");
        tx.write_all(PROBE).await.expect("contract: stream write");
        tx.flush().await.expect("contract: stream flush");
        drop(tx);
    };
    let receive = async {
        let mut rx = acceptor
            .accept()
            .await
            .expect("contract: an opened stream is accepted");
        let mut bytes = Vec::new();
        rx.read_to_end(&mut bytes)
            .await
            .expect("contract: half-close surfaces as end-of-stream");
        assert_eq!(
            bytes, PROBE,
            "contract: a stream delivers its exact bytes in order",
        );
    };
    join(send, receive).await;
}

/// Streams are independent: with streams' receivers never draining, the
/// rest of a full protocol complement still delivers and closes.
///
/// This is the clause the deadlock-freedom argument rests on, probed in
/// two shapes and both directions. The single-stalled shape opens one
/// stream whose receiver never drains it past an identifying first byte,
/// keeps writing into it for as long as the live complement runs, and
/// requires every live stream to deliver and close meanwhile: a shared
/// reader, a shared window, or head-of-line coupling anywhere in the
/// implementation must reveal itself as a hang once the buffering
/// concealing it fills. The pooled shape inverts the ratio — every stream
/// but one stalled under sustained pressure, the last required to flow —
/// so a budget pooled across streams (connection-level flow control)
/// sized below the buffering it must cover exhausts and starves the live
/// stream. Streams are classified by their first byte, never by accept
/// order — the contract lets an acceptor yield them in any order.
pub async fn check_independence<CRa, CWa, Ca, Aa, CRb, CWb, Cb, Ab>(
    a: Link<CRa, CWa, Ca, Aa>,
    b: Link<CRb, CWb, Cb, Ab>,
) where
    CRa: AsyncRead + Unpin + Send,
    CWa: AsyncWrite + Unpin + Send,
    Ca: Connector,
    Aa: Acceptor,
    CRb: AsyncRead + Unpin + Send,
    CWb: AsyncWrite + Unpin + Send,
    Cb: Connector,
    Ab: Acceptor,
{
    let mut a = a.into_parts();
    let mut b = b.into_parts();
    probe_independence(&a.connector, &mut b.acceptor).await;
    probe_independence(&b.connector, &mut a.acceptor).await;
    probe_independence_pooled(&a.connector, &mut b.acceptor).await;
    probe_independence_pooled(&b.connector, &mut a.acceptor).await;
}

/// One direction of [`check_independence`]: a stalled stream under
/// sustained writes beside a live complement.
async fn probe_independence<C: Connector, A: Acceptor>(connector: &C, acceptor: &mut A) {
    let send = async {
        // The stalled stream: tagged so the receiver can hold it unread
        // wherever it lands in arrival order.
        let mut stalled = connector
            .connect()
            .await
            .expect("contract: connect succeeds");
        stalled
            .write_all(&[STALLED_TAG])
            .await
            .expect("contract: one byte lands within any legal window");
        stalled.flush().await.expect("contract: stream flush");
        // Keep pressuring the stalled stream for as long as the live
        // complement runs: an implementation that couples streams behind
        // shared machinery can hide the coupling in per-stream buffers,
        // and only sustained undrained writes force those buffers full
        // while the probe is still watching. A conforming implementation
        // simply backpressures this loop — blocking, not failing — and
        // only this stream blocks with it.
        let pressure = async {
            loop {
                stalled
                    .write_all(STALL_FILL)
                    .await
                    .expect("contract: a backpressured write blocks rather than failing");
                stalled
                    .flush()
                    .await
                    .expect("contract: a backpressured flush blocks rather than failing");
                // Yield between fills: against an implementation that never
                // backpressures (in the limit, an unbounded buffer), nothing
                // above ever returns `Pending`, and without the yield this
                // loop would spin inside a single poll — starving the live
                // arm and any in-task timeout — instead of letting the live
                // complement finish and the probe pass.
                yield_once().await;
            }
        };
        // A full complement of further streams must flow meanwhile:
        // writing to one stream may block only on that stream's receiver.
        let live = async {
            for _ in 1..STREAM_COUNT {
                let mut tx = connector.connect().await.expect("contract: connect");
                tx.write_all(&[LIVE_TAG])
                    .await
                    .expect("contract: stream write");
                tx.write_all(PROBE).await.expect("contract: stream write");
                tx.flush().await.expect("contract: stream flush");
                drop(tx);
            }
        };
        // The pressure loop is polled first on every wake, so it grabs
        // freshly freed capacity before the live streams do on
        // implementations that share any; it never completes, so the race
        // resolves exactly when the live complement does.
        match select(pin!(pressure), pin!(live)).await {
            Either::Left((never, _)) => never,
            Either::Right(((), _)) => {}
        }
    };
    let receive = async {
        let mut stalled = None;
        let mut live_seen = 0usize;
        for _ in 0..STREAM_COUNT {
            let mut rx = acceptor
                .accept()
                .await
                .expect("contract: later streams are accepted beside a stalled one");
            let mut tag = [0u8; 1];
            rx.read_exact(&mut tag)
                .await
                .expect("contract: every stream's first byte is delivered");
            match tag[0] {
                STALLED_TAG => {
                    assert!(
                        stalled.is_none(),
                        "contract: exactly one stream carried the stalled tag",
                    );
                    // Held unread from here on: this receiver never drains,
                    // and everything else must keep flowing regardless.
                    stalled = Some(rx);
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
            STREAM_COUNT - 1,
            "contract: every live stream is delivered beside a stalled one",
        );
        // Keep the stalled receiver alive (and undrained) until the
        // sender's pressure loop has been dropped by the select above.
        stalled
    };
    join(send, receive).await;
}

/// Stalled streams the pooled half of the independence check holds unread
/// at once: every data stream but the one that must keep flowing.
const STALLED_COMPLEMENT: usize = STREAM_COUNT - 1;

/// One direction of [`check_independence`]'s pooled shape: a stalled
/// complement under sustained pressure beside one live stream.
///
/// The single-stalled shape exposes coupling hidden behind less than one
/// stream's buffering; this one exposes budgets pooled across streams.
/// With every stream but one absorbing pressure unread, buffering summed
/// anywhere across them fills while the probe watches, and the live
/// stream must deliver and close regardless: writing to it may block only
/// on its own receiver, never on the stalled complement's.
async fn probe_independence_pooled<C: Connector, A: Acceptor>(connector: &C, acceptor: &mut A) {
    let send = async {
        // The stalled complement: tagged so the receiver can hold each one
        // unread wherever it lands in arrival order.
        let mut stalled = Vec::with_capacity(STALLED_COMPLEMENT);
        for _ in 0..STALLED_COMPLEMENT {
            let mut tx = connector
                .connect()
                .await
                .expect("contract: connect succeeds");
            tx.write_all(&[STALLED_TAG])
                .await
                .expect("contract: one byte lands within any legal window");
            tx.flush().await.expect("contract: stream flush");
            stalled.push(tx);
        }
        // Sustained pressure on the whole complement at once, so buffering
        // pooled anywhere across the streams fills while the probe is
        // still watching; each loop backpressures on its own stream only,
        // and yields so an unbounded buffer cannot trap the poll.
        let pressure = join_all(stalled.iter_mut().map(|tx| async move {
            loop {
                tx.write_all(STALL_FILL)
                    .await
                    .expect("contract: a backpressured write blocks rather than failing");
                tx.flush()
                    .await
                    .expect("contract: a backpressured flush blocks rather than failing");
                yield_once().await;
            }
        }));
        // The one live stream must flow beside the pressured complement.
        let live = async {
            let mut tx = connector.connect().await.expect("contract: connect");
            tx.write_all(&[LIVE_TAG])
                .await
                .expect("contract: stream write");
            tx.write_all(PROBE).await.expect("contract: stream write");
            tx.flush().await.expect("contract: stream flush");
            drop(tx);
        };
        // Pressure is polled first on every wake, so it grabs freshly
        // freed budget before the live stream does on implementations
        // that pool any; it never completes, so the race resolves exactly
        // when the live stream does.
        match select(pin!(pressure), pin!(live)).await {
            Either::Left((_, _)) => {
                unreachable!("contract: the pressure loops never complete")
            }
            Either::Right(((), _)) => {}
        }
    };
    let receive = async {
        let mut held = Vec::with_capacity(STALLED_COMPLEMENT);
        let mut live_seen = 0usize;
        for _ in 0..STREAM_COUNT {
            let mut rx = acceptor
                .accept()
                .await
                .expect("contract: later streams are accepted beside stalled ones");
            let mut tag = [0u8; 1];
            rx.read_exact(&mut tag)
                .await
                .expect("contract: every stream's first byte is delivered");
            match tag[0] {
                STALLED_TAG => {
                    // Held unread from here on; the live stream must keep
                    // flowing regardless.
                    held.push(rx);
                }
                LIVE_TAG => {
                    let mut bytes = Vec::new();
                    rx.read_to_end(&mut bytes)
                        .await
                        .expect("contract: the live stream delivers beside a stalled complement");
                    assert_eq!(bytes, PROBE, "contract: exact bytes on every stream");
                    live_seen += 1;
                }
                other => {
                    panic!("contract: a stream delivered a byte it was never sent: {other:#04x}")
                }
            }
        }
        assert_eq!(
            live_seen, 1,
            "contract: exactly one stream carried the live tag",
        );
        // Keep the stalled receivers alive (and undrained) until the
        // sender's pressure loops have been dropped by the select above.
        held
    };
    join(send, receive).await;
}

/// Yield to the executor exactly once: `Pending` with an immediate
/// self-wake.
///
/// Runtime-agnostic (the suite runs on the caller's executor, which may be
/// no runtime at all), unlike `tokio::task::yield_now`.
async fn yield_once() {
    let mut yielded = false;
    std::future::poll_fn(|cx| {
        if std::mem::replace(&mut yielded, true) {
            Poll::Ready(())
        } else {
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    })
    .await;
}

/// The transport admits a full complement of concurrently open streams.
///
/// All [`STREAM_COUNT`] are held open at once — every write and read half
/// alive — while the last-opened stream's bytes flow to completion past
/// its still-open, backpressured elders. This is the concurrency clause's
/// quantitative bound. The opens are
/// sequential with every earlier stream still open, so a supply that caps
/// concurrent streams below the complement — or serializes an open behind
/// an earlier stream's progress or closure — hangs at the capped open. The
/// drain then reads the last-opened stream to end-of-stream first, while
/// every other writer sits mid-write on an undrained stream: progress on
/// one stream beside backpressured siblings is what the session's lazily
/// held reply streams require. Probed in both directions.
pub async fn check_concurrency<CRa, CWa, Ca, Aa, CRb, CWb, Cb, Ab>(
    a: Link<CRa, CWa, Ca, Aa>,
    b: Link<CRb, CWb, Cb, Ab>,
) where
    CRa: AsyncRead + Unpin + Send,
    CWa: AsyncWrite + Unpin + Send,
    Ca: Connector,
    Aa: Acceptor,
    CRb: AsyncRead + Unpin + Send,
    CWb: AsyncWrite + Unpin + Send,
    Cb: Connector,
    Ab: Acceptor,
{
    let mut a = a.into_parts();
    let mut b = b.into_parts();
    probe_concurrency(&a.connector, &mut b.acceptor).await;
    probe_concurrency(&b.connector, &mut a.acceptor).await;
}

/// One direction of [`check_concurrency`]: the full complement held open at
/// once.
async fn probe_concurrency<C: Connector, A: Acceptor>(connector: &C, acceptor: &mut A) {
    let send = async {
        // Open the whole complement with every earlier stream still open:
        // a capped or open-serializing supply hangs right here.
        let mut held = Vec::with_capacity(STREAM_COUNT);
        for index in 0..STREAM_COUNT {
            let mut tx = connector
                .connect()
                .await
                .expect("contract: a full complement of opens succeeds");
            tx.write_all(&[index as u8])
                .await
                .expect("contract: stream write");
            tx.flush().await.expect("contract: stream flush");
            held.push(tx);
        }
        // Every stream then writes its payload concurrently. The receiver
        // drains the last-opened stream first, so at small windows the
        // elder writers sit backpressured — on their own streams only —
        // while the youngest completes.
        join_all(held.into_iter().map(|mut tx| async move {
            tx.write_all(PROBE).await.expect("contract: stream write");
            tx.flush().await.expect("contract: stream flush");
            drop(tx);
        }))
        .await;
    };
    let receive = async {
        // Accept the whole complement, holding every read half open and
        // pairing streams by their in-band index byte — never by accept
        // order, which the contract leaves to the transport.
        let mut held: Vec<Option<A::Rx>> =
            std::iter::repeat_with(|| None).take(STREAM_COUNT).collect();
        for _ in 0..STREAM_COUNT {
            let mut rx = acceptor
                .accept()
                .await
                .expect("contract: accept succeeds while the peer link lives");
            let mut index = [0u8; 1];
            rx.read_exact(&mut index)
                .await
                .expect("contract: every stream's first byte is delivered");
            let slot = held
                .get_mut(usize::from(index[0]))
                .expect("contract: a stream delivered an index it was never sent");
            assert!(
                slot.replace(rx).is_none(),
                "contract: exactly one stream carries each index",
            );
        }
        let drain = |mut rx: A::Rx| async move {
            let mut bytes = Vec::new();
            rx.read_to_end(&mut bytes)
                .await
                .expect("contract: delivery beside open siblings");
            assert_eq!(bytes, PROBE, "contract: exact bytes on every stream");
        };
        // Youngest first: its bytes and end-of-stream must arrive while
        // every elder sits open and undrained.
        let youngest = held[STREAM_COUNT - 1]
            .take()
            .expect("every slot was filled");
        drain(youngest).await;
        for slot in &mut held[..STREAM_COUNT - 1] {
            drain(slot.take().expect("every slot was filled")).await;
        }
    };
    join(send, receive).await;
}

/// A pending `accept` dropped mid-wait does not lose a stream: every
/// delivery still surfaces from later `accept` calls.
///
/// The probe puts deliveries genuinely in flight — the sender connects
/// every stream and signals the receiving half before writing — then
/// collects the first delivery with a real accept, so the poll-once-drop
/// cycles that follow run inside the observed delivery window on any
/// transport rather than racing ahead of arrival. An acceptor that
/// internally dequeues a delivery and then awaits before returning it
/// drops the dequeued stream with the cancelled future and fails here as
/// a hang on the collecting accept. Probed in both directions.
pub async fn check_accept_cancellation<CRa, CWa, Ca, Aa, CRb, CWb, Cb, Ab>(
    a: Link<CRa, CWa, Ca, Aa>,
    b: Link<CRb, CWb, Cb, Ab>,
) where
    CRa: AsyncRead + Unpin + Send,
    CWa: AsyncWrite + Unpin + Send,
    Ca: Connector,
    Aa: Acceptor,
    CRb: AsyncRead + Unpin + Send,
    CWb: AsyncWrite + Unpin + Send,
    Cb: Connector,
    Ab: Acceptor,
{
    let mut a = a.into_parts();
    let mut b = b.into_parts();
    probe_cancellation(&a.connector, &mut b.acceptor).await;
    probe_cancellation(&b.connector, &mut a.acceptor).await;
}

/// One direction of [`check_accept_cancellation`]: dropped accepts around
/// in-flight deliveries.
async fn probe_cancellation<C: Connector, A: Acceptor>(connector: &C, acceptor: &mut A) {
    {
        // Poll a pending accept once, then drop it before anything arrives —
        // the trivial case of the teardown shape.
        let mut pending = pin!(acceptor.accept());
        let waker = futures::task::noop_waker();
        let mut cx = Context::from_waker(&waker);
        assert!(
            pending.as_mut().poll(&mut cx).is_pending(),
            "no stream was opened yet",
        );
    }
    // Orders the halves across any executor: the poll-drop cycles must not
    // run until every delivery is connected, or a scheduler that lets the
    // receiving half run first (real sockets, one RTT away) would drop its
    // accepts before anything could be lost — and the probe would pass a
    // lossy acceptor.
    let (connected, in_flight) = futures::channel::oneshot::channel();
    let send = async {
        // Connect every stream before writing to any. The signal goes out
        // after the connects but before the writes: a write cannot finish
        // through a one-byte window until the receiver drains it, and the
        // receiver drains nothing until signalled — a post-write signal
        // would deadlock the probe itself.
        let mut streams = Vec::with_capacity(CANCELLED_DELIVERIES);
        for _ in 0..CANCELLED_DELIVERIES {
            streams.push(connector.connect().await.expect("contract: connect"));
        }
        let _ = connected.send(());
        // The writes run concurrently: the receiver drains streams in
        // whatever order its acceptor yields them, and sequential writes
        // through small windows would deadlock the probe against a
        // conforming reordering acceptor.
        join_all(streams.into_iter().map(|mut tx| async move {
            tx.write_all(PROBE).await.expect("contract: stream write");
            tx.flush().await.expect("contract: stream flush");
            drop(tx);
        }))
        .await;
    };
    let receive = async {
        in_flight
            .await
            .expect("the sending half signals after connecting");
        // Each collected stream is drained immediately; the sender writes
        // every stream concurrently, so draining in accept order cannot
        // deadlock however the acceptor orders arrivals.
        let drain = |mut rx: A::Rx| async move {
            let mut bytes = Vec::new();
            rx.read_to_end(&mut bytes)
                .await
                .expect("contract: delivery");
            assert_eq!(bytes, PROBE, "contract: exact bytes after cancellation");
        };
        // Bridge arrival with a real accept: connect completion at the
        // sender does not imply local acceptability (an RTT may separate
        // them), so the poll-drop cycles below start only once a delivery
        // has genuinely surfaced on this side.
        let first = acceptor
            .accept()
            .await
            .expect("contract: accept succeeds while the peer link lives");
        drain(first).await;
        let mut delivered = 1;
        // Poll a fresh accept exactly once with the real waker, then drop
        // it: the shape session teardown produces. Inside the delivery
        // window this is the moment an acceptor holding an internally
        // dequeued stream loses it; a stream the poll yields is simply
        // collected. Yields between cycles give the concurrently written
        // remainder time to surface.
        let mut patience = CANCEL_DROP_PATIENCE;
        while delivered < CANCELLED_DELIVERIES {
            let polled_once = std::future::poll_fn(|cx| {
                let mut accept = pin!(acceptor.accept());
                Poll::Ready(match accept.as_mut().poll(cx) {
                    Poll::Ready(rx) => Some(rx),
                    Poll::Pending => None,
                })
            })
            .await;
            match polled_once {
                Some(rx) => {
                    drain(rx.expect("contract: accept succeeds while the peer link lives")).await;
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
        while delivered < CANCELLED_DELIVERIES {
            let rx = acceptor
                .accept()
                .await
                .expect("contract: a delivery in flight across a dropped accept still arrives");
            drain(rx).await;
            delivered += 1;
        }
    };
    join(send, receive).await;
}

/// A connector that counts successful opens.
///
/// The session check wraps both ends with one, so its final assertion can
/// prove the sized divergence really opened data streams in each direction
/// instead of assuming [`SESSION_PAYLOADS`] stays large enough as the
/// protocol evolves.
struct CountingConnector<C> {
    inner: C,
    opened: Arc<AtomicUsize>,
}

impl<C: Clone> Clone for CountingConnector<C> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            opened: self.opened.clone(),
        }
    }
}

impl<C: Connector> Connector for CountingConnector<C> {
    type Tx = C::Tx;

    async fn connect(&self) -> io::Result<Self::Tx> {
        let tx = self.inner.connect().await?;
        self.opened.fetch_add(1, Ordering::Relaxed);
        Ok(tx)
    }
}

/// Wrap a link's connector so successful opens count into `opened`,
/// preserving every other part — the session state included.
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
    let parts = link.into_parts();
    LinkParts {
        control_read: parts.control_read,
        control_write: parts.control_write,
        connector: CountingConnector {
            inner: parts.connector,
            opened,
        },
        acceptor: parts.acceptor,
        session: parts.session,
    }
    .into_link()
}

/// Full protocol sessions run over the pair: a bootstrap, then divergent
/// gossip wide enough to open data streams in each direction, then a
/// convergence-check session, all serialized on the one link pair.
///
/// This is the end-to-end check: if the implementation violates a clause in
/// a way the focused probes miss, reconciliation deadlocks (hangs) or
/// fails here. The two replicas must converge on the *same set* — asserted
/// by snapshot equality, not merely equal sizes — and each direction must
/// have opened data streams in-session, so the check cannot silently
/// degenerate into control-stream-only traffic.
pub async fn check_sessions<CRa, CWa, Ca, Aa, CRb, CWb, Cb, Ab>(
    a: Link<CRa, CWa, Ca, Aa>,
    b: Link<CRb, CWb, Cb, Ab>,
) where
    CRa: AsyncRead + Unpin + Send,
    CWa: AsyncWrite + Unpin + Send,
    Ca: Connector,
    Aa: Acceptor,
    CRb: AsyncRead + Unpin + Send,
    CWb: AsyncWrite + Unpin + Send,
    Cb: Connector,
    Ab: Acceptor,
{
    let a_opened = Arc::new(AtomicUsize::new(0));
    let b_opened = Arc::new(AtomicUsize::new(0));
    let mut a = counting(a, a_opened.clone());
    let mut b = counting(b, b_opened.clone());
    let seed: Rumors<u64> = Peer::seed().into_rumors();
    // Session one: bootstrap the far side into the near side's universe.
    let (served, joined) = join(seed.gossip(&mut a), Peer::<u64>::bootstrap().join(&mut b)).await;
    served.expect("contract: the bootstrap-serving session completes");
    let newcomer = joined
        .expect("contract: the bootstrap session completes")
        .expect("the seed serves the bootstrap")
        .into_rumors();

    // Divergence wide and deep enough to exercise many streams per side.
    {
        let mut batch = seed.batch();
        for payload in 0..SESSION_PAYLOADS {
            batch.send(payload);
        }
    }
    {
        let mut batch = newcomer.batch();
        for payload in SESSION_PAYLOADS..2 * SESSION_PAYLOADS {
            batch.send(payload);
        }
    }

    // Session two: reconcile the divergence; session three: converge as a
    // no-op. Serialized on the same links, so the epoch counting and
    // per-session stream lifecycle are exercised across sessions.
    for _ in 0..2 {
        let (near, far) = join(seed.gossip(&mut a), newcomer.gossip(&mut b)).await;
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
}

#[cfg(test)]
mod tests;
