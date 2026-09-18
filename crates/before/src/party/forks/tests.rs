//! Checks for compact balanced splitting at every count width.

use proptest::prelude::*;

use super::{Forks, Party, Split};
use crate::testing::bridge::from_oracle_party;
use crate::testing::generators::arb_oracle_party_nonempty;
use crate::Ticks;

/// Split a party through the recursive ceil-left/floor-right definition.
///
/// This deliberately uses only the binary public operation. It is the direct
/// behavioral reference for the compact count plan and one-pass path builder.
fn recursive_split(mut party: Party, count: usize) -> Vec<Party> {
    if count == 1 {
        return vec![party];
    }
    let right = party.fork();
    let left_count = count.div_ceil(2);
    let mut shares = recursive_split(party, left_count);
    shares.extend(recursive_split(right, count / 2));
    shares
}

proptest! {
    /// The compact plan preserves the exact shares and preorder of recursive
    /// balanced splitting for arbitrary party shapes and arities.
    #[test]
    fn compact_plan_matches_recursive_splitting(
        party in arb_oracle_party_nonempty(),
        count in 1usize..65,
    ) {
        let party = from_oracle_party(&party);
        let expected = recursive_split(party.dangerously_alias(), count);
        let actual: Vec<Party> = Split::new(party.0, count.into()).collect();
        prop_assert!(actual == expected);
    }

    /// Dropping after any prefix leaves exactly the untaken region in the
    /// borrower: rejoining the returned prefix reconstructs the input party.
    #[test]
    fn every_partial_drop_conserves_the_party(
        party in arb_oracle_party_nonempty(),
        (count, taken) in (0usize..65).prop_flat_map(|count| (Just(count), 0..=count)),
    ) {
        let original = from_oracle_party(&party);
        let mut keeper = original.dangerously_alias();
        let returned: Vec<Party> = keeper.forks(count).take(taken).collect();
        prop_assert!(keeper.join_all(returned).is_ok());
        prop_assert!(keeper == original);
    }
}

/// Every seed split has the minimum possible maximum depth.
#[test]
fn every_small_arity_is_balanced() {
    for count in 1usize..=256 {
        let shares: Vec<Party> = Split::new(Party::seed().0, count.into()).collect();
        let maximum_depth = shares
            .iter()
            .map(|party| (party.as_bits().len() - 2) / 2)
            .max()
            .expect("a nonzero split has a share");
        assert_eq!(
            maximum_depth,
            usize::BITS as u64 - (count - 1).leading_zeros() as u64,
            "arity {count}"
        );
    }
}

/// The internal remaining count stays exact above `u128::MAX`.
///
/// Constructing the iterator preserves the requested count, and yielding one
/// share subtracts exactly one.
#[test]
fn unbounded_remainder_stays_exact() {
    let count = Ticks::from(u128::MAX) + Ticks::from(1u8);
    let after_one = Ticks::from(u128::MAX);
    let mut keeper = Party::seed();
    let mut forks = Forks::new(&mut keeper, count.clone());

    assert_eq!(forks.split.remaining, count);
    assert!(forks.next().is_some());
    assert_eq!(forks.split.remaining, after_one);
}
