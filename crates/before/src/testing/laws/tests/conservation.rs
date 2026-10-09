//! Focused tests for the conservation laws: regressions whose setup needs
//! overlapping aliases, and the multiplicity comparison the laws apply.

use proptest::prelude::*;

use crate::testing::bridge::from_oracle_party;
use crate::testing::generators::{arb_oracle_party_nonempty, arb_party_family};
use crate::testing::oracles::tree;
use crate::{Clock, Party};

use super::super::same_multiplicity;

/// Party and clock conservation laws hold when a fold contains several aliases
/// of each owned region.
#[test]
fn conservation_laws_cover_multiple_aliases() {
    let mut party = Party::seed();
    let shares: Vec<Party> = party.forks(3u64).collect();
    let [a, b, c] = shares.try_into().expect("three shares");
    let items = vec![
        a.dangerously_alias(),
        a.dangerously_alias(),
        b.dangerously_alias(),
        b.dangerously_alias(),
        c.dangerously_alias(),
    ];
    let party_conservation = super::super::PARTY_AND_LIST
        .iter()
        .find_map(|(name, law)| {
            (*name == "party_join_all_err_conserves_multiplicity").then_some(law)
        })
        .expect("party conservation law is registered");
    assert!(
        party_conservation(&party, &items),
        "the party conservation law failed with repeated aliases"
    );

    let mut seed = Clock::seed();
    let mut lines: Vec<Clock> = seed.forks(3u64).collect();
    for line in &mut lines {
        line.tick();
    }
    let [a, b, c] = lines.try_into().expect("three lines");
    let items = vec![
        a.dangerously_alias(),
        a.dangerously_alias(),
        b.dangerously_alias(),
        b.dangerously_alias(),
        c.dangerously_alias(),
    ];
    let clock_conservation = super::super::CLOCK_AND_LIST
        .iter()
        .find_map(|(name, law)| {
            (*name == "clock_join_all_err_conserves_multiplicity").then_some(law)
        })
        .expect("clock conservation law is registered");
    assert!(
        clock_conservation(&seed, &items),
        "the clock conservation law failed with repeated aliases"
    );
}

/// Return the verdicts of the laws' `same_multiplicity` and the tree oracle's
/// on the same pair of families, in that order.
fn both_verdicts(lhs: &[&tree::Party], rhs: &[&tree::Party]) -> (bool, bool) {
    let lift = |family: &[&tree::Party]| -> Vec<Party> {
        family.iter().map(|p| from_oracle_party(p)).collect()
    };
    let (lhs_parties, rhs_parties) = (lift(lhs), lift(rhs));
    let laws = same_multiplicity(
        &lhs_parties.iter().collect::<Vec<_>>(),
        &rhs_parties.iter().collect::<Vec<_>>(),
    );
    (laws, tree::Party::same_multiplicity(lhs, rhs))
}

proptest! {
    /// The laws' multiplicity comparison over production parties agrees with
    /// the tree oracle's, and both are right, on families where weaker
    /// comparisons go wrong.
    ///
    /// Replacing a party by its two fork halves keeps every point's
    /// multiplicity, and both comparisons must accept it. Each rejected family
    /// differs from the original only where a weaker comparison is blind: one
    /// extra copy of a party leaves the union unchanged; two extra copies also
    /// leave the parity of every count unchanged; and moving one copy between
    /// two disjoint regions of equal measure leaves the union and the total
    /// measure unchanged. A pool-drawn family, which repeats parties at fold
    /// arities and so gives some points a high multiplicity, must agree with
    /// its own reversal with one party split into halves, and must differ from
    /// that reversal plus one more copy of a party.
    #[test]
    fn same_multiplicity_agrees_with_the_tree_oracle(
        p in arb_oracle_party_nonempty(),
        q in arb_oracle_party_nonempty(),
        (receiver, items) in arb_party_family(),
    ) {
        let mut keep = p.clone();
        let give = keep.fork(); // p == keep ⊔ give, disjoint
        prop_assert_eq!(both_verdicts(&[&p, &q], &[&q, &give, &keep]), (true, true));
        prop_assert_eq!(both_verdicts(&[&p, &q], &[&p, &q, &p]), (false, false));
        prop_assert_eq!(both_verdicts(&[&p, &q], &[&p, &q, &p, &p]), (false, false));
        // `p` scaled into each half of the interval: disjoint, equal measure.
        let left = tree::Party::node(p.clone(), tree::Party::Leaf(false));
        let right = tree::Party::node(tree::Party::Leaf(false), p.clone());
        prop_assert_eq!(
            both_verdicts(&[&left, &left, &right, &q], &[&left, &right, &right, &q]),
            (false, false),
        );

        let family: Vec<&tree::Party> = core::iter::once(&receiver).chain(&items).collect();
        let mut last_kept = (*family.last().expect("the receiver is always present")).clone();
        let last_given = last_kept.fork();
        let restated: Vec<&tree::Party> = [&last_kept, &last_given]
            .into_iter()
            .chain(family.iter().rev().skip(1).copied())
            .collect();
        prop_assert_eq!(both_verdicts(&family, &restated), (true, true));
        let mut grown = restated.clone();
        grown.push(family[0]);
        prop_assert_eq!(both_verdicts(&family, &grown), (false, false));
    }
}
