//! Checks for compact balanced forking at every count width.

use num_bigint::BigUint;
use proptest::prelude::*;

use super::{Party, PartyForks, Plan};
use crate::party::io::PartySnapshot;
use crate::testing::bridge::from_oracle_party;
use crate::testing::generators::arb_oracle_party_nonempty;
use crate::testing::generators::regions::arb_deep_party;
use crate::testing::oracles::flat::{self, check_party};
use crate::{Clock, Count, Version};
use proptest::sample::Index;

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

/// Skips shares of a plan without producing them.
impl Plan {
    /// Skips `n` shares of a power-of-two plan that sits between base paths.
    ///
    /// A power-of-two count forks no base path again, so each base path names
    /// one share, and skipping `n` shares advances the base-path index and
    /// lowers the stored remainder by the same `n`.
    ///
    /// # Panics
    ///
    /// Panics unless the count is a power of two, the plan is between base
    /// paths, and at least `n` shares remain.
    fn skip_shares(&mut self, n: &BigUint) {
        assert!(
            self.extra == BigUint::ZERO && !self.second,
            "skipping needs a power-of-two plan between base paths"
        );
        self.index += n;
        self.remaining -= n;
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

/// Recomputes a plan's remainder from its position.
impl Plan {
    /// Counts the shares left from the base-path index, the fork flag, and the
    /// extra-fork count, without reading the stored remainder.
    ///
    /// Base path `q` yields two shares exactly when the `depth`-bit reversal of
    /// `q` is below `extra`, and one share otherwise. The shares produced are
    /// those of every base path before `index`, plus the left share of the
    /// current base path when `second` is set. A power-of-two count forks no
    /// base path again, so its produced count is `index` itself.
    ///
    /// # Panics
    ///
    /// Panics if the count is not a power of two and `index` exceeds
    /// `u64::MAX`: the base paths before `index` are enumerated one by one.
    fn remaining_from_position(&self) -> BigUint {
        let count = (BigUint::from(1u8) << self.depth) + &self.extra;
        let base_paths_produced = if self.extra == BigUint::ZERO {
            self.index.clone()
        } else {
            let index = u64::try_from(&self.index).expect("an enumerable base-path index");
            (0..index)
                .map(|path| {
                    let reversed = (0..self.depth.min(u64::from(u64::BITS)))
                        .filter(|&bit| (path >> bit) & 1 == 1)
                        .fold(BigUint::ZERO, |reversed, bit| {
                            reversed | (BigUint::from(1u8) << (self.depth - 1 - bit))
                        });
                    if reversed < self.extra {
                        2u8
                    } else {
                        1u8
                    }
                })
                .map(BigUint::from)
                .sum()
        };
        count - base_paths_produced - u8::from(self.second)
    }
}

/// A plan's stored remainder equals the remainder its position implies, at
/// every step, so exhaustion and the size hint follow the shares actually
/// left.
///
/// Every count through 256 drains completely. Counts wider than `usize`, with
/// and without extra forks, take real steps from their start; wide powers of
/// two also skip to a five-share tail and drain it.
#[test]
fn stored_remainder_matches_the_plan_position() {
    /// Steps `plan` up to `steps` times, comparing the remainder before each.
    fn check(plan: &mut Plan, steps: usize, label: &str) {
        for step in 0..=steps {
            assert_eq!(
                plan.remaining,
                plan.remaining_from_position(),
                "{label}, step {step}"
            );
            if plan.next().is_none() {
                return;
            }
        }
    }

    let party = Party::seed();
    for count in 1usize..=256 {
        let mut plan = Plan::new(PartySnapshot::new(&party), count.into());
        check(&mut plan, count, &format!("count {count}"));
        assert!(plan.next().is_none(), "count {count} drains");
    }

    let word = u64::from(usize::BITS);
    for depth in [word + 1, word + 2, 128, 130] {
        let power = BigUint::from(1u8) << depth;
        let counts = [
            power.clone(),
            &power + 5u8,
            &power + (&power >> 1u8) + 3u8,
            &power * 2u8 - 1u8,
        ];
        for count in counts {
            let mut plan = Plan::new(PartySnapshot::new(&party), Count(count.clone()));
            check(&mut plan, 8, &format!("count {count}"));
        }

        let mut plan = Plan::new(PartySnapshot::new(&party), Count(power.clone()));
        assert!(plan.next().is_some(), "a wide plan yields a first share");
        plan.skip_shares(&(&power - 6u8));
        check(&mut plan, 6, &format!("count 2^{depth} tail"));
        assert!(plan.next().is_none(), "count 2^{depth} drains");
    }
}

/// The deepest power-of-two plan whose tail the tail tests drain: past every
/// target's `usize` width and past `u128`.
const MAX_TAIL_DEPTH: u64 = 140;

/// The most shares before a plan's end that the tail tests drain.
const TAIL_SHARES: u8 = 40;

/// The rightmost leaf `depth` levels below the seed, built by repeated forks
/// that each keep the half given away.
fn rightmost_leaf(depth: u64) -> Party {
    let mut party = Party::seed();
    for _ in 0..depth {
        party = party.fork();
    }
    party
}

/// Every power-of-two plan of depth 0 to [`MAX_TAIL_DEPTH`] reports an exact
/// size hint and yields exactly its remaining shares over its last
/// [`TAIL_SHARES`] shares, and its last share is the rightmost leaf at its
/// depth.
///
/// Each plan takes one real step from its start, skips through
/// `skip_shares` to its last [`TAIL_SHARES`] shares or every share left, and
/// drains them through real steps, checking the hint before every step and
/// exhaustion at the end. A balanced split hands out the rightmost leaf last.
#[test]
fn every_power_of_two_plan_drains_its_tail_exactly() {
    let party = Party::seed();
    for depth in 0..=MAX_TAIL_DEPTH {
        let count = BigUint::from(1u8) << depth;
        let mut plan = Plan::new(PartySnapshot::new(&party), Count(count.clone()));
        let mut last = plan.next();
        assert!(last.is_some(), "depth {depth}: a plan yields a first share");
        let left = &count - 1u8;
        let tail = left.clone().min(BigUint::from(TAIL_SHARES));
        plan.skip_shares(&(&left - &tail));
        let tail = usize::try_from(&tail).expect("the tail is at most `TAIL_SHARES`");
        for remaining in (1..=tail).rev() {
            assert_eq!(
                plan.size_hint(),
                (remaining, Some(remaining)),
                "depth {depth}, {remaining} shares left"
            );
            last = plan.next();
            assert!(last.is_some(), "depth {depth}: {remaining} shares left");
        }
        assert_eq!(plan.size_hint(), (0, Some(0)), "depth {depth}, drained");
        assert!(plan.next().is_none(), "depth {depth}: the plan ends");
        assert_eq!(
            last,
            Some(rightmost_leaf(depth)),
            "depth {depth}: the last share"
        );
    }
}

/// A borrowing fork iterator of `2^d - 1` children, at every depth to
/// [`MAX_TAIL_DEPTH`], drains its last [`TAIL_SHARES`] children exactly and
/// leaves the borrower holding every region it has not handed out.
///
/// The size hint before each step must be the exact remainder, and the
/// iterator must yield exactly that many children.
///
/// Each iterator yields one child through a real step, skips its plan through
/// `skip_shares` to the last [`TAIL_SHARES`] children or every child left, and
/// drains them through real steps. The skipped children were never handed
/// out, so joining the children produced back into the borrowed party must
/// rebuild the seed.
#[test]
fn every_power_of_two_party_forks_drains_its_tail_and_conserves() {
    for depth in 1..=MAX_TAIL_DEPTH {
        let count = BigUint::from(1u8) << depth;
        let mut keeper = Party::seed();
        let shares = {
            let mut forks = PartyForks::new(&mut keeper, Count(&count - 1u8));
            let mut shares: Vec<Party> = forks.next().into_iter().collect();
            assert_eq!(shares.len(), 1, "depth {depth}: a first child");
            let left = forks.plan.remaining.clone();
            let tail = left.clone().min(BigUint::from(TAIL_SHARES));
            forks.plan.skip_shares(&(&left - &tail));
            let tail = usize::try_from(&tail).expect("the tail is at most `TAIL_SHARES`");
            for remaining in (1..=tail).rev() {
                assert_eq!(
                    forks.size_hint(),
                    (remaining, Some(remaining)),
                    "depth {depth}, {remaining} children left"
                );
                shares.push(forks.next().expect("a child remains"));
            }
            assert_eq!(forks.size_hint(), (0, Some(0)), "depth {depth}, drained");
            assert!(forks.next().is_none(), "depth {depth}: the iterator ends");
            shares
        };
        keeper
            .join_all(shares)
            .expect("the children produced are disjoint from what the borrower kept");
        assert!(
            keeper.is_seed(),
            "depth {depth}: the borrower and children rebuild the seed"
        );
    }
}

proptest! {
    /// A power-of-two plan of any depth to 200 reports `(usize::MAX, None)`
    /// exactly while more than `usize::MAX` shares remain, and the exact
    /// remainder otherwise, wherever `skip_shares` positions it.
    ///
    /// The remainders sit just above zero or just above `usize::MAX`, clipped
    /// to the plan's count, with offsets of 0 and 1 drawn often, so the
    /// remainders `usize::MAX` and `usize::MAX + 1` on either side of the
    /// representable boundary meet counts of every width.
    #[test]
    fn plan_size_hint_follows_one_rule_at_every_position(
        depth in 0u64..=200,
        offset in prop_oneof![Just(0u16), Just(1), 0u16..1024],
        near_word in any::<bool>(),
    ) {
        let count = BigUint::from(1u8) << depth;
        let base = if near_word { BigUint::from(usize::MAX) } else { BigUint::ZERO };
        let remaining = (base + offset).min(count.clone());
        let party = Party::seed();
        let mut plan = Plan::new(PartySnapshot::new(&party), Count(count.clone()));
        plan.skip_shares(&(&count - &remaining));
        let expected = usize::try_from(&remaining).map_or((usize::MAX, None), |r| (r, Some(r)));
        prop_assert_eq!(plan.size_hint(), expected);
    }
}

/// Counts around `usize::MAX` and far past it: across the word, just above
/// twice it, at arbitrary widths through `2^100`, and at powers of two and
/// their neighbors.
fn arb_count_past_usize() -> impl Strategy<Value = BigUint> {
    let word = BigUint::from(usize::MAX);
    let twice = &word * 2u8;
    prop_oneof![
        (0u8..80).prop_map(move |d| &word + d - 3u8),
        (0u8..8).prop_map(move |d| &twice + d - 2u8),
        (1u32..=100, any::<u128>()).prop_map(|(bits, x)| BigUint::from(x >> (128 - bits)) + 1u8),
        (1u32..=100, 0u8..3).prop_map(|(e, d)| (BigUint::from(1u8) << e) + d - 1u8),
    ]
}

proptest! {
    /// `forks(k)` on a deep party yields the flat model's balanced shares and
    /// leaves the party exactly its residual.
    ///
    /// Shares 1 to `k` of `k + 1` arrive in order, with an exact size hint
    /// before each; after any prefix the party holds exactly share 0 and the
    /// untaken shares, and a clock's `forks` pairs the same shares with its
    /// version.
    #[test]
    fn forks_match_the_flat_model_on_deep_parties(
        p in arb_deep_party(),
        k in prop_oneof![0usize..=20, 0usize..=300],
        taken in any::<Index>(),
    ) {
        let model = flat::Party::from_oracle(&p);
        let taken = taken.index(k + 1);
        let shares = model.shares(k + 1);
        let mut party = model.production();
        let mut clock = Clock::from_parts(model.production(), Version::new());
        clock.tick();
        let version = clock.version().clone();
        {
            let mut forks = party.forks(k);
            for (i, share) in shares.iter().enumerate().skip(1).take(taken) {
                let left = k + 1 - i;
                prop_assert_eq!(forks.size_hint(), (left, Some(left)), "the size hint before share {}", i);
                let got = forks.next().expect("the count is not exhausted");
                check_party(&got, share, "a share")?;
            }
        }
        let residual = core::iter::once(&shares[0])
            .chain(&shares[taken + 1..])
            .fold(flat::Party::empty(), |acc, share| acc.union(share));
        check_party(&party, &residual, "the residual")?;
        let children: Vec<Clock> = clock.forks(k).take(taken).collect();
        for (child, share) in children.iter().zip(&shares[1..]) {
            check_party(child.party(), share, "a child clock's share")?;
            prop_assert_eq!(child.version(), &version, "a child clock's version");
        }
        check_party(clock.party(), &residual, "the forking clock's residual")?;
    }

    /// Converting a deep party or clock into an array of `N` parties or
    /// clocks deals exactly the flat model's balanced shares of `N`, in
    /// order, for every `N` from 1 to 9 and for 13, 16, and 17.
    #[test]
    fn array_splits_match_the_flat_model_on_deep_parties(p in arb_deep_party()) {
        let model = flat::Party::from_oracle(&p);
        macro_rules! check_split {
            ($($n:literal),*) => {$(
                let shares = model.shares($n);
                let parties: [Party; $n] = model.production().into();
                for (got, share) in parties.iter().zip(&shares) {
                    check_party(got, share, concat!("a party split into ", $n))?;
                }
                let clocks: [Clock; $n] = Clock::from_parts(model.production(), Version::new()).into();
                for (got, share) in clocks.iter().zip(&shares) {
                    check_party(got.party(), share, concat!("a clock split into ", $n))?;
                }
            )*};
        }
        check_split!(1, 2, 3, 4, 5, 6, 7, 8, 9, 13, 16, 17);
    }

    /// At counts past `usize::MAX`, `forks(k)` on a deep party yields the
    /// flat model's shares and leaves the party exactly its residual.
    ///
    /// Shares 1, 2, … of `k + 1` arrive in order, the party keeps exactly
    /// what it had less the shares taken, which covers share 0, and a clock's
    /// `forks` pairs the same shares with its version.
    ///
    /// The model reaches each share by one descent of about `log₂ k` splits, so
    /// counts to `2^100` cost it only their bit length.
    #[test]
    fn wide_count_shares_match_the_flat_model(
        p in arb_deep_party(),
        k in arb_count_past_usize(),
        taken in 0usize..40,
    ) {
        let model = flat::Party::from_oracle(&p);
        let n = &k + 1u8;
        let share = |i: usize| model.share(&n, &BigUint::from(i));
        let taken = usize::try_from(&k).map_or(taken, |k| taken.min(k));
        let mut party = model.production();
        let got: Vec<Party> = party.forks(Count(k.clone())).take(taken).collect();
        let mut residual = model.clone();
        for (i, got) in got.iter().enumerate() {
            let want = share(i + 1);
            check_party(got, &want, "a share")?;
            residual = residual.difference(&want);
        }
        check_party(&party, &residual, "the residual")?;
        prop_assert!(residual.covers(&share(0)), "the residual covers share 0");
        let mut clock = Clock::from_parts(model.production(), Version::new());
        clock.tick();
        let version = clock.version().clone();
        let children: Vec<Clock> = clock.forks(Count(k.clone())).take(taken.min(6)).collect();
        for (i, child) in children.iter().enumerate() {
            check_party(child.party(), &share(i + 1), "a child clock's share")?;
            prop_assert_eq!(child.version(), &version, "a child clock's version");
        }
    }
}
