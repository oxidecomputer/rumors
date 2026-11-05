//! Sessions run over one real TCP connection.
//!
//! A session's transport is one reliable, ordered duplex byte stream, and a
//! TCP connection is the deployment's usual instance of it. These tests run
//! divergent replicas through a session over a loopback connection split
//! into its two halves, at the platform's default socket buffers and at the
//! smallest the platform allows. Socket I/O uses real deadlines: a paused
//! clock could expire while the OS is still delivering bytes.

use rumors_testkit::common;

use std::time::Duration;

use rumors::{Peer, Rumors};

use crate::common::tcp;

/// Deadline for each session, including its loopback pair setup.
const SESSION_TIMEOUT: Duration = Duration::from_secs(120);

/// Buffer request for the minimal-capacity variant.
///
/// The OS rounds a socket-buffer request up to its floor, so a one-byte
/// request yields the smallest buffer the platform allows.
const MINIMAL_BUFFER_REQUEST: u32 = 1;

/// Messages each side holds that the other lacks: enough that every level
/// of the descent carries frames and both directions carry supply runs.
const DIVERGENT_MESSAGES: u64 = 2_000;

/// Gossip two fully divergent replicas over one loopback TCP connection,
/// then gossip again over the same connection, and require both sessions
/// to converge the replicas.
///
/// The second session checks that the first left both halves at the session
/// boundary, ready for reuse. Each session first reads its peer's preamble,
/// which opens with a fixed prefix, so bytes the first session left unread
/// fail the second unless they happen to begin with that prefix.
async fn converge_over_tcp(buffers: Option<u32>) {
    let seed = Peer::<u64>::seed().into_rumors();
    let a: Rumors<u64> = common::wire::bootstrap_fork_async(&seed).await;
    let b: Rumors<u64> = common::wire::bootstrap_fork_async(&seed).await;
    common::wire::diverge(&a, &b, DIVERGENT_MESSAGES);
    let (a_end, b_end) = tcp::pair(buffers).await.expect("loopback connection");
    let sessions = async {
        let (a_result, b_result) = tokio::join!(
            a.gossip_once(a_end.read, a_end.write),
            b.gossip_once(b_end.read, b_end.write)
        );
        let (_, a_read, a_write) = a_result.expect("first session, side A");
        let (_, b_read, b_write) = b_result.expect("first session, side B");
        a.send(u64::MAX).unwrap();
        let (a_result, b_result) = tokio::join!(
            a.gossip_once(a_read, a_write),
            b.gossip_once(b_read, b_write)
        );
        a_result.expect("second session, side A");
        b_result.expect("second session, side B");
    };
    tokio::time::timeout(SESSION_TIMEOUT, sessions)
        .await
        .expect("sessions over TCP complete before the deadline");
    assert_eq!(
        a.snapshot().latest(),
        b.snapshot().latest(),
        "both replicas converge on one frontier"
    );
    assert_eq!(a.snapshot().len(), b.snapshot().len());
    assert_eq!(
        a.snapshot().len(),
        2 * DIVERGENT_MESSAGES as usize + 1,
        "every message from both sides survives"
    );
}

/// At the platform's default socket buffers, sessions over one TCP
/// connection converge divergent replicas and leave the connection
/// reusable.
#[tokio::test]
async fn sessions_converge_at_default_buffers() {
    converge_over_tcp(None).await;
}

/// With socket buffers clamped to the OS floor, sessions still converge:
/// shrinking capacity changes when backpressure engages, never whether the
/// session progresses.
#[tokio::test]
async fn sessions_converge_at_minimal_buffers() {
    converge_over_tcp(Some(MINIMAL_BUFFER_REQUEST)).await;
}
