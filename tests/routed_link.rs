//! The routed adapter satisfies the link contract, over real sockets
//! and the in-memory network.
//!
//! [`rumors::link::routed`] is the crate's shipped shape for
//! accept/connect transports, so it answers to the same public
//! conformance suite as any caller-built transport — here over TCP at
//! default and OS-floor socket buffers, in both construction
//! orientations (the dialing and accepting ends of a routed link are
//! built by different code paths), and over the in-memory network,
//! whose string names keep the address seam honest. Real sockets need
//! real time, and every run sits under an explicit timeout because the
//! contract's liveness clauses fail as hangs.
//!
//! The mesh test then exercises the process scope the suite cannot: a
//! full mesh of endpoints converging by concurrent gossip over routed
//! links, beside a connection stalled mid-header — the router's
//! never-block law observed end to end over sockets.

mod common;

use std::net::SocketAddr;
use std::time::Duration;

use rumors::link::routed::{Config, Endpoint, Incoming, RoutedLink};
use rumors::testing::{MemoryDial, MemoryName, MemoryNet};
use rumors::{Peer, Rumors};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::time::timeout;

use crate::common::routed_tcp::{TcpDial, TcpListen};
use crate::common::wire::bootstrap_fork_async;

/// Bound on one whole suite run; loopback and in-memory checks finish
/// in seconds, so a run past this is a liveness violation, not a slow
/// machine.
const SUITE_TIMEOUT: Duration = Duration::from_secs(120);

/// Bound on the mesh test's establishment and gossip rounds.
const MESH_TIMEOUT: Duration = Duration::from_secs(60);

/// Buffer request for the minimal-capacity variants: the OS rounds a
/// socket-buffer request up to its floor, so this asks for the
/// smallest per-stream capacity the platform lawfully provides.
const MINIMAL_BUFFER_REQUEST: u32 = 1;

/// Messages each mesh replica contributes before the gossip rounds.
const MESH_PAYLOADS: u64 = 16;

/// One TCP endpoint with its router spawned onto the ambient runtime;
/// the router task runs (and holds the listener) for the rest of the
/// process, which is the deployment shape.
async fn tcp_endpoint(buffers: Option<u32>) -> (Endpoint<TcpDial>, Incoming<TcpDial>, SocketAddr) {
    let (listen, addr) = TcpListen::bind(buffers)
        .await
        .expect("bind a loopback listener");
    let (endpoint, incoming, router) = Endpoint::new(
        listen,
        addr,
        TcpDial {
            send_buffer: buffers,
        },
        Config::default(),
    )
    .expect("an unscoped loopback name is routable");
    tokio::spawn(router);
    (endpoint, incoming, addr)
}

/// Mint a fresh routed-link pair over TCP: two endpoints, one
/// establishment.
///
/// `dialer_first` picks which construction (the dialing or the
/// accepting end) lands in the suite's first seat, so both
/// orientations get every per-side probe.
async fn tcp_pair(
    buffers: Option<u32>,
    dialer_first: bool,
) -> (RoutedLink<TcpDial>, RoutedLink<TcpDial>) {
    let (_a, mut a_incoming, a_addr) = tcp_endpoint(buffers).await;
    let (b, _b_incoming, _b_addr) = tcp_endpoint(buffers).await;
    let (linked, arrival) = tokio::join!(b.link(a_addr), a_incoming.accept());
    let dialed = linked.expect("establishment succeeds");
    let (_info, accepted) = arrival.expect("the router delivers the link");
    if dialer_first {
        (dialed, accepted)
    } else {
        (accepted, dialed)
    }
}

/// Run the whole conformance suite against fresh TCP pairs at the
/// given buffer sizing and construction orientation.
async fn tcp_conformance(buffers: Option<u32>, dialer_first: bool) {
    timeout(
        SUITE_TIMEOUT,
        rumors::conformance::link::check(async || tcp_pair(buffers, dialer_first).await),
    )
    .await
    .expect("conformance suite ran past its liveness bound");
}

/// Mint a fresh routed-link pair over an in-memory network of its own.
async fn memory_pair(dialer_first: bool) -> (RoutedLink<MemoryDial>, RoutedLink<MemoryDial>) {
    let net = MemoryNet::new();
    let name_a = MemoryName::new("a");
    let (_a, mut a_incoming, a_router) = Endpoint::new(
        net.listen(&name_a),
        name_a.clone(),
        net.dial(),
        Config::default(),
    )
    .expect("a valid construction");
    tokio::spawn(a_router);
    let name_b = MemoryName::new("b");
    let (b, _b_incoming, b_router) =
        Endpoint::new(net.listen(&name_b), name_b, net.dial(), Config::default())
            .expect("a valid construction");
    tokio::spawn(b_router);
    let (linked, arrival) = tokio::join!(b.link(name_a), a_incoming.accept());
    let dialed = linked.expect("establishment succeeds");
    let (_info, accepted) = arrival.expect("the router delivers the link");
    if dialer_first {
        (dialed, accepted)
    } else {
        (accepted, dialed)
    }
}

/// At the platform's default socket buffers, the routed TCP link
/// satisfies every contract clause the suite observes.
///
/// The clauses: independent control pipes, receiver-paced independent
/// streams, clean half-close, tolerated accept cancellation, and full
/// reconciliation sessions.
#[tokio::test]
async fn conforms_over_tcp_at_default_buffers() {
    tcp_conformance(None, true).await;
}

/// The default-buffer suite with the pair's seats swapped: the
/// accepting end (router-built) takes the first seat, so its
/// construction path gets the other half of the suite's asymmetric
/// probes.
#[tokio::test]
async fn conforms_over_tcp_at_default_buffers_swapped() {
    tcp_conformance(None, false).await;
}

/// The contract holds with per-stream kernel buffers clamped to the
/// OS floor: shrinking capacity changes when backpressure engages,
/// never whether streams stay independent, receiver-paced, and
/// half-close clean.
#[tokio::test]
async fn conforms_over_tcp_at_minimal_buffers() {
    tcp_conformance(Some(MINIMAL_BUFFER_REQUEST), true).await;
}

/// The minimal-buffer suite with the pair's seats swapped, as for the
/// default-buffer variant.
#[tokio::test]
async fn conforms_over_tcp_at_minimal_buffers_swapped() {
    tcp_conformance(Some(MINIMAL_BUFFER_REQUEST), false).await;
}

/// The adapter conforms over the in-memory network too: nothing in
/// the contract mapping leans on socket semantics, and the string
/// names prove the address seam carries non-IP namespaces.
#[tokio::test]
async fn conforms_over_the_memory_network() {
    timeout(
        SUITE_TIMEOUT,
        rumors::conformance::link::check(async || memory_pair(true).await),
    )
    .await
    .expect("conformance suite ran past its liveness bound");
}

/// The in-memory suite with the pair's seats swapped, as for TCP.
#[tokio::test]
async fn conforms_over_the_memory_network_swapped() {
    timeout(
        SUITE_TIMEOUT,
        rumors::conformance::link::check(async || memory_pair(false).await),
    )
    .await
    .expect("conformance suite ran past its liveness bound");
}

/// A full mesh gossips to convergence over routed TCP links while a
/// connection stalled mid-header sits in one router the whole time.
///
/// Three endpoints, three links, concurrent sessions from shared
/// replicas: per-link routing and the pending-header bound keep every
/// live link flowing beside the stall.
#[tokio::test(flavor = "multi_thread")]
async fn mesh_converges_beside_a_stalled_header() {
    timeout(MESH_TIMEOUT, async {
        let (_a_ep, mut a_incoming, a_addr) = tcp_endpoint(None).await;
        let (b_ep, mut b_incoming, b_addr) = tcp_endpoint(None).await;
        let (c_ep, _c_incoming, _c_addr) = tcp_endpoint(None).await;

        // The stall: a connection into a's router that never finishes
        // its header, held open across the whole mesh's traffic.
        let mut stalled = TcpStream::connect(a_addr).await.expect("dial the stall");
        stalled
            .write_all(b"ROU")
            .await
            .expect("a partial magic writes");

        // The mesh: b→a, c→a, c→b.
        let (linked, arrival) = tokio::join!(b_ep.link(a_addr), a_incoming.accept());
        let mut ab_at_b = linked.expect("b links a");
        let (_, mut ab_at_a) = arrival.expect("a receives b's link");
        let (linked, arrival) = tokio::join!(c_ep.link(a_addr), a_incoming.accept());
        let mut ac_at_c = linked.expect("c links a");
        let (_, mut ac_at_a) = arrival.expect("a receives c's link");
        let (linked, arrival) = tokio::join!(c_ep.link(b_addr), b_incoming.accept());
        let mut bc_at_c = linked.expect("c links b");
        let (_, mut bc_at_b) = arrival.expect("b receives c's link");

        // Three replicas with disjoint content.
        let a: Rumors<u64> = Peer::seed().sync_window_floor().into_rumors();
        let b = bootstrap_fork_async(&a).await;
        let c = bootstrap_fork_async(&a).await;
        for payload in 0..MESH_PAYLOADS {
            a.send(payload).await.expect("in-memory send");
            b.send(100 + payload).await.expect("in-memory send");
            c.send(200 + payload).await.expect("in-memory send");
        }

        // Two rounds, each running all three pairwise sessions
        // concurrently (a replica may gossip on several links at
        // once); the second round settles anything a concurrent first
        // round exchanged before its peers' inserts landed.
        for round in 0..2 {
            let ((ab_a, ab_b), (ac_a, ac_c), (bc_b, bc_c)) = tokio::join!(
                async { tokio::join!(a.gossip(&mut ab_at_a), b.gossip(&mut ab_at_b)) },
                async { tokio::join!(a.gossip(&mut ac_at_a), c.gossip(&mut ac_at_c)) },
                async { tokio::join!(b.gossip(&mut bc_at_b), c.gossip(&mut bc_at_c)) },
            );
            for outcome in [ab_a, ac_a] {
                outcome.unwrap_or_else(|error| panic!("a's round {round} session: {error}"));
            }
            for outcome in [ab_b, bc_b] {
                outcome.unwrap_or_else(|error| panic!("b's round {round} session: {error}"));
            }
            for outcome in [ac_c, bc_c] {
                outcome.unwrap_or_else(|error| panic!("c's round {round} session: {error}"));
            }
        }

        assert_eq!(a.snapshot().hash(), b.snapshot().hash(), "a and b diverge");
        assert_eq!(b.snapshot().hash(), c.snapshot().hash(), "b and c diverge");
        drop(stalled);
    })
    .await
    .expect("the mesh ran past its liveness bound");
}
