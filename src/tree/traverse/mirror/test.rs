use std::cell::OnceCell;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use borsh::{BorshDeserialize, BorshSerialize};
use proptest::collection::vec;
use proptest::prelude::*;
use tokio::runtime::Runtime;

use crate::tree::arb::{arb_tree_root, nth_party};
use crate::tree::traverse::{Action, act};
use crate::tree::typed::Path;
use crate::{message::Message, version::Version};

use super::{local, mirror, remote};

thread_local! {
    /// One current-thread tokio runtime per test thread, initialized lazily on
    /// first use. Cargo's test harness gives each test its own thread, and
    /// proptest cases within a test run sequentially on that thread, so a
    /// single runtime per thread serves every case without contention.
    static RT: OnceCell<Runtime> = const { OnceCell::new() };
}

/// Drive an async future to completion on the per-thread runtime. Used to
/// bridge proptest's synchronous body with the now-async `mirror` driver.
fn block_on<F: Future>(fut: F) -> F::Output {
    RT.with(|cell| {
        cell.get_or_init(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("build tokio current-thread runtime")
        })
        .block_on(fut)
    })
}

/// Which mirror-protocol arrangement to drive: the cardinal product of
/// `{local, remote}` for the initiator side and the responder side.
///
/// In every variant, "A" is the client (holds tree `a`) and "B" is the
/// server (holds tree `b`). What varies is which side's state the *test
/// thread* holds directly (`Known`) versus accesses via a wire proxy
/// (`Remote`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scenario {
    /// Single-threaded, all in-memory: `initiator(local_a, local_b)`.
    LocalLocal,
    /// Test thread holds A locally; B runs on a peer thread reachable
    /// over a duplex pipe.
    LocalRemote,
}

/// In-memory full-duplex byte channel capacity, in bytes. The mirror
/// protocol strictly alternates messages within a session, so the receiver
/// is always reading by the time the sender writes; a small buffer is
/// sufficient and exercises backpressure naturally.
const DUPLEX_BUF: usize = 8 * 1024;

/// Drive the mirror protocol through the high-level [`super::mirror`]
/// driver under the chosen [`Scenario`], and return the reconciled tree
/// (which must be equal on both sides if the protocol converged).
fn mirror_via<T>(
    a: crate::tree::Root<T>,
    b: crate::tree::Root<T>,
    scenario: Scenario,
) -> crate::tree::Root<T>
where
    T: PartialEq + std::fmt::Debug + BorshSerialize + BorshDeserialize + Send + Sync,
{
    block_on(async move {
        match scenario {
            Scenario::LocalLocal => {
                let local_a = local::Exchange::silent(a);
                let local_b = local::Exchange::silent(b);
                match mirror(local_a, local_b).await {
                    Err(e) => match e {},
                    Ok(result) => result.1,
                }
            }

            Scenario::LocalRemote => {
                // Two ends of a full-duplex in-memory pipe: anything the
                // client side writes is readable by the server side and
                // vice versa. Splitting each end gives the four halves the
                // remote proxies expect.
                let (a_side, b_side) = tokio::io::duplex(DUPLEX_BUF);
                let (a_r, a_w) = tokio::io::split(a_side);
                let (b_r, b_w) = tokio::io::split(b_side);

                let local_a = local::Exchange::silent(a);
                let remote_b = remote::Exchange::start(a_r, a_w);
                let client = mirror(local_a, remote_b);

                let local_b = local::Exchange::silent(b);
                let remote_a = remote::Exchange::start(b_r, b_w);
                let server = mirror(local_b, remote_a);

                // Both sides poll on the same current-thread task; no
                // spawning, no Send bound, no thread handoff.
                let (client_result, server_result) = tokio::join!(client, server);
                let out = client_result.expect("test client").0;
                let peer_out = server_result.expect("peer server").0;
                assert_eq!(out, peer_out, "local-remote endpoints should converge");
                out
            }
        }
    })
}

const SCENARIOS: [Scenario; 2] = [Scenario::LocalLocal, Scenario::LocalRemote];

proptest! {

    /// Mirroring a node with itself is a no-op: the two replicas have
    /// identical content and versions, so the reconciled tree is unchanged.
    #[test]
    fn idempotent(a in arb_tree_root(0, 0..=8)) {
        for scenario in SCENARIOS {
            prop_assert_eq!(mirror_via(a.clone(), a.clone(), scenario), a.clone());
        }
    }

    /// The reconciled tree is the same regardless of which replica
    /// initiates and which responds.
    #[test]
    fn commutative(
        a in arb_tree_root(0, 0..=8),
        b in arb_tree_root(1, 0..=8),
    ) {
        for scenario in SCENARIOS {
            prop_assert_eq!(
                mirror_via(a.clone(), b.clone(), scenario),
                mirror_via(b.clone(), a.clone(), scenario),
            );
        }
    }

    /// Re-mirroring the result with a peer already synced with is a no-op:
    /// the result already contains everything the peer had.
    #[test]
    fn absorptive(
        a in arb_tree_root(0, 0..=8),
        b in arb_tree_root(1, 0..=8),
    ) {
        for scenario in SCENARIOS {
            let ab = mirror_via(a.clone(), b.clone(), scenario);
            prop_assert_eq!(mirror_via(ab.clone(), b.clone(), scenario), ab);
        }
    }

    /// Three-way mirror is order-independent: syncing (a,b) then c
    /// produces the same tree as syncing a then (b,c).
    #[test]
    fn associative(
        a in arb_tree_root(0, 0..=4),
        b in arb_tree_root(1, 0..=4),
        c in arb_tree_root(2, 0..=4),
    ) {
        for scenario in SCENARIOS {
            let ab_c = mirror_via(
                mirror_via(a.clone(), b.clone(), scenario),
                c.clone(),
                scenario,
            );
            let a_bc = mirror_via(
                a.clone(),
                mirror_via(b.clone(), c.clone(), scenario),
                scenario,
            );
            prop_assert_eq!(ab_c, a_bc);
        }
    }

    /// Mirror is equivalent to replaying both sides' full action
    /// histories — inserts and forgets — through a single `act` call.
    #[test]
    fn equivalent_to_cross_react(
        entries_a in vec(any::<bool>(), 0..=8),
        entries_b in vec(any::<bool>(), 0..=8),
    ) {
        // Tick the party's disjoint clock once per action so every action
        // carries a strictly-increasing version on that party: inserts take
        // the first `len` ticks, forgets the ticks after them, mirroring the
        // old `(party, scalar)` numbering with distinct, ascending versions.
        // Each leaf goes to its content-addressed path (as a real insert does),
        // and a forget targets the path of the insert it cancels — matching how
        // `redact` reuses the key surfaced by the original insert.
        let make_actions = |party_index: usize, forgets: &[bool]| -> Vec<_> {
            let p = nth_party(party_index);
            let mut version = Version::new();
            let mut actions: Vec<(Path, Version, Action<()>)> = Vec::new();
            let mut paths: Vec<Path> = Vec::new();
            for _ in forgets {
                version.tick(&p);
                let message = Message::new(());
                let path = Path::for_leaf(&version, message.bytes());
                paths.push(path);
                actions.push((path, version.clone(), Action::Insert(message)));
            }
            for (forget, path) in forgets.iter().zip(&paths) {
                if *forget {
                    version.tick(&p);
                    actions.push((*path, version.clone(), Action::Forget));
                }
            }
            actions
        };

        let actions_a = make_actions(0, &entries_a);
        let actions_b = make_actions(1, &entries_b);

        // The wrapper version must be a causal upper bound on every action
        // we apply — `Tree::react` maintains the same invariant by `|=`-ing
        // each action's version into the tree's version vector.
        let wrap = |actions: &[(Path, Version, Action<()>)]| crate::tree::Root {
            ceiling: actions
                .iter()
                .fold(Version::default(), |acc, (_, v, _)| acc | v.clone()),
            root: pollster::block_on(act(None, actions.to_vec(), crate::tree::ignore)),
        };

        let tree_a = wrap(&actions_a);
        let tree_b = wrap(&actions_b);

        let mut all_actions = actions_a;
        all_actions.extend(actions_b);
        let expected = wrap(&all_actions);

        for scenario in SCENARIOS {
            let mirrored = mirror_via(tree_a.clone(), tree_b.clone(), scenario);
            prop_assert_eq!(mirrored, expected.clone());
        }
    }
}

/// A write transport that delivers nothing to its inner writer until an
/// explicit flush — the defining property of a buffering layer such as a
/// compression codec or a [`std::io::BufWriter`], and exactly the property
/// [`tokio::io::duplex`] lacks (it forwards every write immediately, which is
/// why the other scenarios never exercised the flush path).
///
/// `poll_write` accepts bytes into a local buffer and reports progress, but
/// holds them back from the inner writer until `poll_flush` drains them.
struct HoldUntilFlush<W> {
    inner: W,
    buf: Vec<u8>,
    sent: usize,
}

impl<W> HoldUntilFlush<W> {
    fn new(inner: W) -> Self {
        Self {
            inner,
            buf: Vec::new(),
            sent: 0,
        }
    }
}

impl<W: tokio::io::AsyncWrite + Unpin> tokio::io::AsyncWrite for HoldUntilFlush<W> {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        data: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        self.get_mut().buf.extend_from_slice(data);
        Poll::Ready(Ok(data.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        let this = self.get_mut();
        while this.sent < this.buf.len() {
            match Pin::new(&mut this.inner).poll_write(cx, &this.buf[this.sent..]) {
                Poll::Ready(Ok(0)) => {
                    return Poll::Ready(Err(std::io::ErrorKind::WriteZero.into()));
                }
                Poll::Ready(Ok(n)) => this.sent += n,
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            }
        }
        this.buf.clear();
        this.sent = 0;
        Pin::new(&mut this.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.as_mut().poll_flush(cx) {
            Poll::Ready(Ok(())) => {}
            other => return other,
        }
        Pin::new(&mut self.get_mut().inner).poll_shutdown(cx)
    }
}

/// Regression: the protocol handshake must make progress over a transport that
/// delivers bytes only on flush.
///
/// Each peer writes an 8-byte preamble and then `read_exact`s the peer's 8
/// bytes before sending anything further. If the preamble is left sitting in a
/// buffering writer (`write_all` reaches only the buffer, never the wire), both
/// peers block forever — a deadlock that a raw socket hides because the kernel
/// forwards immediately, but that any compression/buffering layer exposes. We
/// drive both handshakes concurrently under a timeout and require completion.
#[test]
fn handshake_flushes_over_buffering_transport() {
    use std::sync::mpsc;
    use std::time::Duration;

    // Run both handshakes on a watchdog thread. A deadlock can't be caught with
    // a future-level timeout here (tokio's `time` feature is off in dev-deps),
    // so we bound it from outside: if the pair hasn't reported back within the
    // deadline, the handshakes are wedged.
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let both_ok = block_on(async {
            // `duplex` cross-connects the two ends: bytes written to `a_side`
            // are readable from `b_side` and vice versa. Wrapping each write
            // half in `HoldUntilFlush` makes delivery contingent on the
            // handshake flushing its preamble.
            let (a_side, b_side) = tokio::io::duplex(64);
            let (mut a_r, a_w) = tokio::io::split(a_side);
            let (mut b_r, b_w) = tokio::io::split(b_side);
            let mut a_w = HoldUntilFlush::new(a_w);
            let mut b_w = HoldUntilFlush::new(b_w);

            let (ra, rb) = tokio::join!(
                remote::handshake(&mut a_r, &mut a_w),
                remote::handshake(&mut b_r, &mut b_w),
            );
            ra.is_ok() && rb.is_ok()
        });
        let _ = tx.send(both_ok);
    });

    match rx.recv_timeout(Duration::from_secs(5)) {
        Ok(true) => {}
        Ok(false) => panic!("handshake errored over a flush-only transport"),
        Err(_) => panic!("handshake deadlocked over a flush-only transport"),
    }
}
