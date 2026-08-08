use super::*;
use crate::Clock;

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

/// The number of holes a query renders, read through the `Debug`
/// window (each hole renders as one `!`-prefixed atom; nothing else
/// in the expression vocabulary or paper notation prints `!`).
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
    assert_eq!(format!("{:?}", since(&w.bottom)), "!before(0)");
    assert_eq!(
        format!("{:?}", after(&w.bottom) & before(&w.bottom)),
        "after(0) & before(0)"
    );
    assert_eq!(
        format!("{:?}", after(&w.bottom).or_concurrent()),
        "!strictly_before(0)"
    );
    assert_eq!(format!("{:?}", !after(&w.bottom)), "!after(0)");
}
