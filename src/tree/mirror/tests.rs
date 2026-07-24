//! The version-containment predicate over its partial order.

use super::contained;
use crate::{Version, tree::arb::nth_party};

/// `contained` accepts exactly the causally-at-or-below regime: an equal or
/// dominated bound passes, while a bound strictly above *or incomparable
/// with* the declared version is uncontained.
///
/// The incomparable case is the reason the predicate is named at all: a
/// bare `!(a <= b)` invites the misreading that only strict dominance is
/// rejected, when on a partial order an escape onto a disjoint party is
/// just as uncontained.
#[test]
fn contained_covers_all_three_regimes() {
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
