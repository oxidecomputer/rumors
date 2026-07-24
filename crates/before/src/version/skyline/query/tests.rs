//! Differential pins for the query folds: the packed-form implementations
//! are the behavioral oracle over the adversarial families, arbitrary
//! trees, organic histories, and the exhaustive small scope; rank is
//! additionally pinned against the recursive tree oracle and the semantic
//! Riemann-sum oracle, which share no structure with the sweep.
//!
//! Every equality here is exact — `Rank` equality is structural on the
//! normalized form, projection agreement is byte identity of the emitted
//! canonical streams — so a fold that drifts by any amount anywhere has
//! no rounding to hide behind.

use proptest::prelude::*;

use crate::meter::{
    alt_spine, bigroot, cancelling_chain, cliff_comb, cliff_fan, dense, harmonic, hugeleaf,
    wide_tooth_comb, Packed,
};
use crate::testing::bridge::{from_oracle_version, to_oracle_party, to_oracle_version};
use crate::testing::exhaustive::{all_normal_events, all_normal_ids, EV_SMALL_DEPTH};
use crate::testing::generators::arb_oracle_version;
use crate::testing::{optrace, semantic_oracle};
use crate::version::skyline::encode;
use crate::{Clock, Party, Version};

use super::{distance, lag, min_ticks, project, rank};

/// Decode a meter-generated packed shape as a [`Version`].
fn version_of(p: &Packed) -> Version {
    Version::decode(&p.bytes[..]).expect("meter shapes are strict normal form")
}

/// Assert the single-operand folds against the packed-form oracle and,
/// for rank, the recursive tree fold.
fn assert_single(v: &Version) {
    let enc = encode(v);
    let want = v.rank();
    assert_eq!(rank(&enc), want, "rank kernel disagrees: {v}");
    assert_eq!(
        to_oracle_version(v).rank(),
        want,
        "tree-fold oracle disagrees with the packed rank: {v}"
    );
    assert_eq!(
        min_ticks(&enc),
        v.min_ticks(),
        "min_ticks kernel disagrees: {v}"
    );
}

/// Assert the projection kernel against the packed-form quotient for one
/// `(version, party)` operand pair: byte identity of the canonical
/// streams.
fn assert_projection(v: &Version, p: &Party) {
    let enc = encode(v);
    assert_eq!(
        project(&enc, p),
        encode(&(v / p)),
        "projection must transcode-commute: {v} / {p}"
    );
}

/// Assert the pair folds against the packed-form measures.
fn assert_pair(a: &Version, b: &Version) {
    let (ea, eb) = (encode(a), encode(b));
    assert_eq!(distance(&ea, &eb), a.distance(b), "distance: {a} vs {b}");
    assert_eq!(distance(&eb, &ea), b.distance(a), "distance: {b} vs {a}");
    assert_eq!(lag(&ea, &eb), a.lag(b), "lag: {a} vs {b}");
    assert_eq!(lag(&eb, &ea), b.lag(a), "lag: {b} vs {a}");
}

/// The adversarial family pool the deterministic sweeps run over.
fn family_pool() -> Vec<Version> {
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

/// The id operand pool for projection: the whole interval, one half, a
/// quarter, and the scattered fragments that keep every other comb tooth.
fn party_pool() -> Vec<Party> {
    let mut seed = Party::seed();
    let mut half = seed.fork();
    let quarter = half.fork();
    vec![
        seed,
        half,
        quarter,
        Party::decode(&crate::meter::scattered_id(1).bytes[..])
            .expect("scattered id is strict normal form"),
        Party::decode(&crate::meter::scattered_id(9).bytes[..])
            .expect("scattered id is strict normal form"),
    ]
}

/// Every adversarial family shape agrees with the packed-form rank,
/// min_ticks, and tree-fold rank; every family × id-pool cross agrees
/// with the packed-form projection; every ordered family pair agrees on
/// distance and lag.
///
/// The families are exactly the shapes whose costs the meter rows pin —
/// carry cliffs, wide teeth, cancelling chains, harmonic spines — so a
/// height-tracking or freeze bookkeeping error surfaces here before any
/// envelope moves.
#[test]
fn families_agree_with_the_packed_forms() {
    let pool = family_pool();
    let parties = party_pool();
    for v in &pool {
        assert_single(v);
        for p in &parties {
            assert_projection(v, p);
        }
    }
    for a in &pool {
        for b in &pool {
            assert_pair(a, b);
        }
    }
}

/// The exhaustive small scope: every normal-form event tree to the small
/// depth agrees on rank and min_ticks, and every tree × normal-form id
/// agrees on projection — every boundary genre by brute force rather
/// than sampling.
#[test]
fn exhaustive_small_scope_agrees() {
    let events: Vec<Version> = all_normal_events(EV_SMALL_DEPTH)
        .iter()
        .map(from_oracle_version)
        .collect();
    let ids: Vec<Party> = all_normal_ids(2)
        .iter()
        .map(crate::testing::bridge::from_oracle_party)
        .collect();
    for v in &events {
        assert_single(v);
        for p in &ids {
            assert_projection(v, p);
        }
    }
}

proptest! {
    /// Arbitrary normal-form trees agree with the packed forms on every
    /// query fold, with rank additionally realized by the semantic
    /// oracle's Riemann sum over the resolving grid — the geometric
    /// ground truth that shares no recursion, no delta, and no
    /// accumulator with either implementation.
    #[test]
    fn arbitrary_trees_agree(oa in arb_oracle_version(), ob in arb_oracle_version()) {
        let a = from_oracle_version(&oa);
        let b = from_oracle_version(&ob);
        assert_single(&a);
        assert_pair(&a, &b);
        let ev = semantic_oracle::lift_ev(oa);
        let g = semantic_oracle::ev_res(&ev);
        prop_assert_eq!(
            semantic_oracle::rank(&ev, g),
            rank(&encode(&a)),
            "the Riemann sum disagrees with the rank kernel: {}", a
        );
        prop_assert_eq!(
            semantic_oracle::min_ticks(&ev, g).to_u64_saturating(),
            min_ticks(&encode(&a)),
            "the semantic tick floor disagrees with the min_ticks kernel: {}", a
        );
    }

    /// Versions and parties produced by one organic
    /// fork/tick/send/sync/join history agree with the packed forms on
    /// every query fold, including projection onto the history's own
    /// parties — the operand pairing production code actually builds.
    #[test]
    fn organic_histories_agree(ops in optrace::world_strategy_up_to(40)) {
        let mut clocks = vec![Clock::seed()];
        for op in &ops {
            optrace::step_impl(&mut clocks, op);
        }
        for c in &clocks {
            assert_single(c.version());
            for other in &clocks {
                assert_projection(c.version(), other.party());
                assert_pair(c.version(), other.version());
            }
        }
    }

    /// The projection kernel agrees with the packed-form quotient over
    /// arbitrary tree × arbitrary id operands, where the id's absent
    /// children exercise the synthetic-empty arm at every depth.
    #[test]
    fn arbitrary_projections_agree(
        ov in arb_oracle_version(),
        oi in proptest::sample::select(all_normal_ids(3)),
    ) {
        let v = from_oracle_version(&ov);
        let p = crate::testing::bridge::from_oracle_party(&oi);
        assert_projection(&v, &p);
        // The projected region is what the semantic mask predicts: the
        // quotient through the oracle pipeline agrees byte for byte.
        let masked = from_oracle_version(&to_oracle_version(&v).project(&to_oracle_party(&p)));
        prop_assert_eq!(
            project(&encode(&v), &p),
            encode(&masked),
            "the oracle mask disagrees: {} / {}", v, p
        );
    }
}
