//! The judgment's own tripwires: each finding category demonstrated on
//! synthetic inputs, so the reconcile cannot rot into a scan that
//! blesses everything.

use std::collections::BTreeSet;

use super::{ANCHORS, Exception, Findings, ITEM_EXCEPTIONS, MODULE_EXCEPTIONS, reconcile_with};
use crate::extract::Surface;

/// A synthetic exception row for the pure-judgment tests.
fn exception(name: &'static str) -> Exception {
    Exception {
        name,
        reason: "synthetic test exception",
        decided: "2026-07-30",
    }
}

/// An owned name set from string literals.
fn names(entries: &[&str]) -> BTreeSet<String> {
    entries.iter().map(|n| (*n).to_owned()).collect()
}

/// A surface whose functions are the given names, with no impls and no
/// items — the shape most function-roster tests need.
fn functions(entries: &[&str]) -> Surface {
    Surface {
        functions: names(entries),
        ..Surface::default()
    }
}

/// A rostered set from string literals.
fn rostered<'a>(entries: &[&'a str]) -> BTreeSet<&'a str> {
    entries.iter().copied().collect()
}

/// A surface that exactly matches the roster and censuses (anchors
/// included) is clean: the check's green path exists.
#[test]
fn exact_match_is_clean() {
    let surface = Surface {
        functions: names(&["A::f", "B::g", "Party::seed", "causally::all"]),
        impls: names(&["A: impl core::ops::Not for A"]),
        items: names(&["A::ZERO"]),
    };
    let ro = rostered(&["A::f", "B::g", "Party::seed", "causally::all"]);
    let findings = reconcile_with(
        &surface,
        &ro,
        &[],
        &[],
        ANCHORS,
        &["A: impl core::ops::Not for A"],
        &["A::ZERO"],
    );
    assert!(findings.is_clean(), "{findings:?}");
}

/// A public item with no roster row and no exception is an unrostered
/// finding: new public surface cannot silently skip the roster.
#[test]
fn unrostered_item_reads_red() {
    let surface = functions(&["A::f", "A::new_fn", "Party::seed", "causally::all"]);
    let ro = rostered(&["A::f", "Party::seed", "causally::all"]);
    let findings = reconcile_with(&surface, &ro, &[], &[], ANCHORS, &[], &[]);
    assert_eq!(findings.unrostered, vec!["A::new_fn".to_owned()]);
    assert!(!findings.is_clean());
}

/// A roster row whose public item is gone is an orphan finding: the
/// roster cannot keep naming removed surface.
#[test]
fn orphaned_row_reads_red() {
    let surface = functions(&["Party::seed", "causally::all"]);
    let ro = rostered(&["Party::seed", "causally::all", "A::removed"]);
    let findings = reconcile_with(&surface, &ro, &[], &[], ANCHORS, &[], &[]);
    assert_eq!(findings.orphaned, vec!["A::removed".to_owned()]);
}

/// A reachable trait impl with no census pin is an unrostered-impl
/// finding, and a pin naming no reachable impl is an orphaned-impl
/// finding: the impl census reconciles both ways.
#[test]
fn impl_census_reconciles_both_ways() {
    let surface = Surface {
        functions: names(&["Party::seed", "causally::all"]),
        impls: names(&["Version: impl core::ops::Not for Version"]),
        items: BTreeSet::new(),
    };
    let ro = rostered(&["Party::seed", "causally::all"]);
    let findings = reconcile_with(
        &surface,
        &ro,
        &[],
        &[],
        ANCHORS,
        &["Version: impl core::ops::Neg for Version"],
        &[],
    );
    assert_eq!(
        findings.unrostered_impls,
        vec!["Version: impl core::ops::Not for Version".to_owned()]
    );
    assert_eq!(
        findings.orphaned_impls,
        vec!["Version: impl core::ops::Neg for Version".to_owned()]
    );
    assert!(!findings.is_clean());
}

/// A reachable const/static/macro with no census pin is an
/// unrostered-item finding, and a dead pin is an orphaned-item finding:
/// the item census reconciles both ways.
#[test]
fn item_census_reconciles_both_ways() {
    let surface = Surface {
        functions: names(&["Party::seed", "causally::all"]),
        impls: BTreeSet::new(),
        items: names(&["Rank::ZERO"]),
    };
    let ro = rostered(&["Party::seed", "causally::all"]);
    let findings = reconcile_with(&surface, &ro, &[], &[], ANCHORS, &[], &["Ticks::ZERO"]);
    assert_eq!(findings.unrostered_items, vec!["Rank::ZERO".to_owned()]);
    assert_eq!(findings.orphaned_items, vec!["Ticks::ZERO".to_owned()]);
    assert!(!findings.is_clean());
}

/// An item exception excuses exactly its named item, and a module
/// exception excuses exactly its `::`-terminated prefix — in every
/// category: functions, impls, and items under an excepted module all
/// pass without pins.
#[test]
fn exceptions_excuse_their_scope() {
    let surface = Surface {
        functions: names(&[
            "Party::seed",
            "causally::all",
            "lone::item",
            "gated::a",
            "gated::deep::b",
        ]),
        impls: names(&["gated::T: impl core::fmt::Debug for T"]),
        items: names(&["gated::CONST"]),
    };
    let ro = rostered(&["Party::seed", "causally::all"]);
    let findings = reconcile_with(
        &surface,
        &ro,
        &[exception("lone::item")],
        &[exception("gated::")],
        ANCHORS,
        &[],
        &[],
    );
    assert!(findings.is_clean(), "{findings:?}");
}

/// A module exception prefix must not match a sibling module whose name
/// merely extends it textually: `gated::` never covers `gatedmore::x`.
#[test]
fn module_exception_does_not_leak_to_siblings() {
    let surface = functions(&["Party::seed", "causally::all", "gated::a", "gatedmore::x"]);
    let ro = rostered(&["Party::seed", "causally::all"]);
    let findings = reconcile_with(
        &surface,
        &ro,
        &[],
        &[exception("gated::")],
        ANCHORS,
        &[],
        &[],
    );
    assert_eq!(findings.unrostered, vec!["gatedmore::x".to_owned()]);
}

/// An exception matching nothing is a dead entry, and an exception
/// shadowing a live roster row is a conflict: both read red, so the
/// lists self-prune.
#[test]
fn dead_and_shadowing_exceptions_read_red() {
    let surface = functions(&["Party::seed", "causally::all"]);
    let ro = rostered(&["Party::seed", "causally::all"]);
    let findings = reconcile_with(
        &surface,
        &ro,
        &[exception("gone::item"), exception("Party::seed")],
        &[exception("gonemod::")],
        ANCHORS,
        &[],
        &[],
    );
    assert_eq!(
        findings.dead_item_exceptions,
        vec!["gone::item".to_owned(), "Party::seed".to_owned()],
    );
    assert_eq!(
        findings.dead_module_exceptions,
        vec!["gonemod::".to_owned()]
    );
}

/// A module exception stays live when its only matches are impls or
/// items: an exception covering an instrument tree of impls must not
/// read as dead just because the tree declares no functions.
#[test]
fn module_exception_live_through_impls_and_items() {
    let surface = Surface {
        functions: names(&["Party::seed", "causally::all"]),
        impls: names(&["implonly::T: impl core::fmt::Debug for T"]),
        items: names(&["itemonly::CONST"]),
    };
    let ro = rostered(&["Party::seed", "causally::all"]);
    let findings = reconcile_with(
        &surface,
        &ro,
        &[],
        &[exception("implonly::"), exception("itemonly::")],
        ANCHORS,
        &[],
        &[],
    );
    assert!(findings.is_clean(), "{findings:?}");
}

/// A walk that returns nothing cannot read green: the liveness anchors
/// are missing-anchor findings even when roster and extraction agree on
/// the empty surface.
#[test]
fn empty_extraction_trips_the_anchors() {
    let findings = reconcile_with(
        &Surface::default(),
        &rostered(&[]),
        &[],
        &[],
        ANCHORS,
        &[],
        &[],
    );
    assert_eq!(findings.missing_anchors.len(), ANCHORS.len());
    assert!(!findings.is_clean());
}

/// The render names every non-empty category, so a red run always says
/// what to do next.
#[test]
fn render_names_the_findings() {
    let findings = Findings {
        unrostered: vec!["A::x".to_owned()],
        orphaned: vec!["B::y".to_owned()],
        unrostered_impls: vec!["C: impl core::ops::Not for C".to_owned()],
        unrostered_items: vec!["D::ZERO".to_owned()],
        ..Findings::default()
    };
    let report = findings.render();
    assert!(report.contains("A::x") && report.contains("B::y"));
    assert!(report.contains("C: impl core::ops::Not for C") && report.contains("D::ZERO"));
    assert!(report.contains("METHOD_SURFACE"));
    assert!(report.contains("TRAIT_IMPLS"));
}

/// The exemption discipline is enforced by the judgment itself: a
/// malformed exception is a finding.
///
/// Malformed means an undated ruling — wrong length, dashes or digits
/// out of place, or a month or day outside its range — a too-thin
/// reason, or a module prefix not ending in `::`.
#[test]
fn malformed_exceptions_read_red() {
    let surface = functions(&[
        "Party::seed",
        "causally::all",
        "a::x",
        "b::y",
        "c::z",
        "d::w",
        "e::v",
    ]);
    let ro = rostered(&["Party::seed", "causally::all"]);
    let undated = Exception {
        name: "a::x",
        reason: "a substantive reason of adequate length",
        decided: "sometime in July",
    };
    let thin = Exception {
        name: "b::y",
        reason: "because",
        decided: "2026-07-30",
    };
    // Ten characters holding two dashes, but no date: the shape check
    // must read the positions, not count characters.
    let misdashed = Exception {
        name: "d::w",
        reason: "a substantive reason of adequate length",
        decided: "20-26-07xx",
    };
    let unmonthed = Exception {
        name: "e::v",
        reason: "a substantive reason of adequate length",
        decided: "2026-13-01",
    };
    let unscoped = Exception {
        name: "c::z",
        reason: "a substantive reason of adequate length",
        decided: "2026-07-30",
    };
    let findings = reconcile_with(
        &surface,
        &ro,
        &[undated, thin, misdashed, unmonthed],
        &[unscoped],
        ANCHORS,
        &[],
        &[],
    );
    assert_eq!(
        findings.malformed_exceptions,
        vec![
            "a::x".to_owned(),
            "b::y".to_owned(),
            "d::w".to_owned(),
            "e::v".to_owned(),
            "c::z".to_owned(),
        ],
    );
}

/// The committed exception lists themselves pass the exemption
/// discipline: the shipping tables carry no malformed entry.
#[test]
fn committed_exceptions_are_well_formed() {
    let surface = functions(&["Party::seed", "causally::all"]);
    let ro = rostered(&["Party::seed", "causally::all"]);
    let findings = reconcile_with(
        &surface,
        &ro,
        ITEM_EXCEPTIONS,
        MODULE_EXCEPTIONS,
        ANCHORS,
        &[],
        &[],
    );
    assert!(
        findings.malformed_exceptions.is_empty(),
        "{:?}",
        findings.malformed_exceptions
    );
}
