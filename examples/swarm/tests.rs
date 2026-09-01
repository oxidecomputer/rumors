//! Deterministic tests for the swarm's steady-state controller.
//!
//! The controller's contract is that each node's live-message count
//! converges onto the target — including after retargeting, and including
//! when other parties' redactions have strewn stale entries through the
//! local pool. Everything here is single-threaded and seeded, so a failure
//! reproduces exactly.

use super::*;

/// Message size for controller tests: small, so runs stay fast — the
/// controller's arithmetic never depends on payload size.
const TEST_MESSAGE_SIZE: usize = 32;

/// In-memory stream capacity for the test links, matching the swarm default.
const TEST_DUPLEX_CAPACITY: usize = 16 * 1024;

/// One test party: its rumor set, observer-fed version pool, and seeded
/// rng — the same per-thread state `run_party` keeps, minus the threads.
struct Party {
    rumors: Rumors<Payload>,
    observer: UnorderedMessages<Payload>,
    pool: Vec<Version>,
    rng: SmallRng,
}

impl Party {
    fn new(rumors: Rumors<Payload>, seed: u64) -> Self {
        let observer = rumors.unordered_messages();
        Party {
            rumors,
            observer,
            pool: Vec::new(),
            rng: SmallRng::seed_from_u64(seed),
        }
    }

    /// Run `ops` controller operations at `target`, draining the observer
    /// and snapshotting before each op exactly as the party loop does.
    fn churn(&mut self, target: u64, ops: usize) {
        for _ in 0..ops {
            drain_versions(&mut self.observer, &mut self.pool);
            let snap = self.rumors.snapshot();
            steady_state_op(
                &mut self.rng,
                &self.rumors,
                &snap,
                &mut self.pool,
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

/// The controller's fixed point is the target, and it survives retargeting.
///
/// Driving three gossiping parties through a target drop and a target raise
/// must land each party's live count within half-to-double of every phase's
/// target, even though every phase's redaction bursts fill each party's
/// pool with entries the others already redacted. Three parties is the
/// smallest swarm where those stale entries outpace the pool's drain (each
/// party's draws must keep up with everyone's inserts), so a controller
/// that burns its redact ops on stale entries stalls far above a lowered
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
        seed.send_all((0..100).map(|_| random_message(&mut rng, TEST_MESSAGE_SIZE)))
            .expect("flat test payloads are within any depth limit");
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
    // every third round, the swarm's own rhythm at test scale; the judgment
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
        // Judge the converged equilibrium, not one draw of it: the count
        // oscillates round to round (churn bursts between syncs are large
        // relative to a small target), so a single post-ring sample puts
        // the band's edge inside the oscillation. Each sample is one
        // churn round settled by a full ring pass — so every party holds
        // the reconciled surviving set when read — and each party's mean
        // over the samples is what must sit in band.
        const SAMPLES: usize = 4;
        let mut settled = [0u64; 3];
        for _ in 0..SAMPLES {
            for party in &mut parties {
                party.churn(target, 25);
            }
            reconcile(&runtime, &parties[0].rumors, &parties[1].rumors);
            reconcile(&runtime, &parties[1].rumors, &parties[2].rumors);
            reconcile(&runtime, &parties[2].rumors, &parties[0].rumors);
            for (sum, party) in settled.iter_mut().zip(&parties) {
                *sum += party.live();
            }
        }
        for sum in settled {
            let live = sum / SAMPLES as u64;
            assert!(
                live >= target / 2 && live <= target * 2,
                "controller failed to converge: mean live {live} vs target {target}",
            );
        }
    }
}
