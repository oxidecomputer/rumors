
// ───────────── reviewer experiments (reverted before handoff) ─────────────

/// `sync_all` as it stood at the branch's base, verbatim except for the
/// receiver binding, as a differential oracle for the refactor.
fn review_old_sync_all<'a>(
    this: &mut Clock,
    iter: impl IntoIterator<Item = &'a mut Clock>,
) -> Result<Version, crate::Overlap> {
    use crate::Overlap;
    let others: Vec<&'a mut Clock> = iter.into_iter().collect();
    let groups = match crate::fold::balanced_try_fold(
        others.iter().map(|other| other.dangerously_alias()),
        |mut top, incoming| match top.join(incoming) {
            Ok(_) => Ok(top),
            Err(back) => Err((top, back)),
        },
    ) {
        Ok(groups) => groups,
        Err(_) => return Err(Overlap),
    };
    let mut whole = this.dangerously_alias();
    for group in groups {
        if whole.join(group).is_err() {
            return Err(Overlap);
        }
    }
    let share_count = others.len() + 1;
    let Clock { party, version } = whole;
    let mut shares = party.into_shares(share_count);
    *this = Clock::from_parts(shares.next().unwrap(), version.clone());
    for slot in others {
        *slot = Clock::from_parts(shares.next().unwrap(), version.clone());
    }
    assert!(shares.next().is_none());
    Ok(this.version().clone())
}

/// How a `(receiver, items)` family meets the merge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum ReviewPath {
    Accepted,
    FoldRejects,
    AbsorbRejectsFirstGroup,
    /// The absorb joins at least one group into the receiver before failing.
    AbsorbRejectsLater,
}

fn review_path(c: &Clock, items: &[Clock]) -> ReviewPath {
    let Ok(groups) = Clock::fold_disjoint(items.iter().map(Clock::dangerously_alias)) else {
        return ReviewPath::FoldRejects;
    };
    let mut whole = c.dangerously_alias();
    let mut groups = groups.into_iter().enumerate();
    while let Some((i, group)) = groups.next() {
        if whole.join(group).is_err() {
            return if i == 0 {
                ReviewPath::AbsorbRejectsFirstGroup
            } else {
                ReviewPath::AbsorbRejectsLater
            };
        }
    }
    ReviewPath::Accepted
}

fn review_arb_case(r: &(tree::Party, tree::Version), items: &[(tree::Party, tree::Version)]) -> (Clock, Vec<Clock>) {
    let clock = |(p, v): &(tree::Party, tree::Version)| Clock::from_parts(from_oracle_party(p), from_oracle_version(v));
    (clock(r), items.iter().map(clock).collect())
}

fn review_organic_strategy() -> impl Strategy<Value = (Clock, Vec<Clock>)> {
    (
        world_strategy(),
        0usize..64,
        crate::testing::generators::arb_fold_arity()
            .prop_flat_map(|arity| proptest::collection::vec(0usize..64, arity)),
    )
        .prop_map(|(ops, i, picks)| {
            let mut imp = vec![Clock::seed()];
            for op in &ops {
                step_impl(&mut imp, op);
            }
            let c = imp[i % imp.len()].dangerously_alias();
            let items = picks.iter().map(|t| imp[t % imp.len()].dangerously_alias()).collect();
            (c, items)
        })
}

/// Compare new and old `sync_all` on one family: same verdict, same values.
fn review_differential(c: &Clock, items: &[Clock]) -> Result<(), String> {
    let mut new_c = c.dangerously_alias();
    let mut new_items: Vec<Clock> = items.iter().map(Clock::dangerously_alias).collect();
    let new = new_c.sync_all(new_items.iter_mut()).cloned().map_err(|_| ());
    let mut old_c = c.dangerously_alias();
    let mut old_items: Vec<Clock> = items.iter().map(Clock::dangerously_alias).collect();
    let old = review_old_sync_all(&mut old_c, old_items.iter_mut()).map_err(|_| ());
    if new != old || new_c != old_c || new_items != old_items {
        return Err(format!("diverged: new {new:?} old {old:?}"));
    }
    Ok(())
}

/// Review witness: the fold returns groups that overlap one another, so
/// overlap between inputs can surface only in the absorb.
#[test]
fn review_fold_groups_can_overlap() {
    let mut root = Clock::seed();
    let mut a = root.fork();
    let b = a.fork();
    let a2 = a.dangerously_alias();
    let groups = Clock::fold_disjoint([a.dangerously_alias(), b.dangerously_alias(), a2.dangerously_alias()])
        .expect("the fold never compares the third input with the first group");
    assert_eq!(groups.len(), 2);
    assert!(!groups[0].party().is_disjoint(groups[1].party()), "groups overlap");
    let (mut r0, mut a0, mut b0, mut c0) = (root.dangerously_alias(), a.dangerously_alias(), b.dangerously_alias(), a2.dangerously_alias());
    assert!(r0.sync_all([&mut a0, &mut b0, &mut c0]).is_err());
    assert!(r0 == root && a0 == a && b0 == b && c0 == a2);
    eprintln!("REVIEW fold-groups-overlap: groups={} overlap=true sync_all=Err unchanged=true", groups.len());
}

/// Review witness: the late-overlap shape of `sync_all_is_join_all_then_forks`
/// absorbs the two shares into the receiver's alias before failing, for
/// receivers of several shapes.
#[test]
fn review_late_overlap_shape_reaches_partial_absorb() {
    let mut seed = Clock::seed();
    let mut half = seed.fork();
    half.tick();
    let mut quarter = half.fork();
    quarter.tick();
    quarter.tick();
    for c in [Clock::seed(), seed, half, quarter] {
        let mut q = c.dangerously_alias();
        let shares: Vec<Clock> = q.forks(2u64).collect();
        let echo = q.dangerously_alias();
        let inputs: Vec<Clock> = shares.iter().map(Clock::dangerously_alias).chain([echo]).collect();
        assert_eq!(review_path(&q, &inputs), ReviewPath::AbsorbRejectsLater);
        let groups = Clock::fold_disjoint(inputs).expect("shares and echo fold");
        assert_eq!(groups.len(), 2);
        let mut whole = q.dangerously_alias();
        let rest = whole.absorb_groups(groups).expect_err("echo overlaps the receiver");
        assert_eq!(rest.len(), 1);
        assert!(whole != q, "the first group reached the receiver's alias");
    }
    eprintln!("REVIEW late-overlap: AbsorbRejectsLater for 4 receivers, partial merge observed");
}

/// Review differential: old and new `sync_all` agree on arbitrary families.
#[test]
fn review_old_new_agree_and_reach() {
    use proptest::strategy::ValueTree;
    use proptest::test_runner::TestRunner;
    use std::collections::BTreeMap;
    const N: usize = 6000;
    let law = |name: &str| {
        crate::testing::laws::CLOCK_AND_LIST
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, law)| *law)
            .expect("law registered")
    };
    let generic = law("sync_all_agrees_with_join_all");
    let shapes = law("sync_all_is_join_all_then_forks");
    let report = |label: &str, cases: Vec<(Clock, Vec<Clock>)>| {
        let mut paths: BTreeMap<ReviewPath, usize> = BTreeMap::new();
        let (mut diverged, mut generic_fail, mut shapes_fail) = (0usize, 0usize, 0usize);
        let mut generic_fail_by_path: BTreeMap<ReviewPath, usize> = BTreeMap::new();
        for (c, items) in &cases {
            let path = review_path(c, items);
            *paths.entry(path).or_default() += 1;
            if let Err(e) = review_differential(c, items) {
                if diverged == 0 {
                    eprintln!("REVIEW {label} first divergence: {e}");
                }
                diverged += 1;
            }
            if !generic(c, items) {
                generic_fail += 1;
                *generic_fail_by_path.entry(path).or_default() += 1;
            }
            if !shapes(c, items) {
                shapes_fail += 1;
            }
        }
        eprintln!(
            "REVIEW {label}: n={} paths={paths:?} diverged={diverged} generic_law_fail={generic_fail} {generic_fail_by_path:?} shapes_law_fail={shapes_fail}",
            cases.len()
        );
    };
    let mut runner = TestRunner::deterministic();
    let arb = crate::testing::generators::arb_clock_family();
    let arb_cases: Vec<(Clock, Vec<Clock>)> = (0..N)
        .map(|_| {
            let (r, items) = arb.new_tree(&mut runner).unwrap().current();
            review_arb_case(&r, &items)
        })
        .collect();
    report("arb_clock_family", arb_cases);
    let mut runner = TestRunner::deterministic();
    let organic = review_organic_strategy();
    let organic_cases: Vec<(Clock, Vec<Clock>)> =
        (0..N).map(|_| organic.new_tree(&mut runner).unwrap().current()).collect();
    report("organic", organic_cases);
}
