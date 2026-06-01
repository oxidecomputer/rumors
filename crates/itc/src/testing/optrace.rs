//! The seed-derived op-trace generator: a proptest strategy that produces a sequence of
//! fork/tick/send/sync/join steps, plus the appliers that run a trace against an oracle
//! population ([`run`]) or an impl population ([`step_impl`]). Values are always generated
//! via operations from a single seed, so every member is valid normal form and the
//! population is pairwise party-disjoint. Used by both the oracle property suite and the
//! impl property tests.

use std::cmp::Ordering;

use proptest::prelude::*;

use crate::oracle;
use crate::Clock;

/// One step of a seed-derived execution. Indices are reduced modulo the live
/// population, so any index is valid and every member descends from one seed via
/// fork/join/sync — keeping all parties pairwise disjoint.
#[derive(Clone, Debug)]
pub(crate) enum Op {
    /// Advance member `i`.
    Tick(usize),
    /// Split member `i`, appending the child.
    Fork(usize),
    /// `i` sends (ticks, emits its version); `j` receives it.
    Send(usize, usize),
    /// Reconcile `i` and `j` (join then re-split).
    Sync(usize, usize),
    /// Join `j` into `i`, removing `j`.
    Join(usize, usize),
}

fn op_strategy() -> impl Strategy<Value = Op> {
    prop_oneof![
        (0usize..8).prop_map(Op::Tick),
        (0usize..8).prop_map(Op::Fork),
        (0usize..8, 0usize..8).prop_map(|(a, b)| Op::Send(a, b)),
        (0usize..8, 0usize..8).prop_map(|(a, b)| Op::Sync(a, b)),
        (0usize..8, 0usize..8).prop_map(|(a, b)| Op::Join(a, b)),
    ]
}

/// A trace of up to 30 ops over a population that starts as a single seed clock.
pub(crate) fn world_strategy() -> impl Strategy<Value = Vec<Op>> {
    prop::collection::vec(op_strategy(), 0..30)
}

/// Apply a trace to a fresh oracle population.
pub(crate) fn run(ops: &[Op]) -> Vec<oracle::Clock> {
    let mut cs = vec![oracle::Clock::seed()];
    for op in ops {
        let n = cs.len();
        match *op {
            Op::Tick(i) => cs[i % n].tick(),
            Op::Fork(i) => {
                let child = cs[i % n].fork();
                cs.push(child);
            }
            Op::Send(i, j) => {
                let (i, j) = (i % n, j % n);
                let msg = cs[i].send();
                cs[j].receive(msg);
            }
            Op::Sync(i, j) => {
                let (i, j) = (i % n, j % n);
                if i != j {
                    let (lo, hi) = (i.min(j), i.max(j));
                    let (a, b) = cs.split_at_mut(hi);
                    a[lo]
                        .sync(&mut b[0])
                        .expect("seed-derived parties are disjoint");
                }
            }
            Op::Join(i, j) => {
                if n > 1 {
                    let (i, j) = (i % n, j % n);
                    if i != j {
                        let victim = cs.remove(j);
                        let i2 = if j < i { i - 1 } else { i };
                        cs[i2]
                            .join(victim)
                            .expect("seed-derived parties are disjoint");
                    }
                }
            }
        }
    }
    cs
}

/// Apply one op to an impl population, mirroring [`run`] for the oracle (same index
/// arithmetic, so traces line up). Used by tests that drive the impl alone.
pub(crate) fn step_impl(imp: &mut Vec<Clock>, op: &Op) {
    let n = imp.len();
    match *op {
        Op::Tick(i) => {
            imp[i % n].tick();
        }
        Op::Fork(i) => {
            let child = imp[i % n].fork();
            imp.push(child);
        }
        Op::Send(i, j) => {
            let (i, j) = (i % n, j % n);
            let msg = imp[i].send().clone();
            imp[j].receive(&msg);
        }
        Op::Sync(i, j) => {
            let (i, j) = (i % n, j % n);
            if i != j {
                let (lo, hi) = (i.min(j), i.max(j));
                let (a, b) = imp.split_at_mut(hi);
                a[lo]
                    .sync(&mut b[0])
                    .expect("seed-derived parties are disjoint");
            }
        }
        Op::Join(i, j) => {
            if n > 1 {
                let (i, j) = (i % n, j % n);
                if i != j {
                    let victim = imp.remove(j);
                    let i2 = if j < i { i - 1 } else { i };
                    imp[i2]
                        .join(victim)
                        .expect("seed-derived parties are disjoint");
                }
            }
        }
    }
}

/// Every live clock's current version.
pub(crate) fn versions(cs: &[oracle::Clock]) -> Vec<oracle::Version> {
    cs.iter().map(|c| c.version()).collect()
}

/// `a <= b` under the oracle causal order (treating concurrency as not-`<=`).
pub(crate) fn leq(a: &oracle::Version, b: &oracle::Version) -> bool {
    a.partial_cmp(b).is_some_and(|o| o != Ordering::Greater)
}
