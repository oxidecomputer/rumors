//! Rescue census (scratch worktree only, never committed): how often each
//! `min_ticks` floor bound is tight, over the L3 history generator and the
//! committed trace generator.
//!
//! A definition error that overcounts by one on some class of versions fails
//! a floor bound only at an observation where that bound is tight. So the
//! tight-observation rate under each bound measures that bound's sensitivity.
//! The committed `min_ticks_floors_every_history` bounds the final clocks by
//! the whole history's total ticks; the L3 probe bounds every clock, after
//! every step, by the ticks in its own causal past.

use std::collections::BTreeMap;

use num_bigint::BigUint;
use proptest::strategy::{Strategy, ValueTree};
use proptest::test_runner::TestRunner;

use crate::testing::optrace::{world_strategy, Op};
use crate::Clock;
use crate::Count;

use super::l3_probe::{arb_hop, HOp};

/// Counts gathered over one generator's histories.
#[derive(Default, Debug)]
struct Tally {
    histories: u64,
    /// (step, clock) observations with a nonzero `min_ticks`.
    observations: u64,
    /// Of those, the causal past is strictly smaller than the history total.
    past_below_total: u64,
    /// Of those, `min_ticks` equals the causal past.
    tight_causal: u64,
    /// Of those, `min_ticks` equals the history total.
    tight_total: u64,
    /// Final-clock observations with a nonzero `min_ticks`.
    final_observations: u64,
    final_tight_total: u64,
    final_tight_causal: u64,
    /// Histories with at least one causal-tight nonzero observation.
    histories_any_tight_causal: u64,
    /// Histories with at least one total-tight nonzero final observation.
    histories_any_final_tight_total: u64,
    /// `min_ticks` above the causal past (a floor violation; expected zero).
    violations: u64,
    /// Histories containing a fused count of at least `2^64`.
    wide_count_histories: u64,
    /// Histories containing an absorb or a foreign tick.
    absorb_histories: u64,
    /// Largest live population reached.
    max_population: usize,
    /// Sum over histories of the final population, for the mean.
    final_population_sum: u64,
    /// Sum over histories of the operation count, for the mean.
    ops_sum: u64,
}

/// One live clock and the tick batches in its causal past.
struct Tracked {
    clock: Clock,
    past: BTreeMap<u64, BigUint>,
}

/// The world of one history: live clocks, the next batch id, and the
/// history's total ticks.
struct World {
    clocks: Vec<Tracked>,
    next: u64,
    total: BigUint,
}

impl World {
    fn new() -> Self {
        World {
            clocks: vec![Tracked {
                clock: Clock::seed(),
                past: BTreeMap::new(),
            }],
            next: 0,
            total: BigUint::ZERO,
        }
    }

    /// Records a fresh tick batch of weight `w` in clock `i`'s past.
    fn fresh(&mut self, i: usize, w: BigUint) {
        self.total += &w;
        self.clocks[i].past.insert(self.next, w);
        self.next += 1;
    }

    fn union_into(&mut self, into: usize, from: &BTreeMap<u64, BigUint>) {
        for (k, w) in from {
            self.clocks[into].past.insert(*k, w.clone());
        }
    }

    /// Observes every live clock; returns whether any observation was
    /// causal-tight on a nonzero version.
    fn observe(&self, tally: &mut Tally) -> bool {
        let mut any = false;
        for t in &self.clocks {
            let m = t.clock.version().min_ticks().0;
            let past: BigUint = t.past.values().sum();
            if m > past {
                tally.violations += 1;
            }
            if m == BigUint::ZERO {
                continue;
            }
            tally.observations += 1;
            tally.past_below_total += u64::from(past < self.total);
            tally.tight_causal += u64::from(m == past);
            tally.tight_total += u64::from(m == self.total);
            any |= m == past;
        }
        any
    }

    fn observe_final(&self, tally: &mut Tally) -> bool {
        let mut any_total = false;
        for t in &self.clocks {
            let m = t.clock.version().min_ticks().0;
            if m == BigUint::ZERO {
                continue;
            }
            let past: BigUint = t.past.values().sum();
            tally.final_observations += 1;
            tally.final_tight_total += u64::from(m == self.total);
            tally.final_tight_causal += u64::from(m == past);
            any_total |= m == self.total;
        }
        any_total
    }
}

/// Applies one L3 history step, mirroring `l3_probe::run_history`.
fn apply_l3(w: &mut World, op: &HOp) {
    let n = w.clocks.len();
    let ix = |k: u8| usize::from(k) % n;
    match *op {
        HOp::Tick(i) => {
            let i = ix(i);
            w.clocks[i].clock.tick();
            w.fresh(i, BigUint::from(1u8));
        }
        HOp::Ticks(i, s, l) => {
            let i = ix(i);
            let weight = (BigUint::from(1u8) << u32::from(s)) + l;
            w.clocks[i].clock.ticks(Count(weight.clone()));
            w.fresh(i, weight);
        }
        HOp::Fork(i) => {
            let i = ix(i);
            let child = w.clocks[i].clock.fork();
            let past = w.clocks[i].past.clone();
            w.clocks.push(Tracked { clock: child, past });
        }
        HOp::Send(i, j) => {
            let (i, j) = (ix(i), ix(j));
            let msg = w.clocks[i].clock.send().clone();
            w.fresh(i, BigUint::from(1u8));
            let mp = w.clocks[i].past.clone();
            w.clocks[j].clock.recv(&msg);
            w.union_into(j, &mp);
            w.fresh(j, BigUint::from(1u8));
        }
        HOp::Absorb(i, j) => {
            let (i, j) = (ix(i), ix(j));
            let msg = w.clocks[i].clock.version().clone();
            let mp = w.clocks[i].past.clone();
            w.clocks[j].clock.absorb(&msg);
            w.union_into(j, &mp);
        }
        HOp::Sync(i, j) => {
            let (i, j) = (ix(i), ix(j));
            if i != j {
                let (lo, hi) = (i.min(j), i.max(j));
                let (left, right) = w.clocks.split_at_mut(hi);
                left[lo]
                    .clock
                    .sync(&mut right[0].clock)
                    .expect("linear history stays disjoint");
                let (a, b) = (w.clocks[lo].past.clone(), w.clocks[hi].past.clone());
                w.union_into(lo, &b);
                w.union_into(hi, &a);
            }
        }
        HOp::Join(i, j) => {
            let (i, j) = (ix(i), ix(j));
            if i != j && n > 1 {
                let other = w.clocks.remove(j);
                let r = if j < i { i - 1 } else { i };
                w.clocks[r]
                    .clock
                    .join(other.clock)
                    .map_err(|_| ())
                    .expect("disjoint");
                w.union_into(r, &other.past);
            }
        }
        HOp::ForeignTick(i, j) => {
            let (i, j) = (ix(i), ix(j));
            let mut v = w.clocks[j].clock.version().clone();
            v.tick(w.clocks[i].clock.party());
            w.clocks[j].clock.absorb(&v);
            w.fresh(j, BigUint::from(1u8));
        }
    }
}

/// Applies one committed trace step, mirroring `optrace::apply` for
/// production.
fn apply_committed(w: &mut World, op: &Op) {
    let n = w.clocks.len();
    match *op {
        Op::Tick(i) => {
            let i = i.index(n);
            w.clocks[i].clock.tick();
            w.fresh(i, BigUint::from(1u8));
        }
        Op::Ticks(i, count) => {
            let i = i.index(n);
            w.clocks[i].clock.ticks(u64::from(count));
            w.fresh(i, BigUint::from(count));
        }
        Op::Fork(i) => {
            let i = i.index(n);
            let child = w.clocks[i].clock.fork();
            let past = w.clocks[i].past.clone();
            w.clocks.push(Tracked { clock: child, past });
        }
        Op::Send(i, j) => {
            let (i, j) = (i.index(n), j.index(n));
            let msg = w.clocks[i].clock.send().clone();
            w.fresh(i, BigUint::from(1u8));
            let mp = w.clocks[i].past.clone();
            w.clocks[j].clock.recv(&msg);
            w.union_into(j, &mp);
            w.fresh(j, BigUint::from(1u8));
        }
        Op::Sync(i, j) => {
            let (i, j) = (i.index(n), j.index(n));
            if i != j {
                let (lo, hi) = (i.min(j), i.max(j));
                let (left, right) = w.clocks.split_at_mut(hi);
                left[lo]
                    .clock
                    .sync(&mut right[0].clock)
                    .expect("one universe stays disjoint");
                let (a, b) = (w.clocks[lo].past.clone(), w.clocks[hi].past.clone());
                w.union_into(lo, &b);
                w.union_into(hi, &a);
            }
        }
        Op::Join(i, j) => {
            if n > 1 {
                let (i, j) = (i.index(n), j.index(n));
                if i != j {
                    let other = w.clocks.remove(j);
                    let r = if j < i { i - 1 } else { i };
                    w.clocks[r]
                        .clock
                        .join(other.clock)
                        .map_err(|_| ())
                        .expect("one universe stays disjoint");
                    w.union_into(r, &other.past);
                }
            }
        }
    }
}

fn report(name: &str, t: &Tally) {
    let pct = |a: u64, b: u64| if b == 0 { 0.0 } else { 100.0 * a as f64 / b as f64 };
    eprintln!(
        "CENSUS {name}: histories={} mean-ops={:.1} mean-final-population={:.2} max-population={} \
         wide-count-histories={} absorb-or-foreign-histories={} violations={}",
        t.histories,
        t.ops_sum as f64 / t.histories as f64,
        t.final_population_sum as f64 / t.histories as f64,
        t.max_population,
        t.wide_count_histories,
        t.absorb_histories,
        t.violations,
    );
    eprintln!(
        "CENSUS {name}: every-step nonzero observations={} past<total={:.1}% causal-tight={:.1}% total-tight={:.1}%",
        t.observations,
        pct(t.past_below_total, t.observations),
        pct(t.tight_causal, t.observations),
        pct(t.tight_total, t.observations),
    );
    eprintln!(
        "CENSUS {name}: final nonzero observations={} final causal-tight={:.1}% final total-tight={:.1}%",
        t.final_observations,
        pct(t.final_tight_causal, t.final_observations),
        pct(t.final_tight_total, t.final_observations),
    );
    eprintln!(
        "CENSUS {name}: histories with a causal-tight observation (any step)={:.1}% \
         histories with a total-tight final observation={:.1}%",
        pct(t.histories_any_tight_causal, t.histories),
        pct(t.histories_any_final_tight_total, t.histories),
    );
}

const HISTORIES: u64 = 2000;

/// Prints how often each floor bound is tight (census; always passes unless
/// a floor is violated).
#[test]
fn l3_rescue_census_floor_bounds() {
    let mut runner = TestRunner::deterministic();
    let l3 = proptest::collection::vec(arb_hop(), 0..60);
    let mut t = Tally::default();
    for _ in 0..HISTORIES {
        let ops = l3.new_tree(&mut runner).expect("strategy").current();
        let mut w = World::new();
        let mut any = false;
        for op in &ops {
            apply_l3(&mut w, op);
            any |= w.observe(&mut t);
            t.max_population = t.max_population.max(w.clocks.len());
        }
        t.histories += 1;
        t.ops_sum += ops.len() as u64;
        t.final_population_sum += w.clocks.len() as u64;
        t.histories_any_tight_causal += u64::from(any);
        t.histories_any_final_tight_total += u64::from(w.observe_final(&mut t));
        t.wide_count_histories += u64::from(
            ops.iter()
                .any(|op| matches!(op, HOp::Ticks(_, s, _) if *s >= 64)),
        );
        t.absorb_histories += u64::from(
            ops.iter()
                .any(|op| matches!(op, HOp::Absorb(..) | HOp::ForeignTick(..))),
        );
    }
    report("l3-histories", &t);
    assert_eq!(t.violations, 0, "a floor violation on the L3 generator");

    let mut runner = TestRunner::deterministic();
    let committed = world_strategy();
    let mut t = Tally::default();
    for _ in 0..HISTORIES {
        let ops = committed.new_tree(&mut runner).expect("strategy").current();
        let mut w = World::new();
        let mut any = false;
        for op in &ops {
            apply_committed(&mut w, op);
            any |= w.observe(&mut t);
            t.max_population = t.max_population.max(w.clocks.len());
        }
        t.histories += 1;
        t.ops_sum += ops.len() as u64;
        t.final_population_sum += w.clocks.len() as u64;
        t.histories_any_tight_causal += u64::from(any);
        t.histories_any_final_tight_total += u64::from(w.observe_final(&mut t));
    }
    report("committed-traces", &t);
    assert_eq!(t.violations, 0, "a floor violation on the committed generator");
}
