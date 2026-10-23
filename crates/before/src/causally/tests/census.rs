//! Exact coverage beyond complete grids, judged by the flat model's census.
//!
//! The parent module's exact-coverage tests enumerate complete grids of
//! shallow versions with small heights, whose antichains are narrow. The
//! [`census`](crate::testing::oracles::flat::census) needs no complete grid:
//! it counts a finite set of covered versions holding a witness for every
//! verdict. [`census_agrees_with_a_finer_pool`] checks it, on small complete
//! grids, against a count over a finer pool that realizes every verdict,
//! with the parent module's clause enums.

use num_bigint::BigUint;
use proptest::prelude::*;

use super::{
    bound_clause_among, down_clause_among, selected, up_clause_among, BoundClause, CausalOrder,
    DownClause, UpClause,
};
use crate::testing::generators::divided_config;
use crate::testing::oracles::flat::census::{census, classify, Grid, Heights};
use crate::testing::oracles::flat::Version as FlatVersion;
use crate::testing::shape_rows::Leaf;

/// The pointwise order on height vectors over one partition, which on an
/// order view equals the causal order of the versions read.
impl CausalOrder for Heights {
    fn le(&self, other: &Self) -> bool {
        Heights::le(self, other)
    }

    fn lt(&self, other: &Self) -> bool {
        Heights::le(self, other) && self != other
    }
}

// ─────────────────────────────── anchors ───────────────────────────────

impl BoundClause {
    /// The pool index of the version this bound compares a probe with.
    fn anchors(&self) -> Vec<usize> {
        match *self {
            Self::After(at) | Self::Before(at) => vec![at],
        }
    }
}

impl DownClause {
    /// The pool indices of the versions this form compares a probe with.
    fn anchors(&self) -> Vec<usize> {
        match *self {
            Self::Since(at)
            | Self::NotBefore(at)
            | Self::StrictlyAfter(at)
            | Self::AfterOrConcurrent(at) => vec![at],
            Self::Delta(start, end) => vec![start, end],
        }
    }
}

impl UpClause {
    /// The pool indices of the versions this form compares a probe with.
    fn anchors(&self) -> Vec<usize> {
        match *self {
            Self::Until(at)
            | Self::NotAfter(at)
            | Self::StrictlyBefore(at)
            | Self::BeforeOrConcurrent(at) => vec![at],
            Self::Toward(start, end) => vec![start, end],
        }
    }
}

/// Which of a case's three queries a predicate or an anchor list belongs to.
#[derive(Clone, Copy, Debug)]
enum Which {
    /// The query of inclusive bounds alone.
    Neutral,
    /// The bounds and the downward forms.
    Downward,
    /// The bounds and the upward forms.
    Upward,
}

/// Every query a case builds, in the order the checks run.
const EVERY_QUERY: [Which; 3] = [Which::Neutral, Which::Downward, Which::Upward];

/// The clauses of one neutral, one downward, and one upward query: the
/// inclusive bounds all three hold, and each polar query's forms.
#[derive(Clone, Debug)]
struct Clauses {
    /// The inclusive bounds every query holds.
    bounds: Vec<BoundClause>,
    /// The downward query's forms.
    down: Vec<DownClause>,
    /// The upward query's forms.
    up: Vec<UpClause>,
}

impl Clauses {
    /// The pool indices the clauses of query `which` compare a probe with.
    fn anchors(&self, which: Which) -> Vec<usize> {
        let bounds = self.bounds.iter().flat_map(BoundClause::anchors);
        match which {
            Which::Neutral => bounds.collect(),
            Which::Downward => bounds
                .chain(self.down.iter().flat_map(DownClause::anchors))
                .collect(),
            Which::Upward => bounds
                .chain(self.up.iter().flat_map(UpClause::anchors))
                .collect(),
        }
    }

    /// Whether query `which` admits `probe`, evaluated directly from the
    /// order on `grid`, the pool its clauses' anchors index.
    fn admits<T: CausalOrder>(&self, which: Which, probe: &T, grid: &[T]) -> bool {
        let bounded = self.bounds.iter().all(|bound| bound.admits(probe, grid));
        bounded
            && match which {
                Which::Neutral => true,
                Which::Downward => self.down.iter().all(|form| form.admits(probe, grid)),
                Which::Upward => self.up.iter().all(|form| form.admits(probe, grid)),
            }
    }
}

/// Draws clauses anchored at the pool indices `anchors` draws: up to
/// `bounds` inclusive bounds, and one to `forms` forms of each polarity.
fn arb_clauses(
    anchors: core::ops::Range<usize>,
    bounds: usize,
    forms: usize,
) -> impl Strategy<Value = Clauses> {
    (
        proptest::collection::vec(bound_clause_among(anchors.clone()), 0..=bounds),
        proptest::collection::vec(down_clause_among(anchors.clone()), 1..=forms),
        proptest::collection::vec(up_clause_among(anchors), 1..=forms),
    )
        .prop_map(|(bounds, down, up)| Clauses { bounds, down, up })
}

// ─────────────────────────────── the census's own check ───────────────────────────────

/// The coarse partitions [`arb_small_grid`] draws, each with the largest
/// height its grid takes: few enough cells and heights that the finer pool
/// stays small.
const SMALL_PARTITIONS: [(&[u64], u64); 4] = [
    (&[1, 1], 3),
    (&[1, 2, 2], 2),
    (&[2, 2, 1], 2),
    (&[2, 2, 2, 2], 1),
];

/// Every height vector over `cells` cells with each height in `0..=max`.
fn every_heights(cells: usize, max: u64) -> Vec<Heights> {
    let mut all = vec![Heights(Vec::with_capacity(cells))];
    for _ in 0..cells {
        all = all
            .into_iter()
            .flat_map(|prefix| {
                (0..=max).map(move |height| {
                    let mut next = prefix.clone();
                    next.0.push(height);
                    next
                })
            })
            .collect();
    }
    all
}

/// Draws one of [`SMALL_PARTITIONS`] and returns every version on it with
/// heights up to its largest: a complete grid, so segments between grid
/// versions nest in every arrangement and hold grid versions strictly
/// inside them.
fn arb_small_grid() -> impl Strategy<Value = Vec<FlatVersion>> {
    prop::sample::select(SMALL_PARTITIONS.to_vec()).prop_map(|(depths, max)| {
        every_heights(depths.len(), max)
            .iter()
            .map(|heights| {
                let rows: Vec<Leaf> = depths
                    .iter()
                    .zip(&heights.0)
                    .map(|(&depth, &height)| Leaf {
                        depth,
                        height: BigUint::from(height),
                    })
                    .collect();
                FlatVersion::from_rows(&rows)
            })
            .collect()
    })
}

/// The anchor indices the census check's clauses draw, reduced modulo the
/// grid's size by [`selected`].
const GRID_PICKS: core::ops::Range<usize> = 0..32;

/// The case divisor of [`census_agrees_with_a_finer_pool`]: each case
/// judges every segment of its grid against a finer pool of hundreds of
/// versions.
const VALIDATOR_DIVISOR: u32 = 4;

proptest! {
    #![proptest_config(divided_config(VALIDATOR_DIVISOR))]

    /// The census holds a witness for every coverage verdict: on small
    /// grids it agrees with a count over a finer pool that realizes every
    /// verdict a covered version can receive.
    ///
    /// Each case takes every version on a partition of two to four cells,
    /// with heights up to a small maximum, and conjunctions from the parent
    /// module's clause strategies anchored anywhere in that grid. For every
    /// segment between grid versions, the census over the grid's order view,
    /// the path the production properties take, must agree with the coverage
    /// counted over the finer pool: every height vector on the grid's
    /// partition with each cell halved. The census can only err by missing
    /// a witness, so the cases that test it are segments holding a version
    /// the query admits strictly inside, between rejected endpoints; a
    /// complete grid holds such segments for every anchor that falls
    /// strictly inside one.
    ///
    /// The finer pool is complete for these verdicts. Each clause compares
    /// a probe with an anchor constant on every cell of the partition, and
    /// `p <= a` holds exactly when `p`'s maximum on each cell is at most
    /// `a`'s height there, `a <= p` exactly when `p`'s minimum is at least
    /// it; the strict forms and equality combine the two. So a conjunction's
    /// verdict depends only on the probe's minimum and maximum on each cell.
    /// A version inside a segment takes integer heights between the
    /// endpoints' on each cell, and the two halves of a cell realize every
    /// such pair. The finer count judges membership by the clauses'
    /// relations in the pointwise order, so no production code takes part.
    #[test]
    fn census_agrees_with_a_finer_pool(
        pool in arb_small_grid(),
        clauses in arb_clauses(GRID_PICKS, 3, 3),
    ) {
        let grid = Grid::new(&pool);
        let view = grid.order_view();
        let height = |h: &BigUint| u64::try_from(h).expect("a small grid's heights fit in u64");
        let lifted: Vec<Heights> = (0..pool.len())
            .map(|i| Heights(grid.heights(i).iter().flat_map(|h| [height(h), height(h)]).collect()))
            .collect();
        let max = lifted.iter().flat_map(|h| h.0.iter().copied()).max().unwrap_or(0);
        let fine = every_heights(2 * grid.depths().len(), max);

        for (lo, hi) in (0..pool.len()).flat_map(|lo| (0..pool.len()).map(move |hi| (lo, hi))) {
            if !view[lo].le(&view[hi]) {
                continue;
            }
            let inside: Vec<&Heights> = fine
                .iter()
                .filter(|probe| lifted[lo].le(probe) && probe.le(&lifted[hi]))
                .collect();
            for which in EVERY_QUERY {
                let anchors = clauses.anchors(which);
                let coarse = census(
                    &view[lo],
                    &view[hi],
                    anchors.iter().map(|&at| selected(&view, at)),
                    |probe| clauses.admits(which, probe, &view),
                );
                let admitted = inside
                    .iter()
                    .filter(|&&probe| clauses.admits(which, probe, &lifted))
                    .count();
                prop_assert_eq!(
                    coarse,
                    classify(admitted, inside.len()),
                    "{:?} census of the segment from grid version {} to {}",
                    which,
                    lo,
                    hi
                );
            }
        }
    }
}
