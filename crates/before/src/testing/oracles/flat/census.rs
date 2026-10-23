//! Exact coverage of a segment by a causal predicate, judged by a census
//! over versions read off the flat model's common refinement.
//!
//! [`Coverage`] is exact by its definition: `Full` when a query admits
//! every version a segment covers, `Empty` when it admits none, and
//! `Partial` otherwise. A segment covers infinitely many versions, so no
//! oracle counts them all. [`census`] counts a finite set of covered
//! versions that provably holds a witness for every verdict, and needs no
//! complete grid of versions to do it, so it judges segments between
//! versions at any depth and with heights of any width.
//!
//! # The view
//!
//! The census works on a view of the flat model, not on a model of its own.
//! [`Grid`] reads a pool of flat [`Version`]s on their common refinement:
//! every pool version is constant on each refinement cell, so each one is a
//! vector of heights, one per cell, and the causal order, join, and meet of
//! any versions built from the pool by joins and meets are the pointwise
//! order, maximum, and minimum of those vectors.
//!
//! [`Grid::order_view`] then keeps only what the causal order reads. On each
//! cell it replaces every height by its rank among the heights the pool takes
//! there, and it merges cells on which every pool version has the same rank.
//! The result is one [`Heights`] per pool version, a vector of small
//! integers. The two steps preserve every answer the census needs: a join or
//! meet of pool versions takes, on each cell, one of the pool's heights
//! there, ranking is a monotone bijection on those heights that commutes
//! with maximum and minimum, and merged cells carry equal values in every
//! join and meet of pool versions. So for any two versions built from the
//! pool by joins and meets, the ranked vectors compare, and are equal,
//! exactly as the versions do. The census's cost therefore depends on the
//! pool's size, not on its depth or the width of its heights.
//!
//! # The census
//!
//! [`census`] judges a predicate over the segment `[lo, hi]` given the
//! predicate's *anchors*, the versions its clauses compare a probe with. Its
//! *generators* are `lo`, `hi`, and every anchor *clamped* into the segment,
//! `(lo ∨ a) ∧ hi`. The census set is every meet and every join of a
//! nonempty set of generators, and the verdict is `Full` when the predicate
//! admits every member, `Empty` when it admits none, and `Partial`
//! otherwise.
//!
//! The verdict is exact for every predicate that admits, inside the segment,
//! an up-closed set `U` intersected with a down-closed set `D`, minus either
//! a down-closed set or an up-closed set `H`, with `D` cut out by inclusive
//! upper bounds `v <= e` and `U` by inclusive lower bounds `s <= v`. Every
//! causal query has this form, with `H` the union of its holes, down-closed
//! for [`Down`](crate::causally::Down) and up-closed for
//! [`Up`](crate::causally::Up). Every member lies in the segment, so the
//! verdict is exact once the set holds a rejected member whenever any
//! covered version is rejected, and an admitted member whenever any covered
//! version is admitted:
//!
//! - A covered version outside `U`, or inside a down-closed `H`, has `lo`
//!   below it, so `lo` is rejected too; one outside `D`, or inside an
//!   up-closed `H`, has `hi` above it, so `hi` is rejected.
//! - Let `t` be `hi` met with every upper bound. An admitted covered version
//!   `v` lies at or below `t`. The meet of `hi` and the upper bounds' clamps
//!   is `lo ∨ t` by distributivity, and `lo ∨ t` is `t` because
//!   `lo <= v <= t`, so `t` is a meet of generators. When `H` is
//!   down-closed, `t` is admitted: it lies in `U` above `v`, in `D`, and
//!   outside `H`, which would hold `v` if it held `t`. When `H` is
//!   up-closed, the dual point, `lo` joined with every lower bound, is
//!   admitted, and it is a join of generators.
//!
//! The census never learns which anchors are upper bounds, lower bounds, or
//! holes: the set holds every meet and join of generators, so it holds the
//! witness for any assignment of roles. It shares no code with production's
//! comparison, its fused walks, or the endpoint refinement those walks
//! finish with, which decides by the same two witnesses but must know each
//! bound's role.
//!
//! `causally`'s census tests check the census on small segments against a
//! count over a finer pool, which realizes every verdict a covered version
//! can receive.

use std::collections::HashSet;

use num_bigint::BigUint;

use super::Version;
use crate::causally::Coverage;

mod tests;

/// The most members a census may hold before it gives up.
///
/// The census set holds every meet and every join of a set of generators,
/// so `g` generators can span up to `2^(g+1)` members. The worlds the
/// census tests draw stay far below this; a world that reaches it is too
/// large to judge exactly, and the census panics rather than skipping it.
pub(crate) const CENSUS_CAP: usize = 1 << 16;

/// A pool of flat versions read on their common refinement.
#[derive(Clone, Debug)]
pub(crate) struct Grid {
    /// The refinement's cells, left to right, by depth.
    depths: Vec<u64>,
    /// Each pool version's height on every cell, version by version.
    heights: Vec<Vec<BigUint>>,
}

impl Grid {
    /// Reads `pool` on its common refinement.
    pub(crate) fn new(pool: &[Version]) -> Grid {
        let refs: Vec<&Version> = pool.iter().collect();
        let cells = Version::cells(&refs);
        let depths = cells.iter().map(|(depth, _)| *depth).collect();
        let heights = (0..pool.len())
            .map(|i| cells.iter().map(|(_, row)| row[i].clone()).collect())
            .collect();
        Grid { depths, heights }
    }

    /// The refinement's cells, left to right, by depth.
    pub(crate) fn depths(&self) -> &[u64] {
        &self.depths
    }

    /// Pool version `i`'s height on every cell, left to right.
    pub(crate) fn heights(&self, i: usize) -> &[BigUint] {
        &self.heights[i]
    }

    /// The pool reduced to what the causal order reads: one [`Heights`] per
    /// pool version, in pool order.
    ///
    /// On each cell, every height becomes its rank among the distinct
    /// heights the pool takes there, and cells on which every pool version
    /// has the same rank merge into one. The module docs give the argument
    /// that order, equality, join, and meet survive the reduction for every
    /// version built from the pool by joins and meets.
    pub(crate) fn order_view(&self) -> Vec<Heights> {
        let mut seen: HashSet<Vec<u64>> = HashSet::new();
        let mut columns: Vec<Vec<u64>> = Vec::new();
        for cell in 0..self.depths.len() {
            let mut values: Vec<&BigUint> = self.heights.iter().map(|h| &h[cell]).collect();
            values.sort();
            values.dedup();
            let column: Vec<u64> = self
                .heights
                .iter()
                .map(|h| {
                    let rank = values
                        .binary_search(&&h[cell])
                        .expect("every pool height is among the cell's sorted heights");
                    u64::try_from(rank).expect("a rank among held heights fits in u64")
                })
                .collect();
            if seen.insert(column.clone()) {
                columns.push(column);
            }
        }
        (0..self.heights.len())
            .map(|i| Heights(columns.iter().map(|column| column[i]).collect()))
            .collect()
    }
}

/// A version's heights on the cells of one partition, as small integers
/// ordered as the heights are: what [`Grid::order_view`] keeps.
///
/// Every operation compares two vectors over one partition, cell by cell,
/// and panics if their lengths differ.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Heights(pub(crate) Vec<u64>);

impl Heights {
    /// Asserts that `self` and `other` share a partition.
    fn same_partition(&self, other: &Heights) {
        assert_eq!(
            self.0.len(),
            other.0.len(),
            "height vectors compare only over one partition"
        );
    }

    /// Whether `self` lies at or below `other` on every cell: the causal
    /// order.
    pub(crate) fn le(&self, other: &Heights) -> bool {
        self.same_partition(other);
        self.0.iter().zip(&other.0).all(|(a, b)| a <= b)
    }

    /// The pointwise maximum: the join.
    pub(crate) fn join(&self, other: &Heights) -> Heights {
        self.same_partition(other);
        Heights(
            self.0
                .iter()
                .zip(&other.0)
                .map(|(a, b)| *a.max(b))
                .collect(),
        )
    }

    /// The pointwise minimum: the meet.
    pub(crate) fn meet(&self, other: &Heights) -> Heights {
        self.same_partition(other);
        Heights(
            self.0
                .iter()
                .zip(&other.0)
                .map(|(a, b)| *a.min(b))
                .collect(),
        )
    }
}

/// Classifies a census: `Full` when every member is admitted, `Empty` when
/// none is, and `Partial` otherwise.
///
/// # Panics
///
/// Panics if `members` is zero: every segment covers its endpoints.
pub(crate) fn classify(admitted: usize, members: usize) -> Coverage {
    assert!(members > 0, "every segment covers its endpoints");
    if admitted == members {
        Coverage::Full
    } else if admitted == 0 {
        Coverage::Empty
    } else {
        Coverage::Partial
    }
}

/// Every combination under `combine`, a meet or a join, of a nonempty subset
/// of `items`.
///
/// The set grows one item at a time: each new item joins the set alone and
/// combined with every member so far, so after `k` items it holds the
/// combination of every nonempty subset of the first `k`.
///
/// # Panics
///
/// Panics if the set would exceed [`CENSUS_CAP`] members.
fn closure(items: &[Heights], combine: impl Fn(&Heights, &Heights) -> Heights) -> Vec<Heights> {
    let mut seen: HashSet<Heights> = HashSet::new();
    let mut members: Vec<Heights> = Vec::new();
    for item in items {
        let combined: Vec<Heights> = members.iter().map(|m| combine(m, item)).collect();
        for member in core::iter::once(item.clone()).chain(combined) {
            if seen.insert(member.clone()) {
                members.push(member);
                assert!(
                    members.len() <= CENSUS_CAP,
                    "a census set passed {CENSUS_CAP} members: the world is too large to judge"
                );
            }
        }
    }
    members
}

/// The exact coverage of the segment `[lo, hi]` by a predicate, given the
/// anchors its clauses compare a probe with.
///
/// The census set is every meet and every join of a nonempty set of
/// generators: `lo`, `hi`, and every anchor clamped into the segment,
/// `(lo ∨ a) ∧ hi`. The verdict counts the members `admits` accepts. The
/// module docs give the argument that it is exact for every causal query's
/// predicate. All vectors must come from one [`Grid::order_view`], so that
/// order and lattice operations on them are the versions' own.
///
/// # Panics
///
/// Panics if `lo` does not lie at or below `hi`, if the vectors do not share
/// a partition, or if the census set would exceed [`CENSUS_CAP`] members.
pub(crate) fn census<'h>(
    lo: &Heights,
    hi: &Heights,
    anchors: impl IntoIterator<Item = &'h Heights>,
    admits: impl Fn(&Heights) -> bool,
) -> Coverage {
    assert!(lo.le(hi), "a segment's bottom lies at or below its top");
    let generators: Vec<Heights> = [lo.clone(), hi.clone()]
        .into_iter()
        .chain(anchors.into_iter().map(|a| lo.join(a).meet(hi)))
        .collect();
    let mut seen: HashSet<&Heights> = HashSet::new();
    let meets = closure(&generators, Heights::meet);
    let joins = closure(&generators, Heights::join);
    let members: Vec<&Heights> = meets
        .iter()
        .chain(&joins)
        .filter(|member| seen.insert(member))
        .collect();
    let admitted = members.iter().filter(|member| admits(member)).count();
    classify(admitted, members.len())
}
