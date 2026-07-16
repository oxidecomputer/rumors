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
//! incomparable, never `<=`). The id-region need not enter the comparison at all
//! — it would only rule out collisions the version order already forbids.
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
//! Unlike `disruption.rs`, this simulation runs on a *current-thread* runtime
//! with a fully deterministic, plan-driven schedule (each session is its own
//! `block_on`). The bug class is about the *ordering* of
//! emit/gossip/crash/retire/persist-fail events and the persistence-fault
//! sequence, not watch-channel thread races; determinism makes counterexamples
//! replay byte-for-byte, makes shrinking sound, and makes capturing each
//! message's emitted version race-free. Message ids and emission sequence
//! numbers are minted by the [`World`], so replay does not depend on any
//! process-global counter shared with other proptest cases.

mod common;

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use before::Party;
use proptest::prelude::*;
use rumors::{Error, Key, MERKLE_HASH_LEN, Network, Peer, Retire, Rumors, Version};

use crate::common::fault::{self, FaultPlan};
use crate::common::flaky::{DurableStore, FaultFeed, FlakyInMemoryBookmark, persisted_record};
use crate::common::sim::arb_fault;
use crate::common::wire::tokio_block_on as block_on;

/// The message payload: a simulation-unique id that is also the message's
/// emission sequence number, so a single per-[`World`] counter mints both at
/// once.
type Msg = u64;

/// Capacity for every in-memory duplex; the mirror protocol alternates within a
/// session, so a modest buffer suffices and exercises backpressure.
const DUPLEX_BUF: usize = 8 * 1024;

/// A hard ceiling on heal-phase full-mesh rounds, per peer. A correct fleet
/// reaches a fixed point in a handful; the cap turns a convergence bug into a
/// loud failure rather than a hang.
const MAX_HEAL_ROUNDS_PER_PEER: usize = 16;

// ---- the emission log: the property's witness -------------------------------

/// One emitted message: the network it entered, its real-time emission order,
/// and the event [`Version`] stamped on its leaf.
///
/// The [`Version`] alone is the whole identifier we guard. Within a single
/// network every party is a fork of one seed, so all versions are causally
/// comparable, and two *distinct* emissions are either ordered (one truly
/// after the other) or concurrent ([`None`]). A later emission can therefore be
/// `<=` an earlier one only by rolling backwards over a version the network
/// already durably held — exactly a recycle. (The emitting party's id-region is
/// deliberately *not* recorded: it would only ever rule out collisions the
/// version order already proves impossible.)
struct Emission {
    network: Network,
    seq: u64,
    version: Version,
}

/// The durable record of every emission that became **known to the network** —
/// persisted by its emitter's bookmark *or* propagated to another peer —
/// grouped by network and the judge of the causality property.
///
/// A purely local send that is lost to a crash before it is ever persisted or
/// reaches another peer never enters here: the network never knew it, so reusing
/// its version is not a recycle. Emissions are held *pending* on their node
/// ([`Node::pending`]) until [`secure`] promotes them here.
///
/// [`promote`]: EmissionLog::promote
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
/// emission's. Coverage is the bookmark's promise that this version survives a
/// crash, so it is the moment the emission becomes durable.
fn store_covers(record: &BTreeMap<Network, Vec<Version>>, emission: &Emission) -> bool {
    record
        .get(&emission.network)
        .is_some_and(|versions| versions.iter().any(|version| emission.version <= *version))
}

/// Decode the id-regions a node has durably checkpointed for `network`, via the
/// same Borsh round trip the bookmark itself makes. The dual of
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
    label: usize,
}

enum NodeState {
    Live(Rumors<Msg, FlakyInMemoryBookmark>),
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

/// The whole simulated world: the fleet and the shared emission log.
struct World {
    nodes: Vec<Node>,
    emissions: EmissionLog,
    next_seq: u64,
}

impl World {
    /// Build `n` nodes, each its own freshly-seeded universe.
    fn seed(n: usize, read_faults: Vec<Vec<bool>>, write_faults: Vec<Vec<bool>>) -> Self {
        let nodes = (0..n)
            .map(|label| {
                let store = Arc::new(Mutex::new(None));
                let faults = Arc::new(Mutex::new(FaultFeed::new(
                    read_faults[label].clone(),
                    write_faults[label].clone(),
                )));
                let bookmark = FlakyInMemoryBookmark::new(store.clone(), faults.clone(), label);
                let peer = block_on(Peer::<Msg>::seed().bookmark(bookmark))
                    .expect("a pristine seed attaches its bookmark without touching storage");
                let network = peer.network();
                Node {
                    state: NodeState::Live(peer.into_rumors()),
                    store,
                    faults,
                    network,
                    pending: Vec::new(),
                    label,
                }
            })
            .collect();
        World {
            nodes,
            emissions: EmissionLog::default(),
            next_seq: 0,
        }
    }

    /// Build `n` nodes that all share *one* network: node 0 seeds it, and nodes
    /// `1..n` bootstrap into it over clean wires. Every bookmark is reliable (an
    /// empty fault schedule never fails), the precondition the leakage property
    /// rests on.
    ///
    /// Unlike [`seed`](World::seed), which starts the fleet fragmented into
    /// per-peer universes and lets [`heal`](World::heal) force-collapse them, a
    /// single shared seed means the id-space is partitioned *once* and then only
    /// moves between peers by donation and reclaim. That is what makes coverage
    /// (`fold` of all live and checkpointed regions equals [`Party::seed`])
    /// meaningful: a region a recovery drops shows up as a genuine gap rather
    /// than being re-minted by a fresh fork.
    fn single_network(n: usize) -> Self {
        assert!(n >= 1, "a fleet needs at least one node");
        let reliable = || Arc::new(Mutex::new(FaultFeed::new(Vec::new(), Vec::new())));

        let store = Arc::new(Mutex::new(None));
        let faults = reliable();
        let bookmark = FlakyInMemoryBookmark::new(store.clone(), faults.clone(), 0);
        let peer = block_on(Peer::<Msg>::seed().bookmark(bookmark))
            .expect("a pristine seed attaches its bookmark without touching storage");
        let network = peer.network();
        let mut nodes = vec![Node {
            state: NodeState::Live(peer.into_rumors()),
            store,
            faults,
            network,
            pending: Vec::new(),
            label: 0,
        }];
        // The rest start dormant in node 0's network; the bootstraps below make
        // them live forks of its identity.
        nodes.extend((1..n).map(|label| Node {
            state: NodeState::Dormant,
            store: Arc::new(Mutex::new(None)),
            faults: reliable(),
            network,
            pending: Vec::new(),
            label,
        }));

        let mut world = World {
            nodes,
            emissions: EmissionLog::default(),
            next_seq: 0,
        };
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

    /// Whether some peer *other than* `who` is live in `who`'s network: a peer
    /// `who` could reboot from. The leakage simulation refuses any crash that
    /// would leave this false, so a restarted party always has a live member to
    /// reclaim its identity from — the precondition that "every party which
    /// restarted eventually restores itself" demands.
    fn other_live_in_network(&self, who: usize) -> bool {
        let network = self.nodes[who].network;
        (0..self.n())
            .any(|k| k != who && self.nodes[k].is_live() && self.nodes[k].network == network)
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
        rumors.send(id); // commits when the returned Batch drops, at the `;`
        // Read back the leaf's version. Under the current-thread schedule no
        // other task runs between the commit and here, so the lookup is
        // race-free and the just-sent unique id is present exactly once.
        let snapshot = rumors.snapshot();
        let mut version = None;
        for (_key, leaf_version, value) in snapshot.iter() {
            if **value == id {
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

    /// Redact one of `who`'s live messages, indexed mod the live count. Pure
    /// adversarial pressure: it advances the clock without a tracked emission.
    fn redact(&mut self, who: usize, which: usize) {
        self.revive(who);
        let Some(rumors) = self.nodes[who].live() else {
            return;
        };
        let snapshot = rumors.snapshot();
        let keys: Vec<Key> = snapshot.iter().map(|(key, _, _)| key).collect();
        if keys.is_empty() {
            return;
        }
        rumors.redact(keys[which % keys.len()]);
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
    }

    /// Secure what is now known to the network, then discard the rest: `who`'s
    /// in-memory state is about to vanish (a crash or a re-bootstrap), so any
    /// emission that was neither persisted nor seen by another peer is lost —
    /// never known to the network, and so no later reuse of its version is a
    /// recycle.
    fn secure_and_lose(&mut self, who: usize) {
        self.secure(who);
        self.nodes[who].pending.clear();
    }

    /// Drop `who`'s in-memory state, keeping its durable store and schedule: a
    /// transient crash. The next operation that targets it revives it.
    fn crash(&mut self, who: usize) {
        self.secure_and_lose(who);
        self.nodes[who].state = NodeState::Dormant;
    }

    /// Run one gossip session between `a` and `b` over a wire faulted per
    /// `fault_a`/`fault_b`. A cross-network pair surfaces
    /// [`Error::NetworkMismatch`] on at least one side; the loser of the
    /// `(min_ticks, network)` tie-break re-bootstraps into the winner. Any other
    /// error is an honest disruption that leaves both replicas unchanged.
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
        // Each side owns its faulted halves inside its own task, so when a wire
        // fault kills one side it returns and *drops* its halves, surfacing EOF
        // to the counterparty. A bare `join!` would instead hold both sides'
        // halves until both finished, deadlocking the survivor on a read that
        // never completes.
        let (out_a, out_b) = block_on(async {
            let (side_a, side_b) = tokio::io::duplex(DUPLEX_BUF);
            let (mut ar, mut aw) = fault::faulty(side_a, fault_a);
            let (mut br, mut bw) = fault::faulty(side_b, fault_b);
            let task_a = tokio::spawn(async move { ra.gossip(&mut ar, &mut aw).await });
            let task_b = tokio::spawn(async move { rb.gossip(&mut br, &mut bw).await });
            let (out_a, out_b) = tokio::join!(task_a, task_b);
            (out_a.expect("gossip task a"), out_b.expect("gossip task b"))
        });

        // The session ran each side's bookmark update before any mismatch, so
        // secure both: their pre-session emissions are now durable.
        self.secure(a);
        self.secure(b);

        let mismatched = matches!(out_a, Err(Error::NetworkMismatch { .. }))
            || matches!(out_b, Err(Error::NetworkMismatch { .. }));
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
        self.bootstrap_into(loser, winner);
    }

    /// This live peer's tie-break key: its frontier's minimum event count, then
    /// its network identifier. `None` while dormant.
    fn tuple(&self, who: usize) -> Option<(u64, Network)> {
        let rumors = self.nodes[who].live()?;
        Some((rumors.snapshot().latest().min_ticks(), rumors.network()))
    }

    /// (Re)create `who` as a fresh replica in `server`'s network by bootstrapping
    /// from it over a clean wire, reusing `who`'s durable store (so it reclaims
    /// any of its own identity the pulled frontier now dominates) and its still-
    /// active fault schedule. Returns whether `who` is live afterwards.
    ///
    /// The bootstrap's wire is clean so the fleet can make progress, but the
    /// bookmarks on both sides stay flaky: the server's donating `slice`/`write`
    /// and `who`'s eager identity persist can each fail, which is precisely the
    /// adversarial persistence path. On any failure `who` is left dormant for a
    /// later attempt.
    fn bootstrap_into(&mut self, who: usize, server: usize) -> bool {
        let Some(server_rumors) = self.nodes[server].live() else {
            return false;
        };
        let server_rumors = server_rumors.clone();
        // The prior incarnation's memory is about to vanish: secure what its
        // store persisted, lose the rest.
        self.secure_and_lose(who);
        let bookmark = self.nodes[who].bookmark();
        // Drop any prior incarnation before minting the new one.
        self.nodes[who].state = NodeState::Dormant;

        let booted = block_on(async {
            let (boot_side, serve_side) = tokio::io::duplex(DUPLEX_BUF);
            let (mut boot_r, mut boot_w) = tokio::io::split(boot_side);
            let (mut serve_r, mut serve_w) = tokio::io::split(serve_side);
            // Spawn both sides so a failing one drops its halves (see `gossip`).
            let boot = tokio::spawn(async move {
                // `Ok(None)` cannot happen (the server is gossiping, not
                // bootstrapping); a wire fault drops us to `None`, as does an
                // injected persistence failure in the eager bookmark attach.
                let peer = Peer::<Msg>::bootstrap(&mut boot_r, &mut boot_w)
                    .await
                    .ok()
                    .flatten()?;
                peer.bookmark(bookmark).await.ok()
            });
            let serve =
                tokio::spawn(async move { server_rumors.gossip(&mut serve_r, &mut serve_w).await });
            let (boot_out, serve_out) = tokio::join!(boot, serve);
            let _ = serve_out;
            boot_out.expect("bootstrap task")
        });

        match booted {
            Some(peer) => {
                self.nodes[who].network = peer.network();
                self.nodes[who].state = NodeState::Live(peer.into_rumors());
                // The eager bootstrap persist secures `who`'s reclaimed identity;
                // the server's donating persist secures its emissions too.
                self.secure(who);
                self.secure(server);
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
        // failed): start fresh. Its old network's identity is left stranded —
        // a harmless leak, never a corruption. The old incarnation's memory
        // vanishes, so secure what was persisted and lose the rest.
        self.secure_and_lose(who);
        let bookmark = self.nodes[who].bookmark();
        let peer = block_on(Peer::<Msg>::seed().bookmark(bookmark))
            .expect("a pristine seed attaches its bookmark without touching storage");
        self.nodes[who].network = peer.network();
        self.nodes[who].state = NodeState::Live(peer.into_rumors());
    }

    /// Retire `retiree` into `absorber`, donating its identity. Same-network
    /// only: a cross-network retire cannot be absorbed, so it is skipped. The
    /// retiree's durable store may later resurrect it — exercising party reuse
    /// across a donation, where a failed `slice` would let the donated region
    /// live twice.
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
        // Take the retiree's sole handle so it can become a `Peer` immediately.
        let NodeState::Live(retiree_rumors) =
            std::mem::replace(&mut self.nodes[retiree].state, NodeState::Dormant)
        else {
            return;
        };

        let outcome = block_on(async {
            let (ret_side, abs_side) = tokio::io::duplex(DUPLEX_BUF);
            let (mut ret_r, mut ret_w) = tokio::io::split(ret_side);
            let (mut abs_r, mut abs_w) = tokio::io::split(abs_side);
            // Spawn both sides so a failing one drops its halves (see `gossip`).
            // The retiree becomes a `Peer` inside its task: it holds the sole
            // handle to its set, so `try_into_peer` resolves at once.
            let retire = tokio::spawn(async move {
                let peer = retiree_rumors
                    .try_into_peer()
                    .await
                    .expect("the node holds the sole handle to its set");
                peer.retire(&mut ret_r, &mut ret_w).await
            });
            let absorb =
                tokio::spawn(async move { absorber_rumors.gossip(&mut abs_r, &mut abs_w).await });
            let (retire_out, gossip_out) = tokio::join!(retire, absorb);
            (
                retire_out.expect("retire task"),
                gossip_out.expect("absorb task"),
            )
        });
        let (outcome, absorbed) = outcome;
        // Never swallow the absorber's result: a retirement's whole point is the
        // hand-off, and silently dropping a failed absorption is exactly what hid
        // the codec leak this test was written to catch. The retire session runs
        // over a *clean* wire, so the absorber can only fail honestly by an
        // injected bookmark fault (`Error::Bookmark`) or by the retiree safely
        // aborting its own bookmark fault and closing the wire (`UnexpectedEof`).
        // A decode failure (`InvalidData`) means a fully-received frame was
        // malformed — a protocol/codec bug like the non-canonical party that
        // motivated this check — so surface it loudly.
        if let Err(error) = &absorbed {
            let codec_bug = matches!(
                error,
                Error::Io(io) if io.kind() == std::io::ErrorKind::InvalidData,
            );
            assert!(
                !codec_bug,
                "retire absorber failed to decode on a clean wire: a protocol/codec bug, \
                 not an honest disruption: {error:?}",
            );
        }

        match outcome {
            // Donated: the retiree's memory is consumed, so secure what it
            // persisted and lose the rest.
            Retire::Retired | Retire::Uncertain { .. } => self.secure_and_lose(retiree),
            // Unchanged: hand the intact peer back to life; its persisted
            // emissions are durable, its unpersisted ones remain pending.
            Retire::Declined { peer } | Retire::Recovered { peer, .. } => {
                self.nodes[retiree].state = NodeState::Live(peer.into_rumors());
                self.secure(retiree);
            }
        }
        self.secure(absorber);
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
        let (out_a, out_b) = block_on(async {
            let (side_a, side_b) = tokio::io::duplex(DUPLEX_BUF);
            let (mut ar, mut aw) = tokio::io::split(side_a);
            let (mut br, mut bw) = tokio::io::split(side_b);
            let task_a = tokio::spawn(async move { ra.gossip(&mut ar, &mut aw).await });
            let task_b = tokio::spawn(async move { rb.gossip(&mut br, &mut bw).await });
            let (out_a, out_b) = tokio::join!(task_a, task_b);
            (
                out_a.expect("heal gossip task A"),
                out_b.expect("heal gossip task B"),
            )
        });
        out_a.expect("clean heal gossip A");
        out_b.expect("clean heal gossip B");
        self.secure(a);
        self.secure(b);
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

    /// After a clean heal: every live peer holds identical content (equal hash
    /// and frontier), and their live parties are pairwise disjoint.
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
            .map(|&k| {
                self.nodes[k]
                    .live()
                    .unwrap()
                    .dangerously_alias_party()
                    .expect("a live peer holds its party")
            })
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
    }

    /// Verify the recycle assertion is not passing vacuously: after the
    /// fault-free heal, every surviving live message must have an exact durable
    /// emission witness, because the converged fleet has now persisted and
    /// propagated all surviving content.
    fn assert_live_content_is_durable(&self, live: &[usize]) {
        let mut live_leaves = 0;
        for &k in live {
            let rumors = self.nodes[k].live().unwrap();
            let network = rumors.network();
            let snapshot = rumors.snapshot();
            for (_key, leaf_version, value) in snapshot.iter() {
                live_leaves += 1;
                let seq = **value;
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
                held.push(
                    rumors
                        .dangerously_alias_party()
                        .expect("a live peer holds its party"),
                );
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
/// adversity is crash/restart and retirement. Crash and retire weigh heavier
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
/// from a bookmark) must absorb cleanly, leaving the absorber holding the whole
/// seed identity.
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
        let a = Peer::<Msg>::seed()
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
        let pa = a.dangerously_alias_party().expect("A live");
        let pb = b.dangerously_alias_party().expect("B live");
        assert!(
            pa.is_disjoint(&pb),
            "A {pa:?} and B {pb:?} must be disjoint"
        );

        // B retires into A. A's gossip must absorb B's party so that A ends up
        // holding the whole seed identity.
        let (ret_side, abs_side) = tokio::io::duplex(DUPLEX_BUF);
        let (mut ret_r, mut ret_w) = tokio::io::split(ret_side);
        let (mut abs_r, mut abs_w) = tokio::io::split(abs_side);
        let absorber = a.clone();
        let retire = tokio::spawn(async move {
            let peer = b.try_into_peer().await.expect("sole handle");
            peer.retire(&mut ret_r, &mut ret_w).await
        });
        let absorb = tokio::spawn(async move { absorber.gossip(&mut abs_r, &mut abs_w).await });
        let (retire_out, absorb_out) = tokio::join!(retire, absorb);

        assert!(
            matches!(retire_out.expect("retire task"), Retire::Retired),
            "the retiree should have retired",
        );
        absorb_out
            .expect("absorb task")
            .expect("the absorber's gossip must not fail while taking a retirement");
        assert_eq!(
            a.dangerously_alias_party().expect("A live"),
            Party::seed(),
            "after absorbing the only other peer, A must hold the whole seed identity",
        );
    });
}

/// Bootstrap a fresh peer with bookmark `bm` from `server` over a clean
/// in-process duplex, returning the booted peer's [`Rumors`]. Diagnostic helper.
async fn boot_from_async(
    server: &Rumors<Msg, FlakyInMemoryBookmark>,
    bm: FlakyInMemoryBookmark,
) -> Rumors<Msg, FlakyInMemoryBookmark> {
    let server = server.clone();
    let (boot_side, serve_side) = tokio::io::duplex(DUPLEX_BUF);
    let (mut boot_r, mut boot_w) = tokio::io::split(boot_side);
    let (mut serve_r, mut serve_w) = tokio::io::split(serve_side);
    let boot = tokio::spawn(async move {
        let peer = Peer::<Msg>::bootstrap(&mut boot_r, &mut boot_w)
            .await
            .expect("bootstrap ok")
            .expect("got a peer");
        // Clean wires, reliable store: the eager persist of the reclaimed
        // identity must succeed.
        match peer.bookmark(bm).await {
            Ok(peer) => peer,
            Err(_) => panic!("bookmark ok"),
        }
    });
    let serve = tokio::spawn(async move { server.gossip(&mut serve_r, &mut serve_w).await });
    let (boot_out, serve_out) = tokio::join!(boot, serve);
    serve_out.unwrap().expect("serve bootstrap");
    boot_out.unwrap().into_rumors()
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

/// Negative control for the verifier itself: a mutant bookmark that handed out
/// the same causal coordinate twice would surface as two durable emissions in
/// one network with equal versions, and the log must reject that witness.
#[test]
#[should_panic(expected = "recycled version identifier")]
fn negative_control_recycled_durable_emission_panics() {
    let log = EmissionLog::default();
    let network = Peer::<Msg>::seed().network();
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

proptest! {
    /// Under arbitrary interleavings of sends, redactions, faulted gossip,
    /// crashes, and retirements across a fleet that starts fragmented into
    /// per-peer networks and converges by the `(min_ticks, network)` tie-break
    /// — with each peer's bookmark reads and writes failing on a shrinkable
    /// schedule — the identity bookmark never recycles a version identifier:
    ///
    /// 1. within every network, no durable message's version is ever dominated
    ///    by, equal to, or in the causal past of an earlier durable one's
    ///    (checked as the durable set grows in [`EmissionLog::promote`], a
    ///    message becoming durable once it is persisted or reaches another peer);
    /// 2. after a clean heal, all surviving peers converge to identical content
    ///    and their live parties are pairwise disjoint.
    #[test]
    fn bookmarking_never_recycles_a_version(plan in arb_plan()) {
        let world = run_plan(plan);
        world.assert_healed();
    }
}

proptest! {
    /// Under arbitrary crash/restart and retirement of a fleet that shares one
    /// network — with *reliable* wires and bookmarks, so a party is never lost
    /// in transit nor a checkpoint lost in storage, and every crashed peer can
    /// always reboot from a surviving member — bookmarking never leaks identity
    /// space:
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
    #[test]
    fn bookmarking_prevents_party_leakage(plan in arb_reliable_plan()) {
        let world = run_reliable_plan(plan);
        world.assert_healed();
        world.assert_no_leak();
    }
}
