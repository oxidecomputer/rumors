//! *When* the identity bookmark is read and written, instrumented at the seam.
//!
//! The crate's contract for a [`Bookmark`](rumors::Bookmark) makes two timing
//! promises that no other test pins directly — the prose in
//! [`bookmark`](rumors::Bookmark) and [`Peer::bookmark`](rumors::Peer::bookmark)
//! states them, the disruption suites (`bookmark_causality.rs`) rely on them,
//! but nothing asserts the *call schedule* itself:
//!
//! 1. **Read once.** The durable record is read exactly once over a peer's whole
//!    lifetime — at attach for a peer born by bootstrap, lazily at first use for
//!    a fresh seed, and in either case before the first write — and never again,
//!    however many sessions, donations, or absorptions follow.
//! 2. **Write on local work, never on hearsay.** A session persists the record
//!    *before* it transmits, but only when the peer has unpersisted *local*
//!    identity work to checkpoint: it has never persisted, or has emitted a local
//!    change (a [`send`](rumors::Rumors::send) or
//!    [`redact`](rumors::Rumors::redact)) or moved a party (donated a fork,
//!    absorbed a retiree) since its last persist. Merely *incorporating* content
//!    learned over gossip — whatever it may be — advances only other parties'
//!    regions, never the peer's own, so it triggers no write at all.
//!
//! The distinction in (2) is the whole point: a local change ticks the peer's
//! *own* identity region, and a checkpoint of that region must reach storage
//! before any peer can causally depend on it; remote content ticks regions the
//! peer does not own, which its bookmark already need not vouch for. The
//! suppression token (`Bookmarked::is_current`) draws exactly this line — own
//! region advanced, or party changed, versus not — and these tests check that
//! the *observable I/O* falls where that line predicts.
//!
//! # The instrument
//!
//! [`Probe`] is a faithful in-memory store — it round-trips the record through
//! Borsh on every call, exactly as a real disk-backed store would, since a
//! [`Clock`] is `!Clone` (see [`common::flaky`] for the same technique) — that
//! additionally appends a [`Io::Read`] or [`Io::Write`] marker to a shared log
//! on each call. It never injects faults: a clean run is the precondition for
//! reasoning about *exact* call counts, since a failed write would reset the
//! cache and re-arm the next write.
//!
//! # The model
//!
//! A [`Model`] mirror tracks two bits — whether the record has been *loaded*
//! (read), and whether a checkpoint is *pending* — and, from the *operation
//! semantics alone*, predicts the exact `(reads, writes)` each operation must
//! drive. The `pending` bit is set purely by *what the operation is*: a send,
//! or a redact that removed a held key, dirties the peer; incorporating content
//! over gossip does not; a donation or absorption moves the party. It is never
//! computed from the version arithmetic the crate's suppression
//! (`Bookmarked::is_current`) uses — that would only check the implementation
//! against itself. The whole point is to show the two agree: the model says
//! "persist iff the *operations* left local work owed," the suppression says
//! "persist iff the *own-region version* advanced," and the proptest asserts the
//! observable I/O matches the former at every step, over an arbitrary lifetime.
//! A peer that read twice, wrote on incorporated content, or skipped a
//! checkpoint before donating would diverge immediately.

mod common;

use std::collections::BTreeMap;
use std::convert::Infallible;
use std::sync::{Arc, Mutex};

use before::Clock;
use proptest::prelude::*;
use rumors::{Bookmark, BookmarkError, Key, Network, Peer, Retire, Rumors};
use tokio::io::{duplex, split};

use crate::common::wire::block_on;

/// In-memory duplex capacity, comfortably larger than any session here ships.
const DUPLEX_BUF: usize = 8 * 1024;

// ---- the instrument --------------------------------------------------------

/// One observed bookmark I/O, in call order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Io {
    Read,
    Write,
}

/// A faithful in-memory [`Bookmark`] that logs the *timing* of every read and
/// write.
///
/// `store` is the durable "disk"; `log` records each call. Both are behind
/// [`Arc`]s so the test inspects them while the peer holds the `Probe`. Reads
/// and writes round-trip the record through Borsh — the only way to duplicate
/// `!Clone` [`Clock`]s, and the same byte trip a real store makes — and never
/// fail, so call counts are exact.
struct Probe {
    store: Arc<Mutex<BTreeMap<Network, Vec<Clock>>>>,
    log: Arc<Mutex<Vec<Io>>>,
}

impl std::fmt::Debug for Probe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Probe").finish_non_exhaustive()
    }
}

/// Deep-copy a record through Borsh: the only way to clone `!Clone` [`Clock`]s,
/// and the serialize/deserialize round trip a real disk-backed store performs.
fn round_trip(record: &BTreeMap<Network, Vec<Clock>>) -> BTreeMap<Network, Vec<Clock>> {
    let bytes = borsh::to_vec(record).expect("encode bookmark record");
    borsh::from_slice(&bytes).expect("decode bookmark record")
}

impl BookmarkError for Probe {
    type Error = Infallible;
}

impl Bookmark for Probe {
    async fn read(&self) -> Result<BTreeMap<Network, Vec<Clock>>, Self::Error> {
        self.log.lock().unwrap().push(Io::Read);
        Ok(round_trip(&self.store.lock().unwrap()))
    }

    async fn write(&self, bookmarks: &BTreeMap<Network, Vec<Clock>>) -> Result<(), Self::Error> {
        self.log.lock().unwrap().push(Io::Write);
        *self.store.lock().unwrap() = round_trip(bookmarks);
        Ok(())
    }
}

/// A live probe attached to the subject, with handles onto its shared store and
/// log so the test can read out the I/O history without disturbing the peer.
struct Instrument {
    subject: Rumors<u64, Probe>,
    log: Arc<Mutex<Vec<Io>>>,
}

impl Instrument {
    /// A fresh, *pristine* seed with a `Probe` attached. A pristine seed has no
    /// identity worth recording, so the attach itself drives no I/O — the read
    /// is deferred to the first session that needs it.
    fn pristine_seed() -> Self {
        block_on(async {
            let log = Arc::new(Mutex::new(Vec::new()));
            let store = Arc::new(Mutex::new(BTreeMap::new()));
            let probe = Probe {
                store,
                log: Arc::clone(&log),
            };
            let subject = Peer::<u64>::seed()
                .bookmark(probe)
                .await
                .expect("a pristine seed attaches without touching storage");
            assert!(
                log.lock().unwrap().is_empty(),
                "attaching a bookmark to a pristine seed must drive no I/O",
            );
            Instrument {
                subject: subject.into_rumors(),
                log,
            }
        })
    }

    /// The full I/O history observed so far.
    fn history(&self) -> Vec<Io> {
        self.log.lock().unwrap().clone()
    }

    /// `(reads, writes)` recorded since the log was `from` entries long.
    fn counts_since(&self, from: usize) -> (usize, usize) {
        let log = self.log.lock().unwrap();
        let slice = &log[from..];
        (
            slice.iter().filter(|e| **e == Io::Read).count(),
            slice.iter().filter(|e| **e == Io::Write).count(),
        )
    }

    /// Current log length, the cursor into [`counts_since`](Self::counts_since).
    fn cursor(&self) -> usize {
        self.log.lock().unwrap().len()
    }
}

// ---- session drivers -------------------------------------------------------
//
// Each runs one in-memory session between the bookmarked subject and a helper
// over a clean duplex, both ends making concurrent progress via `join!` on the
// current-thread runtime.

/// Plain gossip: the subject reconciles content with `helper`. No party moves.
async fn plain_gossip(subject: &Rumors<u64, Probe>, helper: &Rumors<u64>) {
    let (s_side, h_side) = duplex(DUPLEX_BUF);
    let (mut s_r, mut s_w) = split(s_side);
    let (mut h_r, mut h_w) = split(h_side);
    let (s, h) = tokio::join!(
        subject.gossip(&mut s_r, &mut s_w),
        helper.gossip(&mut h_r, &mut h_w),
    );
    s.expect("subject plain gossip");
    h.expect("helper plain gossip");
}

/// Serve a bootstrap: the subject donates a fresh fork of its identity to a
/// newcomer, returning it as a new helper in the same universe.
async fn serve_bootstrap(subject: &Rumors<u64, Probe>) -> Rumors<u64> {
    let (s_side, n_side) = duplex(DUPLEX_BUF);
    let (mut s_r, mut s_w) = split(s_side);
    let (mut n_r, mut n_w) = split(n_side);
    let (s, n) = tokio::join!(
        subject.gossip(&mut s_r, &mut s_w),
        Peer::<u64>::bootstrap(&mut n_r, &mut n_w),
    );
    s.expect("subject serve bootstrap");
    n.expect("bootstrap handshake")
        .expect("subject served the bootstrap")
        .into_rumors()
}

/// Serve a bootstrap from `origin`, returning the newcomer as a still-unbookmarked
/// [`Peer`] ready to have a `Probe` attached — the way a real process is born
/// into an existing universe before it adopts its durable identity store.
async fn bootstrap_fork_peer(origin: &Rumors<u64>) -> Peer<u64> {
    let (o_side, n_side) = duplex(DUPLEX_BUF);
    let (mut o_r, mut o_w) = split(o_side);
    let (mut n_r, mut n_w) = split(n_side);
    let (o, n) = tokio::join!(
        origin.gossip(&mut o_r, &mut o_w),
        Peer::<u64>::bootstrap(&mut n_r, &mut n_w),
    );
    o.expect("origin serves the bootstrap");
    n.expect("bootstrap handshake")
        .expect("origin served the bootstrap")
}

/// Absorb a retiree: `retiree` retires its whole identity into the subject,
/// which runs ordinary gossip and absorbs the donated party.
async fn absorb_retire(subject: &Rumors<u64, Probe>, retiree: Rumors<u64>) {
    let retiree = retiree
        .try_into_peer()
        .await
        .expect("the helper is the sole handle to its set");
    let (s_side, r_side) = duplex(DUPLEX_BUF);
    let (mut s_r, mut s_w) = split(s_side);
    let (mut r_r, mut r_w) = split(r_side);
    let (s, outcome) = tokio::join!(
        subject.gossip(&mut s_r, &mut s_w),
        retiree.retire(&mut r_r, &mut r_w),
    );
    s.expect("subject absorbs the retiree");
    match outcome {
        Retire::Retired => {}
        other => panic!("a clean retirement must succeed, got {other:?}"),
    }
}

/// The subject retires into `absorber`, donating its whole party. Terminal: the
/// subject is consumed.
async fn retire_subject(subject: Rumors<u64, Probe>, absorber: &Rumors<u64>) {
    let subject = subject
        .try_into_peer()
        .await
        .expect("the subject is the sole handle to its set");
    let (s_side, a_side) = duplex(DUPLEX_BUF);
    let (mut s_r, mut s_w) = split(s_side);
    let (mut a_r, mut a_w) = split(a_side);
    let (outcome, a) = tokio::join!(
        subject.retire(&mut s_r, &mut s_w),
        absorber.gossip(&mut a_r, &mut a_w),
    );
    a.expect("absorber gossip");
    match outcome {
        Retire::Retired => {}
        other => panic!("a clean retirement must succeed, got {other:?}"),
    }
}

// ---- the model -------------------------------------------------------------

/// The exact I/O an operation must drive.
#[derive(Debug, PartialEq, Eq)]
struct Delta {
    reads: usize,
    writes: usize,
}

/// An independent mirror of the bookmark's persistence state, tracking only what
/// the contract makes observable.
///
/// `loaded` is whether the record has been read (it is read lazily, on first
/// use). `pending` is whether a pre-session checkpoint would write: true at
/// birth (nothing persisted yet) and after any *local* identity work, false once
/// a session has captured that work. The key invariant the model encodes is
/// `!pending ⟹ loaded`: a checkpoint is cleared only by a write, and a write is
/// always preceded by the lazy load — so the read can never lag the first write.
struct Model {
    loaded: bool,
    pending: bool,
}

impl Model {
    /// A pristine seed: never loaded, and a checkpoint pending — its first
    /// session will record its initial identity.
    fn pristine_seed() -> Self {
        Model {
            loaded: false,
            pending: true,
        }
    }

    /// A peer born by bootstrap, with its `Probe` already attached. A fork is
    /// not pristine, so the attach has *eagerly* persisted it: already loaded
    /// (one read), already written once. But the attach-time record deliberately
    /// does not stage the suppression token, so a checkpoint is still pending —
    /// the first session re-records, folding in any region the fork already
    /// dominates. The lifetime's single read has therefore *already happened*, at
    /// attach; no session ever reads again.
    fn bootstrap_fork() -> Self {
        Model {
            loaded: true,
            pending: true,
        }
    }

    /// The lazy read this op drives, if it is the first to load: one read if a
    /// load is owed, none once loaded.
    fn read_on_first_use(&self) -> usize {
        usize::from(!self.loaded)
    }

    /// A local change — a send, or a redact that removed a held key — that ticks
    /// the subject's own region: no I/O now, but a checkpoint is owed before the
    /// next session.
    fn local_change(&mut self) {
        self.pending = true;
    }

    /// Plain gossip. Writes the owed checkpoint, if any, then nothing more —
    /// reconciled content never re-arms a write. A gossip with no checkpoint
    /// owed is pure hearsay and drives no I/O at all.
    fn plain_gossip(&mut self) -> Delta {
        if self.pending {
            let reads = self.read_on_first_use();
            self.loaded = true;
            self.pending = false;
            Delta { reads, writes: 1 }
        } else {
            // `!pending ⟹ loaded`, so a suppressed session reads nothing.
            Delta {
                reads: 0,
                writes: 0,
            }
        }
    }

    /// Serving a bootstrap: the owed checkpoint (if any) before the fork, then
    /// the donation's own slice-and-write. Donating shrinks the identity, so a
    /// fresh checkpoint is owed afterwards.
    fn serve_bootstrap(&mut self) -> Delta {
        let reads = self.read_on_first_use();
        let writes = 1 + usize::from(self.pending);
        self.loaded = true;
        self.pending = true;
        Delta { reads, writes }
    }

    /// Absorbing a retiree: the owed checkpoint (if any) before the session,
    /// then the post-absorption write that records the grown party. The absorbed
    /// identity is now persisted, so nothing is owed afterwards.
    fn absorb_retire(&mut self) -> Delta {
        let reads = self.read_on_first_use();
        let writes = 1 + usize::from(self.pending);
        self.loaded = true;
        self.pending = false;
        Delta { reads, writes }
    }

    /// The subject retiring: the owed checkpoint (if any) before the session,
    /// then the whole-party donation's slice-and-write. Terminal.
    fn retire_subject(&mut self) -> Delta {
        let reads = self.read_on_first_use();
        let writes = 1 + usize::from(self.pending);
        self.loaded = true;
        Delta { reads, writes }
    }
}

// ---- birth -----------------------------------------------------------------

/// How the subject enters the world. The two origins differ only in their
/// attach-time I/O and initial model state; every later operation is identical.
#[derive(Debug, Clone, Copy)]
enum Origin {
    /// A fresh [`seed`](Peer::seed). Pristine, so the attach drives no I/O and
    /// the read is deferred to first use.
    Seed,
    /// A [`bootstrap`](Peer::bootstrap) fork of an existing seed, bookmarked
    /// after birth. Not pristine, so the attach eagerly reads and writes once.
    Bootstrap,
}

/// Everything a generated lifetime needs once the subject exists: the
/// bookmarked subject, a direct handle on its I/O log (which outlives the
/// subject when it retires), the model in its post-birth state, and any
/// counterparties already present (the origin seed, for a bootstrapped peer).
struct Birth {
    subject: Rumors<u64, Probe>,
    log: Arc<Mutex<Vec<Io>>>,
    model: Model,
    helpers: Vec<Rumors<u64>>,
}

/// Bring a bookmarked subject into being by the given `origin`, asserting the
/// attach-time I/O each origin promises.
async fn birth(origin: Origin) -> Birth {
    let log = Arc::new(Mutex::new(Vec::new()));
    let store = Arc::new(Mutex::new(BTreeMap::new()));
    let probe = Probe {
        store,
        log: Arc::clone(&log),
    };
    match origin {
        Origin::Seed => {
            let subject = Peer::<u64>::seed()
                .bookmark(probe)
                .await
                .expect("a pristine seed attaches without touching storage");
            assert!(
                log.lock().unwrap().is_empty(),
                "attaching a bookmark to a pristine seed must drive no I/O",
            );
            Birth {
                subject: subject.into_rumors(),
                log,
                model: Model::pristine_seed(),
                helpers: Vec::new(),
            }
        }
        Origin::Bootstrap => {
            // A separate seed originates the universe and serves the subject's
            // bootstrap, then stays on as the subject's first counterparty.
            let origin = Peer::<u64>::seed().into_rumors();
            let fork = bootstrap_fork_peer(&origin).await;
            let subject = fork
                .bookmark(probe)
                .await
                .expect("a non-pristine fork attaches by eagerly persisting");
            assert_eq!(
                log.lock().unwrap().clone(),
                vec![Io::Read, Io::Write],
                "attaching to a bootstrap fork reads then writes, exactly once each",
            );
            Birth {
                subject: subject.into_rumors(),
                log,
                model: Model::bootstrap_fork(),
                helpers: vec![origin],
            }
        }
    }
}

// ---- anchor tests ----------------------------------------------------------

/// The read is lazy: deferred past attach to the first session that needs the
/// record, and it is a [`Io::Read`] preceding every [`Io::Write`].
#[test]
fn read_is_deferred_to_first_use() {
    let probe = Instrument::pristine_seed();
    assert!(
        probe.history().is_empty(),
        "no I/O before the first session",
    );

    let _helper = block_on(serve_bootstrap(&probe.subject));

    let history = probe.history();
    assert_eq!(
        history[0],
        Io::Read,
        "the first I/O of all is the lazy read"
    );
    assert_eq!(
        history.iter().filter(|e| **e == Io::Read).count(),
        1,
        "exactly one read",
    );
}

/// The heart of the contract: a session that *only incorporates remote content*
/// writes nothing. A local send drives a checkpoint; the next session, after a
/// *helper's* send, pulls that content in but persists nothing, because the
/// subject's own region did not advance.
#[test]
fn incorporating_remote_content_writes_nothing() {
    let probe = Instrument::pristine_seed();
    let helper = block_on(serve_bootstrap(&probe.subject));

    // A local change, then a session: the change is checkpointed.
    probe.subject.send(1);
    let before = probe.cursor();
    block_on(plain_gossip(&probe.subject, &helper));
    let (_reads, writes) = probe.counts_since(before);
    assert!(writes >= 1, "a session after a local send must persist it");

    // Now the *helper* changes, and the subject pulls it in over a session that
    // does no local work. Not one write may occur.
    helper.send(2);
    let before = probe.cursor();
    block_on(plain_gossip(&probe.subject, &helper));
    let (reads, writes) = probe.counts_since(before);
    assert_eq!(
        (reads, writes),
        (0, 0),
        "incorporating remote content must drive no bookmark I/O",
    );
    assert!(
        probe.subject.snapshot().iter().any(|(_, _, m)| **m == 2),
        "the remote content was nonetheless incorporated",
    );
}

/// The record is read exactly once across a long life of many sessions and
/// sends — the read never repeats once the cache is warm.
#[test]
fn read_happens_exactly_once_across_a_long_life() {
    let probe = Instrument::pristine_seed();
    let helper = block_on(serve_bootstrap(&probe.subject));

    for round in 0..16u64 {
        probe.subject.send(round);
        block_on(plain_gossip(&probe.subject, &helper));
        helper.send(1_000 + round);
        block_on(plain_gossip(&probe.subject, &helper));
    }

    assert_eq!(
        probe.history().iter().filter(|e| **e == Io::Read).count(),
        1,
        "the durable record is read exactly once per peer",
    );
}

/// A peer born by bootstrap is not pristine, so attaching its bookmark eagerly
/// persists its forked identity — exactly one read then one write — and that
/// attach read is the lifetime's only read: a following session never repeats
/// it, and (no local work having intervened) it re-records but does not re-read.
#[test]
fn attaching_to_a_fork_eagerly_persists_then_never_re_reads() {
    block_on(async {
        let Birth {
            subject,
            log,
            helpers,
            ..
        } = birth(Origin::Bootstrap).await;
        assert_eq!(
            log.lock().unwrap().clone(),
            vec![Io::Read, Io::Write],
            "the attach reads then writes the fork's identity, once each",
        );

        // The first session after attach: it re-records the identity (the attach
        // never staged the suppression token), but the warm cache spares a read.
        let before = log.lock().unwrap().len();
        plain_gossip(&subject, &helpers[0]).await;
        let after = log.lock().unwrap().clone();
        let session = &after[before..];
        assert_eq!(
            session.iter().filter(|e| **e == Io::Read).count(),
            0,
            "a bootstrapped peer never reads again after the attach read",
        );
        assert_eq!(
            after.iter().filter(|e| **e == Io::Read).count(),
            1,
            "exactly one read over the whole life, taken at attach",
        );
    });
}

// ---- the proptest ----------------------------------------------------------

/// One step of an arbitrary peer lifetime. Indices are taken modulo the live
/// helper count, so they always name a real counterparty (or are skipped when
/// none exist yet).
#[derive(Debug, Clone, Copy)]
enum Op {
    /// A local send: ticks the subject's own region.
    Send,
    /// A local redact of the key at this index of the subject's snapshot; a
    /// no-op (no tick) when the subject holds nothing.
    Redact(usize),
    /// A helper sends, so the *next* gossip carries genuinely new remote content
    /// the subject must incorporate without persisting.
    HelperSend(usize),
    /// Plain gossip with a helper.
    Gossip(usize),
    /// Serve a bootstrap, donating a fork and gaining a helper.
    Serve,
    /// A helper retires into the subject, which absorbs its party.
    Absorb(usize),
}

fn op_strategy() -> impl Strategy<Value = Op> {
    prop_oneof![
        3 => Just(Op::Send),
        2 => (0usize..8).prop_map(Op::Redact),
        3 => (0usize..8).prop_map(Op::HelperSend),
        4 => (0usize..8).prop_map(Op::Gossip),
        2 => Just(Op::Serve),
        1 => (0usize..8).prop_map(Op::Absorb),
    ]
}

/// The running state threaded through a generated lifetime.
struct World {
    probe: Instrument,
    model: Model,
    helpers: Vec<Rumors<u64>>,
    next_msg: u64,
}

impl World {
    /// Apply one operation, returning the I/O it was *expected* to drive (per
    /// the model) for the caller to check against the probe. `None` means the
    /// operation was skipped (e.g. a session with no helper) and drove nothing.
    async fn apply(&mut self, op: Op) -> Option<Delta> {
        match op {
            Op::Send => {
                // A send always inserts a fresh message, ticking the subject's
                // own region: a checkpoint is always owed afterwards.
                self.probe.subject.send(self.next_msg);
                self.next_msg += 1;
                self.model.local_change();
                None
            }
            Op::Redact(i) => {
                // Redacting a key the application currently holds always records
                // a deletion in the subject's own region, ticking it; redacting
                // nothing (an empty set) is a true no-op. Liveness is read from
                // the snapshot — the application's own view — never from the
                // version arithmetic the suppression uses.
                let keys: Vec<Key> = self
                    .probe
                    .subject
                    .snapshot()
                    .iter()
                    .map(|(k, _, _)| k)
                    .collect();
                if !keys.is_empty() {
                    self.probe.subject.redact(keys[i % keys.len()]);
                    self.model.local_change();
                }
                None
            }
            Op::HelperSend(i) => {
                if !self.helpers.is_empty() {
                    let n = self.helpers.len();
                    self.helpers[i % n].send(1_000_000 + self.next_msg);
                    self.next_msg += 1;
                }
                None
            }
            Op::Gossip(i) => {
                if self.helpers.is_empty() {
                    return None;
                }
                let n = self.helpers.len();
                plain_gossip(&self.probe.subject, &self.helpers[i % n]).await;
                Some(self.model.plain_gossip())
            }
            Op::Serve => {
                let helper = serve_bootstrap(&self.probe.subject).await;
                self.helpers.push(helper);
                Some(self.model.serve_bootstrap())
            }
            Op::Absorb(i) => {
                if self.helpers.is_empty() {
                    return None;
                }
                let n = self.helpers.len();
                let retiree = self.helpers.remove(i % n);
                absorb_retire(&self.probe.subject, retiree).await;
                Some(self.model.absorb_retire())
            }
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 128, ..ProptestConfig::default() })]

    /// Over an arbitrary peer lifetime — born either as a fresh seed or as a
    /// bootstrap fork, then any interleaving of sends, redactions, remote-content
    /// arrivals, plain gossip, bootstrap donations, and retiree absorptions,
    /// optionally ending in the subject's own retirement — the bookmark's
    /// read/write schedule matches the model exactly at every step, and so:
    ///
    /// 1. **Read once.** Across the whole life the record is read at most once
    ///    (at attach for a fork, lazily at first use for a seed), and that read
    ///    precedes every write.
    /// 2. **Write on local work, never on hearsay.** Each session writes iff a
    ///    local change or party movement is owed since the last persist;
    ///    incorporating remote content drives no I/O; a send or redact alone
    ///    drives no I/O until the session that checkpoints it.
    #[test]
    fn bookmark_io_schedule_matches_the_model(
        origin in prop_oneof![Just(Origin::Seed), Just(Origin::Bootstrap)],
        script in prop::collection::vec(op_strategy(), 0..40),
        retire_at_end: bool,
    ) {
        block_on(async {
            let Birth { subject, log, model, helpers } = birth(origin).await;
            let mut world = World {
                probe: Instrument {
                    subject,
                    log: Arc::clone(&log),
                },
                model,
                helpers,
                next_msg: 0,
            };

            for op in script {
                let cursor = world.probe.cursor();
                let expected = world.apply(op).await;
                let (reads, writes) = world.probe.counts_since(cursor);
                let actual = Delta { reads, writes };
                match expected {
                    Some(predicted) => assert_eq!(
                        actual, predicted,
                        "operation {op:?} drove unexpected bookmark I/O",
                    ),
                    None => assert_eq!(
                        actual,
                        Delta { reads: 0, writes: 0 },
                        "a local or skipped operation {op:?} must drive no bookmark I/O",
                    ),
                }
                // The load never lags the first write: a checkpoint is cleared
                // only by a write, and a write is always preceded by the read.
                assert!(
                    world.model.pending || world.model.loaded,
                    "model reached `!pending && !loaded`, which a write cannot produce",
                );
            }

            // Optionally end the life with the subject's own retirement, which
            // consumes it. The subject is the sole handle to its set, so it
            // moves out of the world here rather than cloning.
            if retire_at_end && !world.helpers.is_empty() {
                let cursor = world.probe.cursor();
                let predicted = world.model.retire_subject();
                let absorber = world.helpers.remove(0);
                retire_subject(world.probe.subject, &absorber).await;
                let (reads, writes) = {
                    let log = log.lock().unwrap();
                    let slice = &log[cursor..];
                    (
                        slice.iter().filter(|e| **e == Io::Read).count(),
                        slice.iter().filter(|e| **e == Io::Write).count(),
                    )
                };
                assert_eq!(
                    Delta { reads, writes },
                    predicted,
                    "the subject's retirement drove unexpected bookmark I/O",
                );
            }

            // Global read-once: at most one read over the whole life, and it
            // precedes every write.
            let history = log.lock().unwrap().clone();
            let reads = history.iter().filter(|e| **e == Io::Read).count();
            assert!(reads <= 1, "the record was read more than once: {history:?}");
            if let (Some(r), Some(w)) = (
                history.iter().position(|e| *e == Io::Read),
                history.iter().position(|e| *e == Io::Write),
            ) {
                assert!(r < w, "a write preceded the lazy read: {history:?}");
            }
        });
    }
}
