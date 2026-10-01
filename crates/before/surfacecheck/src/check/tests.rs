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
    }
}

/// An owned name set from string literals.
fn names(entries: &[&str]) -> BTreeSet<String> {
    entries.iter().map(|n| (*n).to_owned()).collect()
}

/// A surface whose functions are the given names, with no impls and no
/// items — the shape most function-coverage tests need.
fn functions(entries: &[&str]) -> Surface {
    Surface {
        functions: names(entries),
        ..Surface::default()
    }
}

/// A board-operation set from string literals.
fn board<'a>(entries: &[&'a str]) -> BTreeSet<&'a str> {
    entries.iter().copied().collect()
}

/// A surface that exactly matches board coverage and the censuses (anchors
/// included) is clean: the check's green path exists.
#[test]
fn exact_match_is_clean() {
    let surface = Surface {
        functions: names(&["A::f", "B::g", "Party::seed", "causally::all"]),
        impls: names(&["A: impl core::ops::Not for A"]),
        items: names(&["A::ZERO"]),
    };
    let coverage = board(&["A::f", "B::g", "Party::seed", "causally::all"]);
    let findings = reconcile_with(
        &surface,
        &coverage,
        &[],
        &[],
        ANCHORS,
        &["A: impl core::ops::Not for A"],
        &["A::ZERO"],
    );
    assert!(findings.is_clean(), "{findings:?}");
}

/// A public item with no board row and no exception is uncovered.
#[test]
fn uncovered_item_reads_red() {
    let surface = functions(&["A::f", "A::new_fn", "Party::seed", "causally::all"]);
    let coverage = board(&["A::f", "Party::seed", "causally::all"]);
    let findings = reconcile_with(&surface, &coverage, &[], &[], ANCHORS, &[], &[]);
    assert_eq!(findings.uncovered_functions, vec!["A::new_fn".to_owned()]);
    assert!(!findings.is_clean());
}

/// A board row whose public item is gone is an orphan finding.
#[test]
fn orphaned_row_reads_red() {
    let surface = functions(&["Party::seed", "causally::all"]);
    let coverage = board(&["Party::seed", "causally::all", "A::removed"]);
    let findings = reconcile_with(&surface, &coverage, &[], &[], ANCHORS, &[], &[]);
    assert_eq!(findings.stale_board_rows, vec!["A::removed".to_owned()]);
}

/// A reachable trait impl with no census pin and a pin naming no reachable impl
/// both fail: the impl census reconciles in both directions.
#[test]
fn impl_census_reconciles_both_ways() {
    let surface = Surface {
        functions: names(&["Party::seed", "causally::all"]),
        impls: names(&["Version: impl core::ops::Not for Version"]),
        items: BTreeSet::new(),
    };
    let coverage = board(&["Party::seed", "causally::all"]);
    let findings = reconcile_with(
        &surface,
        &coverage,
        &[],
        &[],
        ANCHORS,
        &["Version: impl core::ops::Neg for Version"],
        &[],
    );
    assert_eq!(
        findings.unpinned_impls,
        vec!["Version: impl core::ops::Not for Version".to_owned()]
    );
    assert_eq!(
        findings.orphaned_impls,
        vec!["Version: impl core::ops::Neg for Version".to_owned()]
    );
    assert!(!findings.is_clean());
}

/// A reachable const, static, or macro with no census pin and a pin naming no
/// reachable item both fail: the item census reconciles in both directions.
#[test]
fn item_census_reconciles_both_ways() {
    let surface = Surface {
        functions: names(&["Party::seed", "causally::all"]),
        impls: BTreeSet::new(),
        items: names(&["Rank::ZERO"]),
    };
    let coverage = board(&["Party::seed", "causally::all"]);
    let findings = reconcile_with(
        &surface,
        &coverage,
        &[],
        &[],
        ANCHORS,
        &[],
        &["Ticks::ZERO"],
    );
    assert_eq!(findings.unpinned_items, vec!["Rank::ZERO".to_owned()]);
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
    let coverage = board(&["Party::seed", "causally::all"]);
    let findings = reconcile_with(
        &surface,
        &coverage,
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
    let coverage = board(&["Party::seed", "causally::all"]);
    let findings = reconcile_with(
        &surface,
        &coverage,
        &[],
        &[exception("gated::")],
        ANCHORS,
        &[],
        &[],
    );
    assert_eq!(
        findings.uncovered_functions,
        vec!["gatedmore::x".to_owned()]
    );
}

/// An exception matching nothing is a dead entry, and an exception
/// shadowing a live board row is a conflict: both read red, so the
/// lists self-prune.
#[test]
fn dead_and_shadowing_exceptions_read_red() {
    let surface = functions(&["Party::seed", "causally::all"]);
    let coverage = board(&["Party::seed", "causally::all"]);
    let findings = reconcile_with(
        &surface,
        &coverage,
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
    let coverage = board(&["Party::seed", "causally::all"]);
    let findings = reconcile_with(
        &surface,
        &coverage,
        &[],
        &[exception("implonly::"), exception("itemonly::")],
        ANCHORS,
        &[],
        &[],
    );
    assert!(findings.is_clean(), "{findings:?}");
}

/// A walk that returns nothing cannot read green: the liveness anchors
/// are missing-anchor findings even when coverage and extraction agree on
/// the empty surface.
#[test]
fn empty_extraction_trips_the_anchors() {
    let findings = reconcile_with(
        &Surface::default(),
        &board(&[]),
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
        uncovered_functions: vec!["A::x".to_owned()],
        stale_board_rows: vec!["B::y".to_owned()],
        unpinned_impls: vec!["C: impl core::ops::Not for C".to_owned()],
        unpinned_items: vec!["D::ZERO".to_owned()],
        ..Findings::default()
    };
    let report = findings.render();
    assert!(report.contains("A::x") && report.contains("B::y"));
    assert!(report.contains("C: impl core::ops::Not for C") && report.contains("D::ZERO"));
    assert!(report.contains("BOARD_PRICED"));
    assert!(report.contains("TRAIT_IMPLS"));
}

/// The exemption discipline is enforced by the judgment itself: a
/// malformed exception is a finding.
///
/// Malformed means a too-thin reason or a module prefix not ending in `::`.
#[test]
fn malformed_exceptions_read_red() {
    let surface = functions(&["Party::seed", "causally::all", "a::x", "b::y", "c::z"]);
    let coverage = board(&["Party::seed", "causally::all"]);
    let thin = Exception {
        name: "a::x",
        reason: "because",
    };
    let unscoped = Exception {
        name: "c::z",
        reason: "a substantive reason of adequate length",
    };
    let findings = reconcile_with(&surface, &coverage, &[thin], &[unscoped], ANCHORS, &[], &[]);
    assert_eq!(
        findings.malformed_exceptions,
        vec!["a::x".to_owned(), "c::z".to_owned(),],
    );
}

/// The committed exception lists themselves pass the exemption
/// discipline: the shipping tables carry no malformed entry.
#[test]
fn committed_exceptions_are_well_formed() {
    let surface = functions(&["Party::seed", "causally::all"]);
    let coverage = board(&["Party::seed", "causally::all"]);
    let findings = reconcile_with(
        &surface,
        &coverage,
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
