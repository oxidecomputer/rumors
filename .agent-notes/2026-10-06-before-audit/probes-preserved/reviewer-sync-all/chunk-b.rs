
/// Review: both private steps conserve owner multiplicity (#40's contract) on
/// success and on error, over arbitrary and organic families.
#[test]
fn review_steps_conserve_multiplicity() {
    use proptest::strategy::ValueTree;
    use proptest::test_runner::TestRunner;
    const N: usize = 6000;
    let parties = |clocks: &[&Clock]| -> Vec<tree::Party> {
        clocks.iter().map(|c| to_oracle_clock(c).0).collect()
    };
    let same = |lhs: &[tree::Party], rhs: &[tree::Party]| {
        tree::Party::same_multiplicity(&lhs.iter().collect::<Vec<_>>(), &rhs.iter().collect::<Vec<_>>())
    };
    let check = |label: &str, cases: Vec<(Clock, Vec<Clock>)>| {
        let (mut fold_err, mut absorb_err, mut ok) = (0usize, 0usize, 0usize);
        for (c, items) in &cases {
            let received_items = parties(&items.iter().collect::<Vec<_>>());
            match Clock::fold_disjoint(items.iter().map(Clock::dangerously_alias)) {
                Err(rest) => {
                    fold_err += 1;
                    assert!(same(&received_items, &parties(&rest.iter().collect::<Vec<_>>())), "fold error path");
                }
                Ok(groups) => {
                    assert!(same(&received_items, &parties(&groups.iter().collect::<Vec<_>>())), "fold success");
                    let mut received = parties(&[c]);
                    received.extend(parties(&groups.iter().collect::<Vec<_>>()));
                    let mut whole = c.dangerously_alias();
                    match whole.absorb_groups(groups) {
                        Err(rest) => {
                            absorb_err += 1;
                            let mut held = parties(&[&whole]);
                            held.extend(parties(&rest.iter().collect::<Vec<_>>()));
                            assert!(same(&received, &held), "absorb error path");
                        }
                        Ok(_) => {
                            ok += 1;
                            assert!(same(&received, &parties(&[&whole])), "absorb success");
                        }
                    }
                }
            }
        }
        eprintln!("REVIEW multiplicity {label}: n={} fold_err={fold_err} absorb_err={absorb_err} ok={ok} all conserved", cases.len());
    };
    let mut runner = TestRunner::deterministic();
    let arb = crate::testing::generators::arb_clock_family();
    check(
        "arb_clock_family",
        (0..N)
            .map(|_| {
                let (r, items) = arb.new_tree(&mut runner).unwrap().current();
                review_arb_case(&r, &items)
            })
            .collect(),
    );
    let mut runner = TestRunner::deterministic();
    let organic = review_organic_strategy();
    check("organic", (0..N).map(|_| organic.new_tree(&mut runner).unwrap().current()).collect());
}
