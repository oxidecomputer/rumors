//! Routed links satisfy the transport contract over TCP and the memory network.
//!
//! Conformance runs cover both construction paths, connection reuse, and
//! small socket buffers. TCP checks use timeouts to detect stalled sessions.
//! The mesh test checks that unrelated header stalls do not block gossip.

mod common;

use std::net::SocketAddr;
use std::time::Duration;

use rumors::link::routed::{Config, Endpoint, Incoming, RoutedLink};
use rumors::testing::{MemoryDial, MemoryName, MemoryNet};
use rumors::{Peer, Rumors};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::time::{sleep, timeout};

use crate::common::routed_tcp::{TcpDial, TcpListen};
use crate::common::wire::bootstrap_fork_async;

/// Deadline for each conformance check or pooled-session scenario.
const TEST_TIMEOUT: Duration = Duration::from_secs(120);

/// Bound on the mesh test's establishment and gossip rounds.
const MESH_TIMEOUT: Duration = Duration::from_secs(60);

/// Buffer request for the minimal-capacity variants: the OS rounds a
/// socket-buffer request up to its floor, so this asks for the
/// smallest per-stream capacity the platform lawfully provides.
const MINIMAL_BUFFER_REQUEST: u32 = 1;

/// Messages each mesh replica contributes before the gossip rounds.
const MESH_PAYLOADS: u64 = 16;

/// Spawn a router and return its endpoint, incoming links, and address.
async fn tcp_endpoint(
    buffers: Option<u32>,
    pooling: bool,
) -> (Endpoint<TcpDial>, Incoming<TcpDial>, SocketAddr) {
    let (listen, addr) = TcpListen::bind(buffers)
        .await
        .expect("bind a loopback listener");
    let (endpoint, incoming, router) = Endpoint::new(
        listen,
        addr,
        TcpDial {
            send_buffer: buffers,
        },
        Config {
            pooling,
            ..Config::default()
        },
    )
    .expect("an unscoped loopback name is routable");
    tokio::spawn(router);
    (endpoint, incoming, addr)
}

/// Establish a TCP pair, choosing which end occupies the suite's first seat.
async fn tcp_pair(
    buffers: Option<u32>,
    dialer_first: bool,
    pooling: bool,
) -> (RoutedLink<TcpDial>, RoutedLink<TcpDial>) {
    let (_a, mut a_incoming, a_addr) = tcp_endpoint(buffers, pooling).await;
    let (b, _b_incoming, _b_addr) = tcp_endpoint(buffers, pooling).await;
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
    rumors::conformance::link::check(
        async || tcp_pair(buffers, dialer_first, true).await,
        || sleep(TEST_TIMEOUT),
    )
    .await;
}

/// Create a fresh routed-link pair over an in-memory network of its own.
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

/// Default-buffer TCP links satisfy the full conformance suite.
#[tokio::test]
async fn conforms_over_tcp_at_default_buffers() {
    tcp_conformance(None, true).await;
}

/// TCP conformance also holds with the accepting end in the first seat.
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

/// Minimal-buffer TCP links conform with the accepting end in the first seat.
#[tokio::test]
async fn conforms_over_tcp_at_minimal_buffers_swapped() {
    tcp_conformance(Some(MINIMAL_BUFFER_REQUEST), false).await;
}

/// Disabling pooling preserves the link contract in both orientations.
#[tokio::test]
async fn conforms_over_tcp_without_pooling() {
    for dialer_first in [false, true] {
        rumors::conformance::link::check(
            async || tcp_pair(None, dialer_first, false).await,
            || sleep(TEST_TIMEOUT),
        )
        .await;
    }
}

/// Repeated mutual sessions converge with connection reuse under a
/// multithreaded scheduler. Reuse must not couple a stream's delivery to
/// another stream's consumer.
#[tokio::test(flavor = "multi_thread")]
async fn pooled_mutual_sessions_converge() {
    timeout(TEST_TIMEOUT, async {
        let (mut a, mut b) = tcp_pair(None, true, true).await;
        let seed: Rumors<u64> = Peer::seed().into_rumors();
        let (served, joined) =
            tokio::join!(seed.gossip(&mut a), Peer::<u64>::bootstrap().join(&mut b));
        served.expect("the bootstrap-serving session completes");
        let newcomer = joined
            .expect("the bootstrap session completes")
            .expect("the seed serves the bootstrap")
            .into_rumors();
        {
            seed.send_all(0..48u64).unwrap();
        }
        {
            newcomer.send_all(48..96u64).unwrap();
        }
        for _ in 0..2 {
            let (near, far) = tokio::join!(seed.gossip(&mut a), newcomer.gossip(&mut b));
            near.expect("gossip completes over the link");
            far.expect("gossip completes over the link");
        }
        assert_eq!(seed.snapshot().len(), 96);
        assert_eq!(seed.snapshot(), newcomer.snapshot());
    })
    .await
    .expect("pooled mutual sessions timed out");
}

/// The adapter conforms over the in-memory network too: nothing in
/// the contract mapping leans on socket semantics, and the string
/// names prove the address seam carries non-IP namespaces.
#[tokio::test]
async fn conforms_over_the_memory_network() {
    rumors::conformance::link::check(async || memory_pair(true).await, || sleep(TEST_TIMEOUT))
        .await;
}

/// The in-memory suite with the pair's seats swapped, as for TCP.
#[tokio::test]
async fn conforms_over_the_memory_network_swapped() {
    rumors::conformance::link::check(async || memory_pair(false).await, || sleep(TEST_TIMEOUT))
        .await;
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
        let (_a_ep, mut a_incoming, a_addr) = tcp_endpoint(None, true).await;
        let (b_ep, mut b_incoming, b_addr) = tcp_endpoint(None, true).await;
        let (c_ep, _c_incoming, _c_addr) = tcp_endpoint(None, true).await;

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
            a.send(payload).unwrap();
            b.send(100 + payload).unwrap();
            c.send(200 + payload).unwrap();
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
