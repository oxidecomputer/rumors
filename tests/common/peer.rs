//! A simulated peer: a `Rumors<T>` paired with its observation log, plus
//! helpers for the schedule executor (`gossip_step` for one bidirectional
//! wire gossip session, `quiesce` for full-mesh convergence to a fixed
//! point).
//!
//! Observation is pull-based, mirroring the `UnorderedMessages` observer
//! one pass at a time: a [`drain`](Peer::drain) snapshots the peer and
//! records exactly the live leaves its causal checkpoint does not contain
//! — local sends and gossip-learned messages alike — then absorbs the
//! snapshot's ceiling.
//! Every helper drains after the operation it performs, so the log stays in
//! event order and a message redacted before it was ever drained is never
//! observed, matching both the `UnorderedMessages` delivery contract and
//! the shadow simulator's model in `schedule::arb`.

use borsh::{BorshDeserialize, BorshSerialize};
use rumors::{Rumors, Version, causally};

use crate::common::wire::{block_on, wire_gossip_async};

/// One simulated peer.
pub struct Peer<T> {
    pub local: Rumors<T>,
    /// The causal frontier up to which `observations` is complete: each
    /// drain records the live leaves not contained here, then absorbs the
    /// snapshot's ceiling (so redaction ticks, which have no leaves, are
    /// covered too).
    checkpoint: Version,
    /// All observations this peer has accumulated, across `insert_one`,
    /// `gossip_step`, and `quiesce` calls.
    ///
    /// Drain order within a pass is the tree's iteration order; in practice
    /// it is deterministic across runs, so the log is reproducible inside a
    /// counterexample.
    pub observations: Vec<(Version, T)>,
}

impl<T: Clone + BorshSerialize + BorshDeserialize + Send + Sync + 'static> Peer<T> {
    /// Wrap an already-forked `Rumors` as a simulated peer. Observation
    /// starts at the wrapped set's current frontier: content already present
    /// is never logged, only what arrives afterwards.
    ///
    /// The caller must mint `local` by bootstrapping from the shared
    /// universe seed (directly, or via another peer), never by an
    /// independent [`rumors::Peer::seed`]: only then are all peers pairwise
    /// disjoint, the precondition for [`gossip_step`] to succeed.
    pub fn new(local: Rumors<T>) -> Self {
        let checkpoint = local.snapshot().latest().clone();
        Self {
            local,
            checkpoint,
            observations: Vec::new(),
        }
    }

    /// Snapshot of the observation log, in insertion order. Convenience
    /// for tests that read out `peer.observations` for assertions.
    pub fn observations(&self) -> Vec<(Version, T)> {
        self.observations.clone()
    }

    /// Record every live message the checkpoint does not causally contain,
    /// then absorb the snapshot's ceiling. Returns how many were new.
    pub fn drain(&mut self) -> usize {
        let snapshot = self.local.snapshot();
        let mut new = 0;
        for (version, message) in snapshot.range(causally::since(&self.checkpoint)) {
            self.observations
                .push((version.clone(), (**message).clone()));
            new += 1;
        }
        self.checkpoint |= snapshot.latest();
        new
    }

    /// Insert a single value, returning the [`Version`] minted for it.
    pub fn insert_one(&mut self, value: T) -> Version {
        // Catch the log up first, so the send's drain isolates exactly the
        // one new observation and its version.
        self.drain();
        self.local.send(value);
        let pre = self.observations.len();
        let drained = self.drain();
        assert_eq!(drained, 1, "a send mints exactly one new observation");
        self.observations[pre].0.clone()
    }

    pub fn redact_one(&mut self, version: &Version) {
        self.local.redact(version);
        // Redactions fire no observation; the drain just absorbs the
        // version tick into the checkpoint.
        self.drain();
    }
}

/// Bidirectional wire gossip between two peers: one session over an
/// in-memory link, after which both sides hold the same live content and
/// version, and both observation logs have caught up.
pub fn gossip_step<T>(a: &mut Peer<T>, b: &mut Peer<T>)
where
    T: Clone + BorshSerialize + BorshDeserialize + Send + Sync + 'static,
{
    block_on(wire_gossip_async(&a.local, &b.local));
    a.drain();
    b.drain();
}

/// Drive every peer to a full-mesh fixed point.
///
/// See [`quiesce_refs`] for the fixed-point criterion and the
/// non-termination guard.
pub fn quiesce<T>(peers: &mut [Peer<T>])
where
    T: Clone + Eq + BorshSerialize + BorshDeserialize + Send + Sync + 'static,
{
    let mut refs: Vec<&mut Peer<T>> = peers.iter_mut().collect();
    quiesce_refs(&mut refs);
}

/// Drive every live slot to a full-mesh fixed point: [`quiesce`] over a
/// slotted fleet, skipping retired peers' vacated slots.
pub fn quiesce_slots<T>(slots: &mut [Option<Peer<T>>])
where
    T: Clone + Eq + BorshSerialize + BorshDeserialize + Send + Sync + 'static,
{
    let mut refs: Vec<&mut Peer<T>> = slots.iter_mut().filter_map(Option::as_mut).collect();
    quiesce_refs(&mut refs);
}

/// The convergence core: full-mesh `gossip_step` rounds until every peer
/// reports one fingerprint (live-content hash plus causal version).
///
/// Identical fingerprints are the fixed point itself — peers with equal
/// content and version exchange nothing — so the loop stops the moment a
/// round ends uniform instead of spending a further full mesh round to
/// confirm that nothing changes. A bounded outer loop guards against
/// pathological non-termination (which would itself be a bug the test
/// should catch).
fn quiesce_refs<T>(peers: &mut [&mut Peer<T>])
where
    T: Clone + Eq + BorshSerialize + BorshDeserialize + Send + Sync + 'static,
{
    let n = peers.len();
    if n < 2 {
        return;
    }

    let fingerprint = |peer: &Peer<T>| {
        let snapshot = peer.local.snapshot();
        (snapshot.hash(), snapshot.latest().clone())
    };

    let max_rounds = MAX_QUIESCE_ROUNDS_PER_PEER * n;
    for _ in 0..max_rounds {
        let first: ([u8; rumors::MERKLE_HASH_LEN], Version) = fingerprint(peers[0]);
        if peers[1..].iter().all(|p| fingerprint(p) == first) {
            return;
        }

        for i in 0..n {
            for j in (i + 1)..n {
                let (left, right) = peers.split_at_mut(j);
                gossip_step(left[i], right[0]);
            }
        }
    }

    panic!(
        "quiesce did not converge within {max_rounds} rounds for {n} peers: \
         a propagation or shadow-simulator bug (schedules generated by \
         `arb_schedule` are convergent by construction)"
    );
}

/// Headroom on the convergence loop, used only to bound test pathologies.
///
/// A single piece of information needs at most O(diameter) rounds to reach
/// every peer over a full-mesh schedule, so 16 rounds per peer is
/// dramatically more than enough.
const MAX_QUIESCE_ROUNDS_PER_PEER: usize = 16;
