//! Exhaustive small-scope differential tests.
//!
//! Each `check_*` helper runs one op family over the *entire* enumerated corpus
//! (every tree, every ordered pair) and diffs the impl against the recursive
//! oracle — the same structural-agreement contract the sampled differentials
//! use, but total rather than random. The cross-product is the whole point, so
//! it is never sampled (that is what the property tests are for); instead two
//! things keep it tractable:
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
//! module doc): the gate-resident `exhaustive_small` at [`ID_SMALL_DEPTH`] /
//! [`EV_SMALL_DEPTH`], and the `#[ignore]`d `exhaustive_deep` at
//! [`ID_DEEP_DEPTH`] / [`EV_DEEP_DEPTH`].
//!
//! Op *symmetry* (`is_disjoint` symmetric, `sum`/merge commutative,
//! `partial_cmp` anti-symmetric) is NOT relied on to skip half the pairs and is
//! NOT checked here; it is an intrinsic algebraic property of the impl, tested
//! directly and oracle-independently in the "intrinsic symmetry laws" section
//! at the bottom of this file.

use std::cmp::Ordering;

use rayon::prelude::*;

use super::{
    all_normal_events, all_normal_ids, EV_DEEP_DEPTH, EV_SMALL_DEPTH, ID_DEEP_DEPTH, ID_SMALL_DEPTH,
};
use crate::idbits::IdView;
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

/// Lower the enumerated oracle ids to their impl `Party` forms, once.
fn impl_ids(depth: usize) -> Vec<Party> {
    all_normal_ids(depth)
        .iter()
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

/// Every id tree round-trips: the impl is itself normal form (`decode` accepts only
/// canonical bits), and lowering after `decode∘encode` recovers the same oracle tree. The
/// anonymous id is excluded (a standalone `Party` must own a region; `decode` rejects `0`).
fn check_id_codec(ids: &[oracle::Party], imp: &[Party]) {
    (0..ids.len()).into_par_iter().for_each(|i| {
        let oa = &ids[i];
        if oa.is_empty() {
            return;
        }
        let p = &imp[i];
        let bytes = p.encode();
        let decoded = Party::decode(&bytes[..]).expect("canonical id encoding decodes");
        assert!(&decoded == p, "id decode∘encode is not identity for {oa:?}");
        assert_eq!(to_oracle_party(&decoded), *oa);
    });
}

/// `split` (the structural op behind `fork`) on every non-empty id matches the oracle's
/// `split` on both halves, structurally.
fn check_id_split(ids: &[oracle::Party], imp: &[Party]) {
    (0..ids.len()).into_par_iter().for_each(|i| {
        let oa = &ids[i];
        if oa.is_empty() {
            return;
        }
        let mut oracle_self = oa.clone();
        let oracle_give = oracle_self.fork(); // fork == split; mutates self to the kept half

        let (keep_bits, give_bits) = IdView(imp[i].as_bits()).split();
        assert!(Party::from_bits(keep_bits) == from_oracle_party(&oracle_self));
        assert!(Party::from_bits(give_bits) == from_oracle_party(&oracle_give));
    });
}

/// `partial_cmp`, `is_disjoint`, and `sum` over every *ordered pair* of ids agree with the
/// oracle — reaching the overlap (`is_disjoint == false`), incomparable (`partial_cmp ==
/// None`), and overlap-sum (`None`) arms exhaustively.
fn check_id_pairs(ids: &[oracle::Party], imp: &[Party]) {
    par_for_pairs(ids.len(), |i, j| {
        let (oa, ob) = (&ids[i], &ids[j]);
        let (ia, ib) = (&imp[i], &imp[j]);

        assert_eq!(ia.partial_cmp(ib), oa.partial_cmp(ob), "cmp {oa:?} {ob:?}");
        let disjoint = oa.is_disjoint(ob);
        assert_eq!(ia.is_disjoint(ib), disjoint, "is_disjoint {oa:?} {ob:?}");

        let summed = IdView(ia.as_bits()).sum(&IdView(ib.as_bits()));
        if disjoint {
            let mut oracle_sum = oa.clone();
            oracle_sum.join(ob.clone()).expect("disjoint, just checked");
            let bits = summed.expect("disjoint pair sums");
            assert!(Party::from_bits(bits) == from_oracle_party(&oracle_sum));
        } else {
            assert!(
                summed.is_none(),
                "overlapping ids must not sum: {oa:?} {ob:?}"
            );
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
            &decoded == v,
            "event decode∘encode is not identity for {ov:?}"
        );
        assert_eq!(to_oracle_version(&decoded), *ov);
    });
}

/// `partial_cmp` and `|` (merge / LUB) over every *ordered pair* of events agree with the
/// oracle, structurally — reaching the concurrent (`None`) verdict and the join arm selection
/// on shapes the op pipeline never builds.
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
    });
}

// ───────────────────────── (id, event) op families ─────────────────────────

/// `tick` (= `fill` then, on no fill, `grow`) over every (non-empty id, event) pair matches
/// the oracle's `event`. When the pair takes the `grow` branch, the impl's inflation is
/// additionally pinned to the brute-force cost-minimal, right-favoring region
/// ([`best_inflation`]) — holding the packed `grow`'s DP to the global optimum directly, not
/// merely to the oracle that realizes the same DP — and the metamorphic minimality condition
/// (no feasible candidate sits strictly between `e` and `e'`) is checked on the impl's own
/// causal order.
fn check_tick(
    ids: &[oracle::Party],
    imp_ids: &[Party],
    evs: &[oracle::Version],
    imp_evs: &[Version],
) {
    (0..ids.len()).into_par_iter().for_each(|i| {
        let op = &ids[i];
        if op.is_empty() {
            return; // `tick` requires a non-anonymous id
        }
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

/// Run every differential op family over the ids enumerated at `id_depth` and the events at
/// `ev_depth` (the two corpora grow at different rates, so their bounds are decoupled — see
/// the module doc). Each corpus is lowered to its impl form once and the pair loops borrow it.
fn run_all_at(id_depth: usize, ev_depth: usize) {
    let ids = all_normal_ids(id_depth);
    let evs = all_normal_events(ev_depth);
    let imp_ids: Vec<Party> = ids.iter().map(from_oracle_party).collect();
    let imp_evs: Vec<Version> = evs.iter().map(from_oracle_version).collect();

    check_id_codec(&ids, &imp_ids);
    check_id_split(&ids, &imp_ids);
    check_id_pairs(&ids, &imp_ids);

    check_ev_codec(&evs, &imp_evs);
    check_ev_pairs(&evs, &imp_evs);

    check_tick(&ids, &imp_ids, &evs, &imp_evs);
}

/// Sanity-check that the enumeration deduplicates to canonical normal form: every
/// enumerated id and event is `is_normal`, and the corpus has no duplicates (the de-dup key
/// is injective over canonical trees, so equal trees would have collided).
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

/// Every operation, on every enumerated tree and ordered pair, agrees with the oracle at
/// the small depth bound — deterministic coverage of the close-up corners (root-tie `grow`,
/// empty-child spine, `close_node` adjacency, overlap/concurrent verdicts) that random
/// sampling under-hits. Runs in the normal gate.
#[test]
fn exhaustive_small() {
    run_all_at(ID_SMALL_DEPTH, EV_SMALL_DEPTH);
}

/// The same total cross-product at the deep depth bound. The id corpus jumps to 65536 trees,
/// so the `O(corpus²)` id pair-product (~4.3 billion pairs) dominates; with the per-tree
/// precompute and `rayon` parallelism it completes in ~4.5 minutes on a 16-core M4 Max
/// (measured: 270s). It is `#[ignore]`d to keep the normal gate fast. Run it explicitly with:
///
/// ```text
/// cargo nextest run -p before --release --all-features \
///     exhaustive_deep --run-ignored ignored-only
/// ```
///
/// (or `cargo test -p before --release --all-features -- --ignored exhaustive_deep`).
#[test]
#[ignore = "exhaustive deep enumeration: O(corpus^2) over 65536 ids; ~4.5 min even parallelized"]
fn exhaustive_deep() {
    run_all_at(ID_DEEP_DEPTH, EV_DEEP_DEPTH);
}

// ───────────────────────────── intrinsic symmetry laws ─────────────────────────────
//
// The op symmetries are intrinsic algebraic properties of the impl, so they are tested
// DIRECTLY on the impl — no oracle, and not folded into the differential checks above. Two
// payoffs: a symmetry bug the oracle happened to *share* is still caught here, and (being
// deterministic + total over the small-scope corpus) the guarantee is total, not sampled.

/// `is_disjoint` is symmetric: `a.is_disjoint(b) == b.is_disjoint(a)` for every ordered pair
/// of enumerated ids (including the reflexive `a == b` diagonal).
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

/// `sum` is commutative: `sum(a, b)` and `sum(b, a)` are byte-identical (and both `None`
/// exactly when the ids overlap), over every ordered pair of enumerated ids.
#[test]
fn id_sum_is_commutative() {
    let imp = impl_ids(ID_SMALL_DEPTH);
    par_for_pairs(imp.len(), |i, j| {
        let ab = IdView(imp[i].as_bits()).sum(&IdView(imp[j].as_bits()));
        let ba = IdView(imp[j].as_bits()).sum(&IdView(imp[i].as_bits()));
        assert!(ab == ba, "sum not commutative at ({i}, {j})");
    });
}

/// id `partial_cmp` is anti-symmetric: `cmp(a, b) == cmp(b, a).reverse()` (and the two
/// directions agree on the incomparable/`None` verdict), over every ordered pair.
#[test]
fn id_partial_cmp_is_antisymmetric() {
    let imp = impl_ids(ID_SMALL_DEPTH);
    par_for_pairs(imp.len(), |i, j| {
        assert_eq!(
            imp[i].partial_cmp(&imp[j]),
            imp[j].partial_cmp(&imp[i]).map(Ordering::reverse),
            "id partial_cmp not anti-symmetric at ({i}, {j})",
        );
    });
}

/// event `partial_cmp` is anti-symmetric, over every ordered pair of enumerated events.
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

/// event merge (`|`, the join / least upper bound) is commutative: `a | b == b | a`, over
/// every ordered pair of enumerated events.
#[test]
fn event_merge_is_commutative() {
    let imp = impl_events(EV_SMALL_DEPTH);
    par_for_pairs(imp.len(), |i, j| {
        let ab = imp[i].clone() | imp[j].clone();
        let ba = imp[j].clone() | imp[i].clone();
        assert!(ab == ba, "event merge not commutative at ({i}, {j})");
    });
}
