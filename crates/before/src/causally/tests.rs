use super::*;
use crate::testing::bridge::from_oracle_version;
use crate::testing::generators::arb_oracle_version;
use crate::{Clock, Span};
use proptest::prelude::*;
use proptest::test_runner::TestCaseError;

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

/// A complete interval of versions over two parties, with each party at
/// heights zero through two.
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

/// Selects a generated version while allowing one recipe to run against
/// differently sized version pools.
fn selected(grid: &[Version], index: usize) -> &Version {
    &grid[index % grid.len()]
}

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

    /// Evaluates this clause directly from the causal order.
    fn admits(&self, probe: &Version, grid: &[Version]) -> bool {
        match *self {
            Self::After(at) => le(selected(grid, at), probe),
            Self::Before(at) => le(probe, selected(grid, at)),
        }
    }
}

/// Generates bounds throughout the complete two-party grid.
fn bound_clause() -> impl Strategy<Value = BoundClause> {
    prop_oneof![
        (0usize..9).prop_map(BoundClause::After),
        (0usize..9).prop_map(BoundClause::Before),
    ]
}

/// One public downward-polar query form.
#[derive(Clone, Debug)]
enum DownClause {
    Since(usize),
    StrictlyAfter(usize),
    AfterOrConcurrent(usize),
    Delta(usize, usize),
}

impl DownClause {
    /// Constructs this clause through the public query vocabulary.
    fn query<'a>(&self, grid: &'a [Version]) -> Query<'a, Down> {
        match *self {
            Self::Since(at) => since(selected(grid, at)),
            Self::StrictlyAfter(at) => strictly_after(selected(grid, at)),
            Self::AfterOrConcurrent(at) => after(selected(grid, at)).or_concurrent(),
            Self::Delta(start, end) => delta(selected(grid, start), selected(grid, end)),
        }
    }

    /// Evaluates this clause directly from the causal order.
    fn admits(&self, probe: &Version, grid: &[Version]) -> bool {
        match *self {
            Self::Since(at) => !le(probe, selected(grid, at)),
            Self::StrictlyAfter(at) => lt(selected(grid, at), probe),
            Self::AfterOrConcurrent(at) => !lt(probe, selected(grid, at)),
            Self::Delta(start, end) => {
                !le(probe, selected(grid, start)) && le(probe, selected(grid, end))
            }
        }
    }
}

/// Generates every public downward-polar query form across the grid.
fn down_clause() -> impl Strategy<Value = DownClause> {
    prop_oneof![
        (0usize..9).prop_map(DownClause::Since),
        (0usize..9).prop_map(DownClause::StrictlyAfter),
        (0usize..9).prop_map(DownClause::AfterOrConcurrent),
        (0usize..9, 0usize..9).prop_map(|(start, end)| DownClause::Delta(start, end)),
    ]
}

/// One public upward-polar query form.
#[derive(Clone, Debug)]
enum UpClause {
    Until(usize),
    StrictlyBefore(usize),
    BeforeOrConcurrent(usize),
    Toward(usize, usize),
}

impl UpClause {
    /// Constructs this clause through the public query vocabulary.
    fn query<'a>(&self, grid: &'a [Version]) -> Query<'a, Up> {
        match *self {
            Self::Until(at) => until(selected(grid, at)),
            Self::StrictlyBefore(at) => strictly_before(selected(grid, at)),
            Self::BeforeOrConcurrent(at) => before(selected(grid, at)).or_concurrent(),
            Self::Toward(start, end) => toward(selected(grid, start), selected(grid, end)),
        }
    }

    /// Evaluates this clause directly from the causal order.
    fn admits(&self, probe: &Version, grid: &[Version]) -> bool {
        match *self {
            Self::Until(at) => !le(selected(grid, at), probe),
            Self::StrictlyBefore(at) => lt(probe, selected(grid, at)),
            Self::BeforeOrConcurrent(at) => !lt(selected(grid, at), probe),
            Self::Toward(start, end) => {
                le(selected(grid, start), probe) && !le(selected(grid, end), probe)
            }
        }
    }
}

/// Generates every public upward-polar query form across the grid.
fn up_clause() -> impl Strategy<Value = UpClause> {
    prop_oneof![
        (0usize..9).prop_map(UpClause::Until),
        (0usize..9).prop_map(UpClause::StrictlyBefore),
        (0usize..9).prop_map(UpClause::BeforeOrConcurrent),
        (0usize..9, 0usize..9).prop_map(|(start, end)| UpClause::Toward(start, end)),
    ]
}

/// Checks membership and exact span coverage against an independent predicate
/// over the complete grid.
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
            let mut admitted = grid
                .iter()
                .filter(|probe| le(lo, probe) && le(probe, hi))
                .map(&admits);
            let first = admitted
                .next()
                .expect("every valid span contains its endpoints");
            let expected = if admitted.all(|next| next == first) {
                if first {
                    Coverage::Full
                } else {
                    Coverage::Empty
                }
            } else {
                Coverage::Partial
            };
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

/// Builds a neutral conjunction from public inclusive bounds.
fn neutral_query<'a>(bounds: &[BoundClause], versions: &'a [Version]) -> Query<'a> {
    bounds
        .iter()
        .fold(all(), |query, bound| bound.add_to_neutral(query, versions))
}

/// Builds a downward-polar conjunction from public forms and inclusive bounds.
fn down_query<'a>(
    clauses: &[DownClause],
    bounds: &[BoundClause],
    versions: &'a [Version],
) -> Query<'a, Down> {
    let mut clauses = clauses.iter();
    let first = clauses
        .next()
        .expect("the strategy always generates a downward clause");
    let query = clauses.fold(first.query(versions), |query, clause| {
        query & clause.query(versions)
    });
    bounds
        .iter()
        .fold(query, |query, bound| bound.add_to_down(query, versions))
}

/// Builds an upward-polar conjunction from public forms and inclusive bounds.
fn up_query<'a>(
    clauses: &[UpClause],
    bounds: &[BoundClause],
    versions: &'a [Version],
) -> Query<'a, Up> {
    let mut clauses = clauses.iter();
    let first = clauses
        .next()
        .expect("the strategy always generates an upward clause");
    let query = clauses.fold(first.query(versions), |query, clause| {
        query & clause.query(versions)
    });
    bounds
        .iter()
        .fold(query, |query, bound| bound.add_to_up(query, versions))
}

proptest! {
    /// Public query expressions denote the conjunction of their causal
    /// relations, independently of normalization and fused evaluation.
    ///
    /// Every case first exhausts a complete small two-party interval, making
    /// both membership and exact coverage enumerable. The same expression is
    /// then applied to arbitrary normal-form versions whose shapes and numeric
    /// magnitudes cross the skyline walk's representation thresholds; those
    /// versions and their lattice corners exercise membership beyond the
    /// finite interval without pretending that a sparse sample proves span
    /// coverage.
    #[test]
    fn public_queries_match_their_relational_denotation(
        bounds in prop::collection::vec(bound_clause(), 0..=6),
        down in prop::collection::vec(down_clause(), 1..=6),
        up in prop::collection::vec(up_clause(), 1..=6),
        a in arb_oracle_version(),
        b in arb_oracle_version(),
        c in arb_oracle_version(),
    ) {
        let small = two_party_grid();
        let neutral = neutral_query(&bounds, &small);
        assert_denotes(&neutral, &small, |probe| {
            bounds.iter().all(|bound| bound.admits(probe, &small))
        })?;
        let small_down = down_query(&down, &bounds, &small);
        assert_denotes(&small_down, &small, |probe| {
            bounds.iter().all(|bound| bound.admits(probe, &small))
                && down.iter().all(|clause| clause.admits(probe, &small))
        })?;
        let small_up = up_query(&up, &bounds, &small);
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

        let neutral = neutral_query(&bounds, &large);
        let down_query = down_query(&down, &bounds, &large);
        let up_query = up_query(&up, &bounds, &large);
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
/// advertised relation on an organic witness set — the point verdicts
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
/// the merged bounds avoid are pruned — and strictness dissolves
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
/// empty (by floor, by ceiling, and by hole), and genuinely mixed —
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

    // Floor and ceiling each straddle [a1, a2b1] — the fused folds
    // alone read mixed — but nothing is at once above a2 and within
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

/// Coverage is exact over the complete two-party small scope.
///
/// `Full`, `Partial`, and `Empty` each hold iff the brute-force
/// membership census says so, for every version of the tick grid,
/// every ordered segment of it, and a query family covering both
/// polarities, all hole spellings, and their conjunctions.
#[test]
fn coverage_is_exact_on_the_two_party_grid() {
    // The complete interval [⊥, A2B2]: with two parties and no
    // sub-forks, every version is a pair of per-party tick heights.
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
            up.push((!after(x)) & (!after(y)));
            up.push(before(x) & (!after(y)));
        }
    }

    /// The brute-force verdict: membership counted over every grid
    /// version the segment covers (the grid is the whole interval,
    /// so the census is total).
    fn brute<P: Polarity>(
        q: &Query<'_, P>,
        lo: &Version,
        hi: &Version,
        grid: &[Version],
    ) -> Coverage {
        let covered: Vec<&Version> = grid.iter().filter(|v| le(lo, v) && le(v, hi)).collect();
        let admitted = covered.iter().filter(|v| q.contains(v)).count();
        if admitted == covered.len() {
            Coverage::Full
        } else if admitted == 0 {
            Coverage::Empty
        } else {
            Coverage::Partial
        }
    }

    for lo in &grid {
        for hi in &grid {
            let Ok(span) = Span::new(lo, hi) else {
                continue;
            };
            for q in &neutral {
                assert_eq!(
                    q.coverage(span.reborrow()),
                    brute(q, lo, hi, &grid),
                    "{q:?} over [{lo:?}, {hi:?}]"
                );
            }
            for q in &down {
                assert_eq!(
                    q.coverage(span.reborrow()),
                    brute(q, lo, hi, &grid),
                    "{q:?} over [{lo:?}, {hi:?}]"
                );
            }
            for q in &up {
                assert_eq!(
                    q.coverage(span.reborrow()),
                    brute(q, lo, hi, &grid),
                    "{q:?} over [{lo:?}, {hi:?}]"
                );
            }
        }
    }
}

/// A degenerate hole — one nothing can fall into — rides through
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
            le(&w.a1, v) && le(v, &w.a3),
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

/// `Debug` renders the expression vocabulary — the module's one
/// structural window — with holes as the negated atoms they equal.
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
