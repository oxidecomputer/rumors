//! Differential pins for the skyline grow against three witnesses.
//!
//! The recursive oracle's `grow` (through the bridge) is the byte-level
//! value witness — canonical uniqueness makes the differential total:
//! the grown stream must equal the oracle's normalized, encoded
//! inflation byte for byte — a reference *recursive* probe pins the
//! iterative probe's [`Route`] bit vector — the explicit guard against
//! a probe/emit coordinate drift, which would misread a direction
//! silently rather than panic — and the brute-force minimal-inflation
//! search holds the whole kernel to `grow`'s defining optimality
//! directly, not merely to another implementation of the same dynamic
//! program. The deep-spine case swaps the native-frame oracle for
//! closed-form expected values (its test doc states each derivation).

use proptest::prelude::*;
use rayon::prelude::*;

use crate::codec::Bits;
use crate::codec::BitsSlice;
use crate::meter::{
    alt_spine, bigroot, cancelling_chain, cliff_comb, cliff_fan, dense, harmonic, hugeleaf,
    id_spine, scattered_id, wide_tooth_comb, Packed,
};
use crate::recurse::descend;
use crate::testing::bridge::{
    from_oracle_party, from_oracle_version, to_oracle_party, to_oracle_version,
};
use crate::testing::exhaustive::{
    all_normal_events, all_normal_ids, EV_SMALL_DEPTH, ID_SMALL_DEPTH,
};
use crate::testing::grow_brute_force::best_inflation;
use crate::testing::{generators, optrace};
use crate::version::skyline::{encode, validate};
use crate::{Clock, Party, Version};

use super::{grow, id_tag, probe, Cost, EvScan, Kind, Route, COST_MAX};

/// Lift a meter-generated packed event shape into a [`Version`].
fn version_of(p: &Packed) -> Version {
    p.version()
}

/// Decode a meter-generated packed id shape as a [`Party`].
fn party_of(p: &Packed) -> Party {
    Party::decode(&p.bytes[..]).expect("meter shapes are strict normal form")
}

/// Assert the kernel's output validates as canonical on one pair, pin
/// the iterative probe's route against the recursive reference bit for
/// bit, and hold the grown stream to the recursive oracle's inflation
/// byte for byte.
fn assert_grow(v: &Version, p: &Party) {
    let out = assert_grow_depth_safe(v, p);
    let (raw, _) = to_oracle_version(v).grow_for_test(&to_oracle_party(p));
    assert_eq!(
        out,
        encode(&from_oracle_version(&raw.normalized_for_test())),
        "grow must register the recursive oracle's inflation: {v} with {p}"
    );
}

/// The depth-safe half of [`assert_grow`]: canonicality and the route
/// pin (both walks are stack-guarded), returning the grown stream.
///
/// The recursive oracle walks on native frames, so the deep-spine test
/// calls this directly and takes its value witnesses from closed forms
/// instead.
fn assert_grow_depth_safe(v: &Version, p: &Party) -> Bits {
    let enc = encode(v);
    let out = grow(&enc, p);
    validate(&out).expect("a grown stream is canonical");

    let ev_bits = enc.as_bitslice();
    let id_bits = p.as_bits();
    let mut iterative = Route::new(id_bits.len(), ev_bits.len());
    let iterative_cost = probe(ev_bits, id_bits, &mut iterative);
    let (reference, reference_cost) = reference_probe(ev_bits, id_bits);
    assert_eq!(
        iterative_cost, reference_cost,
        "the iterative probe's root cost must match the recursive reference: {v} with {p}"
    );
    assert_eq!(
        iterative.dirs, reference.dirs,
        "the iterative probe's route must match the recursive reference bit for bit: {v} with {p}"
    );
    out
}

// ───────────── the reference recursive probe ─────────────

/// The id side of a reference descent: a real node, the full regime, or
/// an absent (infeasible) child.
#[derive(Clone, Copy)]
enum RefId {
    At,
    Full,
    Empty,
}

/// Probe the cheapest inflation by direct recursion over the `(id, ev)`
/// shape — the transliteration of the recursive walk the iterative probe
/// replaces, kept as its structural witness.
fn reference_probe(ev_bits: &BitsSlice, id_bits: &BitsSlice) -> (Route, Cost) {
    let mut route = Route::new(id_bits.len(), ev_bits.len());
    let mut ev = EvScan::new(ev_bits);
    let mut id_pos = 0usize;
    let root = if id_bits.is_empty() {
        RefId::Empty
    } else {
        RefId::At
    };
    let cost = descend!(
        0,
        rec(&mut route, &mut ev, id_bits, &mut id_pos, root, false, 0)
    );
    (route, cost)
}

/// One reference recursion step; `ev_zero` marks the virtual zero leaf
/// below an expanded event leaf.
#[allow(clippy::too_many_arguments)]
fn rec(
    route: &mut Route,
    ev: &mut EvScan<'_>,
    id_bits: &BitsSlice,
    id_pos: &mut usize,
    id: RefId,
    ev_zero: bool,
    depth: usize,
) -> Cost {
    match id {
        RefId::Empty => {
            if !ev_zero {
                ev.skip();
            }
            COST_MAX
        }
        RefId::Full => {
            if ev_zero {
                return (0, 0);
            }
            let key = ev.pos();
            if ev.read().is_none() {
                let l = descend!(
                    depth + 1,
                    rec(route, ev, id_bits, id_pos, RefId::Full, false, depth + 1)
                );
                let r = descend!(
                    depth + 1,
                    rec(route, ev, id_bits, id_pos, RefId::Full, false, depth + 1)
                );
                combine(route, Kind::FullEvNode, key, l, r)
            } else {
                (0, 0)
            }
        }
        RefId::At => {
            let key = *id_pos;
            let (l, r) = id_tag(id_bits, *id_pos);
            *id_pos += 2;
            if !l && !r {
                return rec(route, ev, id_bits, id_pos, RefId::Full, ev_zero, depth);
            }
            let kind = if ev_zero || ev.read().is_some() {
                Kind::Expand
            } else {
                Kind::Both
            };
            let zero = kind == Kind::Expand;
            let la = if l { RefId::At } else { RefId::Empty };
            let lc = descend!(
                depth + 1,
                rec(route, ev, id_bits, id_pos, la, zero, depth + 1)
            );
            let ra = if r { RefId::At } else { RefId::Empty };
            let rc = descend!(
                depth + 1,
                rec(route, ev, id_bits, id_pos, ra, zero, depth + 1)
            );
            combine(route, kind, key, lc, rc)
        }
    }
}

/// Pick the cheaper child, record the direction, and fold the branch
/// cost, exactly as the iterative probe's pop arm does.
fn combine(route: &mut Route, kind: Kind, key: usize, left: Cost, right: Cost) -> Cost {
    let left_chosen = left < right;
    route.record(kind, key, left_chosen);
    let m = if left_chosen { left } else { right };
    match kind {
        Kind::Expand => (m.0.saturating_add(1), m.1.saturating_add(1)),
        Kind::Both | Kind::FullEvNode => (m.0, m.1.saturating_add(1)),
    }
}

// ───────────── deterministic grids ─────────────

/// The adversarial event pool the deterministic grids run over.
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
/// spines, scattered ownership, and every exhaustive small-scope id
/// that owns anything.
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
    for oid in all_normal_ids(2) {
        let p = from_oracle_party(&oid);
        if !p.as_bits().is_empty() {
            pool.push(p);
        }
    }
    pool
}

/// Every event-family × party-family pair grows byte-identically to
/// the recursive oracle, validates, and probes route-identically to
/// the recursive reference.
#[test]
fn family_pairs_grow_identically() {
    let events = event_pool();
    let parties = party_pool();
    events.par_iter().for_each(|v| {
        for p in &parties {
            assert_grow(v, p);
        }
    });
}

/// Exhaustive small scope: every normal-form event tree × every
/// owning normal-form id grows byte-identically to the recursive
/// oracle AND to the brute-force right-favoring minimal inflation.
///
/// Brute force reaches every branch genre — increments, expansions at
/// every depth, both collapse directions at the inflation point, ties
/// in both cost components — deterministically rather than by
/// sampling.
#[test]
fn exhaustive_small_scope_grows_identically() {
    let events: Vec<(Version, Bits)> = all_normal_events(EV_SMALL_DEPTH)
        .iter()
        .map(|t| {
            let v = from_oracle_version(t);
            let e = encode(&v);
            (v, e)
        })
        .collect();
    let parties: Vec<Party> = all_normal_ids(ID_SMALL_DEPTH)
        .iter()
        .map(from_oracle_party)
        .filter(|p| !p.as_bits().is_empty())
        .collect();
    events.par_iter().for_each(|(v, enc)| {
        for p in &parties {
            assert_grow(v, p);
            let (best, _) = best_inflation(&to_oracle_party(p), &to_oracle_version(v))
                .expect("an owning id always inflates");
            let minimal = from_oracle_version(&best.normalized_for_test());
            assert_eq!(
                grow(enc, p),
                encode(&minimal),
                "grow must register the brute-force minimal inflation: {v} with {p}"
            );
        }
    });
}

/// The worked examples, pinned end to end: a plain increment, both
/// collapse directions at the inflation point, and one- and two-level
/// expansion chains.
#[test]
fn worked_examples_grow_exactly() {
    let cases: [(&str, &str, &str); 6] = [
        // The id owns the left half: the free increment.
        ("(1, 0)", "(0, 1, 0)", "(0, 2, 0)"),
        // Incrementing the right leaf equalizes the pair: collapse.
        ("(0, 1)", "(1, 1, 0)", "2"),
        // Incrementing the left leaf equalizes the pair: collapse.
        ("(1, 0)", "(1, 0, 1)", "2"),
        // An id node over a leaf: one expansion, grown side left.
        ("(1, 0)", "3", "(3, 1, 0)"),
        // Mirrored: grown side right.
        ("(0, 1)", "3", "(3, 0, 1)"),
        // A two-level chain, all left.
        ("((1, 0), 0)", "0", "(0, (0, 1, 0), 0)"),
    ];
    for (party, before, after) in cases {
        let p: Party = party.parse().expect("test party literals parse");
        let v: Version = before.parse().expect("test version literals parse");
        let expected: Version = after.parse().expect("test version literals parse");
        assert_eq!(
            grow(&encode(&v), &p),
            encode(&expected),
            "grow of {before} with {party} must yield {after}"
        );
        assert_grow(&v, &p);
    }
}

/// The version that is `1` on the leftmost `2^-depth` interval and `0`
/// everywhere else: `depth` nested nodes, all bases zero, the single
/// 1-leaf at the bottom left.
///
/// Built as a text literal — the parser is iterative — so the expected
/// tree shares no walk with grow.
fn left_spike(depth: usize) -> Version {
    let mut text = "(0, ".repeat(depth - 1);
    text.push_str("(0, 1, 0)");
    text.push_str(&", 0)".repeat(depth - 1));
    text.parse().expect("the spike literal is normal form")
}

/// Deep spines in every regime stay correct at depths that would
/// overflow a native-frame walk.
///
/// The regimes: the frame-count adversary (alternating spine under the
/// full id), a deep expansion chain (unary id spine over one leaf), and
/// a deep two-cursor descent (both spines together), all long before
/// the resource envelopes notice.
///
/// The recursive oracle walks on native frames, so the value witnesses
/// here are closed forms, derived per case: under the full id the
/// cheapest increment by `(expansions, depth)` is the root's right zero
/// leaf (depth 1; everything under the spine's internal child is
/// deeper), so the grown tree is the pointwise max with `(0, 0, 1)`;
/// the deep unary id over the empty version grows the expansion chain
/// to its owned tip, the left spike; and over the deep spine the same
/// id turns left into the spine's depth-2 zero leaf, so the forced
/// route raises exactly the owned region from 0 to 1 — the pointwise
/// max with the spike. Each max is realized through the
/// independently-pinned join kernel and is byte-exact by canonical
/// uniqueness; canonicality and the route pin ride along.
#[test]
fn deep_spines_grow_identically() {
    let deep_ev = version_of(&alt_spine(4096));
    let bump: Version = "(0, 0, 1)".parse().expect("test literals parse");
    let out = assert_grow_depth_safe(&deep_ev, &Party::seed());
    assert_eq!(
        out,
        encode(&(&deep_ev | &bump)),
        "the full id increments the root's right zero leaf"
    );

    let deep_id = party_of(&id_spine(4096, false));
    let spike = left_spike(4096);
    let out = assert_grow_depth_safe(&Version::new(), &deep_id);
    assert_eq!(
        out,
        encode(&spike),
        "grow expands the chain to the owned tip"
    );

    let out = assert_grow_depth_safe(&deep_ev, &deep_id);
    assert_eq!(
        out,
        encode(&(&deep_ev | &spike)),
        "the grown stream is the pointwise max with the spike"
    );
}

proptest! {
    /// Arbitrary owning parties over arbitrary normal-form versions
    /// grow byte-identically to the recursive oracle.
    ///
    /// Magnitudes past `u64::MAX` included; each pair also validates,
    /// probes route-identically, and registers exactly the brute-force
    /// minimal inflation.
    #[test]
    fn arbitrary_pairs_grow_identically(
        op in generators::arb_oracle_party_nonempty(),
        ov in generators::arb_oracle_version(),
    ) {
        let p = from_oracle_party(&op);
        let v = from_oracle_version(&ov);
        assert_grow(&v, &p);
        let (best, _) = best_inflation(&op, &ov).expect("an owning id always inflates");
        let minimal = from_oracle_version(&best.normalized_for_test());
        prop_assert_eq!(
            grow(&encode(&v), &p),
            encode(&minimal),
            "grow must register the brute-force minimal inflation: {} with {}", v, p
        );
    }

    /// Every clock produced by one organic fork/tick/send/sync/join
    /// history grows its version from its own party byte-identically
    /// to the recursive oracle — and from every *other* clock's
    /// party, the concurrent-editor shape.
    #[test]
    fn organic_histories_grow_identically(ops in optrace::world_strategy_up_to(40)) {
        let mut clocks = vec![Clock::seed()];
        for op in &ops {
            optrace::step_impl(&mut clocks, op);
        }
        for a in &clocks {
            for b in &clocks {
                if !b.party().as_bits().is_empty() {
                    assert_grow(a.version(), b.party());
                }
            }
        }
    }
}
