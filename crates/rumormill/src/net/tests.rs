//! Tests for the pure merge rule. (The networked paths are exercised by the
//! owner's duplex tests and the manual smoke script; `decide` is the one
//! piece both sides must agree on blind.)

use proptest::prelude::*;
use rand::SeedableRng;
use rand::rngs::StdRng;

use super::*;

/// Mint a real (opaque) `Network` from a deterministic seed.
fn network(seed: u64) -> Network {
    Peer::<Entry>::seed_rng(&mut StdRng::seed_from_u64(seed)).network()
}

proptest! {
    /// Antisymmetry: for any two distinct universes, the two sides — each
    /// holding its own pair and the one the mismatch error reported — reach
    /// opposite verdicts, so exactly one resets.
    #[test]
    fn exactly_one_winner(
        events_a in any::<u64>(),
        events_b in any::<u64>(),
        seed_a in any::<u64>(),
        seed_b in any::<u64>(),
    ) {
        prop_assume!(seed_a != seed_b);
        let a = (events_a, network(seed_a));
        let b = (events_b, network(seed_b));
        prop_assert_ne!(decide(a, b), decide(b, a));
    }

    /// The event floor dominates: an older (busier) universe always wins,
    /// whatever the network ids.
    #[test]
    fn event_floor_dominates(
        events in any::<u64>(),
        seed_a in any::<u64>(),
        seed_b in any::<u64>(),
    ) {
        prop_assume!(seed_a != seed_b);
        prop_assume!(events < u64::MAX);
        let younger = (events, network(seed_a));
        let older = (events + 1, network(seed_b));
        prop_assert_eq!(decide(older, younger), Verdict::Win);
        prop_assert_eq!(decide(younger, older), Verdict::Lose);
    }
}

/// On an event-floor tie, the greater network id wins: deterministic, and
/// still antisymmetric.
#[test]
fn ties_break_on_network_id() {
    let (a, b) = (network(1), network(2));
    let (lo, hi) = if a < b { (a, b) } else { (b, a) };
    assert_eq!(decide((7, hi), (7, lo)), Verdict::Win);
    assert_eq!(decide((7, lo), (7, hi)), Verdict::Lose);
}
