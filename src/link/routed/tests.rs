use std::future::{Future, poll_fn};
use std::io;
use std::pin::pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::Poll;

use crate::testing::run_to_quiescence;
use futures::FutureExt;
use futures::future::{Either, select, try_join};
use proptest::prelude::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufStream, DuplexStream};
use tokio::sync::{mpsc, oneshot};

use super::header::{self, Token};
use super::{
    Config, Dial, Endpoint, EndpointError, Incoming, LinkError, LinkInfo, Listen, RoutedLink,
};
use crate::link::{Acceptor, Connector, Link, STREAM_COUNT};
use crate::testing::{MemoryDial, MemoryName, MemoryNet};

mod fairness;

/// One endpoint on `net`, listening at (and advertising) `name`.
fn endpoint(
    net: &MemoryNet,
    name: &str,
    config: Config,
) -> (
    Endpoint<MemoryDial>,
    Incoming<MemoryDial>,
    impl Future<Output = io::Result<()>>,
) {
    let name = MemoryName::new(name);
    let listen = net.listen(&name);
    Endpoint::new(listen, name, net.dial(), config).expect("a valid construction")
}

/// Run `scenario` to completion while `routers` drives; a router that
/// resolves first is a failure (drive futures resolve only on
/// listener failure).
async fn drive<T>(
    routers: impl Future<Output = io::Result<()>>,
    scenario: impl Future<Output = T>,
) -> T {
    match select(pin!(scenario), pin!(routers)).await {
        Either::Left((value, _)) => value,
        Either::Right((outcome, _)) => {
            panic!("a router resolved mid-scenario: {outcome:?}")
        }
    }
}

/// Both endpoints' routers as one drive future.
async fn routers(
    a: impl Future<Output = io::Result<()>>,
    b: impl Future<Output = io::Result<()>>,
) -> io::Result<()> {
    try_join(a, b).await.map(|_| ())
}

/// Establish one link from `from` toward the peer named `name`,
/// collecting the peer's end from `incoming`.
async fn establish(
    from: &Endpoint<MemoryDial>,
    name: &str,
    incoming: &mut Incoming<MemoryDial>,
) -> (
    RoutedLink<MemoryDial>,
    LinkInfo<MemoryName>,
    RoutedLink<MemoryDial>,
) {
    let (linked, arrival) = futures::join!(from.link(MemoryName::new(name)), incoming.accept());
    let dialer_end = linked.expect("establishment succeeds");
    let (info, acceptor_end) = arrival.expect("the router delivers the link");
    (dialer_end, info, acceptor_end)
}

/// Open one stream from `opener` to `acceptor`, move `payload` across
/// it, close by drop, and require the receiver to observe exactly the
/// payload then end-of-stream.
async fn transfer<CRa, CWa, Ca, Aa, CRb, CWb, Cb, Ab>(
    opener: &Link<CRa, CWa, Ca, Aa>,
    acceptor: &mut Link<CRb, CWb, Cb, Ab>,
    payload: &[u8],
) where
    Ca: Connector,
    Ab: Acceptor,
{
    let open = async {
        let (mut tx, _) = opener.connector.connect().await.expect("stream opens");
        tx.write_all(payload).await.expect("payload writes");
    };
    let read = async {
        let (mut rx, _) = acceptor.acceptor.accept().await.expect("stream arrives");
        let mut received = Vec::new();
        rx.read_to_end(&mut received)
            .await
            .expect("stream reads to end-of-stream");
        received
    };
    let ((), received) = futures::join!(open, read);
    assert_eq!(received, payload);
}

/// Establishment wires a full link: the control stream carries bytes
/// both ways, forward stream opens route by token, and reverse opens
/// dial the advertised name from the establishment header.
///
/// Stream half-close is drop-driven: the receiver reads the payload,
/// then end-of-stream, with nothing left pending.
#[test]
fn establishment_connects_control_and_streams() {
    pollster::block_on(async {
        let net = MemoryNet::new();
        let (a, mut a_incoming, a_router) = endpoint(&net, "a", Config::default());
        let (b, _b_incoming, b_router) = endpoint(&net, "b", Config::default());
        drive(routers(a_router, b_router), async {
            let (mut at_b, info, mut at_a) = establish(&b, "a", &mut a_incoming).await;
            assert_eq!(info.peer, MemoryName::new("b"));

            // The control stream is the establishment connection:
            // bytes cross in both directions independently.
            at_b.control_write
                .write_all(b"from b")
                .await
                .expect("control writes");
            let mut probe = [0; 6];
            at_a.control_read
                .read_exact(&mut probe)
                .await
                .expect("control reads");
            assert_eq!(&probe, b"from b");
            at_a.control_write
                .write_all(b"from a")
                .await
                .expect("control writes");
            at_b.control_read
                .read_exact(&mut probe)
                .await
                .expect("control reads");
            assert_eq!(&probe, b"from a");

            // Forward: the establishing side opens toward its peer.
            transfer(&at_b, &mut at_a, b"forward payload").await;
            // Reverse: the accepting side dials the advertised name.
            transfer(&at_a, &mut at_b, b"reverse payload").await;
            drop((a, b));
        })
        .await;
    });
}

/// Buffer routing bytes until the adapter explicitly flushes them.
#[derive(Clone)]
struct Buffered<T>(T);

impl<D: Dial> Dial for Buffered<D> {
    /// Preserve the wrapped dialer's address type.
    type Addr = D::Addr;
    /// Buffer reads and writes on each dialed connection.
    type Conn = BufStream<D::Conn>;

    /// Dial a connection and add buffering.
    async fn dial(&self, addr: &Self::Addr) -> io::Result<Self::Conn> {
        self.0.dial(addr).await.map(BufStream::new)
    }
}

impl<L: Listen> Listen for Buffered<L> {
    /// Buffer reads and writes on each accepted connection.
    type Conn = BufStream<L::Conn>;

    /// Accept a connection and add buffering.
    async fn accept(&mut self) -> io::Result<Self::Conn> {
        self.0.accept().await.map(BufStream::new)
    }

    /// Preserve the wrapped listener's routing deadline.
    fn routing_deadline(&self) -> impl Future<Output = ()> + Send + 'static {
        self.0.routing_deadline()
    }
}

/// Buffered connections establish links, deliver streams before payload writes,
/// and become reusable after completion. Full sessions flush their own traffic.
#[test]
fn buffered_connections_flush_routing_boundaries() {
    run_to_quiescence(async {
        let net = MemoryNet::new();
        let dial = CountingDial::new(&net);
        let a_name = MemoryName::new("a");
        let (_a, mut incoming, a_router) = Endpoint::new(
            Buffered(net.listen(&a_name)),
            a_name.clone(),
            Buffered(dial.clone()),
            Config::default(),
        )
        .unwrap();
        let b_name = MemoryName::new("b");
        let (b, _incoming, b_router) = Endpoint::new(
            Buffered(net.listen(&b_name)),
            b_name,
            Buffered(dial.clone()),
            Config::default(),
        )
        .unwrap();
        drive(routers(a_router, b_router), async {
            let (linked, arrival) = futures::join!(b.link(a_name), incoming.accept());
            let at_b = linked.unwrap();
            let (_, mut at_a) = arrival.unwrap();
            for _ in 0..2 {
                // Routing must finish before either side sends payload bytes.
                let (opened, accepted) =
                    futures::join!(at_b.connector.connect(), at_a.acceptor.accept());
                let (mut tx, sent) = opened.unwrap();
                let (mut rx, received) = accepted.unwrap();
                tx.write_all(b"payload").await.unwrap();
                tx.flush().await.unwrap();
                let mut bytes = [0; 7];
                rx.read_exact(&mut bytes).await.unwrap();
                assert_eq!(&bytes, b"payload");
                sent.complete(tx);
                received.complete(rx);
                settle().await;
            }
            assert_eq!(
                dial.fresh_dials(),
                2,
                "one control and one reused data connection"
            );
            crate::conformance::link::check_sessions(async || (at_a, at_b), std::future::pending)
                .await;
        })
        .await;
    })
    .expect("buffered routing and sessions make progress");
}

/// `local_addr` is the advertised name given at construction, the name
/// peers dial this endpoint at; callers need it back for policies like
/// dial tiebreaks.
#[test]
fn local_addr_is_the_constructed_name() {
    let net = MemoryNet::new();
    let (a, _incoming, _router) = endpoint(&net, "a", Config::default());
    assert_eq!(*a.local_addr(), MemoryName::new("a"));
}

/// A stream connection quoting a token no live link owns is dropped:
/// the dialer observes end-of-stream, and no link's queue sees the
/// connection.
#[test]
fn unknown_token_is_dropped() {
    pollster::block_on(async {
        let net = MemoryNet::new();
        let (_a, _a_incoming, a_router) = endpoint(&net, "a", Config::default());
        drive(a_router, async {
            let mut conn = net.dial().dial(&MemoryName::new("a")).await.expect("dial");
            conn.write_all(&header::stream_header(&Token::new()))
                .await
                .expect("header writes");
            let mut drained = Vec::new();
            conn.read_to_end(&mut drained)
                .await
                .expect("the router drops the connection");
            assert!(drained.is_empty());
        })
        .await;
    });
}

/// Queue overflow evicts the link. Queued streams remain available, then
/// acceptance reports an error; later connections using the token are closed.
#[test]
fn queue_overflow_evicts_the_link() {
    run_to_quiescence(async {
        let net = MemoryNet::new();
        let (a, mut a_incoming, a_router) = endpoint(&net, "a", Config::default());
        let (b, _b_incoming, b_router) = endpoint(&net, "b", Config::default());
        let mut running = pin!(routers(a_router, b_router));
        let (_at_b, info, mut at_a) =
            drive(running.as_mut(), establish(&b, "a", &mut a_incoming)).await;

        let mut flood = Vec::new();
        for _ in 0..=STREAM_COUNT {
            let mut conn = net.dial().dial(&MemoryName::new("a")).await.unwrap();
            conn.write_all(&header::stream_header(&info.token))
                .await
                .unwrap();
            flood.push(conn);
        }
        // Keep the consumer idle until every header has been routed. Draining
        // it concurrently would free slots and might never overflow the queue.
        assert_eq!(
            run_to_quiescence(running.as_mut()).unwrap_err(),
            crate::testing::Quiescence::Stalled
        );
        for _ in 0..STREAM_COUNT {
            at_a.acceptor.accept().await.expect("queued streams drain");
        }
        at_a.acceptor
            .accept()
            .await
            .expect_err("eviction closes the supply");

        drive(running.as_mut(), async {
            let mut conn = net.dial().dial(&MemoryName::new("a")).await.unwrap();
            conn.write_all(&header::stream_header(&info.token))
                .await
                .unwrap();
            assert_eq!(
                conn.read(&mut [0]).await.unwrap(),
                0,
                "the token was revoked"
            );
        })
        .await;
        drop((a, b));
    })
    .expect("overflow reports an error without stalling");
}

/// Dropping a link revokes its token at that moment, router
/// uninvolved: connections quoting the token afterward are dropped on
/// sight.
#[test]
fn dropping_a_link_revokes_its_token() {
    pollster::block_on(async {
        let net = MemoryNet::new();
        let (a, mut a_incoming, a_router) = endpoint(&net, "a", Config::default());
        let (b, _b_incoming, b_router) = endpoint(&net, "b", Config::default());
        drive(routers(a_router, b_router), async {
            let (_at_b, info, at_a) = establish(&b, "a", &mut a_incoming).await;
            drop(at_a);
            let mut conn = net.dial().dial(&MemoryName::new("a")).await.expect("dial");
            conn.write_all(&header::stream_header(&info.token))
                .await
                .expect("header writes");
            let mut drained = Vec::new();
            conn.read_to_end(&mut drained)
                .await
                .expect("the router drops the connection");
            assert!(drained.is_empty());
            drop((a, b));
        })
        .await;
    });
}

/// A connection that stalls inside its connect header occupies one
/// pending-read slot, never the router: establishment and streams
/// proceed beside it untouched.
#[test]
fn stalled_header_does_not_park_the_router() {
    pollster::block_on(async {
        let net = MemoryNet::new();
        let (a, mut a_incoming, a_router) = endpoint(&net, "a", Config::default());
        let (b, _b_incoming, b_router) = endpoint(&net, "b", Config::default());
        drive(routers(a_router, b_router), async {
            let mut stalled = net.dial().dial(&MemoryName::new("a")).await.expect("dial");
            stalled
                .write_all(b"ROU")
                .await
                .expect("a partial magic writes");

            let (at_b, _info, mut at_a) = establish(&b, "a", &mut a_incoming).await;
            transfer(&at_b, &mut at_a, b"alive beside the stall").await;
            drop((stalled, a, b));
        })
        .await;
    });
}

/// Lets the scenario expire each fresh connection's routing independently.
struct Deadlines<L> {
    /// Supplies connections without an I/O timeout.
    listen: L,
    /// Gives the scenario control of each newly created deadline.
    started: mpsc::UnboundedSender<oneshot::Sender<()>>,
}

impl<L: Listen> Listen for Deadlines<L> {
    /// Preserve the wrapped listener's connection type.
    type Conn = L::Conn;

    /// Accept without changing the wrapped listener's behavior.
    async fn accept(&mut self) -> io::Result<Self::Conn> {
        self.listen.accept().await
    }

    /// Give the scenario a handle that expires this routing attempt.
    fn routing_deadline(&self) -> impl Future<Output = ()> + Send + 'static {
        let (expire, expired) = oneshot::channel();
        self.started.send(expire).unwrap();
        async {
            let _ = expired.await;
        }
    }
}

proptest! {
    /// Expiring an incomplete header releases its slot, lets a queued link
    /// establish, and leaves the listener running.
    #[test]
    fn routing_deadline_releases_partial_header(prefix in 0usize..64) {
        run_to_quiescence(async {
            let net = MemoryNet::new();
            let name = MemoryName::new("a");
            let (started, mut deadlines) = mpsc::unbounded_channel();
            let (_a, mut incoming, router) = Endpoint::new(
                Deadlines { listen: net.listen(&name), started }, name.clone(), net.dial(),
                Config { pending_headers: 1, ..Config::default() },
            ).unwrap();
            let mut router = pin!(router);
            let mut stalled = net.dial().dial(&name).await.unwrap();
            let header = header::link_header(&Token::new(), b"peer");
            stalled.write_all(&header[..prefix % header.len()]).await.unwrap();
            assert_eq!(run_to_quiescence(router.as_mut()).unwrap_err(), crate::testing::Quiescence::Stalled);
            let expire = deadlines.try_recv().unwrap();
            let mut waiting = net.dial().dial(&name).await.unwrap();
            waiting.write_all(&header).await.unwrap();
            assert_eq!(run_to_quiescence(router.as_mut()).unwrap_err(), crate::testing::Quiescence::Stalled);
            assert!(deadlines.try_recv().is_err(), "the second arrival has not been admitted");
            expire.send(()).unwrap();
            drive(router.as_mut(), async {
                assert_eq!(stalled.read(&mut [0]).await.unwrap(), 0);
                let (_info, _link) = incoming.accept().await.unwrap();
                assert_eq!(waiting.read_u8().await.unwrap(), header::ACK);
                assert!(deadlines.try_recv().unwrap().is_closed(), "handoff discards the deadline");
            }).await;
        }).expect("expiration restores routing progress");
    }
}

/// Supplies connections whose buffers the test can prepare before acceptance.
struct Queued<C>(mpsc::UnboundedReceiver<C>);

impl<C: super::Conn> Listen for Queued<C> {
    /// A connection supplied by the test.
    type Conn = C;

    /// Receive the next prepared connection or report a closed supply.
    async fn accept(&mut self) -> io::Result<C> {
        self.0
            .recv()
            .await
            .ok_or_else(|| io::ErrorKind::BrokenPipe.into())
    }
}

/// A deadline also covers ACK flushing. Expiration releases the registration
/// and backlog reservation, allowing the same link token to establish again.
#[test]
fn routing_deadline_cancels_blocked_ack_flush() {
    run_to_quiescence(async {
        let (mut local, mut remote) = tokio::io::duplex(64);
        local.write_all(&[0; 64]).await.unwrap();
        let (queued, conns) = mpsc::unbounded_channel();
        queued.send(BufStream::new(local)).unwrap();
        let (started, mut deadlines) = mpsc::unbounded_channel();
        let net = MemoryNet::new();
        let (_a, mut incoming, router) = Endpoint::new(
            Deadlines {
                listen: Queued(conns),
                started,
            },
            MemoryName::new("a"),
            Buffered(net.dial()),
            Config {
                incoming_backlog: 1,
                pending_headers: 1,
                ..Config::default()
            },
        )
        .unwrap();
        let mut router = pin!(router);
        let token = Token::new();
        remote
            .write_all(&header::link_header(&token, b"peer"))
            .await
            .unwrap();
        assert_eq!(
            run_to_quiescence(router.as_mut()).unwrap_err(),
            crate::testing::Quiescence::Stalled
        );
        assert!(
            incoming.accept().now_or_never().is_none(),
            "ACK must flush before handoff"
        );
        deadlines.try_recv().unwrap().send(()).unwrap();
        assert_eq!(
            run_to_quiescence(router.as_mut()).unwrap_err(),
            crate::testing::Quiescence::Stalled
        );
        let mut bytes = Vec::new();
        remote.read_to_end(&mut bytes).await.unwrap();
        assert_eq!(bytes, [0; 64], "the ACK stayed buffered until cancellation");

        let (local, mut remote) = tokio::io::duplex(64);
        queued.send(BufStream::new(local)).unwrap();
        remote
            .write_all(&header::link_header(&token, b"peer"))
            .await
            .unwrap();
        drive(router.as_mut(), async {
            let (info, _link) = incoming.accept().await.unwrap();
            assert_eq!(info.token, token);
            assert_eq!(remote.read_u8().await.unwrap(), header::ACK);
            assert!(deadlines.try_recv().unwrap().is_closed());
        })
        .await;
    })
    .expect("ACK cancellation releases routing resources");
}

/// A caller-owned clock bounds fresh routing without timing out established
/// gossip or retaining deadlines on idle pooled connections.
#[tokio::test(start_paused = true)]
async fn routing_deadline_allows_idle_gossip_and_reuse() {
    use crate::{Gossip, Peer};
    use futures::{SinkExt, StreamExt};
    use std::time::Duration;

    /// Apply a short routing deadline while preserving connection I/O.
    struct Timed<L>(L);
    impl<L: Listen> Listen for Timed<L> {
        /// Preserve the wrapped listener's connection type.
        type Conn = L::Conn;
        /// Accept without imposing an I/O timeout on the connection.
        async fn accept(&mut self) -> io::Result<Self::Conn> {
            self.0.accept().await
        }
        /// Expire initial routing after ten seconds on the test clock.
        fn routing_deadline(&self) -> impl Future<Output = ()> + Send + 'static {
            tokio::time::sleep(Duration::from_secs(10))
        }
    }

    let net = MemoryNet::new();
    let dial = CountingDial::new(&net);
    let a_name = MemoryName::new("a");
    let (_a, mut incoming, a_router) = Endpoint::new(
        Timed(net.listen(&a_name)),
        a_name.clone(),
        dial.clone(),
        Config::default(),
    )
    .unwrap();
    let b_name = MemoryName::new("b");
    let (b, _incoming, b_router) = Endpoint::new(
        Timed(net.listen(&b_name)),
        b_name,
        dial.clone(),
        Config::default(),
    )
    .unwrap();
    drive(routers(a_router, b_router), async {
        let (linked, arrival) = futures::join!(b.link(a_name), incoming.accept());
        let mut at_b = linked.unwrap();
        let (_, mut at_a) = arrival.unwrap();
        complete_streams(&at_a, &mut at_b, STREAM_COUNT).await;
        complete_streams(&at_b, &mut at_a, STREAM_COUNT).await;
        let dials = dial.fresh_dials();
        let alice = Peer::<String>::seed().into_rumors();
        alice.send("first".into()).unwrap();
        let (joined, served) = futures::join!(
            Peer::<String>::bootstrap().join(&mut at_b),
            alice.gossip(&mut at_a),
        );
        served.unwrap();
        let bob = (match joined {
            crate::Joined::Joined { peer } => peer,
            _ => panic!("bootstrap must succeed"),
        })
        .into_rumors();
        let (mut ticks, when) = futures::channel::mpsc::channel(1);
        let mut a_driver = alice.gossip_when(futures::stream::pending::<Gossip>(), &mut at_a);
        let mut b_driver = bob.gossip_when(when, &mut at_b);
        for round in 0..2 {
            tokio::select! {
                result = a_driver.next() => panic!("idle gossip ended: {result:?}"),
                result = b_driver.next() => panic!("idle gossip ended: {result:?}"),
                _ = tokio::time::sleep(Duration::from_secs(30)) => {}
            }
            alice.send(format!("after idle {round}")).unwrap();
            ticks.send(Gossip::Unconditionally).await.unwrap();
            let (a, b) = tokio::time::timeout(Duration::from_secs(10), async {
                futures::join!(a_driver.next(), b_driver.next())
            })
            .await
            .expect("gossip resumes after the idle wait");
            a.unwrap().unwrap();
            b.unwrap().unwrap();
            assert!(alice.snapshot() == bob.snapshot());
            assert_eq!(
                dial.fresh_dials(),
                dials,
                "idle connections remain reusable"
            );
        }
    })
    .await;
}

proptest! {
    /// At capacity, new arrivals wait without displacing admitted attempts.
    /// Finishing or closing any admitted connection lets waiting arrivals route.
    #[test]
    fn pending_header_bound_preserves_admitted_work(
        capacity in 1usize..8,
        arrivals in 1usize..8,
        released in any::<usize>(),
        finish in any::<bool>(),
    ) {
        run_to_quiescence(async {
            let net = MemoryNet::new();
            let (_a, mut incoming, a_router) = endpoint(&net, "a", Config {
                pending_headers: capacity,
                ..Config::default()
            });
            let mut a_router = pin!(a_router);
            let name = MemoryName::new("a");
            let mut admitted = Vec::new();
            for _ in 0..capacity {
                let mut conn = net.dial().dial(&name).await.unwrap();
                conn.write_all(b"ROU").await.unwrap();
                admitted.push(conn);
            }
            assert_eq!(
                run_to_quiescence(a_router.as_mut()).expect_err("partial headers wait"),
                crate::testing::Quiescence::Stalled,
            );

            let mut waiting = Vec::new();
            for _ in 0..arrivals {
                let mut conn = net.dial().dial(&name).await.unwrap();
                let token = Token::new();
                conn.write_all(&header::link_header(&token, b"peer")).await.unwrap();
                waiting.push((conn, token));
            }
            assert_eq!(
                run_to_quiescence(a_router.as_mut()).expect_err("acceptance stays paused"),
                crate::testing::Quiescence::Stalled,
            );
            assert!(incoming.accept().now_or_never().is_none());
            for conn in &mut admitted {
                assert!(conn.read_u8().now_or_never().is_none(), "admitted work stays open");
            }

            let mut conn = admitted.remove(released % capacity);
            if finish {
                let token = Token::new();
                conn.write_all(&header::link_header(&token, b"peer")[3..]).await.unwrap();
                waiting.push((conn, token));
            } else {
                drop(conn);
            }
            drive(a_router.as_mut(), async {
                // One free slot suffices to route the entire waiting group.
                let mut tokens: Vec<_> = waiting.iter().map(|(_, token)| *token).collect();
                for _ in 0..waiting.len() {
                    let (info, _link) = incoming.accept().await.unwrap();
                    let index = tokens.iter().position(|token| *token == info.token).unwrap();
                    tokens.swap_remove(index);
                }
                for (mut conn, _) in waiting {
                    assert_eq!(conn.read_u8().await.unwrap(), header::ACK);
                }
                for mut conn in admitted {
                    let token = Token::new();
                    conn.write_all(&header::link_header(&token, b"peer")[3..]).await.unwrap();
                    assert_eq!(conn.read_u8().await.unwrap(), header::ACK);
                    let (info, _link) = incoming.accept().await.unwrap();
                    assert_eq!(info.token, token);
                }
            }).await;
        }).expect("waiting and admitted connections finish routing");
    }
}

/// A full incoming backlog rejects establishment while the dialer is
/// still waiting on the acknowledgement: the dialer gets a crisp
/// `Rejected`, not a queued link the application never sees.
#[test]
fn full_incoming_backlog_rejects_establishment() {
    pollster::block_on(async {
        let net = MemoryNet::new();
        let config = Config {
            incoming_backlog: 1,
            ..Config::default()
        };
        let (a, a_incoming, a_router) = endpoint(&net, "a", config);
        let (b, _b_incoming, b_router) = endpoint(&net, "b", Config::default());
        drive(routers(a_router, b_router), async {
            // The first link fills the undrained backlog.
            b.link(MemoryName::new("a"))
                .await
                .expect("the backlog admits one link");
            let Err(error) = b.link(MemoryName::new("a")).await else {
                panic!("a full backlog must reject establishment");
            };
            assert!(matches!(error, LinkError::Rejected));
            drop((a, a_incoming, b));
        })
        .await;
    });
}

/// A listener that answers establishment with anything but the
/// acknowledgement byte, or hangs up without answering, rejects the
/// link; the dialer's error says which contract failed.
#[test]
fn non_acknowledgement_rejects_establishment() {
    pollster::block_on(async {
        let net = MemoryNet::new();
        let fake = MemoryName::new("fake");
        let mut fake_listen = net.listen(&fake);
        let (b, _b_incoming, b_router) = endpoint(&net, "b", Config::default());
        drive(b_router, async {
            use super::Listen;

            // A wrong byte instead of the acknowledgement.
            let (linked, served) = futures::join!(b.link(fake.clone()), async {
                let mut conn = fake_listen
                    .accept()
                    .await
                    .expect("the fake listener accepts");
                conn.write_all(&[0x7f])
                    .await
                    .expect("the wrong byte writes");
                conn
            });
            let Err(error) = linked else {
                panic!("a wrong byte must reject establishment");
            };
            assert!(matches!(error, LinkError::Rejected));
            drop(served);

            // A hang-up before any answer.
            let (linked, ()) = futures::join!(b.link(fake.clone()), async {
                let conn = fake_listen
                    .accept()
                    .await
                    .expect("the fake listener accepts");
                drop(conn);
            });
            let Err(error) = linked else {
                panic!("a hang-up must reject establishment");
            };
            assert!(matches!(error, LinkError::Rejected));
        })
        .await;
    });
}

/// Establishment toward a name nothing listens at fails as transport
/// failure, not a hang: the dial's refusal passes through as `Io`.
#[test]
fn linking_to_nowhere_fails_as_transport() {
    pollster::block_on(async {
        let net = MemoryNet::new();
        let (b, _b_incoming, b_router) = endpoint(&net, "b", Config::default());
        drive(b_router, async {
            let Err(error) = b.link(MemoryName::new("nowhere")).await else {
                panic!("an unlistened name must refuse the dial");
            };
            assert!(matches!(error, LinkError::Io(_)));
        })
        .await;
    });
}

/// Dropped stream-open futures leave the link fully usable: whatever
/// stage the cancellation lands in, later opens route and deliver
/// exactly as before.
#[test]
fn cancelled_opens_leave_the_link_usable() {
    pollster::block_on(async {
        let net = MemoryNet::new();
        let (a, mut a_incoming, a_router) = endpoint(&net, "a", Config::default());
        let (b, _b_incoming, b_router) = endpoint(&net, "b", Config::default());
        drive(routers(a_router, b_router), async {
            let (at_b, _info, mut at_a) = establish(&b, "a", &mut a_incoming).await;

            // Cancel opens unpolled, and cancel them after a single
            // poll (the memory network resolves eagerly, so one poll
            // may already have opened and immediately closed a
            // connection; both fates must leave the link healthy).
            drop(at_b.connector.connect());
            for _ in 0..3 {
                let opened = at_b.connector.connect().now_or_never();
                drop(opened);
            }

            // An open that completed before its cancellation delivers
            // a connection that closes unwritten; drain any such
            // orphans until the genuine payload arrives.
            let open = async {
                let (mut tx, _) = at_b.connector.connect().await.expect("stream opens");
                tx.write_all(b"after cancellations")
                    .await
                    .expect("payload writes");
            };
            let read = async {
                loop {
                    let (mut rx, _) = at_a.acceptor.accept().await.expect("stream arrives");
                    let mut received = Vec::new();
                    rx.read_to_end(&mut received)
                        .await
                        .expect("stream reads out");
                    if !received.is_empty() {
                        break received;
                    }
                }
            };
            let ((), received) = futures::join!(open, read);
            assert_eq!(received, b"after cancellations");
            drop((a, b));
        })
        .await;
    });
}

/// Open one stream from `opener` to `acceptor`, move `payload` across
/// it, and end it by completion on both halves: the receiver reads
/// exactly the payload and completes there, never probing for
/// end-of-stream.
async fn transfer_completed<CRa, CWa, Ca, Aa, CRb, CWb, Cb, Ab>(
    opener: &Link<CRa, CWa, Ca, Aa>,
    acceptor: &mut Link<CRb, CWb, Cb, Ab>,
    payload: &[u8],
) where
    Ca: Connector,
    Ab: Acceptor,
{
    let open = async {
        let (mut tx, done) = opener.connector.connect().await.expect("stream opens");
        tx.write_all(payload).await.expect("payload writes");
        tx.flush().await.expect("payload flushes");
        done.complete(tx);
    };
    let read = async {
        let (mut rx, done) = acceptor.acceptor.accept().await.expect("stream arrives");
        let mut received = vec![0u8; payload.len()];
        rx.read_exact(&mut received)
            .await
            .expect("the completed stream delivers its bytes");
        done.complete(rx);
        received
    };
    let ((), received) = futures::join!(open, read);
    assert_eq!(received, payload);
}

/// Counts fresh connections while leaving reuse to the adapter.
#[derive(Clone)]
struct CountingDial {
    /// Supplies fresh connections from the test network.
    inner: MemoryDial,
    /// Shared by clones so all dial attempts are counted.
    fresh: Arc<AtomicUsize>,
}

impl CountingDial {
    /// Wrap the network's dialer with a shared connection counter.
    fn new(net: &MemoryNet) -> Self {
        Self {
            inner: net.dial(),
            fresh: Arc::default(),
        }
    }

    /// Read the number of fresh dials made through any clone.
    fn fresh_dials(&self) -> usize {
        self.fresh.load(Ordering::Relaxed)
    }
}

impl Dial for CountingDial {
    /// A listener name in the test network.
    type Addr = MemoryName;
    /// An in-memory connection supplied by the test network.
    type Conn = DuplexStream;

    /// Count the attempt and delegate to the test network.
    async fn dial(&self, addr: &MemoryName) -> io::Result<DuplexStream> {
        self.fresh.fetch_add(1, Ordering::Relaxed);
        self.inner.dial(addr).await
    }
}

/// Give the routers a few polls, so ready bytes land before the next
/// dial's single-poll admission.
async fn settle() {
    for _ in 0..8 {
        let mut yielded = false;
        poll_fn(|cx| {
            if yielded {
                Poll::Ready(())
            } else {
                yielded = true;
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        })
        .await;
    }
}

/// Completed connections carry later streams in both link directions.
#[test]
fn completed_streams_reuse_their_connection() {
    pollster::block_on(async {
        let net = MemoryNet::new();
        let dial = CountingDial::new(&net);
        let a_name = MemoryName::new("a");
        let (a, mut a_incoming, a_router) =
            Endpoint::new(net.listen(&a_name), a_name, dial.clone(), Config::default())
                .expect("a valid construction");
        let b_name = MemoryName::new("b");
        let (b, _b_incoming, b_router) =
            Endpoint::new(net.listen(&b_name), b_name, dial.clone(), Config::default())
                .expect("a valid construction");
        drive(routers(a_router, b_router), async {
            let (linked, arrival) =
                futures::join!(b.link(MemoryName::new("a")), a_incoming.accept());
            let at_b = linked.expect("establishment succeeds");
            let (_info, mut at_a) = arrival.expect("the router delivers the link");
            let established = dial.fresh_dials();

            // Forward: the first stream dials, the second reuses its
            // recycled connection.
            transfer_completed(&at_b, &mut at_a, b"paid by a dial").await;
            settle().await;
            transfer_completed(&at_b, &mut at_a, b"reuses recycled").await;
            assert_eq!(dial.fresh_dials(), established + 1);

            // Reverse: the accepting side's dial-back pools the same way.
            let mut at_b = at_b;
            transfer_completed(&at_a, &mut at_b, b"paid by a dial").await;
            settle().await;
            transfer_completed(&at_a, &mut at_b, b"reuses recycled").await;
            assert_eq!(dial.fresh_dials(), established + 2);
            drop((a, b));
        })
        .await;
    });
}

/// A dialer over socket addresses for construction tests; endpoint
/// construction never dials, so its `dial` is unreachable.
#[derive(Clone)]
struct SocketDial;

impl Dial for SocketDial {
    /// Exercise the socket-address wire encoding during construction.
    type Addr = std::net::SocketAddr;
    /// A placeholder connection type; construction does not dial.
    type Conn = tokio::io::DuplexStream;

    /// Fail if a construction-only test unexpectedly attempts to dial.
    async fn dial(&self, _addr: &Self::Addr) -> io::Result<Self::Conn> {
        unreachable!("construction tests never dial")
    }
}

/// An advertised name the address type refuses to encode fails
/// construction as `EndpointError::Unencodable`, never a panic.
///
/// The stock `SocketAddr` instantiation refuses a scoped IPv6 name (the
/// 18-byte wire form cannot carry the scope, and the unscoped address
/// dials a different peer), and construction is the one place the
/// router encodes, so the refusal surfaces exactly here.
#[test]
fn scoped_advertised_name_fails_construction() {
    use std::net::{Ipv6Addr, SocketAddr, SocketAddrV6};

    let net = MemoryNet::new();
    let scoped = SocketAddr::V6(SocketAddrV6::new(
        Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1),
        9000,
        0, // flowinfo
        3, // scope_id: interface index 3
    ));
    let Err(error) = Endpoint::new(
        net.listen(&MemoryName::new("scoped")),
        scoped,
        SocketDial,
        Config::default(),
    ) else {
        panic!("a scoped advertised name must fail construction");
    };
    assert!(matches!(error, EndpointError::Unencodable(_)));
}

/// An advertised name encoding outside the header's one-byte length
/// bound (empty, or past `MAX_ADDR_LEN`) fails construction as
/// `EndpointError::NameLength` carrying the offending length.
///
/// An empty name cannot be dialed back, and a longer one does not fit
/// the length prefix.
#[test]
fn out_of_bound_advertised_name_fails_construction() {
    let net = MemoryNet::new();
    for name in [String::new(), "x".repeat(header::MAX_ADDR_LEN + 1)] {
        let len = name.len();
        let Err(error) = Endpoint::new(
            net.listen(&MemoryName::new("bound")),
            MemoryName::new(name),
            net.dial(),
            Config::default(),
        ) else {
            panic!("an out-of-bound advertised name must fail construction");
        };
        assert!(matches!(error, EndpointError::NameLength(reported) if reported == len));
    }
}

/// A zero `Config` bound fails construction with the arm naming the
/// bound: a zero incoming backlog could never deliver a link, and
/// zero pending headers could never admit a connect header.
#[test]
fn zero_config_bounds_fail_construction() {
    let net = MemoryNet::new();
    let zero_backlog = Config {
        incoming_backlog: 0,
        ..Config::default()
    };
    let Err(error) = Endpoint::new(
        net.listen(&MemoryName::new("zero")),
        MemoryName::new("zero"),
        net.dial(),
        zero_backlog,
    ) else {
        panic!("a zero incoming backlog must fail construction");
    };
    assert!(matches!(error, EndpointError::ZeroIncomingBacklog));

    let zero_headers = Config {
        pending_headers: 0,
        ..Config::default()
    };
    let Err(error) = Endpoint::new(
        net.listen(&MemoryName::new("zero")),
        MemoryName::new("zero"),
        net.dial(),
        zero_headers,
    ) else {
        panic!("zero pending headers must fail construction");
    };
    assert!(matches!(error, EndpointError::ZeroPendingHeaders));
}

proptest! {
/// Header pressure must not break streams opened on a link's idle connections.
#[test]
fn pooled_streams_survive_header_pressure(count in 1usize..=STREAM_COUNT, pressure in 1usize..8) {
    crate::testing::run_to_quiescence(async {
        let net = MemoryNet::new();
        let dial = CountingDial::new(&net);
        let a_name = MemoryName::new("a");
        let (a, mut incoming, a_router) = Endpoint::new(
            net.listen(&a_name),
            a_name.clone(),
            dial.clone(),
            Config {
                pending_headers: 1,
                ..Config::default()
            },
        )
        .unwrap();
        let b_name = MemoryName::new("b");
        let (b, _incoming, b_router) =
            Endpoint::new(net.listen(&b_name), b_name, dial.clone(), Config::default()).unwrap();
        drive(routers(a_router, b_router), async {
            let (linked, arrival) = futures::join!(b.link(a_name.clone()), incoming.accept());
            let at_b = linked.unwrap();
            let (_, mut at_a) = arrival.unwrap();
            complete_streams(&at_b, &mut at_a, count).await;
            settle().await;
            let dials = dial.fresh_dials();

            let mut stalled = Vec::new();
            for _ in 0..pressure {
                let mut conn = net.dial().dial(&a_name).await.unwrap();
                conn.write_all(b"ROU").await.unwrap();
                stalled.push(conn);
                settle().await;
            }
            complete_streams(&at_b, &mut at_a, count).await;
            assert_eq!(dial.fresh_dials(), dials, "reuse every healthy connection");
            drop((a, stalled));
        })
        .await;
    })
    .expect("streams make progress despite header pressure");
}

}

/// Open a group before returning any connection, then transfer and complete
/// each stream. A yield between opens lets even a one-slot router keep up.
async fn complete_streams<D: Dial<Addr = MemoryName>>(
    from: &RoutedLink<D>,
    to: &mut RoutedLink<impl Dial<Addr = MemoryName>>,
    count: usize,
) {
    let mut opened = Vec::new();
    for _ in 0..count {
        opened.push(from.connector.connect().await.unwrap());
        settle().await;
    }
    for (mut tx, done) in opened {
        tx.write_all(b"payload").await.unwrap();
        done.complete(tx);
        let (mut rx, done) = to.acceptor.accept().await.unwrap();
        let mut bytes = [0; 7];
        rx.read_exact(&mut bytes).await.unwrap();
        assert_eq!(&bytes, b"payload");
        done.complete(rx);
        settle().await;
    }
}

/// Return a raw stream to the router and read its reuse decision: READY
/// when admitted, EOF when refused. Keep its dialing end to observe reuse.
async fn park(
    net: &MemoryNet,
    link: &mut RoutedLink<MemoryDial>,
    token: Token,
) -> (DuplexStream, Option<u8>) {
    let mut conn = net.dial().dial(&MemoryName::new("a")).await.unwrap();
    conn.write_all(&header::stream_header(&token))
        .await
        .unwrap();
    let (rx, done) = link.acceptor.accept().await.unwrap();
    done.complete(rx);
    let decision = conn.read_u8().await.ok();
    (conn, decision)
}

proptest! {
    /// Admission is bounded per link. Closing any idle connection frees one
    /// slot; dropping or evicting its link closes the rest without affecting
    /// another link to the same peer.
    #[test]
    fn idle_admission_and_release(dead in 0usize..STREAM_COUNT, evict in any::<bool>()) {
        run_to_quiescence(async {
            let net = MemoryNet::new();
            let (_a, mut incoming, a_router) = endpoint(&net, "a", Config::default());
            let (b, _incoming, b_router) = endpoint(&net, "b", Config::default());
            drive(routers(a_router, b_router), async {
                let (_b1, info1, mut a1) = establish(&b, "a", &mut incoming).await;
                let (b2, info2, mut a2) = establish(&b, "a", &mut incoming).await;
                let mut idle = Vec::new();
                for _ in 0..STREAM_COUNT {
                    let (conn, decision) = park(&net, &mut a1, info1.token).await;
                    assert_eq!(decision, Some(header::READY));
                    idle.push(conn);
                }
                for _ in 0..2 {
                    assert_eq!(park(&net, &mut a1, info1.token).await.1, None);
                }
                let (mut other, decision) = park(&net, &mut a2, info2.token).await;
                assert_eq!(decision, Some(header::READY));

                drop(idle.swap_remove(dead));
                settle().await;
                let (conn, decision) = park(&net, &mut a1, info1.token).await;
                assert_eq!(decision, Some(header::READY));
                idle.push(conn);
                assert_eq!(park(&net, &mut a1, info1.token).await.1, None);

                if evict {
                    let mut overflow = Vec::new();
                    for _ in 0..=STREAM_COUNT {
                        let mut conn = net.dial().dial(&MemoryName::new("a")).await.unwrap();
                        conn.write_all(&header::stream_header(&info1.token)).await.unwrap();
                        overflow.push(conn);
                        settle().await;
                    }
                    for _ in 0..STREAM_COUNT {
                        a1.acceptor.accept().await.unwrap();
                    }
                    assert!(a1.acceptor.accept().await.is_err());
                } else {
                    drop(a1);
                }
                for mut conn in idle {
                    assert_eq!(conn.read(&mut [0]).await.unwrap(), 0);
                }
                other.write_all(&header::stream_header(&info2.token)).await.unwrap();
                other.write_all(b"other link").await.unwrap();
                let (mut rx, _) = a2.acceptor.accept().await.unwrap();
                let mut bytes = [0; 10];
                rx.read_exact(&mut bytes).await.unwrap();
                assert_eq!(&bytes, b"other link");
                transfer(&b2, &mut a2, b"still usable").await;
            }).await;
        }).expect("admission and teardown complete");
    }
}

/// A stream whose receiver has not completed cannot block a later open.
/// Dropping one link's pool must leave another link's reusable connections intact.
#[test]
fn reuse_waits_for_completion_without_blocking_other_streams() {
    run_to_quiescence(async {
        let net = MemoryNet::new();
        let (_a, mut incoming, a_router) = endpoint(&net, "a", Config::default());
        let dial = CountingDial::new(&net);
        let b_name = MemoryName::new("b");
        let (b, _incoming, b_router) =
            Endpoint::new(net.listen(&b_name), b_name, dial.clone(), Config::default()).unwrap();
        drive(routers(a_router, b_router), async {
            let (linked, arrival) = futures::join!(b.link(MemoryName::new("a")), incoming.accept());
            let b1 = linked.unwrap();
            let (_, mut a1) = arrival.unwrap();
            let (linked, arrival) = futures::join!(b.link(MemoryName::new("a")), incoming.accept());
            let b2 = linked.unwrap();
            let (_, mut a2) = arrival.unwrap();
            let (tx, done) = b1.connector.connect().await.unwrap();
            done.complete(tx);
            let held = a1.acceptor.accept().await.unwrap();
            let dials = dial.fresh_dials();
            transfer_completed(&b1, &mut a1, b"independent").await;
            assert_eq!(dial.fresh_dials(), dials + 1);
            held.1.complete(held.0);
            settle().await;

            complete_streams(&b2, &mut a2, 2).await;
            let dials = dial.fresh_dials();
            drop(b1);
            drop(a1);
            settle().await;
            complete_streams(&b2, &mut a2, 2).await;
            assert_eq!(dial.fresh_dials(), dials, "the other link keeps its pool");
        })
        .await;
    })
    .expect("independent streams complete");
}

/// Either endpoint can disable outgoing reuse without affecting delivery.
#[test]
fn pooling_can_be_disabled_at_either_end() {
    run_to_quiescence(async {
        for pooling in [false, true] {
            let net = MemoryNet::new();
            let dial = CountingDial::new(&net);
            let a_name = MemoryName::new("a");
            let config = Config {
                pooling,
                ..Config::default()
            };
            let (_a, mut incoming, a_router) =
                Endpoint::new(net.listen(&a_name), a_name.clone(), dial.clone(), config).unwrap();
            let b_name = MemoryName::new("b");
            let (b, _incoming, b_router) =
                Endpoint::new(net.listen(&b_name), b_name, dial.clone(), config).unwrap();
            drive(routers(a_router, b_router), async {
                let (linked, arrival) = futures::join!(b.link(a_name), incoming.accept());
                let mut at_b = linked.unwrap();
                let (_, mut at_a) = arrival.unwrap();
                let dials = dial.fresh_dials();
                for _ in 0..2 {
                    complete_streams(&at_b, &mut at_a, 2).await;
                    complete_streams(&at_a, &mut at_b, 2).await;
                }
                assert_eq!(dial.fresh_dials() - dials, if pooling { 4 } else { 8 });
            })
            .await;
        }
    })
    .expect("streams complete with either pooling setting");
}

/// Supplies prebuilt connections so tests can control buffer capacity.
#[derive(Clone)]
struct PreparedDial(Arc<Mutex<Vec<DuplexStream>>>);

impl Dial for PreparedDial {
    /// An unused address; connections come from the prepared queue.
    type Addr = MemoryName;
    /// A prepared in-memory connection.
    type Conn = DuplexStream;

    /// Take a prepared connection, or fail when none remain.
    async fn dial(&self, _: &MemoryName) -> io::Result<Self::Conn> {
        self.0
            .lock()
            .unwrap()
            .pop()
            .ok_or_else(|| io::ErrorKind::ConnectionRefused.into())
    }
}

proptest! {
    /// Cancelling a reused open after any partial header closes that
    /// connection. A subsequent open can use a fresh connection normally.
    #[test]
    fn cancellation_during_reuse_closes_the_partial_header(prefix in 1usize..header::PREFIX_LEN) {
        run_to_quiescence(async {
            let (first, mut remote) = tokio::io::duplex(prefix);
            let (fresh, mut next_remote) = tokio::io::duplex(header::PREFIX_LEN);
            let dial = PreparedDial(Arc::new(Mutex::new(vec![fresh, first])));
            let token = Token::new();
            let connector = super::stream::StreamConnector::new(dial, MemoryName::new("peer"), token, true);
            let (opened, header) = futures::join!(connector.connect(), header::read::<MemoryName, _>(&mut remote));
            assert!(header.is_ok());
            let (conn, done) = opened.unwrap();
            done.complete(conn);
            remote.write_all(&[header::READY]).await.unwrap();
            assert!(connector.connect().now_or_never().is_none());
            let mut received = Vec::new();
            remote.read_to_end(&mut received).await.unwrap();
            assert_eq!(received, header::stream_header(&token)[..prefix]);
            let (opened, header) = futures::join!(connector.connect(), header::read::<MemoryName, _>(&mut next_remote));
            assert!(opened.is_ok());
            assert!(matches!(header.unwrap(), header::Header::Stream { token: next } if next == token));
        }).expect("cancellation closes the partial stream");
    }
}

/// The outgoing pool refuses excess returns and closes every retained
/// connection when its last connector drops, even if a completion is still held.
#[test]
fn outgoing_pool_is_bounded_and_released_with_its_connector() {
    run_to_quiescence(async {
        let net = MemoryNet::new();
        let name = MemoryName::new("peer");
        let mut listen = net.listen(&name);
        let connector = super::stream::StreamConnector::new(net.dial(), name, Token::new(), true);
        let mut opened = Vec::new();
        let mut remotes = Vec::new();
        for _ in 0..STREAM_COUNT + 2 {
            opened.push(connector.connect().await.unwrap());
            let mut remote = super::Listen::accept(&mut listen).await.unwrap();
            header::read::<MemoryName, _>(&mut remote).await.unwrap();
            remotes.push(remote);
        }
        let held = opened.pop().unwrap();
        for (conn, done) in opened {
            done.complete(conn);
        }
        assert_eq!(remotes[STREAM_COUNT].read(&mut [0]).await.unwrap(), 0);
        drop(connector);
        held.1.complete(held.0);
        for mut remote in remotes {
            assert_eq!(remote.read(&mut [0]).await.unwrap(), 0);
        }
    })
    .expect("pool teardown closes its connections");
}

/// A transport yielding with READY buffered does not stall a new stream.
/// Pooling resumes when the transport can make progress again.
#[tokio::test]
async fn reuse_probe_allows_transport_yields() {
    let net = MemoryNet::new();
    let a_name = MemoryName::new("a");
    let (_a, mut incoming, a_router) = Endpoint::new(
        net.listen(&a_name),
        a_name.clone(),
        net.dial(),
        Config::default(),
    )
    .unwrap();
    let dial = CountingDial::new(&net);
    let b_name = MemoryName::new("b");
    let (b, _incoming, b_router) =
        Endpoint::new(net.listen(&b_name), b_name, dial.clone(), Config::default()).unwrap();
    let task = tokio::spawn(routers(a_router, b_router));
    let (linked, arrival) = tokio::join!(b.link(a_name), incoming.accept());
    let at_b = linked.unwrap();
    let (_, mut at_a) = arrival.unwrap();
    complete_streams(&at_b, &mut at_a, 2).await;
    let dials = dial.fresh_dials();
    spend_budget().await;
    transfer_completed(&at_b, &mut at_a, b"fresh while the probe yields").await;
    assert_eq!(dial.fresh_dials(), dials + 1);
    settle().await;
    transfer_completed(&at_b, &mut at_a, b"reused after the yield").await;
    assert_eq!(dial.fresh_dials(), dials + 1);
    task.abort();
}

/// Exhaust Tokio's cooperative budget using buffered reads without yielding.
async fn spend_budget() {
    let (mut rx, mut tx) = tokio::io::duplex(1024);
    tx.write_all(&[0; 512]).await.unwrap();
    while tokio::task::coop::has_budget_remaining() {
        rx.read_u8().await.unwrap();
    }
}

/// The incoming idle bound includes returns queued for an unpolled router.
/// Excess connections close immediately, before any READY can be sent.
#[test]
fn idle_admission_bounds_queued_returns() {
    run_to_quiescence(async {
        let net = MemoryNet::new();
        let (_a, mut incoming, a_router) = endpoint(&net, "a", Config::default());
        let (b, _incoming, b_router) = endpoint(&net, "b", Config::default());
        drive(routers(a_router, b_router), async {
            let (_b, info, mut at_a) = establish(&b, "a", &mut incoming).await;
            let mut remotes = Vec::new();
            let mut returned = Vec::new();
            for _ in 0..=STREAM_COUNT {
                let mut conn = net.dial().dial(&MemoryName::new("a")).await.unwrap();
                conn.write_all(&header::stream_header(&info.token))
                    .await
                    .unwrap();
                returned.push(at_a.acceptor.accept().await.unwrap());
                remotes.push(conn);
            }
            for (conn, done) in returned {
                done.complete(conn);
            }
            // Observe refusal without polling the router between returns.
            assert_eq!(
                remotes
                    .last_mut()
                    .unwrap()
                    .read(&mut [0])
                    .now_or_never()
                    .unwrap()
                    .unwrap(),
                0
            );
            for conn in &mut remotes[..STREAM_COUNT] {
                assert_eq!(conn.read_u8().await.unwrap(), header::READY);
            }
        })
        .await;
    })
    .expect("bounded returns complete");
}

/// Dropping a link or its router closes queued returns without another
/// router poll. A completion held across teardown cannot retain a connection.
#[test]
fn queued_returns_follow_owner_lifetime() {
    for stop_router in [false, true] {
        run_to_quiescence(async {
            let net = MemoryNet::new();
            let (a, mut incoming, a_router) = endpoint(&net, "a", Config::default());
            let (b, _incoming, b_router) = endpoint(&net, "b", Config::default());
            let mut a_router = Box::pin(a_router);
            let (mut at_a, mut remotes, mut returned) =
                drive(routers(&mut a_router, b_router), async {
                    let (_b, info, mut at_a) = establish(&b, "a", &mut incoming).await;
                    let mut remotes = Vec::new();
                    let mut returned = Vec::new();
                    for _ in 0..3 {
                        let mut conn = net.dial().dial(&MemoryName::new("a")).await.unwrap();
                        conn.write_all(&header::stream_header(&info.token))
                            .await
                            .unwrap();
                        returned.push(at_a.acceptor.accept().await.unwrap());
                        remotes.push(conn);
                    }
                    (at_a, remotes, returned)
                })
                .await;
            let held = returned.pop().unwrap();
            for (conn, done) in returned {
                done.complete(conn);
            }
            if stop_router {
                drop(a_router);
                assert!(
                    at_a.acceptor
                        .accept()
                        .now_or_never()
                        .expect("a stopped router closes its stream supplies")
                        .is_err()
                );
                assert!(matches!(
                    a.link(MemoryName::new("b")).now_or_never()
                        .expect("a stopped router rejects new links"),
                    Err(LinkError::Io(error)) if error.kind() == io::ErrorKind::BrokenPipe
                ));
            } else {
                drop(at_a);
            }
            held.1.complete(held.0);
            for conn in &mut remotes {
                assert_eq!(
                    conn.read(&mut [0])
                        .now_or_never()
                        .expect("teardown closes returns without polling the router")
                        .unwrap(),
                    0
                );
            }
        })
        .expect("teardown releases returned connections");
    }
}

proptest! {
    /// Completion bursts on several links retain each link's full allowance.
    /// A coalesced wakeup must deliver READY to every admitted connection.
    #[test]
    fn concurrent_return_bursts_stay_per_link(links in 2usize..5, count in 1usize..=STREAM_COUNT) {
        run_to_quiescence(async {
            let net = MemoryNet::new();
            let (_a, mut incoming, a_router) = endpoint(&net, "a", Config::default());
            let (b, _incoming, b_router) = endpoint(&net, "b", Config::default());
            drive(routers(a_router, b_router), async {
                let mut owners = Vec::new();
                let mut remotes = Vec::new();
                let mut returned = Vec::new();
                for _ in 0..links {
                    let (at_b, info, mut at_a) = establish(&b, "a", &mut incoming).await;
                    for _ in 0..count {
                        let mut conn = net.dial().dial(&MemoryName::new("a")).await.unwrap();
                        conn.write_all(&header::stream_header(&info.token)).await.unwrap();
                        returned.push(at_a.acceptor.accept().await.unwrap());
                        remotes.push(conn);
                    }
                    owners.push((at_b, at_a));
                }
                for (conn, done) in returned {
                    done.complete(conn);
                }
                for conn in &mut remotes {
                    assert_eq!(conn.read_u8().await.unwrap(), header::READY);
                }
            }).await;
        }).expect("one wakeup drains all links' returns");
    }
}
