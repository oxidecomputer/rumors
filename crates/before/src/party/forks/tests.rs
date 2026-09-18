//! Checks for compact balanced splitting at every count width.

use num_bigint::BigUint;
use proptest::prelude::*;

use super::{Forks, Party, Split};
use crate::testing::bridge::from_oracle_party;
use crate::testing::generators::arb_oracle_party_nonempty;
use crate::Ticks;

/// Arbitrary counts whose representation is wider than a machine word.
fn arb_wide_count() -> impl Strategy<Value = Ticks> {
    let minimum_bytes = usize::BITS as usize / 8 + 1;
    prop::collection::vec(any::<u8>(), minimum_bytes..=64).prop_map(|mut bytes| {
        *bytes.last_mut().expect("a wide count has bytes") |= 0x80;
        Ticks(BigUint::from_bytes_le(&bytes))
    })
}

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

    /// The consuming traversal yields the recursive balanced partition in the
    /// same preorder, while permitting each intermediate party to be split
    /// only once.
    #[test]
    fn consuming_traversal_matches_recursive_splitting(
        party in arb_oracle_party_nonempty(),
        count in 1usize..65,
    ) {
        let party = from_oracle_party(&party);
        let expected = recursive_split(party.dangerously_alias(), count);
        let actual: Vec<Party> = party.into_shares(count).collect();
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

    /// Short prefixes of arbitrary wide-count plans return exact shares and
    /// leave every untaken region with the borrower.
    #[test]
    fn wide_count_prefixes_conserve_the_party(
        party in arb_oracle_party_nonempty(),
        count in arb_wide_count(),
        taken in 0usize..=4,
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

/// Counts through 256 report each remaining size exactly as shares are
/// consumed.
#[test]
fn small_size_hints_are_exact() {
    for count in 1usize..=256 {
        let mut split = Split::new(Party::seed().0, count.into());
        for remaining in (0..=count).rev() {
            assert_eq!(split.size_hint(), (remaining, Some(remaining)));
            if remaining > 0 {
                assert!(split.next().is_some());
            }
        }
        assert!(split.next().is_none());
    }
}

/// The first count above `usize` becomes exact after one share without
/// narrowing on construction.
#[test]
fn adjacent_wide_count_becomes_exact() {
    let count = Ticks::from(usize::MAX) + Ticks::from(1u8);
    let mut split = Split::new(Party::seed().0, count);

    assert_eq!(split.size_hint(), (usize::MAX, None));
    assert!(split.next().is_some());
    assert_eq!(split.size_hint(), (usize::MAX, Some(usize::MAX)));
}

/// The first count classified as distant still reports a sound saturated hint
/// when its base-path depth equals the machine-word width.
#[test]
fn first_distant_count_handles_word_width_depth() {
    let count = Ticks(BigUint::from(usize::MAX) * 2u8);
    let mut keeper = Party::seed();
    let forks = Forks::new(&mut keeper, count);

    assert_eq!(forks.size_hint(), (usize::MAX, None));
}

/// A count of `2^128` remains iterable and reports a saturated lower bound
/// without narrowing.
#[test]
fn two_to_128_count_stays_iterable() {
    let count = Ticks::from(u128::MAX) + Ticks::from(1u8);
    let mut keeper = Party::seed();
    let mut forks = Forks::new(&mut keeper, count);

    assert_eq!(forks.size_hint(), (usize::MAX, None));
    assert!(forks.next().is_some());
    assert_eq!(forks.size_hint(), (usize::MAX, None));
}

/// A distant count lowers its saturated hint before the final machine-word
/// suffix and reports exact exhaustion at the end.
#[test]
fn distant_size_hint_stays_sound_near_exhaustion() {
    let depth = u64::from(usize::BITS) + 2;
    let count = BigUint::from(1u8) << depth;
    let mut split = Split::new(Party::seed().0, Ticks(count.clone()));
    split.index = &count - BigUint::from(usize::MAX);

    assert_eq!(split.size_hint(), (usize::MAX, None));
    split.index += 1u8;
    assert_eq!(split.size_hint(), (0, None));
    split.index = count;
    assert_eq!(split.size_hint(), (0, Some(0)));
}
