
// ---- SCRATCH (fixer-fork-hint): uncommitted discovery family ----

/// The rightmost leaf at `depth` below the seed, built by repeated binary forks.
fn scratch_rightmost_leaf(depth: u64) -> Party {
    let mut party = Party::seed();
    for _ in 0..depth {
        party = party.fork();
    }
    party
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 2048, ..ProptestConfig::default() })]

    /// A power-of-two plan of any width, positioned `r` shares before its
    /// end, reports an exact hint at every step, yields exactly `r` shares,
    /// and ends on the rightmost leaf.
    #[test]
    fn scratch_wide_tail_drains_exactly(depth in 0u64..=140, r in 0u64..=40) {
        let count = BigUint::from(1u8) << depth;
        let r = BigUint::from(r).min(count.clone());
        let party = Party::seed();
        let mut plan = Plan::new(PartySnapshot::new(&party), Count(count.clone()));
        plan.position_at_remaining(&r);
        let r = usize::try_from(&r).unwrap();
        let mut last = None;
        for left in (1..=r).rev() {
            prop_assert_eq!(plan.size_hint(), (left, Some(left)));
            last = plan.next();
            prop_assert!(last.is_some());
        }
        prop_assert_eq!(plan.size_hint(), (0, Some(0)));
        prop_assert!(plan.next().is_none());
        if let Some(last) = last {
            prop_assert!(last == scratch_rightmost_leaf(depth));
        }
    }

    /// The borrowing iterator over `2^d - 1` children shares the plan's
    /// remainder: positioned near its end, it reports exact hints, yields
    /// exactly the remaining children, and still conserves the party.
    #[test]
    fn scratch_wide_party_forks_tail(depth in 1u64..=140, r in 0u64..=40) {
        let count = BigUint::from(1u8) << depth;
        // PartyForks plans k + 1 shares and skips the residual.
        let k = Count(&count - 1u8);
        let r = BigUint::from(r).min(&count - 1u8);
        let mut keeper = Party::seed();
        let shares: Vec<Party> = {
            let mut forks = PartyForks::new(&mut keeper, k);
            forks.plan.position_at_remaining(&r);
            let r = usize::try_from(&r).unwrap();
            let mut shares = Vec::new();
            for left in (1..=r).rev() {
                prop_assert_eq!(forks.size_hint(), (left, Some(left)));
                shares.push(forks.next().unwrap());
            }
            prop_assert_eq!(forks.size_hint(), (0, Some(0)));
            prop_assert!(forks.next().is_none());
            shares
        };
        prop_assert!(keeper.join_all(shares).is_ok());
        prop_assert!(keeper.is_seed());
    }

    /// Wide counts report `(usize::MAX, None)` exactly while the remainder
    /// exceeds `usize::MAX`, and the exact remainder otherwise, at any
    /// position of a power-of-two plan.
    #[test]
    fn scratch_hint_is_one_rule(depth in 0u64..=200, offset in any::<u128>(), near in any::<bool>()) {
        let count = BigUint::from(1u8) << depth;
        let base = if near { BigUint::from(usize::MAX) } else { BigUint::ZERO };
        let r = (base + BigUint::from(offset % 1024)).min(count.clone());
        let party = Party::seed();
        let mut plan = Plan::new(PartySnapshot::new(&party), Count(count));
        plan.position_at_remaining(&r);
        let expected = match usize::try_from(&r) {
            Ok(r) => (r, Some(r)),
            Err(_) => (usize::MAX, None),
        };
        prop_assert_eq!(plan.size_hint(), expected);
    }
}
