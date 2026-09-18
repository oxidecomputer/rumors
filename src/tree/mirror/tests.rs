//! The version-containment predicate over its partial order.

use super::contained;
use crate::{Version, tree::arb::nth_party};

/// Four representative versions distinguish equality, strict containment,
/// strict dominance, and causal incomparability.
///
/// The incomparable case protects the subtle boundary: a version on a
/// disjoint party is no more contained than one strictly above the declaration.
#[test]
fn contained_distinguishes_representative_causal_relations() {
    let party = nth_party(0);
    let disjoint = nth_party(1);

    let mut declared = Version::new();
    declared.tick(&party);

    // Contained: equal, and strictly below.
    assert!(
        contained(&declared, &declared),
        "an equal bound is contained",
    );
    assert!(
        contained(&Version::new(), &declared),
        "a dominated bound is contained",
    );

    // Dominating: strictly above the declared version.
    let mut dominating = declared.clone();
    dominating.tick(&party);
    assert!(
        !contained(&dominating, &declared),
        "a strictly dominating bound is uncontained",
    );

    // Incomparable: one tick on a disjoint party.
    let mut incomparable = Version::new();
    incomparable.tick(&disjoint);
    assert!(
        !contained(&incomparable, &declared),
        "an incomparable bound is uncontained",
    );
}
