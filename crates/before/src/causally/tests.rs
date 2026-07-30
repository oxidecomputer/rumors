use std::cmp::Ordering;
use std::ops::{Bound, RangeBounds};

use proptest::prelude::*;

use super::*;
use crate::testing::bridge::from_oracle_version;
use crate::testing::generators::arb_oracle_version;
use crate::Clock;

/// Three versions exercising every comparison the predicates distinguish:
/// `low < high`, and `side` concurrent to both.
fn fixtures() -> (Version, Version, Version) {
    let mut alice = Clock::seed();
    let mut bob = alice.fork();
    let low = alice.tick().clone();
    let high = alice.tick().clone();
    let side = bob.tick().clone();
    assert!(low < high);
    assert!(low.concurrent(&side) && high.concurrent(&side));
    (low, high, side)
}

/// Every free constructor produces exactly the bound pair its docs promise,
/// observable through the `RangeBounds` accessors.
#[test]
fn constructors_produce_documented_bounds() {
    let (low, high, _) = fixtures();
    let cases: [(Range<'_>, Bound<&Version>, Bound<&Version>); 7] = [
        (all(), Bound::Unbounded, Bound::Unbounded),
        (since(&low), Bound::Excluded(&low), Bound::Unbounded),
        (not_before(&low), Bound::Included(&low), Bound::Unbounded),
        (known_at(&high), Bound::Unbounded, Bound::Included(&high)),
        (before(&high), Bound::Unbounded, Bound::Excluded(&high)),
        (
            delta(&low, &high).unwrap(),
            Bound::Excluded(&low),
            Bound::Included(&high),
        ),
        (
            delta_before(&low, &high).unwrap(),
            Bound::Excluded(&low),
            Bound::Excluded(&high),
        ),
    ];
    for (range, start, end) in cases {
        assert_eq!(range.start_bound(), start);
        assert_eq!(range.end_bound(), end);
    }
}

/// Composition is order-agnostic: refining the start then the end yields
/// the same range as the reverse, for every start/end pairing.
#[test]
fn composition_is_order_agnostic() {
    let (low, high, _) = fixtures();
    assert_eq!(since(&low).known_at(&high), known_at(&high).since(&low),);
    assert_eq!(since(&low).before(&high), before(&high).since(&low));
    assert_eq!(
        not_before(&low).known_at(&high),
        known_at(&high).not_before(&low),
    );
    assert_eq!(
        not_before(&low).before(&high),
        before(&high).not_before(&low),
    );
}

/// Re-setting a bound keeps the latest value, so chains always mean their
/// final state, revalidated against the opposite bound.
#[test]
fn rebinding_a_bound_keeps_the_latest() {
    let (low, high, _) = fixtures();
    assert_eq!(since(&low).since(&high).unwrap(), since(&high));
    assert_eq!(since(&low).not_before(&high).unwrap(), not_before(&high));
    assert_eq!(known_at(&low).before(&high).unwrap(), before(&high));
    assert_eq!(all().since(&low).unwrap(), since(&low));
}

/// A start bound of either kind subtracts only its causal past: versions
/// concurrent to it pass, and the two kinds differ exactly at the bound
/// itself.
#[test]
fn start_bounds_keep_concurrent_versions() {
    let (low, high, side) = fixtures();
    for range in [since(&low), not_before(&low)] {
        assert!(range.contains(&high), "the causal future passes");
        assert!(range.contains(&side), "concurrent versions pass");
    }
    assert!(!since(&low).contains(&low), "since excludes the bound");
    assert!(
        not_before(&low).contains(&low),
        "not_before includes the bound"
    );
}

/// An end bound of either kind demands containment: versions concurrent to
/// it are dropped, and the two kinds differ exactly at the bound itself.
#[test]
fn end_bounds_drop_concurrent_versions() {
    let (low, high, side) = fixtures();
    for range in [known_at(&high), before(&high)] {
        assert!(range.contains(&low), "the causal past passes");
        assert!(!range.contains(&side), "concurrent versions are dropped");
    }
    assert!(
        known_at(&high).contains(&high),
        "known_at includes the bound"
    );
    assert!(!before(&high).contains(&high), "before excludes the bound");
}

/// The two-bound shorthands are definitionally their compositions, and
/// `delta` realizes the reconciliation set: exactly what a replica at
/// `start` lacks of `end`'s knowledge.
#[test]
fn deltas_are_their_compositions() {
    let (low, high, side) = fixtures();
    assert_eq!(delta(&low, &high), since(&low).known_at(&high));
    assert_eq!(delta_before(&low, &high), since(&low).before(&high));

    let range = delta(&low, &high).unwrap();
    assert!(range.contains(&high), "the end's novelty is in the delta");
    assert!(!range.contains(&low), "the start's knowledge is not");
    assert!(
        !range.contains(&side),
        "knowledge outside the end's past is not"
    );
}

/// `all()` is the identity: it contains every version, including genesis,
/// and refining it equals constructing directly.
#[test]
fn all_contains_everything() {
    let (low, high, side) = fixtures();
    for version in [&Version::new(), &low, &high, &side] {
        assert!(all().contains(version));
    }
}

/// Genesis is the bottom of the causal order: `since(genesis)` is every
/// *ticked* version and excludes genesis itself, which is the listener's
/// from-the-beginning shape.
#[test]
fn since_genesis_is_every_ticked_version() {
    let (low, high, side) = fixtures();
    let genesis = Version::new();
    let range = since(&genesis);
    assert!(!range.contains(&genesis));
    for version in [&low, &high, &side] {
        assert!(range.contains(version));
    }
}

/// Version-to-range placement is total — every version classifies as
/// exactly one of below (`Less`), contained (`Equal`), or beyond the end
/// bound (`Greater`).
///
/// Totality holds even where version-to-version comparison is undefined; and
/// `contains` is exactly the `Equal` case.
#[test]
fn placement_is_total() {
    use std::cmp::Ordering;
    let (low, high, side) = fixtures();
    let genesis = Version::new();
    let range = delta(&low, &high).unwrap();

    assert_eq!(range.placement_of(&genesis), Ordering::Less);
    assert_eq!(range.placement_of(&low), Ordering::Less);
    assert_eq!(range.placement_of(&high), Ordering::Equal);
    // Concurrent to the start (passes it) but not contained in the end:
    // beyond the range, despite being causally unordered against both
    // bounds. (Totality itself is carried by the signature: a bare
    // `Ordering` where Version-to-Version comparison returns an `Option`.)
    assert!(low.concurrent(&side));
    assert_eq!(range.placement_of(&side), Ordering::Greater);

    for version in [&genesis, &low, &high, &side] {
        assert_eq!(
            range.contains(version),
            range.placement_of(version) == Ordering::Equal,
        );
    }
}

/// Composition validates the pair at every boundary: a start within the
/// end bound composes, a start beyond or concurrent to the end is rejected
/// as `Crossed`.
///
/// The equal-bounds cases split exactly on the end bound's strictness
/// (`start <= end` under `known_at`, `start < end` under `before`), for
/// both start kinds and both composition orders.
#[test]
fn crossed_compositions_are_rejected_at_the_boundary() {
    let (low, high, side) = fixtures();

    // Distinct ordered bounds compose under every pairing.
    assert!(since(&low).known_at(&high).is_ok());
    assert!(since(&low).before(&high).is_ok());
    assert!(not_before(&low).known_at(&high).is_ok());
    assert!(not_before(&low).before(&high).is_ok());

    // Equal bounds: within an inclusive end, not within an exclusive one.
    assert!(since(&low).known_at(&low).is_ok(), "the empty delta");
    assert!(not_before(&low).known_at(&low).is_ok(), "the singleton");
    assert_eq!(since(&low).before(&low), Err(Crossed));
    assert_eq!(not_before(&low).before(&low), Err(Crossed));

    // A start beyond the end crosses; so does a start concurrent to it.
    assert_eq!(since(&high).known_at(&low), Err(Crossed));
    assert_eq!(not_before(&high).known_at(&low), Err(Crossed));
    assert_eq!(since(&side).known_at(&high), Err(Crossed));
    assert_eq!(not_before(&side).before(&high), Err(Crossed));

    // The gate is order-agnostic: refining the start into an existing end
    // rejects the same pairs.
    assert_eq!(known_at(&low).since(&high), Err(Crossed));
    assert_eq!(before(&low).not_before(&low), Err(Crossed));
    assert_eq!(known_at(&high).since(&side), Err(Crossed));
    assert!(known_at(&low).not_before(&low).is_ok());

    // The shorthands are the same gate.
    assert_eq!(delta(&high, &low), Err(Crossed));
    assert_eq!(delta_before(&low, &low), Err(Crossed));
    assert!(delta(&low, &low).is_ok());
}

/// The boundary compositions the gate admits mean what they say: the empty
/// delta contains nothing, and the singleton `not_before(v).known_at(v)`
/// contains exactly its bound.
#[test]
fn admitted_boundary_compositions_have_exact_membership() {
    let (low, high, side) = fixtures();
    let genesis = Version::new();

    let empty = delta(&low, &low).unwrap();
    let singleton = not_before(&low).known_at(&low).unwrap();
    for version in [&genesis, &low, &high, &side] {
        assert!(!empty.contains(version), "the empty delta keeps nothing");
        assert_eq!(
            singleton.contains(version),
            *version == low,
            "the singleton keeps exactly its bound"
        );
    }
}

/// Every [`Bounded`] verdict on a constructed witness, for every bound
/// kind it is reachable under.
///
/// An alice-chain `a1 < a2 < a3 < a4` against ranges over `[a1, a3]`
/// places each chain version and the concurrent `b1`:
/// `Before`/`AtStart`/`Between`/`AtEnd`/`After` on the line and
/// `Concurrent` off it — and the verdicts are identical under every
/// bound-kind combination, because bound kinds move only the coarsening.
#[test]
fn bounded_places_every_witness() {
    let mut alice = Clock::seed();
    let mut bob = alice.fork();
    let a1 = alice.tick().clone();
    let a2 = alice.tick().clone();
    let a3 = alice.tick().clone();
    let a4 = alice.tick().clone();
    let b1 = bob.tick().clone();
    let genesis = Version::new();

    for range in [
        since(&a1).known_at(&a3).unwrap(),
        since(&a1).before(&a3).unwrap(),
        not_before(&a1).known_at(&a3).unwrap(),
        not_before(&a1).before(&a3).unwrap(),
    ] {
        assert_eq!(range.bounded(&genesis), Bounded::Before);
        assert_eq!(range.bounded(&a1), Bounded::AtStart);
        assert_eq!(range.bounded(&a2), Bounded::Between);
        assert_eq!(range.bounded(&a3), Bounded::AtEnd);
        assert_eq!(range.bounded(&a4), Bounded::After);
        assert_eq!(range.bounded(&b1), Bounded::Concurrent);
    }
}

/// A version concurrent to the start bound but within the end bound is
/// `Between`, never `Concurrent` — start bounds keep concurrent versions,
/// so `Concurrent` is exclusively an end-bound verdict.
#[test]
fn concurrent_to_start_within_end_is_between() {
    let (low, _, side) = fixtures();
    let end = &low | &side; // dominates both, so `side` is within it
    for range in [
        since(&low).known_at(&end).unwrap(),
        not_before(&low).known_at(&end).unwrap(),
    ] {
        assert!(low.concurrent(&side));
        assert_eq!(range.bounded(&side), Bounded::Between);
    }
}

/// An unbounded side makes its verdicts unreachable: with no end bound
/// everything past the start is `Between` (including versions concurrent
/// to the start), and with no start bound nothing is `Before` or
/// `AtStart`.
#[test]
fn unbounded_sides_make_their_verdicts_unreachable() {
    let (low, high, side) = fixtures();
    let genesis = Version::new();

    for range in [since(&low), not_before(&low)] {
        assert_eq!(range.bounded(&high), Bounded::Between);
        assert_eq!(range.bounded(&side), Bounded::Between);
    }

    for range in [known_at(&low), before(&low)] {
        assert_eq!(range.bounded(&genesis), Bounded::Between);
        assert_eq!(range.bounded(&high), Bounded::After);
        assert_eq!(range.bounded(&side), Bounded::Concurrent);
    }

    for version in [&genesis, &low, &high, &side] {
        assert_eq!(all().bounded(version), Bounded::Between);
    }
}

/// The coincident-bounds corner is canonicalized to `AtStart`.
///
/// On a validated `start == end` range, a version equal to both bounds
/// reports `AtStart`, and the coarsening stays sound for both admissible
/// kind pairs (`Less` under the excluded start that subtracts the shared
/// bound, `Equal` under the included one that keeps it).
#[test]
fn coincident_bounds_canonicalize_to_at_start() {
    let (low, _, _) = fixtures();

    let subtracting = since(&low).known_at(&low).unwrap();
    assert_eq!(subtracting.bounded(&low), Bounded::AtStart);
    assert_eq!(subtracting.placement_of(&low), Ordering::Less);
    assert!(!subtracting.contains(&low));

    let keeping = not_before(&low).known_at(&low).unwrap();
    assert_eq!(keeping.bounded(&low), Bounded::AtStart);
    assert_eq!(keeping.placement_of(&low), Ordering::Equal);
    assert!(keeping.contains(&low));
}

/// The span witness fixture: an alice chain `a[0] < ... < a[4]`
/// and `b1` concurrent to every version of it.
fn span_fixtures() -> ([Version; 5], Version) {
    let mut alice = Clock::seed();
    let mut bob = alice.fork();
    let chain = [(); 5].map(|()| alice.tick().clone());
    let b1 = bob.tick().clone();
    for v in &chain {
        assert!(v.concurrent(&b1), "the lines diverge");
    }
    (chain, b1)
}

/// Every one of the nine [`Placement`] verdicts on a constructed witness.
///
/// The five chain regions land on `[a2, a4]`, the coincident
/// `At(Both)` on `[a2, a2]`, and all three `Concurrent` payloads on
/// spans whose endpoints straddle the divergent line.
#[test]
fn span_place_places_every_witness() {
    let ([a1, a2, a3, a4, a5], b1) = span_fixtures();
    let genesis = Version::new();

    // The chain verdicts, on a proper span.
    let span = Span::new(&a2, &a4).unwrap();
    assert_eq!(span.place(&genesis), Placement::Before);
    assert_eq!(span.place(&a1), Placement::Before);
    assert_eq!(span.place(&a2), Placement::At(Endpoint::Start));
    assert_eq!(span.place(&a3), Placement::Between);
    assert_eq!(span.place(&a4), Placement::At(Endpoint::End));
    assert_eq!(span.place(&a5), Placement::After);
    // Concurrent to both endpoints of the same span.
    assert_eq!(span.place(&b1), Placement::Concurrent(Endpoint::Both));

    // Equality to one endpoint of a coincident span is equality to
    // both: always `At(Both)`, never `At(Start)` or `At(End)`.
    let coincident = Span::new(&a2, &a2).unwrap();
    assert_eq!(coincident.place(&a2), Placement::At(Endpoint::Both));

    // Concurrent to the start only: `hi = a2 | b1` dominates both
    // lines, so `b1 ∥ a2` while `b1 < hi`.
    let top = &a2 | &b1;
    let straddling = Span::new(&a2, &top).unwrap();
    assert_eq!(
        straddling.place(&b1),
        Placement::Concurrent(Endpoint::Start)
    );

    // Concurrent to the end only: `a2 > a1` while `a2 ∥ a1 | b1`.
    let side_top = &a1 | &b1;
    let sideways = Span::new(&a1, &side_top).unwrap();
    assert_eq!(sideways.place(&a2), Placement::Concurrent(Endpoint::End));
}

/// Every [`Dominance`] verdict on the nine placement witnesses: the
/// coarsening's three fibers, each exercised through all its members.
#[test]
fn span_dominance_coarsens_every_witness() {
    let ([a1, a2, a3, a4, a5], b1) = span_fixtures();

    let span = Span::new(&a2, &a4).unwrap();
    // After: At(End), Placement::After, and the coincident At(Both).
    assert_eq!(span.dominance_of(&a4), Dominance::After);
    assert_eq!(span.dominance_of(&a5), Dominance::After);
    let coincident = Span::new(&a2, &a2).unwrap();
    assert_eq!(coincident.dominance_of(&a2), Dominance::After);
    // Between: At(Start), Placement::Between, Concurrent(End).
    assert_eq!(span.dominance_of(&a2), Dominance::Between);
    assert_eq!(span.dominance_of(&a3), Dominance::Between);
    let side_top = &a1 | &b1;
    let sideways = Span::new(&a1, &side_top).unwrap();
    assert_eq!(sideways.dominance_of(&a2), Dominance::Between);
    // Before: Placement::Before, Concurrent(Start), Concurrent(Both).
    assert_eq!(span.dominance_of(&a1), Dominance::Before);
    let top = &a2 | &b1;
    let straddling = Span::new(&a2, &top).unwrap();
    assert_eq!(straddling.dominance_of(&b1), Dominance::Before);
    assert_eq!(span.dominance_of(&b1), Dominance::Before);
}

/// The validating door admits exactly the ordered pairs: `lo <= hi`
/// composes (coincident included), while reversed and incomparable
/// pairs are rejected with `Crossed`.
#[test]
fn span_new_rejects_unordered_pairs() {
    let ([_, a2, _, a4, _], b1) = span_fixtures();
    assert!(Span::new(&a2, &a4).is_ok());
    assert!(Span::new(&a2, &a2).is_ok(), "coincident is ordered");
    assert_eq!(Span::new(&a4, &a2), Err(Crossed), "reversed crosses");
    assert_eq!(
        Span::new(&a2, &b1),
        Err(Crossed),
        "an incomparable pair bounds nothing"
    );
    assert_eq!(Span::new(&b1, &a2), Err(Crossed));
}

/// The trusted door constructs the identical span the validating
/// door does on an ordered pair — it skips only the check.
#[test]
fn span_ordered_is_the_validated_span() {
    let ([_, a2, _, a4, _], _) = span_fixtures();
    assert_eq!(Span::ordered(&a2, &a4), Span::new(&a2, &a4).unwrap());
    assert_eq!(Span::ordered(&a2, &a2), Span::new(&a2, &a2).unwrap());
}

/// The trusted door's debug assertion catches a violated guarantee: an
/// unordered pair panics in debug builds rather than constructing an
/// span whose verdicts would be meaningless.
#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "Span::ordered requires lo <= hi")]
fn span_ordered_asserts_the_guarantee_in_debug() {
    let ([_, a2, _, a4, _], _) = span_fixtures();
    let _ = Span::ordered(&a4, &a2);
}

/// The deriving doors on every input genre: the receiver keeps the
/// hull total, and every genre yields its tightest containing span.
///
/// An empty iterator's hull is the coincident `[self, self]`; a
/// comparable pair's is its validated span from either operand order
/// (binary and n-ary alike); a concurrent pair's is a hull whose fresh
/// endpoints strictly bracket both inputs; and owned items feed the
/// n-ary door as references do.
#[test]
fn span_derives_the_hull() {
    let ([a1, a2, _, _, _], b1) = span_fixtures();

    // The empty iterator: the receiver alone, the coincident span.
    assert_eq!(
        a1.span_all(Vec::<&Version>::new()),
        Span::new(&a1, &a1).unwrap()
    );
    // Comparable pairs: the hull is the flip repair, both operand
    // orders, binary and n-ary alike.
    let flat = Span::new(&a1, &a2).unwrap();
    assert_eq!(a1.span(&a2), flat);
    assert_eq!(a2.span(&a1), flat);
    assert_eq!(a1.span_all([&a2]), flat);
    assert_eq!(a2.span_all([&a1]), flat);
    // A concurrent pair has no reordering, but it has a hull: both
    // inputs sit strictly inside it.
    let hull = a2.span(&b1);
    assert_eq!(hull.place(&a2), Placement::Between);
    assert_eq!(hull.place(&b1), Placement::Between);
    // Owned items feed the n-ary door (the Borrow calling convention).
    assert_eq!(a1.span_all([a2.clone()]), flat);
}

/// `a <= b` under the impl causal order (`None` means concurrent, so not
/// ordered).
fn le(a: &Version, b: &Version) -> bool {
    matches!(a.partial_cmp(b), Some(Ordering::Less | Ordering::Equal))
}

proptest! {
    /// The gate's family claim, differentially against `partial_cmp`.
    ///
    /// For arbitrary version pairs and all four start/end pairings in both
    /// composition orders, a composition succeeds exactly when the start
    /// version is within the end bound (`s <= e` for an inclusive end,
    /// `s < e` for an exclusive one); and every range the gate admits has a
    /// coherent trichotomy: no probed version is both subtracted by the
    /// start (`Less`) and outside the end bound, so `Less` placements are
    /// always within the end and `contains` is exactly the `Equal` arm.
    #[test]
    fn gate_admits_exactly_the_uncrossed_and_they_cohere(
        s in arb_oracle_version(),
        e in arb_oracle_version(),
    ) {
        let s = from_oracle_version(&s);
        let e = from_oracle_version(&e);

        // Probes spanning the lattice around the bounds: bottom, the
        // bounds themselves, their join (dominates both), and their meet
        // (dominated by both).
        let probes = [
            Version::new(),
            s.clone(),
            e.clone(),
            &s | &e,
            &s & &e,
        ];

        let compositions: [(Result<Range<'_>, Crossed>, bool); 8] = [
            (since(&s).known_at(&e), le(&s, &e)),
            (since(&s).before(&e), le(&s, &e) && s != e),
            (not_before(&s).known_at(&e), le(&s, &e)),
            (not_before(&s).before(&e), le(&s, &e) && s != e),
            (known_at(&e).since(&s), le(&s, &e)),
            (before(&e).since(&s), le(&s, &e) && s != e),
            (known_at(&e).not_before(&s), le(&s, &e)),
            (before(&e).not_before(&s), le(&s, &e) && s != e),
        ];
        for (composed, expect_ok) in compositions {
            prop_assert_eq!(
                composed.is_ok(),
                expect_ok,
                "the gate admits exactly the uncrossed pairs"
            );
            let Ok(range) = composed else { continue };
            for probe in &probes {
                let placement = range.placement_of(probe);
                let within_end = match range.end_bound() {
                    Bound::Unbounded => true,
                    Bound::Included(end) => le(probe, end),
                    Bound::Excluded(end) => le(probe, end) && probe != end,
                };
                if placement == Ordering::Less {
                    prop_assert!(
                        within_end,
                        "a subtracted version is still within the end: \
                         the bounds cannot disagree about it"
                    );
                }
                prop_assert_eq!(
                    range.contains(probe),
                    placement == Ordering::Equal,
                    "contains is exactly the Equal arm"
                );
            }
        }
    }

    /// The span gate's family claim, differentially against
    /// `partial_cmp`: `Span::new` admits exactly the pairs the
    /// causal order deems ordered, and on every admitted pair the
    /// trusted door builds the identical span.
    #[test]
    fn span_gate_admits_exactly_the_ordered(
        lo in arb_oracle_version(),
        hi in arb_oracle_version(),
    ) {
        let lo = from_oracle_version(&lo);
        let hi = from_oracle_version(&hi);
        let admitted = Span::new(&lo, &hi);
        prop_assert_eq!(
            admitted.is_ok(),
            le(&lo, &hi),
            "the gate admits exactly the ordered pairs"
        );
        if let Ok(span) = admitted {
            prop_assert_eq!(span, Span::ordered(&lo, &hi));
        }
    }
}
