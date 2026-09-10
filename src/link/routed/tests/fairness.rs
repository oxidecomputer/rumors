//! Routing progress under sustained arrivals, backpressure, and cancellation.

use super::*;

/// Queue a header, optionally filling the response buffer to block ACK or READY.
async fn enqueue(
    queued: &mpsc::UnboundedSender<BufStream<DuplexStream>>,
    header: &[u8],
    block_response: bool,
) -> DuplexStream {
    let (mut local, mut remote) = tokio::io::duplex(64);
    if block_response {
        local.write_all(&[0; 64]).await.unwrap();
    }
    remote.write_all(header).await.unwrap();
    queued.send(BufStream::new(local)).unwrap();
    remote
}

/// Supplies prepared connections among an endless stream of closed arrivals.
struct Flood(mpsc::UnboundedReceiver<BufStream<DuplexStream>>);

impl Listen for Flood {
    /// Buffered connections exercise routing flushes as well as reads.
    type Conn = BufStream<DuplexStream>;

    /// Keep acceptance ready even when no test connection is queued.
    async fn accept(&mut self) -> io::Result<Self::Conn> {
        Ok(self.0.try_recv().unwrap_or_else(|_| {
            let (local, remote) = tokio::io::duplex(1);
            drop(remote);
            BufStream::new(local)
        }))
    }
}

proptest! {
    /// Cancelling establishment revokes its token even when ACK is already
    /// buffered. Later attempts still establish and receive data streams.
    #[test]
    fn cancelled_establishments_release_routes(acknowledgements in prop::collection::vec(any::<bool>(), 1..8)) {
        run_to_quiescence(async {
            let net = MemoryNet::new();
            let (endpoint, _incoming, router) = endpoint(&net, "a", Config::default());
            let peer_name = MemoryName::new("peer");
            let mut peer = net.listen(&peer_name);
            drive(router, async {
                for acknowledge in acknowledgements {
                    let mut opening = Box::pin(endpoint.link(peer_name.clone()));
                    assert!(opening.as_mut().now_or_never().is_none());
                    let mut remote = peer.accept().await.unwrap();
                    let header::Header::Link { token, .. } = header::read::<MemoryName, _>(&mut remote).await.unwrap()
                        else { panic!("expected a link header") };
                    if acknowledge {
                        remote.write_u8(header::ACK).await.unwrap();
                    }
                    drop(opening);
                    assert_eq!(remote.read(&mut [0]).await.unwrap(), 0);
                    let mut orphan = net.dial().dial(&MemoryName::new("a")).await.unwrap();
                    orphan.write_all(&header::stream_header(&token)).await.unwrap();
                    assert_eq!(orphan.read(&mut [0]).await.unwrap(), 0, "the cancelled token was revoked");
                }
                let mut opening = Box::pin(endpoint.link(peer_name));
                assert!(opening.as_mut().now_or_never().is_none());
                let mut remote = peer.accept().await.unwrap();
                let header::Header::Link { token, .. } = header::read::<MemoryName, _>(&mut remote).await.unwrap()
                    else { panic!("expected a link header") };
                remote.write_u8(header::ACK).await.unwrap();
                let mut link = opening.await.unwrap();
                let mut stream = net.dial().dial(&MemoryName::new("a")).await.unwrap();
                stream.write_all(&header::stream_header(&token)).await.unwrap();
                stream.write_u8(42).await.unwrap();
                let (mut conn, _done) = link.acceptor.accept().await.unwrap();
                assert_eq!(conn.read_u8().await.unwrap(), 42);
            }).await;
        }).expect("cancellation leaves later establishment usable");
    }

    /// A backlog of immediately ready accepts and failed headers must still
    /// let the executor poll other tasks before the entire backlog is drained.
    #[test]
    fn ready_arrivals_yield_to_other_tasks(burst in 64usize..256, capacity in 1usize..8) {
        let (queued, connections) = mpsc::unbounded_channel();
        let mut peers = Vec::new();
        for _ in 0..burst {
            let (local, mut remote) = tokio::io::duplex(1);
            remote.shutdown().now_or_never().unwrap().unwrap();
            queued.send(local).unwrap();
            peers.push(remote);
        }
        let net = MemoryNet::new();
        let (_endpoint, _incoming, router) = Endpoint::new(
            Queued(connections), MemoryName::new("a"), net.dial(),
            Config { pending_headers: capacity, ..Config::default() },
        ).unwrap();
        let mut router = pin!(router);
        run_to_quiescence(async {
            assert!(router.as_mut().now_or_never().is_none());
            assert!(peers.iter_mut().any(|peer| peer.read_u8().now_or_never().is_none()),
                "one poll monopolized the whole arrival backlog");
        }).expect("another task can run beside the busy router");
    }

    /// Fresh establishment, data delivery, and READY processing all progress
    /// while the listener remains ready with an endless supply of failed arrivals.
    #[test]
    fn continuous_arrivals_allow_establishment_and_reuse(
        capacity in 1usize..8, links in 1usize..5, rounds in 1usize..8,
    ) {
        run_to_quiescence(async {
            let (queued, connections) = mpsc::unbounded_channel();
            let net = MemoryNet::new();
            let (_endpoint, mut incoming, router) = Endpoint::new(
                Flood(connections), MemoryName::new("a"), Buffered(net.dial()),
                Config { pending_headers: capacity, ..Config::default() },
            ).unwrap();
            drive(router, async {
                let mut owners = Vec::new();
                for _ in 0..links {
                    let token = Token::new();
                    let mut control = enqueue(&queued, &header::link_header(&token, b"peer"), false).await;
                    let (info, link) = incoming.accept().await.unwrap();
                    assert_eq!(info.token, token);
                    assert_eq!(control.read_u8().await.unwrap(), header::ACK);
                    let stream = enqueue(&queued, &header::stream_header(&token), false).await;
                    owners.push((token, control, stream, link));
                }
                for round in 0..rounds {
                    for (token, _control, remote, link) in &mut owners {
                        remote.write_u8(round as u8).await.unwrap();
                        let (mut conn, done) = link.acceptor.accept().await.unwrap();
                        assert_eq!(conn.read_u8().await.unwrap(), round as u8);
                        done.complete(conn);
                        assert_eq!(remote.read_u8().await.unwrap(), header::READY);
                        remote.write_all(&header::stream_header(token)).await.unwrap();
                    }
                }
            }).await;
        }).expect("routing and reuse progress under continuous arrivals");
    }

    /// Blocked ACK and READY flushes do not stop healthy links. Each stalled
    /// attempt can later finish or close without disrupting the others.
    #[test]
    fn blocked_routing_writes_leave_other_links_live(
        stalls in prop::collection::vec((any::<bool>(), any::<bool>()), 1..6),
        rounds in 1usize..6,
    ) {
        run_to_quiescence(async {
            let (queued, connections) = mpsc::unbounded_channel();
            let net = MemoryNet::new();
            let (_endpoint, mut incoming, router) = Endpoint::new(
                Queued(connections), MemoryName::new("a"), Buffered(net.dial()),
                Config {
                    pending_headers: stalls.len() + 1,
                    incoming_backlog: stalls.len() + 1,
                    ..Config::default()
                },
            ).unwrap();
            drive(router, async {
                let token = Token::new();
                let mut control = enqueue(&queued, &header::link_header(&token, b"peer"), false).await;
                let (_, mut owner) = incoming.accept().await.unwrap();
                assert_eq!(control.read_u8().await.unwrap(), header::ACK);
                let mut blocked = Vec::new();
                for (reuse, cancel) in stalls {
                    let bytes = if reuse {
                        header::stream_header(&token).to_vec()
                    } else {
                        header::link_header(&Token::new(), b"peer")
                    };
                    let remote = enqueue(&queued, &bytes, true).await;
                    if reuse {
                        let (conn, done) = owner.acceptor.accept().await.unwrap();
                        done.complete(conn);
                    }
                    blocked.push((remote, reuse, cancel));
                }
                let healthy = Token::new();
                let mut peer = enqueue(&queued, &header::link_header(&healthy, b"peer"), false).await;
                let (info, mut link) = incoming.accept().await.unwrap();
                assert_eq!(info.token, healthy);
                assert_eq!(peer.read_u8().await.unwrap(), header::ACK);
                let mut stream = enqueue(&queued, &header::stream_header(&healthy), false).await;
                for round in 0..rounds {
                    stream.write_u8(round as u8).await.unwrap();
                    let (mut conn, done) = link.acceptor.accept().await.unwrap();
                    assert_eq!(conn.read_u8().await.unwrap(), round as u8);
                    done.complete(conn);
                    assert_eq!(stream.read_u8().await.unwrap(), header::READY);
                    stream.write_all(&header::stream_header(&healthy)).await.unwrap();
                }
                for (mut remote, reuse, cancel) in blocked {
                    if cancel {
                        drop(remote);
                        continue;
                    }
                    let mut padding = [0; 64];
                    remote.read_exact(&mut padding).await.unwrap();
                    assert_eq!(padding, [0; 64]);
                    if reuse {
                        assert_eq!(remote.read_u8().await.unwrap(), header::READY);
                        remote.write_all(&header::stream_header(&token)).await.unwrap();
                        let _stream = owner.acceptor.accept().await.unwrap();
                    } else {
                        assert_eq!(remote.read_u8().await.unwrap(), header::ACK);
                        let _link = incoming.accept().await.unwrap();
                    }
                }
            }).await;
        }).expect("blocked routing writes remain independent");
    }
}
