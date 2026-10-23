//! A flat model of versions and parties: step functions over the unit
//! interval, held as leaf lists and interval sets and judged by loops over
//! their common refinement.
//!
//! The crate's other two models of the algebra each leave a region unjudged.
//! The recursive
//! oracle ([`tree`]) transcribes the paper's recursions over trees, so it
//! shares production's tree representation, and the bridge that turns its
//! answers into production values writes them through production's own
//! version writer: a canonicalization error the writer and the bridge share
//! passes every comparison against it. The function-space oracle shares
//! neither, but it scans `2^g` grid points one past the operands' deepest
//! boundary, so it judges nothing past a few dozen levels. This model is
//! independent of the tree recursion and of every production writer, and
//! its cost grows with the number of leaves times the width of a position,
//! not with `2^depth`, so it judges versions and parties hundreds of levels
//! deep.
//!
//! Production meets the model only through bytes. A model value enters
//! production by decoding the independent encoder's ([`codec`]) spelling of
//! its canonical form, and a production result is judged by comparing its
//! bytes with the encoder's spelling of the model's answer. The canonical
//! form, the lattice, the order, projection, shapes, the measures, and every
//! party operation are computed here; nothing is read back from production.
//!
//! # Versions
//!
//! A [`Version`] is a version's leaves left to right, each a
//! [`Leaf`](crate::testing::shape_rows::Leaf) row with its depth and its
//! *absolute height*, the sum of the bases on its root-to-leaf path: the
//! same rows the deep generators draw
//! ([`LeafList`](crate::testing::generators::leaf_list::LeafList)) and the
//! shape walks fold into
//! ([`fold_heights`](crate::testing::shape_rows::fold_heights)). Its rows
//! are always canonical: no two sibling leaves share a height.
//!
//! Every binary operation works on the operands' *common refinement*: the
//! coarsest partition of `[0, 1)` into dyadic cells that each lies inside one
//! leaf of every operand. Join and meet are the pointwise maximum and
//! minimum on it, the order is the pointwise order, and projection zeroes the
//! cells a party does not own. Each result is then canonicalized by
//! collapsing equal siblings, bottom up.
//!
//! # Parties
//!
//! A [`Party`] is a set of disjoint half-open intervals of `[0, 1)` with
//! dyadic endpoints, each endpoint a `BigUint` numerator over `2^scale`.
//! Set algebra is interval arithmetic. [`Party::fork`] is the paper's split,
//! characterized without trees: it finds the smallest dyadic interval that
//! holds the whole set and splits that interval at its midpoint: the keeper
//! takes the part of the set left of the midpoint, and the new party the
//! part right of it. A
//! balanced partition into `n` shares places `⌈n/2⌉` shares in the keeper's
//! half and `⌊n/2⌋` in the other, recursively; [`Party::share`] reaches any
//! one share by a single descent, so counts far wider than a machine word
//! cost only their bit length.
//!
//! # Positions
//!
//! Versions and parties alike locate a dyadic interval by a *position*: the numerator of
//! its start over `2^scale`, for a scale at least the deepest depth in
//! play. A leaf at depth `d` is `2^(scale − d)` positions wide. Positions
//! are `BigUint`s, so no depth is too deep to locate, and every traversal
//! here is a loop, so no depth exhausts the stack.
//!
//! # What it does not model
//!
//! Ticks, `min_ticks`, and spans' storage sharing have no counterpart here,
//! and [`Party::fork`] computes the paper's split, the one production and
//! the recursive oracle compute, where the function-space oracle checks that
//! the algebra holds under any valid split.

use core::cmp::Ordering;

use num_bigint::{BigInt, BigUint, Sign};
use proptest::prelude::*;

use crate::testing::oracles::{codec, tree};
use crate::testing::shape_rows::{assert_tiles, Leaf};

mod tests;

// ─────────────────────────────── refinement ───────────────────────────────

/// One cell of a common refinement: its depth, and for each input the index
/// of the input's leaf that holds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Cell {
    /// The cell's depth; it is `2^-depth` wide.
    pub(crate) depth: u64,
    /// For each input, in input order, the index of its leaf holding the
    /// cell.
    pub(crate) leaves: Vec<usize>,
}

/// The common refinement of leaf depth lists, each tiling `[0, 1)`, as cells
/// left to right.
///
/// The traversal keeps each input's current leaf and the position where it
/// ends. At each cell boundary, the inputs' current leaves all hold the
/// boundary point, so they are nested dyadic intervals, and one of them
/// starts there: the input whose leaf ended at the boundary advanced to it.
/// The deepest of the nested leaves is therefore the next cell, whole.
/// Zero inputs refine to the single cell `[0, 1)`.
///
/// # Panics
///
/// Panics if a depth list does not tile `[0, 1)`.
pub(crate) fn refine(inputs: &[Vec<u64>]) -> Vec<Cell> {
    for depths in inputs {
        assert_tiles(depths.iter().copied());
    }
    let scale = inputs.iter().flatten().copied().max().unwrap_or(0);
    let width = |depth: u64| BigUint::from(1u8) << (scale - depth);
    let one = width(0);
    let mut index = vec![0usize; inputs.len()];
    let mut end: Vec<BigUint> = inputs.iter().map(|depths| width(depths[0])).collect();
    let mut at = BigUint::ZERO;
    let mut cells = Vec::new();
    loop {
        let depth = inputs
            .iter()
            .zip(&index)
            .map(|(depths, &i)| depths[i])
            .max()
            .unwrap_or(0);
        cells.push(Cell {
            depth,
            leaves: index.clone(),
        });
        at += width(depth);
        if at == one {
            return cells;
        }
        for ((depths, i), end) in inputs.iter().zip(&mut index).zip(&mut end) {
            if *end == at {
                *i += 1;
                *end += width(depths[*i]);
            }
        }
    }
}

/// Collapses every pair of sibling leaves of equal height into their parent,
/// bottom up, until none remains: the canonical rows of the same step
/// function.
///
/// A stack holds the completed subtrees still waiting for a sibling, each as
/// its depth and whether it is a single leaf, the last row written. When the
/// incoming subtree and the stack's top sit at one depth they are siblings,
/// and they close into their parent; two single leaves of one height
/// collapse into one leaf there, which may collapse again with the next
/// subtree down the stack. `on_collapse` hears each collapse: the surviving
/// left leaf, and whether the collapse cascaded from one just before it.
///
/// # Panics
///
/// Panics if the rows do not tile `[0, 1)`.
fn collapse(
    rows: impl IntoIterator<Item = Leaf>,
    mut on_collapse: impl FnMut(&[Leaf], bool),
) -> Vec<Leaf> {
    let mut out: Vec<Leaf> = Vec::new();
    let mut open: Vec<(u64, bool)> = Vec::new();
    for leaf in rows {
        let mut depth = leaf.depth;
        let mut single = true;
        let mut cascade = false;
        out.push(leaf);
        while open.last().is_some_and(|&(top, _)| top == depth) {
            let (_, left_single) = open.pop().expect("the loop condition saw a top entry");
            let parent = depth
                .checked_sub(1)
                .expect("rows tile the unit interval: two subtrees close above the root");
            let n = out.len();
            if left_single && single && out[n - 2].height == out[n - 1].height {
                out.pop();
                out[n - 2].depth = parent;
                on_collapse(&out, cascade);
                cascade = true;
            } else {
                single = false;
                cascade = false;
            }
            depth = parent;
        }
        open.push((depth, single));
    }
    assert!(
        matches!(open.as_slice(), [(0, _)]),
        "rows tile the unit interval: some subtree is left without a sibling"
    );
    out
}

// ─────────────────────────────── versions ───────────────────────────────

/// A version as its canonical leaf rows, left to right.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Version {
    /// The canonical rows: no two sibling leaves share a height.
    rows: Vec<Leaf>,
}

/// How often a canonicalization reached two of the version writer's
/// collapse paths, as [`Version::collapses`] counts them.
///
/// Production's version writer collapses two equal leaf siblings into their
/// parent, keeping the left leaf's code, and a collapse may repeat at the
/// next level; a *cascade* is such a repeat. A code is *wide* when it is
/// longer than [`NARROW_CODE_BITS`]: a wide code that reaches a collapse
/// sends the writer down its separated-stream path.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Collapses {
    /// Collapses that follow a collapse at the depth just below.
    pub(crate) cascades: usize,
    /// Cascades whose surviving left leaf holds a wide code.
    pub(crate) wide_cascades: usize,
}

/// The longest payload code production's version writer keeps pending in
/// one 64-bit word: an absolute first height through `2^32 − 2`, or a later
/// height change within `±(2^31 − 1)`.
pub(crate) const NARROW_CODE_BITS: u64 = 63;

/// The length in bits of row `i`'s payload code in a version stream: the
/// gamma code of the first leaf's height plus one, or of a later leaf's
/// zigzag-coded height change plus one.
fn code_bits(rows: &[Leaf], i: usize) -> u64 {
    let gamma = |n: BigUint| 2 * (n + 1u8).bits() - 1;
    match i {
        0 => gamma(rows[0].height.clone()),
        _ => {
            let (before, after) = (&rows[i - 1].height, &rows[i].height);
            let zigzag = if after >= before {
                (after - before) << 1u8
            } else {
                ((before - after) << 1u8) - 1u8
            };
            gamma(zigzag)
        }
    }
}

impl Version {
    /// The empty version: one leaf at height zero.
    pub(crate) fn empty() -> Version {
        Version {
            rows: vec![Leaf {
                depth: 0,
                height: BigUint::ZERO,
            }],
        }
    }

    /// The canonical version of a row listing in any spelling.
    ///
    /// # Panics
    ///
    /// Panics if the rows do not tile `[0, 1)`.
    pub(crate) fn from_rows(rows: &[Leaf]) -> Version {
        Version {
            rows: collapse(rows.iter().cloned(), |_, _| ()),
        }
    }

    /// The canonical version of a recursive-oracle tree, read by the
    /// encoder's iterative flattening.
    ///
    /// Only the tree's structure is read, never an oracle operation, so a
    /// tree that is not in normal form still yields its canonical version.
    pub(crate) fn from_oracle(version: &tree::Version) -> Version {
        let rows: Vec<Leaf> = codec::Version::from_oracle(version)
            .leaves()
            .iter()
            .map(|(depth, height)| Leaf {
                depth: *depth,
                height: height
                    .to_biguint()
                    .expect("an oracle tree's heights are sums of nonnegative bases"),
            })
            .collect();
        Version::from_rows(&rows)
    }

    /// The cascades, and the wide cascades, that canonicalizing `rows`
    /// collapses through.
    ///
    /// # Panics
    ///
    /// Panics if the rows do not tile `[0, 1)`.
    pub(crate) fn collapses(rows: &[Leaf]) -> Collapses {
        let mut census = Collapses::default();
        collapse(rows.iter().cloned(), |out, cascade| {
            if cascade {
                census.cascades += 1;
                if code_bits(out, out.len() - 1) > NARROW_CODE_BITS {
                    census.wide_cascades += 1;
                }
            }
        });
        census
    }

    /// The canonical rows, left to right.
    pub(crate) fn rows(&self) -> &[Leaf] {
        &self.rows
    }

    /// The leaves' depths, left to right.
    fn depths(&self) -> Vec<u64> {
        self.rows.iter().map(|leaf| leaf.depth).collect()
    }

    /// The depth of the deepest leaf.
    fn depth(&self) -> u64 {
        self.rows.iter().map(|leaf| leaf.depth).max().unwrap_or(0)
    }

    /// Whether this is the empty version.
    pub(crate) fn is_empty(&self) -> bool {
        *self == Version::empty()
    }

    /// The rows of `f` applied to two versions on their common refinement,
    /// one per cell, before canonicalization: the rows [`pointwise`]
    /// collapses.
    ///
    /// [`pointwise`]: Version::pointwise
    pub(crate) fn pointwise_cells(
        &self,
        other: &Version,
        f: impl Fn(&BigUint, &BigUint) -> BigUint,
    ) -> Vec<Leaf> {
        refine(&[self.depths(), other.depths()])
            .into_iter()
            .map(|cell| Leaf {
                depth: cell.depth,
                height: f(
                    &self.rows[cell.leaves[0]].height,
                    &other.rows[cell.leaves[1]].height,
                ),
            })
            .collect()
    }

    /// Combines two versions pointwise with `f` on their common refinement.
    pub(crate) fn pointwise(
        &self,
        other: &Version,
        f: impl Fn(&BigUint, &BigUint) -> BigUint,
    ) -> Version {
        Version {
            rows: collapse(self.pointwise_cells(other, f), |_, _| ()),
        }
    }

    /// The join: the pointwise maximum.
    pub(crate) fn join(&self, other: &Version) -> Version {
        self.pointwise(other, |a, b| a.max(b).clone())
    }

    /// The meet: the pointwise minimum.
    pub(crate) fn meet(&self, other: &Version) -> Version {
        self.pointwise(other, |a, b| a.min(b).clone())
    }

    /// The join of `self` and every item, left to right.
    pub(crate) fn join_all<'a>(&self, items: impl IntoIterator<Item = &'a Version>) -> Version {
        items
            .into_iter()
            .fold(self.clone(), |acc, item| acc.join(item))
    }

    /// The meet of `self` and every item, left to right.
    pub(crate) fn meet_all<'a>(&self, items: impl IntoIterator<Item = &'a Version>) -> Version {
        items
            .into_iter()
            .fold(self.clone(), |acc, item| acc.meet(item))
    }

    /// Whether `self` lies at or below `other` everywhere: the causal order.
    pub(crate) fn le(&self, other: &Version) -> bool {
        refine(&[self.depths(), other.depths()])
            .iter()
            .all(|cell| self.rows[cell.leaves[0]].height <= other.rows[cell.leaves[1]].height)
    }

    /// The causal relation, from the pointwise order in both directions:
    /// `None` when the versions are concurrent.
    pub(crate) fn relation(&self, other: &Version) -> Option<Ordering> {
        match (self.le(other), other.le(self)) {
            (true, true) => Some(Ordering::Equal),
            (true, false) => Some(Ordering::Less),
            (false, true) => Some(Ordering::Greater),
            (false, false) => None,
        }
    }

    /// The version restricted to the party's region: unchanged where the
    /// party owns, zero elsewhere.
    pub(crate) fn project(&self, party: &Party) -> Version {
        let regions = party.regions();
        let depths: Vec<u64> = regions.iter().map(|&(_, depth)| depth).collect();
        let rows = refine(&[self.depths(), depths])
            .into_iter()
            .map(|cell| Leaf {
                depth: cell.depth,
                height: if regions[cell.leaves[1]].0 {
                    self.rows[cell.leaves[0]].height.clone()
                } else {
                    BigUint::ZERO
                },
            });
        Version {
            rows: collapse(rows, |_, _| ()),
        }
    }

    /// The rows [`fold_cells`](crate::testing::shape_rows::fold_cells) reads
    /// from [`combine`](crate::shape::combine) over these versions: each
    /// refinement cell's depth and every input's height there.
    pub(crate) fn cells(versions: &[&Version]) -> Vec<(u64, Vec<BigUint>)> {
        let inputs: Vec<Vec<u64>> = versions.iter().map(|version| version.depths()).collect();
        refine(&inputs)
            .into_iter()
            .map(|cell| {
                let heights = versions
                    .iter()
                    .zip(&cell.leaves)
                    .map(|(version, &i)| version.rows[i].height.clone())
                    .collect();
                (cell.depth, heights)
            })
            .collect()
    }

    /// The rows [`fold_overlay`](crate::testing::shape_rows::fold_overlay)
    /// reads from the [`shape`](crate::Clock::shape) of a clock holding
    /// `party` and this version.
    ///
    /// Each row is a cell of the version's leaves refined by the party's
    /// canonical regions, with its height and whether the party owns it.
    pub(crate) fn overlay(&self, party: &Party) -> Vec<(u64, BigUint, bool)> {
        let regions = party.regions();
        let depths: Vec<u64> = regions.iter().map(|&(_, depth)| depth).collect();
        refine(&[self.depths(), depths])
            .into_iter()
            .map(|cell| {
                (
                    cell.depth,
                    self.rows[cell.leaves[0]].height.clone(),
                    regions[cell.leaves[1]].0,
                )
            })
            .collect()
    }

    /// The rank, the area under the step function, as the normalized
    /// `(num, exp)` of `num / 2^exp`.
    pub(crate) fn rank(&self) -> (BigUint, u64) {
        let scale = self.depth();
        let area = self
            .rows
            .iter()
            .map(|leaf| &leaf.height << (scale - leaf.depth))
            .sum();
        codec::normalize_rank(area, scale)
    }

    /// The integral of `f(a − b)` over the unit interval, for `a` this
    /// version and `b` the other, as an unnormalized numerator over
    /// `2^scale` and that scale.
    fn integral(&self, other: &Version, f: impl Fn(BigInt) -> BigInt) -> (BigInt, u64) {
        let scale = self.depth().max(other.depth());
        let total = refine(&[self.depths(), other.depths()])
            .into_iter()
            .map(|cell| {
                let difference = BigInt::from(self.rows[cell.leaves[0]].height.clone())
                    - BigInt::from(other.rows[cell.leaves[1]].height.clone());
                f(difference) << (scale - cell.depth)
            })
            .sum();
        (total, scale)
    }

    /// Normalizes a nonnegative integral into a rank's `(num, exp)`.
    fn nonnegative((total, scale): (BigInt, u64)) -> (BigUint, u64) {
        let num = total
            .to_biguint()
            .expect("the integral of a nonnegative function is nonnegative");
        codec::normalize_rank(num, scale)
    }

    /// The distance to `other`: the integral of `|a − b|`, with `a` this
    /// version and `b` the other.
    pub(crate) fn distance(&self, other: &Version) -> (BigUint, u64) {
        Version::nonnegative(self.integral(other, |d| BigInt::from(d.magnitude().clone())))
    }

    /// How far this version lags `other`: the integral of `max(0, b − a)`,
    /// the part where `other` runs ahead.
    pub(crate) fn lag(&self, other: &Version) -> (BigUint, u64) {
        Version::nonnegative(self.integral(other, |d| match d.sign() {
            Sign::Minus => -d,
            _ => BigInt::ZERO,
        }))
    }

    /// The rank order against `other`, from the sign of the integral of
    /// `a − b`.
    pub(crate) fn rank_cmp(&self, other: &Version) -> Ordering {
        let (total, _) = self.integral(other, |d| d);
        match total.sign() {
            Sign::Minus => Ordering::Less,
            Sign::NoSign => Ordering::Equal,
            Sign::Plus => Ordering::Greater,
        }
    }

    /// The encoder's flat spelling of the canonical rows.
    fn spelled(&self) -> codec::Version {
        codec::Version::from_leaves(
            self.rows
                .iter()
                .map(|leaf| (leaf.depth, BigInt::from(leaf.height.clone())))
                .collect(),
        )
    }

    /// The canonical bytes, spelled by the independent encoder.
    pub(crate) fn encode(&self) -> Vec<u8> {
        self.spelled().encode()
    }

    /// The length in bits of the canonical stream before its marker and
    /// padding.
    pub(crate) fn bits(&self) -> u64 {
        u64::try_from(self.spelled().live_bits().len()).expect("a bit count fits in u64")
    }

    /// The production version with these canonical bytes, decoded by the
    /// public strict decoder.
    ///
    /// # Panics
    ///
    /// Panics if production rejects the encoder's spelling of a canonical
    /// version, which is a disagreement between the two on the format.
    pub(crate) fn production(&self) -> crate::Version {
        let bytes = self.encode();
        crate::Version::decode(&bytes[..]).unwrap_or_else(|error| {
            panic!("production rejects a canonical version ({error:?}): {self:?}")
        })
    }
}

// ─────────────────────────────── parties ───────────────────────────────

/// A party as a set of disjoint half-open intervals of `[0, 1)`, possibly
/// empty.
///
/// Every value is canonical, so equality is set equality: the intervals are
/// sorted, nonempty, and never adjacent, and the scale is the least at which
/// every endpoint is a whole position.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Party {
    /// The denominator's exponent: endpoints are numerators over
    /// `2^scale`.
    scale: u64,
    /// The intervals, as `[start, end)` positions, left to right.
    intervals: Vec<(BigUint, BigUint)>,
}

impl Party {
    /// Builds a party from sorted, disjoint intervals at `scale`, dropping
    /// empty ones, merging adjacent ones, and lowering the scale while every
    /// endpoint is even.
    fn canonical(mut scale: u64, intervals: impl IntoIterator<Item = (BigUint, BigUint)>) -> Party {
        let mut merged: Vec<(BigUint, BigUint)> = Vec::new();
        for (start, end) in intervals {
            if start == end {
                continue;
            }
            match merged.last_mut() {
                Some(last) if last.1 == start => last.1 = end,
                _ => merged.push((start, end)),
            }
        }
        let even = |n: &BigUint| !n.bit(0);
        while scale > 0 && merged.iter().all(|(start, end)| even(start) && even(end)) {
            for (start, end) in &mut merged {
                *start >>= 1u8;
                *end >>= 1u8;
            }
            scale -= 1;
        }
        Party {
            scale,
            intervals: merged,
        }
    }

    /// The party owning the regions `rows` marks owned: `(owned, depth)`
    /// rows, left to right, in any spelling.
    ///
    /// # Panics
    ///
    /// Panics if the rows do not tile `[0, 1)`.
    pub(crate) fn from_regions(rows: &[(bool, u64)]) -> Party {
        assert_tiles(rows.iter().map(|&(_, depth)| depth));
        let scale = rows.iter().map(|&(_, depth)| depth).max().unwrap_or(0);
        let mut at = BigUint::ZERO;
        let mut intervals = Vec::new();
        for &(owned, depth) in rows {
            let end = &at + (BigUint::from(1u8) << (scale - depth));
            if owned {
                intervals.push((at.clone(), end.clone()));
            }
            at = end;
        }
        Party::canonical(scale, intervals)
    }

    /// The party a recursive-oracle tree owns, read by the encoder's
    /// iterative flattening.
    ///
    /// Only the tree's structure is read, never an oracle operation.
    pub(crate) fn from_oracle(party: &tree::Party) -> Party {
        let rows: Vec<(bool, u64)> = codec::Party::from_oracle(party)
            .leaves()
            .iter()
            .map(|&(depth, owned)| (owned, depth))
            .collect();
        Party::from_regions(&rows)
    }

    /// The canonical regions, `(owned, depth)` left to right: the leaves of
    /// the party's normal-form tree.
    ///
    /// Each run between consecutive interval endpoints is uniformly owned or
    /// unowned, and its neighbors are not. A run splits greedily into the
    /// widest aligned dyadic cells that fit, and those are exactly the
    /// canonical leaves: a greedy cell's parent either starts before the run
    /// or ends after it, so it holds cells of the other kind.
    pub(crate) fn regions(&self) -> Vec<(bool, u64)> {
        let one = BigUint::from(1u8) << self.scale;
        let mut runs = Vec::with_capacity(2 * self.intervals.len() + 1);
        let mut at = BigUint::ZERO;
        for (start, end) in &self.intervals {
            runs.push((false, at, start.clone()));
            runs.push((true, start.clone(), end.clone()));
            at = end.clone();
        }
        runs.push((false, at, one));
        let mut rows = Vec::new();
        for (owned, mut at, end) in runs {
            while at < end {
                let aligned = at.trailing_zeros().unwrap_or(self.scale);
                let fits = (&end - &at).bits() - 1;
                let width_bits = aligned.min(fits);
                rows.push((owned, self.scale - width_bits));
                at += BigUint::from(1u8) << width_bits;
            }
        }
        rows
    }

    /// Whether the set is empty.
    pub(crate) fn is_empty(&self) -> bool {
        self.intervals.is_empty()
    }

    /// Both parties' intervals at their common scale.
    #[allow(clippy::type_complexity)]
    fn aligned(&self, other: &Party) -> (u64, Vec<(BigUint, BigUint)>, Vec<(BigUint, BigUint)>) {
        let scale = self.scale.max(other.scale);
        let at = |party: &Party| -> Vec<(BigUint, BigUint)> {
            let shift = scale - party.scale;
            party
                .intervals
                .iter()
                .map(|(start, end)| (start << shift, end << shift))
                .collect()
        };
        (scale, at(self), at(other))
    }

    /// The intersection.
    pub(crate) fn intersection(&self, other: &Party) -> Party {
        let (scale, a, b) = self.aligned(other);
        let mut out = Vec::new();
        let (mut i, mut j) = (0, 0);
        while i < a.len() && j < b.len() {
            let start = (&a[i].0).max(&b[j].0).clone();
            let end = (&a[i].1).min(&b[j].1).clone();
            if start < end {
                out.push((start, end));
            }
            if a[i].1 < b[j].1 {
                i += 1;
            } else {
                j += 1;
            }
        }
        Party::canonical(scale, out)
    }

    /// The complement within `[0, 1)`.
    pub(crate) fn complement(&self) -> Party {
        let one = BigUint::from(1u8) << self.scale;
        let mut out = Vec::with_capacity(self.intervals.len() + 1);
        let mut at = BigUint::ZERO;
        for (start, end) in &self.intervals {
            out.push((at, start.clone()));
            at = end.clone();
        }
        out.push((at, one));
        Party::canonical(self.scale, out)
    }

    /// The union.
    pub(crate) fn union(&self, other: &Party) -> Party {
        self.complement()
            .intersection(&other.complement())
            .complement()
    }

    /// The difference: the part of `self` that `other` does not own.
    pub(crate) fn difference(&self, other: &Party) -> Party {
        self.intersection(&other.complement())
    }

    /// Whether the two sets share no point.
    pub(crate) fn is_disjoint(&self, other: &Party) -> bool {
        self.intersection(other).is_empty()
    }

    /// Whether `self` holds every point of `other`.
    pub(crate) fn covers(&self, other: &Party) -> bool {
        other.difference(self).is_empty()
    }

    /// The dyadic interval at `start` of width `2^(scale − depth)`, at
    /// `scale`.
    fn node(scale: u64, start: BigUint, depth: u64) -> Party {
        let end = &start + (BigUint::from(1u8) << (scale - depth));
        Party::canonical(scale, [(start, end)])
    }

    /// The paper's split: the keeper's half and the new party's half.
    ///
    /// The set's hull, from its first point to its last, lies inside a
    /// smallest dyadic interval. If the set fills that interval, its two
    /// halves are the result. Otherwise the hull crosses the interval's
    /// midpoint, so the set meets both halves, and each half keeps its own
    /// part of the set. Both cases are the set's intersection with each
    /// half. The tree recursion arrives at the same split: it descends
    /// through one-child nodes to the hull's interval, then splits a full
    /// leaf in two or a two-child node into its children.
    ///
    /// # Panics
    ///
    /// Panics if the set is empty, which no party is.
    pub(crate) fn fork(&self) -> (Party, Party) {
        let (first, last) = match (self.intervals.first(), self.intervals.last()) {
            (Some(first), Some(last)) => (&first.0, &last.1),
            _ => panic!("an empty set has no split"),
        };
        // One level finer than the set, so the hull interval's halves are
        // whole positions even when the hull is a single position wide.
        let scale = self.scale + 1;
        let (lo, hi) = (first << 1u8, (last << 1u8) - 1u8);
        let depth = (0..=scale)
            .rev()
            .find(|&depth| (&lo >> (scale - depth)) == (&hi >> (scale - depth)))
            .expect("depth 0 holds every point");
        let start = (&lo >> (scale - depth)) << (scale - depth);
        let middle = &start + (BigUint::from(1u8) << (scale - depth - 1));
        let left = Party::node(scale, start, depth + 1);
        let right = Party::node(scale, middle, depth + 1);
        (self.intersection(&left), self.intersection(&right))
    }

    /// Share `i` of the balanced partition into `n` shares, by one descent:
    /// at each split the keeper's half takes the first `⌈n/2⌉` shares and
    /// the other half the rest.
    ///
    /// # Panics
    ///
    /// Panics if `i` is not below `n`.
    pub(crate) fn share(&self, n: &BigUint, i: &BigUint) -> Party {
        assert!(i < n, "share {i} of a partition into {n}");
        let (mut party, mut n, mut i) = (self.clone(), n.clone(), i.clone());
        while n > BigUint::from(1u8) {
            let (keep, give) = party.fork();
            let left = (&n + 1u8) >> 1u8;
            if i < left {
                party = keep;
                n = left;
            } else {
                party = give;
                i -= &left;
                n -= left;
            }
        }
        party
    }

    /// Every share of the balanced partition into `n` shares, in order.
    pub(crate) fn shares(&self, n: usize) -> Vec<Party> {
        let count = BigUint::from(n);
        (0..n)
            .map(|i| self.share(&count, &BigUint::from(i)))
            .collect()
    }

    /// The encoder's flat spelling of the canonical regions.
    fn spelled(&self) -> codec::Party {
        codec::Party::from_leaves(
            self.regions()
                .into_iter()
                .map(|(owned, depth)| (depth, owned))
                .collect(),
        )
    }

    /// The canonical bytes, spelled by the independent encoder.
    ///
    /// # Panics
    ///
    /// Panics if the set is empty, which has no spelling.
    pub(crate) fn encode(&self) -> Vec<u8> {
        assert!(!self.is_empty(), "the empty set has no party spelling");
        self.spelled().encode()
    }

    /// The production party with these canonical bytes, decoded by the
    /// public strict decoder.
    ///
    /// # Panics
    ///
    /// Panics if the set is empty, or if production rejects the encoder's
    /// spelling of a canonical party, which is a disagreement between the
    /// two on the format.
    pub(crate) fn production(&self) -> crate::Party {
        let bytes = self.encode();
        crate::Party::decode(&bytes[..]).unwrap_or_else(|error| {
            panic!("production rejects a canonical party ({error:?}): {self:?}")
        })
    }
}

// ─────────────────────────────── judging production ───────────────────────────────

/// Checks that a production version is the model's, byte for byte.
pub(crate) fn check_version(
    got: &crate::Version,
    want: &Version,
    what: &str,
) -> Result<(), TestCaseError> {
    prop_assert!(
        got.as_bytes() == want.encode().as_slice(),
        "{}: production's {:?} is not the flat model's {:?}",
        what,
        got,
        want
    );
    Ok(())
}

/// Checks that a production rank is the model's `(num, exp)` exactly: the
/// same normalized parts, and the rank format's spelling of them.
pub(crate) fn check_rank(
    got: &crate::Rank,
    want: &(BigUint, u64),
    what: &str,
) -> Result<(), TestCaseError> {
    let (num, exp) = got.raw_parts();
    prop_assert!(
        (num, exp) == (&want.0, want.1),
        "{}: production's rank {} is not the flat model's {} / 2^{}",
        what,
        got,
        want.0,
        want.1
    );
    prop_assert_eq!(
        got.encode(),
        codec::encode_rank(&want.0, want.1),
        "{}: rank bytes",
        what
    );
    Ok(())
}
