//! Check when bookmark reads and checkpoints are required.
//!
//! With healthy storage, a peer loads once, before its first store. Sessions
//! checkpoint local sends and effective redactions; repeated redactions and
//! learning remote changes need no store. Identity transfers also persist.
//!
//! Attachment and donation leave a checkpoint due even though they store a
//! record: attachment defers reclamation to the first session, and donation
//! invalidates the checkpoint for the previous party. A failed store can force
//! a reload and is tested separately.
//!
//! `Probe` records I/O calls. `Model` predicts the schedule from operations,
//! without reproducing the cache's version arithmetic. The property compares
//! the two throughout a generated peer lifetime.

mod common;
#[path = "bookmark_when/reclamation.rs"]
mod reclamation;

use std::convert::Infallible;
use std::sync::{Arc, Mutex};

use proptest::prelude::*;
use rumors::{Bookmark, Peer, Retire, Rumors, Version};

use crate::common::wire::{LINK_BUF, block_on};

/// One observed bookmark I/O, in call order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Io {
    /// Open the current stored record.
    Read,
    /// Replace the record with a complete checkpoint.
    Write,
}

/// Reliable storage and an I/O history that survive the peer being tested.
#[derive(Clone, Default)]
struct Probe {
    /// The complete stored record, shared across simulated restarts.
    store: Arc<Mutex<Option<Vec<u8>>>>,
    /// Storage calls in the order they occurred.
    log: Arc<Mutex<Vec<Io>>>,
}

/// Describe the probe without exposing its shared storage and log.
impl std::fmt::Debug for Probe {
    /// Identify the probe in assertion failures.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Probe").finish_non_exhaustive()
    }
}

/// Record I/O calls while atomically replacing the complete byte record.
impl Bookmark for Probe {
    /// Replacing an owned in-memory buffer cannot fail.
    type Error = Infallible;
    /// A snapshot of the stored bytes.
    type Reader = std::io::Cursor<Vec<u8>>;

    /// Log a read and return the current record.
    async fn load(&self) -> Result<Option<Self::Reader>, Self::Error> {
        self.log.lock().unwrap().push(Io::Read);
        Ok(self.store.lock().unwrap().clone().map(std::io::Cursor::new))
    }

    /// Log a write and replace the record with the supplied buffer.
    async fn store(&self, bytes: Vec<u8>) -> Result<(), Self::Error> {
        self.log.lock().unwrap().push(Io::Write);
        *self.store.lock().unwrap() = Some(bytes);
        Ok(())
    }
}

/// Reconcile the subject with a helper without transferring identity.
async fn plain_gossip(subject: &Rumors<u64, Probe>, helper: &Rumors<u64>) {
    let (mut s_link, mut h_link) = rumors::link::memory_with_capacity(LINK_BUF);
    let (s, h) = tokio::join!(
        subject.gossip_once(&mut s_link),
        helper.gossip_once(&mut h_link),
    );
    s.expect("subject plain gossip");
    h.expect("helper plain gossip");
}

/// Join a new helper through the subject, donating a fork of its identity.
async fn serve_bootstrap(subject: &Rumors<u64, Probe>) -> Rumors<u64> {
    let (mut s_link, mut n_link) = rumors::link::memory_with_capacity(LINK_BUF);
    let (s, n) = tokio::join!(
        subject.gossip_once(&mut s_link),
        Peer::<u64>::bootstrap().join(&mut n_link),
    );
    s.expect("subject serve bootstrap");
    (match n {
        rumors::Joined::Joined { peer } => peer,
        _ => panic!("subject served the bootstrap"),
    })
    .sync_window_floor()
    .into_rumors()
}

/// Join through `origin`, returning a peer ready for bookmark attachment.
async fn bootstrap_fork_peer(origin: &Rumors<u64>) -> Peer<u64> {
    let (mut o_link, mut n_link) = rumors::link::memory_with_capacity(LINK_BUF);
    let (o, n) = tokio::join!(
        origin.gossip_once(&mut o_link),
        Peer::<u64>::bootstrap().join(&mut n_link),
    );
    o.expect("origin serves the bootstrap");
    (match n {
        rumors::Joined::Joined { peer } => peer,
        _ => panic!("origin served the bootstrap"),
    })
    .sync_window_floor()
}

/// Retire a helper into the subject, transferring its identity.
async fn absorb_retire(subject: &Rumors<u64, Probe>, retiree: Rumors<u64>) {
    let retiree = retiree
        .try_into_peer()
        .await
        .expect("the helper is the sole handle to its set");
    let (mut s_link, mut r_link) = rumors::link::memory_with_capacity(LINK_BUF);
    let (s, outcome) = tokio::join!(
        subject.gossip_once(&mut s_link),
        retiree.retire(&mut r_link),
    );
    s.expect("subject absorbs the retiree");
    match outcome {
        Retire::Retired => {}
        other => panic!("a clean retirement must succeed, got {other:?}"),
    }
}

/// Consume the subject by retiring it into a helper.
async fn retire_subject(subject: Rumors<u64, Probe>, absorber: &Rumors<u64>) {
    let subject = subject
        .try_into_peer()
        .await
        .expect("the subject is the sole handle to its set");
    let (mut s_link, mut a_link) = rumors::link::memory_with_capacity(LINK_BUF);
    let (outcome, a) = tokio::join!(
        subject.retire(&mut s_link),
        absorber.gossip_once(&mut a_link),
    );
    a.expect("absorber gossip");
    match outcome {
        Retire::Retired => {}
        other => panic!("a clean retirement must succeed, got {other:?}"),
    }
}

/// The number of storage calls made by an operation.
#[derive(Debug, Default, PartialEq, Eq)]
struct Delta {
    /// Calls that opened the stored record.
    reads: usize,
    /// Calls that replaced the stored record.
    writes: usize,
}

/// Summarize observed storage calls for comparison with the model.
impl Delta {
    /// Count each kind of I/O in a history or a slice of it.
    fn count(history: &[Io]) -> Self {
        Self {
            reads: history.iter().filter(|io| **io == Io::Read).count(),
            writes: history.iter().filter(|io| **io == Io::Write).count(),
        }
    }
}

/// Predict storage activity from local operations and identity transfers.
struct Model {
    /// Whether the record has been loaded during this peer's lifetime.
    loaded: bool,
    /// The next session must checkpoint local progress or changed ownership.
    pending: bool,
}

/// Apply the checkpoint policy without inspecting the peer's clock or cache.
impl Model {
    /// Predict the I/O before ordinary gossip; remote learning adds none.
    fn plain_gossip(&mut self) -> Delta {
        if !self.pending {
            return Delta::default();
        }
        let reads = usize::from(!self.loaded);
        self.loaded = true;
        self.pending = false;
        Delta { reads, writes: 1 }
    }

    /// Checkpoint if needed, then store the removal of the donated identity.
    ///
    /// The donation invalidates the previous party's checkpoint. The next
    /// session records the retained party once, even with no intervening send.
    fn serve_bootstrap(&mut self) -> Delta {
        let reads = usize::from(!self.loaded);
        let writes = 1 + usize::from(self.pending);
        self.loaded = true;
        self.pending = true;
        Delta { reads, writes }
    }

    /// Checkpoint if needed, then persist the enlarged identity after absorption.
    fn absorb_retire(&mut self) -> Delta {
        let reads = usize::from(!self.loaded);
        let writes = 1 + usize::from(self.pending);
        self.loaded = true;
        self.pending = false;
        Delta { reads, writes }
    }

    /// Predict the final checkpoint and removal of the subject's whole identity.
    fn retire_subject(&self) -> Delta {
        Delta {
            reads: usize::from(!self.loaded),
            writes: 1 + usize::from(self.pending),
        }
    }
}

/// How the subject joins the network, determining its attachment-time I/O.
#[derive(Debug, Clone, Copy)]
enum Origin {
    /// A pristine seed defers storage until its first session.
    Seed,
    /// A bootstrapped peer immediately reads and persists its identity.
    Bootstrap,
}

/// One operation in a peer's lifetime; unavailable helpers or messages are skipped.
#[derive(Debug, Clone)]
enum Op {
    /// Insert a fresh local message.
    Send,
    /// Redact a selection that may include held, absent, or duplicate versions.
    Redact(Vec<usize>),
    /// Repeat the latest local redaction, which must do nothing.
    RedactAgain,
    /// Insert a fresh message at a helper.
    HelperSend(usize),
    /// Redact a held message at a helper.
    HelperRedact(usize),
    /// Reconcile with a helper.
    Gossip(usize),
    /// Serve a bootstrap, gaining a helper.
    Serve,
    /// Absorb a helper's retirement.
    Absorb(usize),
}

/// Mix local edits, remote edits, and lifecycle operations in generated scripts.
fn op_strategy() -> impl Strategy<Value = Op> {
    prop_oneof![
        3 => Just(Op::Send),
        2 => prop::collection::vec(any::<usize>(), 0..8).prop_map(Op::Redact),
        2 => Just(Op::RedactAgain),
        3 => any::<usize>().prop_map(Op::HelperSend),
        2 => any::<usize>().prop_map(Op::HelperRedact),
        4 => any::<usize>().prop_map(Op::Gossip),
        2 => Just(Op::Serve),
        1 => any::<usize>().prop_map(Op::Absorb),
    ]
}

/// A bookmarked subject, its storage history, and helpers in the same network.
struct World {
    /// The peer whose I/O is checked.
    subject: Rumors<u64, Probe>,
    /// A history retained even when retirement consumes the subject.
    log: Arc<Mutex<Vec<Io>>>,
    /// Expected storage state after the operations already applied.
    model: Model,
    /// Live counterparties available for gossip or retirement.
    helpers: Vec<Rumors<u64>>,
    /// Fresh payloads shared by local and helper sends.
    next_msg: u64,
    /// An absent version for exercising repeated redactions.
    last_redacted: Option<Version>,
}

/// Construct a lifetime and drive its operations through the public API.
impl World {
    /// Attach storage, checking the initial I/O for the chosen origin.
    async fn new(origin: Origin) -> Self {
        let probe = Probe::default();
        let log = Arc::clone(&probe.log);
        let (peer, helpers, loaded) = match origin {
            Origin::Seed => (Peer::<u64>::seed().sync_window_floor(), Vec::new(), false),
            Origin::Bootstrap => {
                let origin = Peer::<u64>::seed().sync_window_floor().into_rumors();
                (bootstrap_fork_peer(&origin).await, vec![origin], true)
            }
        };
        let subject = peer.bookmark(probe).await.expect("attach healthy storage");
        assert_eq!(
            *log.lock().unwrap(),
            if loaded {
                vec![Io::Read, Io::Write]
            } else {
                Vec::new()
            },
            "attachment I/O for {origin:?}",
        );
        Self {
            subject: subject.into_rumors(),
            log,
            // A seed has no record yet. Attaching to a fork records ownership
            // without reclaiming, leaving that to its first session. Both
            // origins therefore owe a checkpoint.
            model: Model {
                loaded,
                pending: true,
            },
            helpers,
            next_msg: 0,
            last_redacted: None,
        }
    }

    /// Copy the observed I/O history without holding a lock across a session.
    fn history(&self) -> Vec<Io> {
        self.log.lock().unwrap().clone()
    }

    /// Mark the start of an operation in the shared history.
    fn cursor(&self) -> usize {
        self.log.lock().unwrap().len()
    }

    /// Apply one operation and compare its I/O with the model's prediction.
    async fn check(&mut self, op: Op) {
        let cursor = self.cursor();
        let expected = self.apply(&op).await;
        let actual = Delta::count(&self.history()[cursor..]);
        assert_eq!(actual, expected, "unexpected bookmark I/O for {op:?}");
    }

    /// Apply an operation and return its predicted I/O, including zero for edits.
    async fn apply(&mut self, op: &Op) -> Delta {
        match op {
            Op::Send => {
                self.subject.send(self.next_msg).unwrap();
                self.next_msg += 1;
                self.model.pending = true;
            }
            Op::Redact(indices) => {
                // The public snapshot tells the model which targets are held.
                // An empty batch or a batch of absent versions must do nothing;
                // any effective redaction makes one checkpoint due.
                let held: Vec<_> = self.subject.snapshot().versions().cloned().collect();
                let mut candidates = held.clone();
                candidates.extend(self.last_redacted.iter().cloned());
                // Helpers may know versions the subject has never received.
                // Those are no-ops too, even while a helper still holds them.
                for helper in &self.helpers {
                    candidates.extend(helper.snapshot().versions().cloned());
                }
                let mut targets = Vec::new();
                if !candidates.is_empty() {
                    for i in indices {
                        targets.push(candidates[i % candidates.len()].clone());
                    }
                }
                if let Some(removed) = targets.iter().find(|version| held.contains(version)) {
                    self.last_redacted = Some(removed.clone());
                    self.model.pending = true;
                }
                self.subject.redact_all(&targets);
            }
            Op::RedactAgain => {
                if let Some(version) = &self.last_redacted {
                    self.subject.redact(version);
                }
            }
            Op::HelperSend(i) => {
                if !self.helpers.is_empty() {
                    self.helpers[i % self.helpers.len()]
                        .send(self.next_msg)
                        .unwrap();
                    self.next_msg += 1;
                }
            }
            Op::HelperRedact(i) => {
                if !self.helpers.is_empty() {
                    let helper = &self.helpers[i % self.helpers.len()];
                    let snapshot = helper.snapshot();
                    if !snapshot.is_empty() {
                        let (version, _) = snapshot.iter().nth(i % snapshot.len()).unwrap();
                        helper.redact(version);
                    }
                }
            }
            Op::Gossip(i) => {
                if !self.helpers.is_empty() {
                    plain_gossip(&self.subject, &self.helpers[i % self.helpers.len()]).await;
                    return self.model.plain_gossip();
                }
            }
            Op::Serve => {
                self.helpers.push(serve_bootstrap(&self.subject).await);
                return self.model.serve_bootstrap();
            }
            Op::Absorb(i) => {
                if !self.helpers.is_empty() {
                    let retiree = self.helpers.remove(i % self.helpers.len());
                    absorb_retire(&self.subject, retiree).await;
                    return self.model.absorb_retire();
                }
            }
        }
        Delta::default()
    }
}

/// A seed's first session loads before it writes, with no I/O at attachment.
#[test]
fn read_is_deferred_to_first_use() {
    block_on(async {
        let world = World::new(Origin::Seed).await;
        let _helper = serve_bootstrap(&world.subject).await;
        let history = world.history();
        assert_eq!(
            history.first(),
            Some(&Io::Read),
            "load before the first store"
        );
        assert_eq!(Delta::count(&history).reads, 1, "load exactly once");
    });
}

/// A local send requires one checkpoint; learning a helper's message requires none.
#[test]
fn incorporating_remote_content_writes_nothing() {
    block_on(async {
        let world = World::new(Origin::Seed).await;
        let helper = serve_bootstrap(&world.subject).await;
        // Finish the checkpoint due after donation, so the send is the only
        // reason for the next store.
        plain_gossip(&world.subject, &helper).await;

        world.subject.send(1).unwrap();
        let before = world.cursor();
        plain_gossip(&world.subject, &helper).await;
        assert_eq!(&world.history()[before..], &[Io::Write]);

        helper.send(2).unwrap();
        let before = world.cursor();
        plain_gossip(&world.subject, &helper).await;
        assert!(world.history()[before..].is_empty());
        assert!(world.subject.snapshot().iter().any(|(_, m)| *m == 2));
    });
}

/// Many local and remote sends still require only one load during a peer's life.
#[test]
fn read_happens_exactly_once_across_a_long_life() {
    block_on(async {
        let world = World::new(Origin::Seed).await;
        let helper = serve_bootstrap(&world.subject).await;
        for round in 0..16u64 {
            world.subject.send(round).unwrap();
            plain_gossip(&world.subject, &helper).await;
            helper.send(1_000 + round).unwrap();
            plain_gossip(&world.subject, &helper).await;
        }
        assert_eq!(Delta::count(&world.history()).reads, 1);
    });
}

/// Attachment persists a fork; its first session checkpoints once without reloading.
#[test]
fn attaching_to_a_fork_eagerly_persists_then_never_re_reads() {
    block_on(async {
        let world = World::new(Origin::Bootstrap).await;
        let before = world.cursor();
        plain_gossip(&world.subject, &world.helpers[0]).await;
        assert_eq!(&world.history()[before..], &[Io::Write]);
    });
}

proptest! {
    /// Donation checkpoints pending sends; later sessions record the retained party only once.
    #[test]
    fn donation_leaves_one_checkpoint_due(local_sends in 0u64..8, later_sessions in 1usize..8) {
        block_on(async {
            let world = World::new(Origin::Bootstrap).await;
            plain_gossip(&world.subject, &world.helpers[0]).await;
            for message in 0..local_sends {
                world.subject.send(message).unwrap();
            }

            let before = world.cursor();
            let helper = serve_bootstrap(&world.subject).await;
            assert_eq!(
                Delta::count(&world.history()[before..]),
                Delta { reads: 0, writes: 1 + usize::from(local_sends > 0) },
            );

            let before = world.cursor();
            plain_gossip(&world.subject, &helper).await;
            assert_eq!(&world.history()[before..], &[Io::Write]);

            let before = world.cursor();
            for _ in 0..later_sessions {
                plain_gossip(&world.subject, &helper).await;
            }
            assert!(world.history()[before..].is_empty());
        });
    }

    /// Repeating completed redactions neither advances local progress nor requires a store.
    #[test]
    fn repeated_redactions_need_no_checkpoint(messages in 1u64..12, repetitions in 1usize..8) {
        block_on(async {
            let world = World::new(Origin::Bootstrap).await;
            let helper = &world.helpers[0];
            for message in 0..messages {
                world.subject.send(message).unwrap();
            }
            let versions: Vec<_> = world.subject.snapshot().versions()
                .cloned()
                .collect();
            world.subject.redact_all(&versions);
            plain_gossip(&world.subject, helper).await;
            let checkpointed = world.subject.snapshot();
            assert!(checkpointed.is_empty());

            let before = world.cursor();
            for _ in 0..repetitions {
                world.subject.redact_all(&versions);
                assert_eq!(world.subject.snapshot().latest(), checkpointed.latest());
                plain_gossip(&world.subject, helper).await;
            }
            assert!(world.history()[before..].is_empty(), "repeated redactions did I/O");
        });
    }

    /// Remote redactions need no store, including when only their frontier reaches the subject.
    #[test]
    fn remote_redactions_need_no_checkpoint(
        retained in 0u64..8,
        removed in 1u64..8,
        share_before_redaction: bool,
    ) {
        block_on(async {
            let world = World::new(Origin::Bootstrap).await;
            let helper = &world.helpers[0];
            for message in 0..retained {
                helper.send(message).unwrap();
            }
            plain_gossip(&world.subject, helper).await;

            for message in retained..retained + removed {
                helper.send(message).unwrap();
            }
            let versions: Vec<_> = helper.snapshot().iter()
                .filter(|(_, message)| **message >= retained)
                .map(|(version, _)| version.clone())
                .collect();
            if share_before_redaction {
                plain_gossip(&world.subject, helper).await;
            }
            let prior = world.subject.snapshot();
            helper.redact_all(&versions);

            let before = world.cursor();
            plain_gossip(&world.subject, helper).await;
            let learned = world.subject.snapshot();
            assert_ne!(learned.latest(), prior.latest(), "remote progress reached the subject");
            assert_eq!(learned.latest(), helper.snapshot().latest());
            let mut messages: Vec<_> = learned.iter().map(|(_, message)| *message).collect();
            messages.sort_unstable();
            assert_eq!(messages, (0..retained).collect::<Vec<_>>());
            if !share_before_redaction {
                // Sending and redacting before any gossip leaves the content
                // unchanged. Only the remote frontier advances at the subject.
                assert_eq!(learned.hash(), prior.hash());
            }

            // The first session checks its checkpoint before learning the
            // redactions. A second must still skip it with that progress known.
            plain_gossip(&world.subject, helper).await;
            assert!(world.history()[before..].is_empty(), "remote redactions did I/O");
        });
    }

    /// Every operation drives the predicted I/O; the sole load precedes every store.
    #[test]
    fn bookmark_io_schedule_matches_the_model(
        origin in prop_oneof![Just(Origin::Seed), Just(Origin::Bootstrap)],
        script in prop::collection::vec(op_strategy(), 0..40),
        retire_at_end: bool,
    ) {
        block_on(async {
            let mut world = World::new(origin).await;
            for op in script {
                world.check(op).await;
            }

            // A script may finish on a local edit or after losing every helper.
            // Always drive a final session: otherwise a wrong pending-checkpoint
            // decision at the end of the script could go unobserved.
            if world.helpers.is_empty() {
                world.check(Op::Serve).await;
            }
            // Retain the history separately because retirement consumes the peer.
            let log = Arc::clone(&world.log);
            if retire_at_end {
                let cursor = world.cursor();
                let expected = world.model.retire_subject();
                let absorber = world.helpers.remove(0);
                retire_subject(world.subject, &absorber).await;
                assert_eq!(Delta::count(&log.lock().unwrap()[cursor..]), expected);
            } else {
                world.check(Op::Gossip(0)).await;
                // Once caught up, another session must not write again.
                world.check(Op::Gossip(0)).await;
            }

            let history = log.lock().unwrap();
            assert!(Delta::count(&history).reads <= 1, "repeated load: {history:?}");
            if history.contains(&Io::Write) {
                assert_eq!(history.first(), Some(&Io::Read), "store without a load: {history:?}");
            }
        });
    }
}
