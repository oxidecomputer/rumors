//! Bookmarking never recycles a version identifier, however adverse the run.
//!
//! This is the disruption simulation's sibling, aimed squarely at the identity
//! [`Bookmark`](rumors::Bookmark): a fleet of peers that each begin as their own
//! [`seed`](rumors::Peer::seed) (their own [`Network`]), gossip and converge
//! toward a single network by the documented `(min_ticks, network)` tie-break,
//! and throughout suffer two independent, shrinkable streams of synthetic
//! failure:
//!
//! - **wire faults** — sessions severed at arbitrary byte offsets (reusing
//!   [`common::fault`]), so messages are lost and hand-offs are interrupted; and
//! - **bookmark faults** — reads and writes of each peer's durable identity
//!   store fail on a proptest schedule (reusing [`common::flaky`]), and peers
//!   crash, dropping their in-memory state and reloading from that store.
//!
//! # The property
//!
//! Within any one [`Network`], no message that became **durable** — persisted by
//! its emitter's bookmark *or* propagated to another peer, either of which means
//! the network can no longer forget it — is ever followed by a later durable
//! message whose [`Version`] is dominated by, equal to, or otherwise in the
//! causal past of it. Two independently-`seed`ed
//! universes are causally incomparable (the crate's hard rule), so the property
//! is stated and checked per network; within a network every party is a fork of
//! one seed, so all versions are comparable, and a later version can be `<=` an
//! earlier one only by rolling backwards (concurrent versions compare
//! incomparable, never `<=`). The invariant is observable non-recycling: a
//! recycle re-issues coordinates without knowing whether durable content
//! occupies them, so any bookkeeping able to recycle at all does so
//! observably on the plans that place durable content there (the
//! reconstructed test and the known-bad artifact construct such plans, and
//! the survival checks catch the loss), and the emitter's id-region
//! therefore enters no comparison.
//!
//! A recycle is also checked by its consequence. A rebooted peer that
//! re-owns a region below a frontier some replica durably holds emits
//! versions the causal sieve reads as already deleted wherever the message
//! they collide with is held, and the fleet converges without it. Such an
//! emission compares `Greater` or incomparable to the message it destroys
//! whenever the reclaimer's frontier carries any other region's progress,
//! so the version order alone cannot see it. The [`World`] keeps a
//! per-network ledger of every redaction the network has learned of and
//! checks survival at every session: after a session both sides complete,
//! each holds every message either held before it and never redacted in
//! their network, and after the heal every message the winning network held
//! at heal start and never redacted is live at every peer. The checks are
//! sound because a correct bookmark's frontier dominates a durable emission
//! only by having merged it or a redacter's frontier, so the ledger's
//! entries are the only legitimate losses; they are complete because a
//! failed session leaves content unchanged or commits it whole, so a
//! destruction cannot hide inside an unchecked failed session.
//!
//! Durability is the load-bearing qualifier. A plain `send` neither persists
//! (only sessions do) nor propagates, so a local emission lost to a crash before
//! it is ever persisted *or* reaches another peer was never known to the
//! network, and reusing its version is not a recycle. The test therefore holds
//! each emission *pending* until it is persisted or propagated, and a crash
//! discards what was never secured.
//!
//! A broken bookmark violates the property by **recycling**: a peer that
//! re-owns an id-region whose recorded version the network has not caught up to
//! — a failed `slice` leaving a donated region claimable, or a stale record
//! after a failed `write` — then emits a message whose version a *prior*,
//! durable one already occupies. A correct bookmark cannot: `reclaim` re-owns a
//! region only once the live frontier dominates its recorded version, and
//! absorption/retire only proceed when the absorber reflexively dominates the
//! retiree.
//!
//! # Determinism
//!
//! Unlike `disruption.rs`, this simulation runs single-threaded under the
//! closed-world poller ([`common::wire::block_on`]) with a plan-driven
//! schedule: each session is its own `block_on`, and a session that stops
//! making progress fails at its source instead of hanging the case. The bug
//! class is about the *ordering* of emit/gossip/crash/retire/persist-fail
//! events and the persistence-fault sequence, not watch-channel thread
//! races. Every input the plan does not carry is fixed by the [`World`]:
//! message ids and emission sequence numbers come from a per-world counter,
//! and every universe's [`Network`] identifier, the tie-break between two
//! fresh peers, comes from a per-world RNG seeded with [`NETWORK_SEED`].
//! The schedule is deterministic up to tokio's `select!` branch order
//! inside the session internals: the RNG behind that order is per-thread
//! state advanced by every `select!`, so a wire cut at a fixed byte offset
//! can land on a different frame across runs and across the cases of one
//! run. Shrinking
//! and replay are therefore exact for clean plans, and for faulted plans
//! exact up to which frame a cut lands on. Each message's emitted version
//! is captured race-free.

mod common;

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use before::Party;
use proptest::prelude::*;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rumors::error::{Mismatch, ProtocolViolation};
use rumors::{
    BookmarkError, BookmarkIo, Error, MERKLE_HASH_LEN, Network, Peer, Retire, Rumors, Version,
};

use crate::common::fault::{self, FaultPlan};
use crate::common::flaky::{
    DurableStore, FaultFeed, FlakyError, FlakyInMemoryBookmark, persisted_record,
};
use crate::common::sim::arb_fault;
use crate::common::wire::block_on;

/// The message payload: a simulation-unique id that is also the message's
/// emission sequence number, so a single per-[`World`] counter assigns both at
/// once.
type Msg = u64;

/// The seed of every [`World`]'s network RNG.
///
/// Each universe the simulation creates draws its [`Network`] identifier
/// from this stream, so the `(min_ticks, network)` tie-break between fresh
/// peers is the same on every run and every replay of a plan. The stream is
/// ChaCha8, whose output for a seed is fixed across platforms and `rand`
/// versions; the reconstructed tests pin paths derived from it.
const NETWORK_SEED: u64 = 0;

/// Capacity for every in-memory link stream; the mirror protocol alternates
/// within a session, so a modest buffer suffices and exercises backpressure.
const LINK_BUF: usize = 8 * 1024;

/// A hard ceiling on heal-phase full-mesh rounds, per peer. A correct fleet
/// reaches a fixed point in a handful; the cap turns a convergence bug into a
/// loud failure rather than a hang.
const MAX_HEAL_ROUNDS_PER_PEER: usize = 16;

// ---- the emission log: the property's witness -------------------------------

/// One emitted message: the network it entered, its real-time emission order,
/// and the event [`Version`] stamped on its leaf.
///
/// The [`Version`] alone is the whole identifier we guard: within one network
/// all versions are causally comparable or concurrent (every party forks one
/// seed; concurrent pairs compare [`None`]), so a later emission can be `<=`
/// an earlier one only by rolling backwards over a version the network
/// already durably held — exactly a recycle. The emitting party's identity
/// is deliberately *not* recorded: the module docs state the invariant as
/// observable non-recycling, which the survival checks judge.
struct Emission {
    network: Network,
    seq: u64,
    version: Version,
}

/// The durable record of every emission that became **known to the network** —
/// persisted by its emitter's bookmark *or* propagated to another peer —
/// grouped by network and the judge of the causality property.
///
/// Emissions are held *pending* on their node ([`Node::pending`]) until
/// [`secure`] promotes them here; a local send lost to a crash before being
/// secured was never known to the network, so reusing its version is not a
/// recycle.
///
/// [`secure`]: World::secure
#[derive(Default)]
struct EmissionLog {
    durable: Mutex<BTreeMap<Network, Vec<Emission>>>,
}

impl EmissionLog {
    /// Admit a now-durable emission, asserting it recycles no other durable
    /// emission in its network.
    ///
    /// A recycle is a *later*-emitted message (by `seq`) whose event version is
    /// dominated by or equal to an *earlier* durable one's — a version the
    /// network already held, handed out a second time.
    ///
    /// Checked as the durable set grows (rather than only in a final sweep) so
    /// the failure lands on the first offending emission — the most-shrunk
    /// witness proptest can reach.
    fn promote(&self, emission: Emission) {
        let mut durable = self.durable.lock().unwrap();
        let peers = durable.entry(emission.network).or_default();
        for other in peers.iter() {
            let (earlier, later) = if other.seq < emission.seq {
                (other, &emission)
            } else {
                (&emission, other)
            };
            // A recycle is `later <= earlier` in the causal (partial) order;
            // genuinely concurrent versions compare `None` and are fine.
            assert!(
                !matches!(
                    later.version.partial_cmp(&earlier.version),
                    Some(Ordering::Less | Ordering::Equal)
                ),
                "causality violation in network {:?}: durable emission #{} \
                 (version {:?}) is dominated by or equal to earlier durable \
                 emission #{} (version {:?}) — a recycled version identifier",
                emission.network,
                later.seq,
                later.version,
                earlier.seq,
                earlier.version,
            );
        }
        peers.push(emission);
    }

    /// Total number of durable emissions witnessed across all networks.
    fn len(&self) -> usize {
        self.durable.lock().unwrap().values().map(Vec::len).sum()
    }

    /// Whether the exact live leaf emission has been promoted durable.
    fn contains_exact(&self, network: Network, seq: u64, version: &Version) -> bool {
        self.durable
            .lock()
            .unwrap()
            .get(&network)
            .is_some_and(|emissions| {
                emissions
                    .iter()
                    .any(|emission| emission.seq == seq && emission.version == *version)
            })
    }
}

/// Decode the versions a node has durably persisted, per network, through the
/// same frame the bookmark itself stored. Used to decide which pending
/// emissions the store now covers.
fn decompose_store(store: &DurableStore) -> BTreeMap<Network, Vec<Version>> {
    persisted_record(store)
        .into_iter()
        .map(|(network, clocks)| {
            (
                network,
                clocks
                    .into_iter()
                    .map(|clock| clock.into_parts().1)
                    .collect(),
            )
        })
        .collect()
}

/// Whether a node's persisted `record` covers `emission`: it has persisted, in
/// the emission's network, a frontier whose version dominates or equals the
/// emission's.
///
/// Coverage is the bookmark's promise that this version survives a
/// crash, so it is the moment the emission becomes durable.
fn store_covers(record: &BTreeMap<Network, Vec<Version>>, emission: &Emission) -> bool {
    record
        .get(&emission.network)
        .is_some_and(|versions| versions.iter().any(|version| emission.version <= *version))
}

/// Decode the id-regions a node has durably checkpointed for `network`, via the
/// same encode/decode round trip the bookmark itself makes.
///
/// The dual of
/// [`decompose_store`]: that keeps each clock's version (for durability), this
/// keeps each clock's [`Party`] (for coverage). A region recorded here is one a
/// crashed peer can still reclaim, so it counts as *held*, not leaked.
fn store_parties(store: &DurableStore, network: Network) -> Vec<Party> {
    persisted_record(store)
        .into_iter()
        .filter(|(net, _)| *net == network)
        .flat_map(|(_, clocks)| clocks.into_iter().map(|clock| clock.into_parts().0))
        .collect()
}

// ---- the session error classifier -------------------------------------------

/// Admit only failures explained by this harness: transport cuts, injected
/// bookmark I/O, and network mismatches. Protocol and bookmark-format errors
/// indicate bugs, regardless of the types used by their diagnostic sources.
fn assert_not_codec_bug<B>(step: &str, error: &Error<B>)
where
    B: BookmarkError + std::fmt::Debug,
    B::Error: std::fmt::Debug,
{
    let transport = |io: &std::io::Error| io.kind() != std::io::ErrorKind::InvalidData;
    let admitted = match error {
        Error::Transport(error) => transport(&error.source),
        Error::Mismatch(Mismatch::Network { .. }) | Error::Bookmark(BookmarkIo::Io(_)) => true,
        _ => false,
    };
    assert!(
        admitted,
        "{step}: a protocol, codec, or bookmark-format bug, not an injected disruption: {error:?}",
    );
}

/// Why a bootstrap's joining side came up without a live peer.
#[derive(Debug)]
enum BootFailure {
    /// The join session failed before any peer existed.
    Join(Error),
    /// The join ended without a peer, which only a server that was itself
    /// bootstrapping could cause.
    NoPeer,
    /// The eager attach persist failed; the handed-back peer is dropped.
    Attach(BookmarkIo<FlakyError>),
}

// ---- the fleet --------------------------------------------------------------

/// One participant across all its incarnations: its live handle (or `Dormant`
/// after a crash or retirement), plus the durable state — store and fault
/// schedule — that outlives any single incarnation.
struct Node {
    state: NodeState,
    /// The durable identity bytes (the "disk"), shared with every incarnation.
    store: DurableStore,
    /// The bookmark fail schedule, shared for the same reason.
    faults: Arc<Mutex<FaultFeed>>,
    /// The network this node currently belongs to (its last-known one while
    /// dormant), so a revival rejoins the right universe.
    network: Network,
    /// Emissions made by this incarnation that are not yet known to the network:
    /// neither persisted nor seen by another peer, so still erasable by a crash.
    /// Promoted to the durable [`EmissionLog`] once persisted or propagated.
    pending: Vec<Emission>,
    /// Redactions made by this incarnation that no other peer has seen.
    ///
    /// A crash discards them with the incarnation, and the message they
    /// deleted stays owed to the fleet. Promoted to the network's ledger
    /// ([`World::redacted`]) once another live peer's frontier dominates the
    /// redacter's.
    pending_redactions: Vec<Redacted>,
    label: usize,
}

enum NodeState {
    // Boxed: a live handle carries the peer's whole configuration,
    // hundreds of bytes wide against the dataless `Dormant`.
    Live(Box<Rumors<Msg, FlakyInMemoryBookmark>>),
    Dormant,
}

impl Node {
    fn is_live(&self) -> bool {
        matches!(self.state, NodeState::Live(_))
    }

    fn live(&self) -> Option<&Rumors<Msg, FlakyInMemoryBookmark>> {
        match &self.state {
            NodeState::Live(rumors) => Some(rumors),
            NodeState::Dormant => None,
        }
    }

    fn bookmark(&self) -> FlakyInMemoryBookmark {
        FlakyInMemoryBookmark::new(self.store.clone(), self.faults.clone(), self.label)
    }
}

/// The whole simulated world: the fleet, the shared emission log, and the
/// per-world sources of every input the plan does not carry.
struct World {
    nodes: Vec<Node>,
    emissions: EmissionLog,
    next_seq: u64,
    /// The source of every universe's [`Network`] identifier, seeded with
    /// [`NETWORK_SEED`] so the tie-break between fresh peers replays.
    rng: ChaCha8Rng,
    /// Every network this world has seeded, to assert they are pairwise
    /// distinct: two universes sharing an identifier would gossip as one
    /// network while holding incomparable histories.
    networks: BTreeSet<Network>,
    /// The path this world took through every place a network identifier
    /// decides the outcome, which a reconstructed counterexample pins.
    path: Vec<PathEvent>,
    /// Every redaction the network has learned of, by network: the messages
    /// a session or the heal may legitimately end without.
    ///
    /// A redaction is modeled like an emission: it waits in its redacter's
    /// [`pending_redactions`](Node::pending_redactions) until some other live
    /// peer's frontier dominates the redacter's (the deletion propagated),
    /// and a crash before then discards it, so a message whose only
    /// redaction died with its redacter stays owed to the fleet.
    redacted: BTreeMap<Network, Vec<Redacted>>,
    /// The winning network's live content when the heal began, and the
    /// network itself: what [`assert_healed`](World::assert_healed) holds the
    /// converged fleet to, less the ledger. `None` until a heal has run.
    heal_start: Option<(Network, BTreeSet<u64>)>,
}

/// One redaction as the harness performed it.
///
/// The network, the message's id, the version the application handed to
/// `redact`, and the redacter's frontier afterwards, which another peer must
/// dominate to have learned of the deletion.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Redacted {
    network: Network,
    seq: u64,
    version: Version,
    frontier: Version,
}

/// One step of the path a plan takes through the places where a universe's
/// identifier decides what happens; the reconstructed counterexamples pin
/// the whole path.
#[derive(Debug, Clone, PartialEq, Eq)]
enum PathEvent {
    /// A cross-network session resolved: `loser` re-bootstraps into `winner`.
    Mismatch { winner: usize, loser: usize },
    /// `newcomer` bootstrapped from `server`, or stayed dormant on failure.
    Bootstrap {
        newcomer: usize,
        server: usize,
        booted: bool,
    },
    /// `who` seeded a fresh universe: no live member of its network remained.
    Reseeded(usize),
}

impl World {
    /// A world with no nodes yet and fresh per-world sources.
    fn empty() -> Self {
        World {
            nodes: Vec::new(),
            emissions: EmissionLog::default(),
            next_seq: 0,
            rng: ChaCha8Rng::seed_from_u64(NETWORK_SEED),
            networks: BTreeSet::new(),
            path: Vec::new(),
            redacted: BTreeMap::new(),
            heal_start: None,
        }
    }

    /// Seed a fresh universe for `bookmark`'s node, drawing its network from
    /// the world's RNG and asserting the identifier is new to this world.
    fn seed_universe(
        &mut self,
        bookmark: FlakyInMemoryBookmark,
    ) -> Peer<Msg, FlakyInMemoryBookmark> {
        let peer = block_on(
            Peer::<Msg>::seed_rng(&mut self.rng)
                .sync_window_floor()
                .bookmark(bookmark),
        )
        .expect("a pristine seed attaches its bookmark without touching storage");
        assert!(
            self.networks.insert(peer.network()),
            "the network RNG handed out {:?} twice: two universes would share an identifier",
            peer.network(),
        );
        peer
    }

    /// Build `n` nodes, each its own freshly-seeded universe.
    fn seed(n: usize, read_faults: Vec<Vec<bool>>, write_faults: Vec<Vec<bool>>) -> Self {
        let mut world = World::empty();
        for label in 0..n {
            let store = Arc::new(Mutex::new(None));
            let faults = Arc::new(Mutex::new(FaultFeed::new(
                read_faults[label].clone(),
                write_faults[label].clone(),
            )));
            let bookmark = FlakyInMemoryBookmark::new(store.clone(), faults.clone(), label);
            let peer = world.seed_universe(bookmark);
            let network = peer.network();
            world.nodes.push(Node {
                state: NodeState::Live(Box::new(peer.into_rumors())),
                store,
                faults,
                network,
                pending: Vec::new(),
                pending_redactions: Vec::new(),
                label,
            });
        }
        world
    }

    /// Build `n` nodes that all share *one* network: node 0 seeds it, and nodes
    /// `1..n` bootstrap into it over clean wires.
    ///
    /// Every bookmark is reliable (an
    /// empty fault schedule never fails), the precondition the leakage property
    /// rests on.
    ///
    /// Unlike [`seed`](World::seed), which starts the fleet fragmented into
    /// per-peer universes and lets [`heal`](World::heal) force-collapse them, a
    /// single shared seed means the id-space is partitioned *once* and then only
    /// moves between peers by donation and reclaim. That is what makes coverage
    /// (`fold` of all live and checkpointed regions equals [`Party::seed`])
    /// meaningful: a region a recovery drops shows up as a genuine gap rather
    /// than being re-created by a fresh fork.
    fn single_network(n: usize) -> Self {
        assert!(n >= 1, "a fleet needs at least one node");
        let reliable = || Arc::new(Mutex::new(FaultFeed::new(Vec::new(), Vec::new())));

        let mut world = World::empty();
        let store = Arc::new(Mutex::new(None));
        let faults = reliable();
        let bookmark = FlakyInMemoryBookmark::new(store.clone(), faults.clone(), 0);
        let peer = world.seed_universe(bookmark);
        let network = peer.network();
        world.nodes.push(Node {
            state: NodeState::Live(Box::new(peer.into_rumors())),
            store,
            faults,
            network,
            pending: Vec::new(),
            pending_redactions: Vec::new(),
            label: 0,
        });
        // The rest start dormant in node 0's network; the bootstraps below make
        // them live forks of its identity.
        world.nodes.extend((1..n).map(|label| Node {
            state: NodeState::Dormant,
            store: Arc::new(Mutex::new(None)),
            faults: reliable(),
            network,
            pending: Vec::new(),
            pending_redactions: Vec::new(),
            label,
        }));

        for who in 1..n {
            assert!(
                world.bootstrap_into(who, 0),
                "single-network setup: reliable bootstrap of node {who} into the seed must succeed",
            );
        }
        world
    }

    fn n(&self) -> usize {
        self.nodes.len()
    }

    /// Whether live node `who` holds the message `seq`.
    fn holds(&self, who: usize, seq: u64) -> bool {
        self.nodes[who]
            .live()
            .is_some_and(|rumors| rumors.snapshot().iter().any(|(_, value)| *value == seq))
    }

    /// The version stamped on message `seq` at live node `who`.
    fn leaf_version(&self, who: usize, seq: u64) -> Version {
        self.nodes[who]
            .live()
            .expect("a live node")
            .snapshot()
            .iter()
            .find(|(_, value)| **value == seq)
            .map(|(version, _)| version.clone())
            .expect("the node holds the message")
    }

    /// Whether some peer *other than* `who` is live in `who`'s network: a peer
    /// `who` could reboot from.
    ///
    /// The leakage simulation refuses any crash that
    /// would leave this false, so a restarted party always has a live member to
    /// reclaim its identity from — the precondition that "every party which
    /// restarted eventually restores itself" demands.
    fn other_live_in_network(&self, who: usize) -> bool {
        let network = self.nodes[who].network;
        (0..self.n())
            .any(|k| k != who && self.nodes[k].is_live() && self.nodes[k].network == network)
    }

    /// The ids of every message live at `who`, or none while dormant.
    fn live_ids(&self, who: usize) -> BTreeSet<u64> {
        self.nodes[who]
            .live()
            .map(|rumors| rumors.snapshot().iter().map(|(_, value)| *value).collect())
            .unwrap_or_default()
    }

    /// The ids of every message redacted in `network`.
    fn ledger(&self, network: Network) -> BTreeSet<u64> {
        self.redacted
            .get(&network)
            .map(|entries| entries.iter().map(|entry| entry.seq).collect())
            .unwrap_or_default()
    }

    /// After a session both sides completed, every message `expected`
    /// (what the sides held before it, less the network's ledger) is live at
    /// each of `sides`.
    ///
    /// The recycle check by consequence at session granularity: a reclaimed
    /// region's re-issued versions make the causal sieve delete the message
    /// they collide with at whichever session first meets them, which may be
    /// long before the heal. Sound because a correct bookmark's frontier
    /// dominates a durable emission only by having merged it or a redacter's
    /// frontier, so the ledger's entries are the only legitimate losses.
    /// Complete because the session contract commits whole or not at all:
    /// on `Err` the replica's content is unchanged, and the exceptions
    /// (`Epilogue`, and a `Bookmark` error after absorbing a retiree) commit
    /// the session whole, so a destruction cannot hide inside an unchecked
    /// failed session; `expected` is recomputed from live content before
    /// every session, so a partial commit could never have produced a false
    /// positive either way.
    fn assert_session_preserved(
        &self,
        step: &str,
        network: Network,
        expected: &BTreeSet<u64>,
        sides: &[usize],
    ) {
        for &k in sides {
            let held = self.live_ids(k);
            let destroyed: Vec<u64> = expected.difference(&held).copied().collect();
            assert!(
                destroyed.is_empty(),
                "messages {destroyed:?} were live in network {network:?} before {step} and never \
                 redacted there, but node {k} does not hold them after it: a rebooted peer \
                 re-issued versions below a frontier the fleet durably held, and the causal \
                 sieve read them as deletions",
            );
        }
    }

    /// Whether `who`'s bookmark still has an injected failure scheduled.
    fn bookmark_may_fail(&self, who: usize) -> bool {
        self.nodes[who].faults.lock().unwrap().may_fail()
    }

    /// Whether a session between `a` and `b` can fail for a reason other
    /// than a crate bug or a network mismatch: a wire cut scheduled on either
    /// side, or a bookmark fault still queued on either side's feed.
    ///
    /// Decided before the session, since the session consumes the schedule.
    fn session_may_fail(&self, a: usize, b: usize, fault_a: FaultPlan, fault_b: FaultPlan) -> bool {
        fault_a != FaultPlan::NONE
            || fault_b != FaultPlan::NONE
            || self.bookmark_may_fail(a)
            || self.bookmark_may_fail(b)
    }

    /// Emit a fresh unique message from `who`, capturing its full causal
    /// coordinate and holding it *pending* until it is persisted or propagated.
    fn send(&mut self, who: usize) {
        self.revive(who);
        let id = self.next_seq;
        self.next_seq += 1;
        let Some(rumors) = self.nodes[who].live() else {
            return;
        };
        let network = rumors.network();
        rumors.send(id).unwrap(); // one commit per send
        // Read back the leaf's version. Nothing else runs between the commit
        // and here (every session is its own single-threaded `block_on`), so
        // the lookup is race-free and the just-sent unique id is present
        // exactly once.
        let snapshot = rumors.snapshot();
        let mut version = None;
        for (leaf_version, value) in snapshot.iter() {
            if *value == id {
                version = Some(leaf_version.clone());
                break;
            }
        }
        let version = version.expect("a just-sent message is live on its sender");
        self.nodes[who].pending.push(Emission {
            network,
            seq: id,
            version,
        });
    }

    /// Redact one of `who`'s live messages, indexed mod the live count, and
    /// hold it pending until another peer learns of it.
    ///
    /// Adversarial pressure on the version order (a clock advance with no
    /// tracked emission), and the one legitimate way a message leaves the
    /// fleet.
    fn redact(&mut self, who: usize, which: usize) {
        self.revive(who);
        let Some(rumors) = self.nodes[who].live() else {
            return;
        };
        let network = rumors.network();
        let snapshot = rumors.snapshot();
        let leaves: Vec<(Version, u64)> = snapshot
            .iter()
            .map(|(version, value)| (version.clone(), *value))
            .collect();
        if leaves.is_empty() {
            return;
        }
        let (version, seq) = leaves[which % leaves.len()].clone();
        rumors.redact(&version);
        let frontier = rumors.snapshot().latest().clone();
        self.nodes[who].pending_redactions.push(Redacted {
            network,
            seq,
            version,
            frontier,
        });
    }

    /// Promote every pending emission of `who` that has become **known to the
    /// network** into the checked [`EmissionLog`], leaving the rest pending.
    /// Called after any session, the only thing that secures an identity.
    ///
    /// An emission is known once *either*:
    ///
    /// - `who`'s own store persists a frontier that dominates it (it will
    ///   survive `who`'s crash), *or*
    /// - some *other* live peer in its network holds a frontier that dominates
    ///   it (the message reached that peer, which now knows the version — even
    ///   if `who` never persisted it).
    ///
    /// Either way the network can no longer forget the version, so reusing it
    /// would be a recycle. Only an emission that is *neither* persisted *nor*
    /// propagated remains pending, erasable by a crash with no recycle.
    ///
    /// A pending redaction is promoted to the network's ledger by the second
    /// route only: a persisted frontier carries no deletion to anyone, so
    /// only propagation makes a redaction one the fleet has learned of.
    fn secure(&mut self, who: usize) {
        let record = decompose_store(&self.nodes[who].store);
        // Every other live peer's `(network, frontier)`: a peer knows `who`'s
        // emission iff its frontier in the same network dominates it.
        let observers: Vec<(Network, Version)> = (0..self.n())
            .filter(|&k| k != who)
            .filter_map(|k| {
                self.nodes[k]
                    .live()
                    .map(|rumors| (rumors.network(), rumors.snapshot().latest().clone()))
            })
            .collect();

        let pending = std::mem::take(&mut self.nodes[who].pending);
        let mut still_pending = Vec::new();
        for emission in pending {
            let persisted = store_covers(&record, &emission);
            let propagated = observers.iter().any(|(network, frontier)| {
                *network == emission.network && emission.version <= *frontier
            });
            if persisted || propagated {
                self.emissions.promote(emission);
            } else {
                still_pending.push(emission);
            }
        }
        self.nodes[who].pending = still_pending;

        let pending_redactions = std::mem::take(&mut self.nodes[who].pending_redactions);
        let mut still_pending = Vec::new();
        for redaction in pending_redactions {
            let propagated = observers.iter().any(|(network, frontier)| {
                *network == redaction.network && redaction.frontier <= *frontier
            });
            if propagated {
                self.redacted
                    .entry(redaction.network)
                    .or_default()
                    .push(redaction);
            } else {
                still_pending.push(redaction);
            }
        }
        self.nodes[who].pending_redactions = still_pending;
    }

    /// Secure what is now known to the network, then discard the rest.
    ///
    /// `who`'s
    /// in-memory state is about to vanish (a crash or a re-bootstrap), so any
    /// emission that was neither persisted nor seen by another peer is lost —
    /// never known to the network, and so no later reuse of its version is a
    /// recycle.
    fn secure_and_lose(&mut self, who: usize) {
        self.secure(who);
        self.nodes[who].pending.clear();
        self.nodes[who].pending_redactions.clear();
    }

    /// Drop `who`'s in-memory state, keeping its durable store and schedule: a
    /// transient crash. The next operation that targets it revives it.
    fn crash(&mut self, who: usize) {
        self.secure_and_lose(who);
        self.nodes[who].state = NodeState::Dormant;
    }

    /// Run one gossip session between `a` and `b` over a wire faulted per
    /// `fault_a`/`fault_b`.
    ///
    /// A cross-network pair surfaces
    /// [`Mismatch::Network`] on at least one side; the loser of the
    /// `(min_ticks, network)` tie-break re-bootstraps into the winner. Any other
    /// error is a disruption, admitted only when the step scheduled a fault
    /// or met a mismatch; a step that cannot fail must succeed on both sides,
    /// and a session both sides complete loses no unredacted message.
    fn gossip(&mut self, a: usize, b: usize, fault_a: FaultPlan, fault_b: FaultPlan) {
        if a == b {
            return;
        }
        self.revive(a);
        self.revive(b);
        let (Some(ra), Some(rb)) = (self.nodes[a].live(), self.nodes[b].live()) else {
            return;
        };
        let (ra, rb) = (ra.clone(), rb.clone());
        let may_fail = self.session_may_fail(a, b, fault_a, fault_b);
        let same_network = self.nodes[a].network == self.nodes[b].network;
        let network = self.nodes[a].network;
        let held_before: BTreeSet<u64> = &self.live_ids(a) | &self.live_ids(b);
        // Each side owns its faulted link inside its own `async move` block,
        // so when a wire fault kills one side its block completes and
        // `join!` drops the block, link included, surfacing EOF to the
        // counterparty. Links declared outside the blocks would live until
        // both sides finished, deadlocking the survivor on a read that never
        // completes.
        let (out_a, out_b) = block_on(async {
            let (side_a, side_b) = rumors::link::memory_with_capacity(LINK_BUF);
            tokio::join!(
                async move {
                    let mut link = fault::faulty(side_a, fault_a);
                    ra.gossip_once(&mut link).await
                },
                async move {
                    let mut link = fault::faulty(side_b, fault_b);
                    rb.gossip_once(&mut link).await
                },
            )
        });

        // The session ran each side's bookmark update before any mismatch, so
        // secure both: their pre-session emissions are now durable.
        self.secure(a);
        self.secure(b);

        let mismatched = matches!(out_a, Err(Error::Mismatch(Mismatch::Network { .. })))
            || matches!(out_b, Err(Error::Mismatch(Mismatch::Network { .. })));
        for (side, out) in [(a, &out_a), (b, &out_b)] {
            match out {
                Ok(_) => {}
                Err(Error::Mismatch(Mismatch::Network { .. })) => assert!(
                    !same_network,
                    "gossip {a}<->{b}: node {side} reported a network mismatch inside one network",
                ),
                Err(error) => {
                    assert_not_codec_bug(&format!("gossip {a}<->{b}, node {side}"), error);
                    assert!(
                        may_fail || mismatched,
                        "gossip {a}<->{b}: node {side} failed on a clean wire over reliable \
                         bookmarks with no mismatch: {error:?}",
                    );
                }
            }
        }
        assert!(
            same_network || may_fail || mismatched,
            "gossip {a}<->{b}: a cross-network session on a clean wire must surface the mismatch",
        );
        assert!(
            same_network || out_a.is_err() || out_b.is_err(),
            "gossip {a}<->{b}: a cross-network session completed on both sides",
        );
        if same_network && out_a.is_ok() && out_b.is_ok() {
            let expected = &held_before - &self.ledger(network);
            self.assert_session_preserved(
                &format!("gossip {a}<->{b}"),
                network,
                &expected,
                &[a, b],
            );
        }
        if mismatched {
            self.resolve_mismatch(a, b);
        }
    }

    /// Re-bootstrap the lesser of two mismatched live peers into the greater's
    /// network, by the documented metric: greater `(min_ticks, network)` wins.
    fn resolve_mismatch(&mut self, a: usize, b: usize) {
        let ta = self.tuple(a);
        let tb = self.tuple(b);
        let (Some(ta), Some(tb)) = (ta, tb) else {
            return;
        };
        let (winner, loser) = if ta >= tb { (a, b) } else { (b, a) };
        self.path.push(PathEvent::Mismatch { winner, loser });
        self.bootstrap_into(loser, winner);
    }

    /// This live peer's tie-break key: its frontier's minimum event count, then
    /// its network identifier. `None` while dormant.
    fn tuple(&self, who: usize) -> Option<(rumors::Ticks, Network)> {
        let rumors = self.nodes[who].live()?;
        Some((rumors.snapshot().latest().min_ticks(), rumors.network()))
    }

    /// (Re)create `who` as a fresh replica in `server`'s network by bootstrapping
    /// from it over a clean wire.
    ///
    /// Reuses `who`'s durable store (so it reclaims
    /// any of its own identity the pulled frontier now dominates) and its still-
    /// active fault schedule. Returns whether `who` is live afterwards.
    ///
    /// The bootstrap's wire is clean so the fleet can make progress, but the
    /// bookmarks on both sides stay flaky: the server's donating `slice`/`write`
    /// and `who`'s eager identity persist can each fail, which is precisely the
    /// adversarial persistence path. On any failure `who` is left dormant for a
    /// later attempt. Over reliable bookmarks nothing can fail, and the step
    /// asserts that both sides succeed.
    fn bootstrap_into(&mut self, who: usize, server: usize) -> bool {
        let Some(server_rumors) = self.nodes[server].live() else {
            return false;
        };
        let server_rumors = server_rumors.clone();
        let may_fail = self.bookmark_may_fail(who) || self.bookmark_may_fail(server);
        let network = self.nodes[server].network;
        let served: BTreeSet<u64> = self.live_ids(server);
        // The prior incarnation's memory is about to vanish: secure what its
        // store persisted, lose the rest.
        self.secure_and_lose(who);
        let bookmark = self.nodes[who].bookmark();
        // Drop any prior incarnation before creating the new one.
        self.nodes[who].state = NodeState::Dormant;

        let (boot_out, serve_out) = block_on(async {
            let (boot_side, serve_side) = rumors::link::memory_with_capacity(LINK_BUF);
            // Each side owns its link inside its block so a failing one drops
            // it (see `gossip`).
            tokio::join!(
                async move {
                    let mut link = boot_side;
                    let peer = match Peer::<Msg>::bootstrap().join(&mut link).await {
                        rumors::Joined::Joined { peer } => peer,
                        rumors::Joined::Bailed { .. } => return Err(BootFailure::NoPeer),
                        rumors::Joined::Failed { error, .. } => {
                            return Err(BootFailure::Join(error));
                        }
                        rumors::Joined::Unbookmarked(_) => unreachable!("no bookmark was selected"),
                    };
                    peer.sync_window_floor()
                        .bookmark(bookmark)
                        .await
                        .map_err(|unbookmarked| BootFailure::Attach(unbookmarked.error))
                },
                async move {
                    let mut link = serve_side;
                    server_rumors.gossip_once(&mut link).await
                },
            )
        });

        // The wire is clean, so each side fails only through the bookmarks:
        // the server through its own donating persist, whose abort closes
        // the wire on the newcomer (a truncated hand-off), and the newcomer
        // through its own eager attach persist after the session. Neither
        // can fail over reliable bookmarks.
        let step = format!("bootstrap of {who} from {server}");
        if let Err(error) = &serve_out {
            assert_not_codec_bug(&format!("{step}, serving side"), error);
            assert!(
                may_fail,
                "{step}: the serve failed on a clean wire over reliable bookmarks: {error:?}",
            );
        }
        let booted = match boot_out {
            Ok(peer) => Some(peer),
            Err(BootFailure::NoPeer) => {
                panic!("{step}: the server was gossiping, so the join cannot end without a peer")
            }
            Err(BootFailure::Join(error)) => {
                assert_not_codec_bug(&format!("{step}, joining side"), &error);
                assert!(
                    may_fail,
                    "{step}: the join failed on a clean wire over reliable bookmarks: {error:?}",
                );
                None
            }
            Err(BootFailure::Attach(error)) => {
                assert!(
                    !matches!(error, BookmarkIo::Format(_)),
                    "{step}: the attach rejected a bookmark file the crate wrote: {error:?}",
                );
                assert!(
                    may_fail,
                    "{step}: the attach persist failed over a reliable bookmark: {error:?}",
                );
                None
            }
        };

        self.path.push(PathEvent::Bootstrap {
            newcomer: who,
            server,
            booted: booted.is_some(),
        });
        match booted {
            Some(peer) => {
                self.nodes[who].network = peer.network();
                self.nodes[who].state = NodeState::Live(Box::new(peer.into_rumors()));
                // The eager bootstrap persist secures `who`'s reclaimed identity;
                // the server's donating persist secures its emissions too.
                self.secure(who);
                self.secure(server);
                // A bootstrap copies the server's content whole.
                let expected = &served - &self.ledger(network);
                self.assert_session_preserved(&step, network, &expected, &[who]);
                true
            }
            None => false,
        }
    }

    /// Bring `who` back to life if it is dormant: rejoin its last network from a
    /// live member if one remains (reclaiming its stranded identity), else seed
    /// a brand-new universe (the degenerate solo recovery).
    fn revive(&mut self, who: usize) {
        if self.nodes[who].is_live() {
            return;
        }
        let network = self.nodes[who].network;
        let server = (0..self.n())
            .find(|&k| k != who && self.nodes[k].is_live() && self.nodes[k].network == network);
        if let Some(server) = server
            && self.bootstrap_into(who, server)
        {
            return;
        }
        // No reachable member of its old network (or the rejoin's persistence
        // failed): start fresh. Its old network's identity is left stranded --
        // a harmless leak, never a corruption. The old incarnation's memory
        // vanishes, so secure what was persisted and lose the rest. A
        // single-network world never gets here: its crash guard keeps a live
        // member, and its reliable bookmarks cannot fail the rejoin.
        assert!(
            self.networks.len() > 1,
            "node {who} of a single-network world found no live member to reboot from",
        );
        self.secure_and_lose(who);
        self.path.push(PathEvent::Reseeded(who));
        let bookmark = self.nodes[who].bookmark();
        let peer = self.seed_universe(bookmark);
        self.nodes[who].network = peer.network();
        self.nodes[who].state = NodeState::Live(Box::new(peer.into_rumors()));
    }

    /// Retire `retiree` into `absorber`, donating its identity.
    ///
    /// Same-network
    /// only: a cross-network retire cannot be absorbed, so it is skipped. The
    /// retiree's durable store may later resurrect it -- exercising party reuse
    /// across a donation, where a failed `slice` would let the donated region
    /// live twice. Over reliable bookmarks the clean-wire retirement cannot
    /// fail: the absorber must succeed and the retiree must report `Retired`.
    fn retire(&mut self, retiree: usize, absorber: usize) {
        if retiree == absorber {
            return;
        }
        self.revive(retiree);
        self.revive(absorber);
        if self.nodes[retiree].network != self.nodes[absorber].network {
            return;
        }
        let Some(absorber_rumors) = self.nodes[absorber].live() else {
            return;
        };
        let absorber_rumors = absorber_rumors.clone();
        let may_fail = self.session_may_fail(retiree, absorber, FaultPlan::NONE, FaultPlan::NONE);
        let network = self.nodes[absorber].network;
        let held_before: BTreeSet<u64> = &self.live_ids(retiree) | &self.live_ids(absorber);
        // Take the retiree's sole handle so it can become a `Peer` immediately.
        let NodeState::Live(retiree_rumors) =
            std::mem::replace(&mut self.nodes[retiree].state, NodeState::Dormant)
        else {
            return;
        };

        let (outcome, absorbed) = block_on(async {
            let (ret_side, abs_side) = rumors::link::memory_with_capacity(LINK_BUF);
            // Each side owns its link inside its block so a failing one drops
            // it (see `gossip`). The retiree becomes a `Peer` inside its
            // block: it holds the sole handle to its set, so `try_into_peer`
            // resolves at once.
            tokio::join!(
                async move {
                    let mut link = ret_side;
                    let peer = retiree_rumors
                        .try_into_peer()
                        .await
                        .expect("the node holds the sole handle to its set");
                    peer.retire(&mut link).await
                },
                async move {
                    let mut link = abs_side;
                    absorber_rumors.gossip_once(&mut link).await
                },
            )
        });
        // Never swallow the absorber's result: a retirement's whole point is
        // the hand-off, and a silently dropped failed absorption is what hid
        // the codec leak this test was written to catch. The wire is clean,
        // so the absorber fails only by an injected bookmark fault
        // (`Error::Bookmark`) or by the retiree aborting on its own bookmark
        // fault and closing the wire (`HandOffTruncated`, or `UnexpectedEof`
        // elsewhere in the session); over reliable bookmarks it cannot fail.
        let step = format!("retire of {retiree} into {absorber}");
        if let Err(error) = &absorbed {
            assert_not_codec_bug(&format!("{step}, absorber"), error);
            assert!(
                may_fail,
                "{step}: the absorber failed on a clean wire over reliable bookmarks: {error:?}",
            );
        }
        if let Retire::Recovered { error, .. } | Retire::Uncertain { error } = &outcome {
            assert_not_codec_bug(&format!("{step}, retiree"), error);
        }
        assert!(
            may_fail || matches!(outcome, Retire::Retired),
            "{step}: a clean-wire retirement over reliable bookmarks must land as \
             `Retired`, not {outcome:?}",
        );

        let retired = matches!(outcome, Retire::Retired) && absorbed.is_ok();
        match outcome {
            // Donated: the retiree's memory is consumed, so secure what it
            // persisted and lose the rest.
            Retire::Retired | Retire::Uncertain { .. } => self.secure_and_lose(retiree),
            // Unchanged: hand the intact peer back to life; its persisted
            // emissions are durable, its unpersisted ones remain pending.
            Retire::Recovered { peer, .. } => {
                self.nodes[retiree].state = NodeState::Live(Box::new(peer.into_rumors()));
                self.secure(retiree);
            }
            // The absorber gossips; only a retiring counterparty declines.
            Retire::Declined { .. } => {
                panic!("{step}: the absorber was gossiping, so the retirement cannot be declined")
            }
        }
        self.secure(absorber);
        if retired {
            // The retiree reconciled with the absorber before donating and
            // then vanished, so every redaction the absorber held pending
            // reached a peer that is no longer live to witness it: promote
            // them all, since `secure`'s observer rule cannot see them.
            let learned = std::mem::take(&mut self.nodes[absorber].pending_redactions);
            for redaction in learned {
                self.redacted
                    .entry(redaction.network)
                    .or_default()
                    .push(redaction);
            }
            let expected = &held_before - &self.ledger(network);
            self.assert_session_preserved(&step, network, &expected, &[absorber]);
        }
    }

    /// Drive the fleet to a single network and a content fixed point over clean
    /// wires with all bookmark faults disabled, so the final invariants are
    /// reachable. Mirrors `sim::quiesce`, plus the network-convergence step.
    fn heal(&mut self) {
        for node in &self.nodes {
            node.faults.lock().unwrap().disable();
        }
        for who in 0..self.n() {
            self.revive(who);
        }

        // Collapse onto one network: the peer with the greatest tie-break key
        // wins, and every other peer re-bootstraps into it.
        let winner = (0..self.n())
            .max_by_key(|&k| self.tuple(k))
            .expect("a non-empty fleet");
        let winning_network = self.nodes[winner].network;
        // What the winning network holds as the heal begins: every message
        // live at any of its live members. The other networks' content is
        // discarded by the collapse below, so only the winner's is held to
        // survive.
        let live_at_start: BTreeSet<u64> = (0..self.n())
            .filter(|&k| self.nodes[k].network == winning_network)
            .filter_map(|k| self.nodes[k].live())
            .flat_map(|rumors| {
                rumors
                    .snapshot()
                    .iter()
                    .map(|(_, value)| *value)
                    .collect::<Vec<_>>()
            })
            .collect();
        self.heal_start = Some((winning_network, live_at_start));
        for who in 0..self.n() {
            if who != winner && self.nodes[who].network != winning_network {
                assert!(
                    self.bootstrap_into(who, winner),
                    "fault-free heal bootstrap of node {who} into {winner} must succeed",
                );
            }
        }

        // Full-mesh gossip to a fixed point. All peers now share a network, so
        // no session mismatches; clean wires and disabled faults mean none fail.
        let rounds = MAX_HEAL_ROUNDS_PER_PEER * self.n();
        for _ in 0..rounds {
            let before = self.fingerprints();
            for a in 0..self.n() {
                for b in (a + 1)..self.n() {
                    self.clean_gossip(a, b);
                }
            }
            if self.fingerprints() == before {
                // Secure every emission the converged, fully-persisted fleet
                // now holds, so the final durable set is complete.
                for who in 0..self.n() {
                    self.secure(who);
                }
                return;
            }
        }
        panic!("fleet did not converge within {rounds} heal rounds");
    }

    /// One clean, fault-free gossip session between two live peers.
    fn clean_gossip(&mut self, a: usize, b: usize) {
        let (Some(ra), Some(rb)) = (self.nodes[a].live(), self.nodes[b].live()) else {
            return;
        };
        let (ra, rb) = (ra.clone(), rb.clone());
        let network = self.nodes[a].network;
        let held_before: BTreeSet<u64> = &self.live_ids(a) | &self.live_ids(b);
        let (out_a, out_b) = block_on(async {
            let (side_a, side_b) = rumors::link::memory_with_capacity(LINK_BUF);
            tokio::join!(
                async move {
                    let mut link = side_a;
                    ra.gossip_once(&mut link).await
                },
                async move {
                    let mut link = side_b;
                    rb.gossip_once(&mut link).await
                },
            )
        });
        out_a.expect("clean heal gossip A");
        out_b.expect("clean heal gossip B");
        self.secure(a);
        self.secure(b);
        let expected = &held_before - &self.ledger(network);
        self.assert_session_preserved(
            &format!("heal gossip {a}<->{b}"),
            network,
            &expected,
            &[a, b],
        );
    }

    /// Each live peer's `(hash, latest)` fingerprint, for fixed-point detection.
    fn fingerprints(&self) -> Vec<Option<([u8; MERKLE_HASH_LEN], Version)>> {
        (0..self.n())
            .map(|k| {
                self.nodes[k].live().map(|rumors| {
                    let snapshot = rumors.snapshot();
                    (snapshot.hash(), snapshot.latest().clone())
                })
            })
            .collect()
    }

    /// After a clean heal: every live peer holds identical content, their live
    /// parties are pairwise disjoint, and every unredacted message the winning
    /// network held at heal start is live at every peer.
    fn assert_healed(&self) {
        let live: Vec<usize> = (0..self.n()).filter(|&k| self.nodes[k].is_live()).collect();

        // Convergence.
        let mut reference: Option<([u8; MERKLE_HASH_LEN], Version)> = None;
        for &k in &live {
            let snapshot = self.nodes[k].live().unwrap().snapshot();
            let fingerprint = (snapshot.hash(), snapshot.latest().clone());
            match &reference {
                None => reference = Some(fingerprint),
                Some(expected) => assert_eq!(
                    &fingerprint, expected,
                    "node {k} diverged from the fleet after the heal",
                ),
            }
        }

        // Pairwise party disjointness.
        let parties: Vec<Party> = live
            .iter()
            .map(|&k| self.nodes[k].live().unwrap().dangerously_alias_party())
            .collect();
        for (i, &ni) in live.iter().enumerate() {
            for (j, &nj) in live.iter().enumerate().skip(i + 1) {
                assert!(
                    parties[i].is_disjoint(&parties[j]),
                    "live nodes {ni} and {nj} hold overlapping parties",
                );
            }
        }

        self.assert_live_content_is_durable(&live);
        self.assert_durable_content_survived(&live);
    }

    /// Every message live in the winning network when the heal began, and
    /// never redacted there, is live at every peer after it.
    ///
    /// The heal-window half of the recycle check by consequence
    /// ([`assert_session_preserved`](World::assert_session_preserved) is the
    /// per-session half): a reclaimed region's re-issued versions compare
    /// `Greater` or incomparable to the message they destroy, so
    /// [`EmissionLog::promote`] cannot see the destruction and these checks
    /// can.
    fn assert_durable_content_survived(&self, live: &[usize]) {
        let (network, live_at_start) = self
            .heal_start
            .as_ref()
            .expect("assert_healed runs after a heal");
        let redacted: BTreeSet<u64> = self
            .redacted
            .get(network)
            .map(|entries| entries.iter().map(|entry| entry.seq).collect())
            .unwrap_or_default();
        for &k in live {
            let held: BTreeSet<u64> = self.nodes[k]
                .live()
                .unwrap()
                .snapshot()
                .iter()
                .map(|(_, value)| *value)
                .collect();
            let destroyed: Vec<u64> = live_at_start
                .iter()
                .filter(|seq| !redacted.contains(seq) && !held.contains(seq))
                .copied()
                .collect();
            assert!(
                destroyed.is_empty(),
                "messages {destroyed:?} were live in network {network:?} when the heal began \
                 and never redacted there, but node {k} does not hold them after it: a \
                 rebooted peer re-issued versions below a frontier the fleet durably held, \
                 and the causal sieve read them as deletions",
            );
        }
    }

    /// Verify the recycle assertion is not passing vacuously: after the
    /// fault-free heal, every surviving live message must have an exact durable
    /// emission witness.
    ///
    /// The witness must exist because the converged fleet has now persisted
    /// and propagated all surviving content.
    fn assert_live_content_is_durable(&self, live: &[usize]) {
        let mut live_leaves = 0;
        for &k in live {
            let rumors = self.nodes[k].live().unwrap();
            let network = rumors.network();
            let snapshot = rumors.snapshot();
            for (leaf_version, value) in snapshot.iter() {
                live_leaves += 1;
                let seq = *value;
                assert!(
                    self.emissions.contains_exact(network, seq, leaf_version),
                    "live message #{seq} at version {leaf_version:?} in network {network:?} \
                     survived the heal without an exact durable emission witness",
                );
            }
        }

        if self.next_seq > 0 && live_leaves > 0 {
            assert!(
                self.emissions.len() > 0,
                "{live_leaves} live message leaves survived after {} sends, but the durable \
                 emission log is empty",
                self.next_seq,
            );
        }
    }

    /// After a clean heal of a single-network fleet: no id-region has leaked.
    ///
    /// Every region of the seed identity must be *accounted for* — held by a
    /// live peer, or checkpointed in some peer's bookmark store, where a crashed
    /// peer can still reclaim it. Folding all such regions out of
    /// [`Party::seed`] must leave nothing: a region held nowhere and recorded
    /// nowhere has been lost forever, the leak this property forbids.
    ///
    /// This is the dual of [`assert_healed`](World::assert_healed)'s live-party
    /// disjointness. Disjointness catches a region claimed *twice* (a recovery
    /// that re-owned a donated region the absorber still holds); coverage catches
    /// a region claimed *zero* times (a recovery that dropped a fragment). A
    /// correct retire/reboot leaves the donated region live in its absorber and
    /// every other fragment live or checkpointed, so both hold.
    ///
    /// Counting checkpointed-but-not-live regions is essential, not lenient: a
    /// reboot reclaims a stored region only once the live frontier dominates its
    /// recorded version, so a just-rebooted peer may legitimately hold a region
    /// only in its store, awaiting a later reclaim. That region is recoverable,
    /// hence not leaked.
    fn assert_no_leak(&self) {
        // A single-network fleet converges to one network; any live peer names
        // it. The heal guarantees at least one live peer.
        let network = (0..self.n())
            .find_map(|k| self.nodes[k].live().map(Rumors::network))
            .expect("a healed fleet has at least one live peer");

        // Gather every region the fleet can still account for: each live peer's
        // party, plus every region any peer (live or dormant) has checkpointed
        // in this network.
        let mut held: Vec<Party> = Vec::new();
        for k in 0..self.n() {
            if let Some(rumors) = self.nodes[k].live() {
                held.push(rumors.dangerously_alias_party());
            }
            held.extend(store_parties(&self.nodes[k].store, network));
        }

        // Carve each accounted-for region out of the whole identity space. What
        // remains, if anything, is held and recorded nowhere: a leak. Regions
        // overlap freely here (a peer's own store records its live party), and
        // `without` carves an already-carved region to a harmless no-op.
        let mut remaining = Some(Party::seed());
        for region in &held {
            let Some(rest) = remaining.take() else { break };
            remaining = rest.without(region);
        }
        assert!(
            remaining.is_none(),
            "identity space leaked in network {network:?}: region {remaining:?} is held by no \
             live peer and checkpointed in no bookmark store",
        );
    }
}

// ---- plan and strategy ------------------------------------------------------

/// One step of a deterministic simulation, replayed in order.
#[derive(Debug, Clone)]
enum Step {
    /// Emit a fresh unique message from a node.
    Send(usize),
    /// Redact one of a node's live messages (index mod live count).
    Redact(usize, usize),
    /// Gossip between two distinct nodes, each side wire-faulted.
    Gossip(usize, usize, FaultPlan, FaultPlan),
    /// Crash a node: drop its memory, keep its durable store.
    Crash(usize),
    /// Retire one node into another.
    Retire(usize, usize),
}

/// A whole simulation: fleet size, the ordered steps, and per-node bookmark
/// read/write fault schedules.
#[derive(Debug, Clone)]
struct Plan {
    n: usize,
    steps: Vec<Step>,
    read_faults: Vec<Vec<bool>>,
    write_faults: Vec<Vec<bool>>,
}

/// A bookmark fail schedule for one node: a short, success-biased bit sequence,
/// or empty when this plan injects no faults at all.
fn arb_fault_bits(faults: bool) -> BoxedStrategy<Vec<bool>> {
    if !faults {
        return Just(Vec::new()).boxed();
    }
    prop::collection::vec(prop_oneof![3 => Just(false), 1 => Just(true)], 0..8).boxed()
}

/// One simulation step over a fleet of `n`. Gossip and retire pick a second,
/// distinct node by a `1..n` offset so the shrinker can never collapse a pair
/// onto one node.
fn arb_step(n: usize, faults: bool) -> BoxedStrategy<Step> {
    prop_oneof![
        3 => (0..n).prop_map(Step::Send),
        1 => (0..n, 0usize..8).prop_map(|(i, k)| Step::Redact(i, k)),
        4 => (0..n, 1..n, arb_fault(faults), arb_fault(faults))
            .prop_map(move |(i, off, fa, fb)| Step::Gossip(i, (i + off) % n, fa, fb)),
        1 => (0..n).prop_map(Step::Crash),
        1 => (0..n, 1..n).prop_map(move |(i, off)| Step::Retire(i, (i + off) % n)),
    ]
    .boxed()
}

/// A whole plan. The top-level `faults` boolean is the first thing proptest
/// shrinks: clearing it disables every wire and bookmark fault, so the loss-free
/// convergence path is exercised often and shrinks toward cleanly.
fn arb_plan() -> impl Strategy<Value = Plan> {
    (any::<bool>(), 2usize..=4).prop_flat_map(|(faults, n)| {
        (
            Just(n),
            prop::collection::vec(arb_step(n, faults), 0..40),
            prop::collection::vec(arb_fault_bits(faults), n),
            prop::collection::vec(arb_fault_bits(faults), n),
        )
            .prop_map(|(n, steps, read_faults, write_faults)| Plan {
                n,
                steps,
                read_faults,
                write_faults,
            })
    })
}

/// Execute a plan to its post-heal end state.
fn run_plan(plan: Plan) -> World {
    let mut world = World::seed(plan.n, plan.read_faults, plan.write_faults);
    for step in plan.steps {
        match step {
            Step::Send(i) => world.send(i),
            Step::Redact(i, k) => world.redact(i, k),
            Step::Gossip(i, j, fa, fb) => world.gossip(i, j, fa, fb),
            Step::Crash(i) => world.crash(i),
            Step::Retire(i, j) => world.retire(i, j),
        }
    }
    world.heal();
    world
}

// ---- the reliable-recovery (no-leakage) variant -----------------------------

/// One step over a fleet of `n` for the *reliable-recovery* simulation: every
/// wire is clean ([`FaultPlan::NONE`]) and every bookmark reliable, so the only
/// adversity is crash/restart and retirement.
///
/// Crash and retire weigh heavier
/// than in [`arb_step`] because recovery — not fault injection — is the property
/// under test.
fn arb_reliable_step(n: usize) -> BoxedStrategy<Step> {
    prop_oneof![
        3 => (0..n).prop_map(Step::Send),
        1 => (0..n, 0usize..8).prop_map(|(i, k)| Step::Redact(i, k)),
        3 => (0..n, 1..n).prop_map(move |(i, off)| Step::Gossip(i, (i + off) % n, FaultPlan::NONE, FaultPlan::NONE)),
        2 => (0..n).prop_map(Step::Crash),
        2 => (0..n, 1..n).prop_map(move |(i, off)| Step::Retire(i, (i + off) % n)),
    ]
    .boxed()
}

/// A whole reliable-recovery plan. No fault schedules at all — the bookmarks
/// never fail and the wires never sever — so the only thing that ever removes a
/// peer is an explicit [`Step::Crash`] or [`Step::Retire`].
fn arb_reliable_plan() -> impl Strategy<Value = Plan> {
    (2usize..=4).prop_flat_map(|n| {
        prop::collection::vec(arb_reliable_step(n), 0..40).prop_map(move |steps| Plan {
            n,
            steps,
            read_faults: vec![Vec::new(); n],
            write_faults: vec![Vec::new(); n],
        })
    })
}

/// Minimal library-level regression for the codec leak this suite found:
/// retiring into an absorber that has itself rebooted (reclaimed its identity
/// from a bookmark) must absorb cleanly.
///
/// The absorber is left holding the whole seed identity.
///
/// Both peers reclaiming once grew the retiree's donated party via
/// [`Party::join`](before::Party), which left stale bits in its `as_bytes`
/// encoding; the absorber's session then aborted decoding it (`before::codec`
/// `TrailingBits`) while the retiree reported [`Retire::Retired`], having
/// already shipped and sliced away its party — so the donated region was held
/// by no one: a leak. With the `join` normalization fixed, the absorption
/// lands.
///
/// No `World` harness, no faults, one universe, empty content — raw library
/// calls — so it pins the fix at the library boundary. The trigger is **both**
/// peers having reclaimed: with only one side rebooted the absorption was always
/// clean.
#[test]
fn retire_into_rebooted_absorber_absorbs_cleanly() {
    let store_a = Arc::new(Mutex::new(None));
    let faults_a = Arc::new(Mutex::new(FaultFeed::new(Vec::new(), Vec::new())));
    let store_b = Arc::new(Mutex::new(None));
    let faults_b = Arc::new(Mutex::new(FaultFeed::new(Vec::new(), Vec::new())));
    let bm_a = || FlakyInMemoryBookmark::new(store_a.clone(), faults_a.clone(), 0);
    let bm_b = || FlakyInMemoryBookmark::new(store_b.clone(), faults_b.clone(), 1);

    block_on(async move {
        // A seeds, B bootstraps from A. Then each reboots once, reclaiming its
        // region from its bookmark (drop = crash; re-bootstrap = revive).
        let a = Peer::<Msg>::seed_rng(&mut ChaCha8Rng::seed_from_u64(NETWORK_SEED))
            .sync_window_floor()
            .bookmark(bm_a())
            .await
            .expect("a pristine seed attaches its bookmark without touching storage")
            .into_rumors();
        let b = boot_from_async(&a, bm_b()).await;
        drop(a);
        let a = boot_from_async(&b, bm_a()).await; // A reclaims
        drop(b);
        let b = boot_from_async(&a, bm_b()).await; // B reclaims

        // Both parties are well-formed, disjoint, and tile the seed: a valid
        // single-universe state.
        let pa = a.dangerously_alias_party();
        let pb = b.dangerously_alias_party();
        assert!(
            pa.is_disjoint(&pb),
            "A {pa:?} and B {pb:?} must be disjoint"
        );

        // B retires into A. A's gossip must absorb B's party so that A ends up
        // holding the whole seed identity.
        let (ret_side, abs_side) = rumors::link::memory_with_capacity(LINK_BUF);
        let absorber = a.clone();
        let (retire_out, absorb_out) = tokio::join!(
            async move {
                let mut link = ret_side;
                let peer = b.try_into_peer().await.expect("sole handle");
                peer.retire(&mut link).await
            },
            async move {
                let mut link = abs_side;
                absorber.gossip_once(&mut link).await
            },
        );

        assert!(
            matches!(retire_out, Retire::Retired),
            "the retiree should have retired",
        );
        absorb_out.expect("the absorber's gossip must not fail while taking a retirement");
        assert_eq!(
            a.dangerously_alias_party(),
            Party::seed(),
            "after absorbing the only other peer, A must hold the whole seed identity",
        );
    });
}

/// Bootstrap a fresh peer with bookmark `bm` from `server` over a clean
/// in-memory link, returning the booted peer's [`Rumors`]. Diagnostic helper.
async fn boot_from_async(
    server: &Rumors<Msg, FlakyInMemoryBookmark>,
    bm: FlakyInMemoryBookmark,
) -> Rumors<Msg, FlakyInMemoryBookmark> {
    let server = server.clone();
    let (boot_side, serve_side) = rumors::link::memory_with_capacity(LINK_BUF);
    let (boot_out, serve_out) = tokio::join!(
        async move {
            let mut link = boot_side;
            let peer = (match Peer::<Msg>::bootstrap().join(&mut link).await {
                rumors::Joined::Joined { peer } => peer,
                _ => panic!("got a peer"),
            })
            .sync_window_floor();
            // Clean wires, reliable store: the eager persist of the reclaimed
            // identity must succeed.
            match peer.bookmark(bm).await {
                Ok(peer) => peer,
                Err(_) => panic!("bookmark ok"),
            }
        },
        async move {
            let mut link = serve_side;
            server.gossip_once(&mut link).await
        },
    );
    serve_out.expect("serve bootstrap");
    boot_out.into_rumors()
}

/// Execute a reliable-recovery plan to its post-heal end state.
///
/// The fleet shares one network from the start, and the only departure from
/// [`run_plan`] is the crash guard: a crash that would extinguish the network —
/// leaving no live member for the victim to later reboot from — is skipped, so
/// that the precondition "every party which restarted eventually restores
/// itself" holds by construction. (Retirement never extinguishes the network:
/// the absorber stays live.)
fn run_reliable_plan(plan: Plan) -> World {
    let mut world = World::single_network(plan.n);
    for step in plan.steps {
        match step {
            Step::Send(i) => world.send(i),
            Step::Redact(i, k) => world.redact(i, k),
            Step::Gossip(i, j, fa, fb) => world.gossip(i, j, fa, fb),
            Step::Crash(i) => {
                if world.nodes[i].is_live() && world.other_live_in_network(i) {
                    world.crash(i);
                }
            }
            Step::Retire(i, j) => world.retire(i, j),
        }
    }
    world.heal();
    world
}

/// Negative control for the verifier itself: the log must reject the witness
/// of a recycled coordinate.
///
/// A mutant bookmark that handed out
/// the same causal coordinate twice would surface as two durable emissions in
/// one network with equal versions.
#[test]
#[should_panic(expected = "recycled version identifier")]
fn negative_control_recycled_durable_emission_panics() {
    let log = EmissionLog::default();
    let network = Peer::<Msg>::seed_rng(&mut ChaCha8Rng::seed_from_u64(NETWORK_SEED))
        .sync_window_floor()
        .network();
    let mut version = Version::new();
    version.tick(&Party::seed());

    log.promote(Emission {
        network,
        seq: 0,
        version: version.clone(),
    });
    log.promote(Emission {
        network,
        seq: 1,
        version,
    });
}

/// Negative control for the session error classifier: each error the
/// harness deems an unconditional crate bug fails the step it is reported
/// on, and an injected disruption does not.
#[test]
fn negative_control_classifier_rejects_codec_bugs() {
    /// Whether the classifier rejects a synthetic outcome.
    fn fires(error: Error<FlakyInMemoryBookmark>) -> bool {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            assert_not_codec_bug("negative control", &error)
        }))
        .err()
        .and_then(|payload| payload.downcast::<String>().ok())
        .is_some_and(|message| message.contains("a protocol, codec, or bookmark-format bug"))
    }
    let (mut link, remote) = rumors::link::memory();
    drop(remote);
    let replica = Peer::<u64>::seed().into_rumors();
    let failure = rumors::testing::run_to_quiescence(replica.gossip_once(&mut link))
        .expect("a closed link must not deadlock")
        .expect_err("a closed link cannot complete a session");
    let Error::Transport(transport) = failure else {
        panic!("a closed link must report a transport failure: {failure:?}");
    };
    let context = transport.context;
    for kind in [
        std::io::ErrorKind::InvalidData,
        std::io::ErrorKind::UnexpectedEof,
    ] {
        assert!(
            fires(Error::Protocol(ProtocolViolation {
                context,
                source: Box::new(std::io::Error::new(kind, "invalid complete record")),
            })),
            "a diagnostic I/O source cannot turn a violation into a transport cut"
        );
    }
    for bug in [
        Error::Bookmark(BookmarkIo::Format(rumors::FormatError::Truncated {
            len: 0,
        })),
        Error::LinkPoisoned,
    ] {
        assert!(fires(bug), "an unconditional crate bug must fail the step");
    }
    for disruption in [
        Error::Transport(transport),
        Error::Bookmark(BookmarkIo::Io(FlakyError::injected_write())),
    ] {
        assert!(
            !fires(disruption),
            "an injected disruption must not fail the step"
        );
    }
}

/// A message made durable by propagation survives its emitter's crash, a
/// rejoin from a peer that never saw it, and the emitter's reclaim and later
/// sends.
///
/// A sends `m` and gossips it to B; C sends three times without meeting `m`;
/// A crashes, revives from C (the lowest-index live member), gossips with C
/// so its bookmark update runs, and sends again. A bookmark that re-admits
/// every stored region on reboot has A' re-issue coordinates below `m`'s,
/// compared `Greater` to `m`'s version because A' carries C's ticks, and the
/// heal's sieve deletes `m` fleet-wide: the survival check catches that and
/// the version order cannot.
#[test]
fn reconstructed_reclaim_after_crash_keeps_durable_content() {
    let (a, b, c) = (2, 1, 0);
    let mut world = World::single_network(3);
    world.send(a);
    world.gossip(a, b, FaultPlan::NONE, FaultPlan::NONE);
    let network = world.nodes[b].network;
    assert!(
        world
            .emissions
            .contains_exact(network, 0, &world.leaf_version(b, 0)),
        "m is durable: it propagated to B",
    );
    for _ in 0..3 {
        world.send(c);
    }
    world.crash(a);
    world.gossip(a, c, FaultPlan::NONE, FaultPlan::NONE);
    // The shape discriminates only if A' rebooted from a peer that never
    // saw m; a change to how `revive` picks its server would retire it.
    assert!(
        !world.holds(a, 0),
        "A' revived from a peer that never saw m"
    );
    for _ in 0..4 {
        world.send(a);
    }
    world.heal();
    world.assert_healed();
    for k in [a, b, c] {
        assert!(
            world.holds(k, 0),
            "the never-redacted durable message m must survive the heal at node {k}",
        );
    }
}

/// Known-bad artifact for the survival checks, against unmodified
/// production code: a stale bookmark record makes a correct `reclaim`
/// destroy a durable message, and the checks must catch it.
///
/// The shape of [`reconstructed_reclaim_after_crash_keeps_durable_content`]
/// with one change: A's store is snapshotted right after the fleet forms
/// and put back after A's crash, so A' reboots from a record that predates
/// `m` (the lost-write hazard the bookmark's own docs describe). C's
/// frontier dominates that stale record, so A''s first update reclaims A's
/// old region, its sends re-issue coordinates below `m`'s, and the next
/// session with B sieves `m` out of it. That session is the one the
/// per-session check names, so the pin is on that check alone: without it,
/// `m` is live nowhere when the heal begins and the heal-window check would
/// pass.
#[test]
#[should_panic(expected = "before gossip 2<->1")]
fn known_bad_stale_record_destroys_durable_content() {
    let (a, b, c) = (2, 1, 0);
    let mut world = World::single_network(3);
    let stale = world.nodes[a].store.lock().unwrap().clone();
    world.send(a);
    world.gossip(a, b, FaultPlan::NONE, FaultPlan::NONE);
    for _ in 0..3 {
        world.send(c);
    }
    world.crash(a);
    *world.nodes[a].store.lock().unwrap() = stale;
    world.gossip(a, c, FaultPlan::NONE, FaultPlan::NONE);
    assert!(
        !world.holds(a, 0),
        "A' revived from a peer that never saw m"
    );
    for _ in 0..4 {
        world.send(a);
    }
    world.gossip(a, b, FaultPlan::NONE, FaultPlan::NONE);
    world.heal();
    world.assert_healed();
}

/// Negative control for the survival check: a message that left the fleet
/// fails the check unless the ledger accounts for it.
///
/// One peer redacts a propagated message and the heal carries the
/// redaction everywhere; with the ledger emptied the same converged fleet
/// fails the check naming the message, which is what a destroyed message or
/// an unrecorded redaction looks like.
#[test]
fn negative_control_unledgered_loss_fails_the_survival_check() {
    let mut world = World::single_network(2);
    world.send(0);
    world.gossip(0, 1, FaultPlan::NONE, FaultPlan::NONE);
    world.redact(0, 0);
    world.heal();
    world.assert_healed();
    assert!(
        !world.holds(0, 0) && !world.holds(1, 0),
        "the redaction reached every peer",
    );
    world.redacted.clear();
    let unledgered =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| world.assert_healed()));
    let message = unledgered
        .expect_err("with the ledger emptied, the redacted message reads as destroyed")
        .downcast::<String>()
        .expect("a formatted assertion message");
    assert!(
        message.contains("messages [0] were live"),
        "the survival check must name the missing message: {message}",
    );
}

proptest! {
    /// Under arbitrary interleavings of sends, redactions, faulted gossip,
    /// crashes, and retirements, the identity bookmark never recycles a
    /// version identifier:
    ///
    /// 1. within every network, no durable message's version is ever dominated
    ///    by, equal to, or in the causal past of an earlier durable one's
    ///    (checked as the durable set grows in [`EmissionLog::promote`], a
    ///    message becoming durable once it is persisted or reaches another peer);
    /// 2. after a clean heal, all surviving peers converge to identical content
    ///    and their live parties are pairwise disjoint;
    /// 3. no session both sides complete loses a message either side held
    ///    and no one redacted, and every message the winning network held
    ///    when the heal began, and never redacted, survives at every peer:
    ///    the recycle checked by its consequence
    ///    ([`World::assert_session_preserved`] and
    ///    [`World::assert_durable_content_survived`]).
    ///
    /// The fleet starts fragmented into per-peer networks and converges by
    /// the `(min_ticks, network)` tie-break, with each peer's bookmark reads
    /// and writes failing on a shrinkable schedule.
    #[test]
    fn bookmarking_never_recycles_a_version(plan in arb_plan()) {
        let world = run_plan(plan);
        world.assert_healed();
    }
}

// Historical shrunk counterexamples for the property above, preserved as
// explicit constructions: their committed seeds regenerate gossip fault
// plans through the strategy's cut range, so a range change re-maps the
// offsets and the seed no longer replays the case it pinned. The seed
// files stay committed; these constructions carry the counterexamples.

/// Reconstructed counterexample: both peers crash around a send, then the
/// crashed-and-rebooted peer retires into the sender.
///
/// The bookmark must not recycle a version across the crash/retire
/// pair, and the heal must still converge. The path is pinned: each crash
/// re-seeds its node (neither has a live member to reboot from), the
/// cross-network retire is skipped, and the heal collapses node 1 into
/// node 0, whose one send outranks a fresh universe.
#[test]
fn reconstructed_crash_pair_then_retire() {
    let world = run_plan(Plan {
        n: 2,
        steps: vec![
            Step::Crash(0),
            Step::Send(0),
            Step::Crash(1),
            Step::Retire(1, 0),
        ],
        read_faults: vec![vec![], vec![]],
        write_faults: vec![vec![], vec![]],
    });
    assert_eq!(
        world.path,
        vec![
            PathEvent::Reseeded(0),
            PathEvent::Reseeded(1),
            PathEvent::Bootstrap {
                newcomer: 1,
                server: 0,
                booted: true,
            },
        ],
        "the plan's path through revival and heal",
    );
    world.assert_healed();
}

/// Reconstructed counterexample: a send, one gossip cut in both directions
/// mid-frame, then a retirement, under bookmark read/write fail
/// schedules on every node.
///
/// Versions must survive the faulted persistence without recycling. The
/// path is pinned: the cut session still exchanges greetings, so the
/// mismatch between the two fresh peers resolves by network identifier
/// with node 1 the winner and node 2 re-bootstrapping into it; the retire
/// of 1 into 2 fails on the scheduled bookmark faults and hands node 1 back
/// intact, so nothing is re-seeded; the heal then collapses both into node
/// 0, whose send outranks every fresh universe.
#[test]
fn reconstructed_cut_gossip_then_retire_under_bookmark_faults() {
    let world = run_plan(Plan {
        n: 3,
        steps: vec![
            Step::Send(0),
            Step::Gossip(
                1,
                2,
                FaultPlan {
                    write_cut: Some(1324),
                    read_cut: None,
                    vanish: None,
                },
                FaultPlan {
                    write_cut: None,
                    read_cut: Some(134),
                    vanish: None,
                },
            ),
            Step::Retire(1, 2),
        ],
        read_faults: vec![
            vec![false, true, false, false, false, false],
            vec![false, true, false, true],
            vec![false],
        ],
        write_faults: vec![
            vec![],
            vec![false, false, false, true],
            vec![false, false, false, false, true, false],
        ],
    });
    assert_eq!(
        world.path,
        vec![
            PathEvent::Mismatch {
                winner: 1,
                loser: 2
            },
            PathEvent::Bootstrap {
                newcomer: 2,
                server: 1,
                booted: true,
            },
            PathEvent::Bootstrap {
                newcomer: 1,
                server: 0,
                booted: true,
            },
            PathEvent::Bootstrap {
                newcomer: 2,
                server: 0,
                booted: true,
            },
        ],
        "the plan's path through mismatch resolution, revival, and heal",
    );
    world.assert_healed();
}

proptest! {
    /// Under arbitrary crash/restart and retirement of a fleet that shares one
    /// network, bookmarking never leaks identity space:
    ///
    /// 1. **No region claimed twice.** After a clean heal, the live parties are
    ///    pairwise disjoint ([`World::assert_healed`]). A reboot that wrongly
    ///    re-owned a region it had donated — a retirement whose `slice` failed
    ///    to excise the donated party from the bookmark — would surface here as
    ///    the rebooted peer overlapping its absorber.
    /// 2. **No region claimed zero times.** Every region of the seed identity is
    ///    held by a live peer or checkpointed in some bookmark store
    ///    ([`World::assert_no_leak`]). A reboot that dropped a fragment it should
    ///    have reconstituted would surface here as a gap in the coverage of the
    ///    seed.
    ///
    /// Together these witness the retire-then-reboot contract: the donated
    /// region lives on in its absorber, and every fragment the donation did not
    /// excise reconstitutes the rebooted peer's remaining identity.
    ///
    /// Wires and bookmarks are *reliable*, so a party is never lost
    /// in transit nor a checkpoint lost in storage, and every crashed peer can
    /// always reboot from a surviving member.
    #[test]
    fn bookmarking_prevents_party_leakage(plan in arb_reliable_plan()) {
        let world = run_reliable_plan(plan);
        world.assert_healed();
        world.assert_no_leak();
    }
}
