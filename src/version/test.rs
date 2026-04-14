use super::*;
use proptest::prelude::*;

#[test]
fn new_default() {
    assert_eq!(Version::<u64>::new([]), Version::default());
}

/// Two empty versions compare equal.
#[test]
fn empty_equal() {
    let a = Version::<u64>::default();
    let b = Version::<u64>::default();
    assert_eq!(a.partial_cmp(&b), Some(Ordering::Equal));
}

/// A version with any event strictly dominates the empty version.
#[test]
fn empty_less_than_event() {
    let a = Version::<u64>::default();
    let mut b = Version::<u64>::default();
    b.event(1);
    assert_eq!(a.partial_cmp(&b), Some(Ordering::Less));
    assert_eq!(b.partial_cmp(&a), Some(Ordering::Greater));
}

/// Events on different parties produce concurrent, incomparable versions.
#[test]
fn concurrent_events_incomparable() {
    let mut a = Version::<u64>::default();
    let mut b = Version::<u64>::default();
    a.event(1);
    b.event(2);
    assert_eq!(a.partial_cmp(&b), None);
    assert_eq!(b.partial_cmp(&a), None);
}

/// Appending an event to a clone yields a strictly greater version.
#[test]
fn extension_is_greater() {
    let mut a = Version::<u64>::default();
    a.event(1);
    a.event(2);
    let mut b = a.clone();
    b.event(1);
    assert_eq!(a.partial_cmp(&b), Some(Ordering::Less));
    assert_eq!(b.partial_cmp(&a), Some(Ordering::Greater));
}

/// `|=` merges per-party counters by taking the max.
#[test]
fn bitor_assign_takes_max() {
    let mut a = Version::<u64>::default();
    a.event(1);
    a.event(1);
    a.event(2);
    let mut b = Version::<u64>::default();
    b.event(1);
    b.event(3);
    let expected = a.clone() | b.clone();
    a |= b;
    assert_eq!(a, expected);
}

fn arb_version() -> impl Strategy<Value = Version<u8>> {
    prop::collection::vec((any::<u8>(), 0u64..=4), 0..8).prop_map(|pairs| {
        let mut v = Version::<u8>::default();
        for (p, n) in pairs {
            for _ in 0..n {
                v.event(p);
            }
        }
        v
    })
}

proptest! {
    /// partial_cmp is reflexive: every version compares equal to itself.
    #[test]
    fn reflexive(a in arb_version()) {
        prop_assert_eq!(a.partial_cmp(&a), Some(Ordering::Equal));
    }

    /// BitOr is a join: the union dominates both operands under the partial order.
    #[test]
    fn bitor_is_upper_bound(a in arb_version(), b in arb_version()) {
        let j = a.clone() | b.clone();
        prop_assert!(matches!(a.partial_cmp(&j), Some(Ordering::Less | Ordering::Equal)));
        prop_assert!(matches!(b.partial_cmp(&j), Some(Ordering::Less | Ordering::Equal)));
    }

    /// Version::new folds BitOr over its inputs: the result dominates every input.
    #[test]
    fn new_is_upper_bound(vs in prop::collection::vec(arb_version(), 0..5)) {
        let j = Version::new(vs.iter().cloned());
        for v in &vs {
            prop_assert!(matches!(v.partial_cmp(&j), Some(Ordering::Less | Ordering::Equal)));
        }
    }

    /// `|=` produces the same value as `|`, regardless of operand history.
    #[test]
    fn bitor_assign_matches_bitor(a in arb_version(), b in arb_version()) {
        let mut assigned = a.clone();
        assigned |= b.clone();
        prop_assert_eq!(assigned, a | b);
    }

    /// `|=` is a join in place: after the merge the receiver dominates both
    /// its prior value and the argument.
    #[test]
    fn bitor_assign_is_upper_bound(a in arb_version(), b in arb_version()) {
        let prior = a.clone();
        let mut merged = a;
        merged |= b.clone();
        prop_assert!(matches!(prior.partial_cmp(&merged), Some(Ordering::Less | Ordering::Equal)));
        prop_assert!(matches!(b.partial_cmp(&merged), Some(Ordering::Less | Ordering::Equal)));
    }

    /// partial_cmp is antisymmetric: mutual `<=` implies structural equality.
    #[test]
    fn antisymmetric(a in arb_version(), b in arb_version()) {
        let le_ab = matches!(a.partial_cmp(&b), Some(Ordering::Less | Ordering::Equal));
        let le_ba = matches!(b.partial_cmp(&a), Some(Ordering::Less | Ordering::Equal));
        if le_ab && le_ba {
            prop_assert_eq!(a, b);
        }
    }

    /// partial_cmp agrees with PartialEq: Some(Equal) iff the versions are equal.
    #[test]
    fn cmp_agrees_with_eq(a in arb_version(), b in arb_version()) {
        prop_assert_eq!(a.partial_cmp(&b) == Some(Ordering::Equal), a == b);
    }

    /// PartialOrd on version vectors exactly reflects causal history: for any
    /// sequence of forks, joins, and events over a bounded set of parties, the
    /// version-vector ordering on each pair of live replicas matches subset
    /// ordering on their ground-truth event sets.
    #[test]
    fn partial_ord_matches_causality(trace in arb_trace()) {
        run_trace(trace)?;
    }
}

/// An operation in a simulated history: forking creates a new branch with a
/// fresh party so its event stream is sequential and distinguishable, events
/// record onto the branch's own party, and joins merge one branch's history
/// into another.
#[derive(Debug, Clone)]
enum Op {
    Fork(usize),
    Event(usize),
    Join { src: usize, dst: usize },
}

fn arb_trace() -> impl Strategy<Value = Vec<Op>> {
    let op = prop_oneof![
        any::<usize>().prop_map(Op::Fork),
        any::<usize>().prop_map(Op::Event),
        (any::<usize>(), any::<usize>()).prop_map(|(src, dst)| Op::Join { src, dst }),
    ];
    prop::collection::vec(op, 0..16)
}

fn run_trace(trace: Vec<Op>) -> Result<(), TestCaseError> {
    use std::collections::BTreeSet;
    // Each branch carries its party, the version vector under test, and a
    // ground-truth set of event identifiers witnessing its causal history.
    // Giving each fork a fresh party keeps every party's event stream
    // sequential, so the version vector can faithfully encode causality.
    let mut branches: Vec<(usize, Version<usize>, BTreeSet<usize>)> =
        vec![(0, Version::default(), BTreeSet::new())];
    let mut next_party: usize = 1;
    let mut next_event: usize = 0;
    for op in trace {
        match op {
            Op::Fork(i) => {
                let i = i % branches.len();
                let mut clone = branches[i].clone();
                clone.0 = next_party;
                next_party = next_party.checked_add(1).expect("party id overflow");
                branches.push(clone);
            }
            Op::Event(i) => {
                let i = i % branches.len();
                let party = branches[i].0;
                branches[i].1.event(party);
                branches[i].2.insert(next_event);
                next_event += 1;
            }
            Op::Join { src, dst } => {
                let src = src % branches.len();
                let dst = dst % branches.len();
                let src_v = branches[src].1.clone();
                let src_e = branches[src].2.clone();
                branches[dst].1 = branches[dst].1.clone() | src_v;
                branches[dst].2.extend(src_e);
            }
        }
    }
    for (_, va, ea) in &branches {
        for (_, vb, eb) in &branches {
            let expected = match (ea.is_subset(eb), eb.is_subset(ea)) {
                (true, true) => Some(Ordering::Equal),
                (true, false) => Some(Ordering::Less),
                (false, true) => Some(Ordering::Greater),
                (false, false) => None,
            };
            prop_assert_eq!(va.partial_cmp(vb), expected);
        }
    }
    Ok(())
}
