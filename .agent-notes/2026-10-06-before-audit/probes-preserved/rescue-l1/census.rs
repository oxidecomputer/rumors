//! Rescue census (scratch worktree only, never committed): how often the
//! history driver's operations execute, the overlay generator's reach, and how
//! often `join_all_failure_conserves_multiplicity` takes each arm.

use std::collections::BTreeMap;

use before::{Clock, Version};
use proptest::prelude::*;
use proptest::sample::Index;
use proptest::strategy::ValueTree;
use proptest::test_runner::TestRunner;

use crate::gen::{arb_disjoint, arb_set, party};
use crate::history::{self, Op};
use crate::model::Set;

/// Histogram bucket: exact below 4, then the next power of two.
fn bucket(x: usize) -> usize {
    if x < 4 {
        x
    } else {
        x.next_power_of_two()
    }
}

/// The history driver's population cap (`history::CAP`, private there).
const CAP: usize = 24;

/// Distinct indices other than `i`, in pick order (`history::others`).
fn others(n: usize, i: usize, js: &[Index]) -> Vec<usize> {
    let mut out = Vec::new();
    for j in js {
        let j = j.index(n);
        if j != i && !out.contains(&j) {
            out.push(j);
        }
    }
    out
}

/// Per operation kind: generated and executed counts, applying exactly the
/// skip rules of `history::run`, which depend only on the population size.
#[test]
fn census_history_ops() {
    let mut runner = TestRunner::deterministic();
    let histories = 2000;
    let mut kinds: BTreeMap<&'static str, (usize, usize)> = BTreeMap::new();
    let mut join_all_arity: BTreeMap<usize, usize> = BTreeMap::new();
    let mut sync_all_arity: BTreeMap<usize, usize> = BTreeMap::new();
    let mut forks_class: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut split_n: BTreeMap<usize, usize> = BTreeMap::new();
    let mut peak: BTreeMap<usize, usize> = BTreeMap::new();
    let mut lengths: BTreeMap<usize, usize> = BTreeMap::new();
    let (mut steps, mut steps_ge8) = (0usize, 0usize);
    for _ in 0..histories {
        let ops = proptest::collection::vec(history::arb_op(), 1..40)
            .new_tree(&mut runner)
            .unwrap()
            .current();
        *lengths.entry(bucket(ops.len())).or_default() += 1;
        let mut n = 1usize;
        let mut top = 1usize;
        for op in &ops {
            let (name, executed) = match op {
                Op::Tick(_) => ("tick", true),
                Op::Fork(_) => {
                    let ok = n < CAP;
                    if ok {
                        n += 1;
                    }
                    ("fork", ok)
                }
                Op::Forks(_, k, t) => {
                    let (k, t) = (usize::from(*k), usize::from(*t.min(k)));
                    let ok = n + t <= CAP;
                    if ok {
                        n += t;
                        let class = if k == 0 {
                            "k=0"
                        } else if t == 0 {
                            "take 0 of k>0"
                        } else if t < k {
                            "partial 0<t<k"
                        } else {
                            "full t=k"
                        };
                        *forks_class.entry(class).or_default() += 1;
                    }
                    ("forks", ok)
                }
                Op::Split(_, sel) => {
                    let s = [2usize, 3, 5, 8][usize::from(*sel)];
                    let ok = n - 1 + s <= CAP;
                    if ok {
                        n += s - 1;
                        *split_n.entry(s).or_default() += 1;
                    }
                    ("split", ok)
                }
                Op::Join(i, j) => {
                    let ok = i.index(n) != j.index(n);
                    if ok {
                        n -= 1;
                    }
                    ("join", ok)
                }
                Op::JoinAll(i, js) => {
                    let os = others(n, i.index(n), js);
                    *join_all_arity.entry(os.len()).or_default() += 1;
                    n -= os.len();
                    ("join_all", true)
                }
                Op::Sync(i, j) => ("sync", i.index(n) != j.index(n)),
                Op::SyncAll(i, js) => {
                    let os = others(n, i.index(n), js);
                    *sync_all_arity.entry(os.len()).or_default() += 1;
                    ("sync_all", true)
                }
                Op::Move(_) => ("move clock bytes", true),
                Op::MoveParty(_) => ("move party bytes", true),
            };
            let e = kinds.entry(name).or_default();
            e.0 += 1;
            e.1 += usize::from(executed);
            steps += 1;
            steps_ge8 += usize::from(n >= 8);
            top = top.max(n);
        }
        *peak.entry(bucket(top)).or_default() += 1;
    }
    println!("CENSUS history: {histories} histories, {steps} steps; steps ending at population >= 8: {steps_ge8}");
    println!("CENSUS history lengths {lengths:?}");
    for (k, (g, e)) in &kinds {
        println!("CENSUS history op {k}: generated {g}, executed {e}");
    }
    println!("CENSUS history join_all others-count {join_all_arity:?}");
    println!("CENSUS history sync_all others-count {sync_all_arity:?}");
    println!("CENSUS history forks classes {forks_class:?}");
    println!("CENSUS history split sizes {split_n:?}");
    println!("CENSUS history peak population {peak:?}");
}

/// The overlay probe's inputs: party depth, version depth, plateau and cell
/// counts. The version strategy copies `overlay::arb_version` (private there).
#[test]
fn census_overlay_reach() {
    let version = proptest::collection::vec((arb_set(), 0u8..4), 0..5).prop_map(|steps| {
        let mut v = Version::new();
        for (s, n) in steps {
            party(&s).ticks(&mut v, u64::from(n));
        }
        v
    });
    let mut runner = TestRunner::deterministic();
    let draws = 2000;
    let mut h: [BTreeMap<usize, usize>; 5] = Default::default();
    let mut empty = 0usize;
    for _ in 0..draws {
        let a = arb_set().new_tree(&mut runner).unwrap().current();
        let v = version.new_tree(&mut runner).unwrap().current();
        let pd = a.regions().iter().map(|r| r.1 as usize).max().unwrap();
        let plateaus: Vec<u64> = v.shape().map(|p| p.depth).collect();
        let vd = *plateaus.iter().max().unwrap() as usize;
        let clock = Clock::from_parts(party(&a), v.clone());
        let cells: Vec<u64> = clock.shape().map(|(p, _)| p.depth).collect();
        let cd = *cells.iter().max().unwrap() as usize;
        empty += usize::from(v.is_empty());
        *h[0].entry(bucket(pd)).or_default() += 1;
        *h[1].entry(bucket(vd)).or_default() += 1;
        *h[2].entry(bucket(plateaus.len())).or_default() += 1;
        *h[3].entry(bucket(cells.len())).or_default() += 1;
        *h[4].entry(bucket(cd)).or_default() += 1;
    }
    println!("CENSUS overlay: {draws} draws; empty version {empty}");
    println!("CENSUS overlay party depth {:?}", h[0]);
    println!("CENSUS overlay version depth {:?}", h[1]);
    println!("CENSUS overlay plateau count {:?}", h[2]);
    println!("CENSUS overlay cell count {:?}", h[3]);
    println!("CENSUS overlay cell max depth {:?}", h[4]);
}

/// The `join_all` failure property's inputs: how often production accepts or
/// rejects, by the property's `mode`, and how many parties a rejection returns.
#[test]
fn census_join_all_failure_arms() {
    let strategy = (
        (2usize..=10).prop_flat_map(arb_disjoint),
        arb_set(),
        any::<Index>(),
        0u8..3,
        any::<Index>(),
        any::<u64>(),
    );
    let mut runner = TestRunner::deterministic();
    let draws = 2000;
    let mut arms: BTreeMap<(u8, &'static str), usize> = BTreeMap::new();
    let mut returned: BTreeMap<usize, usize> = BTreeMap::new();
    let mut family: BTreeMap<usize, usize> = BTreeMap::new();
    for _ in 0..draws {
        let (v, intruder, dup, mode, pos, order) =
            strategy.new_tree(&mut runner).unwrap().current();
        let mut items: Vec<Set> = v[1..].to_vec();
        crate::shuffle(&mut items, order);
        let extra = match mode {
            0 => v[dup.index(v.len())].clone(),
            1 => intruder.clone(),
            _ => intruder.union(&v[dup.index(v.len())]),
        };
        let at = pos.index(items.len() + 1);
        items.insert(at, extra);
        *family.entry(items.len()).or_default() += 1;
        let mut acc = party(&v[0]);
        match acc.join_all(items.iter().map(party)) {
            Ok(()) => *arms.entry((mode, "Ok")).or_default() += 1,
            Err(back) => {
                *arms.entry((mode, "Err")).or_default() += 1;
                *returned.entry(back.len()).or_default() += 1;
            }
        }
    }
    println!("CENSUS join_all failure: {draws} draws; arms by mode {arms:?}");
    println!("CENSUS join_all failure: inputs per call {family:?}");
    println!("CENSUS join_all failure: parties returned per rejection {returned:?}");
}
