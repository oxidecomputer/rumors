use std::collections::HashSet;

use proptest::prelude::*;
use rumors::testing::property::{
    NetworkSeed, SESSION_DIVISOR, distinct_network_seeds, divided_proptest, network_seed,
};
use tokio::io::{BufReader, duplex, split};

use super::*;
use crate::point::{Connection, Link, Replicas, Settings, TcpBuffers, Transport};
use crate::sizes::SizeDistribution;
use crate::units::{LinkRate, Rate};

/// Runs `future` to completion on a current-thread runtime with timers,
/// as a peer process does.
fn block_on<F: std::future::Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a current-thread runtime builds")
        .block_on(future)
}

/// A point over `replicas` with variable message sizes and default settings.
fn point(replicas: Replicas) -> Point {
    Point {
        replicas,
        sizes: SizeDistribution {
            mean: 60.0,
            cv: 0.5,
            max: 1000,
        },
        settings: Settings::default(),
        link: Link::Emulated {
            rate: LinkRate::symmetric(Rate::Uncapped),
            rtt: "1ms".parse().unwrap(),
        },
        connection: Connection::Fresh,
        transport: Transport::default(),
        tcp_buffers: TcpBuffers::Host,
    }
}

/// Small replica pairs, every count free to be zero.
fn small_replicas() -> impl Strategy<Value = Replicas> {
    (0u64..300, 0u64..100, 0u64..100, 0u64..50, 0u64..50).prop_map(
        |(shared, insert_a, insert_b, redact_a, redact_b)| Replicas {
            shared,
            insert_a,
            insert_b,
            redact_a,
            redact_b,
        },
    )
}

/// Builds the pair for `point` in the network `network` names, drawing its
/// history from the same stream after the network's secret, so the seed
/// alone reproduces the build.
async fn seeded_build(point: &Point, network: NetworkSeed) -> Pair {
    let mut rng = network.rng();
    let root = Peer::seed_rng(&mut rng);
    build_from(point, root, &mut rng).await.unwrap()
}

/// Builds the pair for `point` in the network `network` names and joins
/// side B's process's copy from it over an in-memory pipe, as the two peer
/// processes do over TCP.
async fn build_and_join(point: &Point, network: NetworkSeed) -> (Pair, Rumors<Payload>) {
    let pair = seeded_build(point, network).await;
    let (near, far) = duplex(1 << 16);
    let (near_read, near_write) = split(near);
    let (far_read, far_write) = split(far);
    let (served, joined) = tokio::join!(
        pair.b.gossip_once(BufReader::new(far_read), far_write),
        join_from(BufReader::new(near_read), near_write, point)
    );
    served.unwrap();
    (pair, joined.unwrap())
}

divided_proptest! {
    #![divisor(SESSION_DIVISOR)]

    /// Each side holds the live messages its counts require, side B's joined
    /// copy holds B's live set message for message, and one session converges
    /// both on *s + i_A + i_B*.
    ///
    /// That is: every insertion crosses, and every redaction is honored on the
    /// other side. Each case builds a pair (two in-memory joins) and runs a
    /// join and a session, so it takes the session divisor.
    ///
    /// The claim holds for every network seed and every drawn history.
    #[test]
    fn joined_copy_matches_and_converges_as_counted(
        replicas in small_replicas(),
        network in network_seed(),
    ) {
        let point = point(replicas);
        block_on(async {
            let (pair, b) = build_and_join(&point, network).await;
            assert_eq!(pair.a.snapshot().len() as u64, replicas.live_a());
            assert_eq!(b.snapshot().len() as u64, replicas.live_b());
            assert_eq!(fingerprint(&b.snapshot()), pair.report.b_fingerprint);

            let (near, far) = duplex(1 << 16);
            let (near_read, near_write) = split(near);
            let (far_read, far_write) = split(far);
            let (from_a, from_b) = tokio::join!(
                pair.a.gossip_once(BufReader::new(near_read), near_write),
                b.gossip_once(BufReader::new(far_read), far_write)
            );
            from_a.unwrap();
            from_b.unwrap();
            let converged = replicas.shared + replicas.insert_a + replicas.insert_b;
            assert_eq!(pair.a.snapshot().len() as u64, converged);
            assert_eq!(b.snapshot().len() as u64, converged);
        });
    }

    /// Two builds with different offsets draw disjoint versions, and so
    /// independent trees.
    ///
    /// The builds found two independent networks. One build in 2,048 draws
    /// the first build's offset again, and equal offsets are the same draw
    /// by construction, so the second build continues its stream until its
    /// offset differs.
    ///
    /// The claim holds for every pair of network seeds.
    #[test]
    fn offsets_draw_disjoint_versions((x_network, y_network) in distinct_network_seeds()) {
        let point = point(Replicas {
            shared: 200,
            ..Replicas::default()
        });
        block_on(async {
            let x = seeded_build(&point, x_network).await;
            let mut y_rng = y_network.rng();
            let mut y = build_from(&point, Peer::seed_rng(&mut y_rng), &mut y_rng)
                .await
                .unwrap();
            while y.report.offset == x.report.offset {
                y = build_from(&point, Peer::seed_rng(&mut y_rng), &mut y_rng)
                    .await
                    .unwrap();
            }
            let xs: HashSet<Version> = x.a.snapshot().versions().cloned().collect();
            let ys = y.a.snapshot();
            assert!(ys.versions().all(|version| !xs.contains(version)));
        });
    }
}
