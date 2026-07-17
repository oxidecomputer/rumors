//! Conformance checks for caller-built [`Link`] implementations.
//!
//! The gossip protocol's deadlock-freedom argument rests on the [link
//! contract](crate::link): independent, receiver-paced streams, half-close,
//! and accept-cancellation tolerance. This crate validates its own in-memory
//! instantiation with these checks; a deployment that builds its own `Link`
//! — over QUIC, TCP, or anything else — should validate it the same way,
//! from a dev-dependency with the `conformance` feature enabled.
//!
//! # Using the suite
//!
//! Provide a factory that mints a *fresh, connected* pair of link ends per
//! call, then run [`check`]:
//!
//! ```
//! # tokio::runtime::Builder::new_current_thread().build().unwrap().block_on(async {
//! rumors::conformance::check(async || rumors::link::memory()).await;
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
//! Three contract clauses are only partially observable from outside and
//! remain the implementation's obligation:
//!
//! - **Bounded buffering.** The independence probe keeps writing into its
//!   stalled stream for as long as the live streams run, so coupling hidden
//!   behind per-stream buffers must reveal itself once those buffers fill —
//!   but only within the bytes written before the live complement completes.
//!   An implementation that buffers more than that (in the limit, an
//!   unbounded one) passes regardless.
//! - **Cancellation mid-delivery.** The poll-and-drop cycles exercise a
//!   dropped `accept` with deliveries in flight only when the acceptor
//!   reports `Pending` at that moment. An acceptor that happens to resolve
//!   on its first poll is never observed mid-wait — though at that moment it
//!   also holds nothing a drop could lose.
//! - **Failure classification.** That `connect`/`accept` fail only for
//!   transport reasons is unobservable from a healthy link.
//!
//! State all three in your implementation's own documentation.

use std::pin::pin;
use std::task::{Context, Poll};

use futures::future::{Either, join, select};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::link::{Acceptor, Connector, Link, STREAM_COUNT};
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

/// Streams already in flight when the cancellation probe's poll-and-drop
/// cycles run: enough that an acceptor caught holding one still has
/// another to lose.
const CANCELLED_DELIVERIES: usize = 2;

/// Accept futures polled exactly once and dropped while deliveries are in
/// flight, before the collecting accepts of the cancellation probe.
const POLL_DROP_CYCLES: usize = 3;

/// Payload count per side in the session check, sized to open several data
/// streams per direction and drive the reconciliation descent through
/// multiple levels.
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
    check_streams(a, b).await;
    let (a, b) = pair().await;
    check_independence(a, b).await;
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

/// Streams are independent: with one stream's receiver never draining, a
/// full protocol complement of concurrent streams still delivers and closes.
///
/// This is the clause the deadlock-freedom argument rests on. The probe
/// opens one stream whose receiver never drains it past an identifying
/// first byte, keeps writing into it for as long as the live complement
/// runs, and requires every live stream to deliver and close meanwhile: a
/// shared reader, a shared window, or head-of-line coupling anywhere in
/// the implementation must reveal itself as a hang once the buffering
/// concealing it fills. Streams are classified by their first byte, never
/// by accept order — the contract lets an acceptor yield them in any
/// order. Probed in both directions.
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

/// A pending `accept` dropped mid-wait does not lose a stream: every
/// delivery still surfaces from later `accept` calls.
///
/// With deliveries already in flight, the probe polls a fresh accept
/// exactly once and drops it, several times — the shape session teardown
/// produces — then collects every stream with real accepts. An acceptor
/// that internally dequeues a delivery and then awaits before returning it
/// drops the dequeued stream with the cancelled future and fails here as a
/// hang on the collecting accept. Probed in both directions.
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
            std::future::Future::poll(pending.as_mut(), &mut cx).is_pending(),
            "no stream was opened yet",
        );
    }
    let send = async {
        for _ in 0..CANCELLED_DELIVERIES {
            let mut tx = connector.connect().await.expect("contract: connect");
            tx.write_all(PROBE).await.expect("contract: stream write");
            tx.flush().await.expect("contract: stream flush");
            drop(tx);
        }
    };
    let receive = async {
        // Each collected stream is drained immediately: the sender opens its
        // streams one after another through possibly one-byte windows, so
        // holding an unread stream while waiting for the next would deadlock
        // the probe itself, not the implementation.
        let drain = |mut rx: A::Rx| async move {
            let mut bytes = Vec::new();
            rx.read_to_end(&mut bytes)
                .await
                .expect("contract: delivery");
            assert_eq!(bytes, PROBE, "contract: exact bytes after cancellation");
        };
        let mut delivered = 0;
        for _ in 0..POLL_DROP_CYCLES {
            if delivered == CANCELLED_DELIVERIES {
                break;
            }
            // Poll a fresh accept exactly once with the real waker, then
            // drop it. With deliveries in flight, this is the moment an
            // acceptor holding an internally dequeued stream loses it; a
            // stream it yields immediately is simply collected.
            let polled_once = std::future::poll_fn(|cx| {
                let mut accept = pin!(acceptor.accept());
                Poll::Ready(match accept.as_mut().poll(cx) {
                    Poll::Ready(rx) => Some(rx),
                    Poll::Pending => None,
                })
            })
            .await;
            if let Some(rx) = polled_once {
                drain(rx.expect("contract: accept succeeds while the peer link lives")).await;
                delivered += 1;
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

/// Full protocol sessions run over the pair: a bootstrap, then divergent
/// gossip wide enough to open several streams per direction, then a
/// convergence-check session, all serialized on the one link pair.
///
/// This is the end-to-end check: if the implementation violates a clause in
/// a way the focused probes miss, reconciliation deadlocks (hangs) or
/// fails here.
pub async fn check_sessions<CRa, CWa, Ca, Aa, CRb, CWb, Cb, Ab>(
    mut a: Link<CRa, CWa, Ca, Aa>,
    mut b: Link<CRb, CWb, Cb, Ab>,
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
    let seed: Rumors<u64> = Peer::seed().into_rumors();
    // Session one: bootstrap the far side into the near side's universe.
    let (served, joined) = join(seed.gossip(&mut a), Peer::<u64>::bootstrap(&mut b)).await;
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
        newcomer.snapshot().len(),
        (2 * SESSION_PAYLOADS) as usize,
        "reconciliation over the link converged",
    );
}

#[cfg(test)]
mod tests;
