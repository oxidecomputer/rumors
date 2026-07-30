use std::cmp::Ordering;
use std::ops::{Bound, RangeBounds};

use proptest::prelude::*;
use proptest::test_runner::TestCaseError;

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
fn span_new_unchecked_is_the_validated_span() {
    let ([_, a2, _, a4, _], _) = span_fixtures();
    assert_eq!(Span::new_unchecked(&a2, &a4), Span::new(&a2, &a4).unwrap());
    assert_eq!(Span::new_unchecked(&a2, &a2), Span::new(&a2, &a2).unwrap());
}

/// The trusted door's debug assertion catches a violated guarantee: an
/// unordered pair panics in debug builds rather than constructing an
/// span whose verdicts would be meaningless.
#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "Span::new_unchecked requires lo <= hi")]
fn span_new_unchecked_asserts_the_guarantee_in_debug() {
    let ([_, a2, _, a4, _], _) = span_fixtures();
    let _ = Span::new_unchecked(&a4, &a2);
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
            prop_assert_eq!(span, Span::new_unchecked(&lo, &hi));
        }
    }
}

// ───────────────────────────── the span wire form ─────────────────────────────

/// Committed witnesses, one per rejection genre the span wire decode
/// can reach.
///
/// A strictly crossed pair, a concurrent pair (both orders), a
/// non-canonical component on each side of the seam, truncation at
/// every byte boundary (inside the meet, at the seam with the join
/// missing entirely, inside the join), a trailing byte after the
/// complete composite, and a set padding bit inside each component's
/// final byte.
#[test]
fn span_decode_rejects_each_genre() {
    use crate::error::Decode;
    let mut alice = Clock::seed();
    let mut bob = alice.fork();
    let older = alice.tick().clone();
    let newer = alice.tick().clone();
    let beside = bob.tick().clone(); // concurrent to alice's whole line
    let bytes = Span::new(&older, &newer).unwrap().encode();
    let lo_len = older.encode().len();

    // The accepted baseline the witnesses below are each one defect away
    // from.
    assert_eq!(
        Span::decode(&bytes[..]).unwrap(),
        Span::new(&older, &newer).unwrap()
    );

    // Strictly crossed: the join strictly below the meet.
    let crossed = [newer.encode(), older.encode()].concat();
    assert!(
        matches!(Span::decode(&crossed[..]), Err(Decode::NotCanonical)),
        "a strictly crossed pair is the canonical spelling of no span"
    );

    // Concurrent: neither component bounds the other, in both orders.
    for (a, b) in [(&older, &beside), (&beside, &older)] {
        let concurrent = [a.encode(), b.encode()].concat();
        assert!(
            matches!(Span::decode(&concurrent[..]), Err(Decode::NotCanonical)),
            "a concurrent pair is the canonical spelling of no span"
        );
    }

    // Truncation at every byte boundary. Every cut lands mid-tree: a
    // component's final byte always carries live bits (encode pads only
    // to the next byte boundary), so no proper byte prefix parses whole.
    assert!(
        matches!(Span::decode(&[][..]), Err(Decode::Truncated)),
        "empty input"
    );
    for cut in 1..bytes.len() {
        let genre = match cut.cmp(&lo_len) {
            Ordering::Less => "inside the meet",
            Ordering::Equal => "at the seam: the join missing entirely",
            Ordering::Greater => "inside the join",
        };
        assert!(
            matches!(Span::decode(&bytes[..cut]), Err(Decode::Truncated)),
            "cut at byte {cut} ({genre})"
        );
    }

    // Live bits past the complete composite.
    let trailing = [bytes.clone(), vec![0]].concat();
    assert!(
        matches!(Span::decode(&trailing[..]), Err(Decode::TrailingBits)),
        "trailing zero byte"
    );

    // A set padding bit inside each component's final byte. Both
    // witnesses must end mid-byte for the padding to exist at all.
    assert_ne!(
        older.encoded_bits() % 8,
        0,
        "the meet witness ends mid-byte"
    );
    assert_ne!(
        newer.encoded_bits() % 8,
        0,
        "the join witness ends mid-byte"
    );
    let mut meet_padding = bytes.clone();
    meet_padding[lo_len - 1] |= 0x01;
    assert!(
        matches!(Span::decode(&meet_padding[..]), Err(Decode::TrailingBits)),
        "set bit in the meet's padding"
    );
    let mut join_padding = bytes.clone();
    *join_padding.last_mut().unwrap() |= 0x01;
    assert!(
        matches!(Span::decode(&join_padding[..]), Err(Decode::TrailingBits)),
        "set bit in the join's padding"
    );

    // A non-canonical component on each side of the seam: an internal
    // node whose two leaf children carry height 0 and delta 0 — the
    // collapsible sibling pair minimal topology forbids. As a *join* it
    // denotes the empty version, so it dominates an empty meet and only
    // canonicality can reject it — which is exactly the check the fused
    // walk must not lose.
    let collapsible: Vec<u8> = vec![0b0111_1000];
    assert!(
        matches!(Version::decode(&collapsible[..]), Err(Decode::NotCanonical)),
        "the component witness is itself non-canonical"
    );
    let empty = Version::new().encode();
    let meet_noncanon = [collapsible.clone(), empty.clone()].concat();
    assert!(
        matches!(Span::decode(&meet_noncanon[..]), Err(Decode::NotCanonical)),
        "non-canonical meet"
    );
    let join_noncanon = [empty, collapsible].concat();
    assert!(
        matches!(Span::decode(&join_noncanon[..]), Err(Decode::NotCanonical)),
        "non-canonical join"
    );
}

/// FUSED-VALIDATE VERDICT IDENTITY, exhaustively at small scope.
///
/// Over every ordered pair of normal-form event trees to the committed
/// depth bound, the fused wire decode of `lo.encode() ++ hi.encode()`
/// agrees with the composed form — decode each component, then
/// validate with `Span::new` — accepting exactly the same composites,
/// producing the same span on every accept, and rejecting every
/// crossed or concurrent pair as `NotCanonical`. The corpus reaches
/// ordered, reversed, coincident, and concurrent pairs by brute force,
/// and the liveness floors prove both verdicts fired at scale.
#[test]
fn span_decode_verdict_matches_the_composed_form_exhaustively() {
    use crate::error::Decode;
    use crate::testing::exhaustive::{all_normal_events, EV_SMALL_DEPTH};
    let corpus: Vec<Version> = all_normal_events(EV_SMALL_DEPTH)
        .iter()
        .map(from_oracle_version)
        .collect();
    let encodings: Vec<Vec<u8>> = corpus.iter().map(Version::encode).collect();
    let (mut accepted, mut rejected) = (0u64, 0u64);
    for (lo, lo_bytes) in corpus.iter().zip(&encodings) {
        for (hi, hi_bytes) in corpus.iter().zip(&encodings) {
            let composite = [lo_bytes.as_slice(), hi_bytes.as_slice()].concat();
            let fused = Span::decode(&composite[..]);
            match Span::new(lo, hi) {
                Ok(span) => {
                    accepted += 1;
                    match fused {
                        Ok(decoded) => assert_eq!(
                            decoded, span,
                            "the fused decode's accept is the composed span"
                        ),
                        Err(e) => {
                            panic!("fused decode must accept the ordered pair [{lo}, {hi}]: {e}")
                        }
                    }
                }
                Err(Crossed) => {
                    rejected += 1;
                    assert!(
                        matches!(fused, Err(Decode::NotCanonical)),
                        "fused decode must reject the unordered pair [{lo}, {hi}] as NotCanonical"
                    );
                }
            }
        }
    }
    // Liveness: both verdicts fire, at scale.
    assert!(accepted > 1_000, "acceptance is live: {accepted}");
    assert!(rejected > 1_000, "rejection is live: {rejected}");
}

proptest! {
    /// FUSED-VALIDATE VERDICT IDENTITY over arbitrary pairs.
    ///
    /// The fused wire decode of `a.encode() ++ b.encode()` agrees with
    /// the composed form (decode each component, then `Span::new`) on
    /// both verdicts, and on every accept the two forms produce the
    /// same span.
    #[test]
    fn span_decode_verdict_matches_the_composed_form(
        oa in arb_oracle_version(),
        ob in arb_oracle_version(),
    ) {
        use crate::error::Decode;
        let a = from_oracle_version(&oa);
        let b = from_oracle_version(&ob);
        let composite = [a.encode(), b.encode()].concat();
        let fused = Span::decode(&composite[..]);
        match Span::new(&a, &b) {
            Ok(span) => match fused {
                Ok(decoded) => prop_assert_eq!(
                    decoded, span,
                    "the fused decode's accept is the composed span"
                ),
                Err(e) => return Err(TestCaseError::fail(format!(
                    "fused decode must accept the ordered pair [{a}, {b}]: {e}"
                ))),
            },
            Err(Crossed) => prop_assert!(
                matches!(fused, Err(Decode::NotCanonical)),
                "fused decode must reject the unordered pair [{a}, {b}] as NotCanonical"
            ),
        }
    }

    /// The span composite is prefix-free: distinct spans' encodings are
    /// never byte prefixes of one another.
    ///
    /// Pinned directly on the composite (it rides the components'
    /// committed prefix-freedom, but the pin is on the composite
    /// itself, never inferred). Prefix-freedom is what lets one
    /// composite self-delimit inside a larger stream: the borsh leg
    /// reads exactly one span and leaves the next field's bytes
    /// unread.
    #[test]
    fn span_encoding_is_prefix_free(
        oa in arb_oracle_version(),
        ob in arb_oracle_version(),
        oc in arb_oracle_version(),
        od in arb_oracle_version(),
    ) {
        let x = from_oracle_version(&oa).span(&from_oracle_version(&ob));
        let y = from_oracle_version(&oc).span(&from_oracle_version(&od));
        if x != y {
            let (ex, ey) = (x.encode(), y.encode());
            prop_assert!(
                !ex.starts_with(&ey) && !ey.starts_with(&ex),
                "prefix-free: {:02x?} vs {:02x?}", ex, ey
            );
        }
    }
}

/// Structural genres outrank the pair verdict on multiply-defective
/// composites, exactly as decoding the components would order them.
///
/// Each witness stacks a second defect on a composite the pair
/// relation already rejects, and the structural genre wins: a set
/// padding bit or a spurious trailing byte after a crossed join is
/// `TrailingBits`, a cut after an early refutation is `Truncated` —
/// never the pair rejection's `NotCanonical`. The negative-height
/// witnesses pin the admission walk's subsumption seam: a whole
/// negative-height join rejects `NotCanonical` (the same genre the
/// standalone validator gives those bytes), and a negative-height
/// join that is *also* truncated rejects `Truncated` — the one
/// deliberate divergence from component-wise decoding, which reports
/// the height dip it meets first; the fused walk carries no height
/// accumulator, so the whole-parse rule decides instead.
#[test]
fn span_decode_structural_genres_outrank_the_pair_verdict() {
    use crate::error::Decode;
    let mut clock = Clock::seed();
    let one = clock.tick().clone();
    let empty = Version::new();

    // The empty version's canonical byte: leaf flag `1`, gamma(0) `1`,
    // six zero padding bits.
    assert_eq!(empty.encode(), vec![0xC0]);
    // A negative-height stream: root internal `0`, left leaf `1` with
    // absolute height gamma(0) `1`, right leaf `1` with delta
    // zigzag(-1) `010`, one zero padding bit — 0b0111_0100. Its running
    // height dips to -1, which only canonicality rejects.
    let neg = vec![0x74];
    assert!(matches!(
        Version::decode(&neg[..]),
        Err(Decode::NotCanonical)
    ));

    // A whole negative-height join over the empty meet: the join never
    // dominates (its dip sits below the meet's zero), so the admission
    // verdict subsumes the height check under the same genre.
    let composite = [empty.encode(), neg].concat();
    assert!(
        matches!(Span::decode(&composite[..]), Err(Decode::NotCanonical)),
        "a whole negative-height join rejects as the validator would"
    );

    // The same stream cut before its right subtree: 0b0011_1010 parses
    // root internal, left-inner leaf height 0, then delta zigzag(-1) —
    // the dip — and then runs out of bits. The structural genre wins.
    let truncated_neg = [empty.encode(), vec![0x3A]].concat();
    assert!(
        matches!(Span::decode(&truncated_neg[..]), Err(Decode::Truncated)),
        "truncation outranks the refuted pair verdict"
    );

    // A crossed pair (join strictly below the meet) with a set padding
    // bit in the join's final byte: the padding defect wins.
    let crossed_padding = [one.encode(), vec![0xC4]].concat();
    assert!(
        matches!(
            Span::decode(&crossed_padding[..]),
            Err(Decode::TrailingBits)
        ),
        "nonzero padding outranks the refuted pair verdict"
    );

    // A crossed pair with a spurious all-zero byte after the join: the
    // composite re-encoding shorter than its input is the same
    // trailing-bits genre.
    let crossed_trailing = [one.encode(), vec![0xC0, 0x00]].concat();
    assert!(
        matches!(
            Span::decode(&crossed_trailing[..]),
            Err(Decode::TrailingBits)
        ),
        "a trailing zero byte outranks the refuted pair verdict"
    );

    // A crossed pair whose join is also cut mid-tree: refutation is
    // decided early, and the walk still parses to the cut.
    let taller = {
        let mut main = Clock::seed();
        let mut other = main.fork();
        other.tick();
        main.recv(other.send());
        main.tick();
        main.version().clone()
    };
    let join = {
        let mut main = Clock::seed();
        let mut other = main.fork();
        other.tick();
        main.recv(other.send());
        main.version().clone()
    };
    let join_bytes = join.encode();
    let crossed_truncated = [taller.encode(), join_bytes[..join_bytes.len() - 1].to_vec()].concat();
    assert!(
        matches!(Span::decode(&crossed_truncated[..]), Err(Decode::Truncated)),
        "truncation outranks the refuted pair verdict"
    );
}

/// FUSED-VALIDATE VERDICT IDENTITY beyond the exhaustive corpus's
/// reach: deep spines, wide fans, and payload magnitudes at and past
/// the machine word, on both verdicts.
///
/// The small-scope sweep is exhaustive to depth 2; these constructed
/// families sample the genres it cannot contain — 300-level spines
/// (deep path stacks, long unary runs), 1024-leaf fans (maximal-width
/// plateaus), absolute heights above `2^64` (payload codes past the
/// decoder's word window), and heights at the 63/64-bit sign edges of
/// the zigzag map — and check the fused decode against the composed
/// form (decode, decode, `Span::new`) on accept, reject, and the
/// decoded span itself, for the pair, its hulls, and the coincident
/// span.
#[test]
fn span_decode_verdict_matches_the_composed_form_off_corpus() {
    use crate::error::Decode;
    use crate::oracle;

    fn composed(bytes: &[u8], seam: usize) -> Result<Span<'static>, Decode> {
        let lo = Version::decode(&bytes[..seam])?;
        let hi = Version::decode(&bytes[seam..])?;
        Span::new(&lo, &hi)
            .map(|s| s.into_owned())
            .map_err(|Crossed| Decode::NotCanonical)
    }

    fn check_identity(lo: &Version, hi: &Version) {
        let lo_bytes = lo.encode();
        let seam = lo_bytes.len();
        let composite = [lo_bytes, hi.encode()].concat();
        let fused = Span::decode(&composite[..]);
        match (fused, composed(&composite, seam)) {
            (Ok(f), Ok(c)) => {
                assert_eq!(f, c, "accept identity for [{lo}, {hi}]");
                assert_eq!(f.encode(), composite, "re-encode identity");
            }
            (Err(ef), Err(ec)) => assert_eq!(
                std::mem::discriminant(&ef),
                std::mem::discriminant(&ec),
                "genre identity for [{lo}, {hi}]: fused {ef:?}, composed {ec:?}"
            ),
            (f, c) => panic!("verdict mismatch for [{lo}, {hi}]: fused {f:?}, composed {c:?}"),
        }
    }

    // A left-descending spine: every level one internal node whose
    // right child is a leaf.
    let spine = |depth: usize, bump: u64| {
        let mut t = oracle::Version::leaf(0u64);
        for i in 0..depth {
            let i = i as u64;
            t = oracle::Version::node(i % 7 + bump, t, oracle::Version::leaf(i % 3));
        }
        t
    };
    // A complete tree: `2^depth` leaves with mixed heights.
    fn fan(depth: usize, salt: u64) -> oracle::Version {
        fn go(d: usize, ix: u64, salt: u64) -> oracle::Version {
            if d == 0 {
                oracle::Version::leaf(ix.wrapping_mul(2654435761).wrapping_add(salt) % 5)
            } else {
                oracle::Version::node(ix % 2, go(d - 1, ix * 2, salt), go(d - 1, ix * 2 + 1, salt))
            }
        }
        go(depth, 1, salt)
    }
    // Nested `u64::MAX` bases: absolute heights above `2^64`, so the
    // payload gamma codes outgrow the decoder's word window.
    let giant = |extra: u64| {
        oracle::Version::node(
            u64::MAX,
            oracle::Version::node(
                u64::MAX,
                oracle::Version::leaf(extra),
                oracle::Version::leaf(0u64),
            ),
            oracle::Version::leaf(1u64),
        )
    };
    // One height at a chosen bit edge beside a zero leaf.
    let bit_edge =
        |h: u64| oracle::Version::node(0u64, oracle::Version::leaf(h), oracle::Version::leaf(0u64));

    let shapes = [
        spine(300, 0),
        spine(300, 1),
        spine(120, 3),
        fan(10, 0),
        fan(10, 9),
        fan(6, 4),
        giant(0),
        giant(5),
        bit_edge((1u64 << 63) - 1),
        bit_edge(1u64 << 63),
        bit_edge((1u64 << 62) + 1),
        bit_edge(u64::MAX),
        oracle::Version::leaf(0u64),
    ];
    let versions: Vec<Version> = shapes.iter().map(from_oracle_version).collect();
    for a in &versions {
        for b in &versions {
            // The raw pair (ordered, crossed, or concurrent), the
            // hull against each operand (always ordered), and the
            // coincident span.
            check_identity(a, b);
            let hull = a.span(b);
            check_identity(a, hull.join());
            check_identity(hull.meet(), b);
            check_identity(a, a);
        }
    }
}

proptest! {
    /// A single-bit mutation of a valid composite never aliases it:
    /// the mutated bytes are rejected, or they decode to a *different*
    /// span whose canonical encoding is the mutated composite itself —
    /// the span-level face of the components' mutation sweeps, crossing
    /// the seam and both padding regions that only the composite has.
    #[test]
    fn span_single_bit_mutations_never_alias(
        oa in arb_oracle_version(),
        ob in arb_oracle_version(),
        flip_seed in any::<prop::sample::Index>(),
    ) {
        let a = from_oracle_version(&oa);
        let b = from_oracle_version(&ob);
        let span = a.span(&b);
        let mut bytes = span.encode();
        let flip = flip_seed.index(bytes.len() * 8);
        bytes[flip / 8] ^= 0x80 >> (flip % 8);
        match Span::decode(&bytes[..]) {
            Err(_) => {}
            Ok(mutant) => {
                prop_assert_ne!(
                    &mutant, &span,
                    "a single-bit mutation decoded back to the same span: \
                     two spellings of one value were both accepted"
                );
                prop_assert_eq!(
                    mutant.encode(), bytes,
                    "an accepted composite must be the canonical encoding of its span"
                );
            }
        }
    }
}
