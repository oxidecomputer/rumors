use std::ops::{Bound, RangeBounds};

use super::*;
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
            delta(&low, &high),
            Bound::Excluded(&low),
            Bound::Included(&high),
        ),
        (
            delta_before(&low, &high),
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

/// Re-setting a bound keeps the latest value, so chains never panic and
/// always mean their final state.
#[test]
fn rebinding_a_bound_keeps_the_latest() {
    let (low, high, _) = fixtures();
    assert_eq!(since(&low).since(&high), since(&high));
    assert_eq!(since(&low).not_before(&high), not_before(&high));
    assert_eq!(known_at(&low).before(&high), before(&high));
    assert_eq!(all().since(&low), since(&low));
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

    let range = delta(&low, &high);
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
    let range = delta(&low, &high);

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
