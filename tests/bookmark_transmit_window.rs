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
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use before::Version;
use rumors::{Bookmark, BookmarkError, Peer, Rumors, Serialized};
use tokio::sync::Notify;

use crate::common::flaky::{DurableStore, persisted_record};
use crate::common::wire::tokio_block_on as block_on;
use rumors::testing::SnapshotCollect as _;

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
    /// Fail the Nth `store` call from now (1 = the very next); 0 = disarmed.
    fail_at: Arc<AtomicUsize>,
}

impl GatedBookmark {
    fn new(store: DurableStore) -> Self {
        GatedBookmark {
            store,
            armed: Arc::new(AtomicBool::new(false)),
            entered: Arc::new(Notify::new()),
            release: Arc::new(Notify::new()),
            fail_at: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Make the `n`th `store` call from now (1 = the very next) return an
    /// injected fault, committing nothing.
    fn fail_at(&self, n: usize) {
        self.fail_at.store(n, Ordering::SeqCst);
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
#[error("injected store fault")]
struct InjectedFault;

impl BookmarkError for GatedBookmark {
    type Error = InjectedFault;
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
        match self.fail_at.load(Ordering::SeqCst) {
            0 => {}
            1 => {
                self.fail_at.store(0, Ordering::SeqCst);
                return Err(InjectedFault);
            }
            n => self.fail_at.store(n - 1, Ordering::SeqCst),
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
        let peer = Peer::<Msg>::bootstrap()
            .join(&mut link)
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
        .collected()
        .find(|(_, _, value)| **value == payload)
        .map(|(_, version, _)| version.clone())
}

/// The staged world: A bookmarked and gated, B and C booted from it, one
/// converged message, and A's suppression token cleared by the bootstrap
/// donations.
///
/// This is the precondition under which A's next update re-records an
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
        .sync_window_floor()
        .bookmark(bm_a.clone())
        .await
        .expect("a pristine seed attaches its bookmark without touching storage")
        .into_rumors();
    a.send(M0)
        .await
        .expect("the in-memory backend is infallible");

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
    a.send(M1)
        .await
        .expect("the in-memory backend is infallible");
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
/// session transmits.
///
/// The wire never carries an own event the durable record
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

        let own_transmitted = &transmitted / &party;
        let own_recorded = &recorded / &party;
        assert!(
            own_transmitted <= own_recorded,
            "the session transmitted own-party events the persisted record does not \
             dominate: transmitted {own_transmitted:?}, recorded {own_recorded:?}: a crash \
             now reclaims the identity below a frontier another replica durably holds",
        );
    });
}

/// A session future dropped mid-persist never suppresses the next update.
///
/// The suppression token commits only when the durable write completes, so
/// a cancelled write leaves no token claiming coverage the disk lacks, and
/// the next session persists afresh before transmitting.
///
/// Cancelled sessions are an anticipated class (dropping a gossip future is
/// documented to behave like an `Err`), and the persist is application I/O
/// of unbounded duration — precisely where a drop lands in practice. A token
/// staged before the write survives such a drop as a lie: the next session
/// sees the live `(party, version)` "current", skips the persist, and
/// transmits own events the durable record does not cover — re-opening the
/// remint collision through a schedule the in-flight-window fix does not
/// touch.
#[test]
fn cancelled_persist_never_suppresses_the_next_update() {
    block_on(async {
        // The scene through the two bootstrap serves, exactly as
        // `transmit_during_persist` stages it: token cleared by the
        // donations, record persisted at M0's frontier.
        let store_a = DurableStore::default();
        let bm_a = GatedBookmark::new(store_a.clone());
        let a = Peer::<Msg>::seed()
            .sync_window_floor()
            .bookmark(bm_a.clone())
            .await
            .expect("a pristine seed attaches its bookmark without touching storage")
            .into_rumors();
        pollster::block_on(a.send(M0)).expect("the in-memory backend is infallible");
        let b = boot_from(&a, GatedBookmark::new(DurableStore::default())).await;
        let _c = boot_from(&a, GatedBookmark::new(DurableStore::default())).await;

        // M1 advances the frontier, so the next update stages a token for
        // M1's version and parks in the durable write; dropping the session
        // futures there cancels the persist mid-flight.
        pollster::block_on(a.send(M1)).expect("the in-memory backend is infallible");
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
        ga.abort();
        gb.abort();
        let (out_a, out_b) = tokio::join!(ga, gb);
        assert!(
            out_a.is_err() && out_b.is_err(),
            "both session futures were dropped mid-persist",
        );

        // The next session runs on a fresh link. It must persist M1's
        // frontier before transmitting M1: a suppression token surviving
        // the cancelled write would skip that persist.
        gossip(&a, &b).await;

        let party = a
            .dangerously_alias_party()
            .expect("a live peer holds its party");
        let recorded = persisted_record(&store_a)
            .remove(&a.network())
            .expect("the follow-up session persisted a record")
            .into_iter()
            .fold(Version::new(), |acc, clock| acc | clock.version());
        let transmitted = b.snapshot().latest().clone();

        let own_transmitted = &transmitted / &party;
        let own_recorded = &recorded / &party;
        assert!(
            own_transmitted <= own_recorded,
            "a session after a cancelled persist transmitted own-party events the durable \
             record does not dominate: transmitted {own_transmitted:?}, recorded \
             {own_recorded:?}: the suppression token outlived the write it claimed",
        );
    });
}

/// A crash after the gated session cannot make the network destroy a live,
/// unredacted message.
///
/// The restarted peer reclaims its identity only at a
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
        //
        // Under the transmit-window invariant the gated session snapshots
        // its tree before the persist, so M1 — committed inside the persist's
        // in-flight window — stays out of the session and dies, never
        // durable, with A's crash: the destruction arm below is vacuous on
        // this schedule and goes live only if a change lets a mid-persist
        // commit ride the wire again. Asserting the expected outcome makes
        // such drift loud here rather than silently un-arming the check.
        let m1_at_b = leaf_version(&b, M1);
        assert!(
            m1_at_b.is_none(),
            "the gated session must not transmit an own event committed inside the \
             persist's in-flight window",
        );

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
            pollster::block_on(a2.send(100 + i)).expect("the in-memory backend is infallible");
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

/// A failed donation persist aborts the serve before the party crosses the
/// wire.
///
/// The donor's identity and durable record are exactly as before the
/// attempt, the newcomer receives nothing, and the next serve donates
/// cleanly — a crash at any moment around the abort strands no region.
///
/// The abort's mechanics under test: the slice runs in memory, its write
/// fails, the failure resets the in-memory record to the authoritative
/// on-disk state, the session returns [`rumors::Error::Bookmark`] without
/// the party crossing the wire, and the speculative fork re-joins the live
/// party on the way out.
#[test]
fn donation_persist_failure_aborts_before_the_wire() {
    block_on(async {
        let store_a = DurableStore::default();
        let bm_a = GatedBookmark::new(store_a.clone());
        let a = Peer::<Msg>::seed()
            .sync_window_floor()
            .bookmark(bm_a.clone())
            .await
            .expect("a pristine seed attaches its bookmark without touching storage")
            .into_rumors();
        pollster::block_on(a.send(M0)).expect("the in-memory backend is infallible");
        let b = boot_from(&a, GatedBookmark::new(DurableStore::default())).await;

        let party_before = a
            .dangerously_alias_party()
            .expect("a live peer holds its party");
        // Settle the record: a session with B re-records the post-donation
        // identity (the serve's slice cleared the suppression token), so the
        // failing serve below mutates nothing but the donation itself.
        gossip(&a, &b).await;
        let bytes_before = store_a.lock().unwrap().clone();

        // Serve a bootstrap whose donation persist fails. The session's
        // update is suppressed (the record is current), so the donation
        // slice's write is the next store call.
        bm_a.fail_at(1);
        let (boot_side, serve_side) = rumors::link::memory_with_capacity(LINK_BUF);
        let boot = tokio::spawn(async move {
            let mut link = boot_side;
            Peer::<Msg>::bootstrap().join(&mut link).await
        });
        let serve = {
            let a = a.clone();
            tokio::spawn(async move {
                let mut link = serve_side;
                a.gossip(&mut link).await
            })
        };
        let (boot_out, serve_out) = tokio::join!(boot, serve);
        assert!(
            matches!(serve_out.unwrap(), Err(rumors::Error::Bookmark(_))),
            "the serve must surface the failed donation persist",
        );
        assert!(
            !matches!(boot_out.unwrap(), Ok(Some(_))),
            "the newcomer must not receive a party the donor could not persist away",
        );

        // The abort left no trace: identity re-joined, disk untouched.
        assert_eq!(
            a.dangerously_alias_party()
                .expect("a live peer holds its party"),
            party_before,
            "the speculative fork must re-join the donor's party on abort",
        );
        assert_eq!(
            *store_a.lock().unwrap(),
            bytes_before,
            "a failed donation persist must leave the durable record untouched",
        );

        // The next serve donates cleanly from the recovered state.
        let d = boot_from(&a, GatedBookmark::new(DurableStore::default())).await;
        gossip(&a, &d).await;
        assert_eq!(
            a.snapshot().hash(),
            d.snapshot().hash(),
            "the recovered donor serves and converges normally",
        );
    });
}

/// Repeated donation-persist aborts normalize: after any number of failed
/// serves, one clean serve leaves the fleet with disjoint identities,
/// converged content, and every failed newcomer holding nothing.
///
/// The single-abort mechanics are pinned above; this drives the *schedule* —
/// abort, recover, abort again — through the same live party, so a residue
/// any one abort leaves behind (a stale suppression token, a half-reset
/// record, an unreturned fork fragment) compounds where this test can see it.
#[test]
fn repeated_donation_aborts_normalize() {
    block_on(async {
        let store_a = DurableStore::default();
        let bm_a = GatedBookmark::new(store_a.clone());
        let a = Peer::<Msg>::seed()
            .sync_window_floor()
            .bookmark(bm_a.clone())
            .await
            .expect("a pristine seed attaches its bookmark without touching storage")
            .into_rumors();
        pollster::block_on(a.send(M0)).expect("the in-memory backend is infallible");
        let b = boot_from(&a, GatedBookmark::new(DurableStore::default())).await;
        let party_before = a
            .dangerously_alias_party()
            .expect("a live peer holds its party");

        for round in 0..3 {
            // A failed write resets the in-memory record, so the next
            // session's update re-records (one store call) before the
            // donation slice's write (the second): fail the second.
            bm_a.fail_at(2);
            let (boot_side, serve_side) = rumors::link::memory_with_capacity(LINK_BUF);
            let boot = tokio::spawn(async move {
                let mut link = boot_side;
                Peer::<Msg>::bootstrap().join(&mut link).await
            });
            let serve = {
                let a = a.clone();
                tokio::spawn(async move {
                    let mut link = serve_side;
                    a.gossip(&mut link).await
                })
            };
            let (boot_out, serve_out) = tokio::join!(boot, serve);
            assert!(
                matches!(serve_out.unwrap(), Err(rumors::Error::Bookmark(_))),
                "round {round}: the serve must surface the failed donation persist",
            );
            assert!(
                !matches!(boot_out.unwrap(), Ok(Some(_))),
                "round {round}: the newcomer must not receive a party",
            );
            assert_eq!(
                a.dangerously_alias_party()
                    .expect("a live peer holds its party"),
                party_before,
                "round {round}: the donor's identity must be whole again",
            );
        }

        // One clean serve, then full convergence: nothing compounded.
        let d = boot_from(&a, GatedBookmark::new(DurableStore::default())).await;
        gossip(&a, &b).await;
        gossip(&a, &d).await;
        gossip(&b, &d).await;
        let reference = a.snapshot().hash();
        assert_eq!(
            reference,
            b.snapshot().hash(),
            "B diverged after the aborts"
        );
        assert_eq!(
            reference,
            d.snapshot().hash(),
            "D diverged after the aborts"
        );
        let pa = a.dangerously_alias_party().expect("A live");
        let pd = d.dangerously_alias_party().expect("D live");
        assert!(
            pa.is_disjoint(&pd),
            "the clean donation must be disjoint from the donor",
        );
    });
}
