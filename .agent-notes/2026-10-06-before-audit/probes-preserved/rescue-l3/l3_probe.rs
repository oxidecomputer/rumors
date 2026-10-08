//! L3 audit probes (explore branch only): co-generated (version, party) pairs.
//!
//! The committed arbitrary-pair suites draw the party and the version
//! independently at recursion depth 4, so nested lookahead sites (a party
//! branch with an owned left child over a version branch, inside the right
//! sibling of another such site) are rare. This generator builds the two trees
//! together, region by region, so the walk's interesting arms (lookahead
//! sites, nested pre-scans, owned-right raises, expansions under version
//! leaves) appear at controlled depth, and draws leaf heights from a small
//! per-case palette so ties, near-ties, and wide values all recur.

use num_bigint::BigUint;
use proptest::prelude::*;

use crate::testing::bridge::{
    from_oracle_party, from_oracle_version, to_oracle_party, to_oracle_version,
};
use crate::testing::oracles::tree::{Party as OP, Version as OV};
use crate::version::instrument::validate;
use crate::{Count, Party, Version};

use super::{Decision, TickWalk};

/// A version subtree skeleton whose leaves index the case's height palette.
#[derive(Clone, Debug)]
pub(crate) enum VSk {
    Leaf(u8),
    Node(Box<VSk>, Box<VSk>),
}

/// A party subtree skeleton.
#[derive(Clone, Debug)]
pub(crate) enum PSk {
    Leaf(bool),
    Node(Box<PSk>, Box<PSk>),
}

/// A co-generated region: the party and version subtrees over one interval.
#[derive(Clone, Debug)]
pub(crate) enum Pair {
    /// The party owns the whole region over an arbitrary version subtree.
    Owned(VSk),
    /// The party owns nothing here.
    Unowned(VSk),
    /// A party branch over one version leaf.
    OverLeaf(PSk, PSk, u8),
    /// Both trees branch here.
    Aligned(Box<Pair>, Box<Pair>),
}

pub(crate) fn arb_vsk(depth: u32) -> impl Strategy<Value = VSk> {
    any::<u8>()
        .prop_map(VSk::Leaf)
        .prop_recursive(depth, 24, 2, |inner| {
            (inner.clone(), inner).prop_map(|(l, r)| VSk::Node(Box::new(l), Box::new(r)))
        })
}

pub(crate) fn arb_psk(depth: u32) -> impl Strategy<Value = PSk> {
    any::<bool>()
        .prop_map(PSk::Leaf)
        .prop_recursive(depth, 16, 2, |inner| {
            (inner.clone(), inner).prop_map(|(l, r)| PSk::Node(Box::new(l), Box::new(r)))
        })
}

pub(crate) fn arb_pair(depth: u32, nodes: u32) -> impl Strategy<Value = Pair> {
    let leaf = prop_oneof![
        3 => arb_vsk(2).prop_map(Pair::Owned),
        1 => any::<u8>().prop_map(|h| Pair::Owned(VSk::Leaf(h))),
        2 => arb_vsk(2).prop_map(Pair::Unowned),
        1 => (arb_psk(2), arb_psk(2), any::<u8>()).prop_map(|(l, r, h)| Pair::OverLeaf(l, r, h)),
    ];
    leaf.prop_recursive(depth, nodes, 2, |inner| {
        prop_oneof![
            6 => (inner.clone(), inner.clone())
                .prop_map(|(l, r)| Pair::Aligned(Box::new(l), Box::new(r))),
            // Lookahead sites: an owned left beside a partial right.
            3 => (arb_vsk(2), inner.clone())
                .prop_map(|(l, r)| Pair::Aligned(Box::new(Pair::Owned(l)), Box::new(r))),
            // Owned-right raises: a partial left beside an owned right.
            2 => (inner, arb_vsk(2))
                .prop_map(|(l, r)| Pair::Aligned(Box::new(l), Box::new(Pair::Owned(r)))),
        ]
    })
}

/// One spine level: which side continues, and what sits on the other side.
#[derive(Clone, Debug)]
pub(crate) struct SpineLevel {
    continue_right: bool,
    side: Pair,
}

/// A deep spine of co-generated levels ending in one region.
pub(crate) fn arb_spine(max_len: usize) -> impl Strategy<Value = Pair> {
    let side = prop_oneof![
        4 => arb_vsk(2).prop_map(Pair::Owned),
        2 => arb_vsk(2).prop_map(Pair::Unowned),
        1 => (arb_psk(1), arb_psk(1), any::<u8>()).prop_map(|(l, r, h)| Pair::OverLeaf(l, r, h)),
        2 => arb_pair(2, 4),
    ];
    let level = (prop::bool::weighted(0.7), side).prop_map(|(continue_right, side)| SpineLevel {
        continue_right,
        side,
    });
    (prop::collection::vec(level, 1..max_len), arb_pair(2, 4)).prop_map(|(levels, tip)| {
        let mut acc = tip;
        for level in levels.into_iter().rev() {
            acc = if level.continue_right {
                Pair::Aligned(Box::new(level.side), Box::new(acc))
            } else {
                Pair::Aligned(Box::new(acc), Box::new(level.side))
            };
        }
        acc
    })
}

/// A per-case palette of absolute leaf heights.
pub(crate) fn arb_palette() -> impl Strategy<Value = Vec<BigUint>> {
    let base = prop_oneof![
        4 => (0u64..6).prop_map(BigUint::from),
        2 => (0u64..1000).prop_map(BigUint::from),
        1 => any::<u64>().prop_map(BigUint::from),
        1 => (u64::MAX - 3..=u64::MAX).prop_map(BigUint::from),
        2 => (0u64..4).prop_map(|k| (BigUint::from(1u8) << 64u32) + k),
        2 => (0u64..4).prop_map(|k| (BigUint::from(1u8) << 130u32) + k),
        1 => (1u64..4, 60u32..200).prop_map(|(m, s)| BigUint::from(m) << s),
    ];
    prop::collection::vec(base, 1..=5)
}

fn height(palette: &[BigUint], tag: u8) -> BigUint {
    palette[usize::from(tag) % palette.len()].clone()
}

fn build_v(sk: &VSk, palette: &[BigUint]) -> OV {
    match sk {
        VSk::Leaf(h) => OV::Leaf(height(palette, *h)),
        VSk::Node(l, r) => OV::node(0u64, build_v(l, palette), build_v(r, palette)),
    }
}

fn build_p(sk: &PSk) -> OP {
    match sk {
        PSk::Leaf(b) => OP::Leaf(*b),
        PSk::Node(l, r) => OP::node(build_p(l), build_p(r)),
    }
}

/// Resolve a co-generated region into normal-form oracle trees.
pub(crate) fn build(pair: &Pair, palette: &[BigUint]) -> (OP, OV) {
    match pair {
        Pair::Owned(v) => (OP::Leaf(true), build_v(v, palette)),
        Pair::Unowned(v) => (OP::Leaf(false), build_v(v, palette)),
        Pair::OverLeaf(l, r, h) => (
            OP::node(build_p(l), build_p(r)),
            OV::Leaf(height(palette, *h)),
        ),
        Pair::Aligned(l, r) => {
            let (pl, vl) = build(l, palette);
            let (pr, vr) = build(r, palette);
            (OP::node(pl, pr), OV::node(0u64, vl, vr))
        }
    }
}

/// Resolve to production values; an empty party becomes the seed.
pub(crate) fn resolve(pair: &Pair, palette: &[BigUint]) -> (Version, Party, OV, OP) {
    let (op, ov) = build(pair, palette);
    let op = if op.is_empty() { OP::Leaf(true) } else { op };
    (from_oracle_version(&ov), from_oracle_party(&op), ov, op)
}

/// The oracle party owning exactly the complement of `p`.
pub(crate) fn complement(p: &OP) -> OP {
    match p {
        OP::Leaf(b) => OP::Leaf(!b),
        OP::Node(l, r) => OP::node(complement(l), complement(r)),
    }
}

fn oracle_tick(ov: &OV, op: &OP) -> Version {
    let mut o = ov.clone();
    o.tick(op);
    from_oracle_version(&o)
}

/// Every claim about one tick, checked on one pair.
pub(crate) fn check_one(v: &Version, p: &Party, ov: &OV, op: &OP) -> Result<(), TestCaseError> {
    // Paper fidelity: the fill flag and the event.
    let filled = from_oracle_version(&ov.fill_for_test(op));
    let changed = filled != *v;
    match TickWalk::decide(v, p) {
        Decision::Simplified(out) => {
            prop_assert!(
                changed,
                "flag tripped but oracle fill is identity: {:?} {:?}",
                ov,
                op
            );
            prop_assert_eq!(&out, &filled, "fill output differs: {:?} {:?}", ov, op);
        }
        Decision::Raise(_) => {
            prop_assert!(
                !changed,
                "flag clear but oracle fill moved: {:?} {:?}",
                ov,
                op
            );
        }
    }
    let out = {
        let mut w = v.clone();
        w.tick(p);
        w
    };
    prop_assert!(
        validate(&out).is_ok(),
        "tick output not canonical: {:?} {:?}",
        ov,
        op
    );
    let expected = oracle_tick(ov, op);
    prop_assert_eq!(
        &out,
        &expected,
        "tick differs from oracle event: {:?} {:?}",
        ov,
        op
    );

    // Strict domination.
    prop_assert!(
        out > *v,
        "tick does not strictly dominate: {:?} {:?}",
        ov,
        op
    );

    // The committed output envelope (tick/tests.rs `tick_output_is_input_bounded`).
    let bound = 2 * v.stored_len() + 4 * p.stored_len() + 32;
    prop_assert!(
        out.stored_len() <= bound,
        "tick output {} bits exceeds the envelope {} (event {}, party {}): {:?} {:?}",
        out.stored_len(),
        bound,
        v.stored_len(),
        p.stored_len(),
        ov,
        op
    );

    // Region locality: the complement's projection is unchanged.
    let oq = complement(op);
    if !oq.is_empty() {
        let q = from_oracle_party(&oq);
        let before = (v / &q).to_version();
        let after = (&out / &q).to_version();
        prop_assert_eq!(
            after,
            before,
            "tick changed history outside the party: {:?} {:?}",
            ov,
            op
        );
    }

    // Floor step: one tick raises min_ticks by at most one.
    let m0 = v.min_ticks();
    let m1 = out.min_ticks();
    prop_assert!(
        m1 <= Count(m0.0.clone() + 1u8),
        "min_ticks rose by more than one: {:?} {:?}",
        ov,
        op
    );

    // ticks(k) is k sequential ticks, for small k against both the iterated
    // production tick and the oracle's iteration.
    let mut iterated = v.clone();
    let mut oracle = ov.clone();
    for k in 1u64..=4 {
        iterated.tick(p);
        oracle.tick(op);
        let mut fused = v.clone();
        fused.ticks(p, k);
        prop_assert_eq!(
            &fused,
            &iterated,
            "ticks({}) differs from iterated tick: {:?} {:?}",
            k,
            ov,
            op
        );
        prop_assert_eq!(
            &fused,
            &from_oracle_version(&oracle),
            "ticks({}) differs from oracle: {:?} {:?}",
            k,
            ov,
            op
        );
    }
    let mut zero = v.clone();
    zero.ticks(p, 0u64);
    prop_assert_eq!(&zero, v, "ticks(0) is not the identity");

    // Composition at wide counts: ticks(a) then ticks(b) is ticks(a + b).
    let wides = [
        BigUint::from(1u8),
        BigUint::from(u64::MAX),
        (BigUint::from(1u8) << 64u32) + 3u8,
        (BigUint::from(1u8) << 200u32) - 1u8,
    ];
    for a in &wides {
        for b in &wides {
            let mut stepwise = v.clone();
            stepwise.ticks(p, Count(a.clone()));
            stepwise.ticks(p, Count(b.clone()));
            let mut joint = v.clone();
            joint.ticks(p, Count(a + b));
            prop_assert_eq!(
                &stepwise,
                &joint,
                "ticks({}) then ticks({}) differs from ticks(sum): {:?} {:?}",
                a,
                b,
                ov,
                op
            );
            // Floor at wide counts.
            prop_assert!(
                joint.min_ticks() <= Count(m0.0.clone() + a + b),
                "min_ticks rose by more than the count"
            );
        }
    }
    Ok(())
}

proptest! {
    /// Co-generated bushy pairs satisfy every tick claim.
    #[test]
    fn l3_cogen_bushy(pair in arb_pair(7, 48), palette in arb_palette()) {
        let (v, p, ov, op) = resolve(&pair, &palette);
        check_one(&v, &p, &ov, &op)?;
    }

    /// Co-generated deep spines satisfy every tick claim.
    #[test]
    fn l3_cogen_spine(pair in arb_spine(40), palette in arb_palette()) {
        let (v, p, ov, op) = resolve(&pair, &palette);
        check_one(&v, &p, &ov, &op)?;
    }
}

/// A trivial sanity check of the bridge round trip on co-generated values.
#[test]
fn l3_bridge_round_trip() {
    let pair = Pair::Aligned(
        Box::new(Pair::Owned(VSk::Leaf(0))),
        Box::new(Pair::Aligned(
            Box::new(Pair::Owned(VSk::Node(
                Box::new(VSk::Leaf(1)),
                Box::new(VSk::Leaf(2)),
            ))),
            Box::new(Pair::Unowned(VSk::Leaf(0))),
        )),
    );
    let palette = vec![BigUint::from(3u8), BigUint::from(1u8), BigUint::from(7u8)];
    let (v, p, ov, op) = resolve(&pair, &palette);
    assert_eq!(to_oracle_version(&v), ov);
    assert_eq!(to_oracle_party(&p), op);
}

// ───────────────────────── min_ticks floor over histories ─────────────────────────

use std::collections::BTreeMap;

use crate::Clock;

/// A clock with the tick batches in its causal past: batch id to weight.
struct Tracked {
    clock: Clock,
    past: BTreeMap<u64, BigUint>,
}

fn past_size(past: &BTreeMap<u64, BigUint>) -> BigUint {
    past.values().fold(BigUint::ZERO, |acc, w| acc + w)
}

fn union_into(into: &mut BTreeMap<u64, BigUint>, from: &BTreeMap<u64, BigUint>) {
    for (k, w) in from {
        into.insert(*k, w.clone());
    }
}

#[derive(Clone, Debug)]
pub(crate) enum HOp {
    Tick(u8),
    /// Wide fused ticks: `2^shift + low`.
    Ticks(u8, u8, u8),
    Fork(u8),
    Send(u8, u8),
    Absorb(u8, u8),
    Sync(u8, u8),
    Join(u8, u8),
    /// Party of `i` ticks a copy of `j`'s version, which `j` then absorbs.
    ForeignTick(u8, u8),
}

pub(crate) fn arb_hop() -> impl Strategy<Value = HOp> {
    prop_oneof![
        4 => any::<u8>().prop_map(HOp::Tick),
        2 => (any::<u8>(), 0u8..80, 0u8..4).prop_map(|(i, s, l)| HOp::Ticks(i, s, l)),
        3 => any::<u8>().prop_map(HOp::Fork),
        2 => (any::<u8>(), any::<u8>()).prop_map(|(a, b)| HOp::Send(a, b)),
        2 => (any::<u8>(), any::<u8>()).prop_map(|(a, b)| HOp::Absorb(a, b)),
        1 => (any::<u8>(), any::<u8>()).prop_map(|(a, b)| HOp::Sync(a, b)),
        1 => (any::<u8>(), any::<u8>()).prop_map(|(a, b)| HOp::Join(a, b)),
        2 => (any::<u8>(), any::<u8>()).prop_map(|(a, b)| HOp::ForeignTick(a, b)),
    ]
}

/// Run a history, checking the floor after every step. Returns the number of
/// clocks whose floor was exactly tight at the end (a coverage statistic).
pub(crate) fn run_history(ops: &[HOp]) -> Result<usize, TestCaseError> {
    let mut world = vec![Tracked {
        clock: Clock::seed(),
        past: BTreeMap::new(),
    }];
    let mut next = 0u64;
    let mut fresh = |w: BigUint, past: &mut BTreeMap<u64, BigUint>| {
        past.insert(next, w);
        next += 1;
    };
    for op in ops {
        let n = world.len();
        let ix = |k: u8| usize::from(k) % n;
        match *op {
            HOp::Tick(i) => {
                let t = &mut world[ix(i)];
                t.clock.tick();
                fresh(BigUint::from(1u8), &mut t.past);
            }
            HOp::Ticks(i, s, l) => {
                let w = (BigUint::from(1u8) << u32::from(s)) + l;
                let t = &mut world[ix(i)];
                t.clock.ticks(Count(w.clone()));
                fresh(w, &mut t.past);
            }
            HOp::Fork(i) => {
                let t = &mut world[ix(i)];
                let child = t.clock.fork();
                let past = t.past.clone();
                world.push(Tracked { clock: child, past });
            }
            HOp::Send(i, j) => {
                let (i, j) = (ix(i), ix(j));
                let msg = world[i].clock.send().clone();
                fresh(BigUint::from(1u8), &mut world[i].past);
                let mp = world[i].past.clone();
                world[j].clock.recv(&msg);
                union_into(&mut world[j].past, &mp);
                fresh(BigUint::from(1u8), &mut world[j].past);
            }
            HOp::Absorb(i, j) => {
                let (i, j) = (ix(i), ix(j));
                let msg = world[i].clock.version().clone();
                let mp = world[i].past.clone();
                world[j].clock.absorb(&msg);
                union_into(&mut world[j].past, &mp);
            }
            HOp::Sync(i, j) => {
                let (i, j) = (ix(i), ix(j));
                if i != j {
                    let (lo, hi) = (i.min(j), i.max(j));
                    let (left, right) = world.split_at_mut(hi);
                    let (a, b) = (&mut left[lo], &mut right[0]);
                    a.clock
                        .sync(&mut b.clock)
                        .expect("linear history stays disjoint");
                    let mut u = a.past.clone();
                    union_into(&mut u, &b.past);
                    a.past = u.clone();
                    b.past = u;
                }
            }
            HOp::Join(i, j) => {
                let (i, j) = (ix(i), ix(j));
                if i != j && n > 1 {
                    let other = world.remove(j);
                    let r = if j < i { i - 1 } else { i };
                    world[r]
                        .clock
                        .join(other.clock)
                        .map_err(|_| ())
                        .expect("disjoint");
                    union_into(&mut world[r].past, &other.past);
                }
            }
            HOp::ForeignTick(i, j) => {
                let (i, j) = (ix(i), ix(j));
                let mut v = world[j].clock.version().clone();
                v.tick(world[i].clock.party());
                // The tick is a new event in v's past: j's past plus i's? No:
                // only j's past plus the new tick.
                let mut vp = world[j].past.clone();
                fresh(BigUint::from(1u8), &mut vp);
                world[j].clock.absorb(&v);
                world[j].past = vp;
            }
        }
        for t in &world {
            let floor = t.clock.version().min_ticks();
            let have = past_size(&t.past);
            prop_assert!(
                floor.0 <= have,
                "min_ticks {} exceeds the {} ticks in the causal past after {:?}",
                floor.0,
                have,
                op
            );
        }
    }
    Ok(world
        .iter()
        .filter(|t| t.clock.version().min_ticks().0 == past_size(&t.past))
        .count())
}

proptest! {
    /// Every version a linear history produces has `min_ticks` at most the
    /// ticks in its causal past.
    #[test]
    fn l3_min_ticks_floor_over_histories(ops in prop::collection::vec(arb_hop(), 0..60)) {
        run_history(&ops)?;
    }
}

// ───────────────────────── generator coverage statistics ─────────────────────────

#[derive(Default, Debug, Clone)]
pub(crate) struct Stats {
    sites: u64,
    max_site_nesting: u64,
    multi_site_ranges: u64,
    owned_right_raises: u64,
    over_leaf: u64,
    wide_leaves: u64,
    max_depth: u64,
}

fn walk_stats(op: &OP, ov: &OV, nesting: u64, depth: u64, st: &mut Stats) -> u64 {
    // Returns the number of sites at the next nesting level inside this region
    // (sites not inside another site's range).
    st.max_depth = st.max_depth.max(depth);
    match (op, ov) {
        (_, OV::Leaf(h)) => {
            if h.bits() > 64 {
                st.wide_leaves += 1;
            }
            if matches!(op, OP::Node(..)) {
                st.over_leaf += 1;
            }
            0
        }
        (OP::Leaf(_), OV::Node(_, l, r)) => {
            let mut w = 0;
            for c in [l, r] {
                if let OV::Leaf(h) = &**c {
                    if h.bits() > 64 {
                        st.wide_leaves += 1;
                    }
                }
            }
            let _ = &mut w;
            0
        }
        (OP::Node(pl, pr), OV::Node(_, el, er)) => {
            if matches!(**pl, OP::Leaf(true)) && !pr.is_empty() {
                st.sites += 1;
                let nest = nesting + 1;
                st.max_site_nesting = st.max_site_nesting.max(nest);
                let inner = walk_stats(pr, er, nest, depth + 1, st);
                if inner >= 2 {
                    st.multi_site_ranges += 1;
                }
                1
            } else {
                if matches!(**pr, OP::Leaf(true)) && !pl.is_empty() {
                    st.owned_right_raises += 1;
                }
                walk_stats(pl, el, nesting, depth + 1, st)
                    + walk_stats(pr, er, nesting, depth + 1, st)
            }
        }
    }
}

fn histogram_of<S: Strategy<Value = (OP, OV)>>(name: &str, strat: S) {
    use proptest::strategy::ValueTree;
    use proptest::test_runner::TestRunner;
    let mut runner = TestRunner::deterministic();
    let n = 2000;
    let (
        mut changed,
        mut sites_hist,
        mut nest_hist,
        mut multi,
        mut orr,
        mut over,
        mut wide,
        mut depth_hist,
    ) = (
        0u64, [0u64; 8], [0u64; 8], 0u64, 0u64, 0u64, 0u64, [0u64; 8],
    );
    for _ in 0..n {
        let (op, ov) = strat.new_tree(&mut runner).unwrap().current();
        let op = if op.is_empty() { OP::Leaf(true) } else { op };
        let v = from_oracle_version(&ov);
        let p = from_oracle_party(&op);
        if matches!(TickWalk::decide(&v, &p), Decision::Simplified(_)) {
            changed += 1;
        }
        let mut st = Stats::default();
        walk_stats(&op, &ov, 0, 0, &mut st);
        sites_hist[(st.sites as usize).min(7)] += 1;
        nest_hist[(st.max_site_nesting as usize).min(7)] += 1;
        depth_hist[((st.max_depth / 4) as usize).min(7)] += 1;
        multi += u64::from(st.multi_site_ranges > 0);
        orr += u64::from(st.owned_right_raises > 0);
        over += u64::from(st.over_leaf > 0);
        wide += u64::from(st.wide_leaves > 0);
    }
    eprintln!(
        "HIST {name}: n={n} fill-changed={changed} sites(0..7+)={sites_hist:?} \
         max-site-nesting(0..7+)={nest_hist:?} depth/4(0..7+)={depth_hist:?} \
         multi-site-range={multi} owned-right={orr} over-leaf={over} wide={wide}"
    );
}

fn resolved<S: Strategy<Value = Pair>>(strat: S) -> impl Strategy<Value = (OP, OV)> {
    (strat, arb_palette()).prop_map(|(pair, palette)| build(&pair, &palette))
}

/// Prints generator coverage histograms (diagnostic; always passes).
#[test]
fn l3_generator_histograms() {
    histogram_of("bushy", resolved(arb_pair(7, 48)));
    histogram_of("spine", resolved(arb_spine(40)));
    histogram_of(
        "committed-arbitrary",
        (
            crate::testing::generators::arb_oracle_party_nonempty(),
            crate::testing::generators::arb_oracle_version(),
        ),
    );
}

// ───────────────────────── late-divergence perturbations ─────────────────────────

/// An event tree with absolute leaf heights.
#[derive(Clone, Debug)]
enum Abs {
    Leaf(BigUint),
    Node(Box<Abs>, Box<Abs>),
}

fn to_abs(ov: &OV, off: &BigUint) -> Abs {
    match ov {
        OV::Leaf(n) => Abs::Leaf(off + n),
        OV::Node(n, l, r) => {
            let o = off + n;
            Abs::Node(Box::new(to_abs(l, &o)), Box::new(to_abs(r, &o)))
        }
    }
}

fn from_abs(a: &Abs) -> OV {
    match a {
        Abs::Leaf(h) => OV::Leaf(h.clone()),
        Abs::Node(l, r) => OV::node(0u64, from_abs(l), from_abs(r)),
    }
}

fn leaf_count(a: &Abs) -> usize {
    match a {
        Abs::Leaf(_) => 1,
        Abs::Node(l, r) => leaf_count(l) + leaf_count(r),
    }
}

/// Replace the `k`-th leaf (preorder) with `h`.
fn set_leaf(a: &mut Abs, k: &mut usize, h: &BigUint) -> bool {
    match a {
        Abs::Leaf(x) => {
            if *k == 0 {
                *x = h.clone();
                return true;
            }
            *k -= 1;
            false
        }
        Abs::Node(l, r) => set_leaf(l, k, h) || set_leaf(r, k, h),
    }
}

/// Replace one leaf, chosen `from_end` leaves before the last, with `h`.
fn perturb(ov: &OV, from_end: usize, h: &BigUint) -> OV {
    let mut a = to_abs(ov, &BigUint::ZERO);
    let n = leaf_count(&a);
    let mut k = n - 1 - (from_end % n);
    set_leaf(&mut a, &mut k, h);
    from_abs(&a)
}

/// The base pair, its once-ticked fixed point, and a late perturbation of it.
pub(crate) fn check_family(
    pair: &Pair,
    palette: &[BigUint],
    from_end: usize,
    tag: u8,
) -> Result<(), TestCaseError> {
    let (v, p, ov, op) = resolve(pair, palette);
    check_one(&v, &p, &ov, &op)?;
    let mut o1 = ov.clone();
    o1.tick(&op);
    let v1 = from_oracle_version(&o1);
    check_one(&v1, &p, &o1, &op)?;
    let o2 = perturb(&o1, from_end, &height(palette, tag));
    let v2 = from_oracle_version(&o2);
    check_one(&v2, &p, &o2, &op)?;
    // A second perturbation one leaf earlier, to vary the divergence site.
    let o3 = perturb(&o1, from_end + 1, &(height(palette, tag) + 1u8));
    let v3 = from_oracle_version(&o3);
    check_one(&v3, &p, &o3, &op)
}

proptest! {
    /// Bushy pairs, their fixed points, and late perturbations satisfy every
    /// tick claim.
    #[test]
    fn l3_family_bushy(
        pair in arb_pair(10, 96),
        palette in arb_palette(),
        from_end in 0usize..6,
        tag in any::<u8>(),
    ) {
        check_family(&pair, &palette, from_end, tag)?;
    }

    /// Deep spines, their fixed points, and late perturbations satisfy every
    /// tick claim.
    #[test]
    fn l3_family_spine(
        pair in arb_spine(48),
        palette in arb_palette(),
        from_end in 0usize..6,
        tag in any::<u8>(),
    ) {
        check_family(&pair, &palette, from_end, tag)?;
    }
}

// ───────────────────────── min_ticks tightness ─────────────────────────

/// Run `f` with a party owning exactly the dyadic interval at `path`, forked
/// down from `seed` and joined back afterwards (one universe throughout).
fn with_path_party(seed: &mut Party, path: &[bool], f: impl FnOnce(&Party)) {
    let mut parked = Vec::new();
    let mut p = core::mem::replace(seed, Party::seed());
    for &right in path {
        let give = p.fork();
        if right {
            parked.push(core::mem::replace(&mut p, give));
        } else {
            parked.push(give);
        }
    }
    f(&p);
    while let Some(q) = parked.pop() {
        p.join(q)
            .map_err(|_| ())
            .expect("parked halves are disjoint");
    }
    *seed = p;
}

/// Realize `ov` by a linear history: preorder over the normal-form tree, each
/// node's base applied by that many ticks of a party owning its interval.
/// Returns the version reached and the ticks spent.
fn realize(ov: &OV) -> (Version, BigUint) {
    fn rec(
        node: &OV,
        path: &mut Vec<bool>,
        seed: &mut Party,
        w: &mut Version,
        spent: &mut BigUint,
    ) {
        let base = match node {
            OV::Leaf(n) | OV::Node(n, ..) => n.clone(),
        };
        if base != BigUint::ZERO {
            with_path_party(seed, path, |p| w.ticks(p, Count(base.clone())));
            *spent += &base;
        }
        if let OV::Node(_, l, r) = node {
            path.push(false);
            rec(l, path, seed, w, spent);
            path.pop();
            path.push(true);
            rec(r, path, seed, w, spent);
            path.pop();
        }
    }
    let mut seed = Party::seed();
    let mut w = Version::new();
    let mut spent = BigUint::ZERO;
    rec(ov, &mut Vec::new(), &mut seed, &mut w, &mut spent);
    assert!(seed.is_seed(), "the universe reassembles");
    (w, spent)
}

proptest! {
    /// Every canonical version is reached by a linear history spending
    /// exactly `min_ticks` ticks.
    #[test]
    fn l3_min_ticks_tight(ov in crate::testing::generators::arb_oracle_version()) {
        let v = from_oracle_version(&ov);
        let (w, spent) = realize(&ov);
        prop_assert_eq!(&w, &v, "realized history reaches a different version");
        prop_assert_eq!(v.min_ticks().0, spent, "realized history spends a different count");
    }

    /// Tightness on co-generated deep versions.
    #[test]
    fn l3_min_ticks_tight_spine(pair in arb_spine(40), palette in arb_palette()) {
        let (_, _, ov, _) = resolve(&pair, &palette);
        let v = from_oracle_version(&ov);
        let (w, spent) = realize(&ov);
        prop_assert_eq!(&w, &v, "realized history reaches a different version");
        prop_assert_eq!(v.min_ticks().0, spent, "realized history spends a different count");
    }
}

// ───────────────────────── wide pre-scans crossing memo blocks ─────────────────────────

/// One outermost lookahead over a balanced tree of `n` random regions, so a
/// single pre-scan reserves more memo slots than one 64-slot block holds.
pub(crate) fn arb_wide(lo: usize, hi: usize) -> impl Strategy<Value = Pair> {
    let region = prop_oneof![
        // A minimal lookahead site with a one-leaf range.
        4 => (any::<u8>(), any::<u8>()).prop_map(|(l, c)| Pair::Aligned(
            Box::new(Pair::Owned(VSk::Leaf(l))),
            Box::new(Pair::OverLeaf(PSk::Leaf(true), PSk::Leaf(false), c)),
        )),
        // A site whose range nests another site.
        2 => (any::<u8>(), any::<u8>(), any::<u8>()).prop_map(|(l, m, c)| Pair::Aligned(
            Box::new(Pair::Owned(VSk::Leaf(l))),
            Box::new(Pair::Aligned(
                Box::new(Pair::Owned(VSk::Leaf(m))),
                Box::new(Pair::OverLeaf(PSk::Leaf(false), PSk::Leaf(true), c)),
            )),
        )),
        2 => arb_vsk(1).prop_map(Pair::Unowned),
        1 => arb_vsk(1).prop_map(Pair::Owned),
        2 => arb_pair(2, 4),
    ];
    (prop::collection::vec(region, lo..hi), any::<u8>()).prop_map(|(regions, root_left)| {
        fn balance(mut v: Vec<Pair>) -> Pair {
            if v.len() == 1 {
                return v.pop().expect("one region");
            }
            let right = v.split_off(v.len() / 2);
            Pair::Aligned(Box::new(balance(v)), Box::new(balance(right)))
        }
        Pair::Aligned(
            Box::new(Pair::Owned(VSk::Leaf(root_left))),
            Box::new(balance(regions)),
        )
    })
}

proptest! {
    /// Pre-scans whose memo spans several blocks satisfy every tick claim.
    #[test]
    fn l3_family_wide(
        pair in arb_wide(40, 400),
        palette in arb_palette(),
        from_end in 0usize..6,
        tag in any::<u8>(),
    ) {
        check_family(&pair, &palette, from_end, tag)?;
    }
}

// ───────────────────────── deep, production-only claims ─────────────────────────

/// Every claim that needs no oracle, on one deep pair.
fn check_deep(v: &Version, p: &Party, op: &OP) -> Result<(), TestCaseError> {
    let mut out = v.clone();
    out.tick(p);
    prop_assert!(validate(&out).is_ok(), "tick output not canonical");
    prop_assert!(out > *v, "tick does not strictly dominate");
    let oq = complement(op);
    if !oq.is_empty() {
        let q = from_oracle_party(&oq);
        prop_assert_eq!(
            (&out / &q).to_version(),
            (v / &q).to_version(),
            "tick changed history outside the party"
        );
        core::mem::forget(q);
    }
    if let Decision::Simplified(s) = TickWalk::decide(v, p) {
        prop_assert!(
            matches!(TickWalk::decide(&s, p), Decision::Raise(_)),
            "fill is not idempotent"
        );
    }
    let m0 = v.min_ticks().0;
    prop_assert!(
        out.min_ticks().0 <= m0.clone() + 1u8,
        "min_ticks rose by more than one"
    );
    let mut iterated = v.clone();
    for k in 1u64..=3 {
        iterated.tick(p);
        let mut fused = v.clone();
        fused.ticks(p, k);
        prop_assert_eq!(&fused, &iterated, "ticks({}) differs from iterated tick", k);
    }
    let a = (BigUint::from(1u8) << 70u32) + 5u8;
    let b = BigUint::from(3u8);
    let mut stepwise = v.clone();
    stepwise.ticks(p, Count(a.clone()));
    stepwise.ticks(p, Count(b.clone()));
    let mut joint = v.clone();
    joint.ticks(p, Count(a + b));
    prop_assert_eq!(&stepwise, &joint, "wide composition differs");
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig { cases: std::env::var("L3_DEEP_CASES").ok().and_then(|s| s.parse().ok()).unwrap_or(8), ..ProptestConfig::default() })]

    /// Deep co-generated spines satisfy every oracle-free tick claim.
    #[test]
    fn l3_deep_spine(pair in arb_spine(3000), palette in arb_palette()) {
        let result = std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn(move || {
                let (v, p, ov, op) = resolve(&pair, &palette);
                let r = check_deep(&v, &p, &op);
                let mut o1 = ov.clone();
                o1.tick(&op);
                let v1 = from_oracle_version(&o1);
                let r = r.and_then(|()| check_deep(&v1, &p, &op));
                core::mem::forget((ov, op, o1, pair));
                r
            })
            .expect("spawn")
            .join()
            .expect("join");
        result?;
    }
}

// ───────────────────────── several pre-scans in one walk ─────────────────────────

/// Two to four outermost lookaheads side by side, each over its own wide
/// region, so one walk runs several pre-scans and reuses memo blocks.
pub(crate) fn arb_multi_wide() -> impl Strategy<Value = Pair> {
    prop::collection::vec(arb_wide(20, 160), 2..=4).prop_map(|mut scans| {
        let mut acc = scans.pop().expect("at least two scans");
        while let Some(next) = scans.pop() {
            acc = Pair::Aligned(Box::new(next), Box::new(acc));
        }
        acc
    })
}

proptest! {
    /// Walks running several wide pre-scans satisfy every tick claim.
    #[test]
    fn l3_family_multi_wide(
        pair in arb_multi_wide(),
        palette in arb_palette(),
        from_end in 0usize..6,
        tag in any::<u8>(),
    ) {
        check_family(&pair, &palette, from_end, tag)?;
    }
}

// ───────────────────────── memo reach of the wide strategies ─────────────────────────

/// Whether `(op, ov)` is a lookahead site: an owned left child beside a
/// partially owned right child, over a version branch.
fn is_site(op: &OP, ov: &OV) -> bool {
    matches!((op, ov), (OP::Node(l, r), OV::Node(..)) if matches!(**l, OP::Leaf(true)) && !r.is_empty())
}

/// Counts every site in a region (any depth).
fn sites_in(op: &OP, ov: &OV) -> usize {
    match (op, ov) {
        (OP::Node(pl, pr), OV::Node(_, el, er)) => {
            usize::from(is_site(op, ov)) + sites_in(pl, el) + sites_in(pr, er)
        }
        _ => 0,
    }
}

/// Collects, for each outermost site, the memo slots its pre-scan reserves:
/// the sites inside its right range plus itself.
fn scans(op: &OP, ov: &OV, out: &mut Vec<usize>) {
    if let (OP::Node(pl, pr), OV::Node(_, el, er)) = (op, ov) {
        if is_site(op, ov) {
            out.push(1 + sites_in(pr, er));
            return;
        }
        scans(pl, el, out);
        scans(pr, er, out);
    }
}

fn reach_of<S: Strategy<Value = (OP, OV)>>(name: &str, strat: S) {
    use proptest::strategy::ValueTree;
    use proptest::test_runner::TestRunner;
    let mut runner = TestRunner::deterministic();
    let n = 500;
    let (mut over64, mut two_big_scans, mut max_slots, mut scans_hist) =
        (0u64, 0u64, 0usize, [0u64; 6]);
    for _ in 0..n {
        let (op, ov) = strat.new_tree(&mut runner).unwrap().current();
        let mut s = Vec::new();
        scans(&op, &ov, &mut s);
        let big = s.iter().filter(|&&k| k > 64).count();
        over64 += u64::from(big >= 1);
        two_big_scans += u64::from(big >= 2);
        max_slots = max_slots.max(s.iter().copied().max().unwrap_or(0));
        scans_hist[s.len().min(5)] += 1;
    }
    eprintln!(
        "REACH {name}: n={n} cases-with-a-scan-over-64-slots={over64} cases-with-two-such-scans={two_big_scans} max-slots-in-one-scan={max_slots} scans-per-walk(0..5+)={scans_hist:?}"
    );
}

/// Prints memo reach for the wide, multi-scan, spine, and committed generators
/// (diagnostic; always passes).
#[test]
fn l3_generator_reach() {
    reach_of("wide", resolved(arb_wide(40, 400)));
    reach_of("multi-wide", resolved(arb_multi_wide()));
    reach_of("spine", resolved(arb_spine(40)));
    reach_of("bushy", resolved(arb_pair(7, 48)));
    reach_of(
        "committed-arbitrary",
        (
            crate::testing::generators::arb_oracle_party_nonempty(),
            crate::testing::generators::arb_oracle_version(),
        ),
    );
}

// ───────────────────────── counter worst-case search ─────────────────────────

/// Samples co-generated pairs and prints the largest tick and min_ticks
/// counter readings per input byte (diagnostic; always passes). Inputs under
/// 64 bytes are skipped so fixed costs do not dominate the ratios.
#[cfg(all(feature = "touch-meter", feature = "scan-meter"))]
#[test]
fn l3_counter_search() {
    use proptest::strategy::ValueTree;
    use proptest::test_runner::TestRunner;

    fn search<S: Strategy<Value = (OP, OV)>>(name: &str, strat: S, n: usize) {
        let mut runner = TestRunner::deterministic();
        let mut best: Vec<(f64, &'static str, u64, u64, String)> = Vec::new();
        for _ in 0..n {
            let (op, ov) = strat.new_tree(&mut runner).unwrap().current();
            let op = if op.is_empty() { OP::Leaf(true) } else { op };
            let v = from_oracle_version(&ov);
            let p = from_oracle_party(&op);
            let bytes = (v.stored_len() + p.stored_len()).div_ceil(8);
            if bytes < 64 {
                continue;
            }
            let mut w = v.clone();
            crate::testing::meter::reset_scan_bits();
            suanpan::touch_meter::reset();
            w.tick(&p);
            let (scan, touch) = (
                crate::testing::meter::scan_bits(),
                suanpan::touch_meter::touches(),
            );
            let shape = format!("{ov:?} | {op:?}");
            let shape: String = shape.chars().take(300).collect();
            best.push((
                touch as f64 / bytes as f64,
                "tick-touch",
                touch,
                bytes,
                shape.clone(),
            ));
            best.push((
                scan as f64 / bytes as f64,
                "tick-scan",
                scan,
                bytes,
                shape.clone(),
            ));
            let vb = v.stored_len().div_ceil(8);
            crate::testing::meter::reset_scan_bits();
            suanpan::touch_meter::reset();
            let m = v.min_ticks();
            drop(m);
            let (scan, touch) = (
                crate::testing::meter::scan_bits(),
                suanpan::touch_meter::touches(),
            );
            best.push((
                touch as f64 / vb.max(1) as f64,
                "min_ticks-touch",
                touch,
                vb,
                shape.clone(),
            ));
            best.push((
                scan as f64 / vb.max(1) as f64,
                "min_ticks-scan",
                scan,
                vb,
                shape,
            ));
        }
        for kind in [
            "tick-touch",
            "tick-scan",
            "min_ticks-touch",
            "min_ticks-scan",
        ] {
            let mut of: Vec<_> = best.iter().filter(|b| b.1 == kind).collect();
            of.sort_by(|a, b| b.0.partial_cmp(&a.0).expect("finite"));
            for (ratio, kind, count, bytes, shape) in of.into_iter().take(3) {
                eprintln!("COUNTER {name} {kind} ratio={ratio:.2} count={count} bytes={bytes} shape={shape}");
            }
        }
    }
    search("spine", resolved(arb_spine(200)), 4000);
    search("bushy", resolved(arb_pair(10, 128)), 4000);
    search("wide", resolved(arb_wide(40, 400)), 400);
}
