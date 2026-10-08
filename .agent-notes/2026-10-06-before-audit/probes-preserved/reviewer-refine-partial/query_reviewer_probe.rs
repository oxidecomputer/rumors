//! Reviewer probe (temporary, never committed): differential of the coverage
//! refinement against the endpoint-clamp formula it replaces, plus an explicit
//! release-mode check of the walk's `Partial` certification.

use std::borrow::Cow;
use std::marker::PhantomData;

use proptest::prelude::*;
use proptest::test_runner::{Config, TestCaseError, TestRunner};

use super::{Coverage, Query};
use crate::causally::polarity::{Down, Hole, Polarity, Up};
use crate::testing::bridge::from_oracle_version;
use crate::testing::generators::arb_oracle_version;
use crate::version::place::filter;
use crate::{Clock, Span, Version};

/// The replaced refinement, verbatim in meaning: build both clamped endpoints,
/// compare them, and hand the polarity's endpoint to the hole test.
fn old_refine<P: Polarity>(q: &Query<'_, P>, lo: &Version, hi: &Version) -> Coverage {
    let clamped_lo = match q.floor.as_deref() {
        Some(f) => lo | f,
        None => lo.clone(),
    };
    let clamped_hi = match q.ceiling.as_deref() {
        Some(c) => hi & c,
        None => hi.clone(),
    };
    if !(clamped_lo <= clamped_hi) {
        return Coverage::Empty;
    }
    if q.holes.is_empty() {
        return Coverage::Partial;
    }
    // With both bounds absent, the new dispatch borrows the requested endpoint.
    let endpoint = P::covering_endpoint(&clamped_lo, &clamped_hi, None, None);
    if filter::admits(&endpoint, Query::<'_, P>::hole_demands(&q.holes)) {
        Coverage::Partial
    } else {
        Coverage::Empty
    }
}

#[derive(Default, Debug)]
struct Stats {
    checks: u64,
    walk_partial: u64,
    partial_both: u64,
    partial_both_holes: u64,
    crossed: u64,
    crossed_holes: u64,
    equal_bounds: u64,
    incomparable_bounds: u64,
    floor_eq_hi: u64,
    ceiling_eq_lo: u64,
    verdict_empty_from_refine: u64,
    brute_checked: u64,
}

fn check<P: Polarity>(
    q: &Query<'_, P>,
    lo: &Version,
    hi: &Version,
    st: &mut Stats,
) -> Result<bool, String> {
    st.checks += 1;
    let span = Span::new(lo, hi).map_err(|_| "unordered span".to_string())?;
    let new = q.coverage(span.reborrow());
    let walk = if lo.ptr_eq(hi) {
        None
    } else {
        Some(filter::coverage(lo, hi, q.demands()))
    };
    let old = match walk {
        None => new,
        Some(Coverage::Partial) => old_refine(q, lo, hi),
        Some(v) => v,
    };
    if walk == Some(Coverage::Partial) {
        st.walk_partial += 1;
        let (f, c) = (q.floor.as_deref(), q.ceiling.as_deref());
        // The precondition, checked explicitly so release builds check it too.
        if !(f.is_none_or(|f| f <= hi) && c.is_none_or(|c| lo <= c)) {
            return Err(format!("PRECONDITION BROKEN: {q:?} over [{lo:?}, {hi:?}]"));
        }
        if let (Some(f), Some(c)) = (f, c) {
            st.partial_both += 1;
            if !q.holes.is_empty() {
                st.partial_both_holes += 1;
            }
            if !(f <= c) {
                st.crossed += 1;
                if !q.holes.is_empty() {
                    st.crossed_holes += 1;
                }
            }
            if f == c {
                st.equal_bounds += 1;
            }
            if !(f <= c) && !(c <= f) {
                st.incomparable_bounds += 1;
            }
        }
        if f.is_some_and(|f| f == hi) {
            st.floor_eq_hi += 1;
        }
        if c.is_some_and(|c| c == lo) {
            st.ceiling_eq_lo += 1;
        }
        if new == Coverage::Empty {
            st.verdict_empty_from_refine += 1;
        }
    }
    if new != old {
        return Err(format!(
            "DIVERGENCE new {new:?} old {old:?}: {q:?} over [{lo:?}, {hi:?}]"
        ));
    }
    Ok(walk == Some(Coverage::Partial))
}

/// Builds a stored form directly: a superset of what the public conjunction
/// normalizes to.
fn raw<'v, P: Polarity>(
    floor: Option<&'v Version>,
    ceiling: Option<&'v Version>,
    holes: &[(&'v Version, bool)],
) -> Query<'v, P> {
    Query {
        floor: floor.map(Cow::Borrowed),
        ceiling: ceiling.map(Cow::Borrowed),
        holes: holes
            .iter()
            .map(|&(at, strict)| Hole {
                at: Cow::Borrowed(at),
                strict,
            })
            .collect(),
        polarity: PhantomData,
    }
}

/// The census over a sublattice grid: exact because every emptiness or
/// non-fullness witness is a lattice endpoint of grid bounds.
fn brute<P: Polarity>(q: &Query<'_, P>, lo: &Version, hi: &Version, grid: &[Version]) -> Coverage {
    let covered: Vec<&Version> = grid.iter().filter(|v| lo <= *v && *v <= hi).collect();
    let admitted = covered.iter().filter(|v| q.contains(v)).count();
    if admitted == covered.len() {
        Coverage::Full
    } else if admitted == 0 {
        Coverage::Empty
    } else {
        Coverage::Partial
    }
}

fn three_party_grid(ticks: usize) -> Vec<Version> {
    let mut alice = Clock::seed();
    let mut bob = alice.fork();
    let mut carol = alice.fork();
    let lines: Vec<Vec<Version>> = [&mut alice, &mut bob, &mut carol]
        .into_iter()
        .map(|clock| {
            let mut line = vec![Version::new()];
            for _ in 0..ticks {
                line.push(clock.tick().clone());
            }
            line
        })
        .collect();
    let mut grid = Vec::new();
    for a in &lines[0] {
        for b in &lines[1] {
            for c in &lines[2] {
                grid.push(&(a | b) | c);
            }
        }
    }
    grid
}

fn exhaust<P: Polarity>(grid: &[Version], with_brute: bool, st: &mut Stats) {
    let bounds: Vec<Option<&Version>> =
        std::iter::once(None).chain(grid.iter().map(Some)).collect();
    let mut holes: Vec<Vec<(&Version, bool)>> = vec![vec![]];
    for h in grid {
        holes.push(vec![(h, false)]);
        holes.push(vec![(h, true)]);
    }
    for lo in grid {
        for hi in grid {
            if !(lo <= hi) {
                continue;
            }
            for &f in &bounds {
                for &c in &bounds {
                    for hs in &holes {
                        let q = raw::<P>(f, c, hs);
                        let partial = check(&q, lo, hi, st).unwrap_or_else(|e| panic!("{e}"));
                        let partial_both = partial && f.is_some() && c.is_some();
                        if with_brute || partial_both {
                            let want = brute(&q, lo, hi, grid);
                            let got = q.coverage(Span::new(lo, hi).unwrap());
                            st.brute_checked += 1;
                            assert_eq!(got, want, "BRUTE {q:?} over [{lo:?}, {hi:?}]");
                        }
                    }
                }
            }
        }
    }
}

/// Exhaustive three-party differential and census.
#[test]
fn reviewer_refine_partial_exhaustive() {
    let ticks = if cfg!(debug_assertions) { 1 } else { 2 };
    let grid = three_party_grid(ticks);
    let mut down = Stats::default();
    let mut up = Stats::default();
    exhaust::<Down>(&grid, ticks == 1, &mut down);
    exhaust::<Up>(&grid, ticks == 1, &mut up);
    eprintln!(
        "RV exhaustive ticks={ticks} grid={} down={down:?}",
        grid.len()
    );
    eprintln!("RV exhaustive ticks={ticks} grid={} up={up:?}", grid.len());
    for st in [&down, &up] {
        assert!(st.crossed_holes > 0 && st.equal_bounds > 0 && st.incomparable_bounds > 0);
    }
}

/// Random differential over arbitrary normal-form versions (deep trees,
/// magnitudes near `u64::MAX`), bounds and holes drawn from a lattice pool
/// around the span so `Partial` walks with both bounds are common.
#[test]
fn reviewer_refine_partial_random() {
    let cases = if cfg!(debug_assertions) {
        2_000
    } else {
        30_000
    };
    let mut runner = TestRunner::new(Config {
        cases,
        failure_persistence: None,
        ..Config::default()
    });
    let idx = || proptest::option::weighted(0.8, 0usize..12);
    let strategy = (
        (
            arb_oracle_version(),
            arb_oracle_version(),
            arb_oracle_version(),
            arb_oracle_version(),
        ),
        (idx(), idx()),
        prop::collection::vec((0usize..12, any::<bool>()), 0..=2),
    );
    let stats = std::cell::RefCell::new((Stats::default(), Stats::default()));
    let result = runner.run(&strategy, |((a, b, c, d), (fi, ci), hi_idx)| {
        let [a, b, c, d] = [a, b, c, d].map(|v| from_oracle_version(&v));
        let lo = &a & &b;
        let hi = &a | &b;
        let m1 = &(&lo | &c) & &hi;
        let m2 = &(&lo | &d) & &hi;
        let pool = [
            lo.clone(),
            hi.clone(),
            m1.clone(),
            m2.clone(),
            &m1 | &m2,
            &m1 & &m2,
            c.clone(),
            d.clone(),
            a.clone(),
            b.clone(),
            &c | &lo,
            &d & &hi,
        ];
        let f = fi.map(|i| &pool[i]);
        let ce = ci.map(|i| &pool[i]);
        let hs: Vec<(&Version, bool)> = hi_idx.iter().map(|&(i, s)| (&pool[i], s)).collect();
        let mut guard = stats.borrow_mut();
        let qd = raw::<Down>(f, ce, &hs);
        let qu = raw::<Up>(f, ce, &hs);
        check(&qd, &lo, &hi, &mut guard.0).map_err(TestCaseError::fail)?;
        check(&qu, &lo, &hi, &mut guard.1).map_err(TestCaseError::fail)?;
        // Soundness against in-segment probes.
        let probes: Vec<&Version> = pool.iter().filter(|v| lo <= **v && **v <= hi).collect();
        macro_rules! sound {
            ($q:expr) => {
                match $q.coverage(Span::new(&lo, &hi).unwrap()) {
                    Coverage::Full => prop_assert!(
                        probes.iter().all(|p| $q.contains(p)),
                        "Full unsound {:?}",
                        $q
                    ),
                    Coverage::Empty => prop_assert!(
                        !probes.iter().any(|p| $q.contains(p)),
                        "Empty unsound {:?}",
                        $q
                    ),
                    Coverage::Partial => {}
                }
            };
        }
        sound!(qd);
        sound!(qu);
        Ok(())
    });
    let (down, up) = stats.into_inner();
    eprintln!("RV random cases={cases} down={down:?}");
    eprintln!("RV random cases={cases} up={up:?}");
    if let Err(e) = result {
        panic!("{e}");
    }
    for st in [&down, &up] {
        assert!(st.crossed_holes > 0 && st.equal_bounds > 0 && st.incomparable_bounds > 0);
    }
}
