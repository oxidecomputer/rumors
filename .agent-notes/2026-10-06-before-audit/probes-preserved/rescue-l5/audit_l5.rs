//! L5 audit harness (explore branch only): spans and causal queries against a
//! grid-function model of versions.
//!
//! A version on a dyadic grid of `2^d` cells is a vector of heights; the
//! causal order is the pointwise order, join and meet are pointwise max and
//! min, and projection masks unowned cells to zero. The oracle below shares
//! no code with production comparison, lattice, walk, or query code: every
//! verdict is computed from integer vectors.

#![allow(dead_code, clippy::all)]

use std::cmp::Ordering;
use std::collections::HashSet;

use num_bigint::BigUint;
use proptest::prelude::*;
use proptest::test_runner::TestCaseError;

use crate::causally::{self, Coverage, Down, Neutral, Polarity, Query, Up};
use crate::span::{Dominance, Endpoint, Placement, Precedence};
use crate::testing::bridge::{from_oracle_party, from_oracle_version};
use crate::testing::oracles::tree;
use crate::{Party, Span, Version};

// ───────────────────────────── stats ─────────────────────────────

static STATS: std::sync::Mutex<std::collections::BTreeMap<String, u64>> =
    std::sync::Mutex::new(std::collections::BTreeMap::new());

/// Count one occurrence of a behavioral event.
pub(crate) fn stat(key: impl Into<String>) {
    *STATS.lock().unwrap().entry(key.into()).or_insert(0) += 1;
}

// ───────────────────────────── vector model ─────────────────────────────

/// A grid function: one height per cell.
pub(crate) type Vect = Vec<BigUint>;

pub(crate) fn vle(a: &[BigUint], b: &[BigUint]) -> bool {
    a.iter().zip(b).all(|(x, y)| x <= y)
}

pub(crate) fn vcmp(a: &[BigUint], b: &[BigUint]) -> Option<Ordering> {
    match (vle(a, b), vle(b, a)) {
        (true, true) => Some(Ordering::Equal),
        (true, false) => Some(Ordering::Less),
        (false, true) => Some(Ordering::Greater),
        (false, false) => None,
    }
}

pub(crate) fn vlt(a: &[BigUint], b: &[BigUint]) -> bool {
    vcmp(a, b) == Some(Ordering::Less)
}

pub(crate) fn vjoin(a: &[BigUint], b: &[BigUint]) -> Vect {
    a.iter().zip(b).map(|(x, y)| x.max(y).clone()).collect()
}

pub(crate) fn vmeet(a: &[BigUint], b: &[BigUint]) -> Vect {
    a.iter().zip(b).map(|(x, y)| x.min(y).clone()).collect()
}

pub(crate) fn vproject(a: &[BigUint], mask: &[bool]) -> Vect {
    a.iter()
        .zip(mask)
        .map(|(x, m)| if *m { x.clone() } else { BigUint::ZERO })
        .collect()
}

/// A full binary tree shape whose leaves are the cells, left to right.
#[derive(Clone, Debug)]
pub(crate) enum Layout {
    Leaf,
    Node(Box<Layout>, Box<Layout>),
}

impl Layout {
    pub fn leaves(&self) -> usize {
        match self {
            Layout::Leaf => 1,
            Layout::Node(l, r) => l.leaves() + r.leaves(),
        }
    }
    pub fn depth(&self) -> usize {
        match self {
            Layout::Leaf => 0,
            Layout::Node(l, r) => 1 + l.depth().max(r.depth()),
        }
    }
    pub fn uniform(depth: u32) -> Layout {
        if depth == 0 {
            Layout::Leaf
        } else {
            Layout::Node(
                Box::new(Layout::uniform(depth - 1)),
                Box::new(Layout::uniform(depth - 1)),
            )
        }
    }
    /// A spine of `n` internal nodes; `turns` bit k picks the side the
    /// spine continues on at level k (0 = right).
    pub fn spine(n: usize, turns: u64) -> Layout {
        let mut t = Layout::Leaf;
        for k in (0..n).rev() {
            t = if turns >> (k % 64) & 1 == 0 {
                Layout::Node(Box::new(Layout::Leaf), Box::new(t))
            } else {
                Layout::Node(Box::new(t), Box::new(Layout::Leaf))
            };
        }
        t
    }
}

thread_local! {
    /// The current world's layout; `None` is the uniform halving layout.
    static LAYOUT: std::cell::RefCell<Option<Layout>> = const { std::cell::RefCell::new(None) };
}

pub(crate) fn set_layout(layout: Option<Layout>) {
    LAYOUT.with(|l| *l.borrow_mut() = layout);
}

fn tree_by_layout(layout: &Layout, cells: &[BigUint]) -> tree::Version {
    match layout {
        Layout::Leaf => {
            assert_eq!(cells.len(), 1);
            tree::Version::leaf(cells[0].clone())
        }
        Layout::Node(l, r) => {
            let k = l.leaves();
            tree::Version::node(
                0u64,
                tree_by_layout(l, &cells[..k]),
                tree_by_layout(r, &cells[k..]),
            )
        }
    }
}

fn party_by_layout(layout: &Layout, mask: &[bool]) -> tree::Party {
    match layout {
        Layout::Leaf => tree::Party::Leaf(mask[0]),
        Layout::Node(l, r) => {
            let k = l.leaves();
            tree::Party::node(
                party_by_layout(l, &mask[..k]),
                party_by_layout(r, &mask[k..]),
            )
        }
    }
}

/// The normal-form event tree of a grid function (cells in left-to-right
/// order), under the current layout.
pub(crate) fn tree_of(cells: &[BigUint]) -> tree::Version {
    if let Some(t) = LAYOUT.with(|l| l.borrow().as_ref().map(|lay| tree_by_layout(lay, cells))) {
        return t;
    }
    if cells.len() == 1 {
        return tree::Version::leaf(cells[0].clone());
    }
    let half = cells.len() / 2;
    tree::Version::node(0u64, tree_of(&cells[..half]), tree_of(&cells[half..]))
}

pub(crate) fn version_of(cells: &[BigUint]) -> Version {
    from_oracle_version(&tree_of(cells))
}

pub(crate) fn party_tree_of(mask: &[bool]) -> tree::Party {
    if let Some(t) = LAYOUT.with(|l| l.borrow().as_ref().map(|lay| party_by_layout(lay, mask))) {
        return t;
    }
    if mask.len() == 1 {
        return tree::Party::Leaf(mask[0]);
    }
    let half = mask.len() / 2;
    tree::Party::node(party_tree_of(&mask[..half]), party_tree_of(&mask[half..]))
}

pub(crate) fn party_of(mask: &[bool]) -> Party {
    from_oracle_party(&party_tree_of(mask))
}

/// The placement verdict from two vector comparisons.
pub(crate) fn oracle_place(lo: &[BigUint], hi: &[BigUint], v: &[BigUint]) -> Placement {
    let l = vcmp(v, lo);
    let h = vcmp(v, hi);
    match l {
        Some(Ordering::Less) => Placement::Before,
        Some(Ordering::Equal) => match h {
            Some(Ordering::Equal) => Placement::At(Endpoint::Both),
            _ => Placement::At(Endpoint::Start),
        },
        Some(Ordering::Greater) => match h {
            Some(Ordering::Less) => Placement::Between,
            Some(Ordering::Equal) => Placement::At(Endpoint::End),
            Some(Ordering::Greater) => Placement::After,
            None => Placement::Concurrent(Endpoint::End),
        },
        None => match h {
            None => Placement::Concurrent(Endpoint::Both),
            _ => Placement::Concurrent(Endpoint::Start),
        },
    }
}

/// Dominance straight from the documented definitions.
pub(crate) fn oracle_dominance(lo: &[BigUint], hi: &[BigUint], v: &[BigUint]) -> Dominance {
    if vle(hi, v) {
        Dominance::After
    } else if vle(lo, v) {
        Dominance::Between
    } else {
        Dominance::Before
    }
}

/// Precedence straight from the documented definitions.
pub(crate) fn oracle_precedence(lo: &[BigUint], hi: &[BigUint], v: &[BigUint]) -> Precedence {
    if vle(v, lo) {
        Precedence::Before
    } else if vle(v, hi) {
        Precedence::Between
    } else {
        Precedence::After
    }
}

// ───────────────────────────── worlds ─────────────────────────────

/// A value alphabet, sorted ascending and distinct.
fn alphabet_of(kind: u8, wexp: u32) -> Vec<BigUint> {
    let w = BigUint::from(1u8) << wexp;
    let one = BigUint::from(1u8);
    let mut a: Vec<BigUint> = match kind % 7 {
        0 => (0u32..3).map(BigUint::from).collect(),
        1 => (0u32..4).map(BigUint::from).collect(),
        2 => vec![&w + 0u8, &w + 1u8, &w + 2u8],
        3 => vec![BigUint::ZERO, one.clone(), w.clone(), &w + 1u8],
        4 => vec![BigUint::ZERO, w.clone(), &w * 2u8],
        5 => vec![&w - 1u8, w.clone(), &w + 1u8],
        _ => vec![BigUint::ZERO, one.clone(), &w - 1u8, &w * &w],
    };
    a.sort();
    a.dedup();
    a
}

/// The generated input: an alphabet, a grid depth, and a pool of index
/// vectors.
#[derive(Clone, Debug)]
pub(crate) struct World {
    pub kind: u8,
    pub wexp: u32,
    pub depth: u32,
    pub pool: Vec<Vec<usize>>,
    pub layout: Option<Layout>,
    /// Organic worlds: the sorted value set and the vectors themselves.
    pub organic: Option<(Vec<BigUint>, Vec<Vect>)>,
}

/// The shape of an oracle event tree.
fn shape_of(t: &tree::Version) -> Layout {
    match t {
        tree::Version::Leaf(_) => Layout::Leaf,
        tree::Version::Node(_, l, r) => Layout::Node(Box::new(shape_of(l)), Box::new(shape_of(r))),
    }
}

/// The shape of an oracle id tree.
fn party_shape(t: &tree::Party) -> Layout {
    match t {
        tree::Party::Leaf(_) => Layout::Leaf,
        tree::Party::Node(l, r) => Layout::Node(Box::new(party_shape(l)), Box::new(party_shape(r))),
    }
}

/// The common refinement of two shapes.
fn overlay(a: &Layout, b: &Layout) -> Layout {
    match (a, b) {
        (Layout::Leaf, Layout::Leaf) => Layout::Leaf,
        (Layout::Node(l, r), Layout::Leaf) | (Layout::Leaf, Layout::Node(l, r)) => Layout::Node(
            Box::new(overlay(l, &Layout::Leaf)),
            Box::new(overlay(r, &Layout::Leaf)),
        ),
        (Layout::Node(al, ar), Layout::Node(bl, br)) => {
            Layout::Node(Box::new(overlay(al, bl)), Box::new(overlay(ar, br)))
        }
    }
}

/// Heights of `t` on every leaf of a refining layout.
fn heights(t: &tree::Version, layout: &Layout, base: &BigUint, out: &mut Vect) {
    match (layout, t) {
        (Layout::Leaf, tree::Version::Leaf(n)) => out.push(base + n),
        (Layout::Leaf, tree::Version::Node(..)) => panic!("layout does not refine the tree"),
        (Layout::Node(l, r), tree::Version::Leaf(n)) => {
            heights(t, l, base, out);
            heights(t, r, base, out);
            let _ = n;
        }
        (Layout::Node(l, r), tree::Version::Node(n, tl, tr)) => {
            let b = base + n;
            heights(tl, l, &b, out);
            heights(tr, r, &b, out);
        }
    }
}

/// Worlds from organic fork/tick/join/sync traces: the population's versions
/// on the overlay of every version and party shape.
pub(crate) fn arb_organic_world() -> impl Strategy<Value = World> {
    crate::testing::optrace::world_strategy().prop_map(|ops| {
        let clocks = crate::testing::optrace::run(&ops);
        let trees: Vec<tree::Version> = crate::testing::optrace::versions(&clocks);
        let mut layout = Layout::Leaf;
        for t in &trees {
            layout = overlay(&layout, &shape_of(t));
        }
        for c in &clocks {
            layout = overlay(&layout, &party_shape(c.trees().0));
        }
        let mut vects: Vec<Vect> = trees
            .iter()
            .map(|t| {
                let mut out = Vec::new();
                heights(t, &layout, &BigUint::ZERO, &mut out);
                out
            })
            .collect();
        for (t, v) in trees.iter().zip(&vects) {
            assert_eq!(
                &tree_by_layout(&layout, v),
                t,
                "the overlay round-trips every organic version"
            );
        }
        vects.push(vec![BigUint::ZERO; layout.leaves()]);
        let mut values: Vec<BigUint> = vects.iter().flatten().cloned().collect();
        values.sort();
        values.dedup();
        World {
            kind: 0,
            wexp: 0,
            depth: layout.depth() as u32,
            pool: Vec::new(),
            layout: Some(layout),
            organic: Some((values, vects)),
        }
    })
}

/// A random full binary shape: spines (each turn chosen), or a recursive
/// random shape.
pub(crate) fn arb_layout() -> impl Strategy<Value = Layout> {
    let rec = Just(Layout::Leaf).prop_recursive(10, 24, 2, |inner| {
        (inner.clone(), inner).prop_map(|(l, r)| Layout::Node(Box::new(l), Box::new(r)))
    });
    prop_oneof![
        (1usize..=40, any::<u64>()).prop_map(|(n, t)| Layout::spine(n, t)),
        (1usize..=40).prop_map(|n| Layout::spine(n, 0)),
        rec,
    ]
}

/// Worlds over a random layout: every version shares the layout's leaves.
pub(crate) fn arb_shaped_world(
    pool_len: std::ops::RangeInclusive<usize>,
) -> impl Strategy<Value = World> {
    (
        0u8..7,
        prop::sample::select(vec![31u32, 64, 65, 128, 1000]),
        arb_layout(),
    )
        .prop_flat_map(move |(kind, wexp, layout)| {
            let cells = layout.leaves();
            let alen = alphabet_of(kind, wexp).len();
            (
                Just(kind),
                Just(wexp),
                Just(layout),
                proptest::collection::vec(
                    proptest::collection::vec(0..alen, cells),
                    pool_len.clone(),
                ),
            )
        })
        .prop_map(|(kind, wexp, layout, pool)| World {
            kind,
            wexp,
            depth: layout.depth() as u32,
            pool,
            layout: Some(layout),
            organic: None,
        })
}

pub(crate) fn arb_world(
    max_depth: u32,
    pool_len: std::ops::RangeInclusive<usize>,
) -> impl Strategy<Value = World> {
    (
        0u8..7,
        prop::sample::select(vec![31u32, 32, 63, 64, 65, 127, 128, 200, 1000]),
        0..=max_depth,
    )
        .prop_flat_map(move |(kind, wexp, depth)| {
            let cells = 1usize << depth;
            let alen = alphabet_of(kind, wexp).len();
            (
                Just(kind),
                Just(wexp),
                Just(depth),
                proptest::collection::vec(
                    proptest::collection::vec(0..alen, cells),
                    pool_len.clone(),
                ),
            )
        })
        .prop_map(|(kind, wexp, depth, pool)| World {
            kind,
            wexp,
            depth,
            pool,
            layout: None,
            organic: None,
        })
}

impl World {
    pub fn alphabet(&self) -> Vec<BigUint> {
        set_layout(self.layout.clone());
        if let Some((values, _)) = &self.organic {
            return values.clone();
        }
        alphabet_of(self.kind, self.wexp)
    }

    pub fn vects(&self) -> Vec<Vect> {
        if let Some((_, v)) = &self.organic {
            return v.clone();
        }
        let a = self.alphabet();
        self.pool
            .iter()
            .map(|ix| ix.iter().map(|&i| a[i].clone()).collect())
            .collect()
    }
}

/// Extend a pool with pairwise joins and meets and single-cell bumps, so that
/// ordered, equal, and near-equal relations are frequent.
pub(crate) fn extended(base: &[Vect], alphabet: &[BigUint]) -> Vec<Vect> {
    let mut out: Vec<Vect> = base.to_vec();
    let n = base.len().min(4);
    for i in 0..n {
        for j in (i + 1)..n {
            out.push(vjoin(&base[i], &base[j]));
            out.push(vmeet(&base[i], &base[j]));
        }
    }
    // Bump one cell of the first vector up and down by one alphabet step.
    if let Some(first) = base.first() {
        for c in 0..first.len().min(4) {
            let pos = alphabet.iter().position(|x| *x == first[c]).unwrap();
            if pos + 1 < alphabet.len() {
                let mut up = first.clone();
                up[c] = alphabet[pos + 1].clone();
                out.push(up);
            }
            if pos > 0 {
                let mut down = first.clone();
                down[c] = alphabet[pos - 1].clone();
                out.push(down);
            }
        }
    }
    let mut seen = HashSet::new();
    out.retain(|v| seen.insert(v.clone()));
    out
}

/// Ordered pairs drawn from the pool: hulls, points, and incidentally ordered
/// raw pairs.
pub(crate) fn span_pairs(pool: &[Vect]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    for i in 0..pool.len() {
        out.push((i, i));
        for j in 0..pool.len() {
            if i != j && vle(&pool[i], &pool[j]) {
                out.push((i, j));
            }
        }
    }
    out
}

// ───────────────────────────── query clauses ─────────────────────────────

#[derive(Clone, Copy, Debug)]
pub(crate) enum NClause {
    After(usize),
    Before(usize),
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum DClause {
    N(NClause),
    Since(usize),
    NotBefore(usize),
    StrictlyAfter(usize),
    AfterOrConc(usize),
    Delta(usize, usize),
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum UClause {
    N(NClause),
    Until(usize),
    NotAfter(usize),
    StrictlyBefore(usize),
    BeforeOrConc(usize),
    Toward(usize, usize),
}

pub(crate) fn arb_n() -> impl Strategy<Value = NClause> {
    prop_oneof![
        (0usize..64).prop_map(NClause::After),
        (0usize..64).prop_map(NClause::Before),
    ]
}

pub(crate) fn arb_d() -> impl Strategy<Value = DClause> {
    prop_oneof![
        2 => arb_n().prop_map(DClause::N),
        1 => (0usize..64).prop_map(DClause::Since),
        1 => (0usize..64).prop_map(DClause::NotBefore),
        1 => (0usize..64).prop_map(DClause::StrictlyAfter),
        1 => (0usize..64).prop_map(DClause::AfterOrConc),
        1 => (0usize..64, 0usize..64).prop_map(|(a, b)| DClause::Delta(a, b)),
    ]
}

pub(crate) fn arb_u() -> impl Strategy<Value = UClause> {
    prop_oneof![
        2 => arb_n().prop_map(UClause::N),
        1 => (0usize..64).prop_map(UClause::Until),
        1 => (0usize..64).prop_map(UClause::NotAfter),
        1 => (0usize..64).prop_map(UClause::StrictlyBefore),
        1 => (0usize..64).prop_map(UClause::BeforeOrConc),
        1 => (0usize..64, 0usize..64).prop_map(|(a, b)| UClause::Toward(a, b)),
    ]
}

pub(crate) fn arb_nclauses() -> impl Strategy<Value = Vec<NClause>> {
    proptest::collection::vec(arb_n(), 0..=5)
}

impl NClause {
    pub fn admits(&self, v: &[BigUint], vs: &[Vect]) -> bool {
        let at = |i: usize| &vs[i % vs.len()];
        match *self {
            NClause::After(i) => vle(at(i), v),
            NClause::Before(i) => vle(v, at(i)),
        }
    }
    pub fn bounds(&self, vs: &[Vect]) -> Vec<Vect> {
        let at = |i: usize| vs[i % vs.len()].clone();
        match *self {
            NClause::After(i) | NClause::Before(i) => vec![at(i)],
        }
    }
    /// The atom this clause denotes.
    pub fn item<'a, P: Polarity>(&self, ps: &'a [Version]) -> Item<'a, P> {
        let at = |i: usize| &ps[i % ps.len()];
        match *self {
            NClause::After(i) => Item::Floor(causally::after(at(i))),
            NClause::Before(i) => Item::Ceiling(causally::before(at(i))),
        }
    }
}

/// One conjunct: an atom or a polar query.
pub(crate) enum Item<'a, P: Polarity> {
    Floor(causally::Floor<'a>),
    Ceiling(causally::Ceiling<'a>),
    Polar(Query<'a, P>),
}

impl DClause {
    pub fn admits(&self, v: &[BigUint], vs: &[Vect]) -> bool {
        let at = |i: usize| &vs[i % vs.len()];
        match *self {
            DClause::N(n) => n.admits(v, vs),
            DClause::Since(i) | DClause::NotBefore(i) => !vle(v, at(i)),
            DClause::StrictlyAfter(i) => vlt(at(i), v),
            DClause::AfterOrConc(i) => !vlt(v, at(i)),
            DClause::Delta(i, j) => !vle(v, at(i)) && vle(v, at(j)),
        }
    }
    pub fn bounds(&self, vs: &[Vect]) -> Vec<Vect> {
        let at = |i: usize| vs[i % vs.len()].clone();
        match *self {
            DClause::N(n) => n.bounds(vs),
            DClause::Since(i)
            | DClause::NotBefore(i)
            | DClause::StrictlyAfter(i)
            | DClause::AfterOrConc(i) => vec![at(i)],
            DClause::Delta(i, j) => vec![at(i), at(j)],
        }
    }
    pub fn item<'a>(&self, ps: &'a [Version]) -> Item<'a, Down> {
        let at = |i: usize| &ps[i % ps.len()];
        Item::Polar(match *self {
            DClause::N(n) => return n.item(ps),
            DClause::Since(i) => causally::since(at(i)),
            DClause::NotBefore(i) => !causally::before(at(i)),
            DClause::StrictlyAfter(i) => causally::strictly_after(at(i)),
            DClause::AfterOrConc(i) => causally::after(at(i)).or_concurrent(),
            DClause::Delta(i, j) => causally::delta(at(i), at(j)),
        })
    }
}

impl UClause {
    pub fn admits(&self, v: &[BigUint], vs: &[Vect]) -> bool {
        let at = |i: usize| &vs[i % vs.len()];
        match *self {
            UClause::N(n) => n.admits(v, vs),
            UClause::Until(i) | UClause::NotAfter(i) => !vle(at(i), v),
            UClause::StrictlyBefore(i) => vlt(v, at(i)),
            UClause::BeforeOrConc(i) => !vlt(at(i), v),
            UClause::Toward(i, j) => vle(at(i), v) && !vle(at(j), v),
        }
    }
    pub fn bounds(&self, vs: &[Vect]) -> Vec<Vect> {
        let at = |i: usize| vs[i % vs.len()].clone();
        match *self {
            UClause::N(n) => n.bounds(vs),
            UClause::Until(i)
            | UClause::NotAfter(i)
            | UClause::StrictlyBefore(i)
            | UClause::BeforeOrConc(i) => vec![at(i)],
            UClause::Toward(i, j) => vec![at(i), at(j)],
        }
    }
    pub fn item<'a>(&self, ps: &'a [Version]) -> Item<'a, Up> {
        let at = |i: usize| &ps[i % ps.len()];
        Item::Polar(match *self {
            UClause::N(n) => return n.item(ps),
            UClause::Until(i) => causally::until(at(i)),
            UClause::NotAfter(i) => !causally::after(at(i)),
            UClause::StrictlyBefore(i) => causally::strictly_before(at(i)),
            UClause::BeforeOrConc(i) => causally::before(at(i)).or_concurrent(),
            UClause::Toward(i, j) => causally::toward(at(i), at(j)),
        })
    }
}

/// Conjoin a neutral clause list, folding each atom in on the side `shape`
/// picks, starting from `all()`.
pub(crate) fn conjoin_neutral<'a>(items: Vec<Item<'a, Neutral>>, shape: u64) -> Query<'a, Neutral> {
    let mut acc: Query<'a, Neutral> = causally::all();
    for (k, it) in items.into_iter().enumerate() {
        let left = shape >> (k % 64) & 1 == 0;
        acc = match (it, left) {
            (Item::Floor(f), true) => acc & f,
            (Item::Floor(f), false) => f & acc,
            (Item::Ceiling(c), true) => acc & c,
            (Item::Ceiling(c), false) => c & acc,
            (Item::Polar(q), true) => acc & q,
            (Item::Polar(q), false) => q & acc,
        };
    }
    acc
}

macro_rules! conjoin_polar {
    ($name:ident, $P:ty) => {
        /// Conjoin a polar clause list: the first polar item (in rotated
        /// order) seeds the fold, every other item joins on the side `shape`
        /// picks. `None` when no polar item exists.
        pub(crate) fn $name<'a>(items: Vec<Item<'a, $P>>, shape: u64) -> Option<Query<'a, $P>> {
            let n = items.len();
            let seed_at = (0..n)
                .map(|k| (k + (shape as usize % n.max(1))) % n)
                .find(|&k| matches!(items[k], Item::Polar(_)))?;
            let mut items: Vec<Option<Item<'a, $P>>> = items.into_iter().map(Some).collect();
            let Some(Item::Polar(mut acc)) = items[seed_at].take() else {
                unreachable!()
            };
            for (k, it) in items.into_iter().enumerate() {
                let Some(it) = it else { continue };
                let left = shape >> (k % 64) & 1 == 0;
                acc = match (it, left) {
                    (Item::Floor(f), true) => acc & f,
                    (Item::Floor(f), false) => f & acc,
                    (Item::Ceiling(c), true) => acc & c,
                    (Item::Ceiling(c), false) => c & acc,
                    (Item::Polar(q), true) => acc & q,
                    (Item::Polar(q), false) => q & acc,
                };
            }
            Some(acc)
        }
    };
}
conjoin_polar!(conjoin_down, Down);
conjoin_polar!(conjoin_up, Up);

// ───────────────────────────── coverage census ─────────────────────────────

/// The sublattice generated by `gens` under pointwise min and max, or `None`
/// when it exceeds `cap` elements.
pub(crate) fn sublattice(gens: &[Vect], cap: usize) -> Option<Vec<Vect>> {
    let mut set: HashSet<Vect> = HashSet::new();
    let mut all: Vec<Vect> = Vec::new();
    let mut work: Vec<Vect> = Vec::new();
    for g in gens {
        if set.insert(g.clone()) {
            all.push(g.clone());
            work.push(g.clone());
        }
    }
    while let Some(x) = work.pop() {
        let n = all.len();
        for k in 0..n {
            let y = all[k].clone();
            for z in [vjoin(&x, &y), vmeet(&x, &y)] {
                if set.insert(z.clone()) {
                    if set.len() > cap {
                        return None;
                    }
                    all.push(z.clone());
                    work.push(z);
                }
            }
        }
    }
    Some(all)
}

/// Exact coverage by census over the sublattice generated by the segment's
/// endpoints and every bound clamped into the segment.
///
/// Exactness: `Full` is decided by the two endpoints (a bound excluding any
/// covered version excludes `lo` or `hi`), and a nonempty admitted set always
/// contains the clamp of the polarity's deciding meet or join of bounds, which
/// lies in the generated sublattice because clamping is a lattice
/// homomorphism of a distributive lattice onto the segment.
pub(crate) fn census(
    lo: &Vect,
    hi: &Vect,
    bounds: &[Vect],
    admits: impl Fn(&[BigUint]) -> bool,
    cap: usize,
) -> Option<Coverage> {
    let mut gens = vec![lo.clone(), hi.clone()];
    for b in bounds {
        gens.push(vmeet(&vjoin(lo, b), hi));
    }
    let members = sublattice(&gens, cap)?;
    let mut admitted = 0usize;
    for m in &members {
        debug_assert!(vle(lo, m) && vle(m, hi));
        if admits(m) {
            admitted += 1;
        }
    }
    Some(if admitted == members.len() {
        Coverage::Full
    } else if admitted == 0 {
        Coverage::Empty
    } else {
        Coverage::Partial
    })
}

/// Coverage by the witness argument alone: Full iff both endpoints admitted;
/// Empty iff no clamped extreme bound is admitted.
pub(crate) fn witness_coverage(
    lo: &Vect,
    hi: &Vect,
    bounds: &[Vect],
    admits: impl Fn(&[BigUint]) -> bool,
) -> Coverage {
    if admits(lo) && admits(hi) {
        return Coverage::Full;
    }
    // Any admitted member: try the endpoints, every clamped bound, and every
    // meet and join of two clamped bounds. (Heuristic witness set; census
    // is the exact oracle.)
    let mut cands = vec![lo.clone(), hi.clone()];
    let clamped: Vec<Vect> = bounds.iter().map(|b| vmeet(&vjoin(lo, b), hi)).collect();
    cands.extend(clamped.iter().cloned());
    if admits(lo) || admits(hi) || cands.iter().any(|c| admits(c)) {
        Coverage::Partial
    } else {
        Coverage::Empty
    }
}

// ───────────────────────────── the properties ─────────────────────────────

/// Check every span verdict for one span against the oracle.
fn check_span_verdicts(
    span: &Span<'_>,
    lo: &Vect,
    hi: &Vect,
    probes: &[(Vect, Version)],
    label: &str,
) -> Result<(), TestCaseError> {
    for (pv, p) in probes {
        stat(format!("verdicts {label} {:?}", oracle_place(lo, hi, pv)));
        prop_assert_eq!(
            span.place(p),
            oracle_place(lo, hi, pv),
            "{} place lo={:?} hi={:?} v={:?}",
            label,
            lo,
            hi,
            pv
        );
        prop_assert_eq!(
            span.dominance(p),
            oracle_dominance(lo, hi, pv),
            "{} dominance lo={:?} hi={:?} v={:?}",
            label,
            lo,
            hi,
            pv
        );
        prop_assert_eq!(
            span.precedence(p),
            oracle_precedence(lo, hi, pv),
            "{} precedence lo={:?} hi={:?} v={:?}",
            label,
            lo,
            hi,
            pv
        );
        let inside = vle(lo, pv) && vle(pv, hi);
        prop_assert_eq!(
            span.contains(p),
            inside,
            "{} contains lo={:?} hi={:?} v={:?}",
            label,
            lo,
            hi,
            pv
        );
        prop_assert_eq!(
            span.contains(p.clone()),
            inside,
            "{} contains(owned)",
            label
        );
        prop_assert_eq!(
            span.contains(Span::at(p)),
            inside,
            "{} contains(point)",
            label
        );
    }
    Ok(())
}

/// Every span verdict equals the vector oracle on grid worlds, for spans
/// built with distinct buffers, a shared buffer, decoded storage, and
/// value-equal separate buffers.
fn body_span_verdicts_match_vectors(world: World) -> Result<(), TestCaseError> {
    stat(format!(
        "verdicts world kind={} depth={}",
        world.kind, world.depth
    ));
    let alphabet = world.alphabet();
    let pool = extended(&world.vects(), &alphabet);
    let versions: Vec<Version> = pool.iter().map(|v| version_of(v)).collect();
    let probes: Vec<(Vect, Version)> = pool.iter().cloned().zip(versions.iter().cloned()).collect();
    for (i, j) in span_pairs(&pool) {
        let (lo, hi) = (&pool[i], &pool[j]);
        let mut more = probes.clone();
        for x in pool.iter().take(4) {
            let mid = vmeet(&vjoin(lo, x), hi);
            more.push((mid.clone(), version_of(&mid)));
        }
        let s = Span::new(&versions[i], &versions[j]).expect("ordered");
        check_span_verdicts(&s, lo, hi, &more, "new")?;
        // Separately built buffers.
        let lo2 = version_of(lo);
        let hi2 = version_of(hi);
        let s2 = Span::new(&lo2, &hi2).expect("ordered");
        check_span_verdicts(&s2, lo, hi, &more, "fresh")?;
        // Decoded storage.
        let dec = Span::decode(&s.encode()[..]).expect("roundtrip");
        prop_assert_eq!(&dec, &s);
        check_span_verdicts(&dec, lo, hi, &more, "decoded")?;
        if i == j {
            let at = Span::at(&versions[i]);
            check_span_verdicts(&at, lo, hi, &more, "at")?;
        }
        // Span containment against every other ordered pair.
        for (k, l) in span_pairs(&pool) {
            let t = Span::new(&versions[k], &versions[l]).unwrap();
            let want = vle(lo, &pool[k]) && vle(&pool[l], hi);
            prop_assert_eq!(s.contains(&t), want, "span contains span");
            prop_assert_eq!(s2.contains(t.reborrow()), want, "fresh span contains span");
        }
    }
    Ok(())
}

/// The four span operators and their `_all` forms equal the vector lattice
/// formulas, and intersection is the set intersection over the generated
/// sublattice.
fn body_span_algebra_matches_vectors(world: World, picks: Vec<usize>) -> Result<(), TestCaseError> {
    let alphabet = world.alphabet();
    let pool = extended(&world.vects(), &alphabet);
    let versions: Vec<Version> = pool.iter().map(|v| version_of(v)).collect();
    let pairs = span_pairs(&pool);
    let spans: Vec<Span<'_>> = pairs
        .iter()
        .map(|&(i, j)| Span::new(&versions[i], &versions[j]).unwrap())
        .collect();
    let ends: Vec<(Vect, Vect)> = pairs
        .iter()
        .map(|&(i, j)| (pool[i].clone(), pool[j].clone()))
        .collect();
    let n = spans.len();
    for a in 0..n.min(8) {
        for b in 0..n.min(8) {
            let (s, t) = (&spans[a], &spans[b]);
            let ((sl, sh), (tl, th)) = (&ends[a], &ends[b]);
            let u = s + t;
            prop_assert_eq!(u.lo(), &version_of(&vmeet(sl, tl)));
            prop_assert_eq!(u.hi(), &version_of(&vjoin(sh, th)));
            let j = s | t;
            prop_assert_eq!(j.lo(), &version_of(&vjoin(sl, tl)));
            prop_assert_eq!(j.hi(), &version_of(&vjoin(sh, th)));
            let m = s & t;
            prop_assert_eq!(m.lo(), &version_of(&vmeet(sl, tl)));
            prop_assert_eq!(m.hi(), &version_of(&vmeet(sh, th)));
            let il = vjoin(sl, tl);
            let ih = vmeet(sh, th);
            let want = vle(&il, &ih).then(|| (version_of(&il), version_of(&ih)));
            let got = (s * t).map(|x| (x.lo().clone(), x.hi().clone()));
            prop_assert_eq!(got, want);
        }
    }
    // The `_all` forms over a picked family.
    if n > 0 {
        let fam: Vec<usize> = picks.iter().map(|p| p % n).collect();
        let r = 0;
        let items: Vec<&Span<'_>> = fam.iter().map(|&k| &spans[k]).collect();
        let (mut ul, mut uh) = ends[r].clone();
        let (mut jl, mut jh) = ends[r].clone();
        let (mut ml, mut mh) = ends[r].clone();
        let (mut il, mut ih) = ends[r].clone();
        for &k in &fam {
            let (l, h) = &ends[k];
            ul = vmeet(&ul, l);
            uh = vjoin(&uh, h);
            jl = vjoin(&jl, l);
            jh = vjoin(&jh, h);
            ml = vmeet(&ml, l);
            mh = vmeet(&mh, h);
            il = vjoin(&il, l);
            ih = vmeet(&ih, h);
        }
        let u = spans[r].union_all(items.iter().copied());
        prop_assert_eq!(
            (u.lo().clone(), u.hi().clone()),
            (version_of(&ul), version_of(&uh)),
            "union_all"
        );
        let j = spans[r].join_all(items.iter().copied());
        prop_assert_eq!(
            (j.lo().clone(), j.hi().clone()),
            (version_of(&jl), version_of(&jh)),
            "join_all"
        );
        let m = spans[r].meet_all(items.iter().copied());
        prop_assert_eq!(
            (m.lo().clone(), m.hi().clone()),
            (version_of(&ml), version_of(&mh)),
            "meet_all"
        );
        let i = spans[r]
            .intersect_all(items.iter().copied())
            .map(|x| (x.lo().clone(), x.hi().clone()));
        let want = vle(&il, &ih).then(|| (version_of(&il), version_of(&ih)));
        prop_assert_eq!(i, want, "intersect_all");
        // Point families through the version-item folds.
        let vitems: Vec<&Version> = fam.iter().map(|&k| &versions[pairs[k].0]).collect();
        let hull = versions[0].span_all(vitems.iter().copied());
        let mut hl = pool[0].clone();
        let mut hh = pool[0].clone();
        for &k in &fam {
            hl = vmeet(&hl, &pool[pairs[k].0]);
            hh = vjoin(&hh, &pool[pairs[k].0]);
        }
        prop_assert_eq!(
            (hull.lo().clone(), hull.hi().clone()),
            (version_of(&hl), version_of(&hh)),
            "span_all"
        );
        // Sum, Product, and collect spellings.
        let summed: Option<Span<'static>> = items.iter().copied().sum();
        let want_sum = (!fam.is_empty()).then(|| {
            let (mut l, mut h) = ends[fam[0]].clone();
            for &k in &fam[1..] {
                l = vmeet(&l, &ends[k].0);
                h = vjoin(&h, &ends[k].1);
            }
            (version_of(&l), version_of(&h))
        });
        prop_assert_eq!(
            summed.map(|x| (x.lo().clone(), x.hi().clone())),
            want_sum.clone(),
            "sum"
        );
        let prod: Option<Span<'static>> = items.iter().copied().product();
        let want_prod = (!fam.is_empty())
            .then(|| {
                let (mut l, mut h) = ends[fam[0]].clone();
                for &k in &fam[1..] {
                    l = vjoin(&l, &ends[k].0);
                    h = vmeet(&h, &ends[k].1);
                }
                (l, h)
            })
            .and_then(|(l, h)| vle(&l, &h).then(|| (version_of(&l), version_of(&h))));
        prop_assert_eq!(
            prod.map(|x| (x.lo().clone(), x.hi().clone())),
            want_prod,
            "product"
        );
        let collected: Option<Span<'static>> = vitems.iter().copied().collect();
        let want_col = (!fam.is_empty()).then(|| {
            let mut l = pool[pairs[fam[0]].0].clone();
            let mut h = l.clone();
            for &k in &fam[1..] {
                l = vmeet(&l, &pool[pairs[k].0]);
                h = vjoin(&h, &pool[pairs[k].0]);
            }
            (version_of(&l), version_of(&h))
        });
        prop_assert_eq!(
            collected.map(|x| (x.lo().clone(), x.hi().clone())),
            want_col,
            "collect versions"
        );
    }
    // Pairwise hull and the version-left operators.
    for a in 0..pool.len().min(6) {
        for b in 0..pool.len().min(6) {
            let (va, vb) = (&versions[a], &versions[b]);
            let h = va.span(vb);
            prop_assert_eq!(
                (h.lo().clone(), h.hi().clone()),
                (
                    version_of(&vmeet(&pool[a], &pool[b])),
                    version_of(&vjoin(&pool[a], &pool[b]))
                ),
                "hull"
            );
            prop_assert_eq!(va ^ vb, h.clone(), "xor hull");
            if let Some(s) = spans.get(b) {
                let (sl, sh) = &ends[b];
                let u = va + s;
                prop_assert_eq!(
                    (u.lo().clone(), u.hi().clone()),
                    (
                        version_of(&vmeet(&pool[a], sl)),
                        version_of(&vjoin(&pool[a], sh))
                    ),
                    "v + s"
                );
                let j = va | s;
                prop_assert_eq!(
                    (j.lo().clone(), j.hi().clone()),
                    (
                        version_of(&vjoin(&pool[a], sl)),
                        version_of(&vjoin(&pool[a], sh))
                    ),
                    "v | s"
                );
                let m = va & s;
                prop_assert_eq!(
                    (m.lo().clone(), m.hi().clone()),
                    (
                        version_of(&vmeet(&pool[a], sl)),
                        version_of(&vmeet(&pool[a], sh))
                    ),
                    "v & s"
                );
                let mut asg = s.clone();
                asg += va;
                prop_assert_eq!(asg, u, "+=");
            }
        }
    }
    Ok(())
}

/// Query membership and coverage equal the vector predicate and the
/// census, for neutral, down, and up conjunctions of every public form.
fn body_queries_match_vectors(
    world: World,
    nclauses: Vec<NClause>,
    dclauses: Vec<DClause>,
    uclauses: Vec<UClause>,
    shape: u64,
) -> Result<(), TestCaseError> {
    let alphabet = world.alphabet();
    let pool = extended(&world.vects(), &alphabet);
    let versions: Vec<Version> = pool.iter().map(|v| version_of(v)).collect();

    let nq = conjoin_neutral(
        nclauses
            .iter()
            .map(|c| c.item::<Neutral>(&versions))
            .collect(),
        shape,
    );
    // A list of only atoms has no polar seed: append an always-polar clause.
    let mut dclauses = dclauses;
    if dclauses.iter().all(|c| matches!(c, DClause::N(_))) {
        dclauses.push(DClause::Since(shape as usize));
    }
    let mut uclauses = uclauses;
    if uclauses.iter().all(|c| matches!(c, UClause::N(_))) {
        uclauses.push(UClause::Until(shape as usize));
    }
    let dq = conjoin_down(dclauses.iter().map(|c| c.item(&versions)).collect(), shape).unwrap();
    let uq = conjoin_up(
        uclauses.iter().map(|c| c.item(&versions)).collect(),
        shape.rotate_left(17),
    )
    .unwrap();

    let nadm = |v: &[BigUint]| nclauses.iter().all(|c| c.admits(v, &pool));
    let dadm = |v: &[BigUint]| dclauses.iter().all(|c| c.admits(v, &pool));
    let uadm = |v: &[BigUint]| uclauses.iter().all(|c| c.admits(v, &pool));
    let nb: Vec<Vect> = nclauses.iter().flat_map(|c| c.bounds(&pool)).collect();
    let db: Vec<Vect> = dclauses.iter().flat_map(|c| c.bounds(&pool)).collect();
    let ub: Vec<Vect> = uclauses.iter().flat_map(|c| c.bounds(&pool)).collect();

    let mut probes: Vec<Vect> = pool.clone();
    for b in nb.iter().chain(&db).chain(&ub) {
        probes.push(b.clone());
    }
    stat(format!(
        "queries holes down={} up={}",
        format!("{dq:?}").matches('!').count(),
        format!("{uq:?}").matches('!').count()
    ));
    for (pv, p) in probes.iter().map(|v| (v, version_of(v))) {
        stat(format!(
            "queries contains n={} d={} u={}",
            nadm(pv),
            dadm(pv),
            uadm(pv)
        ));
        prop_assert_eq!(
            nq.contains(&p),
            nadm(pv),
            "neutral contains {:?} at {:?}",
            nq,
            pv
        );
        prop_assert_eq!(
            dq.contains(&p),
            dadm(pv),
            "down contains {:?} at {:?}",
            dq,
            pv
        );
        prop_assert_eq!(
            uq.contains(&p),
            uadm(pv),
            "up contains {:?} at {:?}",
            uq,
            pv
        );
    }

    for (i, j) in span_pairs(&pool) {
        let (lo, hi) = (&pool[i], &pool[j]);
        let s = Span::new(&versions[i], &versions[j]).unwrap();
        let fresh_lo = version_of(lo);
        let fresh_hi = version_of(hi);
        let s2 = Span::new(&fresh_lo, &fresh_hi).unwrap();
        macro_rules! cov {
            ($q:expr, $adm:expr, $b:expr, $name:literal) => {
                let got = census(lo, hi, &$b, &$adm, 20_000);
                stat(format!(
                    "queries {} census {:?} point={}",
                    $name,
                    got,
                    i == j
                ));
                if let Some(want) = got {
                    prop_assert_eq!(
                        $q.coverage(s.reborrow()),
                        want,
                        "{} coverage {:?} over [{:?}, {:?}]",
                        $name,
                        $q,
                        lo,
                        hi
                    );
                    prop_assert_eq!(
                        $q.coverage(s2.reborrow()),
                        want,
                        "{} coverage (fresh) {:?} over [{:?}, {:?}]",
                        $name,
                        $q,
                        lo,
                        hi
                    );
                }
            };
        }
        cov!(nq, nadm, nb, "neutral");
        cov!(dq, dadm, db, "down");
        cov!(uq, uadm, ub, "up");
        // Bare atoms' own coverage.
        for k in 0..pool.len().min(4) {
            let b = vec![pool[k].clone()];
            let fl = |v: &[BigUint]| vle(&pool[k], v);
            let ce = |v: &[BigUint]| vle(v, &pool[k]);
            if let Some(want) = census(lo, hi, &b, &fl, 20_000) {
                prop_assert_eq!(
                    causally::after(&versions[k]).coverage(s.reborrow()),
                    want,
                    "floor atom coverage"
                );
            }
            if let Some(want) = census(lo, hi, &b, &ce, 20_000) {
                prop_assert_eq!(
                    causally::before(&versions[k]).coverage(s2.reborrow()),
                    want,
                    "ceiling atom coverage"
                );
            }
            // A version's singleton query.
            let eq = |v: &[BigUint]| v == &pool[k][..];
            if let Some(want) = census(lo, hi, &b, &eq, 20_000) {
                prop_assert_eq!(
                    Query::from(&versions[k]).coverage(s.reborrow()),
                    want,
                    "singleton coverage"
                );
            }
        }
        if i == j {
            let p = Span::at(&versions[i]);
            let w = |b: bool| if b { Coverage::Full } else { Coverage::Empty };
            prop_assert_eq!(nq.coverage(p.reborrow()), w(nadm(lo)));
            prop_assert_eq!(dq.coverage(p.reborrow()), w(dadm(lo)));
            prop_assert_eq!(uq.coverage(p.reborrow()), w(uadm(lo)));
            prop_assert_eq!(dq.coverage(&versions[i]), w(dadm(lo)));
        }
    }
    Ok(())
}

/// A span's query converts to the same membership, and its coverage of a
/// second span is containment (`Full`), disjointness (`Empty`), or
/// overlap (`Partial`) as the span algebra computes them.
fn body_span_query_agree(world: World) -> Result<(), TestCaseError> {
    let alphabet = world.alphabet();
    let pool = extended(&world.vects(), &alphabet);
    let versions: Vec<Version> = pool.iter().map(|v| version_of(v)).collect();
    let pairs = span_pairs(&pool);
    for &(i, j) in pairs.iter().take(12) {
        let s = Span::new(&versions[i], &versions[j]).unwrap();
        let q = Query::from(&s);
        let qo = Query::from(s.clone().into_owned());
        for (pv, p) in pool.iter().zip(&versions) {
            prop_assert_eq!(q.contains(p), s.contains(p));
            prop_assert_eq!(qo.contains(p), s.contains(p));
            prop_assert_eq!(Query::from(p).contains(&versions[i]), pv == &pool[i]);
        }
        for &(k, l) in pairs.iter().take(12) {
            let t = Span::new(&versions[k], &versions[l]).unwrap();
            let want = if s.contains(&t) {
                Coverage::Full
            } else if (&s * &t).is_none() {
                Coverage::Empty
            } else {
                Coverage::Partial
            };
            stat(format!("spanquery {want:?}"));
            prop_assert_eq!(
                q.coverage(t.reborrow()),
                want,
                "segment query coverage s=[{:?},{:?}] t=[{:?},{:?}]",
                pool[i],
                pool[j],
                pool[k],
                pool[l]
            );
        }
    }
    Ok(())
}

/// The projected-span view's verdicts equal the oracle over masked
/// vectors, and materialization equals the masked endpoints.
fn body_own_span_matches_vectors(world: World, mask_bits: u64) -> Result<(), TestCaseError> {
    let alphabet = world.alphabet();
    let pool = extended(&world.vects(), &alphabet);
    let cells = pool[0].len();
    let mut mask: Vec<bool> = (0..cells).map(|c| mask_bits >> (c % 64) & 1 == 1).collect();
    if !mask.iter().any(|m| *m) {
        mask[0] = true;
    }
    let party = party_of(&mask);
    let versions: Vec<Version> = pool.iter().map(|v| version_of(v)).collect();
    for (i, j) in span_pairs(&pool) {
        let s = Span::new(&versions[i], &versions[j]).unwrap();
        let view = s.project(&party);
        let plo = vproject(&pool[i], &mask);
        let phi = vproject(&pool[j], &mask);
        prop_assert_eq!(
            view.to_span(),
            Span::new(version_of(&plo), version_of(&phi)).unwrap()
        );
        for (pv, p) in pool.iter().zip(&versions) {
            stat(format!("own {:?}", oracle_place(&plo, &phi, pv)));
            prop_assert_eq!(view.place(p), oracle_place(&plo, &phi, pv), "own place");
            prop_assert_eq!(
                view.dominance(p),
                oracle_dominance(&plo, &phi, pv),
                "own dominance"
            );
            prop_assert_eq!(
                view.precedence(p),
                oracle_precedence(&plo, &phi, pv),
                "own precedence"
            );
            prop_assert_eq!(
                view.contains(p),
                vle(&plo, pv) && vle(pv, &phi),
                "own contains"
            );
        }
    }
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(
        std::env::var("PROPTEST_CASES").ok().and_then(|s| s.parse().ok()).unwrap_or(64)
    ))]

    /// Every span verdict equals the vector oracle on grid worlds, for spans
    /// built with distinct buffers, a shared buffer, decoded storage, and
    /// value-equal separate buffers.
    #[test]
    fn l5_span_verdicts_match_vectors(world in arb_world(4, 2..=5)) {
        body_span_verdicts_match_vectors(world)?;
    }

    /// The four span operators and their `_all` forms equal the vector lattice
    /// formulas, and intersection is the set intersection over the generated
    /// sublattice.
    #[test]
    fn l5_span_algebra_matches_vectors(world in arb_world(4, 2..=5), picks in proptest::collection::vec(0usize..64, 0..=6)) {
        body_span_algebra_matches_vectors(world, picks)?;
    }

    /// Query membership and coverage equal the vector predicate and the
    /// census, for neutral, down, and up conjunctions of every public form.
    #[test]
    fn l5_queries_match_vectors(
        world in arb_world(3, 2..=4),
        nclauses in arb_nclauses(),
        dclauses in proptest::collection::vec(arb_d(), 1..=5),
        uclauses in proptest::collection::vec(arb_u(), 1..=5),
        shape in any::<u64>(),
    ) {
        body_queries_match_vectors(world, nclauses, dclauses, uclauses, shape)?;
    }

    /// A span's query converts to the same membership, and its coverage of a
    /// second span is containment (`Full`), disjointness (`Empty`), or
    /// overlap (`Partial`) as the span algebra computes them.
    #[test]
    fn l5_span_query_agree(world in arb_world(4, 2..=5)) {
        body_span_query_agree(world)?;
    }

    /// The projected-span view's verdicts equal the oracle over masked
    /// vectors, and materialization equals the masked endpoints.
    #[test]
    fn l5_own_span_matches_vectors(
        world in arb_world(4, 2..=5),
        mask_bits in any::<u64>(),
    ) {
        body_own_span_matches_vectors(world, mask_bits)?;
    }
}

/// Run one property body over `cases` generated inputs, failing loudly.
fn run_stats<S: Strategy>(
    name: &str,
    strat: S,
    body: impl Fn(S::Value) -> Result<(), TestCaseError>,
) {
    use proptest::test_runner::{Config, TestRunner};
    let cases = std::env::var("L5_STATS_CASES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(256);
    let mut runner = TestRunner::new(Config::with_cases(cases));
    runner
        .run(&strat, |v| body(v))
        .unwrap_or_else(|e| panic!("{name}: {e}"));
}

/// Behavioral histograms for every property (run with `--run-ignored`).
#[test]
#[ignore]
fn l5_stats() {
    run_stats(
        "l5_span_verdicts_match_vectors",
        (arb_world(4, 2..=5),),
        |(world,)| body_span_verdicts_match_vectors(world),
    );
    run_stats(
        "l5_span_algebra_matches_vectors",
        (
            arb_world(4, 2..=5),
            proptest::collection::vec(0usize..64, 0..=6),
        ),
        |(world, picks)| body_span_algebra_matches_vectors(world, picks),
    );
    run_stats(
        "l5_queries_match_vectors",
        (
            arb_world(3, 2..=4),
            arb_nclauses(),
            proptest::collection::vec(arb_d(), 1..=5),
            proptest::collection::vec(arb_u(), 1..=5),
            any::<u64>(),
        ),
        |(world, nclauses, dclauses, uclauses, shape)| {
            body_queries_match_vectors(world, nclauses, dclauses, uclauses, shape)
        },
    );
    run_stats("l5_span_query_agree", (arb_world(4, 2..=5),), |(world,)| {
        body_span_query_agree(world)
    });
    run_stats(
        "l5_own_span_matches_vectors",
        (arb_world(4, 2..=5), any::<u64>()),
        |(world, mask_bits)| body_own_span_matches_vectors(world, mask_bits),
    );
    let stats = STATS.lock().unwrap();
    for (k, v) in stats.iter() {
        eprintln!("STAT {k} = {v}");
    }
}

/// The census and witness oracles agree with the full finer universe on tiny
/// grids: every function on twice as many cells with heights bounded by the
/// alphabet's range.
#[test]
fn l5_census_is_exact_against_a_finer_universe() {
    use proptest::test_runner::{Config, TestRunner};
    let mut runner = TestRunner::new(Config::with_cases(200));
    let strat = (
        proptest::collection::vec(proptest::collection::vec(0u32..3, 2), 3..=4),
        proptest::collection::vec(arb_d(), 1..=3),
        proptest::collection::vec(arb_u(), 1..=3),
    );
    runner
        .run(&strat, |(raw, ds, us)| {
            // Coarse vectors on 2 cells, values 0..=2.
            let pool: Vec<Vect> = raw
                .iter()
                .map(|r| r.iter().map(|&x| BigUint::from(x)).collect())
                .collect();
            // Fine universe: 4 cells, values 0..=2; a coarse vector lifts by
            // duplicating each cell.
            let lift =
                |v: &Vect| -> Vect { v.iter().flat_map(|x| [x.clone(), x.clone()]).collect() };
            let fine_pool: Vec<Vect> = pool.iter().map(lift).collect();
            let mut universe = Vec::new();
            for code in 0..81u32 {
                let mut c = code;
                let v: Vect = (0..4)
                    .map(|_| {
                        let x = c % 3;
                        c /= 3;
                        BigUint::from(x)
                    })
                    .collect();
                universe.push(v);
            }
            for (i, j) in span_pairs(&pool) {
                let (lo, hi) = (&fine_pool[i], &fine_pool[j]);
                let members: Vec<&Vect> = universe
                    .iter()
                    .filter(|v| vle(lo, v) && vle(v, hi))
                    .collect();
                let brute = |adm: &dyn Fn(&[BigUint]) -> bool| {
                    let a = members.iter().filter(|v| adm(&v[..])).count();
                    if a == members.len() {
                        Coverage::Full
                    } else if a == 0 {
                        Coverage::Empty
                    } else {
                        Coverage::Partial
                    }
                };
                let dadm = |v: &[BigUint]| ds.iter().all(|c| c.admits(v, &fine_pool));
                let uadm = |v: &[BigUint]| us.iter().all(|c| c.admits(v, &fine_pool));
                let db: Vec<Vect> = ds.iter().flat_map(|c| c.bounds(&fine_pool)).collect();
                let ub: Vec<Vect> = us.iter().flat_map(|c| c.bounds(&fine_pool)).collect();
                prop_assert_eq!(census(lo, hi, &db, &dadm, 100_000).unwrap(), brute(&dadm));
                prop_assert_eq!(census(lo, hi, &ub, &uadm, 100_000).unwrap(), brute(&uadm));
            }
            Ok(())
        })
        .unwrap();
}

// ───────────────────────────── antichain holes ─────────────────────────────

/// Pool extension with pairwise-concurrent single-cell bumps of the first
/// vector, up and down, plus the bottom vector.
fn with_bumps(pool: &[Vect], alphabet: &[BigUint]) -> (Vec<Vect>, Vec<usize>, Vec<usize>) {
    let mut out = pool.to_vec();
    let first = pool[0].clone();
    let mut ups = Vec::new();
    let mut downs = Vec::new();
    for c in 0..first.len() {
        let pos = alphabet.iter().position(|x| *x == first[c]).unwrap();
        if pos + 1 < alphabet.len() {
            let mut up = first.clone();
            up[c] = alphabet[pos + 1].clone();
            ups.push(out.len());
            out.push(up);
        }
        if pos > 0 {
            let mut down = first.clone();
            down[c] = alphabet[pos - 1].clone();
            downs.push(out.len());
            out.push(down);
        }
    }
    out.push(vec![BigUint::ZERO; first.len()]);
    // Hull of all bumps, so spans straddle the antichain.
    if !ups.is_empty() {
        let j = ups
            .iter()
            .fold(first.clone(), |acc, &k| vjoin(&acc, &out[k]));
        out.push(j);
    }
    if !downs.is_empty() {
        let m = downs
            .iter()
            .fold(first.clone(), |acc, &k| vmeet(&acc, &out[k]));
        out.push(m);
    }
    (out, ups, downs)
}

/// Queries whose holes form antichains of pairwise-concurrent bumps agree
/// with the census on every ordered segment.
fn body_antichain_queries(
    world: World,
    hole_picks: Vec<(usize, bool)>,
    bounds: Vec<NClause>,
    shape: u64,
) -> Result<(), TestCaseError> {
    let alphabet = world.alphabet();
    let (pool, ups, downs) = with_bumps(&world.vects(), &alphabet);
    let versions: Vec<Version> = pool.iter().map(|v| version_of(v)).collect();
    // Down: holes at up-bumps (and at down-bumps, mixing), each strict or not.
    let mut dclauses: Vec<DClause> = Vec::new();
    let mut uclauses: Vec<UClause> = Vec::new();
    for &(k, strict) in &hole_picks {
        let src = if k % 2 == 0 { &ups } else { &downs };
        if src.is_empty() {
            continue;
        }
        let at = src[(k / 2) % src.len()];
        dclauses.push(if strict {
            DClause::AfterOrConc(at)
        } else {
            DClause::Since(at)
        });
        uclauses.push(if strict {
            UClause::BeforeOrConc(at)
        } else {
            UClause::Until(at)
        });
    }
    if dclauses.is_empty() {
        return Ok(());
    }
    for b in &bounds {
        dclauses.push(DClause::N(*b));
        uclauses.push(UClause::N(*b));
    }
    let dq = conjoin_down(dclauses.iter().map(|c| c.item(&versions)).collect(), shape).unwrap();
    let uq = conjoin_up(
        uclauses.iter().map(|c| c.item(&versions)).collect(),
        shape.rotate_left(29),
    )
    .unwrap();
    let dadm = |v: &[BigUint]| dclauses.iter().all(|c| c.admits(v, &pool));
    let uadm = |v: &[BigUint]| uclauses.iter().all(|c| c.admits(v, &pool));
    let db: Vec<Vect> = dclauses.iter().flat_map(|c| c.bounds(&pool)).collect();
    let ub: Vec<Vect> = uclauses.iter().flat_map(|c| c.bounds(&pool)).collect();
    stat(format!(
        "antichain holes down={} up={}",
        format!("{dq:?}").matches('!').count(),
        format!("{uq:?}").matches('!').count()
    ));
    for (pv, p) in pool.iter().zip(&versions) {
        prop_assert_eq!(
            dq.contains(p),
            dadm(pv),
            "antichain down contains {:?} at {:?}",
            dq,
            pv
        );
        prop_assert_eq!(
            uq.contains(p),
            uadm(pv),
            "antichain up contains {:?} at {:?}",
            uq,
            pv
        );
    }
    for (i, j) in span_pairs(&pool) {
        let (lo, hi) = (&pool[i], &pool[j]);
        let s = Span::new(&versions[i], &versions[j]).unwrap();
        if let Some(want) = census(lo, hi, &db, &dadm, 20_000) {
            stat(format!("antichain down {want:?}"));
            prop_assert_eq!(
                dq.coverage(s.reborrow()),
                want,
                "antichain down coverage {:?} over [{:?}, {:?}]",
                dq,
                lo,
                hi
            );
        } else {
            stat("antichain down census-skip");
        }
        if let Some(want) = census(lo, hi, &ub, &uadm, 20_000) {
            stat(format!("antichain up {want:?}"));
            prop_assert_eq!(
                uq.coverage(s.reborrow()),
                want,
                "antichain up coverage {:?} over [{:?}, {:?}]",
                uq,
                lo,
                hi
            );
        } else {
            stat("antichain up census-skip");
        }
    }
    Ok(())
}

/// Deep-grid span verdicts (no census): grids up to 64 cells.
fn body_deep_verdicts(world: World) -> Result<(), TestCaseError> {
    let alphabet = world.alphabet();
    let pool = extended(&world.vects(), &alphabet);
    let versions: Vec<Version> = pool.iter().map(|v| version_of(v)).collect();
    let probes: Vec<(Vect, Version)> = pool.iter().cloned().zip(versions.iter().cloned()).collect();
    for (i, j) in span_pairs(&pool) {
        let s = Span::new(&versions[i], &versions[j]).unwrap();
        check_span_verdicts(&s, &pool[i], &pool[j], &probes, "deep")?;
    }
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(
        std::env::var("PROPTEST_CASES").ok().and_then(|s| s.parse().ok()).unwrap_or(64)
    ))]

    /// Antichain-hole queries agree with the census.
    #[test]
    fn l5_antichain_queries(
        world in arb_world(3, 1..=3),
        hole_picks in proptest::collection::vec((0usize..64, any::<bool>()), 2..=8),
        bounds in proptest::collection::vec(arb_n(), 0..=2),
        shape in any::<u64>(),
    ) {
        body_antichain_queries(world, hole_picks, bounds, shape)?;
    }

    /// Deep-grid span verdicts agree with the vector oracle.
    #[test]
    fn l5_deep_verdicts(world in arb_world(6, 2..=4)) {
        body_deep_verdicts(world)?;
    }
}

/// Behavioral histograms for the antichain and deep properties.
#[test]
#[ignore]
fn l5_stats2() {
    run_stats(
        "l5_antichain_queries",
        (
            arb_world(3, 1..=3),
            proptest::collection::vec((0usize..64, any::<bool>()), 2..=8),
            proptest::collection::vec(arb_n(), 0..=2),
            any::<u64>(),
        ),
        |(w, h, b, s)| body_antichain_queries(w, h, b, s),
    );
    run_stats("l5_deep_verdicts", (arb_world(6, 2..=4),), |(w,)| {
        body_deep_verdicts(w)
    });
    let stats = STATS.lock().unwrap();
    for (k, v) in stats.iter() {
        eprintln!("STAT {k} = {v}");
    }
}

// ───────────────────────────── carry-boundary cost family ─────────────────────────────

/// Touch readings for the walks on a probe whose cells alternate across a
/// power-of-two boundary, against bounds placed so every running difference
/// also alternates across one.
#[cfg(feature = "touch-meter")]
#[test]
#[ignore]
fn l5_touch_carry_family() {
    use suanpan::touch_meter;
    for (m, cells) in [(256usize, 64usize), (512, 128), (1024, 256), (2048, 512)] {
        let p = (BigUint::from(1u8) << m) - 1u8; // 2^m - 1
        let k = m - 8;
        let b = &p - ((BigUint::from(1u8) << k) - 1u8); // probe - b alternates 2^k - 1, 2^k
        let hi_v = BigUint::from(1u8) << (m + 1);
        let probe_cells: Vect = (0..cells)
            .map(|c| if c % 2 == 0 { p.clone() } else { &p + 1u8 })
            .collect();
        let probe = version_of(&probe_cells);
        let lo = version_of(&vec![b.clone(); cells]);
        let hi = version_of(&vec![hi_v.clone(); cells]);
        let hole = version_of(&vec![&b - 1u8; cells]);
        let bytes = probe.encode().len() + lo.encode().len() + hi.encode().len();
        let span = Span::new(&lo, &hi).unwrap();
        let q = causally::after(&lo) & causally::before(&hi) & causally::since(&hole);
        let fresh_probe = version_of(&probe_cells);
        let pspan = Span::new(&probe, &fresh_probe).unwrap();
        let mut row = Vec::new();
        macro_rules! t {
            ($name:literal, $e:expr) => {{
                touch_meter::reset();
                std::hint::black_box($e);
                row.push(format!("{}={}", $name, touch_meter::touches()));
            }};
        }
        t!("cmp", probe.partial_cmp(&lo));
        t!("place", span.place(&probe));
        t!("dominance", span.dominance(&probe));
        t!("contains", span.contains(&probe));
        t!("qcontains", q.contains(&probe));
        t!("qcoverage", q.coverage(pspan.reborrow()));
        eprintln!("CARRY m={m} cells={cells} bytes={bytes} {}", row.join(" "));
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(
        std::env::var("PROPTEST_CASES").ok().and_then(|s| s.parse().ok()).unwrap_or(64)
    ))]

    /// Shaped-world span verdicts (spines and random shapes).
    #[test]
    fn l5_shaped_verdicts(world in arb_shaped_world(2..=4)) {
        body_deep_verdicts(world)?;
    }

    /// Shaped-world queries against the census.
    #[test]
    fn l5_shaped_queries(
        world in arb_shaped_world(2..=4),
        nclauses in arb_nclauses(),
        dclauses in proptest::collection::vec(arb_d(), 1..=5),
        uclauses in proptest::collection::vec(arb_u(), 1..=5),
        shape in any::<u64>(),
    ) {
        body_queries_match_vectors(world, nclauses, dclauses, uclauses, shape)?;
    }

    /// Shaped-world antichain queries against the census.
    #[test]
    fn l5_shaped_antichains(
        world in arb_shaped_world(1..=3),
        hole_picks in proptest::collection::vec((0usize..64, any::<bool>()), 2..=8),
        bounds in proptest::collection::vec(arb_n(), 0..=2),
        shape in any::<u64>(),
    ) {
        body_antichain_queries(world, hole_picks, bounds, shape)?;
    }

    /// Shaped-world projected-span views.
    #[test]
    fn l5_shaped_own_span(world in arb_shaped_world(2..=4), mask_bits in any::<u64>()) {
        body_own_span_matches_vectors(world, mask_bits)?;
    }

    /// Shaped-world span algebra.
    #[test]
    fn l5_shaped_algebra(world in arb_shaped_world(2..=4), picks in proptest::collection::vec(0usize..64, 0..=6)) {
        body_span_algebra_matches_vectors(world, picks)?;
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(
        std::env::var("PROPTEST_CASES").ok().and_then(|s| s.parse().ok()).unwrap_or(32)
    ))]

    /// Organic-population span verdicts.
    #[test]
    fn l5_organic_verdicts(world in arb_organic_world()) {
        body_deep_verdicts(world)?;
    }

    /// Organic-population queries against the census.
    #[test]
    fn l5_organic_queries(
        world in arb_organic_world(),
        nclauses in arb_nclauses(),
        dclauses in proptest::collection::vec(arb_d(), 1..=5),
        uclauses in proptest::collection::vec(arb_u(), 1..=5),
        shape in any::<u64>(),
    ) {
        body_queries_match_vectors(world, nclauses, dclauses, uclauses, shape)?;
    }

    /// Organic-population antichain queries against the census.
    #[test]
    fn l5_organic_antichains(
        world in arb_organic_world(),
        hole_picks in proptest::collection::vec((0usize..64, any::<bool>()), 2..=8),
        bounds in proptest::collection::vec(arb_n(), 0..=2),
        shape in any::<u64>(),
    ) {
        body_antichain_queries(world, hole_picks, bounds, shape)?;
    }

    /// Organic-population projected-span views and algebra.
    #[test]
    fn l5_organic_own_and_algebra(world in arb_organic_world(), mask_bits in any::<u64>(), picks in proptest::collection::vec(0usize..64, 0..=6)) {
        body_own_span_matches_vectors(world.clone(), mask_bits)?;
        body_span_algebra_matches_vectors(world, picks)?;
    }
}

/// Behavioral histograms for shaped and organic worlds.
#[test]
#[ignore]
fn l5_stats3() {
    run_stats("organic-world", (arb_organic_world(),), |(w,)| {
        stat(format!(
            "organic leaves={} depth={} pool={}",
            (w.layout.as_ref().unwrap().leaves() / 8) * 8,
            w.depth,
            w.vects().len()
        ));
        Ok(())
    });
    run_stats(
        "l5_organic_queries",
        (
            arb_organic_world(),
            arb_nclauses(),
            proptest::collection::vec(arb_d(), 1..=5),
            proptest::collection::vec(arb_u(), 1..=5),
            any::<u64>(),
        ),
        |(w, n, d, u, s)| body_queries_match_vectors(w, n, d, u, s),
    );
    run_stats("shaped-world", (arb_shaped_world(2..=4),), |(w,)| {
        stat(format!(
            "shaped leaves={} depth={}",
            w.layout.as_ref().unwrap().leaves(),
            w.depth
        ));
        Ok(())
    });
    run_stats(
        "l5_shaped_queries",
        (
            arb_shaped_world(2..=4),
            arb_nclauses(),
            proptest::collection::vec(arb_d(), 1..=5),
            proptest::collection::vec(arb_u(), 1..=5),
            any::<u64>(),
        ),
        |(w, n, d, u, s)| body_queries_match_vectors(w, n, d, u, s),
    );
    let stats = STATS.lock().unwrap();
    for (k, v) in stats.iter() {
        eprintln!("STAT {k} = {v}");
    }
}

/// Exhaustive check over the 4-cell boolean cube (16 versions): every single
/// clause and every same-polarity clause pair, against every ordered segment,
/// for membership and exact coverage.
#[test]
#[ignore]
fn l5_exhaustive_boolean_cube() {
    set_layout(None);
    let pool: Vec<Vect> = (0u32..16)
        .map(|b| (0..4).map(|c| BigUint::from((b >> c) & 1)).collect())
        .collect();
    let versions: Vec<Version> = pool.iter().map(|v| version_of(v)).collect();
    let n = pool.len();
    let mut ns: Vec<NClause> = Vec::new();
    for i in 0..n {
        ns.push(NClause::After(i));
        ns.push(NClause::Before(i));
    }
    let mut ds: Vec<DClause> = ns.iter().map(|c| DClause::N(*c)).collect();
    let mut us: Vec<UClause> = ns.iter().map(|c| UClause::N(*c)).collect();
    for i in 0..n {
        ds.push(DClause::Since(i));
        ds.push(DClause::StrictlyAfter(i));
        ds.push(DClause::AfterOrConc(i));
        us.push(UClause::Until(i));
        us.push(UClause::StrictlyBefore(i));
        us.push(UClause::BeforeOrConc(i));
    }
    let pairs = span_pairs(&pool);
    let spans: Vec<Span<'_>> = pairs
        .iter()
        .map(|&(i, j)| Span::new(&versions[i], &versions[j]).unwrap())
        .collect();
    let mut checked = 0u64;
    let mut tally = std::collections::BTreeMap::<String, u64>::new();
    macro_rules! sweep {
        ($clauses:expr, $conj:ident, $name:literal, $polar:expr) => {
            for a in 0..$clauses.len() {
                for b in a..$clauses.len() {
                    let cl = [$clauses[a], $clauses[b]];
                    if !cl.iter().any($polar) {
                        continue;
                    }
                    for flip in [0u64, u64::MAX] {
                        let q =
                            $conj(cl.iter().map(|c| c.item(&versions)).collect(), flip).unwrap();
                        let adm = |v: &[BigUint]| cl.iter().all(|c| c.admits(v, &pool));
                        let bounds: Vec<Vect> = cl.iter().flat_map(|c| c.bounds(&pool)).collect();
                        for (pv, p) in pool.iter().zip(&versions) {
                            assert_eq!(
                                q.contains(p),
                                adm(pv),
                                "{} contains {:?} at {:?}",
                                $name,
                                q,
                                pv
                            );
                        }
                        for (k, &(i, j)) in pairs.iter().enumerate() {
                            let want = census(&pool[i], &pool[j], &bounds, &adm, 1 << 20).unwrap();
                            *tally.entry(format!("{} {want:?}", $name)).or_insert(0) += 1;
                            assert_eq!(
                                q.coverage(spans[k].reborrow()),
                                want,
                                "{} coverage {:?} over [{:?}, {:?}]",
                                $name,
                                q,
                                pool[i],
                                pool[j]
                            );
                            checked += 1;
                        }
                    }
                }
            }
        };
    }
    sweep!(ds, conjoin_down, "down", |c: &DClause| !matches!(
        c,
        DClause::N(_)
    ));
    sweep!(us, conjoin_up, "up", |c: &UClause| !matches!(
        c,
        UClause::N(_)
    ));
    // Neutral pairs through the neutral fold.
    for a in 0..ns.len() {
        for b in a..ns.len() {
            let cl = [ns[a], ns[b]];
            let q = conjoin_neutral(cl.iter().map(|c| c.item::<Neutral>(&versions)).collect(), 0);
            let adm = |v: &[BigUint]| cl.iter().all(|c| c.admits(v, &pool));
            let bounds: Vec<Vect> = cl.iter().flat_map(|c| c.bounds(&pool)).collect();
            for (k, &(i, j)) in pairs.iter().enumerate() {
                let want = census(&pool[i], &pool[j], &bounds, &adm, 1 << 20).unwrap();
                *tally.entry(format!("neutral {want:?}")).or_insert(0) += 1;
                assert_eq!(
                    q.coverage(spans[k].reborrow()),
                    want,
                    "neutral coverage {:?}",
                    q
                );
                checked += 1;
            }
        }
    }
    eprintln!("EXHAUSTIVE checked={checked} {tally:?}");
}

// ───────────────────────────── reach comparison ─────────────────────────────

/// Count leaves and depth of an oracle event tree.
fn tree_leaves_depth(t: &tree::Version) -> (usize, usize) {
    match t {
        tree::Version::Leaf(_) => (1, 0),
        tree::Version::Node(_, l, r) => {
            let (a, da) = tree_leaves_depth(l);
            let (b, db) = tree_leaves_depth(r);
            (a + b, 1 + da.max(db))
        }
    }
}

/// Reach of the committed walk-differential population
/// (`span_walks_match_the_composed_sweeps` and
/// `filter_coverage_matches_the_composed_sweeps`: three `arb_oracle_version`
/// draws, spans from meet/join, probes the operands and corners) against the
/// grid worlds: placement mix, coverage-verdict mix of the composed fold, and
/// operand shape.
#[test]
#[ignore]
fn l5_reach_committed() {
    use crate::testing::generators::arb_oracle_version;
    use proptest::test_runner::{Config, TestRunner};
    let cases = std::env::var("L5_STATS_CASES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(256);
    let mut runner = TestRunner::new(Config::with_cases(cases));
    runner
        .run(
            &(
                arb_oracle_version(),
                arb_oracle_version(),
                arb_oracle_version(),
            ),
            |(a, b, c)| {
                for t in [&a, &b, &c] {
                    let (leaves, depth) = tree_leaves_depth(t);
                    stat(format!(
                        "committed shape leaves<={} depth={}",
                        leaves.next_power_of_two(),
                        depth
                    ));
                }
                let (a, b, c) = (
                    from_oracle_version(&a),
                    from_oracle_version(&b),
                    from_oracle_version(&c),
                );
                let (meet, join) = (&b & &c, &b | &c);
                let mut pairs: Vec<(&Version, &Version)> = vec![(&meet, &join), (&meet, &meet)];
                if b <= c {
                    pairs.push((&b, &c));
                    stat("committed raw pair ordered");
                } else {
                    stat("committed raw pair unordered");
                }
                for (lo, hi) in pairs {
                    let s = Span::new(lo, hi).unwrap();
                    for probe in [&a, &b, &c, &meet, &join] {
                        stat(format!("committed place {:?}", s.place(probe)));
                    }
                }
                // Committed exact-coverage reach: query coverage on random spans
                // is only soundness-checked there; record the verdict mix the
                // composed fold would produce for a hole at c over [meet, join].
                let s = Span::new(&meet, &join).unwrap();
                stat(format!(
                    "committed coverage since(c) {:?}",
                    causally::since(&c).coverage(s.reborrow())
                ));
                stat(format!(
                    "committed coverage after(c) {:?}",
                    Query::from(causally::after(&c)).coverage(s.reborrow())
                ));
                Ok(())
            },
        )
        .unwrap();
    // The grid worlds' shape reach, for comparison.
    let mut runner = TestRunner::new(Config::with_cases(cases));
    runner
        .run(&(arb_shaped_world(2..=4),), |(w,)| {
            let lay = w.layout.clone().unwrap();
            for v in w.vects() {
                let t = tree_by_layout(&lay, &v);
                let (leaves, depth) = tree_leaves_depth(&t);
                stat(format!(
                    "shaped shape leaves<={} depth<={}",
                    leaves.next_power_of_two(),
                    depth.next_power_of_two()
                ));
            }
            Ok(())
        })
        .unwrap();
    let stats = STATS.lock().unwrap();
    for (k, v) in stats.iter() {
        eprintln!("STAT {k} = {v}");
    }
}

/// Scan bits of `Query::coverage` on a neutral `Partial` verdict, against the
/// fused walk alone: the difference is the clamp refinement's cost.
#[cfg(feature = "scan-meter")]
#[test]
#[ignore]
fn l5_refine_partial_scan_cost() {
    use crate::testing::meter::{reset_scan_bits, scan_bits};
    use crate::version::place::filter::{self, Demand};
    set_layout(None);
    for depth in [6u32, 7, 8, 9] {
        let cells = 1usize << depth;
        let v = |f: &dyn Fn(usize) -> u32| -> Version {
            version_of(&(0..cells).map(|c| BigUint::from(f(c))).collect::<Vect>())
        };
        let lo = v(&|_| 0);
        let hi = v(&|c| 3 - (c % 2) as u32);
        let floor = v(&|c| (c % 2) as u32);
        let ceiling = v(&|c| 2 + ((c / 2) % 2) as u32);
        let span = Span::new(&lo, &hi).unwrap();
        let q = causally::after(&floor) & causally::before(&ceiling);
        reset_scan_bits();
        let got = q.coverage(span.reborrow());
        let total = scan_bits();
        reset_scan_bits();
        let walk = filter::coverage(
            &lo,
            &hi,
            [(&floor, Demand::After), (&ceiling, Demand::Before)],
        );
        let walk_bits = scan_bits();
        reset_scan_bits();
        let direct = floor <= ceiling;
        let direct_bits = scan_bits();
        let bytes =
            lo.encode().len() + hi.encode().len() + floor.encode().len() + ceiling.encode().len();
        eprintln!(
            "REFINE cells={cells} bytes={bytes} verdict={got:?} walk={walk:?} total_scan={total} walk_scan={walk_bits} refine_scan={} floor<=ceiling={direct} direct_scan={direct_bits}",
            total - walk_bits
        );
    }
}
