//! A simulated peer: a `Known<T>` paired with its observation log, plus
//! helpers for the schedule executor (`gossip_step` for bidirectional
//! `Known::learn`, `quiesce` for full-mesh convergence to a fixed
//! point).

use std::sync::{Arc, Mutex};

use borsh::{BorshDeserialize, BorshSerialize};
use rumors::sync::Known;
use rumors::{Key, Version};

/// One simulated peer.
///
/// The observation log is held behind an `Arc<Mutex<_>>` because the
/// sync API's callback bounds are `FnMut(...) + Send + 'static` (so the
/// caller can drive gossip on a spawned thread). A `&mut self.observations`
/// borrow would force a `'static` reference, which `&mut` can't satisfy;
/// each helper instead clones the `Arc` into its closure and locks it on
/// each callback invocation.
pub struct Peer<T> {
    pub local: Known<T>,
    /// All observations this peer has accumulated, across `message`,
    /// `redact`, and `learn` calls. The public API contract says
    /// callback order within a batch is arbitrary; in practice it is
    /// deterministic across runs because the underlying tree is an
    /// `imbl::OrdMap`, so the log is reproducible inside a counterexample.
    pub observations: Arc<Mutex<Vec<(Key, Version, T)>>>,
}

impl<T: Clone + BorshSerialize + BorshDeserialize + Send + Sync + 'static> Peer<T> {
    /// Wrap an already-forked `Known` as a simulated peer.
    ///
    /// The caller must mint `local` by [`fork`](Known::fork)ing the shared
    /// universe seed (directly, or via another peer), never by an independent
    /// [`Known::seed`]: only then are all peers pairwise disjoint, the
    /// precondition for [`gossip_step`] to succeed.
    pub fn new(local: Known<T>) -> Self {
        Self {
            local,
            observations: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Snapshot of the observation log, in insertion order. Convenience
    /// for tests that read out `peer.observations` for assertions.
    pub fn observations(&self) -> Vec<(Key, Version, T)> {
        self.observations.lock().unwrap().clone()
    }

    /// Insert a single value, returning the `Key` minted for it.
    pub fn insert_one(&mut self, value: T) -> Key {
        // Both the observation log and the produced `Key` cross into the
        // `Send + 'static` callback via `Arc<Mutex<_>>` clones; the outer
        // function unwraps the single remaining reference after the call
        // returns.
        let produced: Arc<Mutex<Option<Key>>> = Arc::new(Mutex::new(None));
        let observations = Arc::clone(&self.observations);
        let produced_in = Arc::clone(&produced);
        self.local.message([value], move |k, v, m| {
            observations
                .lock()
                .unwrap()
                .push((k, v.clone(), T::clone(m)));
            *produced_in.lock().unwrap() = Some(k);
        });
        produced
            .lock()
            .unwrap()
            .expect("Known::message must fire on_message for every inserted value")
    }

    pub fn redact_one(&mut self, key: Key) {
        self.local.redact([key]);
    }
}

/// Bidirectional gossip between two peers: each side merges the other's
/// state into its own and records observations. After this returns,
/// `a.local == b.local`.
pub fn gossip_step<T>(a: &mut Peer<T>, b: &mut Peer<T>)
where
    T: Clone + BorshSerialize + BorshDeserialize + Send + Sync + 'static,
{
    let a_snapshot = a.local.fork();
    let b_snapshot = b.local.fork();

    // `learn` is fallible only if the two parties are not disjoint; every peer
    // in a test fleet descends from one seed by forking, so they always are.
    // (`unwrap_or_else` rather than `expect` keeps `T: Debug` off the bound.)
    let obs_a = Arc::clone(&a.observations);
    a.local
        .learn(b_snapshot, move |k, v, m| {
            obs_a.lock().unwrap().push((k, v.clone(), T::clone(m)));
        })
        .unwrap_or_else(|_| unreachable!("fleet peers share one seed, so are disjoint"));

    let obs_b = Arc::clone(&b.observations);
    b.local
        .learn(a_snapshot, move |k, v, m| {
            obs_b.lock().unwrap().push((k, v.clone(), T::clone(m)));
        })
        .unwrap_or_else(|_| unreachable!("fleet peers share one seed, so are disjoint"));
}

/// Drive every pair toward convergence by repeatedly running
/// `gossip_step` over all pairs in a fixed order until no peer's
/// `Known` changes for a full round. A bounded outer loop guards
/// against pathological non-termination (which would itself be a bug
/// the test should catch).
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
        let snapshot: Vec<Known<T>> = peers.iter_mut().map(|p| p.local.fork()).collect();

        for i in 0..n {
            for j in (i + 1)..n {
                let (left, right) = peers.split_at_mut(j);
                gossip_step(&mut left[i], &mut right[0]);
            }
        }

        let changed = peers
            .iter()
            .zip(snapshot.iter())
            .any(|(p, s)| &p.local != s);
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
