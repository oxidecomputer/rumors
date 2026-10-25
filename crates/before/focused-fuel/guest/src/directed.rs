//! Probes of directed comparisons, which may stop once their requested
//! ordering is impossible, against the full relation.
//!
//! The operand `later` is the comb `C(1024, teeth)`, whose first leaf sits at
//! `2^1024 − 1`, so every directed question about it against the empty
//! version is answered at the first elementary interval, while the comb's
//! tail grows with the probe's tooth count.

use std::cmp::Ordering;

use before::testing::meter::registry::Shape;
use before::{causally, Dominance, Party, Span, Version};

use crate::measured;

/// Builds the comb `C(1024, teeth)`.
fn later(teeth: usize) -> Version {
    let later = Shape::CliffComb.build2(1_024, teeth).version();
    assert!(
        Version::new() < later,
        "the comb lies above the empty version"
    );
    later
}

/// Measures `later <= empty`.
pub fn le(teeth: usize) -> u64 {
    let (later, empty) = (later(teeth), Version::new());
    measured(
        || later <= empty,
        |ordered| assert!(!ordered, "later is above empty"),
    )
}

/// Measures `later.partial_cmp(empty)`.
pub fn full_compare(teeth: usize) -> u64 {
    let (later, empty) = (later(teeth), Version::new());
    measured(
        || later.partial_cmp(&empty),
        |order| assert_eq!(order, Some(Ordering::Greater), "later is above empty"),
    )
}

/// Measures `Span::at(later).dominance(empty)`.
pub fn point_dominance(teeth: usize) -> u64 {
    let (later, empty) = (later(teeth), Version::new());
    let point = Span::at(&later);
    measured(
        || point.dominance(&empty),
        |verdict| assert_eq!(verdict, Dominance::Before, "empty does not dominate later"),
    )
}

/// Measures `causally::after(later).contains(empty)`.
pub fn floor_contains(teeth: usize) -> u64 {
    let (later, empty) = (later(teeth), Version::new());
    let floor = causally::after(&later);
    measured(
        || floor.contains(&empty),
        |held| assert!(!held, "empty is not after later"),
    )
}

/// Measures `projection <= empty`, the spine viewed through the seed.
pub fn projected_le(teeth: usize) -> u64 {
    let (later, empty, seed) = (later(teeth), Version::new(), Party::seed());
    let projection = &later / &seed;
    measured(
        || projection <= empty,
        |ordered| assert!(!ordered, "the projection is above empty"),
    )
}

/// Measures `projection.partial_cmp(empty)`, the spine viewed through the
/// seed.
pub fn projected_compare(teeth: usize) -> u64 {
    let (later, empty, seed) = (later(teeth), Version::new(), Party::seed());
    let projection = &later / &seed;
    measured(
        || projection.partial_cmp(&empty),
        |order| {
            assert_eq!(
                order,
                Some(Ordering::Greater),
                "the projection is above empty"
            )
        },
    )
}
