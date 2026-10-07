//! Checks for compact balanced forking at every count width.

use num_bigint::BigUint;
use proptest::prelude::*;

use super::{Party, PartyForks, Plan, Remaining};
use crate::party::io::PartySnapshot;
use crate::testing::bridge::from_oracle_party;
use crate::testing::generators::arb_oracle_party_nonempty;
use crate::Count;

/// Arbitrary counts whose representation is wider than a machine word.
fn arb_wide_count() -> impl Strategy<Value = Count> {
    let minimum_bytes = usize::BITS as usize / 8 + 1;
    prop::collection::vec(any::<u8>(), minimum_bytes..=64).prop_map(|mut bytes| {
        *bytes.last_mut().expect("a wide count has bytes") |= 0x80;
        Count(BigUint::from_bytes_le(&bytes))
    })
}

/// Plan a party through the recursive ceil-left/floor-right definition.
///
/// This deliberately uses only the binary public operation. It is the direct
/// behavioral reference for the compact count plan and one-pass path builder.
fn recursive_forks(mut party: Party, count: usize) -> Vec<Party> {
    if count == 1 {
        return vec![party];
    }
    let right = party.fork();
    let left_count = count.div_ceil(2);
    let mut shares = recursive_forks(party, left_count);
    shares.extend(recursive_forks(right, count / 2));
    shares
}

proptest! {
    /// The compact plan preserves the exact shares and preorder of recursive
    /// balanced forking for arbitrary party shapes and arities.
    #[test]
    fn compact_plan_matches_recursive_forking(
        party in arb_oracle_party_nonempty(),
        count in 1usize..65,
    ) {
        let party = from_oracle_party(&party);
        let expected = recursive_forks(party.dangerously_alias(), count);
        let actual: Vec<Party> = Plan::new(PartySnapshot::new(&party), count.into()).collect();
        prop_assert!(actual == expected);
    }

    /// The consuming traversal yields the recursive balanced partition in the
    /// same preorder, while permitting each intermediate party to be forked
    /// only once.
    #[test]
    fn consuming_traversal_matches_recursive_forking(
        party in arb_oracle_party_nonempty(),
        count in 1usize..65,
    ) {
        let party = from_oracle_party(&party);
        let expected = recursive_forks(party.dangerously_alias(), count);
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

/// Every seed fork has the minimum possible maximum depth.
#[test]
fn every_small_arity_is_balanced() {
    for count in 1usize..=256 {
        let party = Party::seed();
        let shares: Vec<Party> = Plan::new(PartySnapshot::new(&party), count.into()).collect();
        let maximum_depth = shares
            .iter()
            .map(|party| (party.stored_len() - 2) / 2)
            .max()
            .expect("a nonzero fork has a share");
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
        let party = Party::seed();
        let mut plan = Plan::new(PartySnapshot::new(&party), count.into());
        for remaining in (0..=count).rev() {
            assert_eq!(plan.size_hint(), (remaining, Some(remaining)));
            if remaining > 0 {
                assert!(plan.next().is_some());
            }
        }
        assert!(plan.next().is_none());
    }
}

/// The first count above `usize` becomes exact after one share without
/// narrowing on construction.
#[test]
fn adjacent_wide_count_becomes_exact() {
    let count = Count::from(usize::MAX) + Count::from(1u8);
    let party = Party::seed();
    let mut plan = Plan::new(PartySnapshot::new(&party), count);

    assert_eq!(plan.size_hint(), (usize::MAX, None));
    assert!(plan.next().is_some());
    assert_eq!(plan.size_hint(), (usize::MAX, Some(usize::MAX)));
}

/// The first count classified as distant still reports a sound saturated hint
/// when its base-path depth equals the machine-word width.
#[test]
fn first_distant_count_handles_word_width_depth() {
    let count = Count(BigUint::from(usize::MAX) * 2u8);
    let mut keeper = Party::seed();
    let forks = PartyForks::new(&mut keeper, count);

    assert_eq!(forks.size_hint(), (usize::MAX, None));
}

/// A count of `2^128` remains iterable and reports a saturated lower bound
/// without narrowing.
#[test]
fn two_to_128_count_stays_iterable() {
    let count = Count::from(u128::MAX) + Count::from(1u8);
    let mut keeper = Party::seed();
    let mut forks = PartyForks::new(&mut keeper, count);

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
    let party = Party::seed();
    let mut plan = Plan::new(PartySnapshot::new(&party), Count(count.clone()));
    plan.index = &count - BigUint::from(usize::MAX);

    assert_eq!(plan.size_hint(), (usize::MAX, None));
    plan.index += 1u8;
    assert_eq!(plan.size_hint(), (0, None));
    plan.index = count;
    assert_eq!(plan.size_hint(), (0, Some(0)));
}

/// Skips shares of a plan without producing them.
impl Plan {
    /// Skips `n` shares of a power-of-two plan that sits between base paths.
    ///
    /// A power-of-two count forks no base path again, so each base path names
    /// one share, and skipping `n` shares advances the base-path index by `n`.
    ///
    /// # Panics
    ///
    /// Panics unless the count is a power of two above twice `usize::MAX`,
    /// whose exhaustion and hint the plan derives from its base-path index
    /// alone, and the plan is between base paths.
    fn skip_shares(&mut self, n: &BigUint) {
        assert!(
            matches!(self.remaining, Remaining::Distant)
                && self.extra == BigUint::ZERO
                && !self.second,
            "skipping needs a power-of-two count above twice `usize::MAX`"
        );
        self.index += n;
    }
}

/// A wide plan's size hint is its exact remainder whenever the remainder fits
/// `usize`, and `(usize::MAX, None)` when more than `usize::MAX` remain; the
/// plan then yields exactly that remainder.
///
/// Each plan has a power-of-two count wider than `usize` on every target, the
/// widest beyond `u128`, and takes one real step from its start. Tail plans
/// then skip to 0, 1, or 5 remaining shares and drain them through real steps,
/// checking the hint before every step and exhaustion at the end. Boundary
/// plans skip to `usize::MAX + 1` or `usize::MAX` remaining shares and take one
/// real step across the representable boundary.
#[test]
fn wide_count_size_hint_is_exact_once_the_remainder_fits() {
    /// The hint for `remaining` shares under the one rule for every width.
    fn hint(remaining: &BigUint) -> (usize, Option<usize>) {
        usize::try_from(remaining).map_or((usize::MAX, None), |r| (r, Some(r)))
    }

    let word = u64::from(usize::BITS);
    let max = BigUint::from(usize::MAX);
    let party = Party::seed();
    let mut mismatches = Vec::new();
    for depth in [word + 1, word + 2, 128, 130] {
        let count = BigUint::from(1u8) << depth;
        // A plan that has produced its first share through a real step.
        let started = || {
            let mut plan = Plan::new(PartySnapshot::new(&party), Count(count.clone()));
            let before = plan.size_hint();
            assert!(plan.next().is_some(), "a wide plan yields a first share");
            (plan, before)
        };

        let (plan, before) = started();
        let actual = (before, plan.size_hint());
        let expected = (hint(&count), hint(&(&count - 1u8)));
        if actual != expected {
            mismatches.push(format!(
                "count 2^{depth}, first step: reported {actual:?}, expected {expected:?}"
            ));
        }

        for tail in [0usize, 1, 5] {
            let (mut plan, _) = started();
            plan.skip_shares(&(&count - 1u8 - tail));
            let mut hints = Vec::new();
            let mut yielded = 0;
            // Allow one extra step so an overlong drain is observed, not hidden.
            while yielded <= tail {
                hints.push(plan.size_hint());
                if plan.next().is_none() {
                    break;
                }
                yielded += 1;
            }
            let expected: Vec<_> = (0..=tail).rev().map(|r| (r, Some(r))).collect();
            if hints != expected || yielded != tail {
                mismatches.push(format!(
                    "count 2^{depth}, tail {tail}: reported {hints:?} over {yielded} shares, \
                     expected {expected:?} over {tail}"
                ));
            }
        }

        for from in [&max + 1u8, max.clone()] {
            let (mut plan, _) = started();
            plan.skip_shares(&(&count - 1u8 - &from));
            let before = plan.size_hint();
            assert!(plan.next().is_some(), "a nonempty plan yields a share");
            let actual = (before, plan.size_hint());
            let expected = (hint(&from), hint(&(&from - 1u8)));
            if actual != expected {
                mismatches.push(format!(
                    "count 2^{depth}, step from {from}: reported {actual:?}, expected {expected:?}"
                ));
            }
        }
    }
    assert!(
        mismatches.is_empty(),
        "size hints differ from the remaining count:\n{}",
        mismatches.join("\n")
    );
}
