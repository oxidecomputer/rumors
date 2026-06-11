//! A simulated peer: a `Known<T>` paired with its observation log, plus
//! helpers for the schedule executor (`gossip_step` for one bidirectional
//! wire gossip session, `quiesce` for full-mesh convergence to a fixed
//! point).
//!
//! Observation is pull-based, mirroring the `Messages` observer one pass at
//! a time: a [`drain`](Peer::drain) snapshots the peer and records exactly
//! the live leaves its causal cursor does not contain — local sends and
//! gossip-learned messages alike — then absorbs the snapshot's ceiling.
//! Every helper drains after the operation it performs, so the log stays in
//! event order and a message redacted before it was ever drained is never
//! observed, matching both the `Messages` delivery contract and the shadow
//! simulator's model in `schedule::arb`.

use borsh::{BorshDeserialize, BorshSerialize};
use rumors::{Key, Known, Version, causally};

use crate::common::wire::{block_on, wire_gossip_async};

/// One simulated peer.
pub struct Peer<T> {
    pub local: Known<T>,
    /// The causal frontier up to which `observations` is complete: each
    /// drain records the live leaves not contained here, then absorbs the
    /// snapshot's ceiling (so redaction ticks, which have no leaves, are
    /// covered too).
    cursor: Version,
    /// All observations this peer has accumulated, across `insert_one`,
    /// `gossip_step`, and `quiesce` calls. Drain order within a pass is the
    /// tree's iteration order; in practice it is deterministic across runs,
    /// so the log is reproducible inside a counterexample.
    pub observations: Vec<(Key, Version, T)>,
}

impl<T: Clone + BorshSerialize + BorshDeserialize + Send + Sync + 'static> Peer<T> {
    /// Wrap an already-forked `Known` as a simulated peer. Observation
    /// starts at the wrapped set's current frontier: content already present
    /// is never logged, only what arrives afterwards.
    ///
    /// The caller must mint `local` by bootstrapping from the shared
    /// universe seed (directly, or via another peer), never by an
    /// independent [`Known::seed`]: only then are all peers pairwise
    /// disjoint, the precondition for [`gossip_step`] to succeed.
    pub fn new(local: Known<T>) -> Self {
        let cursor = local.latest();
        Self {
            local,
            cursor,
            observations: Vec::new(),
        }
    }

    /// Snapshot of the observation log, in insertion order. Convenience
    /// for tests that read out `peer.observations` for assertions.
    pub fn observations(&self) -> Vec<(Key, Version, T)> {
        self.observations.clone()
    }

    /// Record every live message the cursor does not causally contain,
    /// then absorb the snapshot's ceiling. Returns how many were new.
    pub fn drain(&mut self) -> usize {
        let snapshot = self.local.snapshot();
        let mut new = 0;
        for (key, version, message) in snapshot.range(causally::since(&self.cursor)) {
            self.observations
                .push((key, version.clone(), (**message).clone()));
            new += 1;
        }
        self.cursor |= snapshot.latest();
        new
    }

    /// Insert a single value, returning the `Key` minted for it.
    pub fn insert_one(&mut self, value: T) -> Key {
        // Catch the log up first, so the send's drain isolates exactly the
        // one new observation and its key.
        self.drain();
        self.local.send(value);
        let pre = self.observations.len();
        let drained = self.drain();
        assert_eq!(drained, 1, "a send mints exactly one new observation");
        self.observations[pre].0
    }

    pub fn redact_one(&mut self, key: Key) {
        self.local.redact(key);
        // Redactions fire no observation; the drain just absorbs the
        // version tick into the cursor.
        self.drain();
    }
}

/// Bidirectional wire gossip between two peers: one session over an
/// in-memory duplex, after which both sides hold the same live content and
/// version, and both observation logs have caught up.
pub fn gossip_step<T>(a: &mut Peer<T>, b: &mut Peer<T>)
where
    T: Clone + BorshSerialize + BorshDeserialize + Send + Sync + 'static,
{
    block_on(wire_gossip_async(&mut a.local, &mut b.local));
    a.drain();
    b.drain();
}

/// Drive every pair toward convergence by repeatedly running
/// `gossip_step` over all pairs in a fixed order until no peer's
/// live content (`hash`) or causal version (`latest`) changes for a
/// full round. A bounded outer loop guards against pathological
/// non-termination (which would itself be a bug the test should catch).
pub fn quiesce<T>(peers: &mut [Peer<T>])
where
    T: Clone + Eq + BorshSerialize + BorshDeserialize + Send + Sync + 'static,
{
    let n = peers.len();
    if n < 2 {
        return;
    }

    let max_rounds = MAX_QUIESCE_ROUNDS_PER_PEER * n;
    for _ in 0..max_rounds {
        let before: Vec<([u8; 32], Version)> = peers
            .iter()
            .map(|p| (p.local.hash(), p.local.latest()))
            .collect();

        for i in 0..n {
            for j in (i + 1)..n {
                let (left, right) = peers.split_at_mut(j);
                gossip_step(&mut left[i], &mut right[0]);
            }
        }

        let changed = peers
            .iter()
            .zip(before.iter())
            .any(|(p, (hash, latest))| p.local.hash() != *hash || p.local.latest() != *latest);
        if !changed {
            return;
        }
    }

    panic!(
        "quiesce did not converge within {max_rounds} rounds for {n} peers: \
         a propagation or shadow-simulator bug (schedules generated by \
         `arb_schedule` are convergent by construction)"
    );
}

/// Headroom on the convergence loop: a single piece of information
/// needs at most O(diameter) rounds to reach every peer over a
/// full-mesh schedule, so 16 rounds per peer is dramatically more than
/// enough. Used only to bound test pathologies.
const MAX_QUIESCE_ROUNDS_PER_PEER: usize = 16;
