//! Differential pins for the skyline fill and the tick splice.
//!
//! The packed-form implementations are the byte-level oracles
//! (canonical uniqueness makes the differentials total): `fill` must
//! transcode-commute with the packed-form fill AND with
//! `oracle::Version::fill` through the bridge — the splice's simplify
//! branch pinned to two independent witnesses — and `tick` must
//! transcode-commute with the *public* `Version::tick`, so the splice's
//! fall-through to grow is pinned end to end, not assembled from parts.

use proptest::prelude::*;
use rayon::prelude::*;

use crate::meter::{
    alt_spine, bigroot, cancelling_chain, cliff_comb, cliff_fan, dense, harmonic, hugeleaf,
    id_spine, scattered_id, wide_tooth_comb, Packed,
};
use crate::testing::bridge::{
    from_oracle_party, from_oracle_version, to_oracle_party, to_oracle_version,
};
use crate::testing::exhaustive::{
    all_normal_events, all_normal_ids, EV_SMALL_DEPTH, ID_SMALL_DEPTH,
};
use crate::testing::{generators, optrace};
use crate::version::skyline::{encode, validate};
use crate::{Clock, Party, Version};

use super::{fill, tick};

/// Lift a meter-generated packed event shape into a [`Version`].
fn version_of(p: &Packed) -> Version {
    p.version()
}

/// Decode a meter-generated packed id shape as a [`Party`].
fn party_of(p: &Packed) -> Party {
    Party::decode(&p.bytes[..]).expect("meter shapes are strict normal form")
}

/// Assert the fill kernel against the recursive oracle on one pair, and
/// that its output validates as canonical.
fn assert_fill(v: &Version, p: &Party) {
    let enc = encode(v);
    let out = fill(&enc, p);
    validate(&out.bytes, out.bits).expect("a filled stream is canonical");
    let oracle = to_oracle_version(v).fill_for_test(&to_oracle_party(p));
    assert_eq!(
        out,
        encode(&from_oracle_version(&oracle)),
        "fill must match the recursive oracle: {v} with {p}"
    );
}

/// Assert the tick splice against the public `Version::tick` on one
/// pair, and that its output validates as canonical.
fn assert_tick(v: &Version, p: &Party) {
    let out = tick(&encode(v), p);
    let mut expected = v.clone();
    expected.tick(p);
    assert_eq!(
        out,
        encode(&expected),
        "the tick splice and the public tick must agree: {v} with {p}"
    );
    validate(&out.bytes, out.bits).expect("a ticked stream is canonical");
}

/// The adversarial event pool: every §2 family at two scales, plus the
/// empty version.
fn event_pool() -> Vec<Version> {
    vec![
        Version::new(),
        version_of(&dense(1)),
        version_of(&dense(2)),
        version_of(&dense(64)),
        version_of(&bigroot(7, 3)),
        version_of(&bigroot(64, 16)),
        version_of(&hugeleaf(1)),
        version_of(&hugeleaf(64)),
        version_of(&cliff_comb(3, 2)),
        version_of(&cliff_comb(16, 16)),
        version_of(&wide_tooth_comb(16, 8, 8)),
        version_of(&cliff_fan(16, 8)),
        version_of(&cancelling_chain(16, 8)),
        version_of(&alt_spine(3)),
        version_of(&alt_spine(64)),
        version_of(&harmonic(16)),
    ]
}

/// The adversarial party pool: the seed, deep and diverted unary
/// spines, scattered ownership, and every exhaustive small-scope id —
/// including the empty id, which `fill` (unlike `grow`) accepts as the
/// identity arm.
fn party_pool() -> Vec<Party> {
    let mut pool = vec![
        Party::seed(),
        party_of(&id_spine(1, false)),
        party_of(&id_spine(3, false)),
        party_of(&id_spine(3, true)),
        party_of(&id_spine(64, false)),
        party_of(&id_spine(64, true)),
        party_of(&scattered_id(1)),
        party_of(&scattered_id(16)),
    ];
    pool.extend(all_normal_ids(2).iter().map(from_oracle_party));
    pool
}

/// Every event-family × party-family pair fills byte-identically to
/// both witnesses and ticks byte-identically to the public tick
/// (owning parties; the empty id has no tick).
#[test]
fn family_pairs_fill_and_tick_identically() {
    let events = event_pool();
    let parties = party_pool();
    events.par_iter().for_each(|v| {
        for p in &parties {
            assert_fill(v, p);
            if !p.as_bits().is_empty() {
                assert_tick(v, p);
            }
        }
    });
}

/// Exhaustive small scope: every normal-form event tree × every
/// normal-form id fills byte-identically to both witnesses, and every
/// owning id ticks byte-identically to the public tick.
#[test]
fn exhaustive_small_scope_fills_and_ticks_identically() {
    let events: Vec<Version> = all_normal_events(EV_SMALL_DEPTH)
        .iter()
        .map(from_oracle_version)
        .collect();
    let parties: Vec<Party> = all_normal_ids(ID_SMALL_DEPTH)
        .iter()
        .map(from_oracle_party)
        .collect();
    events.par_iter().for_each(|v| {
        for p in &parties {
            assert_fill(v, p);
            if !p.as_bits().is_empty() {
                assert_tick(v, p);
            }
        }
    });
}

/// The worked examples, pinned end to end: the full-id collapse, both
/// shortcut raises (taken and declined), and a nested arm.
#[test]
fn worked_examples_fill_exactly() {
    let cases: [(&str, &str, &str); 6] = [
        // The full id collapses the whole tree to its max (heights 2
        // and 3; the collapse is the higher plateau).
        ("1", "(2, 0, 1)", "3"),
        // Left-full: the collapsed left rises to the right's minimum
        // (min fill(0, er) = 3 > max(el) = 2), and the pair merges.
        ("(1, 0)", "(2, 0, 1)", "3"),
        // Right-full, mirrored.
        ("(0, 1)", "(2, 1, 0)", "3"),
        // Right-full where the raise is declined (max(er) = 3 already
        // clears min(el') = 2): nothing changes.
        ("(0, 1)", "(2, 0, 1)", "(2, 0, 1)"),
        // Left-full over an internal left child: the whole el subtree
        // collapses into the raised leaf.
        ("(1, 0)", "(2, (0, 1, 0), 3)", "5"),
        // A node id whose left child is itself a shortcut site: the
        // inner raise lifts the root's minimum, and norm re-lifts it.
        ("((1, 0), 0)", "(1, (0, 0, 1), 2)", "(2, 0, 1)"),
    ];
    for (party, before, after) in cases {
        let p: Party = party.parse().expect("test party literals parse");
        let v: Version = before.parse().expect("test version literals parse");
        let expected: Version = after.parse().expect("test version literals parse");
        assert_eq!(
            fill(&encode(&v), &p),
            encode(&expected),
            "fill of {before} with {party} must yield {after}"
        );
        assert_fill(&v, &p);
    }
}

/// The splice takes both branches: a fill that simplifies is the tick,
/// and a fill that changes nothing falls through to grow.
#[test]
fn tick_splices_fill_and_grow() {
    // fill simplifies: the collapse is the tick.
    let v: Version = "(2, 0, 1)".parse().expect("test literals parse");
    let p: Party = "(1, 0)".parse().expect("test literals parse");
    assert_eq!(tick(&encode(&v), &p), encode(&"3".parse().unwrap()));
    // fill is the identity: grow registers the event.
    let v: Version = "(0, 1, 0)".parse().expect("test literals parse");
    assert_eq!(tick(&encode(&v), &p), encode(&"(0, 2, 0)".parse().unwrap()));
    assert_tick(&v, &p);
}

/// Deep spines in every regime stay correct at depths that would
/// overflow a native-frame walk: the collapse scan, the pass-through
/// copy, and the two-cursor descent.
///
/// Held to depth-safe witnesses only: the recursive `oracle` enums walk
/// on native frames (they are the small-scope reference, not a deep-input
/// one), so the deep pins are canonicality, fill's idempotence (a filled
/// tree re-fills to itself), and tick agreement through both entries.
#[test]
fn deep_spines_fill_and_tick_identically() {
    let assert_deep = |v: &Version, p: &Party| {
        let out = fill(&encode(v), p);
        validate(&out.bytes, out.bits).expect("a filled stream is canonical");
        let again = fill(&out, p);
        assert_eq!(again, out, "deep fill must be idempotent");
        assert_tick(v, p);
    };
    let deep_ev = version_of(&alt_spine(4096));
    assert_deep(&deep_ev, &Party::seed());
    let deep_id = party_of(&id_spine(4096, false));
    assert_deep(&Version::new(), &deep_id);
    assert_deep(&deep_ev, &deep_id);
}

proptest! {
    /// Arbitrary parties over arbitrary normal-form versions fill
    /// byte-identically to both witnesses (and tick, when the party
    /// owns anything), magnitudes past `u64::MAX` included.
    #[test]
    fn arbitrary_pairs_fill_and_tick_identically(
        op in generators::arb_oracle_party_nonempty(),
        ov in generators::arb_oracle_version(),
    ) {
        let p = from_oracle_party(&op);
        let v = from_oracle_version(&ov);
        assert_fill(&v, &p);
        if !p.as_bits().is_empty() {
            assert_tick(&v, &p);
        }
    }

    /// Organic histories fill and tick byte-identically to the packed
    /// forms.
    ///
    /// Every clock produced by one fork/tick/send/sync/join history is
    /// exercised from its own party — and from every *other* clock's
    /// party, the concurrent-editor shape.
    #[test]
    fn organic_histories_fill_and_tick_identically(ops in optrace::world_strategy_up_to(40)) {
        let mut clocks = vec![Clock::seed()];
        for op in &ops {
            optrace::step_impl(&mut clocks, op);
        }
        for a in &clocks {
            for b in &clocks {
                assert_fill(a.version(), b.party());
                if !b.party().as_bits().is_empty() {
                    assert_tick(a.version(), b.party());
                }
            }
        }
    }
}
