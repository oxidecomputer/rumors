//! Exact coverage beyond complete grids, judged by the flat model's census.
//!
//! The parent module's exact-coverage tests enumerate complete grids of
//! shallow versions with small heights, whose antichains are narrow. This
//! module judges queries over pools of deep versions with wide heights, and
//! queries whose holes form antichains wider than a small grid holds, against
//! [`census`](crate::testing::oracles::flat::census), which counts a finite
//! set of covered versions holding a witness for every verdict. The pools
//! are flat versions; production meets them only through the independent
//! encoder's bytes, and the census reads them only through
//! [`Grid::order_view`].
//!
//! The properties share the clause enums of the parent module:
//!
//! - [`census_agrees_with_a_finer_pool`] checks the census itself, on small
//!   complete grids, against a count over a finer pool that realizes every
//!   verdict.
//! - [`antichain_holes_cover_exactly`] judges queries whose holes sit on an
//!   antichain of bumps at pairwise disjoint deep cells.
//! - [`deep_queries_cover_exactly`] judges conjunctions of every public form
//!   over deep triples and their joins and meets.
//!
//! [`census_cases_keep_mass_on_their_regimes`] floors the regimes the
//! production properties claim.

use core::ops::{Range, RangeInclusive};

use num_bigint::BigUint;
use proptest::prelude::*;
use proptest::sample::Index;
use proptest::test_runner::TestCaseError;

use super::{
    bound_clause_among, down_clause_among, down_query, neutral_query, rendered_holes, selected,
    up_clause_among, up_query, BoundClause, CausalOrder, DownClause, FinerPool, Segment, UpClause,
};
use crate::causally::{Down, Polarity, Query, Up};
use crate::testing::exhaustive::cell_heights;
use crate::testing::generators::census::seeded::{
    assert_default_case_count, assert_floors, regime_floor, sample, Tally, CENSUS_CASES,
};
use crate::testing::generators::leaf_list::{arb_deep_version, arb_deep_version_triple};
use crate::testing::oracles::flat::census::{census, classify, Grid, Heights};
use crate::testing::oracles::flat::Version as FlatVersion;
use crate::testing::property::divided_proptest;
use crate::testing::shape_rows::Leaf;
use crate::{Span, Version};

// ─────────────────────────────── anchors ───────────────────────────────

impl BoundClause {
    /// The pool indices of the versions this bound compares a probe with:
    /// its one anchor.
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
    anchors: Range<usize>,
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

/// The pool indices a clause strategy draws for the deep pools, reduced
/// modulo each pool's size by [`selected`].
const POOL_PICKS: Range<usize> = 0..64;

// ─────────────────────────────── sites ───────────────────────────────

/// Pairwise disjoint dyadic cells at chosen depths: the siblings, off one
/// root-to-leaf path, of the path's nodes at those depths.
///
/// Two sites never overlap: the deeper one lies inside the path's node at
/// the shallower one's depth, which the shallower site, that node's
/// sibling, does not meet.
#[derive(Clone, Debug)]
struct Sites {
    /// The path's steps from the root: `true` steps to the right child.
    path: Vec<bool>,
    /// The sites' depths, increasing, each in `1..=path.len()`.
    depths: Vec<u64>,
}

/// The path lengths [`arb_sites`] draws, long enough that sites often lie
/// past depth 32, where a cell is narrower than a 32-bit fraction resolves.
///
/// Each site's depth is uniform along a path of uniform length, so about
/// half of all sites lie that deep; the floor census checks that the
/// antichain cases keep placing one there.
const SITE_PATH: RangeInclusive<usize> = 3..=160;

impl Sites {
    /// The partition the path cuts: every sibling off the path, and the
    /// path's last node, left to right, each as its depth and the index of
    /// the site it is, if any.
    ///
    /// A step to the right leaves its sibling on the left of everything
    /// deeper, and a step to the left leaves it on the right, so the left
    /// siblings come first in increasing depth, then the last node, then the
    /// right siblings in decreasing depth.
    fn partition(&self) -> Vec<(u64, Option<usize>)> {
        let (mut left, mut right) = (Vec::new(), Vec::new());
        for (depth, &steps_right) in (1u64..).zip(&self.path) {
            let cell = (depth, self.depths.iter().position(|&d| d == depth));
            if steps_right {
                left.push(cell);
            } else {
                right.push(cell);
            }
        }
        let last = u64::try_from(self.path.len()).expect("a path length fits in u64");
        left.push((last, None));
        left.extend(right.into_iter().rev());
        left
    }

    /// `base` raised on the sites `lifted` selects, each by its amount.
    fn lift(
        &self,
        base: &FlatVersion,
        amounts: &[BigUint],
        lifted: impl Fn(usize) -> bool,
    ) -> FlatVersion {
        let rows: Vec<Leaf> = self
            .partition()
            .into_iter()
            .map(|(depth, site)| Leaf {
                depth,
                height: match site {
                    Some(site) if lifted(site) => amounts[site].clone(),
                    _ => BigUint::ZERO,
                },
            })
            .collect();
        base.pointwise(&FlatVersion::from_rows(&rows), |height, by| height + by)
    }

    /// One bump of `base` per site, pairwise concurrent, and `top`: `base`
    /// raised on every site by its amount.
    ///
    /// Read the sites as a cycle, in depth order. Bump `j` raises `base` on
    /// the *window* of `width` consecutive sites starting at site `j`, each
    /// by its amount. `pick` selects `width` from one through one less than
    /// the site count, or one when there is a single site, so distinct
    /// bumps' windows are distinct sets of one size, and each holds a site
    /// the other lacks.
    /// Every amount is at least one, so each bump lies strictly above every
    /// other on some site and strictly below it on another: no bump lies at
    /// or below another.
    ///
    /// At width one each bump raises its own site alone, and at one short of
    /// the site count each is `top` with one site dropped back to `base`.
    /// Between, two bumps meet on the overlap of their windows, so their
    /// pairwise meets differ.
    ///
    /// # Panics
    ///
    /// Panics if `amounts` holds fewer amounts than there are sites.
    fn bumps(
        &self,
        base: &FlatVersion,
        amounts: &[BigUint],
        pick: Index,
    ) -> (FlatVersion, Vec<FlatVersion>) {
        let count = self.depths.len();
        assert!(amounts.len() >= count, "one amount per site");
        let width = 1 + pick.index(count.saturating_sub(1).max(1));
        let top = self.lift(base, amounts, |_| true);
        let bumps = (0..count)
            .map(|own| self.lift(base, amounts, |site| (site + count - own) % count < width))
            .collect();
        (top, bumps)
    }
}

/// Draws a path from [`SITE_PATH`] and `count` distinct site depths along
/// it, as many as the path allows.
fn arb_sites(count: RangeInclusive<usize>) -> impl Strategy<Value = Sites> {
    SITE_PATH.prop_flat_map(move |len| {
        let depths: Vec<u64> =
            (1..=u64::try_from(len).expect("a path length fits in u64")).collect();
        let size = (*count.start()).min(len)..=(*count.end()).min(len);
        (
            proptest::collection::vec(any::<bool>(), len),
            proptest::sample::subsequence(depths, size),
        )
            .prop_map(|(path, depths)| Sites { path, depths })
    })
}

/// Draws `count` amounts a bump moves its site by: one and two, values on
/// both sides of the 32- and 64-bit word boundaries, and far past them.
fn arb_amounts(count: usize) -> impl Strategy<Value = Vec<BigUint>> {
    let pow2 = |k: u32| BigUint::from(1u8) << k;
    let amount = prop::sample::select(vec![
        BigUint::from(1u8),
        BigUint::from(2u8),
        pow2(31),
        pow2(32) + 1u8,
        pow2(64) - 1u8,
        pow2(64),
        pow2(64) + 1u8,
        pow2(200),
    ]);
    proptest::collection::vec(amount, count)
}

// ─────────────────────────────── cases ───────────────────────────────

/// A pool of flat versions and the clauses of one neutral, one downward,
/// and one upward query over it.
#[derive(Clone, Debug)]
struct QueryCase {
    /// The pool the clauses' anchors index.
    pool: Vec<FlatVersion>,
    /// The three queries' clauses.
    clauses: Clauses,
    /// The operand-order bits [`down_query`] and its siblings read.
    sides: u64,
    /// The depth of each site the pool's bumps move.
    site_depths: Vec<u64>,
}

/// How many sites an antichain case places holes on.
const ANTICHAIN_SITES: RangeInclusive<usize> = 3..=8;

/// One antichain site's hole spellings: downward `since`, `!before`, or the
/// strict `after(..).or_concurrent()`, and upward `until`, `!after`, or the
/// strict `before(..).or_concurrent()`.
fn arb_hole_spellings() -> impl Strategy<Value = (u8, u8)> {
    (0u8..3, 0u8..3)
}

/// Draws a case whose downward and upward holes sit on an antichain of
/// three to eight bumps of one deep version, at pairwise disjoint sites.
///
/// The pool is the base version, `top` (the base raised on every site), the
/// bumps, each raising the base on its own window of sites (see
/// [`Sites::bumps`]), and the empty version, so its segments run from below
/// the antichain to above it. Each site carries one hole of each polarity at
/// its bump, in a drawn spelling, and both queries hold up to two inclusive
/// bounds at pool versions, which may prune the holes they make vacuous.
fn arb_antichain_case() -> impl Strategy<Value = QueryCase> {
    (
        arb_deep_version(),
        arb_sites(ANTICHAIN_SITES),
        arb_amounts(*ANTICHAIN_SITES.end()),
        any::<Index>(),
        proptest::collection::vec(arb_hole_spellings(), *ANTICHAIN_SITES.end()),
        proptest::collection::vec(bound_clause_among(POOL_PICKS), 0..=2),
        any::<u64>(),
    )
        .prop_map(|(base, sites, amounts, width, spellings, bounds, sides)| {
            let base = FlatVersion::from_rows(base.leaves());
            let (top, bumps) = sites.bumps(&base, &amounts, width);
            const FIRST_BUMP: usize = 2;
            let mut pool = vec![base, top];
            pool.extend(bumps);
            pool.push(FlatVersion::empty());
            let (down, up) = spellings[..sites.depths.len()]
                .iter()
                .enumerate()
                .map(|(site, &(down, up))| {
                    let at = FIRST_BUMP + site;
                    let down = match down {
                        0 => DownClause::Since(at),
                        1 => DownClause::NotBefore(at),
                        _ => DownClause::AfterOrConcurrent(at),
                    };
                    let up = match up {
                        0 => UpClause::Until(at),
                        1 => UpClause::NotAfter(at),
                        _ => UpClause::BeforeOrConcurrent(at),
                    };
                    (down, up)
                })
                .unzip();
            QueryCase {
                pool,
                clauses: Clauses { bounds, down, up },
                sides,
                site_depths: sites.depths,
            }
        })
}

/// How many sites a deep-query case bumps its first version at.
const QUERY_SITES: RangeInclusive<usize> = 1..=3;

/// Draws a case of conjunctions of every public form over a deep triple.
///
/// The pool is a deep triple, related by construction so equal and nearly
/// equal versions are common, every pairwise join and meet, and an antichain
/// of one to three bumps of the first version at deep sites, with `top`.
/// Each query holds up to five inclusive bounds; the downward and upward
/// queries also hold one to five forms of their polarity, anchored anywhere
/// in the pool.
fn arb_query_case() -> impl Strategy<Value = QueryCase> {
    (
        arb_deep_version_triple(),
        arb_sites(QUERY_SITES),
        arb_amounts(*QUERY_SITES.end()),
        any::<Index>(),
        arb_clauses(POOL_PICKS, 5, 5),
        any::<u64>(),
    )
        .prop_map(|((a, b, c), sites, amounts, width, clauses, sides)| {
            let triple = [a, b, c].map(|list| FlatVersion::from_rows(list.leaves()));
            let mut pool = triple.to_vec();
            for (i, x) in triple.iter().enumerate() {
                for y in &triple[i + 1..] {
                    pool.push(x.join(y));
                    pool.push(x.meet(y));
                }
            }
            let (top, bumps) = sites.bumps(&triple[0], &amounts, width);
            pool.push(top);
            pool.extend(bumps);
            QueryCase {
                pool,
                clauses,
                sides,
                site_depths: sites.depths,
            }
        })
}

// ─────────────────────────────── judging ───────────────────────────────

/// A pool as production reads it and as the census reads it.
struct Judged {
    /// The pool decoded from the encoder's bytes, once.
    versions: Vec<Version>,
    /// The pool decoded again, so no version shares a buffer with
    /// `versions`.
    fresh: Vec<Version>,
    /// The pool's order view.
    view: Vec<Heights>,
}

impl Judged {
    /// Reads a pool both ways.
    fn new(pool: &[FlatVersion]) -> Judged {
        Judged {
            versions: pool.iter().map(FlatVersion::production).collect(),
            fresh: pool.iter().map(FlatVersion::production).collect(),
            view: Grid::new(pool).order_view(),
        }
    }

    /// The census anchors at the given pool indices.
    fn anchors(&self, indices: &[usize]) -> Vec<&Heights> {
        indices.iter().map(|&at| selected(&self.view, at)).collect()
    }
}

/// Checks query `which`'s membership verdict on every pool version, and its
/// coverage of every segment between pool versions, against the census.
///
/// Each segment is spelled twice: over the pool's own buffers, so a point
/// segment borrows one version twice and takes production's shared-buffer
/// shortcut, and with its top from a separately decoded copy, so no two
/// endpoints share a buffer and every segment, points included, takes the
/// coverage walk.
fn assert_exact<P: Polarity>(
    query: &Query<'_, P>,
    which: Which,
    clauses: &Clauses,
    judged: &Judged,
) -> Result<(), TestCaseError> {
    let Judged {
        versions,
        fresh,
        view,
    } = judged;
    let admits = |probe: &Heights| clauses.admits(which, probe, view);
    for (i, probe) in view.iter().enumerate() {
        prop_assert_eq!(
            query.contains(&versions[i]),
            admits(probe),
            "{:?} membership of pool version {} in {:?}",
            which,
            i,
            query
        );
    }
    let anchors = judged.anchors(&clauses.anchors(which));
    for (i, lo) in view.iter().enumerate() {
        for (j, hi) in view.iter().enumerate() {
            if !lo.le(hi) {
                continue;
            }
            let want = census(lo, hi, anchors.iter().copied(), admits);
            for (spelling, upper_pool) in [("shared", versions), ("crossed", fresh)] {
                let span = Span::new(&versions[i], &upper_pool[j]).map_err(|_| {
                    TestCaseError::fail(format!(
                        "production refuses the segment from pool version {i} to {j}, \
                         which the flat model orders"
                    ))
                })?;
                prop_assert_eq!(
                    query.coverage(span),
                    want,
                    "{:?} coverage by {:?} of the {} segment from pool version {} to {}",
                    which,
                    query,
                    spelling,
                    i,
                    j
                );
            }
        }
    }
    Ok(())
}

/// The three queries a case's clauses build over production's pool.
fn queries<'a>(
    case: &QueryCase,
    versions: &'a [Version],
) -> (Query<'a>, Query<'a, Down>, Query<'a, Up>) {
    let Clauses { bounds, down, up } = &case.clauses;
    (
        neutral_query(bounds, case.sides, versions),
        down_query(down, bounds, case.sides, versions),
        up_query(up, bounds, case.sides, versions),
    )
}

/// Checks every query a case builds, at every polarity, against the census.
fn assert_case(case: &QueryCase) -> Result<(), TestCaseError> {
    let judged = Judged::new(&case.pool);
    let (neutral, down, up) = queries(case, &judged.versions);
    assert_exact(&neutral, Which::Neutral, &case.clauses, &judged)?;
    assert_exact(&down, Which::Downward, &case.clauses, &judged)?;
    assert_exact(&up, Which::Upward, &case.clauses, &judged)
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

/// Draws one of [`SMALL_PARTITIONS`] and returns every version on it with
/// heights up to its largest: a complete grid, so segments between grid
/// versions nest in every arrangement and hold grid versions strictly
/// inside them.
fn arb_small_grid() -> impl Strategy<Value = Vec<FlatVersion>> {
    prop::sample::select(SMALL_PARTITIONS.to_vec()).prop_map(|(depths, max)| {
        let cells =
            u32::try_from(depths.len()).expect("a small partition's cell count fits in u32");
        cell_heights(cells, max)
            .iter()
            .map(|heights| {
                let rows: Vec<Leaf> = depths
                    .iter()
                    .zip(heights)
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
const GRID_PICKS: Range<usize> = 0..32;

/// Reads a small grid on its cells halved, beside its [`FinerPool`].
///
/// # Panics
///
/// If a height does not fit in `u64`, which no small grid's does.
fn finer_pool(pool: &[FlatVersion]) -> FinerPool {
    let grid = Grid::new(pool);
    let height = |h: &BigUint| u64::try_from(h).expect("a small grid's heights fit in u64");
    let heights: Vec<Vec<u64>> = (0..pool.len())
        .map(|i| grid.heights(i).iter().map(height).collect())
        .collect();
    FinerPool::new(&heights)
}

/// Whether some segment of `finer` rejects both its endpoints under query
/// `which` yet holds a finer member the query admits: a segment on which a
/// census that missed its interior witness would answer `Empty` for
/// `Partial`.
fn has_interior_witness(finer: &FinerPool, clauses: &Clauses, which: Which) -> bool {
    let admits = |probe: &Heights| clauses.admits(which, probe, &finer.lifted);
    finer.segments.iter().any(|segment| {
        !admits(&finer.lifted[segment.lo])
            && !admits(&finer.lifted[segment.hi])
            && segment.fine_members.iter().any(|&m| admits(&finer.fine[m]))
    })
}

/// The case divisor of [`census_agrees_with_a_finer_pool`]: one, so it runs
/// the full case count. Each case is cheap, and only a minority of cases
/// hold a segment that can expose a census missing its witness.
const VALIDATOR_DIVISOR: u32 = 1;

divided_proptest! {
    #![divisor(VALIDATOR_DIVISOR)]

    /// The census holds a witness for every coverage verdict: on small
    /// grids it agrees with the count over a [`FinerPool`], which realizes
    /// every verdict a covered version can receive.
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
    #[test]
    fn census_agrees_with_a_finer_pool(
        pool in arb_small_grid(),
        clauses in arb_clauses(GRID_PICKS, 3, 3),
    ) {
        let view = Grid::new(&pool).order_view();
        let FinerPool { lifted, fine, segments } = finer_pool(&pool);
        for Segment { lo, hi, fine_members, .. } in &segments {
            let (lo, hi) = (*lo, *hi);
            for which in EVERY_QUERY {
                let anchors = clauses.anchors(which);
                let coarse = census(
                    &view[lo],
                    &view[hi],
                    anchors.iter().map(|&at| selected(&view, at)),
                    |probe| clauses.admits(which, probe, &view),
                );
                let admitted = fine_members
                    .iter()
                    .filter(|&&m| clauses.admits(which, &fine[m], &lifted))
                    .count();
                prop_assert_eq!(
                    coarse,
                    classify(admitted, fine_members.len()),
                    "{:?} census of the segment from grid version {} to {}",
                    which,
                    lo,
                    hi
                );
            }
        }
    }
}

// ─────────────────────────────── production against the census ───────────────────────────────

/// The case divisor of [`antichain_holes_cover_exactly`]: each case holds a
/// pool of deep versions and judges two queries on every segment between
/// them.
const ANTICHAIN_DIVISOR: u32 = 4;

/// The case divisor of [`deep_queries_cover_exactly`]: each case holds a
/// pool of deep versions and judges three queries on every segment between
/// them.
const QUERY_DIVISOR: u32 = 6;

divided_proptest! {
    #![divisor(ANTICHAIN_DIVISOR)]

    /// Queries whose holes form antichains of three to eight on deep cells
    /// admit exactly what their clauses admit, and cover every segment
    /// between pool versions exactly as the census says.
    ///
    /// Each case picks three to eight pairwise disjoint dyadic cells along
    /// one path to depth 160 and builds an antichain of as many bumps of a
    /// deep version, each raising a window of consecutive cells of one
    /// drawn width, by amounts from one to `2^200`. It places one hole of
    /// each polarity at every bump, spelled `since`, `!before`, or
    /// `after(..).or_concurrent()` downward and `until`, `!after`, or
    /// `before(..).or_concurrent()` upward, beside up to two inclusive
    /// bounds, conjoined in a drawn operand order. Membership is checked on
    /// every pool version, and coverage on every segment between two, both
    /// over the pool's own buffers and with the segment's top from a
    /// separate copy.
    #[test]
    fn antichain_holes_cover_exactly(case in arb_antichain_case()) {
        assert_case(&case)?;
    }
}

divided_proptest! {
    #![divisor(QUERY_DIVISOR)]

    /// Neutral, downward, and upward conjunctions of every public form over
    /// deep versions admit exactly what their clauses admit, and cover every
    /// segment between pool versions exactly as the census says.
    ///
    /// Each case's pool is a deep triple to depth 300, with heights to
    /// thousands of bits, every pairwise join and meet, and an antichain of
    /// one to three bumps of the first version at cells to depth 160. Each
    /// query conjoins up to five inclusive bounds and, at a polarity, one to
    /// five forms of it, each in a drawn public spelling, so every spelling
    /// appears across cases. The forms are anchored anywhere in the pool and
    /// conjoined in a drawn operand order. Membership is checked on every
    /// pool version, and coverage on every segment between two, both over
    /// the pool's own buffers and with the segment's top from a separate
    /// copy.
    #[test]
    fn deep_queries_cover_exactly(case in arb_query_case()) {
        assert_case(&case)?;
    }
}

// ─────────────────────────────── floors ───────────────────────────────

/// The floor census seed: names the sampled corpus (see
/// [`crate::testing::rng::strategy_rng`]).
///
/// Fixed because the census measures its generators
/// deterministically: a regime it counts has no mass derived from the
/// generators' weights, so its floors hold for this corpus alone.
const FLOOR_SEED: u64 = 17;

/// The depth past which a site or a version counts as deep: a cell there is
/// narrower than a 32-bit fraction can resolve.
const DEEP_DEPTH: u64 = 32;

/// Samples `strategy` [`CENSUS_CASES`] times under [`FLOOR_SEED`].
fn floor_corpus<T>(strategy: impl Strategy<Value = T>) -> Vec<T> {
    sample(&strategy, FLOOR_SEED).collect()
}

/// What a case reaches: the holes production stores in its downward and
/// upward queries, and whether its pool is deep and wide.
struct Reach {
    /// The holes the downward query stores.
    down_holes: usize,
    /// The holes the upward query stores.
    up_holes: usize,
    /// Some site lies deeper than [`DEEP_DEPTH`].
    deep_site: bool,
    /// Some pool version's canonical depth exceeds [`DEEP_DEPTH`].
    deep_version: bool,
    /// Some pool height is at least `2^64`.
    wide: bool,
}

/// Measures a case's reach.
///
/// Production stores a query's holes as an antichain, so the holes it
/// stores are pairwise concurrent; the count reads them through `Debug`,
/// the one structural window [`Query`] offers.
fn reach(case: &QueryCase) -> Reach {
    let versions: Vec<Version> = case.pool.iter().map(FlatVersion::production).collect();
    let (_, down, up) = queries(case, &versions);
    let two64 = BigUint::from(1u8) << 64u8;
    let rows = || case.pool.iter().flat_map(FlatVersion::rows);
    Reach {
        down_holes: rendered_holes(&down),
        up_holes: rendered_holes(&up),
        deep_site: case.site_depths.iter().any(|&depth| depth > DEEP_DEPTH),
        deep_version: rows().any(|leaf| leaf.depth > DEEP_DEPTH),
        wide: rows().any(|leaf| leaf.height >= two64),
    }
}

/// Each census property's strategy keeps enough mass on the regimes its docs
/// claim that each default run expects
/// [`REGIME_CASES_PER_RUN`](crate::testing::generators::census::seeded::REGIME_CASES_PER_RUN)
/// cases there.
///
/// [`census_agrees_with_a_finer_pool`] claims the segments that can expose a
/// census missing its witness: for each query, a segment that rejects both
/// its endpoints yet holds a version the query admits.
///
/// [`antichain_holes_cover_exactly`] claims antichains of three or more
/// holes on deep cells, beside wide heights: its downward and its upward
/// query each store three or more holes, on a pool whose sites reach past
/// depth 32 and whose heights reach `2^64`. [`deep_queries_cover_exactly`]
/// claims deep, wide pools and holed queries of both polarities: a pool
/// version past depth 32 beside a height of `2^64` or more, and downward
/// and upward queries that each store a hole. Each floor follows from that
/// premise and its property's case count.
#[test]
fn census_cases_keep_mass_on_their_regimes() {
    assert_default_case_count();
    let antichains: Vec<Reach> = floor_corpus(arb_antichain_case())
        .iter()
        .map(reach)
        .collect();
    let queries: Vec<Reach> = floor_corpus(arb_query_case()).iter().map(reach).collect();
    let grids: Vec<(FinerPool, Clauses)> =
        floor_corpus((arb_small_grid(), arb_clauses(GRID_PICKS, 3, 3)))
            .into_iter()
            .map(|(pool, clauses)| (finer_pool(&pool), clauses))
            .collect();
    let count = |corpus: &[Reach], in_regime: fn(&Reach) -> bool| {
        corpus.iter().filter(|reach| in_regime(reach)).count()
    };
    let with_interior = |which: Which| {
        grids
            .iter()
            .filter(|(finer, clauses)| has_interior_witness(finer, clauses, which))
            .count()
    };
    let validator_floor = regime_floor(CENSUS_CASES, VALIDATOR_DIVISOR);
    let antichain_floor = regime_floor(CENSUS_CASES, ANTICHAIN_DIVISOR);
    let query_floor = regime_floor(CENSUS_CASES, QUERY_DIVISOR);
    let tallies = [
        Tally::new(
            "census check cases: a segment whose neutral query rejects both ends but admits inside",
            with_interior(Which::Neutral),
            validator_floor,
        ),
        Tally::new(
            "census check cases: a segment whose downward query rejects both ends but admits inside",
            with_interior(Which::Downward),
            validator_floor,
        ),
        Tally::new(
            "census check cases: a segment whose upward query rejects both ends but admits inside",
            with_interior(Which::Upward),
            validator_floor,
        ),
        Tally::new(
            "antichain cases: three or more downward holes, a deep site, a wide height",
            count(&antichains, |r| r.down_holes >= 3 && r.deep_site && r.wide),
            antichain_floor,
        ),
        Tally::new(
            "antichain cases: three or more upward holes, a deep site, a wide height",
            count(&antichains, |r| r.up_holes >= 3 && r.deep_site && r.wide),
            antichain_floor,
        ),
        Tally::new(
            "query cases: a version past depth 32 and a wide height",
            count(&queries, |r| r.deep_version && r.wide),
            query_floor,
        ),
        Tally::new(
            "query cases: a downward hole",
            count(&queries, |r| r.down_holes >= 1),
            query_floor,
        ),
        Tally::new(
            "query cases: an upward hole",
            count(&queries, |r| r.up_holes >= 1),
            query_floor,
        ),
    ];
    assert_floors(
        &tallies,
        "an arm has gone dead or the bumps no longer form an antichain",
    );
}
