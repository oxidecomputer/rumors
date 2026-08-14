use std::collections::HashMap;
use std::future::{Future, poll_fn};
use std::io;
use std::mem::take;
use std::pin::{Pin, pin};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use futures::FutureExt;
use futures::future::{Either, select, try_join};
use futures::task::noop_waker;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt, DuplexStream, ReadBuf};

use super::header::{self, Token};
use super::{Config, Dial, Endpoint, EndpointError, Incoming, LinkError, LinkInfo, RoutedLink};
use crate::link::{Acceptor, Connector, Link, STREAM_COUNT};
use crate::testing::{MemoryDial, MemoryName, MemoryNet};

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

/// A stream queue driven past a full session complement proves peer
/// misbehavior and evicts the link: the queued complement still
/// drains, the next accept errors instead of hanging, and the token
/// routes nothing afterward.
#[test]
fn queue_overflow_evicts_the_link() {
    pollster::block_on(async {
        let net = MemoryNet::new();
        let (a, mut a_incoming, a_router) = endpoint(&net, "a", Config::default());
        let (b, _b_incoming, b_router) = endpoint(&net, "b", Config::default());
        drive(routers(a_router, b_router), async {
            let (_at_b, info, mut at_a) = establish(&b, "a", &mut a_incoming).await;

            // A misbehaving peer: one connection past the complement,
            // dialed raw so the link's own connector is not implicated.
            let mut flood = Vec::new();
            for _ in 0..=STREAM_COUNT {
                let mut conn = net.dial().dial(&MemoryName::new("a")).await.expect("dial");
                conn.write_all(&header::stream_header(&info.token))
                    .await
                    .expect("header writes");
                flood.push(conn);
            }

            // The queued complement drains; the accept after it
            // surfaces the eviction as a transport error.
            for _ in 0..STREAM_COUNT {
                at_a.acceptor.accept().await.expect("queued streams drain");
            }
            at_a.acceptor
                .accept()
                .await
                .expect_err("eviction surfaces as a transport error");

            // The token is revoked: a further stream connection is
            // dropped on sight.
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

/// Past the pending-header bound the oldest stalled connection is
/// evicted (its dialer observes end-of-stream), and the router keeps
/// serving: the count bound is the no-clock substitute for a header
/// deadline.
#[test]
fn pending_header_bound_evicts_oldest() {
    pollster::block_on(async {
        let net = MemoryNet::new();
        let config = Config {
            pending_headers: 1,
            ..Config::default()
        };
        let (a, mut a_incoming, a_router) = endpoint(&net, "a", config);
        let (b, _b_incoming, b_router) = endpoint(&net, "b", Config::default());
        drive(routers(a_router, b_router), async {
            let mut stalled = net.dial().dial(&MemoryName::new("a")).await.expect("dial");
            stalled
                .write_all(b"ROU")
                .await
                .expect("a partial magic writes");

            // The establishment connection displaces the stalled one.
            let (_at_b, _info, _at_a) = establish(&b, "a", &mut a_incoming).await;
            let mut drained = Vec::new();
            stalled
                .read_to_end(&mut drained)
                .await
                .expect("eviction drops the stalled connection");
            assert!(drained.is_empty());
            drop((a, b));
        })
        .await;
    });
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

/// A dialer that pools recycled connections per peer and counts the
/// fresh dials it performs: how the reuse tests observe which streams
/// paid for a connection.
#[derive(Clone)]
struct PoolingDial {
    inner: MemoryDial,
    /// Recycled, awaiting the router's ready byte.
    pending: Arc<Mutex<HashMap<String, Vec<DuplexStream>>>>,
    /// Ready for reuse.
    pool: Arc<Mutex<HashMap<String, Vec<DuplexStream>>>>,
    fresh: Arc<AtomicUsize>,
}

impl PoolingDial {
    fn new(net: &MemoryNet) -> Self {
        PoolingDial {
            inner: net.dial(),
            pending: Arc::default(),
            pool: Arc::default(),
            fresh: Arc::default(),
        }
    }

    fn fresh_dials(&self) -> usize {
        self.fresh.load(Ordering::Relaxed)
    }

    /// Admit pending connections whose ready byte has arrived, polling
    /// each read exactly once: the byte is consumed off the dialing
    /// path, never awaited.
    fn admit(&self, addr: &MemoryName) {
        let mut pending = self.pending.lock().expect("pending lock");
        let Some(conns) = pending.get_mut(&addr.0) else {
            return;
        };
        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);
        for mut conn in take(conns) {
            let mut byte = [0u8; 1];
            let mut buf = ReadBuf::new(&mut byte);
            match Pin::new(&mut conn).poll_read(&mut cx, &mut buf) {
                Poll::Ready(Ok(())) if buf.filled().len() == 1 => {
                    self.pool
                        .lock()
                        .expect("pool lock")
                        .entry(addr.0.clone())
                        .or_default()
                        .push(conn);
                }
                Poll::Pending => conns.push(conn),
                // EOF or error: the connection is dead.
                _ => {}
            }
        }
    }
}

impl Dial for PoolingDial {
    type Addr = MemoryName;
    type Conn = DuplexStream;

    async fn dial(&self, addr: &MemoryName) -> io::Result<DuplexStream> {
        self.admit(addr);
        let pooled = self
            .pool
            .lock()
            .expect("pool lock")
            .get_mut(&addr.0)
            .and_then(Vec::pop);
        if let Some(conn) = pooled {
            return Ok(conn);
        }
        self.fresh.fetch_add(1, Ordering::Relaxed);
        self.inner.dial(addr).await
    }

    fn recycle(&self, peer: &MemoryName, conn: DuplexStream) {
        self.pending
            .lock()
            .expect("pending lock")
            .entry(peer.0.clone())
            .or_default()
            .push(conn);
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

/// A completed stream's connection carries the next stream instead of
/// a fresh dial.
///
/// The write half hands it back through [`Dial::recycle`], the read
/// half returns it to the router for its next connect header, and the
/// recycled connection routes exactly as a dialed one would — in both
/// directions of the link.
#[test]
fn completed_streams_reuse_their_connection() {
    pollster::block_on(async {
        let net = MemoryNet::new();
        let dial = PoolingDial::new(&net);
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
    type Addr = std::net::SocketAddr;
    type Conn = tokio::io::DuplexStream;

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
