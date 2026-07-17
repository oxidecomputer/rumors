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
//! Two contract clauses are not observable from outside and remain the
//! implementation's obligation: that per-stream buffering is *bounded* (the
//! suite exercises backpressure but cannot distinguish a large buffer from
//! an unbounded one), and that `connect`/`accept` fail only for transport
//! reasons. State both in your implementation's own documentation.

use futures::future::join;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::link::{Acceptor, Connector, Link, STREAM_COUNT};
use crate::{Peer, Rumors};

/// Bytes used to probe stream delivery without assuming any capacity.
const PROBE: &[u8] = b"rumors-conformance-probe";

/// Payload count per side in the session check, sized to open several data
/// streams per direction and drive the reconciliation descent through
/// multiple levels.
const SESSION_PAYLOADS: u64 = 48;

/// Run the whole conformance suite against fresh pairs from `pair`.
///
/// Each check consumes one fresh pair; `pair` is called once per check. See
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
            .write_all(PROBE)
            .await
            .expect("contract: control writes succeed while the peer link lives");
        a_write.flush().await.expect("contract: control flush");
        let mut bytes = vec![0u8; PROBE.len()];
        a_read
            .read_exact(&mut bytes)
            .await
            .expect("contract: control delivers the peer's bytes");
        assert_eq!(bytes, PROBE, "contract: control preserves bytes in order");
    };
    let pong = async {
        let mut bytes = vec![0u8; PROBE.len()];
        b_read
            .read_exact(&mut bytes)
            .await
            .expect("contract: control delivers the peer's bytes");
        assert_eq!(bytes, PROBE, "contract: control preserves bytes in order");
        b_write
            .write_all(PROBE)
            .await
            .expect("contract: control writes succeed while the peer link lives");
        b_write.flush().await.expect("contract: control flush");
    };
    join(ping, pong).await;
}

/// An opened stream delivers its exact bytes to the peer's acceptor, and the
/// writer's drop surfaces as end-of-stream after the final byte.
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
    let a = a.into_parts();
    let mut b = b.into_parts();
    let send = async {
        let mut tx = a
            .connector
            .connect()
            .await
            .expect("contract: connect succeeds while the peer link lives");
        tx.write_all(PROBE).await.expect("contract: stream write");
        tx.flush().await.expect("contract: stream flush");
        drop(tx);
    };
    let receive = async {
        let mut rx = b
            .acceptor
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
/// full protocol complement of later streams still delivers and closes.
///
/// This is the clause the deadlock-freedom argument rests on; a shared
/// reader, a shared window, or head-of-line coupling anywhere in the
/// implementation fails here as a hang.
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
    let a = a.into_parts();
    let mut b = b.into_parts();
    let send = async {
        // The stalled stream: written to, never drained by its receiver. A
        // single byte, so it lands within any legal window — at the
        // smallest windows it also fills the window outright, exercising a
        // fully backpressured sibling.
        let mut stalled = a
            .connector
            .connect()
            .await
            .expect("contract: connect succeeds");
        stalled
            .write_all(&PROBE[..1])
            .await
            .expect("contract: one byte lands within any legal window");
        // A full complement of further streams must still flow: writing to
        // one stream may block only on that stream's receiver.
        for _ in 1..STREAM_COUNT {
            let mut live = a.connector.connect().await.expect("contract: connect");
            live.write_all(PROBE).await.expect("contract: stream write");
            live.flush().await.expect("contract: stream flush");
            drop(live);
        }
        stalled
    };
    let receive = async {
        let _stalled = b.acceptor.accept().await.expect("contract: accept");
        for _ in 1..STREAM_COUNT {
            let mut rx = b
                .acceptor
                .accept()
                .await
                .expect("contract: later streams are accepted beside a stalled one");
            let mut bytes = Vec::new();
            rx.read_to_end(&mut bytes)
                .await
                .expect("contract: later streams deliver beside a stalled one");
            assert_eq!(bytes, PROBE, "contract: exact bytes on every stream");
        }
    };
    join(send, receive).await;
}

/// A pending `accept` dropped mid-wait does not lose a stream: the delivery
/// surfaces from a later `accept` call.
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
    let a = a.into_parts();
    let mut b = b.into_parts();
    {
        // Poll a pending accept once, then drop it before anything arrives —
        // exactly what a session's teardown does to its accept driver.
        let mut pending = std::pin::pin!(b.acceptor.accept());
        let waker = futures::task::noop_waker();
        let mut cx = std::task::Context::from_waker(&waker);
        assert!(
            std::future::Future::poll(pending.as_mut(), &mut cx).is_pending(),
            "no stream was opened yet",
        );
    }
    let send = async {
        let mut tx = a.connector.connect().await.expect("contract: connect");
        tx.write_all(PROBE).await.expect("contract: stream write");
        tx.flush().await.expect("contract: stream flush");
        drop(tx);
    };
    let receive = async {
        let mut rx = b
            .acceptor
            .accept()
            .await
            .expect("contract: a stream opened after a cancelled accept still arrives");
        let mut bytes = Vec::new();
        rx.read_to_end(&mut bytes)
            .await
            .expect("contract: delivery");
        assert_eq!(bytes, PROBE, "contract: exact bytes after cancellation");
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
