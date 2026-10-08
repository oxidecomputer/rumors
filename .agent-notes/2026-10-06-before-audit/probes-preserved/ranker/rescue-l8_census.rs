//! Generator census (audit lane L8): histograms of what a generator produces.
//!
//! The census functions take any iterator of values, so the same metrics
//! describe the committed generators and any explore branch's generators:
//!
//! - [`version_census`] / [`version_value_census`]: oracle or production
//!   versions; depth, nodes, leaves, widest base, widest absolute height
//!   (root-to-leaf path sum), nonzero interior bases, root kind, encoded size.
//! - [`party_census`] / [`party_value_census`]: depth, leaves, owned leaves,
//!   root kind, encoded size.
//! - [`version_pair_census`] / [`party_pair_census`]: the relation mix.
//! - [`trace_census`]: operation traces as `optrace` replays them.
//!
//! To census another generator, copy this file into that branch's
//! `src/testing/`, register it with `#[cfg(test)] mod l8_census;`, and call a
//! census function from an `#[ignore]`d test, sampling a proptest strategy
//! with [`sample`] (fixed seed, the strategy's own draw distribution: what a
//! property sees before shrinking). Run with
//! `cargo nextest run -p before --all-features --run-ignored only -E 'test(/l8_census/)' --no-capture`.
//! Every block prints as `== <name>: <metric> (n = N)` followed by
//! `key: count (percent)` lines and a `summary` line, so two runs compare by
//! diffing their output.

use std::collections::BTreeMap;
use std::fmt::Debug;

use num_bigint::BigUint;
use proptest::strategy::{Strategy, ValueTree};
use proptest::test_runner::{Config, TestRunner};

use crate::testing::bridge::{
    from_oracle_party, from_oracle_version, to_oracle_party, to_oracle_version,
};
use crate::testing::generators::{arb_oracle_party, arb_oracle_party_nonempty, arb_oracle_version};
use crate::testing::optrace::{self, world_strategy, Op};
use crate::testing::oracles::function::{ev_depth, id_depth};
use crate::testing::oracles::tree;
use crate::testing::rng::strategy_rng;
use crate::{Party, Version};

/// Samples per census in the committed-generator runs.
const N: usize = 20_000;

/// Draw `n` values from `strategy` under a fixed seed, without shrinking.
pub(crate) fn sample<S: Strategy>(strategy: S, seed: u64, n: usize) -> Vec<S::Value> {
    let mut runner = TestRunner::new_with_rng(Config::default(), strategy_rng(seed));
    (0..n)
        .map(|_| strategy.new_tree(&mut runner).expect("strategy").current())
        .collect()
}

/// A histogram keyed by any ordered value.
struct Hist<K: Ord>(BTreeMap<K, usize>);

impl<K: Ord + Debug> Hist<K> {
    fn new() -> Self {
        Hist(BTreeMap::new())
    }

    fn bump(&mut self, k: K) {
        *self.0.entry(k).or_default() += 1;
    }

    fn show(&self, name: &str, metric: &str) {
        let total: usize = self.0.values().sum();
        eprintln!("== {name}: {metric} (n = {total})");
        for (k, v) in &self.0 {
            eprintln!(
                "   {k:?}: {v} ({:.2}%)",
                100.0 * *v as f64 / total.max(1) as f64
            );
        }
    }
}

/// A histogram over a numeric metric, printed with percentiles.
struct Numeric(Vec<u64>);

impl Numeric {
    fn new() -> Self {
        Numeric(Vec::new())
    }

    fn push(&mut self, v: u64) {
        self.0.push(v);
    }

    /// Print the exact histogram when it is small, a power-of-two bucketed one
    /// otherwise, then the percentile summary.
    fn show(&mut self, name: &str, metric: &str) {
        self.0.sort_unstable();
        let distinct = {
            let mut d = self.0.clone();
            d.dedup();
            d.len()
        };
        let mut hist = Hist::new();
        for &v in &self.0 {
            hist.bump(if distinct <= 24 {
                v
            } else {
                v.next_power_of_two()
            });
        }
        let label = if distinct <= 24 {
            metric.to_string()
        } else {
            format!("{metric} (power-of-two bucket)")
        };
        hist.show(name, &label);
        let at = |q: f64| {
            self.0
                .get(((self.0.len().saturating_sub(1)) as f64 * q) as usize)
                .copied()
                .unwrap_or(0)
        };
        eprintln!(
            "   summary {metric}: min {} p50 {} p90 {} p99 {} max {}",
            at(0.0),
            at(0.5),
            at(0.9),
            at(0.99),
            at(1.0)
        );
    }
}

/// Leaves and owned leaves of an oracle party.
fn party_leaves(p: &tree::Party) -> (u64, u64) {
    use tree::Party as P;
    let (mut leaves, mut owned) = (0, 0);
    let mut stack = vec![p];
    while let Some(n) = stack.pop() {
        match n {
            P::Leaf(b) => {
                leaves += 1;
                owned += u64::from(*b);
            }
            P::Node(l, r) => {
                stack.push(l);
                stack.push(r);
            }
        }
    }
    (leaves, owned)
}

/// Shape and magnitude statistics of an oracle version.
struct VersionStats {
    nodes: u64,
    leaves: u64,
    /// Widest stored base (relative to its parent), in bits.
    max_base_bits: u64,
    /// Widest absolute leaf height (root-to-leaf sum of bases), in bits.
    max_height_bits: u64,
    nonzero_interior: u64,
}

fn version_stats(v: &tree::Version) -> VersionStats {
    use tree::Version as V;
    let mut s = VersionStats {
        nodes: 0,
        leaves: 0,
        max_base_bits: 0,
        max_height_bits: 0,
        nonzero_interior: 0,
    };
    let mut stack = vec![(v, BigUint::ZERO)];
    while let Some((n, above)) = stack.pop() {
        s.nodes += 1;
        match n {
            V::Leaf(b) => {
                s.leaves += 1;
                s.max_base_bits = s.max_base_bits.max(b.bits());
                s.max_height_bits = s.max_height_bits.max((above + b).bits());
            }
            V::Node(b, l, r) => {
                s.max_base_bits = s.max_base_bits.max(b.bits());
                s.nonzero_interior += u64::from(*b != BigUint::ZERO);
                let here = above + b;
                stack.push((l, here.clone()));
                stack.push((r, here));
            }
        }
    }
    s
}

/// Census of oracle versions.
pub(crate) fn version_census(name: &str, versions: impl IntoIterator<Item = tree::Version>) {
    let (mut depth, mut nodes, mut leaves, mut base, mut height, mut nzi, mut size) = (
        Numeric::new(),
        Numeric::new(),
        Numeric::new(),
        Numeric::new(),
        Numeric::new(),
        Numeric::new(),
        Numeric::new(),
    );
    let mut kind = Hist::new();
    for v in versions {
        depth.push(u64::from(ev_depth(&v)));
        let s = version_stats(&v);
        nodes.push(s.nodes);
        leaves.push(s.leaves);
        base.push(s.max_base_bits);
        height.push(s.max_height_bits);
        nzi.push(s.nonzero_interior);
        size.push(from_oracle_version(&v).encoded_bits());
        kind.bump(match &v {
            tree::Version::Leaf(b) if *b == BigUint::ZERO => "zero leaf",
            tree::Version::Leaf(_) => "nonzero leaf",
            tree::Version::Node(..) => "node",
        });
    }
    depth.show(name, "depth");
    nodes.show(name, "nodes");
    leaves.show(name, "leaves");
    base.show(name, "widest base bits");
    height.show(name, "widest absolute height bits");
    nzi.show(name, "nonzero interior bases");
    kind.show(name, "root kind");
    size.show(name, "encoded bits");
}

/// Census of production versions, through the oracle bridge.
#[allow(dead_code)]
pub(crate) fn version_value_census(name: &str, versions: impl IntoIterator<Item = Version>) {
    version_census(name, versions.into_iter().map(|v| to_oracle_version(&v)));
}

/// Census of oracle parties (the anonymous party included).
pub(crate) fn party_census(name: &str, parties: impl IntoIterator<Item = tree::Party>) {
    let (mut depth, mut leaves, mut owned, mut size) = (
        Numeric::new(),
        Numeric::new(),
        Numeric::new(),
        Numeric::new(),
    );
    let mut kind = Hist::new();
    for p in parties {
        depth.push(u64::from(id_depth(&p)));
        let (l, o) = party_leaves(&p);
        leaves.push(l);
        owned.push(o);
        kind.bump(match &p {
            tree::Party::Leaf(true) => "seed",
            tree::Party::Leaf(false) => "anonymous",
            tree::Party::Node(..) => "node",
        });
        if !p.is_empty() {
            size.push(from_oracle_party(&p).encoded_bits());
        }
    }
    depth.show(name, "depth");
    leaves.show(name, "leaves");
    owned.show(name, "owned leaves");
    kind.show(name, "root kind");
    size.show(name, "encoded bits (nonempty)");
}

/// Census of production parties, through the oracle bridge.
#[allow(dead_code)]
pub(crate) fn party_value_census(name: &str, parties: impl IntoIterator<Item = Party>) {
    party_census(name, parties.into_iter().map(|p| to_oracle_party(&p)));
}

/// The relation mix of version pairs under the causal order.
pub(crate) fn version_pair_census(
    name: &str,
    pairs: impl IntoIterator<Item = (tree::Version, tree::Version)>,
) {
    let mut order = Hist::new();
    for (a, b) in pairs {
        order.bump(format!("{:?}", a.partial_cmp(&b)));
    }
    order.show(name, "partial_cmp");
}

/// The relation mix of party pairs as regions.
pub(crate) fn party_pair_census(
    name: &str,
    pairs: impl IntoIterator<Item = (tree::Party, tree::Party)>,
) {
    let mut rel = Hist::new();
    for (a, b) in pairs {
        rel.bump(if a == b {
            "equal"
        } else if a.is_disjoint(&b) {
            "disjoint"
        } else if a.covers(&b) {
            "a covers b"
        } else if b.covers(&a) {
            "b covers a"
        } else {
            "partial overlap"
        });
    }
    rel.show(name, "relation");
}

/// Census of operation traces, replayed as `optrace` replays them.
///
/// A join, sync, or send whose two operands resolve to the same clock, a join
/// in a population of one, and `ticks(0)` are reported as degenerate: the
/// driver skips the first two and the last records nothing.
pub(crate) fn trace_census(name: &str, traces: impl IntoIterator<Item = Vec<Op>>) {
    let (mut len, mut pop, mut pdepth, mut vdepth, mut vbits) = (
        Numeric::new(),
        Numeric::new(),
        Numeric::new(),
        Numeric::new(),
        Numeric::new(),
    );
    let (mut kinds, mut degenerate) = (Hist::new(), Hist::new());
    for ops in traces {
        len.push(ops.len() as u64);
        let mut n = 1usize;
        for op in &ops {
            let (kind, skip) = match *op {
                Op::Tick(_) => ("tick", false),
                Op::Ticks(_, c) => ("ticks", c == 0),
                Op::Fork(_) => {
                    n += 1;
                    ("fork", false)
                }
                Op::Send(a, b) => ("send", a.index(n) == b.index(n)),
                Op::Sync(a, b) => ("sync", a.index(n) == b.index(n)),
                Op::Join(a, b) => {
                    let skip = n == 1 || a.index(n) == b.index(n);
                    if !skip {
                        n -= 1;
                    }
                    ("join", skip)
                }
            };
            kinds.bump(kind);
            if skip {
                degenerate.bump(kind);
            }
        }
        let cs = optrace::run(&ops);
        pop.push(cs.len() as u64);
        pdepth.push(
            cs.iter()
                .map(|c| u64::from(id_depth(c.party())))
                .max()
                .unwrap_or(0),
        );
        vdepth.push(
            cs.iter()
                .map(|c| u64::from(ev_depth(&c.version())))
                .max()
                .unwrap_or(0),
        );
        vbits.push(
            cs.iter()
                .map(|c| version_stats(&c.version()).max_height_bits)
                .max()
                .unwrap_or(0),
        );
    }
    len.show(name, "trace length");
    pop.show(name, "final population");
    kinds.show(name, "op kinds");
    degenerate.show(name, "degenerate ops");
    pdepth.show(name, "max party depth");
    vdepth.show(name, "max version depth");
    vbits.show(name, "widest absolute height bits");
}

// ───────────────────── the committed generators' baseline ─────────────────────

/// Census of the committed arbitrary party strategies.
#[test]
#[ignore = "exploratory census"]
fn l8_census_parties() {
    party_census("arb_oracle_party", sample(arb_oracle_party(), 11, N));
    party_census(
        "arb_oracle_party_nonempty",
        sample(arb_oracle_party_nonempty(), 12, N),
    );
}

/// Census of the committed arbitrary version strategy.
#[test]
#[ignore = "exploratory census"]
fn l8_census_versions() {
    version_census("arb_oracle_version", sample(arb_oracle_version(), 21, N));
}

/// Relation mix of independently drawn committed operands.
#[test]
#[ignore = "exploratory census"]
fn l8_census_pairs() {
    let vs = sample(arb_oracle_version(), 31, 2 * N);
    version_pair_census(
        "arbitrary version pair",
        vs.chunks_exact(2).map(|p| (p[0].clone(), p[1].clone())),
    );
    let ps = sample(arb_oracle_party_nonempty(), 32, 2 * N);
    party_pair_census(
        "arbitrary nonempty party pair",
        ps.chunks_exact(2).map(|p| (p[0].clone(), p[1].clone())),
    );
    let mut proj = Hist::new();
    for (v, p) in vs.iter().zip(ps.iter()) {
        let projected = v.clone() / p;
        proj.bump(if projected == *v {
            "identity"
        } else if projected == tree::Version::new() {
            "zero"
        } else {
            "other"
        });
    }
    proj.show("arbitrary version / party", "projection");
}

/// Census of the committed op-trace strategy, and of the organic drivers'
/// operand picks (`i % len`, `j % len` with `i, j in 0..64`).
#[test]
#[ignore = "exploratory census"]
fn l8_census_traces() {
    let traces = sample(world_strategy(), 41, 5000);
    let picks = sample((0usize..64, 0usize..64), 42, traces.len());
    let (mut same, mut order) = (Hist::new(), Hist::new());
    for (ops, (i, j)) in traces.iter().zip(picks) {
        let cs = optrace::run(ops);
        let (a, b) = (i % cs.len(), j % cs.len());
        same.bump(a == b);
        order.bump(format!(
            "{:?}",
            cs[a].version().partial_cmp(&cs[b].version())
        ));
    }
    trace_census("world_strategy", traces);
    same.show("organic picks", "same clock");
    order.show("organic picks", "v[0] vs v[1] partial_cmp");
}

/// Census of the family generators: how often the receiver and items are
/// pairwise disjoint (the success path of `join_all`), by arity.
#[test]
#[ignore = "exploratory census"]
fn l8_census_party_family_success() {
    use crate::testing::generators::{arb_clock_family, arb_party_family};
    let mut by_arity: BTreeMap<usize, (usize, usize)> = BTreeMap::new();
    for (r, items) in sample(arb_party_family(), 51, N) {
        let mut all = vec![r];
        all.extend(items.iter().cloned());
        let ok = all
            .iter()
            .enumerate()
            .all(|(i, a)| all[i + 1..].iter().all(|b| a.is_disjoint(b)));
        let e = by_arity.entry(items.len()).or_default();
        e.0 += 1;
        e.1 += usize::from(ok);
    }
    let (total, ok): (usize, usize) = by_arity
        .values()
        .fold((0, 0), |a, v| (a.0 + v.0, a.1 + v.1));
    eprintln!(
        "== arb_party_family: success {ok} of {total} ({:.2}%)",
        100.0 * ok as f64 / total as f64
    );
    for (arity, (n, s)) in &by_arity {
        eprintln!("   arity {arity}: {s} of {n}");
    }
    let (mut ok, mut total) = (0, 0);
    for ((rp, _), items) in sample(arb_clock_family(), 52, N) {
        let mut all = vec![rp];
        all.extend(items.iter().map(|(p, _)| p.clone()));
        total += 1;
        ok += usize::from(
            all.iter()
                .enumerate()
                .all(|(i, a)| all[i + 1..].iter().all(|b| a.is_disjoint(b))),
        );
    }
    eprintln!(
        "== arb_clock_family: success {ok} of {total} ({:.2}%)",
        100.0 * ok as f64 / total as f64
    );
}

// ───────────────────── rescue census: inside the family generators ─────────────────────

/// Census of the relation mix inside the committed receiver-and-items family
/// generators: every unordered pair among the receiver and its items, how
/// often a family holds an equal pair, how often the receiver reappears among
/// its items, how many distinct values a family holds, and how often the empty
/// version (the pool's appended identity) appears.
#[test]
#[ignore = "exploratory census"]
fn rescue_census_families() {
    use crate::testing::generators::{arb_clock_family, arb_party_family, arb_version_family};
    use std::cmp::Ordering;

    const FAMILIES: usize = 5_000;

    // Versions.
    let (mut pairs, mut arity, mut distinct) = (Hist::new(), Numeric::new(), Numeric::new());
    let (mut any_equal, mut receiver_repeats, mut has_empty, mut any_ordered) = (0, 0, 0, 0);
    for (r, items) in sample(arb_version_family(), 61, FAMILIES) {
        arity.push(items.len() as u64);
        receiver_repeats += usize::from(items.iter().any(|v| *v == r));
        let mut all = vec![r];
        all.extend(items);
        has_empty += usize::from(all.iter().any(|v| *v == tree::Version::new()));
        let (mut eq, mut ordered) = (false, false);
        for i in 0..all.len() {
            for j in i + 1..all.len() {
                let o = all[i].partial_cmp(&all[j]);
                eq |= o == Some(Ordering::Equal);
                ordered |= matches!(o, Some(Ordering::Less | Ordering::Greater));
                pairs.bump(format!("{o:?}"));
            }
        }
        any_equal += usize::from(eq);
        any_ordered += usize::from(ordered);
        let d = (0..all.len())
            .filter(|&i| all[..i].iter().all(|w| *w != all[i]))
            .count();
        distinct.push(d as u64);
    }
    pairs.show(
        "arb_version_family",
        "pairwise partial_cmp (receiver and items)",
    );
    arity.show("arb_version_family", "item count");
    distinct.show("arb_version_family", "distinct values per family");
    eprintln!(
        "== arb_version_family: families with an equal pair {any_equal}, with a strictly \
         ordered pair {any_ordered}, receiver repeated among items {receiver_repeats}, \
         holding the empty version {has_empty} (of {FAMILIES})"
    );

    // Parties.
    let (mut pairs, mut distinct) = (Hist::new(), Numeric::new());
    let (mut any_equal, mut all_disjoint) = (0, 0);
    for (r, items) in sample(arb_party_family(), 62, FAMILIES) {
        let mut all = vec![r];
        all.extend(items);
        let (mut eq, mut disjoint) = (false, true);
        for i in 0..all.len() {
            for j in i + 1..all.len() {
                let (a, b) = (&all[i], &all[j]);
                let rel = if a == b {
                    "equal"
                } else if a.is_disjoint(b) {
                    "disjoint"
                } else if a.covers(b) || b.covers(a) {
                    "nested"
                } else {
                    "partial overlap"
                };
                eq |= rel == "equal";
                disjoint &= rel == "disjoint";
                pairs.bump(rel);
            }
        }
        any_equal += usize::from(eq);
        all_disjoint += usize::from(disjoint);
        let d = (0..all.len())
            .filter(|&i| all[..i].iter().all(|w| *w != all[i]))
            .count();
        distinct.push(d as u64);
    }
    pairs.show("arb_party_family", "pairwise relation (receiver and items)");
    distinct.show("arb_party_family", "distinct values per family");
    eprintln!(
        "== arb_party_family: families with an equal pair {any_equal}, pairwise disjoint \
         {all_disjoint} (of {FAMILIES})"
    );

    // Clocks: party and version relations separately.
    let (mut party_pairs, mut version_pairs) = (Hist::new(), Hist::new());
    for ((rp, rv), items) in sample(arb_clock_family(), 63, FAMILIES) {
        let mut all = vec![(rp, rv)];
        all.extend(items);
        for i in 0..all.len() {
            for j in i + 1..all.len() {
                let ((p, v), (q, w)) = (&all[i], &all[j]);
                party_pairs.bump(if p == q {
                    "equal"
                } else if p.is_disjoint(q) {
                    "disjoint"
                } else {
                    "overlapping"
                });
                version_pairs.bump(format!("{:?}", v.partial_cmp(w)));
            }
        }
    }
    party_pairs.show("arb_clock_family", "pairwise party relation");
    version_pairs.show("arb_clock_family", "pairwise version partial_cmp");
}
