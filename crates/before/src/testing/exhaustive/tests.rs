//! Exhaustive small-scope differential tests.
//!
//! Each `check_*` helper runs one op family over the *entire* enumerated corpus
//! (every tree, every ordered pair) and diffs the impl against the recursive
//! oracle — the same structural-agreement contract the sampled differentials
//! use, but total rather than random. Every impl reading goes through the
//! **public ops** (`fork`, `join`, `without`, `is_disjoint`, `covers`, `tick`,
//! the codec, the event operators), so the suite pins the surface callers
//! invoke — a drift between an internal walk and its public routing cannot
//! pass unseen. Two consequences of that choice:
//!
//! - **The anonymous id sits out the op checks.** The enumeration includes the
//!   empty tree, but a standalone [`Party`] is never anonymous (nothing public
//!   constructs one), so the corpus lowering drops it up front — once, not as a
//!   per-row test inside the billion-pair loops — matching the ops' public
//!   domain. The empty *region* is still exercised everywhere it publicly
//!   occurs: as the absent child inside every non-trivial pair.
//!
//! - **Mutating/consuming contracts are checked as such.** `join` mutates its
//!   receiver and consumes its operand, `without` consumes its receiver, `fork`
//!   mutates — so those checks duplicate the borrowed corpus entry per pair with
//!   [`Party::dangerously_alias`] (the public escape hatch for exactly this
//!   handoff shape) and assert the full public contract, including `join`'s
//!   leave-self-unmodified/hand-back on overlap.
//!
//! The cross-product is the whole point, so it is never sampled (that is what
//! the property tests are for); instead two things keep it tractable:
//!
//! - **Precompute once.** Each oracle tree is lowered to its impl form a single
//!   time into a `Vec<Party>` / `Vec<Version>` that the pair loops *borrow*,
//!   rather than re-lowering both operands inside the inner loop (which, at the
//!   deep bound, would be billions of allocations).
//!
//! - **Parallelize.** The outer loop of every check runs on a `rayon` thread
//!   pool; a failing `assert!` in a worker propagates as a panic when the
//!   parallel region joins, so the test semantics are unchanged. The `step!()`
//!   metric is a `thread_local`, so parallel traversals do not contend (and these
//!   tests do not read it).
//!
//! The two entry points wire the helpers to decoupled id/event depth bounds
//! (events grow far faster, so they are held a level shallower — see the parent
//! module doc): the gate-resident [`exhaustive_small`] at [`ID_SMALL_DEPTH`] /
//! [`EV_SMALL_DEPTH`] runs every helper, and the `#[ignore]`d
//! [`exhaustive_deep`] at [`ID_DEEP_DEPTH`] / [`EV_DEEP_DEPTH`] runs all but
//! the structural pair legs (`join`/`without`), which stay at the small bound
//! — the parent module doc states the split and where the deep structural
//! coverage lives instead.
//!
//! Op *symmetry* (`is_disjoint` symmetric, `join` commutative, event
//! `partial_cmp` anti-symmetric) is NOT relied on to skip half the pairs and is
//! NOT checked here; it is an intrinsic algebraic property of the impl, tested
//! directly and oracle-independently in the "intrinsic symmetry laws" section
//! at the bottom of this file.

use std::cmp::Ordering;

use rayon::prelude::*;

use super::{
    all_normal_events, all_normal_ids, EV_DEEP_DEPTH, EV_SMALL_DEPTH, ID_DEEP_DEPTH, ID_SMALL_DEPTH,
};
use crate::oracle;
use crate::testing::bridge::{
    from_oracle_party, from_oracle_version, to_oracle_party, to_oracle_version,
};
use crate::testing::grow_brute_force::{all_inflations, best_inflation};
use crate::{Party, Version};

/// `a <= b` under the impl event causal order (concurrency is not-`<=`).
fn ev_le(a: &Version, b: &Version) -> bool {
    a.partial_cmp(b).is_some_and(|o| o != Ordering::Greater)
}

/// Run `body(i, j)` for every ordered pair `(i, j)` in `0..n × 0..n`, with the outer index
/// parallelized across the `rayon` pool. The diagonal (`i == j`) is included — the reflexive
/// cases are deliberate coverage.
fn par_for_pairs(n: usize, body: impl Fn(usize, usize) + Sync) {
    (0..n).into_par_iter().for_each(|i| {
        for j in 0..n {
            body(i, j);
        }
    });
}

/// Lower the enumerated oracle ids to their impl `Party` forms, once, dropping
/// the anonymous tree.
///
/// A standalone `Party` is never anonymous — see the module doc. Used by the
/// intrinsic symmetry laws, which have no oracle side to filter against.
fn impl_ids(depth: usize) -> Vec<Party> {
    all_normal_ids(depth)
        .iter()
        .filter(|t| !t.is_empty())
        .map(from_oracle_party)
        .collect()
}

/// Lower the enumerated oracle events to their impl `Version` forms, once.
fn impl_events(depth: usize) -> Vec<Version> {
    all_normal_events(depth)
        .iter()
        .map(from_oracle_version)
        .collect()
}

// ───────────────────────────── id op families ─────────────────────────────

/// Every id tree round-trips: the impl is itself normal form (`decode` accepts
/// only canonical bits), and lowering after `decode∘encode` recovers the same
/// oracle tree.
fn check_id_codec(ids: &[oracle::Party], imp: &[Party]) {
    (0..ids.len()).into_par_iter().for_each(|i| {
        let oa = &ids[i];
        let p = &imp[i];
        let bytes = p.encode();
        let decoded = Party::decode(&bytes[..]).expect("canonical id encoding decodes");
        assert!(&decoded == p, "id decode∘encode is not identity for {oa:?}");
        assert_eq!(to_oracle_party(&decoded), *oa);
    });
}

/// [`Party::fork`] on every standalone id matches the oracle's fork on both
/// halves, structurally: the kept half replaces the receiver, the given half is
/// returned.
fn check_id_fork(ids: &[oracle::Party], imp: &[Party]) {
    (0..ids.len()).into_par_iter().for_each(|i| {
        let oa = &ids[i];
        let mut oracle_keep = oa.clone();
        let oracle_give = oracle_keep.fork();

        let mut keep = imp[i].dangerously_alias();
        let give = keep.fork();
        assert!(keep == from_oracle_party(&oracle_keep), "fork keep {oa:?}");
        assert!(give == from_oracle_party(&oracle_give), "fork give {oa:?}");
    });
}

/// [`Party::is_disjoint`] and [`Party::covers`] over every *ordered pair* of
/// standalone ids agree with the oracle.
///
/// The cross-product reaches the overlap (`is_disjoint == false`) and partial-
/// and non-overlap (`covers == false`) arms exhaustively. Both verdicts read
/// borrowed operands and allocate nothing — the only pair legs cheap enough
/// to cross the deep bound at all (see the parent module doc's leg split) —
/// yet the deep `corpus²` product is still the dominant phase of
/// [`exhaustive_deep`], whose doc carries the measured state.
fn check_id_pair_verdicts(ids: &[oracle::Party], imp: &[Party]) {
    par_for_pairs(ids.len(), |i, j| {
        let (oa, ob) = (&ids[i], &ids[j]);
        let (ia, ib) = (&imp[i], &imp[j]);
        assert_eq!(
            ia.is_disjoint(ib),
            oa.is_disjoint(ob),
            "is_disjoint {oa:?} {ob:?}"
        );
        assert_eq!(ia.covers(ib), oa.covers(ob), "covers {oa:?} {ob:?}");
    });
}

/// [`Party::join`] and [`Party::without`] over every *ordered pair* of
/// standalone ids agree with the oracle, structurally, through the full
/// public contracts.
///
/// The contracts, exactly: `join` reunites exactly the disjoint pairs and, on
/// overlap, leaves the receiver unmodified and hands the operand back;
/// `without` returns the remainder region, or `None` exactly when nothing
/// remains (the empty region is not a `Party`).
///
/// The cross-product reaches the overlap-join (`Err`) and covered-difference
/// (`None`) arms exhaustively. Both ops build a result tree per pair (and the
/// operands are duplicated per pair, since `join` mutates and consumes and
/// `without` consumes), so this check runs at the small bound only — the
/// parent module doc states the split.
fn check_id_pair_algebra(ids: &[oracle::Party], imp: &[Party]) {
    par_for_pairs(ids.len(), |i, j| {
        let (oa, ob) = (&ids[i], &ids[j]);
        let (ia, ib) = (&imp[i], &imp[j]);

        let mut joined = ia.dangerously_alias();
        match joined.join(ib.dangerously_alias()) {
            Ok(()) => {
                assert!(
                    oa.is_disjoint(ob),
                    "join accepted an overlapping pair: {oa:?} {ob:?}"
                );
                let mut oracle_join = oa.clone();
                oracle_join
                    .join(ob.clone())
                    .expect("the oracle agrees the pair is disjoint");
                assert!(
                    joined == from_oracle_party(&oracle_join),
                    "join {oa:?} {ob:?}"
                );
            }
            Err(back) => {
                assert!(
                    !oa.is_disjoint(ob),
                    "join refused a disjoint pair: {oa:?} {ob:?}"
                );
                assert!(
                    joined == *ia,
                    "a refused join must leave the receiver unmodified: {oa:?} {ob:?}"
                );
                assert!(
                    back == *ib,
                    "a refused join must hand the operand back: {oa:?} {ob:?}"
                );
            }
        }

        let oracle_diff = oa.without(ob);
        match ia.dangerously_alias().without(ib) {
            Some(rest) => {
                assert!(
                    !oracle_diff.is_empty(),
                    "without found a remainder the oracle says is empty: {oa:?} \\ {ob:?}"
                );
                assert!(
                    rest == from_oracle_party(&oracle_diff),
                    "without {oa:?} \\ {ob:?}"
                );
            }
            None => {
                assert!(
                    oracle_diff.is_empty(),
                    "without found nothing but the oracle finds a remainder: {oa:?} \\ {ob:?}"
                );
            }
        }
    });
}

// ───────────────────────────── event op families ─────────────────────────────

/// Every event tree round-trips through the widened codec and lowers back to the same
/// oracle value.
fn check_ev_codec(evs: &[oracle::Version], imp: &[Version]) {
    (0..evs.len()).into_par_iter().for_each(|i| {
        let ov = &evs[i];
        let v = &imp[i];
        let bytes = v.encode();
        let decoded = Version::decode(&bytes[..]).expect("canonical event encoding decodes");
        assert!(
            decoded == v,
            "event decode∘encode is not identity for {ov:?}"
        );
        assert_eq!(to_oracle_version(&decoded), *ov);
    });
}

/// `partial_cmp`, `|` (merge / LUB), and `&` (meet / GLB) over every *ordered
/// pair* of events agree with the oracle, structurally.
///
/// Reaching the concurrent (`None`) verdict and the join/meet arm selection on
/// shapes the op pipeline never builds.
fn check_ev_pairs(evs: &[oracle::Version], imp: &[Version]) {
    par_for_pairs(evs.len(), |i, j| {
        let (oa, ob) = (&evs[i], &evs[j]);
        let (ia, ib) = (&imp[i], &imp[j]);

        assert_eq!(ia.partial_cmp(ib), oa.partial_cmp(ob), "cmp {oa:?} {ob:?}");

        let merged = ia.clone() | ib.clone();
        let oracle_join = oa.clone() | ob.clone();
        assert!(
            merged == from_oracle_version(&oracle_join),
            "merge {oa:?} | {ob:?}"
        );
        assert_eq!(to_oracle_version(&merged), oracle_join);

        let met = ia.clone() & ib.clone();
        let oracle_meet = oa.clone() & ob.clone();
        assert!(
            met == from_oracle_version(&oracle_meet),
            "meet {oa:?} & {ob:?}"
        );
        assert_eq!(to_oracle_version(&met), oracle_meet);
    });
}

// ───────────────────────── (id, event) op families ─────────────────────────

/// `tick` (= `fill` then, on no fill, `grow`) over every (non-empty id, event)
/// pair matches the oracle's `event`.
///
/// When the pair takes the `grow` branch, the impl's inflation is additionally
/// pinned to the brute-force cost-minimal, right-favoring region
/// ([`best_inflation`]) — holding the packed `grow`'s DP to the global optimum
/// directly, not merely to the oracle that realizes the same DP — and the
/// metamorphic minimality condition (no feasible candidate sits strictly
/// between `e` and `e'`) is checked on the impl's own causal order.
fn check_tick(
    ids: &[oracle::Party],
    imp_ids: &[Party],
    evs: &[oracle::Version],
    imp_evs: &[Version],
) {
    (0..ids.len()).into_par_iter().for_each(|i| {
        let op = &ids[i];
        let ip = &imp_ids[i];
        for j in 0..evs.len() {
            let ov = &evs[j];
            // Differential: impl `tick` == oracle `event`.
            let mut oracle_after = ov.clone();
            oracle_after.tick(op);

            let e = &imp_evs[j];
            let mut eprime = e.clone();
            eprime.tick(ip);
            assert!(
                eprime == from_oracle_version(&oracle_after),
                "tick {op:?} on {ov:?}"
            );

            // Grow-branch only: pin the inflation to the global brute-force optimum.
            if ov.fill_for_test(op) != *ov {
                continue; // fill simplified the tree; grow was not taken
            }
            let (best_tree, _cost) = best_inflation(op, ov).expect("non-empty id inflates");
            assert_eq!(
                to_oracle_version(&eprime),
                best_tree.normalized_for_test(),
                "grow chose a non-minimal inflation for {op:?} on {ov:?}",
            );

            // Metamorphic minimality on the impl: no candidate `x` with `e ≤ x < e'`.
            for (cand, _) in all_inflations(op, ov) {
                let cand_v = from_oracle_version(&cand.normalized_for_test());
                let above_e = ev_le(e, &cand_v);
                let strictly_below = cand_v.partial_cmp(&eprime) == Some(Ordering::Less);
                assert!(
                    !(above_e && strictly_below),
                    "an inflation candidate sits strictly between e and e' for {op:?} on {ov:?}",
                );
            }
        }
    });
}

// ─────────────────────────────── drivers ───────────────────────────────

/// The enumerated corpora at the given bounds, each lowered to its impl form
/// once so the pair loops borrow rather than re-lower.
///
/// The two corpora grow at different rates, so their bounds are decoupled — see
/// the parent module doc. The anonymous (empty) id is dropped here, up front:
/// a standalone `Party` is never anonymous (module doc), and testing emptiness
/// per row instead would put two boxed-tree walks inside the deep bound's
/// ~4.3-billion-iteration pair loops.
#[allow(clippy::type_complexity)]
fn corpora_at(
    id_depth: usize,
    ev_depth: usize,
) -> (
    Vec<oracle::Party>,
    Vec<Party>,
    Vec<oracle::Version>,
    Vec<Version>,
) {
    let ids: Vec<oracle::Party> = all_normal_ids(id_depth)
        .into_iter()
        .filter(|t| !t.is_empty())
        .collect();
    let evs = all_normal_events(ev_depth);
    let imp_ids: Vec<Party> = ids.iter().map(from_oracle_party).collect();
    let imp_evs: Vec<Version> = evs.iter().map(from_oracle_version).collect();
    (ids, imp_ids, evs, imp_evs)
}

/// Sanity-check that the enumeration deduplicates to canonical normal form:
/// every enumerated id and event is `is_normal`, and the corpus has no
/// duplicates.
///
/// The de-dup key is injective over canonical trees, so equal trees would have
/// collided.
#[test]
fn corpus_is_canonical() {
    let ids = all_normal_ids(ID_SMALL_DEPTH);
    for p in &ids {
        assert!(p.is_normal(), "enumerated id not normal: {p:?}");
    }
    let evs = all_normal_events(EV_SMALL_DEPTH);
    for v in &evs {
        assert!(v.is_normal(), "enumerated event not normal: {v:?}");
    }
    // The corpus is non-trivial (guards against an enumeration that silently produces
    // nothing and makes every cross-product loop vacuous).
    assert!(
        ids.len() > 20,
        "id corpus suspiciously small: {}",
        ids.len()
    );
    assert!(
        evs.len() > 20,
        "event corpus suspiciously small: {}",
        evs.len()
    );
}

/// Every operation, on every enumerated tree and ordered pair, agrees with the
/// oracle at the small depth bound.
///
/// Deterministic coverage of the close-up corners (root-tie `grow`, empty-child
/// spine, `close_node` adjacency, overlap/concurrent verdicts) that random
/// sampling under-hits. Runs in the normal gate.
#[test]
fn exhaustive_small() {
    let (ids, imp_ids, evs, imp_evs) = corpora_at(ID_SMALL_DEPTH, EV_SMALL_DEPTH);

    check_id_codec(&ids, &imp_ids);
    check_id_fork(&ids, &imp_ids);
    check_id_pair_verdicts(&ids, &imp_ids);
    check_id_pair_algebra(&ids, &imp_ids);

    check_ev_codec(&evs, &imp_evs);
    check_ev_pairs(&evs, &imp_evs);

    check_tick(&ids, &imp_ids, &evs, &imp_evs);
}

/// The total cross-product at the deep depth bound: every check but the
/// structural pair legs, which stay at the small bound (see the parent module
/// doc for the split and where deep structural coverage lives).
///
/// The id corpus jumps to 65536 trees, so the `O(corpus²)` id verdict
/// pair-product (~4.3 billion pairs) dominates everything else — the per-id
/// checks are seconds, and the `tick` grid (65536 ids × 691 events with the
/// brute-force minimality pin) extrapolates linearly along its id axis to
/// about a minute. Measured state (2026-07-27, aarch64-apple-darwin, 16
/// cores, release, quiet machine): two fully parallel runs were stopped at a
/// 45-minute cap, one row-major and one cache-tiled, the row-major one
/// profiled still inside the verdict pair product at 31 minutes — budget
/// upwards of an hour and run it detached. Sampled-corpus extrapolation
/// undershoots this product badly (a 268-million-pair stride sample prices
/// it at under a minute): the expensive verdict pairs are *structurally
/// similar* trees, which the full cross-product contains in every
/// near-diagonal block and a strided sample quadratically thins out. It is
/// `#[ignore]`d to keep the normal gate fast. Run it with:
///
/// ```text
/// cargo test -p before --release --all-features -- --ignored exhaustive_deep
/// ```
///
/// (`cargo test`, not nextest: the workspace's nextest profile terminates
/// any test at 180 seconds, which this enumeration exceeds).
#[test]
#[ignore = "exhaustive deep enumeration: O(corpus^2) over 65536 ids; hour-scale, run detached"]
fn exhaustive_deep() {
    let (ids, imp_ids, evs, imp_evs) = corpora_at(ID_DEEP_DEPTH, EV_DEEP_DEPTH);

    check_id_codec(&ids, &imp_ids);
    check_id_fork(&ids, &imp_ids);
    check_id_pair_verdicts(&ids, &imp_ids);

    check_ev_codec(&evs, &imp_evs);
    check_ev_pairs(&evs, &imp_evs);

    check_tick(&ids, &imp_ids, &evs, &imp_evs);
}

// ───────────────────────────── intrinsic symmetry laws ─────────────────────────────
//
// The op symmetries are intrinsic algebraic properties of the impl, so they are
// tested DIRECTLY on the impl — no oracle, and not folded into the differential
// checks above. Two payoffs: a symmetry bug the oracle happened to *share* is
// still caught here, and (being deterministic + total over the small-scope
// corpus) the guarantee is total, not sampled.

/// `is_disjoint` is symmetric: `a.is_disjoint(b) == b.is_disjoint(a)` for every
/// ordered pair of enumerated ids (including the reflexive `a == b` diagonal).
#[test]
fn id_is_disjoint_is_symmetric() {
    let imp = impl_ids(ID_SMALL_DEPTH);
    par_for_pairs(imp.len(), |i, j| {
        assert_eq!(
            imp[i].is_disjoint(&imp[j]),
            imp[j].is_disjoint(&imp[i]),
            "is_disjoint not symmetric at ({i}, {j})",
        );
    });
}

/// `join` is commutative: `a.join(b)` and `b.join(a)` agree on acceptance
/// (both `Err` exactly when the ids overlap), and the accepted unions are
/// equal, over every ordered pair of enumerated ids.
#[test]
fn id_join_is_commutative() {
    let imp = impl_ids(ID_SMALL_DEPTH);
    par_for_pairs(imp.len(), |i, j| {
        let mut ab = imp[i].dangerously_alias();
        let ra = ab.join(imp[j].dangerously_alias());
        let mut ba = imp[j].dangerously_alias();
        let rb = ba.join(imp[i].dangerously_alias());
        assert_eq!(
            ra.is_ok(),
            rb.is_ok(),
            "join acceptance not symmetric at ({i}, {j})",
        );
        if ra.is_ok() {
            assert!(ab == ba, "join not commutative at ({i}, {j})");
        }
    });
}

/// event `partial_cmp` is anti-symmetric, over every ordered pair of enumerated
/// events.
#[test]
fn event_partial_cmp_is_antisymmetric() {
    let imp = impl_events(EV_SMALL_DEPTH);
    par_for_pairs(imp.len(), |i, j| {
        assert_eq!(
            imp[i].partial_cmp(&imp[j]),
            imp[j].partial_cmp(&imp[i]).map(Ordering::reverse),
            "event partial_cmp not anti-symmetric at ({i}, {j})",
        );
    });
}

/// event merge (`|`, the join / least upper bound) is commutative: `a | b == b
/// | a`, over every ordered pair of enumerated events.
#[test]
fn event_merge_is_commutative() {
    let imp = impl_events(EV_SMALL_DEPTH);
    par_for_pairs(imp.len(), |i, j| {
        let ab = imp[i].clone() | imp[j].clone();
        let ba = imp[j].clone() | imp[i].clone();
        assert!(ab == ba, "event merge not commutative at ({i}, {j})");
    });
}

/// event meet (`&`, the meet / greatest lower bound) is commutative: `a & b ==
/// b & a`, over every ordered pair of enumerated events. The dual of
/// [`event_merge_is_commutative`].
#[test]
fn event_meet_is_commutative() {
    let imp = impl_events(EV_SMALL_DEPTH);
    par_for_pairs(imp.len(), |i, j| {
        let ab = imp[i].clone() & imp[j].clone();
        let ba = imp[j].clone() & imp[i].clone();
        assert!(ab == ba, "event meet not commutative at ({i}, {j})");
    });
}
