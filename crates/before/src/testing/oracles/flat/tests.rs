//! Calibration of the flat model against the recursive oracle.
//!
//! The differentials that judge production against the flat model trust
//! every answer it gives, so each property here compares the model with the
//! recursive oracle, which computes the same answers by the paper's tree
//! recursions and shares no code with the model. Neither side reads
//! production. The recursive oracle handles the deep strategies' depths, so
//! the calibration runs on arbitrary normal forms and on deep inputs alike.

use num_bigint::BigUint;
use proptest::prelude::*;

use super::{Party, Version};
use crate::testing::generators::leaf_list::{arb_deep_version_pair, TWO_VERSION_DIVISOR};
use crate::testing::generators::regions::arb_deep_party;
use crate::testing::generators::{arb_oracle_party_nonempty, arb_oracle_version};
use crate::testing::oracles::tree;
use crate::testing::property::divided_proptest;
use crate::testing::shape_rows::{oracle_plateaus, oracle_regions, Leaf};
use crate::Rank;

/// A rank's stored `(num, exp)`, the form the model's measures return.
fn raw(rank: &Rank) -> (BigUint, u64) {
    let (num, exp) = rank.raw_parts();
    (num.clone(), exp)
}

/// Asserts that the model's version answers equal the recursive oracle's on
/// one pair of normal-form trees and one party.
///
/// The canonical form is checked in both directions: the model keeps each
/// tree's own leaves, which are already in normal form, and it collapses a
/// spelling with every leaf split in two back to them. The measures are
/// checked against the oracle's ranks through the valuation identities the
/// rustdoc states: `distance(a, b) = rank(a ∨ b) − rank(a ∧ b)` and
/// `a.lag(b) = rank(a ∨ b) − rank(a)`.
fn assert_versions_agree(
    ta: &tree::Version,
    tb: &tree::Version,
    tp: &tree::Party,
) -> Result<(), TestCaseError> {
    for tree in [ta, tb] {
        let rows = oracle_plateaus(tree);
        let split: Vec<Leaf> = rows
            .iter()
            .flat_map(|leaf| {
                let child = Leaf {
                    depth: leaf.depth + 1,
                    height: leaf.height.clone(),
                };
                [child.clone(), child]
            })
            .collect();
        prop_assert_eq!(
            Version::from_oracle(tree).rows().to_vec(),
            rows.clone(),
            "canonical form"
        );
        prop_assert_eq!(
            Version::from_rows(&split).rows().to_vec(),
            rows,
            "canonical form of every leaf split in two"
        );
    }
    let (a, b) = (Version::from_oracle(ta), Version::from_oracle(tb));
    let p = Party::from_oracle(tp);
    let (join, meet) = (ta.clone() | tb.clone(), ta.clone() & tb.clone());
    prop_assert_eq!(a.join(&b), Version::from_oracle(&join), "join");
    prop_assert_eq!(a.meet(&b), Version::from_oracle(&meet), "meet");
    prop_assert_eq!(a.relation(&b), ta.partial_cmp(tb), "relation");
    prop_assert_eq!(a.rank(), raw(&ta.rank()), "rank");
    let distance = join
        .rank()
        .checked_sub(&meet.rank())
        .expect("the join's rank is at least the meet's");
    prop_assert_eq!(a.distance(&b), raw(&distance), "distance");
    let lag = join
        .rank()
        .checked_sub(&ta.rank())
        .expect("the join's rank is at least an operand's");
    prop_assert_eq!(a.lag(&b), raw(&lag), "lag");
    prop_assert_eq!(a.rank_cmp(&b), ta.rank().cmp(&tb.rank()), "rank_cmp");
    prop_assert_eq!(
        a.project(&p),
        Version::from_oracle(&ta.project(tp)),
        "project"
    );
    Ok(())
}

/// Asserts that the model's party answers equal the recursive oracle's on
/// one pair of normal-form trees.
///
/// The oracle has no intersection, so the model's is checked as
/// `p − (p − q)`.
fn assert_parties_agree(tp: &tree::Party, tq: &tree::Party) -> Result<(), TestCaseError> {
    let (p, q) = (Party::from_oracle(tp), Party::from_oracle(tq));
    prop_assert_eq!(p.regions(), oracle_regions(tp), "regions");
    prop_assert_eq!(
        p.union(&q),
        Party::from_oracle(&tp.clone().union(tq.clone())),
        "union"
    );
    prop_assert_eq!(
        p.difference(&q),
        Party::from_oracle(&tp.without(tq)),
        "difference"
    );
    prop_assert_eq!(
        p.intersection(&q),
        Party::from_oracle(&tp.without(&tp.without(tq))),
        "intersection"
    );
    prop_assert_eq!(p.is_disjoint(&q), tp.is_disjoint(tq), "is_disjoint");
    prop_assert_eq!(p.covers(&q), tp.covers(tq), "covers");
    let mut keep = tp.clone();
    let give = keep.fork();
    prop_assert_eq!(
        p.fork(),
        (Party::from_oracle(&keep), Party::from_oracle(&give)),
        "fork"
    );
    Ok(())
}

proptest! {
    /// On arbitrary normal-form versions and parties, the model's canonical
    /// form, join, meet, causal relation, rank, distance, lag, rank order,
    /// and projection equal the recursive oracle's.
    #[test]
    fn flat_versions_agree_with_the_tree_oracle_on_arbitrary_trees(
        ta in arb_oracle_version(),
        tb in arb_oracle_version(),
        tp in arb_oracle_party_nonempty(),
    ) {
        assert_versions_agree(&ta, &tb, &tp)?;
    }

    /// On arbitrary normal-form parties, the model's regions, union,
    /// difference, intersection, disjointness, cover, and fork equal the
    /// recursive oracle's.
    #[test]
    fn flat_parties_agree_with_the_tree_oracle_on_arbitrary_trees(
        tp in arb_oracle_party_nonempty(),
        tq in arb_oracle_party_nonempty(),
    ) {
        assert_parties_agree(&tp, &tq)?;
    }

    /// On deep, fragmented parties to depth 80, the model's regions, union,
    /// difference, intersection, disjointness, cover, and fork equal the
    /// recursive oracle's.
    #[test]
    fn flat_parties_agree_with_the_tree_oracle_on_deep_parties(
        tp in arb_deep_party(),
        tq in arb_deep_party(),
    ) {
        assert_parties_agree(&tp, &tq)?;
    }

    /// Every share of the model's balanced partition of a deep party, into
    /// one to 40 shares, equals the share the recursive oracle's splits
    /// produce.
    #[test]
    fn flat_shares_agree_with_recursive_oracle_splits(
        tp in arb_deep_party(),
        n in 1usize..=40,
    ) {
        let expected: Vec<Party> = tp.shares(n).iter().map(Party::from_oracle).collect();
        prop_assert_eq!(Party::from_oracle(&tp).shares(n), expected);
    }
}

divided_proptest! {
    #![divisor(TWO_VERSION_DIVISOR)]

    /// On deep version pairs and a deep party, the model's version answers
    /// equal the recursive oracle's.
    ///
    /// The pairs reach depth 300. The model's canonical form of a leaf list
    /// equals the oracle's normal form, and its join, meet, causal relation,
    /// rank, distance, lag, rank order, and projection equal the oracle's.
    #[test]
    fn flat_versions_agree_with_the_tree_oracle_on_deep_versions(
        (a, b) in arb_deep_version_pair(),
        tp in arb_deep_party(),
    ) {
        let (ta, tb) = (a.tree(), b.tree());
        prop_assert_eq!(
            Version::from_rows(a.leaves()).rows().to_vec(),
            oracle_plateaus(&ta),
            "canonical form"
        );
        assert_versions_agree(&ta, &tb, &tp)?;
    }
}
