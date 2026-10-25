use super::*;
use crate::testing::bridge::from_oracle_version;
use crate::testing::exhaustive::{cell_event, cell_heights};
use crate::testing::generators::arb_oracle_version;
use crate::testing::oracles::flat::census::Heights;
use crate::{Clock, Span, Version};
use core::ops::Range;
use proptest::prelude::*;
use proptest::test_runner::TestCaseError;
use std::sync::LazyLock;

mod census;

/// The organic witness set: a three-step chain on one party, a
/// concurrent line on a second, and their join.
struct Witnesses {
    bottom: Version,
    a1: Version,
    a2: Version,
    a3: Version,
    b1: Version,
    joined: Version,
}

fn witnesses() -> Witnesses {
    let mut alice = Clock::seed();
    let mut bob = alice.fork();
    let a1 = alice.tick().clone();
    let a2 = alice.tick().clone();
    let a3 = alice.tick().clone();
    let b1 = bob.tick().clone();
    let joined = &a3 | &b1;
    Witnesses {
        bottom: Version::new(),
        a1,
        a2,
        a3,
        b1,
        joined,
    }
}

/// Builds the grid of versions in which each of two forked parties has ticked
/// zero, one, or two times.
///
/// The grid is not the whole interval from the empty version to its top: that
/// interval also holds canonical versions off the grid, such as one in which
/// one party's identity, split in two, has ticked twice on one half and once
/// on the other. A census over the grid still decides coverage exactly for
/// queries anchored on it, because the grid holds a witness for every verdict
/// a segment between grid versions supports;
/// `grid_census_agrees_with_a_finer_pool` checks that premise against every
/// version the segments cover, up to the clauses' verdicts.
fn two_party_grid() -> Vec<Version> {
    let mut alice = Clock::seed();
    let mut bob = alice.fork();
    let a1 = alice.tick().clone();
    let a2 = alice.tick().clone();
    let b1 = bob.tick().clone();
    let b2 = bob.tick().clone();
    let a = [None, Some(&a1), Some(&a2)];
    let b = [None, Some(&b1), Some(&b2)];

    a.iter()
        .flat_map(|a| b.iter().map(move |b| (a, b)))
        .map(|(a, b)| match (a, b) {
            (None, None) => Version::new(),
            (Some(a), None) => (*a).clone(),
            (None, Some(b)) => (*b).clone(),
            (Some(a), Some(b)) => *a | *b,
        })
        .collect()
}

/// One inclusive bound that can be conjoined with a query of any polarity.
#[derive(Clone, Debug)]
enum BoundClause {
    After(usize),
    Before(usize),
}

/// Selects a generated anchor while allowing one recipe to run against
/// differently sized pools.
fn selected<T>(grid: &[T], index: usize) -> &T {
    &grid[index % grid.len()]
}

/// The order a clause is evaluated in.
///
/// Versions use production's causal order. Height vectors use the pointwise
/// order, which equals the causal order on the versions they describe and
/// shares no code with production: the finer-pool tests read the heights of
/// versions constant on each cell of one shared partition, and the census
/// tests read height vectors off the flat model.
trait CausalOrder {
    /// Whether `self` lies at or below `other`.
    fn le(&self, other: &Self) -> bool;

    /// Whether `self` lies strictly below `other`.
    fn lt(&self, other: &Self) -> bool;
}

/// Production's causal order.
impl CausalOrder for Version {
    fn le(&self, other: &Self) -> bool {
        self <= other
    }

    fn lt(&self, other: &Self) -> bool {
        self < other
    }
}

/// The pointwise order on height vectors over one partition, which equals
/// the causal order of the versions they describe.
impl CausalOrder for Heights {
    fn le(&self, other: &Self) -> bool {
        Heights::le(self, other)
    }

    fn lt(&self, other: &Self) -> bool {
        Heights::le(self, other) && self != other
    }
}

/// Whether bit `k` of `sides` puts the `k`th conjoined item on the left of
/// the conjunction so far, rather than on its right.
fn on_left(sides: u64, k: usize) -> bool {
    sides >> (k % 64) & 1 == 1
}

/// The bit of `sides` that makes [`down_query`] and [`up_query`] conjoin their
/// bounds as one neutral query instead of one atom at a time, so the typed
/// `&` between a neutral query and a polar one occurs.
///
/// The polar fold reads bits from zero upward, one per later form and per
/// bound, so it reaches this bit and [`NEUTRAL_BLOCK_SIDE`] only for
/// expressions far longer than the strategies build.
const NEUTRAL_BLOCK: usize = 63;

/// The bit of `sides` that puts the neutral block of [`NEUTRAL_BLOCK`] on the
/// left of the polar query, rather than on its right.
const NEUTRAL_BLOCK_SIDE: usize = 62;

/// How far `sides` shifts before [`neutral_query`] reads the neutral block's
/// own operand orders, so they vary independently of the polar fold's.
const NEUTRAL_BLOCK_ORDERS: u32 = 32;

impl BoundClause {
    /// Conjoins this bound with a neutral query.
    fn add_to_neutral<'a>(&self, query: Query<'a>, grid: &'a [Version]) -> Query<'a> {
        match *self {
            Self::After(at) => query & after(selected(grid, at)),
            Self::Before(at) => query & before(selected(grid, at)),
        }
    }

    /// Conjoins this bound with a downward-polar query.
    fn add_to_down<'a>(&self, query: Query<'a, Down>, grid: &'a [Version]) -> Query<'a, Down> {
        match *self {
            Self::After(at) => query & after(selected(grid, at)),
            Self::Before(at) => query & before(selected(grid, at)),
        }
    }

    /// Conjoins this bound with an upward-polar query.
    fn add_to_up<'a>(&self, query: Query<'a, Up>, grid: &'a [Version]) -> Query<'a, Up> {
        match *self {
            Self::After(at) => query & after(selected(grid, at)),
            Self::Before(at) => query & before(selected(grid, at)),
        }
    }

    /// Conjoins this bound in front of a neutral query, the operand order
    /// [`Self::add_to_neutral`] reverses.
    fn prepend_to_neutral<'a>(&self, query: Query<'a>, grid: &'a [Version]) -> Query<'a> {
        match *self {
            Self::After(at) => after(selected(grid, at)) & query,
            Self::Before(at) => before(selected(grid, at)) & query,
        }
    }

    /// Conjoins this bound in front of a downward-polar query, the operand
    /// order [`Self::add_to_down`] reverses.
    fn prepend_to_down<'a>(&self, query: Query<'a, Down>, grid: &'a [Version]) -> Query<'a, Down> {
        match *self {
            Self::After(at) => after(selected(grid, at)) & query,
            Self::Before(at) => before(selected(grid, at)) & query,
        }
    }

    /// Conjoins this bound in front of an upward-polar query, the operand
    /// order [`Self::add_to_up`] reverses.
    fn prepend_to_up<'a>(&self, query: Query<'a, Up>, grid: &'a [Version]) -> Query<'a, Up> {
        match *self {
            Self::After(at) => after(selected(grid, at)) & query,
            Self::Before(at) => before(selected(grid, at)) & query,
        }
    }

    /// The neutral query admitting what this bound admits.
    fn query<'a>(&self, grid: &'a [Version]) -> Query<'a> {
        match *self {
            Self::After(at) => after(selected(grid, at)).into(),
            Self::Before(at) => before(selected(grid, at)).into(),
        }
    }

    /// Conjoins this bound with another as atoms, `self & other`, without
    /// first lifting either into a query.
    fn conjoin_atoms<'a>(&self, other: &Self, grid: &'a [Version]) -> Query<'a> {
        let at = |index| selected(grid, index);
        match (self, other) {
            (Self::After(a), Self::After(b)) => (after(at(*a)) & after(at(*b))).into(),
            (Self::After(a), Self::Before(b)) => after(at(*a)) & before(at(*b)),
            (Self::Before(a), Self::After(b)) => before(at(*a)) & after(at(*b)),
            (Self::Before(a), Self::Before(b)) => (before(at(*a)) & before(at(*b))).into(),
        }
    }

    /// Evaluates this clause directly from the order.
    fn admits<T: CausalOrder>(&self, probe: &T, grid: &[T]) -> bool {
        match *self {
            Self::After(at) => selected(grid, at).le(probe),
            Self::Before(at) => probe.le(selected(grid, at)),
        }
    }
}

/// Generates bounds at every version of the two-party grid.
fn bound_clause() -> impl Strategy<Value = BoundClause> {
    bound_clause_among(0..9)
}

/// Generates bounds anchored at the pool indices `anchors` draws, which
/// [`selected`] reduces modulo the pool's size.
fn bound_clause_among(anchors: Range<usize>) -> impl Strategy<Value = BoundClause> {
    prop_oneof![
        anchors.clone().prop_map(BoundClause::After),
        anchors.prop_map(BoundClause::Before),
    ]
}

/// One public downward-polar query form.
///
/// `Since` and `NotBefore` denote one predicate in its two public
/// spellings, `since(v)` and `!before(v)`.
#[derive(Clone, Debug)]
enum DownClause {
    Since(usize),
    NotBefore(usize),
    StrictlyAfter(usize),
    AfterOrConcurrent(usize),
    Delta(usize, usize),
}

impl DownClause {
    /// Constructs this clause through the public query vocabulary.
    fn query<'a>(&self, grid: &'a [Version]) -> Query<'a, Down> {
        match *self {
            Self::Since(at) => since(selected(grid, at)),
            Self::NotBefore(at) => !before(selected(grid, at)),
            Self::StrictlyAfter(at) => strictly_after(selected(grid, at)),
            Self::AfterOrConcurrent(at) => after(selected(grid, at)).or_concurrent(),
            Self::Delta(start, end) => delta(selected(grid, start), selected(grid, end)),
        }
    }

    /// Evaluates this clause directly from the order.
    fn admits<T: CausalOrder>(&self, probe: &T, grid: &[T]) -> bool {
        match *self {
            Self::Since(at) | Self::NotBefore(at) => {
                let at_or_before = probe.le(selected(grid, at));
                !at_or_before
            }
            Self::StrictlyAfter(at) => selected(grid, at).lt(probe),
            Self::AfterOrConcurrent(at) => {
                let strictly_before = probe.lt(selected(grid, at));
                !strictly_before
            }
            Self::Delta(start, end) => {
                let at_or_before_start = probe.le(selected(grid, start));
                !at_or_before_start && probe.le(selected(grid, end))
            }
        }
    }
}

/// Generates every public downward-polar query form across the grid.
fn down_clause() -> impl Strategy<Value = DownClause> {
    down_clause_among(0..9)
}

/// Generates every public downward-polar query form, anchored at the pool
/// indices `anchors` draws.
fn down_clause_among(anchors: Range<usize>) -> impl Strategy<Value = DownClause> {
    prop_oneof![
        anchors.clone().prop_map(DownClause::Since),
        anchors.clone().prop_map(DownClause::NotBefore),
        anchors.clone().prop_map(DownClause::StrictlyAfter),
        anchors.clone().prop_map(DownClause::AfterOrConcurrent),
        (anchors.clone(), anchors).prop_map(|(start, end)| DownClause::Delta(start, end)),
    ]
}

/// One public upward-polar query form.
///
/// `Until` and `NotAfter` denote one predicate in its two public spellings,
/// `until(v)` and `!after(v)`.
#[derive(Clone, Debug)]
enum UpClause {
    Until(usize),
    NotAfter(usize),
    StrictlyBefore(usize),
    BeforeOrConcurrent(usize),
    Toward(usize, usize),
}

impl UpClause {
    /// Constructs this clause through the public query vocabulary.
    fn query<'a>(&self, grid: &'a [Version]) -> Query<'a, Up> {
        match *self {
            Self::Until(at) => until(selected(grid, at)),
            Self::NotAfter(at) => !after(selected(grid, at)),
            Self::StrictlyBefore(at) => strictly_before(selected(grid, at)),
            Self::BeforeOrConcurrent(at) => before(selected(grid, at)).or_concurrent(),
            Self::Toward(start, end) => toward(selected(grid, start), selected(grid, end)),
        }
    }

    /// Evaluates this clause directly from the order.
    fn admits<T: CausalOrder>(&self, probe: &T, grid: &[T]) -> bool {
        match *self {
            Self::Until(at) | Self::NotAfter(at) => {
                let at_or_after = selected(grid, at).le(probe);
                !at_or_after
            }
            Self::StrictlyBefore(at) => probe.lt(selected(grid, at)),
            Self::BeforeOrConcurrent(at) => {
                let strictly_after = selected(grid, at).lt(probe);
                !strictly_after
            }
            Self::Toward(start, end) => {
                let at_or_after_end = selected(grid, end).le(probe);
                selected(grid, start).le(probe) && !at_or_after_end
            }
        }
    }
}

/// Generates every public upward-polar query form across the grid.
fn up_clause() -> impl Strategy<Value = UpClause> {
    up_clause_among(0..9)
}

/// Generates every public upward-polar query form, anchored at the pool
/// indices `anchors` draws.
fn up_clause_among(anchors: Range<usize>) -> impl Strategy<Value = UpClause> {
    prop_oneof![
        anchors.clone().prop_map(UpClause::Until),
        anchors.clone().prop_map(UpClause::NotAfter),
        anchors.clone().prop_map(UpClause::StrictlyBefore),
        anchors.clone().prop_map(UpClause::BeforeOrConcurrent),
        (anchors.clone(), anchors).prop_map(|(start, end)| UpClause::Toward(start, end)),
    ]
}

/// Classifies a segment by the admission verdicts of the versions it covers:
/// `Full` when all admit, `Empty` when none do, and `Partial` otherwise.
///
/// The verdicts must include the segment's endpoints, so they are never
/// empty.
fn coverage_of(mut admitted: impl Iterator<Item = bool>) -> Coverage {
    let first = admitted
        .next()
        .expect("every valid segment covers its endpoints");
    if admitted.all(|next| next == first) {
        if first {
            Coverage::Full
        } else {
            Coverage::Empty
        }
    } else {
        Coverage::Partial
    }
}

/// The census coverage of the segment from `lo` to `hi`: the admission
/// verdicts of exactly the pool's versions inside it, classified by
/// [`coverage_of`].
///
/// The census equals the exact coverage only when the pool holds a witness for
/// each verdict the whole segment supports: an admitted version whenever any
/// version in the segment admits, and a rejected one whenever any rejects. The
/// exhaustive coverage tests rest on that premise for their pools, and the
/// `*_census_agrees_with_a_finer_pool` tests check it.
fn pool_coverage(
    pool: &[Version],
    lo: &Version,
    hi: &Version,
    admits: impl Fn(&Version) -> bool,
) -> Coverage {
    coverage_of(
        pool.iter()
            .filter(|probe| lo <= *probe && *probe <= hi)
            .map(admits),
    )
}

/// Checks membership at every grid version, and coverage of every span between
/// two of them, against an independent predicate.
fn assert_denotes<P: Polarity>(
    query: &Query<'_, P>,
    grid: &[Version],
    admits: impl Fn(&Version) -> bool,
) -> Result<(), TestCaseError> {
    for probe in grid {
        prop_assert_eq!(
            query.contains(probe),
            admits(probe),
            "membership for {:?} at {:?}",
            query,
            probe,
        );
    }

    for lo in grid {
        for hi in grid {
            let Ok(span) = Span::new(lo, hi) else {
                continue;
            };
            let expected = pool_coverage(grid, lo, hi, &admits);
            prop_assert_eq!(
                query.coverage(span),
                expected,
                "coverage for {:?} over [{:?}, {:?}]",
                query,
                lo,
                hi,
            );
        }
    }
    Ok(())
}

/// Builds a neutral conjunction from public inclusive bounds, starting from
/// `all()`.
///
/// Bound `k` joins the conjunction on the side [`on_left`] reads from bit
/// `k` of `sides`, so both operand orders of each typed `&` occur.
fn neutral_query<'a>(bounds: &[BoundClause], sides: u64, versions: &'a [Version]) -> Query<'a> {
    bounds.iter().enumerate().fold(all(), |query, (k, bound)| {
        if on_left(sides, k) {
            bound.prepend_to_neutral(query, versions)
        } else {
            bound.add_to_neutral(query, versions)
        }
    })
}

/// Builds a downward-polar conjunction from public forms and inclusive bounds.
///
/// The first form seeds the conjunction. Each later form joins it on the side
/// [`on_left`] reads from the next bit of `sides`, so both operand orders of
/// each typed `&` occur. The bounds then join one of two ways, as bit
/// [`NEUTRAL_BLOCK`] of `sides` picks: each bound as an atom, on the side the
/// next bit picks, or all of them as one [`neutral_query`], on the side bit
/// [`NEUTRAL_BLOCK_SIDE`] picks.
fn down_query<'a>(
    clauses: &[DownClause],
    bounds: &[BoundClause],
    sides: u64,
    versions: &'a [Version],
) -> Query<'a, Down> {
    let (first, rest) = clauses
        .split_first()
        .expect("the strategy always generates a downward clause");
    let query = rest
        .iter()
        .enumerate()
        .fold(first.query(versions), |query, (k, clause)| {
            if on_left(sides, k) {
                clause.query(versions) & query
            } else {
                query & clause.query(versions)
            }
        });
    if on_left(sides, NEUTRAL_BLOCK) {
        let block = neutral_query(bounds, sides >> NEUTRAL_BLOCK_ORDERS, versions);
        return if on_left(sides, NEUTRAL_BLOCK_SIDE) {
            block & query
        } else {
            query & block
        };
    }
    bounds.iter().enumerate().fold(query, |query, (k, bound)| {
        if on_left(sides, rest.len() + k) {
            bound.prepend_to_down(query, versions)
        } else {
            bound.add_to_down(query, versions)
        }
    })
}

/// Builds an upward-polar conjunction from public forms and inclusive bounds.
///
/// The forms and the bounds join as they do in [`down_query`].
fn up_query<'a>(
    clauses: &[UpClause],
    bounds: &[BoundClause],
    sides: u64,
    versions: &'a [Version],
) -> Query<'a, Up> {
    let (first, rest) = clauses
        .split_first()
        .expect("the strategy always generates an upward clause");
    let query = rest
        .iter()
        .enumerate()
        .fold(first.query(versions), |query, (k, clause)| {
            if on_left(sides, k) {
                clause.query(versions) & query
            } else {
                query & clause.query(versions)
            }
        });
    if on_left(sides, NEUTRAL_BLOCK) {
        let block = neutral_query(bounds, sides >> NEUTRAL_BLOCK_ORDERS, versions);
        return if on_left(sides, NEUTRAL_BLOCK_SIDE) {
            block & query
        } else {
            query & block
        };
    }
    bounds.iter().enumerate().fold(query, |query, (k, bound)| {
        if on_left(sides, rest.len() + k) {
            bound.prepend_to_up(query, versions)
        } else {
            bound.add_to_up(query, versions)
        }
    })
}

proptest! {
    /// Atom coverage is exactly the coverage of the equivalent one-bound
    /// query for arbitrary bounds and spans.
    #[test]
    fn atom_coverage_matches_query_coverage(
        bound in arb_oracle_version(),
        a in arb_oracle_version(),
        b in arb_oracle_version(),
    ) {
        let bound = from_oracle_version(&bound);
        let a = from_oracle_version(&a);
        let b = from_oracle_version(&b);
        let span = a.span(&b);

        let floor = after(&bound);
        prop_assert_eq!(
            floor.coverage(span.reborrow()),
            Query::from(floor).coverage(span.reborrow()),
        );

        let ceiling = before(&bound);
        prop_assert_eq!(
            ceiling.coverage(span.reborrow()),
            Query::from(ceiling).coverage(span),
        );
    }

    /// Public query expressions denote the conjunction of their causal
    /// relations, independently of normalization, operand order, and fused
    /// evaluation.
    ///
    /// Each expression joins every operand after its first on the side
    /// `sides` picks, joins a polar expression's bounds either one at a time
    /// or as one neutral query, and spells each negation in either public
    /// form (`since(v)` or `!before(v)`, `until(v)` or `!after(v)`). Every case
    /// first exhausts the two-party grid, on which membership is enumerable
    /// and a census decides coverage exactly (see `two_party_grid`). The same
    /// expression is then applied to arbitrary normal-form versions whose
    /// shapes and numeric magnitudes cross the skyline walk's representation
    /// thresholds; those versions and their lattice corners exercise
    /// membership beyond the grid without pretending that a sparse sample
    /// proves span coverage.
    #[test]
    fn public_queries_match_their_relational_denotation(
        bounds in prop::collection::vec(bound_clause(), 0..=6),
        down in prop::collection::vec(down_clause(), 1..=6),
        up in prop::collection::vec(up_clause(), 1..=6),
        sides in any::<u64>(),
        a in arb_oracle_version(),
        b in arb_oracle_version(),
        c in arb_oracle_version(),
    ) {
        let small = two_party_grid();
        let neutral = neutral_query(&bounds, sides, &small);
        assert_denotes(&neutral, &small, |probe| {
            bounds.iter().all(|bound| bound.admits(probe, &small))
        })?;
        let small_down = down_query(&down, &bounds, sides, &small);
        assert_denotes(&small_down, &small, |probe| {
            bounds.iter().all(|bound| bound.admits(probe, &small))
                && down.iter().all(|clause| clause.admits(probe, &small))
        })?;
        let small_up = up_query(&up, &bounds, sides, &small);
        assert_denotes(&small_up, &small, |probe| {
            bounds.iter().all(|bound| bound.admits(probe, &small))
                && up.iter().all(|clause| clause.admits(probe, &small))
        })?;

        let a = from_oracle_version(&a);
        let b = from_oracle_version(&b);
        let c = from_oracle_version(&c);
        let mut large = vec![a, b, c];
        large.push(&large[0] & &large[1]);
        large.push(&large[0] | &large[1]);
        large.push(&large[1] & &large[2]);
        large.push(&large[1] | &large[2]);

        let neutral = neutral_query(&bounds, sides, &large);
        let down_query = down_query(&down, &bounds, sides, &large);
        let up_query = up_query(&up, &bounds, sides, &large);
        for probe in &large {
            prop_assert_eq!(
                neutral.contains(probe),
                bounds.iter().all(|bound| bound.admits(probe, &large)),
                "large neutral expression {:?} at {:?}",
                neutral,
                probe,
            );
            prop_assert_eq!(
                down_query.contains(probe),
                bounds.iter().all(|bound| bound.admits(probe, &large))
                    && down.iter().all(|clause| clause.admits(probe, &large)),
                "large downward expression {:?} at {:?}",
                down_query,
                probe,
            );
            prop_assert_eq!(
                up_query.contains(probe),
                bounds.iter().all(|bound| bound.admits(probe, &large))
                    && up.iter().all(|clause| clause.admits(probe, &large)),
                "large upward expression {:?} at {:?}",
                up_query,
                probe,
            );
        }
    }
}

/// The number of holes in a query's debug output.
fn rendered_holes<P: Polarity>(q: &Query<'_, P>) -> usize {
    format!("{q:?}").matches('!').count()
}

/// Every atom, negation, widening, and strict form keeps exactly its
/// advertised relation on an organic witness set: the point verdicts
/// the quantified laws sweep, pinned here against named versions.
#[test]
fn forms_keep_their_relations() {
    let w = witnesses();
    // The inclusive atoms demand the relation; concurrency fails it.
    assert!(after(&w.a1).contains(&w.a2));
    assert!(after(&w.a1).contains(&w.a1));
    assert!(!after(&w.a1).contains(&w.b1));
    assert!(before(&w.a2).contains(&w.a1));
    assert!(before(&w.a2).contains(&w.a2));
    assert!(!before(&w.a2).contains(&w.b1));
    // The strict forms exclude exactly their bound.
    assert!(strictly_after(&w.a1).contains(&w.a2));
    assert!(!strictly_after(&w.a1).contains(&w.a1));
    assert!(!strictly_after(&w.a1).contains(&w.b1));
    assert!(strictly_before(&w.a2).contains(&w.a1));
    assert!(!strictly_before(&w.a2).contains(&w.a2));
    assert!(!strictly_before(&w.a2).contains(&w.b1));
    // Negation keeps the complement: the other side and concurrency.
    assert!((!before(&w.a1)).contains(&w.a2));
    assert!((!before(&w.a1)).contains(&w.b1));
    assert!(!(!before(&w.a1)).contains(&w.a1));
    assert!((!after(&w.a2)).contains(&w.a1));
    assert!((!after(&w.a2)).contains(&w.b1));
    assert!(!(!after(&w.a2)).contains(&w.a2));
    // The named negations spell the same complements.
    assert!(since(&w.a1).contains(&w.b1));
    assert!(!since(&w.a1).contains(&w.a1));
    assert!(until(&w.a2).contains(&w.b1));
    assert!(!until(&w.a2).contains(&w.a2));
    // The frontier form: reached `a1`, not yet `a3`.
    assert!(toward(&w.a1, &w.a3).contains(&w.a2));
    assert!(toward(&w.a1, &w.a3).contains(&w.a1));
    assert!(!toward(&w.a1, &w.a3).contains(&w.a3));
    assert!(!toward(&w.a1, &w.a3).contains(&w.b1));
    // Widening keeps the relation and adds concurrency.
    assert!(after(&w.a1).or_concurrent().contains(&w.a1));
    assert!(after(&w.a1).or_concurrent().contains(&w.b1));
    assert!(!after(&w.a1).or_concurrent().contains(&w.bottom));
    assert!(before(&w.a1).or_concurrent().contains(&w.a1));
    assert!(before(&w.a1).or_concurrent().contains(&w.b1));
    assert!(!before(&w.a1).or_concurrent().contains(&w.a2));
}

/// Conjunction normalizes to the collapse laws.
///
/// Elementary bounds join and meet, comparable holes absorb, holes
/// the merged bounds avoid are pruned, and strictness dissolves
/// across concurrent bounds, because the join sits strictly above
/// both. The hole census is read through the `Debug` window, the
/// module's one structural surface.
#[test]
fn conjunction_normalizes() {
    let w = witnesses();
    // Elementary floors collapse to their join.
    let floors = after(&w.a1) & after(&w.b1);
    assert!(floors.contains(&w.joined));
    assert!(!floors.contains(&w.a3));
    assert!(!floors.contains(&w.b1));
    // Strictness survives a comparable merge…
    let strict = strictly_after(&w.a1) & strictly_after(&w.a2);
    assert!(!strict.contains(&w.a2));
    assert!(strict.contains(&w.a3));
    assert_eq!(rendered_holes(&strict), 1);
    // …and dissolves across concurrent bounds: both holes prune as
    // vacuous under the joined floor, so the join itself is admitted.
    let dissolved = strictly_after(&w.a3) & strictly_after(&w.b1);
    assert!(dissolved.contains(&w.joined));
    assert_eq!(rendered_holes(&dissolved), 0);
    // Comparable holes absorb into the larger.
    let absorbed = since(&w.a1) & since(&w.a2);
    assert_eq!(rendered_holes(&absorbed), 1);
    assert!(!absorbed.contains(&w.a2));
    assert!(absorbed.contains(&w.a3));
    // A hole the floor already avoids is pruned.
    let pruned = after(&w.a2) & since(&w.a1);
    assert_eq!(rendered_holes(&pruned), 0);
    assert!(pruned.contains(&w.a2));
    // Conjunction with self re-absorbs instead of accumulating.
    let q = delta(&w.a1, &w.a3);
    assert_eq!(rendered_holes(&(q.clone() & q)), 1);
    // Incomparable holes form an antichain: both stored, both firing.
    let antichain = since(&w.a3) & since(&w.b1);
    assert_eq!(rendered_holes(&antichain), 2);
    assert!(antichain.contains(&w.joined));
    assert!(!antichain.contains(&w.a2));
    assert!(!antichain.contains(&w.b1));
}

/// Coverage's three verdicts on an organic witness matrix: full,
/// empty (by floor, by ceiling, and by hole), and genuinely mixed,
/// with the coincident span degenerating to membership.
#[test]
fn coverage_witness_matrix() {
    let w = witnesses();
    let span = w.a1.span(&w.a3);
    // No constraints: everything covered.
    assert_eq!(all().coverage(span.reborrow()), Coverage::Full);
    // A hole beside the whole segment subtracts nothing from it…
    assert_eq!(since(&w.b1).coverage(span.reborrow()), Coverage::Full);
    // …one straddling it splits it…
    assert_eq!(since(&w.a1).coverage(span.reborrow()), Coverage::Partial);
    // …and one holding its top swallows it at exhaustion.
    assert_eq!(since(&w.a3).coverage(span.reborrow()), Coverage::Empty);
    // A floor above (or beside) the whole segment: the early bail.
    assert_eq!(
        Query::from(after(&w.joined)).coverage(span.reborrow()),
        Coverage::Empty
    );
    assert_eq!(
        Query::from(after(&w.b1)).coverage(span.reborrow()),
        Coverage::Empty
    );
    // A ceiling beside the segment, dually.
    assert_eq!(
        Query::from(before(&w.b1)).coverage(span.reborrow()),
        Coverage::Empty
    );
    // Straddling bounds read mixed.
    assert_eq!(
        Query::from(after(&w.a2)).coverage(span.reborrow()),
        Coverage::Partial
    );
    assert_eq!(
        Query::from(before(&w.a2)).coverage(span.reborrow()),
        Coverage::Partial
    );
    // The coincident span is membership: one verdict per bound.
    for v in [&w.a1, &w.a2, &w.b1] {
        let q = delta(&w.a1, &w.a3);
        let want = if q.contains(v) {
            Coverage::Full
        } else {
            Coverage::Empty
        };
        assert_eq!(q.coverage(v), want);
        assert_eq!(q.coverage(Span::at(v)), want);
    }
}

/// The clamp refinement decides emptiness the endpoint folds alone
/// cannot see.
///
/// A crossed clamp (floor and ceiling each straddling the segment but
/// jointly empty) and a hole holding the clamped top both read
/// `Empty`, exactly.
#[test]
fn coverage_clamp_refinement_is_exact() {
    let mut alice = Clock::seed();
    let mut bob = alice.fork();
    let a1 = alice.tick().clone();
    let a2 = alice.tick().clone();
    let b1 = bob.tick().clone();
    let b2 = bob.tick().clone();
    let a1b1 = &a1 | &b1;
    let a2b1 = &a2 | &b1;
    let a1b2 = &a1 | &b2;

    // Floor and ceiling each straddle [a1, a2b1] (the fused folds
    // alone read mixed), but nothing is at once above a2 and within
    // a1b1: the clamp crosses.
    let crossed = after(&a2) & before(&a1b1);
    let span = a1.span(&a2b1);
    assert_eq!(crossed.coverage(span.reborrow()), Coverage::Empty);
    for probe in [&a1, &a2, &a1b1, &a2b1] {
        assert!(!crossed.contains(probe));
    }

    // The anti-entropy delta with concurrent bounds: everything in
    // [a1, a1b2] within the ceiling a2b1 is also within the hole
    // a1b1 (their meet), so the segment is jointly empty while both
    // bounds straddle it.
    let anti_entropy = delta(&a1b1, &a2b1);
    let span = a1.span(&a1b2);
    assert_eq!(anti_entropy.coverage(span.reborrow()), Coverage::Empty);
    for probe in [&a1, &a1b1, &a1b2] {
        assert!(!anti_entropy.contains(probe));
    }
}

/// Coverage is exact over the two-party grid.
///
/// `Full`, `Partial`, and `Empty` each hold iff the membership census over the
/// grid says so, for every version of the tick grid, every ordered segment of
/// it, and a query family covering both polarities, all hole spellings, and
/// their conjunctions. The family sets each polarity's hole beside a floor,
/// beside a ceiling, and beside both: the bound that clamps the hole's covering
/// endpoint (`delta` clamps a down-set's top by a ceiling, and `toward` clamps
/// an up-set's bottom by a floor), the bound that does not, and a floor and a
/// ceiling together, whose crossed clamp reads `Empty` whatever the holes
/// cover.
///
/// Every segment that holds an off-grid version, one the census never counts,
/// also checks production's verdict there: `Full` must admit it and `Empty`
/// must reject it. No other test judges a verdict at a version strictly inside
/// a segment and outside the pool its census counts.
#[test]
fn coverage_is_exact_on_the_two_party_grid() {
    // The grid of per-party tick heights zero to two, as `two_party_grid`
    // builds it, from this test's own seed so that the off-grid version below
    // shares its universe.
    let mut alice = Clock::seed();
    let mut bob = alice.fork();
    let a1 = alice.tick().clone();
    let a2 = alice.tick().clone();
    let b1 = bob.tick().clone();
    let b2 = bob.tick().clone();
    let grid: Vec<Version> = {
        let a = [None, Some(&a1), Some(&a2)];
        let b = [None, Some(&b1), Some(&b2)];
        a.iter()
            .flat_map(|a| b.iter().map(move |b| (a, b)))
            .map(|(a, b)| match (a, b) {
                (None, None) => Version::new(),
                (Some(a), None) => (*a).clone(),
                (None, Some(b)) => (*b).clone(),
                (Some(a), Some(b)) => *a | *b,
            })
            .collect()
    };

    // Alice's identity, split in two: one half ticks twice from the empty
    // version and the other once. The join lies strictly between `a1` and
    // `a2`, inside the grid's interval but off the grid.
    let (mut alice_left, _) = alice.into_parts();
    let alice_right = alice_left.fork();
    let mut twice = Version::new();
    alice_left.ticks(&mut twice, 2u8);
    let mut once = Version::new();
    alice_right.tick(&mut once);
    let off_grid = twice | once;
    assert!(
        a1 < off_grid && off_grid < a2 && !grid.contains(&off_grid),
        "{off_grid:?} lies strictly between a1 and a2, off the grid"
    );

    let a1b1 = &a1 | &b1;
    let anchors = [&a1, &b1, &a1b1];
    let mut down: Vec<Query<'_, Down>> = Vec::new();
    let mut up: Vec<Query<'_, Up>> = Vec::new();
    let mut neutral: Vec<Query<'_>> = vec![all()];
    for &x in &anchors {
        neutral.push(after(x).into());
        neutral.push(before(x).into());
        down.push(since(x));
        down.push(strictly_after(x));
        down.push(after(x).or_concurrent());
        up.push(!after(x));
        up.push(strictly_before(x));
        up.push(before(x).or_concurrent());
        for &y in &anchors {
            neutral.push(after(x) & before(y));
            down.push(since(x) & since(y));
            down.push(delta(x, y));
            down.push(after(x) & since(y));
            up.push(toward(x, y));
            up.push((!after(x)) & (!after(y)));
            up.push(before(x) & (!after(y)));
            for &z in &anchors {
                down.push(after(x) & delta(y, z));
                up.push(toward(x, y) & before(z));
            }
        }
    }

    /// Checks `q`'s verdict over `span` against the census, and, when the
    /// segment holds `off_grid`, its soundness at that version. `Partial`
    /// promises nothing about any one version.
    ///
    /// The census counts membership over the grid versions the segment
    /// covers. It is the exact coverage because the grid holds a witness for
    /// every verdict these queries can produce, a premise
    /// `grid_census_agrees_with_a_finer_pool` checks over the same clause
    /// forms.
    fn check<P: Polarity>(q: &Query<'_, P>, span: &Span<'_>, grid: &[Version], off_grid: &Version) {
        let (lo, hi) = (span.lo(), span.hi());
        let verdict = q.coverage(span.reborrow());
        assert_eq!(
            verdict,
            pool_coverage(grid, lo, hi, |v| q.contains(v)),
            "{q:?} over [{lo:?}, {hi:?}]"
        );
        if lo <= off_grid && off_grid <= hi {
            match verdict {
                Coverage::Full => assert!(
                    q.contains(off_grid),
                    "{q:?} is Full over [{lo:?}, {hi:?}] but rejects {off_grid:?}"
                ),
                Coverage::Empty => assert!(
                    !q.contains(off_grid),
                    "{q:?} is Empty over [{lo:?}, {hi:?}] but admits {off_grid:?}"
                ),
                Coverage::Partial => {}
            }
        }
    }

    let mut segments_holding_off_grid = 0;
    for lo in &grid {
        for hi in &grid {
            let Ok(span) = Span::new(lo, hi) else {
                continue;
            };
            if *lo <= off_grid && off_grid <= *hi {
                segments_holding_off_grid += 1;
            }
            for q in &neutral {
                check(q, &span, &grid, &off_grid);
            }
            for q in &down {
                check(q, &span, &grid, &off_grid);
            }
            for q in &up {
                check(q, &span, &grid, &off_grid);
            }
        }
    }
    assert!(
        segments_holding_off_grid > 0,
        "no grid segment holds the off-grid version, so its soundness check never ran"
    );
}

/// The boolean cube: the sixteen versions whose four quarter cells each hold
/// height 0 or 1, indexed so that bit `c` of a version's index is its height
/// on cell `c`.
fn boolean_cube() -> Vec<Version> {
    cell_heights(4, 1)
        .iter()
        .map(|heights| from_oracle_version(&cell_event(heights)))
        .collect()
}

/// One clause of the cube's query family: an inclusive bound, or a form of
/// one polarity.
#[derive(Clone, Debug)]
enum CubeClause {
    Bound(BoundClause),
    DownForm(DownClause),
    UpForm(UpClause),
}

impl CubeClause {
    /// Evaluates this clause directly from the order.
    fn admits<T: CausalOrder>(&self, probe: &T, cube: &[T]) -> bool {
        match self {
            Self::Bound(clause) => clause.admits(probe, cube),
            Self::DownForm(clause) => clause.admits(probe, cube),
            Self::UpForm(clause) => clause.admits(probe, cube),
        }
    }
}

/// Every clause the cube's family conjoins: each inclusive bound, and each
/// strict, negated, and widened form of either polarity, at every cube
/// version.
///
/// The two-anchor forms, `delta` and `toward`, stay out: their pairs would
/// multiply the family by sixteen, and the two-party grid tests cover them.
fn cube_clauses() -> Vec<CubeClause> {
    (0..16)
        .flat_map(|at| {
            [
                CubeClause::Bound(BoundClause::After(at)),
                CubeClause::Bound(BoundClause::Before(at)),
                CubeClause::DownForm(DownClause::Since(at)),
                CubeClause::DownForm(DownClause::StrictlyAfter(at)),
                CubeClause::DownForm(DownClause::AfterOrConcurrent(at)),
                CubeClause::UpForm(UpClause::Until(at)),
                CubeClause::UpForm(UpClause::StrictlyBefore(at)),
                CubeClause::UpForm(UpClause::BeforeOrConcurrent(at)),
            ]
        })
        .collect()
}

/// One query of the cube's family: a clause alone, or two clauses conjoined
/// in order, as indices into [`cube_clauses`].
#[derive(Clone, Copy, Debug)]
struct CubeQuery {
    first: usize,
    second: Option<usize>,
}

impl CubeQuery {
    /// Whether the query admits pool version `member`, given each clause's
    /// verdicts on the pool.
    fn admits(&self, table: &[Vec<bool>], member: usize) -> bool {
        table[self.first][member] && self.second.is_none_or(|second| table[second][member])
    }
}

/// Every clause alone, and every ordered pair of clauses the types let us
/// conjoin: any pair except a downward form with an upward one.
fn cube_queries(clauses: &[CubeClause]) -> Vec<CubeQuery> {
    let opposed = |a: &CubeClause, b: &CubeClause| {
        matches!(
            (a, b),
            (CubeClause::DownForm(_), CubeClause::UpForm(_))
                | (CubeClause::UpForm(_), CubeClause::DownForm(_))
        )
    };
    let singles = (0..clauses.len()).map(|first| CubeQuery {
        first,
        second: None,
    });
    let pairs = (0..clauses.len()).flat_map(|first| {
        (0..clauses.len())
            .filter(move |&second| !opposed(&clauses[first], &clauses[second]))
            .map(move |second| CubeQuery {
                first,
                second: Some(second),
            })
    });
    singles.chain(pairs).collect()
}

/// Each clause's verdict on each pool member, with the clauses anchored at
/// the cube's members.
fn admission_table<T: CausalOrder>(
    clauses: &[CubeClause],
    cube: &[T],
    pool: &[T],
) -> Vec<Vec<bool>> {
    clauses
        .iter()
        .map(|clause| {
            pool.iter()
                .map(|probe| clause.admits(probe, cube))
                .collect()
        })
        .collect()
}

/// Production's verdicts for one query of the cube's family: its membership
/// verdict on each cube version, and its coverage of each span.
fn cube_verdicts(
    query: CubeQuery,
    clauses: &[CubeClause],
    cube: &[Version],
    spans: &[Span<'_>],
) -> (Vec<bool>, Vec<Coverage>) {
    fn read<P: Polarity>(
        query: Query<'_, P>,
        cube: &[Version],
        spans: &[Span<'_>],
    ) -> (Vec<bool>, Vec<Coverage>) {
        let membership = cube.iter().map(|probe| query.contains(probe)).collect();
        let coverage = spans
            .iter()
            .map(|span| query.coverage(span.reborrow()))
            .collect();
        (membership, coverage)
    }

    use CubeClause::{Bound, DownForm, UpForm};
    let first = &clauses[query.first];
    let second = query.second.map(|second| &clauses[second]);
    match (first, second) {
        (Bound(a), None) => read(a.query(cube), cube, spans),
        (DownForm(a), None) => read(a.query(cube), cube, spans),
        (UpForm(a), None) => read(a.query(cube), cube, spans),
        (Bound(a), Some(Bound(b))) => read(a.conjoin_atoms(b, cube), cube, spans),
        (Bound(a), Some(DownForm(b))) => read(a.prepend_to_down(b.query(cube), cube), cube, spans),
        (DownForm(a), Some(Bound(b))) => read(b.add_to_down(a.query(cube), cube), cube, spans),
        (DownForm(a), Some(DownForm(b))) => read(a.query(cube) & b.query(cube), cube, spans),
        (Bound(a), Some(UpForm(b))) => read(a.prepend_to_up(b.query(cube), cube), cube, spans),
        (UpForm(a), Some(Bound(b))) => read(b.add_to_up(a.query(cube), cube), cube, spans),
        (UpForm(a), Some(UpForm(b))) => read(a.query(cube) & b.query(cube), cube, spans),
        (DownForm(_), Some(UpForm(_))) | (UpForm(_), Some(DownForm(_))) => {
            unreachable!("`cube_queries` never pairs a downward form with an upward one")
        }
    }
}

/// The valid segments of the cube, as `(lo, hi)` index pairs with `lo`'s
/// cells a subset of `hi`'s.
fn cube_segments() -> Vec<(usize, usize)> {
    (0..16)
        .flat_map(|lo| (0..16).map(move |hi| (lo, hi)))
        .filter(|&(lo, hi)| lo & !hi == 0)
        .collect()
}

/// Coverage is exact over the boolean cube.
///
/// The cube holds the sixteen versions whose four quarter cells each have
/// height 0 or 1, so its widest antichain has six versions, where the
/// two-party grid's has three. For every cube version, every one of the 81
/// segments between cube versions, and every query of the cube's family, the
/// test checks two things:
///
/// - membership agrees with the conjunction of the clauses' relations;
/// - `Full`, `Partial`, and `Empty` each hold exactly when the census over the
///   cube versions the segment covers says so.
///
/// The family is every bound, strict, negated, and widened clause alone, and
/// every ordered pair of them the polarity rules allow, so each pair is folded
/// from both sides.
///
/// The census is exact because every clause admits an up-closed or a
/// down-closed set and the cube is a sublattice: a segment's verdict is
/// decided by its endpoints and by the clauses' anchors clamped into it,
/// `(lo | anchor) & hi`, and the cube holds every such clamp. The test first
/// checks that production orders, joins, and meets the cube as the boolean
/// lattice on its cells, and `cube_census_agrees_with_a_finer_pool` checks the
/// census against a pool holding every version the segments cover, up to the
/// clauses' verdicts.
#[test]
fn coverage_is_exact_on_the_boolean_cube() {
    let cube = boolean_cube();
    for (i, a) in cube.iter().enumerate() {
        for (j, b) in cube.iter().enumerate() {
            let subset = i & !j == 0;
            assert_eq!(a == b, i == j, "equality of cube versions {i} and {j}");
            assert_eq!(a <= b, subset, "order of cube versions {i} and {j}");
            assert_eq!(
                a < b,
                subset && i != j,
                "strict order of cube versions {i} and {j}"
            );
            assert_eq!(a | b, cube[i | j], "join of cube versions {i} and {j}");
            assert_eq!(a & b, cube[i & j], "meet of cube versions {i} and {j}");
        }
    }

    let segments = cube_segments();
    let spans: Vec<Span<'_>> = segments
        .iter()
        .map(|&(lo, hi)| Span::new(&cube[lo], &cube[hi]).expect("`lo` lies below `hi`"))
        .collect();
    let covered: Vec<Vec<usize>> = segments
        .iter()
        .map(|&(lo, hi)| (0..16).filter(|&m| lo & !m == 0 && m & !hi == 0).collect())
        .collect();

    let clauses = cube_clauses();
    let table = admission_table(&clauses, &cube, &cube);
    for query in cube_queries(&clauses) {
        let admitted: Vec<bool> = (0..cube.len()).map(|m| query.admits(&table, m)).collect();
        let (membership, coverage) = cube_verdicts(query, &clauses, &cube, &spans);
        let first = &clauses[query.first];
        let second = query.second.map(|second| &clauses[second]);
        assert_eq!(membership, admitted, "membership of {first:?} & {second:?}");
        for ((segment, members), verdict) in segments.iter().zip(&covered).zip(coverage) {
            assert_eq!(
                verdict,
                coverage_of(members.iter().map(|&m| admitted[m])),
                "coverage of segment {segment:?} by {first:?} & {second:?}"
            );
        }
    }
}

/// Each height doubled onto the two cells that halve its cell: the same
/// version's height vector on the partition twice as fine.
fn lifted(heights: &[u64]) -> Heights {
    Heights(
        heights
            .iter()
            .flat_map(|&height| [height, height])
            .collect(),
    )
}

/// The indices of the members of `pool` that lie between `lo` and `hi` in
/// the pointwise order.
fn members_between(pool: &[Heights], lo: &Heights, hi: &Heights) -> Vec<usize> {
    (0..pool.len())
        .filter(|&m| lo.le(&pool[m]) && pool[m].le(hi))
        .collect()
}

/// A pool of versions constant on each cell of one partition, read on its
/// cells halved, beside the *finer pool*: every height vector on the halved
/// cells with heights up to the pool's largest.
///
/// The finer pool realizes every verdict that a version covered by a segment
/// between pool versions can receive from a conjunction of clauses anchored
/// at pool versions. Each clause compares a probe with an anchor constant on
/// every cell of the partition, and `p <= a` holds exactly when `p`'s maximum
/// on each cell is at most `a`'s height there, `a <= p` exactly when `p`'s
/// minimum is at least it; the strict forms and equality combine the two. So
/// a conjunction's verdict depends only on the probe's minimum and maximum on
/// each cell. A version inside a segment takes integer heights between the
/// endpoints' on each cell, and the finer pool holds every combination of
/// heights on the halves, so the two halves of each cell realize every such
/// pair of a minimum and a maximum, on all cells at once.
///
/// That is: a census over a segment agrees with the count over the finer
/// pool's members inside it exactly when the census is exact on the segment.
/// The finer-pool tests judge membership by the clauses' relations in the
/// pointwise order on height vectors, so no production code takes part.
struct FinerPool {
    /// Each pool version's heights on the halved cells, in pool order.
    lifted: Vec<Heights>,
    /// The finer pool.
    fine: Vec<Heights>,
    /// Every segment between two pool versions.
    segments: Vec<Segment>,
}

/// A segment between two pool versions, with what it covers in each pool.
struct Segment {
    /// The pool index of the segment's bottom endpoint.
    lo: usize,
    /// The pool index of the segment's top endpoint.
    hi: usize,
    /// The indices of the pool versions the segment covers.
    members: Vec<usize>,
    /// The indices of the finer pool's members the segment covers.
    fine_members: Vec<usize>,
}

impl FinerPool {
    /// Reads a pool given as each version's heights on the partition's
    /// cells, and lists every segment between two of its versions, bottom
    /// index outermost.
    ///
    /// # Panics
    ///
    /// If the pool is empty, or its height vectors differ in length.
    fn new(pool: &[Vec<u64>]) -> FinerPool {
        let lifted: Vec<Heights> = pool.iter().map(|heights| lifted(heights)).collect();
        let cells = lifted.first().expect("a pool holds a version").0.len();
        let cells = u32::try_from(cells).expect("a small partition's cell count fits in u32");
        let max = lifted
            .iter()
            .flat_map(|heights| heights.0.iter().copied())
            .max()
            .unwrap_or(0);
        let fine: Vec<Heights> = cell_heights(cells, max).into_iter().map(Heights).collect();
        assert!(
            lifted.iter().all(|heights| fine.contains(heights)),
            "every pool version lifts into the finer pool"
        );
        let indices = 0..lifted.len();
        let segments = indices
            .clone()
            .flat_map(|lo| indices.clone().map(move |hi| (lo, hi)))
            .filter(|&(lo, hi)| lifted[lo].le(&lifted[hi]))
            .map(|(lo, hi)| Segment {
                lo,
                hi,
                members: members_between(&lifted, &lifted[lo], &lifted[hi]),
                fine_members: members_between(&fine, &lifted[lo], &lifted[hi]),
            })
            .collect();
        FinerPool {
            lifted,
            fine,
            segments,
        }
    }
}

/// The cube holds a witness for every coverage verdict its query family
/// produces.
///
/// A segment between cube versions also covers versions outside the cube,
/// such as one ticked on a single eighth of the unit interval. For every
/// query of the cube's family and every segment between cube versions, the
/// census over the cube must agree with the count over its [`FinerPool`]:
/// the 256 height vectors holding 0 or 1 on each eighth.
#[test]
fn cube_census_agrees_with_a_finer_pool() {
    let FinerPool {
        lifted: cube,
        fine,
        segments,
    } = FinerPool::new(&cell_heights(4, 1));
    let clauses = cube_clauses();
    let cube_table = admission_table(&clauses, &cube, &cube);
    let fine_table = admission_table(&clauses, &cube, &fine);
    for query in cube_queries(&clauses) {
        let cube_admitted: Vec<bool> = (0..cube.len())
            .map(|m| query.admits(&cube_table, m))
            .collect();
        let fine_admitted: Vec<bool> = (0..fine.len())
            .map(|m| query.admits(&fine_table, m))
            .collect();
        for segment in &segments {
            assert_eq!(
                coverage_of(segment.members.iter().map(|&m| cube_admitted[m])),
                coverage_of(segment.fine_members.iter().map(|&m| fine_admitted[m])),
                "census of the segment from cube version {} to {} by {:?} & {:?}",
                segment.lo,
                segment.hi,
                clauses[query.first],
                query.second.map(|second| &clauses[second]),
            );
        }
    }
}

/// The two-party grid read on quarters beside its [`FinerPool`], built once
/// because every case reads the same pools.
///
/// The grid's heights are listed index for index with [`two_party_grid`]:
/// each party owns one half, so its height fills both of that half's
/// quarters.
static GRID_IN_FINER_POOL: LazyLock<FinerPool> = LazyLock::new(|| {
    let grid: Vec<Vec<u64>> = (0..=2u64)
        .flat_map(|a| (0..=2u64).map(move |b| vec![a, b]))
        .collect();
    for (version, heights) in two_party_grid().iter().zip(&grid) {
        assert_eq!(
            *version,
            from_oracle_version(&cell_event(heights)),
            "the committed grid is the height grid, index for index"
        );
    }
    FinerPool::new(&grid)
});

proptest! {
    /// The two-party grid holds a witness for every coverage verdict of the
    /// queries `public_queries_match_their_relational_denotation` draws.
    ///
    /// For conjunctions drawn from the same clause strategies, and every
    /// segment between grid members, the census [`pool_coverage`] takes over
    /// the grid must agree with the count over its [`FinerPool`]: the 81
    /// height vectors holding 0, 1, or 2 on each quarter.
    #[test]
    fn grid_census_agrees_with_a_finer_pool(
        bounds in prop::collection::vec(bound_clause(), 0..=6),
        down in prop::collection::vec(down_clause(), 1..=6),
        up in prop::collection::vec(up_clause(), 1..=6),
    ) {
        let FinerPool { lifted: grid, fine, segments } = &*GRID_IN_FINER_POOL;
        let neutral = |probe: &Heights| bounds.iter().all(|bound| bound.admits(probe, grid));
        let downward = |probe: &Heights| {
            neutral(probe) && down.iter().all(|clause| clause.admits(probe, grid))
        };
        let upward = |probe: &Heights| {
            neutral(probe) && up.iter().all(|clause| clause.admits(probe, grid))
        };
        let predicates: [&dyn Fn(&Heights) -> bool; 3] = [&neutral, &downward, &upward];
        for admits in predicates {
            let grid_admitted: Vec<bool> = grid.iter().map(admits).collect();
            let fine_admitted: Vec<bool> = fine.iter().map(admits).collect();
            for segment in segments {
                prop_assert_eq!(
                    coverage_of(segment.members.iter().map(|&m| grid_admitted[m])),
                    coverage_of(segment.fine_members.iter().map(|&m| fine_admitted[m])),
                    "census over [{:?}, {:?}]",
                    grid[segment.lo],
                    grid[segment.hi],
                );
            }
        }
    }
}

/// A degenerate hole, one nothing can fall into, rides through
/// inert: it subtracts nothing on every path, and no corner-case
/// machinery exists (or is needed) to strip it.
#[test]
fn degenerate_holes_are_inert() {
    let w = witnesses();
    // ¬(v < ⊥): nothing lies below the empty version.
    let inert = after(&w.bottom).or_concurrent();
    for v in [&w.bottom, &w.a1, &w.b1, &w.joined] {
        assert!(inert.contains(v));
    }
    assert_eq!(
        inert.coverage(w.bottom.span(&w.joined).reborrow()),
        Coverage::Full
    );
    // The hole at ⊥ subtracts exactly ⊥.
    let s = since(&w.bottom);
    assert!(!s.contains(&w.bottom));
    assert!(s.contains(&w.a1));
    assert!(s.contains(&w.b1));
}

/// The conversions denote what they claim: a span converts to its
/// segment's query, a version to the singleton admitting exactly
/// itself, and `into_owned` preserves behavior while erasing the
/// borrows.
#[test]
fn conversions_denote() {
    let w = witnesses();
    let span = w.a1.span(&w.a3);
    let segment = Query::from(&span);
    for v in [&w.bottom, &w.a1, &w.a2, &w.a3, &w.b1, &w.joined] {
        assert_eq!(
            segment.contains(v),
            w.a1 <= v && v <= w.a3,
            "segment membership at {v:?}"
        );
        assert_eq!(
            Query::from(span.clone()).contains(v),
            segment.contains(v),
            "consuming and borrowing conversions agree at {v:?}"
        );
        assert_eq!(Query::from(&w.a2).contains(v), *v == w.a2);
    }
    let owned: Query<'static, Down> = {
        let borrowed = delta(&w.a1, &w.a3);
        borrowed.into_owned()
    };
    assert!(owned.contains(&w.a2));
    assert!(!owned.contains(&w.a1));
    assert!(!owned.contains(&w.b1));
}

/// `Debug` renders the expression vocabulary (the module's one
/// structural window), with holes as the negated atoms they equal.
#[test]
fn debug_renders_expressions() {
    let w = witnesses();
    assert_eq!(format!("{:?}", all()), "all()");
    assert_eq!(format!("{:?}", since(&w.bottom)), "!before(Version(0b11))");
    assert_eq!(
        format!("{:?}", after(&w.bottom) & before(&w.bottom)),
        "after(Version(0b11)) & before(Version(0b11))"
    );
    assert_eq!(
        format!("{:?}", after(&w.bottom).or_concurrent()),
        "!strictly_before(Version(0b11))"
    );
    assert_eq!(format!("{:?}", !after(&w.bottom)), "!after(Version(0b11))");
}
