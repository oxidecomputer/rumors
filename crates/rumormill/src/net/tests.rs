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

/// One PeerView roster entry around `peer`, everything else defaulted.
fn roster_of(peer: PeerId) -> View {
    View {
        roster: vec![crate::view::PeerView {
            peer,
            name: String::new(),
            last_seen: 0,
        }],
        ..View::default()
    }
}

proptest! {
    /// The dialing tie-break covers every roster pair exactly once: for any
    /// two distinct peers, exactly one side lists the other as a dial
    /// candidate, so the steady-state mesh settles on one connection per
    /// pair with neither a dial storm nor an orphaned pair.
    #[test]
    fn exactly_one_roster_side_dials(a in any::<[u8; 32]>(), b in any::<[u8; 32]>()) {
        prop_assume!(a != b);
        let (active, backoff) = (HashSet::new(), HashMap::new());
        let a_dials = !dial_candidates(&roster_of(b), &active, &backoff, a).is_empty();
        let b_dials = !dial_candidates(&roster_of(a), &active, &backoff, b).is_empty();
        prop_assert!(a_dials ^ b_dials);
    }
}

/// A manual dial target is always ours to dial — the other side may not
/// know us yet, so the roster tie-break cannot apply — but live connections
/// and backed-off peers are still excluded.
#[test]
fn manual_targets_are_always_ours_to_dial() {
    let me = [9u8; 32];
    let target = [1u8; 32]; // smaller than `me`: the tie-break would defer
    let view = View {
        dial_targets: vec![target],
        ..View::default()
    };

    let (active, backoff) = (HashSet::new(), HashMap::new());
    assert_eq!(dial_candidates(&view, &active, &backoff, me), vec![target]);

    let connected = HashSet::from([target]);
    assert!(dial_candidates(&view, &connected, &backoff, me).is_empty());

    let resting = HashMap::from([(target, Instant::now() + Duration::from_secs(60))]);
    assert!(dial_candidates(&view, &active, &resting, me).is_empty());
}

/// Self never appears as a dial candidate, from the roster or the manual
/// targets: a node must not gossip with itself.
#[test]
fn self_is_never_a_candidate() {
    let me = [7u8; 32];
    let view = View {
        dial_targets: vec![me],
        ..roster_of(me)
    };
    let (active, backoff) = (HashSet::new(), HashMap::new());
    assert!(dial_candidates(&view, &active, &backoff, me).is_empty());
}
