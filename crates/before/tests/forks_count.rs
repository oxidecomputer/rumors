//! Exact-count checks for the borrowing balanced-fork iterators.

use before::{Clock, Party, Ticks};
use proptest::prelude::*;

proptest! {
    /// At every practical requested width, `Party::forks` reports and yields
    /// exactly that many pairwise-disjoint shares, which rejoin the keeper to
    /// recover the original party.
    #[test]
    fn party_forks_yield_the_requested_count(k in 0u16..=256) {
        let count = Ticks::from(k);
        let mut keeper = Party::seed();
        let mut forks = keeper.forks(count.clone());
        prop_assert_eq!(forks.size_hint(), (usize::from(k), Some(usize::from(k))));

        let mut shares = Vec::with_capacity(usize::from(k));
        while let Some(share) = forks.next() {
            shares.push(share);
            let left = k - u16::try_from(shares.len()).expect("the test width fits u16");
            prop_assert_eq!(
                forks.size_hint(),
                (usize::from(left), Some(usize::from(left)))
            );
        }
        drop(forks);

        prop_assert!(shares.iter().all(|share| keeper.is_disjoint(share)));
        for (i, share) in shares.iter().enumerate() {
            prop_assert!(shares[i + 1..]
                .iter()
                .all(|other| share.is_disjoint(other)));
        }
        prop_assert!(keeper.join_all(shares).is_ok());
        prop_assert!(keeper.is_seed());
    }
}

/// A count wider than `u128` constructs and iterates normally, uses the
/// standard unbounded `size_hint`, and returns every untaken region on drop.
#[test]
fn party_forks_accept_an_unbounded_count() {
    let count: Ticks = "340282366920938463463374607431768211456"
        .parse()
        .expect("2^128 is a natural-number count");
    let mut keeper = Party::seed();
    let share = {
        let mut forks = keeper.forks(count.clone());
        assert_eq!(forks.size_hint(), (usize::MAX, None));
        let share = forks.next().expect("a positive count yields a share");
        assert_eq!(forks.size_hint(), (usize::MAX, None));
        share
    };

    assert!(keeper.is_disjoint(&share));
    keeper.join(share).expect("disjoint shares rejoin");
    assert!(keeper.is_seed());
}

/// `Clock::forks` accepts the same unbounded count while each yielded child
/// carries the parent's version.
#[test]
fn clock_forks_accept_an_unbounded_count() {
    let count: Ticks = "340282366920938463463374607431768211456"
        .parse()
        .expect("2^128 is a natural-number count");
    let mut keeper = Clock::seed();
    let child = {
        let mut forks = keeper.forks(count.clone());
        assert_eq!(forks.size_hint(), (usize::MAX, None));
        forks.next().expect("a positive count yields a child")
    };

    assert!(keeper.party().is_disjoint(child.party()));
    assert_eq!(keeper.version(), child.version());
    keeper.join(child).expect("disjoint children rejoin");
    assert_eq!(keeper.party().to_string(), "1");
}
