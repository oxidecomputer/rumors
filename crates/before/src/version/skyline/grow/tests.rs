//! Differential pins for the grow splice against three witnesses, on
//! the pairs it is reachable for.
//!
//! The splice runs exactly when the fused walk's changed flag stays
//! clear (`fill(i, e) = e`), so every grid here decides the branch per
//! pair and holds the grow-branch members to: the recursive oracle's
//! `grow` (through the bridge) as the byte-level value witness —
//! canonical uniqueness makes the differential total; a reference
//! *recursive* probe pinning the fused walk's [`Route`] bit vector —
//! the explicit guard against a walk/emit coordinate drift, which
//! would misread a direction silently rather than panic; and the
//! brute-force minimal-inflation search, holding the whole kernel to
//! `grow`'s defining optimality directly, not merely to another
//! implementation of the same dynamic program. The deterministic grids
//! pin their grow-branch pair counts exactly, so a regression that
//! silently reroutes pairs to the fill branch cannot pass vacuously.
//! The deep-spine case swaps the native-frame oracle for closed-form
//! expected values (its test doc states each derivation).

use std::sync::atomic::{AtomicUsize, Ordering};

use proptest::prelude::*;
use rayon::prelude::*;

use crate::codec::BitsMut;
use crate::codec::BitsSlice;
use crate::meter::registry::Shape;
use crate::meter::Packed;
use crate::recurse::descend;
use crate::testing::bridge::{
    from_oracle_party, from_oracle_version, to_oracle_party, to_oracle_version,
};
use crate::testing::exhaustive::{
    all_normal_events, all_normal_ids, EV_SMALL_DEPTH, ID_SMALL_DEPTH,
};
use crate::testing::grow_brute_force::best_inflation;
use crate::testing::{generators, optrace};
use crate::version::skyline::fill::{fused_fill, tick, FillOutcome};
use crate::version::skyline::{encode, validate};
use crate::{Clock, Party, Version};

use super::{id_tag, Cost, EvScan, Route, COST_MAX};

/// Lift a meter-generated packed event shape into a [`Version`].
fn version_of(p: &Packed) -> Version {
    p.version()
}

/// Decode a meter-generated packed id shape as a [`Party`].
fn party_of(p: &Packed) -> Party {
    Party::decode(&p.bytes[..]).expect("meter shapes are strict normal form")
}

/// Assert the grow branch on one pair when the pair takes it.
///
/// The
/// fused walk's route must equal the recursive reference probe's bit
/// for bit, and the ticked stream must validate and register the
/// recursive oracle's inflation byte for byte; the return says
/// whether the pair took the grow branch, so the deterministic grids
/// can pin their coverage counts.
fn assert_grow(v: &Version, p: &Party) -> bool {
    match assert_grow_depth_safe(v, p) {
        None => false,
        Some(out) => {
            let (raw, _) = to_oracle_version(v).grow_for_test(&to_oracle_party(p));
            assert_eq!(
                out,
                encode(&from_oracle_version(&raw.normalized_for_test())),
                "grow must register the recursive oracle's inflation: {v} with {p}"
            );
            true
        }
    }
}

/// The depth-safe half of [`assert_grow`].
///
/// Runs the branch decision,
/// the route pin, and canonicality (the fused walk, the reference
/// probe, and the splice are all stack-guarded or iterative),
/// returning the ticked stream on the grow branch.
///
/// The recursive oracle walks on native frames, so the deep-spine test
/// calls this directly and takes its value witnesses from closed forms
/// instead.
fn assert_grow_depth_safe(v: &Version, p: &Party) -> Option<BitsMut> {
    let enc = encode(v);
    match fused_fill(&enc, p) {
        // fill moved the tree: the splice is unreachable for this pair.
        FillOutcome::Changed(_) => None,
        FillOutcome::Unchanged(route) => {
            let (reference, _) = reference_probe(enc.as_bitslice(), p.as_bits());
            assert_eq!(
                route.dirs(),
                reference.dirs(),
                "the fused walk's route must match the recursive reference bit for bit: \
                 {v} with {p}"
            );
            let out = tick(&enc, p);
            validate(&out).expect("a grown stream is canonical");
            Some(out)
        }
    }
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
/// shape — the transliteration of the recursive walk whose route fold
/// the fused tick walk carries, kept as its structural witness.
fn reference_probe(ev_bits: &BitsSlice, id_bits: &BitsSlice) -> (Route, Cost) {
    let mut route = Route::new(id_bits.len());
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
            if ev.read().is_none() {
                // The reference is driven only on grow-branch pairs,
                // where a fully-owned region is always a single leaf:
                // fill(1, node) collapses it and trips the flag.
                unreachable!("a full id over an event node collapses under fill");
            }
            (0, 0)
        }
        RefId::At => {
            let key = *id_pos;
            let (l, r) = id_tag(id_bits, *id_pos);
            *id_pos += 2;
            if !l && !r {
                return rec(route, ev, id_bits, id_pos, RefId::Full, ev_zero, depth);
            }
            let expand = ev_zero || ev.read().is_some();
            let la = if l { RefId::At } else { RefId::Empty };
            let lc = descend!(
                depth + 1,
                rec(route, ev, id_bits, id_pos, la, expand, depth + 1)
            );
            let ra = if r { RefId::At } else { RefId::Empty };
            let rc = descend!(
                depth + 1,
                rec(route, ev, id_bits, id_pos, ra, expand, depth + 1)
            );
            combine(route, expand, key, lc, rc)
        }
    }
}

/// Pick the cheaper child, record the direction, and fold the branch
/// cost — one expansion and one depth per expansion-chain level, one
/// depth otherwise — exactly as the fused walk's fold does.
fn combine(route: &mut Route, expand: bool, key: usize, left: Cost, right: Cost) -> Cost {
    let left_chosen = left < right;
    route.record(key, left_chosen);
    let m = if left_chosen { left } else { right };
    if expand {
        (m.0.saturating_add(1), m.1.saturating_add(1))
    } else {
        (m.0, m.1.saturating_add(1))
    }
}

// ───────────── deterministic grids ─────────────

/// The adversarial event pool the deterministic grids run over.
fn event_pool() -> Vec<Version> {
    vec![
        Version::new(),
        version_of(&Shape::Dense.packed1(1)),
        version_of(&Shape::Dense.packed1(2)),
        version_of(&Shape::Dense.packed1(64)),
        version_of(&Shape::Bigroot.packed2(7, 3)),
        version_of(&Shape::Bigroot.packed2(64, 16)),
        version_of(&Shape::Hugeleaf.packed1(1)),
        version_of(&Shape::Hugeleaf.packed1(64)),
        version_of(&Shape::CliffComb.packed2(3, 2)),
        version_of(&Shape::CliffComb.packed2(16, 16)),
        version_of(&Shape::WideToothComb.packed3(16, 8, 8)),
        version_of(&Shape::CliffFan.packed2(16, 8)),
        version_of(&Shape::CancellingChain.packed2(16, 8)),
        version_of(&Shape::AltSpine.packed1(3)),
        version_of(&Shape::AltSpine.packed1(64)),
        version_of(&Shape::Harmonic.packed1(16)),
    ]
}

/// The adversarial party pool: the seed, deep and diverted unary
/// spines, scattered ownership, and every exhaustive small-scope id
/// that owns anything.
fn party_pool() -> Vec<Party> {
    let mut pool = vec![
        Party::seed(),
        party_of(&Shape::IdSpine.packed_flagged(1, false)),
        party_of(&Shape::IdSpine.packed_flagged(3, false)),
        party_of(&Shape::IdSpine.packed_flagged(3, true)),
        party_of(&Shape::IdSpine.packed_flagged(64, false)),
        party_of(&Shape::IdSpine.packed_flagged(64, true)),
        party_of(&Shape::ScatteredId.packed1(1)),
        party_of(&Shape::ScatteredId.packed1(16)),
    ];
    for oid in all_normal_ids(2) {
        let p = from_oracle_party(&oid);
        if !p.as_bits().is_empty() {
            pool.push(p);
        }
    }
    pool
}

/// The grow-branch pair count the family grid must reach.
///
/// The grids'
/// pools are deterministic, so the count is exact, and a regression
/// that reroutes pairs to the fill branch (vacuously passing the
/// per-pair asserts) moves this pin.
const FAMILY_GROW_PAIRS: usize = 182;

/// Every grow-branch event-family × party-family pair ticks
/// byte-identically to the recursive oracle's inflation.
///
/// Each pair
/// also validates and probes route-identically to the recursive
/// reference, and the grow-branch coverage count is pinned exactly.
#[test]
fn family_pairs_grow_identically() {
    let events = event_pool();
    let parties = party_pool();
    let taken = AtomicUsize::new(0);
    events.par_iter().for_each(|v| {
        for p in &parties {
            if assert_grow(v, p) {
                taken.fetch_add(1, Ordering::Relaxed);
            }
        }
    });
    assert_eq!(
        taken.load(Ordering::Relaxed),
        FAMILY_GROW_PAIRS,
        "the family grid's grow-branch coverage moved: re-derive the pin \
         from the deterministic pools"
    );
}

/// The grow-branch pair count the exhaustive grid must reach (see
/// [`FAMILY_GROW_PAIRS`]).
const EXHAUSTIVE_GROW_PAIRS: usize = 114_621;

/// Exhaustive small scope: every grow-branch pair grows identically
/// to the oracle and the brute force.
///
/// Each normal-form event tree ×
/// owning normal-form id on the grow branch must tick byte-identically
/// to the recursive oracle's inflation AND to the brute-force
/// right-favoring minimal inflation, with the coverage count pinned
/// exactly.
///
/// Brute force reaches every reachable branch genre — increments,
/// expansions at every depth, ties in both cost components —
/// deterministically rather than by sampling.
#[test]
fn exhaustive_small_scope_grows_identically() {
    let events: Vec<Version> = all_normal_events(EV_SMALL_DEPTH)
        .iter()
        .map(from_oracle_version)
        .collect();
    let parties: Vec<Party> = all_normal_ids(ID_SMALL_DEPTH)
        .iter()
        .map(from_oracle_party)
        .filter(|p| !p.as_bits().is_empty())
        .collect();
    let taken = AtomicUsize::new(0);
    events.par_iter().for_each(|v| {
        for p in &parties {
            if let Some(out) = assert_grow_depth_safe(v, p) {
                taken.fetch_add(1, Ordering::Relaxed);
                let (raw, _) = to_oracle_version(v).grow_for_test(&to_oracle_party(p));
                assert_eq!(
                    out,
                    encode(&from_oracle_version(&raw.normalized_for_test())),
                    "grow must register the recursive oracle's inflation: {v} with {p}"
                );
                let (best, _) = best_inflation(&to_oracle_party(p), &to_oracle_version(v))
                    .expect("an owning id always inflates");
                let minimal = from_oracle_version(&best.normalized_for_test());
                assert_eq!(
                    out,
                    encode(&minimal),
                    "grow must register the brute-force minimal inflation: {v} with {p}"
                );
            }
        }
    });
    assert_eq!(
        taken.load(Ordering::Relaxed),
        EXHAUSTIVE_GROW_PAIRS,
        "the exhaustive grid's grow-branch coverage moved: re-derive the pin \
         from the enumerated scope"
    );
}

/// The worked examples, pinned end to end: a plain increment and one-
/// and two-level expansion chains, on pairs whose fill is the identity.
///
/// The increment-equalizes-collapse genre has no member here: making
/// the grown leaf equal its leaf sibling requires ownership of a leaf
/// sitting one below that sibling, and fill's raise preempts exactly
/// that configuration (the owned leaf is lifted to the sibling range's
/// minimum first), so such pairs take the fill branch and the splice
/// never sees them; the exhaustive grid holds every *reachable* pair
/// to the oracle either way.
#[test]
fn worked_examples_grow_exactly() {
    let cases: [(&str, &str, &str); 4] = [
        // The id owns the left half: the free increment.
        ("(1, 0)", "(0, 1, 0)", "(0, 2, 0)"),
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
            tick(&encode(&v), &p),
            encode(&expected),
            "grow of {before} with {party} must yield {after}"
        );
        assert!(
            assert_grow(&v, &p),
            "the worked grow examples are grow-branch pairs by construction"
        );
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

/// Deep spines in the grow branch stay correct at depths that would
/// overflow a native-frame walk.
///
/// The regimes: a deep expansion chain (unary id spine over one leaf)
/// and a deep two-cursor descent mixing into an id-only expansion
/// where the id outruns the event, all long before the resource
/// envelopes notice.
///
/// The recursive oracle walks on native frames, so the value witnesses
/// here are closed forms, derived per case: the deep unary id over the
/// empty version grows the expansion chain to its owned tip, the left
/// spike; and over the deep alternating spine the same id turns left
/// into the spine's depth-2 zero leaf, so the forced route raises
/// exactly the owned region from 0 to 1 — the pointwise max with the
/// spike. Each max is realized through the independently-pinned join
/// kernel and is byte-exact by canonical uniqueness; canonicality and
/// the route pin (the reference probe is stack-guarded) ride along.
#[test]
fn deep_spines_grow_identically() {
    let deep_id = party_of(&Shape::IdSpine.packed_flagged(4096, false));
    let spike = left_spike(4096);
    let out = assert_grow_depth_safe(&Version::new(), &deep_id)
        .expect("a leaf under a node id is a grow-branch pair");
    assert_eq!(
        out,
        encode(&spike),
        "grow expands the chain to the owned tip"
    );

    let deep_ev = version_of(&Shape::AltSpine.packed1(4096));
    let out = assert_grow_depth_safe(&deep_ev, &deep_id)
        .expect("no full-id region meets a subdividable subtree: a grow-branch pair");
    assert_eq!(
        out,
        encode(&(&deep_ev | &spike)),
        "the grown stream is the pointwise max with the spike"
    );
}

proptest! {
    /// Arbitrary owning parties over arbitrary normal-form versions
    /// grow identically to the oracle and the brute force.
    ///
    /// Every
    /// grow-branch pair must tick byte-identically to the recursive
    /// oracle's inflation and to the brute-force minimal inflation,
    /// with the route pinned against the recursive reference.
    ///
    /// Magnitudes past `u64::MAX` included. The branch split itself is
    /// pinned by the fill suite's flag differential, so a fill-branch
    /// draw here is covered coverage, not a skip.
    #[test]
    fn arbitrary_pairs_grow_identically(
        op in generators::arb_oracle_party_nonempty(),
        ov in generators::arb_oracle_version(),
    ) {
        let p = from_oracle_party(&op);
        let v = from_oracle_version(&ov);
        if assert_grow(&v, &p) {
            let (best, _) = best_inflation(&op, &ov).expect("an owning id always inflates");
            let minimal = from_oracle_version(&best.normalized_for_test());
            prop_assert_eq!(
                tick(&encode(&v), &p),
                encode(&minimal),
                "grow must register the brute-force minimal inflation: {} with {}", v, p
            );
        }
    }

    /// Organic histories grow byte-identically to the recursive
    /// oracle on the grow-branch pairs they produce.
    ///
    /// Every clock from
    /// one fork/tick/send/sync/join history is exercised from its own
    /// party — and from every *other* clock's party, the
    /// concurrent-editor shape.
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
