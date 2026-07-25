//! The simulations' TCP link satisfies the link contract.
//!
//! `common::tcp` is the workspace's one [`Link`](rumors::link::Link)
//! instantiation over real sockets, and `tests/disruption.rs` trusts it
//! under process kills; these tests run it through the public
//! [`rumors::conformance::link`] suite so a disruption failure indicts the
//! protocol, never an accidentally nonconforming transport. Real sockets
//! need real time: a paused clock's auto-advance would fire the harness
//! timeout while socket I/O is genuinely pending. Every check runs under an
//! explicit timeout because the contract's liveness clauses fail as hangs,
//! and only the surrounding harness can bound them.

mod common;

use std::time::Duration;

use tokio::net::{TcpListener, TcpStream};

use crate::common::tcp;

/// Bound on one whole suite run.
///
/// Loopback checks finish in seconds; a run past this bound is a liveness
/// violation, not a slow machine.
const SUITE_TIMEOUT: Duration = Duration::from_secs(120);

/// Buffer request for the minimal-capacity variant.
///
/// The OS rounds a socket-buffer request up to its floor, so this asks for
/// the smallest per-stream capacity the platform lawfully provides.
const MINIMAL_BUFFER_REQUEST: u32 = 1;

/// Mint a fresh connected loopback pair, both ends through the TCP link
/// constructor.
async fn tcp_pair(stream_buffers: Option<u32>) -> (tcp::TcpLink, tcp::TcpLink) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind control listener");
    let addr = listener.local_addr().expect("control listener address");
    let (dialed, accepted) = tokio::join!(TcpStream::connect(addr), listener.accept());
    let dialed = dialed.expect("dial control connection");
    let (accepted, _) = accepted.expect("accept control connection");
    // The port swap inside `link` writes then reads two bytes on the
    // control connection, so both ends must progress concurrently.
    let (a, b) = tokio::join!(
        tcp::link_with_stream_buffers(dialed, stream_buffers),
        tcp::link_with_stream_buffers(accepted, stream_buffers),
    );
    (a.expect("dialer link"), b.expect("acceptor link"))
}

/// Run the whole conformance suite against fresh TCP pairs at the given
/// per-stream buffer sizing.
async fn conformance(stream_buffers: Option<u32>) {
    tokio::time::timeout(
        SUITE_TIMEOUT,
        rumors::conformance::link::check(async || tcp_pair(stream_buffers).await),
    )
    .await
    .expect("conformance suite ran past its liveness bound");
}

/// At the platform's default socket buffers, the TCP link satisfies every
/// contract clause the suite observes.
///
/// The clauses: independent control pipes,
/// receiver-paced independent streams, clean half-close, tolerated accept
/// cancellation, and a full reconciliation session.
#[tokio::test]
async fn conforms_at_default_buffers() {
    conformance(None).await;
}

/// The contract holds with per-stream kernel buffers clamped to the OS
/// floor: shrinking capacity changes when backpressure engages, never
/// whether streams stay independent, receiver-paced, and half-close clean.
#[tokio::test]
async fn conforms_at_minimal_buffers() {
    conformance(Some(MINIMAL_BUFFER_REQUEST)).await;
}
