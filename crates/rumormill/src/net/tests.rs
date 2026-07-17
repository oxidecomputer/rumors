//! Tests for the pure merge rule. (The networked paths are exercised by the
//! owner's in-memory link tests and the manual smoke script; `decide` is the
//! one piece both sides must agree on blind.)

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
    /// plugging the mismatch error's two declared pairs into `decide` in
    /// opposite roles — reach opposite verdicts, so exactly one resets.
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

proptest! {
    /// The merge verdict must be computed from the handshake-declared
    /// floors on both sides. For any declared floors and any mid-session
    /// drift (each side's fresh floor is >= what it declared, because
    /// local commits only add events), the declared-floor verdicts are
    /// opposite: exactly one side wins, whatever the drift. Drift can
    /// never demote a declared winner (`decide` is monotone in `ours`),
    /// only promote the declared loser into a second winner — so a side
    /// that substitutes its fresh floor for the declared one risks the
    /// both-Win collision, never a both-Lose one. This is the property
    /// that failed when `ours` was read from a fresh snapshot at mismatch
    /// time.
    #[test]
    fn declared_floors_yield_exactly_one_winner(
        declared_a in any::<u64>(),
        drift_a in any::<u64>(),
        declared_b in any::<u64>(),
        drift_b in any::<u64>(),
        seed_a in any::<u64>(),
        seed_b in any::<u64>(),
    ) {
        prop_assume!(seed_a != seed_b);
        let (net_a, net_b) = (network(seed_a), network(seed_b));
        let fresh_a = declared_a.saturating_add(drift_a);
        let fresh_b = declared_b.saturating_add(drift_b);

        // The deployed rule: both sides decide from the declared floors —
        // the values that actually crossed the wire in the handshake.
        let verdict_a = decide((declared_a, net_a), (declared_b, net_b));
        let verdict_b = decide((declared_b, net_b), (declared_a, net_a));
        prop_assert_ne!(verdict_a, verdict_b);

        // A declared winner still wins from its fresh floor: the collision
        // the fresh-snapshot bug produced is one-sided (both Win), so the
        // failure mode is two servers waiting on absent losers, never two
        // losers bootstrapping into each other.
        if verdict_a == Verdict::Win {
            prop_assert_eq!(decide((fresh_a, net_a), (declared_b, net_b)), Verdict::Win);
        }
        if verdict_b == Verdict::Win {
            prop_assert_eq!(decide((fresh_b, net_b), (declared_a, net_a)), Verdict::Win);
        }
    }
}

/// Regression for the fresh-snapshot verdict bug: with equal declared
/// floors and one commit landing mid-session on each side, deciding from
/// the fresh floors makes both sides Win — each then serves a merge the
/// other never requests — while deciding from the declared floors stays
/// antisymmetric.
#[test]
fn fresh_floors_can_make_both_sides_win() {
    let (net_a, net_b) = (network(1), network(2));
    let declared = 7;
    let fresh = declared + 1; // one local commit landed mid-session

    // The broken construction, kept as documentation of the failure mode:
    // each side pairs its own fresh floor with the peer's declared one.
    assert_eq!(decide((fresh, net_a), (declared, net_b)), Verdict::Win);
    assert_eq!(decide((fresh, net_b), (declared, net_a)), Verdict::Win);

    // The deployed construction: declared against declared, exactly one
    // winner.
    assert_ne!(
        decide((declared, net_a), (declared, net_b)),
        decide((declared, net_b), (declared, net_a))
    );
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
