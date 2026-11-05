//! The default budget pipelines an ordinary disputed frontier.
//!
//! The fixture uses ordinary hashed paths. With the default budget, a
//! session keeps the questions of a whole frontier in flight at once, so
//! its round trips follow the tree's depth rather than the number of
//! disputes.

use std::time::Duration;

use rand::{RngCore, SeedableRng};
use rand_chacha::ChaChaRng;
use rumors::Rumors;
use rumors_testkit::bench::latency;
use rumors_testkit::common::budget::BudgetChoice;
use rumors_testkit::common::property::FLEET_DIVISOR;
use rumors_testkit::common::seed::{NetworkSeed, network_seed};
use rumors_testkit::common::wire::bootstrap_fork_with_budget;
use rumors_testkit::divided_proptest;

/// Messages both peers share before the fork.
const COMMON: usize = 2_048;

/// Messages each peer originates after the fork: enough divergence to
/// dispute nearly all 256 root-child scopes.
const DIVERGENT_PER_SIDE: usize = 512;

/// One-way link delay, in whole milliseconds (the timer wheel's grain).
const DELAY: Duration = Duration::from_millis(10);

/// The most one-way hops the default-budget session may take.
///
/// The session measures 7: a few levels of descent, each a round trip,
/// with every level's questions in flight together. The ceiling leaves room
/// for modest changes in fixed protocol cost. A session that asked these
/// hundreds of disputes one question at a time would take a round trip for
/// each; the test prints that count for the same fixture at a budget of
/// zero.
const HOP_CEILING: u32 = 24;

/// Per-direction in-flight window: far above this session's transfers, so
/// only round-trip structure is measured (see the module docs).
const LINK_CAPACITY: usize = 8 * 1024 * 1024;

divided_proptest! {
    #![divisor(FLEET_DIVISOR)]

    /// The default budget completes this fixture's session within
    /// [`HOP_CEILING`] one-way hops.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn default_budget_pipelines_disputed_scopes(network in network_seed()) {
        let pipelined =
            latency::session_hops(LINK_CAPACITY, DELAY, diverged_pair(network, BudgetChoice::Default));
        let one_at_a_time =
            latency::session_hops(LINK_CAPACITY, DELAY, diverged_pair(network, BudgetChoice::Zero));
        eprintln!("default budget: {pipelined} hops; budget zero: {one_at_a_time} hops");
        assert!(
            (2..HOP_CEILING).contains(&pipelined),
            "the default budget took {pipelined} hops, expected a completed exchange below \
             {HOP_CEILING}",
        );
    }
}

/// Two peers of the network `network` names, with shared history and
/// concurrent additions, both on `budget`.
fn diverged_pair(network: NetworkSeed, budget: BudgetChoice) -> (Rumors<u64>, Rumors<u64>) {
    let left = budget.apply(network.peer()).into_rumors();
    // Fixed, so the hop ceiling is judged on the same corpus every run.
    let mut rng = ChaChaRng::seed_from_u64(0x9e37_79b9_7f4a_7c15);
    send_random(&left, COMMON, &mut rng);
    let right = bootstrap_fork_with_budget(&left, budget);
    send_random(&left, DIVERGENT_PER_SIDE, &mut rng);
    send_random(&right, DIVERGENT_PER_SIDE, &mut rng);
    (left, right)
}

/// Commit `n` random payloads as one batch.
fn send_random(rumors: &Rumors<u64>, n: usize, rng: &mut ChaChaRng) {
    rumors.send_all((0..n).map(|_| rng.next_u64())).unwrap();
}
