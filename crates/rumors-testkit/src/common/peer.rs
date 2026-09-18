//! A simulated peer: a `Rumors<T>` paired with its observation log, plus
//! helpers for the schedule executor (`gossip_step` for one bidirectional
//! wire gossip session, `quiesce` for full-mesh convergence to a fixed
//! point).
//!
//! Each peer owns a real `UnorderedMessages` observer. Every helper drains it
//! after the operation it performs, so the log stays in event order and the
//! generated schedules exercise the public observation API alongside gossip.
//! A message redacted before a drain is never observed, matching both the
//! observer's contract and the shadow simulator's model in `schedule::arb`.

use rumors::{Rumors, TryNext, UnorderedMessages, Version};

use crate::common::wire::{block_on, wire_gossip_async};

use serde::Serialize;
use serde::de::DeserializeOwned;

/// One simulated peer.
pub struct Peer<T: Send + Sync + 'static> {
    /// The peer's live Rumors handle.
    pub local: Rumors<T>,
    /// The public observer drained into `observations` after each operation.
    observer: UnorderedMessages<T>,
    /// All observations this peer has accumulated, across `insert_one`,
    /// `gossip_step`, and `quiesce` calls.
    ///
    /// Drain order within a pass is the tree's iteration order; in practice
    /// it is deterministic across runs, so the log is reproducible inside a
    /// counterexample.
    pub observations: Vec<(Version, T)>,
}

impl<T: Clone + Serialize + DeserializeOwned + Eq + Send + Sync + 'static> Peer<T> {
    /// Wrap an already-forked `Rumors` as a simulated peer. Observation
    /// starts at the wrapped set's current frontier: content already present
    /// is never logged, only what arrives afterwards.
    ///
    /// The caller must create `local` by bootstrapping from the shared
    /// universe seed (directly, or via another peer), never by an
    /// independent [`rumors::Peer::seed`]: only then are all peers pairwise
    /// disjoint, the precondition for [`gossip_step`] to succeed.
    pub fn new(local: Rumors<T>) -> Self {
        let since = local.snapshot().latest().clone();
        let observer = local.unordered_messages_since(since);
        Self {
            local,
            observer,
            observations: Vec::new(),
        }
    }

    /// Snapshot of the observation log, in insertion order. Convenience
    /// for tests that read out `peer.observations` for assertions.
    pub fn observations(&self) -> Vec<(Version, T)> {
        self.observations.clone()
    }

    /// Drain every message currently ready from the public observer.
    ///
    /// Returns the number of messages appended to the observation log.
    pub fn drain(&mut self) -> usize {
        let mut new = 0;
        loop {
            match self.observer.try_next() {
                TryNext::Message((version, message)) => {
                    self.observations.push((version, (*message).clone()));
                    new += 1;
                }
                TryNext::Quiet => return new,
                TryNext::Ended => {
                    panic!("the observer cannot end while its Rumors handle is alive")
                }
            }
        }
    }

    /// Insert a single value, returning the [`Version`] created for it.
    pub fn insert_one(&mut self, value: T) -> Version {
        // Catch the log up first, so the next drain isolates this send.
        self.drain();
        let version = self.local.send(value).unwrap();
        let pre = self.observations.len();
        let drained = self.drain();
        assert_eq!(drained, 1, "a send creates exactly one new observation");
        assert_eq!(self.observations[pre].0, version);
        version
    }

    /// Redact one message and advance the observer past the resulting change.
    pub fn redact_one(&mut self, version: &Version) {
        self.local.redact(version);
        // Redactions yield no message, but draining completes the observer's
        // empty pass so its checkpoint covers the change.
        self.drain();
    }
}

/// Bidirectional wire gossip between two peers: one session over an
/// in-memory link, after which both sides hold the same live content and
/// version, and both observation logs have caught up.
pub fn gossip_step<T>(a: &mut Peer<T>, b: &mut Peer<T>)
where
    T: Clone + Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
{
    block_on(wire_gossip_async(&a.local, &b.local));
    a.drain();
    b.drain();
}

/// Drive every peer to a full-mesh fixed point.
///
/// See `quiesce_refs` for the fixed-point criterion and the
/// non-termination guard.
pub fn quiesce<T>(peers: &mut [Peer<T>])
where
    T: Clone + Eq + Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
{
    let mut refs: Vec<&mut Peer<T>> = peers.iter_mut().collect();
    quiesce_refs(&mut refs);
}

/// Drive every live slot to a full-mesh fixed point: [`quiesce`] over a
/// slotted fleet, skipping retired peers' vacated slots.
pub fn quiesce_slots<T>(slots: &mut [Option<Peer<T>>])
where
    T: Clone + Eq + Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
{
    let mut refs: Vec<&mut Peer<T>> = slots.iter_mut().filter_map(Option::as_mut).collect();
    quiesce_refs(&mut refs);
}

/// The convergence core: full-mesh `gossip_step` rounds until every peer
/// reports the same snapshot.
///
/// Identical fingerprints are the fixed point itself — peers with equal
/// content and version exchange nothing — so the loop stops the moment a
/// round ends uniform instead of spending a further full mesh round to
/// confirm that nothing changes. A bounded outer loop guards against
/// pathological non-termination (which would itself be a bug the test
/// should catch).
fn quiesce_refs<T>(peers: &mut [&mut Peer<T>])
where
    T: Clone + Eq + Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
{
    let n = peers.len();
    if n < 2 {
        return;
    }

    let fingerprint = |peer: &Peer<T>| peer.local.snapshot();

    let max_rounds = MAX_QUIESCE_ROUNDS_PER_PEER * n;
    for _ in 0..max_rounds {
        let first = fingerprint(peers[0]);
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
