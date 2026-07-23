//! Deterministic tests for the swarm's steady-state controller.
//!
//! The controller's contract is that each node's live-message count
//! converges onto the target — including after retargeting, and including
//! when other parties' redactions have strewn stale keys through the local
//! pool. Everything here is single-threaded and seeded, so a failure
//! reproduces exactly.

use super::*;

/// Message size for controller tests: small, so runs stay fast — the
/// controller's arithmetic never depends on payload size.
const TEST_MESSAGE_SIZE: usize = 32;

/// In-memory stream capacity for the test links, matching the swarm default.
const TEST_DUPLEX_CAPACITY: usize = 16 * 1024;

/// One test party: its rumor set, observer-fed key pool, and seeded rng —
/// the same per-thread state `run_party` keeps, minus the threads.
struct Party {
    rumors: Rumors<Payload>,
    observer: UnorderedMessages<Payload>,
    keys: Vec<Key>,
    rng: SmallRng,
}

impl Party {
    fn new(rumors: Rumors<Payload>, seed: u64) -> Self {
        let observer = rumors.unordered_messages();
        Party {
            rumors,
            observer,
            keys: Vec::new(),
            rng: SmallRng::seed_from_u64(seed),
        }
    }

    /// Run `ops` controller operations at `target`, draining the observer
    /// and snapshotting before each op exactly as the party loop does.
    fn churn(&mut self, target: u64, ops: usize) {
        for _ in 0..ops {
            drain_keys(&mut self.observer, &mut self.keys);
            let snap = self.rumors.snapshot();
            steady_state_op(
                &mut self.rng,
                &self.rumors,
                &snap,
                &mut self.keys,
                target,
                TEST_MESSAGE_SIZE,
            );
        }
    }

    fn live(&self) -> u64 {
        self.rumors.snapshot().len() as u64
    }
}

/// Reconcile two parties over an in-memory link, both ends on one
/// current-thread runtime, as `bootstrap_fork` runs its halves.
fn reconcile(runtime: &tokio::runtime::Runtime, a: &Rumors<Payload>, b: &Rumors<Payload>) {
    let (mut la, mut lb) = rumors::link::memory_with_capacity(TEST_DUPLEX_CAPACITY);
    let (ra, rb) = runtime.block_on(async { tokio::join!(a.gossip(&mut la), b.gossip(&mut lb)) });
    ra.expect("gossip a");
    rb.expect("gossip b");
}

/// The controller's fixed point is the target, and it survives retargeting:
/// driving three gossiping parties through a target drop and a target raise
/// must land each party's live count within half-to-double of every phase's
/// target, even though every phase's redaction bursts fill each party's key
/// pool with entries the others already redacted. Three parties is the
/// smallest swarm where those stale entries outpace the pool's drain (each
/// party's draws must keep up with everyone's inserts), so a controller
/// that burns its redact ops on stale keys stalls far above a lowered
/// target here, while a two-party run would sit at the balance boundary
/// and hide the defect.
#[test]
fn controller_converges_through_retargeting() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("build test runtime");

    // Seed shared content, then fork three disjoint parties from it, exactly
    // as the swarm boots.
    let seed: Rumors<Payload> = Peer::seed().into_rumors();
    {
        let mut rng = SmallRng::seed_from_u64(0x5eed);
        let mut batch = seed.batch();
        for _ in 0..100 {
            batch.send(random_message(&mut rng, TEST_MESSAGE_SIZE));
        }
    }
    let mut parties = [
        Party::new(
            bootstrap_fork(&runtime, &seed, TEST_DUPLEX_CAPACITY),
            0xa11ce,
        ),
        Party::new(bootstrap_fork(&runtime, &seed, TEST_DUPLEX_CAPACITY), 0xb0b),
        Party::new(
            bootstrap_fork(&runtime, &seed, TEST_DUPLEX_CAPACITY),
            0xca201,
        ),
    ];
    drop(seed);

    // Three phases: settle at the seed-sized target, drop hard, raise hard.
    // Each phase interleaves bursts of local churn with ring reconciliation
    // every third round, the swarm's own rhythm at test scale; the assertion
    // runs after the phase's full budget.
    for (target, rounds) in [(100u64, 12), (20, 24), (200, 24)] {
        for round in 0..rounds {
            for party in &mut parties {
                party.churn(target, 25);
            }
            if round % 3 == 0 {
                reconcile(&runtime, &parties[0].rumors, &parties[1].rumors);
                reconcile(&runtime, &parties[1].rumors, &parties[2].rumors);
            }
        }
        // One full ring pass so every party holds the reconciled set before
        // its count is judged.
        reconcile(&runtime, &parties[0].rumors, &parties[1].rumors);
        reconcile(&runtime, &parties[1].rumors, &parties[2].rumors);
        reconcile(&runtime, &parties[2].rumors, &parties[0].rumors);
        for party in &parties {
            let live = party.live();
            assert!(
                live >= target / 2 && live <= target * 2,
                "controller failed to converge: live {live} vs target {target}",
            );
        }
    }
}
