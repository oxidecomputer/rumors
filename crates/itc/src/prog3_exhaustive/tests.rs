//! PROG-3 exhaustive small-scope differential tests.
//!
//! Each `check_*` helper runs one op family over the *entire* enumerated corpus (every
//! tree, every ordered pair) and diffs the impl against the recursive oracle — the same
//! structural-agreement contract the PROG-1 sampled differentials use, but total rather
//! than random. The two test entry points wire the helpers to decoupled id/event depth
//! bounds (events grow far faster, so they are held a level shallower — see the parent
//! module doc): the gate-resident `prog3_exhaustive_small` at [`ID_SMALL_DEPTH`] /
//! [`EV_SMALL_DEPTH`], and the `#[ignore]`d `prog3_exhaustive_deep` at [`ID_DEEP_DEPTH`] /
//! [`EV_DEEP_DEPTH`].

use std::cmp::Ordering;

use super::{
    all_normal_events, all_normal_ids, EV_DEEP_DEPTH, EV_SMALL_DEPTH, ID_DEEP_DEPTH, ID_SMALL_DEPTH,
};
use crate::idbits::IdView;
use crate::oracle;
use crate::test_support::{
    all_inflations, best_inflation, from_oracle_party, from_oracle_version, to_oracle_party,
    to_oracle_version,
};
use crate::{Party, Version};

/// `a <= b` under the impl event causal order (concurrency is not-`<=`).
fn ev_le(a: &Version, b: &Version) -> bool {
    a.partial_cmp(b).is_some_and(|o| o != Ordering::Greater)
}

// ───────────────────────────── id op families ─────────────────────────────

/// Every id tree round-trips: the impl is itself normal form (`decode` accepts only
/// canonical bits), and lowering after `decode∘encode` recovers the same oracle tree. The
/// anonymous id is excluded (a standalone `Party` must own a region; `decode` rejects `0`).
fn check_id_codec(ids: &[oracle::Party]) {
    for oa in ids {
        if oa.is_empty() {
            continue;
        }
        let p = from_oracle_party(oa);
        let bytes = p.encode();
        let decoded = Party::decode(&bytes).expect("canonical id encoding decodes");
        assert!(decoded == p, "id decode∘encode is not identity for {oa:?}");
        assert_eq!(to_oracle_party(&decoded), *oa);
    }
}

/// `split` (the structural op behind `fork`) on every non-empty id matches the oracle's
/// `split` on both halves, structurally.
fn check_id_split(ids: &[oracle::Party]) {
    for oa in ids {
        if oa.is_empty() {
            continue;
        }
        let mut oracle_self = oa.clone();
        let oracle_give = oracle_self.fork(); // fork == split; mutates self to the kept half

        let p = from_oracle_party(oa);
        let (keep_bits, give_bits) = IdView(p.as_bits()).split();
        assert!(Party::from_bits(keep_bits) == from_oracle_party(&oracle_self));
        assert!(Party::from_bits(give_bits) == from_oracle_party(&oracle_give));
    }
}

/// `partial_cmp`, `is_disjoint`, and `sum` over every *ordered pair* of ids agree with the
/// oracle — reaching the overlap (`is_disjoint == false`), incomparable (`partial_cmp ==
/// None`), and overlap-sum (`None`) arms exhaustively. `is_disjoint` is checked symmetric on
/// the impl directly.
fn check_id_pairs(ids: &[oracle::Party]) {
    for oa in ids {
        let ia = from_oracle_party(oa);
        for ob in ids {
            let ib = from_oracle_party(ob);

            assert_eq!(ia.partial_cmp(&ib), oa.partial_cmp(ob), "cmp {oa:?} {ob:?}");
            let disjoint = oa.is_disjoint(ob);
            assert_eq!(ia.is_disjoint(&ib), disjoint, "is_disjoint {oa:?} {ob:?}");
            assert_eq!(
                ia.is_disjoint(&ib),
                ib.is_disjoint(&ia),
                "disjoint symmetric"
            );

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
        }
    }
}

// ───────────────────────────── event op families ─────────────────────────────

/// Every event tree round-trips through the widened codec and lowers back to the same
/// oracle value.
fn check_ev_codec(evs: &[oracle::Version]) {
    for ov in evs {
        let v = from_oracle_version(ov);
        let bytes = v.encode();
        let decoded = Version::decode(&bytes).expect("canonical event encoding decodes");
        assert!(
            decoded == v,
            "event decode∘encode is not identity for {ov:?}"
        );
        assert_eq!(to_oracle_version(&decoded), *ov);
    }
}

/// `partial_cmp` and `|` (merge / LUB) over every *ordered pair* of events agree with the
/// oracle, structurally — reaching the concurrent (`None`) verdict and the join arm
/// selection on shapes the op pipeline never builds. The verdict is checked anti-symmetric
/// on the impl directly.
fn check_ev_pairs(evs: &[oracle::Version]) {
    for oa in evs {
        let ia = from_oracle_version(oa);
        for ob in evs {
            let ib = from_oracle_version(ob);

            assert_eq!(ia.partial_cmp(&ib), oa.partial_cmp(ob), "cmp {oa:?} {ob:?}");
            assert_eq!(
                ib.partial_cmp(&ia),
                ia.partial_cmp(&ib).map(Ordering::reverse),
                "cmp anti-symmetric {oa:?} {ob:?}",
            );

            let merged = ia.clone() | ib.clone();
            let oracle_join = oa.clone() | ob.clone();
            assert!(
                merged == from_oracle_version(&oracle_join),
                "merge {oa:?} | {ob:?}",
            );
            assert_eq!(to_oracle_version(&merged), oracle_join);
        }
    }
}

// ───────────────────────── (id, event) op families ─────────────────────────

/// `tick` (= `fill` then, on no fill, `grow`) over every (non-empty id, event) pair matches
/// the oracle's `event`. When the pair takes the `grow` branch, the impl's inflation is
/// additionally pinned to the brute-force cost-minimal, right-favoring region
/// ([`best_inflation`]) — holding the packed `grow`'s DP to the global optimum directly, not
/// merely to the oracle that realizes the same DP — and the paper's metamorphic minimality
/// condition (no feasible candidate sits strictly between `e` and `e'`) is checked on the
/// impl's own causal order.
fn check_tick(ids: &[oracle::Party], evs: &[oracle::Version]) {
    for op in ids {
        if op.is_empty() {
            continue; // `tick` requires a non-anonymous id
        }
        let ip = from_oracle_party(op);
        for ov in evs {
            // Differential: impl `tick` == oracle `event`.
            let mut oracle_after = ov.clone();
            oracle_after.tick(op);

            let e = from_oracle_version(ov);
            let mut eprime = e.clone();
            eprime.tick(&ip);
            assert!(
                eprime == from_oracle_version(&oracle_after),
                "tick {op:?} on {ov:?}",
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
                let above_e = ev_le(&e, &cand_v);
                let strictly_below = cand_v.partial_cmp(&eprime) == Some(Ordering::Less);
                assert!(
                    !(above_e && strictly_below),
                    "an inflation candidate sits strictly between e and e' for {op:?} on {ov:?}",
                );
            }
        }
    }
}

// ─────────────────────────────── drivers ───────────────────────────────

/// Run every op family over the ids enumerated at `id_depth` and the events at `ev_depth`
/// (the two corpora grow at different rates, so their bounds are decoupled — see the module
/// doc).
fn run_all_at(id_depth: usize, ev_depth: usize) {
    let ids = all_normal_ids(id_depth);
    let evs = all_normal_events(ev_depth);

    check_id_codec(&ids);
    check_id_split(&ids);
    check_id_pairs(&ids);

    check_ev_codec(&evs);
    check_ev_pairs(&evs);

    check_tick(&ids, &evs);
}

/// PROG-3. Sanity-check that the enumeration deduplicates to canonical normal form: every
/// enumerated id and event is `is_normal`, and the corpus has no duplicates (the de-dup key
/// is injective over canonical trees, so equal trees would have collided).
#[test]
fn prog3_corpus_is_canonical() {
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

/// PROG-3 (fast). Every operation, on every enumerated tree and ordered pair, agrees with
/// the oracle at the small depth bound — deterministic coverage of the close-up corners
/// (root-tie `grow`, empty-child spine, `close_node` adjacency, overlap/concurrent verdicts)
/// that random sampling under-hits. Runs in the normal gate.
#[test]
fn prog3_exhaustive_small() {
    run_all_at(ID_SMALL_DEPTH, EV_SMALL_DEPTH);
}

/// PROG-3 (deep). The same total cross-product at [`DEEP_DEPTH`]. The corpus is far larger
/// and the op cross-product is `O(corpus²)`, so this takes minutes; it is `#[ignore]`d to
/// keep the normal gate fast. Run it explicitly with:
///
/// ```text
/// cargo nextest run -p itc --release --all-features \
///     prog3_exhaustive_deep --run-ignored ignored-only
/// ```
///
/// (or `cargo test -p itc --release --all-features -- --ignored prog3_exhaustive_deep`).
#[test]
#[ignore = "exhaustive deep enumeration: O(corpus^2), runs in minutes; see doc comment"]
fn prog3_exhaustive_deep() {
    run_all_at(ID_DEEP_DEPTH, EV_DEEP_DEPTH);
}
