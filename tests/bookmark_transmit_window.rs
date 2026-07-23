//! The persisted bookmark record dominates every own-party event a session
//! transmits.
//!
//! The bookmark exists so a crashed peer can reclaim its identity without
//! reminting causal coordinates the network already holds. That safety
//! reduces to one invariant at the transmit boundary: **at the moment a
//! session snapshots its tree for the wire, the persisted record's own-party
//! projection dominates the snapshot's own-party version.** An own event that
//! crosses the wire while the durable record does not cover it is a time
//! bomb: the emitter crashes, restarts from a replica that satisfies the
//! record (but not the uncovered event), reclaims its region, and its next
//! tick collides with a coordinate another replica durably holds — which the
//! deletion-honoring merge then reads as an already-deleted message,
//! silently destroying live content network-wide.
//!
//! The adversarial schedule that separates the record from the wire is
//! narrow and needs *both* of these, which this suite constructs
//! deterministically:
//!
//! - a session whose bookmark update persists a version whose own-party
//!   projection the network already knows (serving a bootstrap clears the
//!   update-suppression token without any new own event, so the next
//!   session re-records the *old* frontier); and
//! - an own event committed while that update's durable write is in flight
//!   (the write is application I/O of unbounded duration), landing in the
//!   session's tree snapshot but not in the persisted record.
//!
//! [`GatedBookmark`] makes the write's in-flight window a deterministic
//! interleaving point: the test parks the session inside the persist, commits
//! a send, and releases.

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use before::Version;
use rumors::{Bookmark, BookmarkError, Peer, Rumors, Serialized};
use tokio::sync::Notify;

use crate::common::flaky::{DurableStore, persisted_record};
use crate::common::wire::tokio_block_on as block_on;

/// The message payload: a test-unique id.
type Msg = u64;

/// Capacity for every in-memory link stream, matching the sibling bookmark
/// suites.
const LINK_BUF: usize = 8 * 1024;

/// A full-mesh heal round cap; a correct fleet reaches a fixed point in a
/// handful of rounds, and the cap turns a convergence bug into a loud failure.
const MAX_HEAL_ROUNDS: usize = 16;

// ---- the gated bookmark ------------------------------------------------------

/// An in-memory [`Bookmark`] whose `store` can be armed to park mid-persist:
/// the deterministic stand-in for a durable write racing a concurrent `send`.
///
/// Disarmed (the default) it persists synchronously, like the sibling suites'
/// in-memory bookmarks. Armed, the next `store` signals `entered`, then parks
/// until `release` is notified; the test body runs in that window, on the
/// same current-thread runtime, so the interleaving is exact and replayable.
#[derive(Clone, Debug)]
struct GatedBookmark {
    store: DurableStore,
    armed: Arc<AtomicBool>,
    entered: Arc<Notify>,
    release: Arc<Notify>,
}

impl GatedBookmark {
    fn new(store: DurableStore) -> Self {
        GatedBookmark {
            store,
            armed: Arc::new(AtomicBool::new(false)),
            entered: Arc::new(Notify::new()),
            release: Arc::new(Notify::new()),
        }
    }

    /// Park the next `store` call on the gate.
    fn arm(&self) {
        self.armed.store(true, Ordering::SeqCst);
    }

    /// Wait until an armed `store` has parked.
    async fn entered(&self) {
        self.entered.notified().await;
    }

    /// Let the parked `store` complete.
    fn release(&self) {
        self.release.notify_one();
    }
}

#[derive(Debug, thiserror::Error)]
#[error("gated bookmark never fails")]
struct NeverFails;

impl BookmarkError for GatedBookmark {
    type Error = NeverFails;
}

impl Bookmark for GatedBookmark {
    type Reader = std::io::Cursor<Vec<u8>>;

    async fn load(&self) -> Result<Option<Self::Reader>, Self::Error> {
        Ok(self.store.lock().unwrap().clone().map(std::io::Cursor::new))
    }

    async fn store<F>(&self, write: F) -> Result<(), Self::Error>
    where
        F: for<'a> FnOnce(&'a mut (dyn tokio::io::AsyncWrite + Unpin + Send)) -> Serialized<'a>
            + Send,
    {
        if self.armed.swap(false, Ordering::SeqCst) {
            self.entered.notify_one();
            self.release.notified().await;
        }
        let mut bytes = Vec::new();
        write(&mut bytes).await.expect("in-memory serialize");
        *self.store.lock().unwrap() = Some(bytes);
        Ok(())
    }
}

// ---- schedule helpers --------------------------------------------------------

/// Bootstrap a fresh peer with bookmark `bm` from `server` over a clean
/// in-memory link, returning the booted peer's [`Rumors`].
async fn boot_from(
    server: &Rumors<Msg, GatedBookmark>,
    bm: GatedBookmark,
) -> Rumors<Msg, GatedBookmark> {
    let server = server.clone();
    let (boot_side, serve_side) = rumors::link::memory_with_capacity(LINK_BUF);
    let boot = tokio::spawn(async move {
        let mut link = boot_side;
        let peer = Peer::<Msg>::bootstrap(&mut link)
            .await
            .expect("bootstrap ok")
            .expect("the server is established");
        peer.bookmark(bm).await.expect("in-memory persist")
    });
    let serve = tokio::spawn(async move {
        let mut link = serve_side;
        server.gossip(&mut link).await
    });
    let (boot_out, serve_out) = tokio::join!(boot, serve);
    serve_out.unwrap().expect("serve bootstrap");
    boot_out.unwrap().into_rumors()
}

/// One clean gossip session between two peers, both sides required to succeed.
async fn gossip(a: &Rumors<Msg, GatedBookmark>, b: &Rumors<Msg, GatedBookmark>) {
    let (a, b) = (a.clone(), b.clone());
    let (side_a, side_b) = rumors::link::memory_with_capacity(LINK_BUF);
    let task_a = tokio::spawn(async move {
        let mut link = side_a;
        a.gossip(&mut link).await
    });
    let task_b = tokio::spawn(async move {
        let mut link = side_b;
        b.gossip(&mut link).await
    });
    let (out_a, out_b) = tokio::join!(task_a, task_b);
    out_a.unwrap().expect("gossip side a");
    out_b.unwrap().expect("gossip side b");
}

/// The version stamped on the live leaf carrying `payload`, if present.
fn leaf_version(rumors: &Rumors<Msg, GatedBookmark>, payload: Msg) -> Option<Version> {
    rumors
        .snapshot()
        .iter()
        .find(|(_, _, value)| ***value == payload)
        .map(|(_, version, _)| version.clone())
}

/// The staged world: A bookmarked and gated, B and C booted from it, one
/// converged message, and A's suppression token cleared by the bootstrap
/// donations — the precondition under which A's next update re-records an
/// already-propagated frontier.
struct Scene {
    a: Rumors<Msg, GatedBookmark>,
    b: Rumors<Msg, GatedBookmark>,
    c: Rumors<Msg, GatedBookmark>,
    bm_a: GatedBookmark,
    store_a: DurableStore,
}

const M0: Msg = 0;
const M1: Msg = 1;

/// Build the scene, then run the gated session: A gossips with B while M1
/// commits inside the bookmark persist's in-flight window.
async fn transmit_during_persist() -> Scene {
    let store_a = DurableStore::default();
    let bm_a = GatedBookmark::new(store_a.clone());
    let a = Peer::<Msg>::seed()
        .bookmark(bm_a.clone())
        .await
        .expect("a pristine seed attaches its bookmark without touching storage")
        .into_rumors();
    a.send(M0);

    // Each serve records A's frontier, then slices the donated fork out of
    // the record — clearing the update-suppression token with no new own
    // event, so the gated session below re-records the same frontier.
    let b = boot_from(&a, GatedBookmark::new(DurableStore::default())).await;
    let c = boot_from(&a, GatedBookmark::new(DurableStore::default())).await;

    // The gated session: A's bookmark update reads the frontier, stages the
    // record, and parks inside the durable write; M1 commits in that window,
    // so it rides the session's snapshot while the record persisted without
    // covering it.
    bm_a.arm();
    let (side_a, side_b) = rumors::link::memory_with_capacity(LINK_BUF);
    let ga = {
        let a = a.clone();
        tokio::spawn(async move {
            let mut link = side_a;
            a.gossip(&mut link).await
        })
    };
    let gb = {
        let b = b.clone();
        tokio::spawn(async move {
            let mut link = side_b;
            b.gossip(&mut link).await
        })
    };
    bm_a.entered().await;
    a.send(M1);
    bm_a.release();
    let (out_a, out_b) = tokio::join!(ga, gb);
    out_a.unwrap().expect("gated gossip side a");
    out_b.unwrap().expect("gated gossip side b");

    Scene {
        a,
        b,
        c,
        bm_a,
        store_a,
    }
}

// ---- the tests ----------------------------------------------------------------

/// A session's persisted bookmark record covers every own-party event the
/// session transmits: the wire never carries an own event the durable record
/// does not dominate, however the persist's in-flight window interleaves
/// with concurrent sends.
#[test]
fn record_dominates_the_transmitted_frontier() {
    block_on(async {
        let scene = transmit_during_persist().await;
        let Scene { a, b, store_a, .. } = &scene;

        let party = a
            .dangerously_alias_party()
            .expect("a live peer holds its party");
        let recorded = persisted_record(store_a)
            .remove(&a.network())
            .expect("the gated session persisted a record")
            .into_iter()
            .fold(Version::new(), |acc, clock| acc | clock.version());
        let transmitted = b.snapshot().latest().clone();

        let own_transmitted = transmitted / &party;
        let own_recorded = recorded / &party;
        assert!(
            own_transmitted <= own_recorded,
            "the session transmitted own-party events the persisted record does not \
             dominate: transmitted {own_transmitted:?}, recorded {own_recorded:?}: a crash \
             now reclaims the identity below a frontier another replica durably holds",
        );
    });
}

/// A crash after the gated session cannot make the network destroy a live,
/// unredacted message: the restarted peer reclaims its identity only at a
/// frontier that accounts for every own event it ever transmitted, so no
/// remint collides with a coordinate a replica durably holds.
#[test]
fn restart_after_transmit_never_destroys_durable_messages() {
    block_on(async {
        let scene = transmit_during_persist().await;
        let Scene {
            a,
            b,
            c,
            bm_a,
            store_a,
        } = scene;

        // Whether M1 became durable: it reached B in the gated session. (Its
        // sender crashing below cannot erase it from the network once it did.)
        let m1_at_b = leaf_version(&b, M1);

        // Crash A: every handle drops; only the durable store survives.
        drop(a);
        drop(bm_a);

        // Restart from C, whose frontier satisfies A's persisted record but
        // never saw M1: the record admits reclaiming here iff it accounts for
        // everything A transmitted.
        let a2 = boot_from(&c, GatedBookmark::new(store_a.clone())).await;
        gossip(&a2, &c).await; // the first update reclaims

        // The restarted peer mints fresh events. A remint below M1's durable
        // coordinate is the recycle this test exists to catch; several ticks
        // give a colliding placement every chance to occur.
        for i in 0..8 {
            a2.send(100 + i);
        }

        // Heal to a fixed point over clean wires.
        let peers = [&a2, &b, &c];
        let mut rounds = 0;
        loop {
            let before: Vec<_> = peers.iter().map(|p| p.snapshot().hash()).collect();
            for i in 0..peers.len() {
                for j in (i + 1)..peers.len() {
                    gossip(peers[i], peers[j]).await;
                }
            }
            let after: Vec<_> = peers.iter().map(|p| p.snapshot().hash()).collect();
            if before == after {
                break;
            }
            rounds += 1;
            assert!(
                rounds <= MAX_HEAL_ROUNDS,
                "fleet did not converge within {MAX_HEAL_ROUNDS} rounds",
            );
        }

        // Convergence: all replicas hold identical content.
        let reference = a2.snapshot().hash();
        assert_eq!(reference, b.snapshot().hash(), "B diverged after the heal");
        assert_eq!(reference, c.snapshot().hash(), "C diverged after the heal");

        // The property: a message that became durable pre-crash survives the
        // heal on every replica. (If M1 never reached B, its loss with A's
        // crash was never known to the network and is not a violation.)
        if let Some(version) = m1_at_b {
            for (label, peer) in [("A'", &a2), ("B", &b), ("C", &c)] {
                assert!(
                    leaf_version(peer, M1).is_some(),
                    "durable message M1 (version {version:?}) was destroyed at {label} by a \
                     restarted peer reminting below a transmitted own-party frontier",
                );
            }
        }
    });
}
