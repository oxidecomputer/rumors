//! A session must checkpoint its snapshot before transmitting local events.
//!
//! The durable record must cover the snapshot's version in the sender's own
//! identity region. Otherwise, a restart could reclaim that region without
//! knowing about transmitted events and reuse their versions, losing messages.
//! Donation likewise requires confirmed durable removal before transmission.
//!
//! `GatedBookmark` pauses or fails stores on either side of replacement. Tests
//! insert local writes, fail sessions, or cancel them at those boundaries, then
//! check recovery and convergence. The closed-world poller reports a stall
//! instead of leaving an in-memory session waiting indefinitely.

mod common;
#[path = "bookmark_transmit_window/retirement.rs"]
mod retirement;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use rumors::{Bookmark, Peer, Rumors, Version};
use tokio::sync::Notify;

use crate::common::flaky::{DurableStore, persisted_record};
use crate::common::wire::{LINK_BUF, block_on};

/// The message payload: a test-unique id.
type Msg = u64;

/// A full-mesh heal round cap; a correct fleet reaches a fixed point in a
/// handful of rounds, and the cap turns a convergence bug into a loud failure.
const MAX_HEAL_ROUNDS: usize = 16;

// ---- the gated bookmark ------------------------------------------------------

/// An in-memory bookmark that can pause before or after a durable write.
/// Tests wait for `entered`, inspect the paused state, then release or cancel it.
#[derive(Clone, Debug)]
struct GatedBookmark {
    /// Bytes that survive a cancelled session or a dropped peer.
    store: DurableStore,
    /// Whether the next write should pause.
    armed: Arc<AtomicBool>,
    /// Park after durability instead of before it.
    after_write: Arc<AtomicBool>,
    /// Announces that a write reached its pause.
    entered: Arc<Notify>,
    /// Allows a paused write to continue.
    release: Arc<Notify>,
    /// Fail the Nth `store` call from now (1 = the very next); 0 = disarmed.
    fail_at: Arc<AtomicUsize>,
    /// Let the scheduled failing write replace storage before reporting its error.
    fail_after_write: Arc<AtomicBool>,
}

/// Control where persistence pauses or fails.
impl GatedBookmark {
    /// Wrap a durable store with initially disarmed gates.
    fn new(store: DurableStore) -> Self {
        GatedBookmark {
            store,
            armed: Arc::new(AtomicBool::new(false)),
            after_write: Arc::new(AtomicBool::new(false)),
            entered: Arc::new(Notify::new()),
            release: Arc::new(Notify::new()),
            fail_at: Arc::new(AtomicUsize::new(0)),
            fail_after_write: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Fail the `n`th store, optionally after its replacement has taken effect.
    fn fail_at(&self, n: usize, after_write: bool) {
        self.fail_after_write.store(after_write, Ordering::SeqCst);
        self.fail_at.store(n, Ordering::SeqCst);
    }

    /// Park the next `store` call on the gate.
    fn arm(&self) {
        self.armed.store(true, Ordering::SeqCst);
    }

    /// Park after the next write becomes durable, before it returns.
    fn arm_after_write(&self) {
        self.after_write.store(true, Ordering::SeqCst);
        self.arm();
    }

    /// Announce the pause and wait for the test to release it.
    async fn park(&self) {
        self.entered.notify_one();
        self.release.notified().await;
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

/// A deliberately failed durable write.
#[derive(Debug, thiserror::Error)]
#[error("injected store fault")]
struct InjectedFault;

/// Persist bytes with the configured pause and failure schedule.
impl Bookmark for GatedBookmark {
    /// The storage failure reported by this implementation.
    type Error = InjectedFault;
    /// Owned reader over the durable bytes.
    type Reader = std::io::Cursor<Vec<u8>>;

    /// Read the current complete record, whether or not its write returned `Ok`.
    async fn load(&self) -> Result<Option<Self::Reader>, Self::Error> {
        Ok(self.store.lock().unwrap().clone().map(std::io::Cursor::new))
    }

    /// Store the bytes, pausing on the configured side of durability.
    async fn store(&self, bytes: Vec<u8>) -> Result<(), Self::Error> {
        let armed = self.armed.swap(false, Ordering::SeqCst);
        let after_write = self.after_write.swap(false, Ordering::SeqCst);
        if armed && !after_write {
            self.park().await;
        }
        let fail = match self.fail_at.load(Ordering::SeqCst) {
            0 => false,
            1 => {
                self.fail_at.store(0, Ordering::SeqCst);
                true
            }
            n => {
                self.fail_at.store(n - 1, Ordering::SeqCst);
                false
            }
        };
        if fail && !self.fail_after_write.load(Ordering::SeqCst) {
            return Err(InjectedFault);
        }
        *self.store.lock().unwrap() = Some(bytes);
        if armed && after_write {
            self.park().await;
        }
        if fail { Err(InjectedFault) } else { Ok(()) }
    }
}

// ---- schedule helpers --------------------------------------------------------

/// An attachment error returns the same live peer even if its record reached
/// storage; retrying attachment and gossip does not duplicate its identity.
#[test]
fn attachment_error_after_replacement_can_retry() {
    block_on(async {
        let bookmark = GatedBookmark::new(DurableStore::default());
        let rumors = Peer::<Msg>::seed().into_rumors();
        rumors.send(7).unwrap();
        let peer = rumors.try_into_peer().await.unwrap();
        let identity = peer.dangerously_alias_party();
        bookmark.fail_at(1, true);
        let rumors::Unbookmarked { peer, error } =
            peer.bookmark(bookmark.clone()).await.unwrap_err();
        assert!(matches!(error, rumors::BookmarkIo::Io(_)));
        assert_eq!(peer.dangerously_alias_party(), identity);
        let record = persisted_record(&bookmark.store);
        assert_eq!(record[&peer.network()][0].party(), &identity);

        let live = peer.bookmark(bookmark).await.unwrap().into_rumors();
        let joined = boot_from(&live, GatedBookmark::new(DurableStore::default())).await;
        gossip(&live, &joined).await;
        assert_eq!(live.snapshot().hash(), joined.snapshot().hash());
        assert!(
            live.dangerously_alias_party()
                .is_disjoint(&joined.dangerously_alias_party())
        );
    });
}

/// Joining drops its wire deadline before attaching the local bookmark, so a
/// slow attachment cannot turn successful synchronization into a failed join.
#[test]
fn bootstrap_attachment_is_outside_the_session_deadline() {
    use futures::channel::oneshot;
    use rumors::Joined;

    block_on(async {
        let provider = Peer::<Msg>::seed().into_rumors();
        provider.send(7).unwrap();
        let bookmark = GatedBookmark::new(DurableStore::default());
        bookmark.arm();
        let (expire, deadline) = oneshot::channel();
        let deadline = std::sync::Mutex::new(Some(deadline));
        let bootstrap = Peer::<Msg>::bootstrap()
            .session_deadline(move || {
                let deadline = deadline.lock().unwrap().take().unwrap();
                async move { deadline.await.unwrap() }
            })
            .bookmark(bookmark.clone());
        let (mut a, mut b) = rumors::link::memory();
        let joining = async {
            let mut joining = std::pin::pin!(bootstrap.join(&mut b));
            tokio::select! {
                biased;
                () = bookmark.entered() => {},
                outcome = &mut joining => panic!("joining bypassed attachment: {outcome:?}"),
            }
            assert!(
                expire.is_canceled(),
                "wire completion must release its deadline"
            );
            bookmark.release();
            joining.await
        };
        let (served, joined) = tokio::join!(provider.gossip_once(&mut a), joining);
        served.unwrap();
        let Joined::Joined { peer } = joined else {
            panic!("attachment succeeds")
        };
        assert_eq!(
            provider.snapshot().hash(),
            peer.into_rumors().snapshot().hash()
        );
        assert!(!b.session_state().poisoned());
    });
}

/// Bootstrap a fresh peer with bookmark `bm` from `server` over a clean
/// in-memory link, returning the booted peer's [`Rumors`].
async fn boot_from(
    server: &Rumors<Msg, GatedBookmark>,
    bm: GatedBookmark,
) -> Rumors<Msg, GatedBookmark> {
    let server = server.clone();
    let (boot_side, serve_side) = rumors::link::memory_with_capacity(LINK_BUF);
    let (boot_out, serve_out) = tokio::join!(
        async move {
            let mut link = boot_side;
            let peer = match Peer::<Msg>::bootstrap().join(&mut link).await {
                rumors::Joined::Joined { peer } => peer,
                _ => panic!("the server is established"),
            };
            peer.bookmark(bm).await.expect("in-memory persist")
        },
        async move {
            let mut link = serve_side;
            server.gossip_once(&mut link).await
        },
    );
    serve_out.expect("serve bootstrap");
    boot_out.into_rumors()
}

/// One clean gossip session between two peers, both sides required to succeed.
async fn gossip(a: &Rumors<Msg, GatedBookmark>, b: &Rumors<Msg, GatedBookmark>) {
    let (a, b) = (a.clone(), b.clone());
    let (side_a, side_b) = rumors::link::memory_with_capacity(LINK_BUF);
    let (out_a, out_b) = tokio::join!(
        async move {
            let mut link = side_a;
            a.gossip_once(&mut link).await
        },
        async move {
            let mut link = side_b;
            b.gossip_once(&mut link).await
        },
    );
    out_a.expect("gossip side a");
    out_b.expect("gossip side b");
}

/// The version stamped on the live leaf carrying `payload`, if present.
fn leaf_version(rumors: &Rumors<Msg, GatedBookmark>, payload: Msg) -> Option<Version> {
    rumors
        .snapshot()
        .iter()
        .find(|(_, value)| **value == payload)
        .map(|(version, _)| version.clone())
}

/// A sender holding local M1, and two peers that received only M0.
struct Scene {
    /// Sender that commits M1 after its session snapshot is fixed.
    a: Rumors<Msg, GatedBookmark>,
    /// Recipient of the gated session, which must not receive M1.
    b: Rumors<Msg, GatedBookmark>,
    /// Source for restarting A from the progress shared before the pause.
    c: Rumors<Msg, GatedBookmark>,
    /// Handle controlling A's store, separate from its replica handle.
    bm_a: GatedBookmark,
    /// A's durable bytes, retained when its live handles are dropped.
    store_a: DurableStore,
}

/// Message replicated before the checkpoint pause.
const M0: Msg = 0;
/// Message committed after the gated session fixes its snapshot.
const M1: Msg = 1;

/// Replicate M0, then send M1 while A's next session is checkpointing its snapshot.
async fn transmit_during_persist() -> Scene {
    let store_a = DurableStore::default();
    let bm_a = GatedBookmark::new(store_a.clone());
    let a = Peer::<Msg>::seed()
        .sync_window_floor()
        .bookmark(bm_a.clone())
        .await
        .expect("a pristine seed attaches its bookmark without touching storage")
        .into_rumors();
    a.send(M0).unwrap();

    // Both newcomers receive M0. Donation invalidates A's last checkpoint,
    // so its next session must store again even without a new local write.
    let b = boot_from(&a, GatedBookmark::new(DurableStore::default())).await;
    let c = boot_from(&a, GatedBookmark::new(DurableStore::default())).await;

    // The session fixes its snapshot before checkpointing. Sending M1 while
    // the store is paused puts it outside both the checkpoint and that snapshot;
    // only a later session may transmit it.
    bm_a.arm();
    let (side_a, side_b) = rumors::link::memory_with_capacity(LINK_BUF);
    let ga = {
        let a = a.clone();
        async move {
            let mut link = side_a;
            a.gossip_once(&mut link).await
        }
    };
    let gb = {
        let b = b.clone();
        async move {
            let mut link = side_b;
            b.gossip_once(&mut link).await
        }
    };
    let (out_a, out_b, ()) = tokio::join!(ga, gb, async {
        bm_a.entered().await;
        a.send(M1).unwrap();
        bm_a.release();
    });
    out_a.expect("gated gossip side a");
    out_b.expect("gated gossip side b");

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

        let party = a.dangerously_alias_party();
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
/// coordinate-reuse collision through a schedule the in-flight-window fix
/// does not touch.
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
        a.send(M0).unwrap();
        let b = boot_from(&a, GatedBookmark::new(DurableStore::default())).await;
        let _c = boot_from(&a, GatedBookmark::new(DurableStore::default())).await;

        // M1 advances the frontier, so the next update stages a token for
        // M1's version and parks in the durable write; dropping the session
        // futures there cancels the persist mid-flight.
        a.send(M1).unwrap();
        bm_a.arm();
        let (side_a, side_b) = rumors::link::memory_with_capacity(LINK_BUF);
        // Drive the session until A's persist parks, then drop both session
        // futures there (the block ends): the cancellation lands inside the
        // durable write.
        {
            let (a, b) = (a.clone(), b.clone());
            let mut session = std::pin::pin!(async move {
                let (mut side_a, mut side_b) = (side_a, side_b);
                tokio::join!(a.gossip_once(&mut side_a), b.gossip_once(&mut side_b))
            });
            tokio::select! {
                biased;
                () = bm_a.entered() => {}
                _ = &mut session => {
                    panic!("the gated session cannot complete while its persist is parked")
                }
            }
        }

        // The next session runs on a fresh link. It must persist M1's
        // frontier before transmitting M1: a suppression token surviving
        // the cancelled write would skip that persist.
        gossip(&a, &b).await;

        let party = a.dangerously_alias_party();
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

/// Restart preserves M0 on every replica; the uncheckpointed M1 stays untransmitted.
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

        // M0 is durable because other replicas already hold it. M1 was sent
        // after the session's snapshot, so losing it with A is permitted.
        let durable_version = leaf_version(&b, M0).expect("B must already hold M0");
        assert_eq!(leaf_version(&c, M0).as_ref(), Some(&durable_version));
        assert!(
            leaf_version(&b, M1).is_none(),
            "the gated session must not transmit a write committed after its snapshot",
        );

        // Crash A: every handle drops; only the durable store survives.
        drop(a);
        drop(bm_a);

        // C knows the progress in A's checkpoint. A may reclaim those rights
        // after catching up, even though the lost M1 is absent.
        let a2 = boot_from(&c, GatedBookmark::new(store_a.clone())).await;
        gossip(&a2, &c).await; // the first update reclaims

        // Exercise new writes after recovery. Their versions must not collide
        // with M0's durable version or cause it to disappear during gossip.
        for i in 0..8 {
            a2.send(100 + i).unwrap();
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

        // Agreement alone could hide a shared loss. Require the known durable
        // message to survive with its original version on every replica.
        for (label, peer) in [("A'", &a2), ("B", &b), ("C", &c)] {
            assert_eq!(
                leaf_version(peer, M0).as_ref(),
                Some(&durable_version),
                "durable message M0 was lost or changed version at {label}",
            );
        }
    });
}

/// A failed donation write prevents handoff even if removal reached storage.
/// The donor recovers its fork and can checkpoint and donate again.
#[test]
fn donation_persist_failure_aborts_before_the_wire() {
    for after_write in [false, true] {
        block_on(async {
            let store_a = DurableStore::default();
            let bm_a = GatedBookmark::new(store_a.clone());
            let a = Peer::<Msg>::seed()
                .sync_window_floor()
                .bookmark(bm_a.clone())
                .await
                .expect("a pristine seed attaches its bookmark without touching storage")
                .into_rumors();
            a.send(M0).unwrap();
            let b = boot_from(&a, GatedBookmark::new(DurableStore::default())).await;

            let party_before = a.dangerously_alias_party();
            // Settle the record: a session with B re-records the post-donation
            // identity (the serve's slice cleared the suppression token), so the
            // failing serve below mutates nothing but the donation itself.
            gossip(&a, &b).await;
            let bytes_before = store_a.lock().unwrap().clone();

            // Serve a bootstrap whose donation persist fails. The session's
            // update is suppressed (the record is current), so the donation
            // slice's write is the next store call.
            bm_a.fail_at(1, after_write);
            let (boot_side, serve_side) = rumors::link::memory_with_capacity(LINK_BUF);
            let serve = {
                let a = a.clone();
                async move {
                    let mut link = serve_side;
                    a.gossip_once(&mut link).await
                }
            };
            let (boot_out, serve_out) = tokio::join!(
                async move {
                    let mut link = boot_side;
                    Peer::<Msg>::bootstrap().join(&mut link).await
                },
                serve,
            );
            assert!(
                matches!(serve_out, Err(rumors::Error::Bookmark(_))),
                "the serve must surface the failed donation persist",
            );
            assert!(
                matches!(boot_out, rumors::Joined::Failed { .. }),
                "the newcomer must not receive a party the donor could not persist away",
            );

            // The fork returns even if its removal from storage took effect.
            assert_eq!(
                a.dangerously_alias_party(),
                party_before,
                "the speculative fork must re-join the donor's party on abort",
            );
            assert_eq!(
                *store_a.lock().unwrap() != bytes_before,
                after_write,
                "exercise both complete storage outcomes",
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
        a.send(M0).unwrap();
        let b = boot_from(&a, GatedBookmark::new(DurableStore::default())).await;
        let party_before = a.dangerously_alias_party();

        for round in 0..3 {
            // A failed write resets the in-memory record, so the next
            // session's update re-records (one store call) before the
            // donation slice's write (the second): fail the second.
            bm_a.fail_at(2, false);
            let (boot_side, serve_side) = rumors::link::memory_with_capacity(LINK_BUF);
            let serve = {
                let a = a.clone();
                async move {
                    let mut link = serve_side;
                    a.gossip_once(&mut link).await
                }
            };
            let (boot_out, serve_out) = tokio::join!(
                async move {
                    let mut link = boot_side;
                    Peer::<Msg>::bootstrap().join(&mut link).await
                },
                serve,
            );
            assert!(
                matches!(serve_out, Err(rumors::Error::Bookmark(_))),
                "round {round}: the serve must surface the failed donation persist",
            );
            assert!(
                matches!(boot_out, rumors::Joined::Failed { .. }),
                "round {round}: the newcomer must not receive a party",
            );
            assert_eq!(
                a.dangerously_alias_party(),
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
        let pa = a.dangerously_alias_party();
        let pd = d.dangerously_alias_party();
        assert!(
            pa.is_disjoint(&pd),
            "the clean donation must be disjoint from the donor",
        );
    });
}
