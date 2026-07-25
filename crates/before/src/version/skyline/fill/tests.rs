//! Differential pins for the skyline fill and the tick splice.
//!
//! `fill` is held to the recursive oracle (`oracle::Version::fill`
//! through the bridge) over every pool, with canonical uniqueness
//! making the differential total: the filled stream must equal the
//! oracle's encoded result byte for byte. The `tick` asserts run the
//! module function against the public `Version::tick`, which routes to
//! the same kernel — an entry-agreement and determinism pin, not an
//! independent value; the splice's value correctness rests on the fill
//! oracle here and the grow suite's oracle and brute-force pins
//! (`grow/tests.rs`), so each branch is pinned, not just their
//! composition. The deep-spine case derives its expected values in
//! closed form (its test doc states each derivation).

use proptest::prelude::*;
use rayon::prelude::*;

use crate::meter::{
    alt_spine, bigroot, cancelling_chain, cliff_comb, cliff_fan, dense, harmonic, hugeleaf,
    id_spine, nested_full_id, scattered_id, wide_tooth_comb, Packed,
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

/// Assert the tick splice through both entry points on one pair, and
/// that the output validates as canonical.
///
/// The module function runs against the public `Version::tick`, which
/// routes to the same kernel: this pins entry agreement and
/// determinism, not an independent value.
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
        party_of(&nested_full_id(1)),
        party_of(&nested_full_id(8)),
    ];
    pool.extend(all_normal_ids(2).iter().map(from_oracle_party));
    pool
}

/// Every event-family × party-family pair fills byte-identically to
/// the recursive oracle and ticks in agreement through both entry
/// points (owning parties; the empty id has no tick).
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
/// normal-form id fills byte-identically to the recursive oracle, and
/// every owning id ticks in agreement through both entry points.
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

/// The version that is `1` on the leftmost `2^-depth` interval and `0`
/// everywhere else: `depth` nested nodes, all bases zero, the single
/// 1-leaf at the bottom left.
///
/// Built as a text literal — the parser is iterative — so the expected
/// tree shares no walk with fill or grow.
fn left_spike(depth: usize) -> Version {
    let mut text = "(0, ".repeat(depth - 1);
    text.push_str("(0, 1, 0)");
    text.push_str(&", 0)".repeat(depth - 1));
    text.parse().expect("the spike literal is normal form")
}

/// Deep spines in every regime stay correct at depths that would
/// overflow a native-frame walk: the collapse scan, the pass-through
/// copy, and the two-cursor descent.
///
/// The recursive `oracle` enums walk on native frames (they are the
/// small-scope reference, not a deep-input one), so the value witnesses
/// here are closed forms, derived per case: the full id collapses the
/// whole spine to one leaf at its maximum height — the alternating
/// spine's only nonzero leaf is the `1` at the bottom pair — and that
/// collapse is the tick; the deep unary id over the empty version fills
/// to the identity (fill of a leaf under a node id is the leaf) and
/// ticks to the left spike, the expansion chain to its owned tip; and
/// over the deep spine the same id turns left into the spine's depth-2
/// zero leaf (the spine's structure continues right there), so fill is
/// again the identity and the grown tree raises exactly the owned
/// region from 0 to 1 — the pointwise max with the spike, realized
/// through the independently-pinned join kernel and byte-exact by
/// canonical uniqueness. The nested-full-sibling id over its matched
/// spine fills to the identity, derived: every level's right-full
/// raise is `max(max(er), min(fill(il, el)))`, where `er` is a single
/// leaf (its own maximum) and the left range's minimum stays at the
/// spine's floor of zero, so no raise ever moves a value — and the id
/// terminus pairs with a leaf (the untouched-leaf arm). The case
/// drives the drift stack and both re-scan genres at full depth (the
/// #33 cost adversary) with a value witness the small scope pins
/// against the oracle. Canonicality, fill idempotence, and tick entry
/// agreement ride along on every case.
#[test]
fn deep_spines_fill_and_tick_identically() {
    let assert_deep = |v: &Version, p: &Party| {
        let out = fill(&encode(v), p);
        validate(&out.bytes, out.bits).expect("a filled stream is canonical");
        let again = fill(&out, p);
        assert_eq!(again, out, "deep fill must be idempotent");
        assert_tick(v, p);
        out
    };
    let deep_ev = version_of(&alt_spine(4096));
    let deep_id = party_of(&id_spine(4096, false));
    let spike = left_spike(4096);
    let one: Version = "1".parse().expect("test literals parse");

    // The full id: the collapse to the maximum leaf is the fill, and
    // fill changed the tree, so it is also the tick.
    let filled = assert_deep(&deep_ev, &Party::seed());
    assert_eq!(
        filled,
        encode(&one),
        "the full id collapses the spine to its maximum leaf"
    );
    assert_eq!(
        tick(&encode(&deep_ev), &Party::seed()),
        encode(&one),
        "tick takes the fill branch: the collapse"
    );

    // The deep unary id over the empty version: identity fill, so tick
    // falls through to grow's expansion chain.
    let filled = assert_deep(&Version::new(), &deep_id);
    assert_eq!(
        filled,
        encode(&Version::new()),
        "fill of a leaf under a node id is the identity"
    );
    assert_eq!(
        tick(&encode(&Version::new()), &deep_id),
        encode(&spike),
        "tick grows the expansion chain to the owned tip"
    );

    // Both deep: no fully-owned region meets a subdividable subtree, so
    // fill is the identity; the grown tree is the pointwise max with
    // the spike.
    let filled = assert_deep(&deep_ev, &deep_id);
    assert_eq!(
        filled,
        encode(&deep_ev),
        "no full-id region: fill is the identity"
    );
    assert_eq!(
        tick(&encode(&deep_ev), &deep_id),
        encode(&(&deep_ev | &spike)),
        "the grown stream is the pointwise max with the spike"
    );

    // The nested-full-sibling id over its matched spine: a right-full
    // shortcut site at every one of the 4096 levels — the drift stack
    // and both re-scan genres at full depth. Fill is the identity (the
    // module doc's derivation: each raise maxes a lone leaf against a
    // zero minimum), and the small scope pins the same cross against
    // the oracle at every depth it enumerates.
    let mut text = "(0, ".repeat(4095);
    text.push_str("(0, 0, 1)");
    text.push_str(&", 0)".repeat(4095));
    let matched: Version = text.parse().expect("the matched spine literal parses");
    let nested = party_of(&nested_full_id(4096));
    let filled = assert_deep(&matched, &nested);
    assert_eq!(
        filled,
        encode(&matched),
        "every nested raise maxes a lone leaf against a zero minimum: identity"
    );
}

proptest! {
    /// Arbitrary parties over arbitrary normal-form versions fill
    /// byte-identically to the recursive oracle (and tick through both
    /// entry points, when the party owns anything), magnitudes past
    /// `u64::MAX` included.
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

    /// Organic histories fill byte-identically to the recursive oracle
    /// and tick in agreement through both entry points.
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
