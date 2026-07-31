//! The judgment: extracted surface against the roster and the pinned
//! censuses, with the committed exception lists and the scan-liveness
//! anchors.
//!
//! Functions reconcile against `METHOD_SURFACE`; trait impls and
//! non-function items reconcile against the pinned lists in
//! [`crate::census`]. Every exception is named, dated, and reasoned —
//! the registry exemption-list discipline — and every exception and
//! census entry must still *match* something: an entry whose item
//! vanished is a dead entry this check fails on, so the lists are
//! self-pruning. The anchors are the extractor's liveness floor: a walk
//! that returns nothing (or the wrong tree) cannot pass, because two
//! known-public items must be present by name; the censuses need no
//! separate anchor because their reconciliation is two-way — an impl
//! walk that goes silent orphans every pinned row at once.

use std::collections::BTreeSet;

use crate::census;
use crate::extract::Surface;

#[cfg(test)]
mod tests;

/// One named exception: a public function-like item deliberately outside
/// the roster, with the ruling of record.
pub(crate) struct Exception {
    /// The extracted name (item exceptions) or `::`-terminated path
    /// prefix (module exceptions) this ruling covers.
    pub name: &'static str,
    /// Why this surface earns no roster row.
    pub reason: &'static str,
    /// The date of the ruling of record (YYYY-MM-DD).
    pub decided: &'static str,
}

/// Per-item exceptions: none at this tip. An entry here is a deliberate,
/// owner-reviewable ruling that one public function-like item stays off
/// the roster.
pub(crate) const ITEM_EXCEPTIONS: &[Exception] = &[];

/// Module-scope exceptions: entire public trees deliberately outside the
/// roster, each a feature-gated instrument surface with its own totality
/// discipline.
///
/// Prefixes must end in `::` so they can never match a sibling module by
/// accident.
pub(crate) const MODULE_EXCEPTIONS: &[Exception] = &[
    Exception {
        name: "meter::",
        reason: "adversarial generators, deterministic resource meters, and the \
                 kernel-seam re-exports the envelope suite drives (the `skyline` \
                 transcoder spellings of kernels whose API spellings hold roster \
                 rows), public under the `meter` feature for the instrument \
                 binaries alone and never part of a production build; the tree's \
                 totality discipline is the family registry's own (the \
                 compiler-forced constructor table and the registry pins), and \
                 the differential roster excludes it by the meter/error/iter \
                 plumbing family row",
        decided: "2026-07-30",
    },
    Exception {
        name: "oracle::",
        reason: "the paper-transcription reference oracle, public under the \
                 `oracle` feature so the bench suite can time it; it is the \
                 differential architecture's ground truth, not a surface to bind \
                 against itself",
        decided: "2026-07-30",
    },
    Exception {
        name: "surface::",
        reason: "the roster's own row accessors (`Leg::cited`, \
                 `Leg::exclusion_reason`), public under the `meter` feature so \
                 instrument crates can read the rows; rostering the roster's \
                 accessors in itself would be circular",
        decided: "2026-07-30",
    },
    Exception {
        name: "laws::",
        reason: "the named algebraic-law predicate tables (statics of law \
                 rows), public under the `laws` feature so the fuzz workspace \
                 can drive the same collection the in-tree proptests assert; \
                 the laws are instruments over the rostered operations, not \
                 surface to roster against itself",
        decided: "2026-07-31",
    },
];

/// The extractor's liveness anchors: known-public items that must be
/// extracted by name, so a walk that silently returns nothing (or walks
/// the wrong tree) reads red instead of green.
///
/// One root re-export method and one public-module free function, so
/// both naming paths (bare `Type::fn`, `module::fn`) are proven live.
pub(crate) const ANCHORS: &[&str] = &["Party::seed", "causally::all"];

/// Everything a sweep can find; empty on a clean tree.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Findings {
    /// Public function-like items with no roster row and no exception.
    pub unrostered: Vec<String>,
    /// Roster rows naming no reachable public item.
    pub orphaned: Vec<String>,
    /// Reachable trait impls with no census pin and no module exception.
    pub unrostered_impls: Vec<String>,
    /// Census impl pins naming no reachable trait impl.
    pub orphaned_impls: Vec<String>,
    /// Reachable consts, statics, associated types, or macros with no
    /// census pin and no module exception.
    pub unrostered_items: Vec<String>,
    /// Census item pins naming no reachable item.
    pub orphaned_items: Vec<String>,
    /// Item exceptions matching no extracted item, or also rostered
    /// (an exception may never shadow a roster row).
    pub dead_item_exceptions: Vec<String>,
    /// Module exceptions matching nothing in any extracted category.
    pub dead_module_exceptions: Vec<String>,
    /// Liveness anchors missing from the extraction.
    pub missing_anchors: Vec<String>,
    /// Exceptions violating the exemption discipline: undated rulings,
    /// too-thin reasons, or module prefixes not ending in `::`.
    pub malformed_exceptions: Vec<String>,
}

impl Findings {
    /// Whether the sweep found nothing.
    pub fn is_clean(&self) -> bool {
        *self == Findings::default()
    }

    /// The findings as a report, one guidance-bearing block per
    /// category.
    pub fn render(&self) -> String {
        let mut out = String::new();
        let mut block = |title: &str, guidance: &str, names: &[String]| {
            if names.is_empty() {
                return;
            }
            out.push_str(&format!("surfacecheck: {title} ({}):\n", names.len()));
            for name in names {
                out.push_str(&format!("  {name}\n"));
            }
            out.push_str(&format!("  -> {guidance}\n"));
        };
        block(
            "public function-like items with no roster row and no exception",
            "add a METHOD_SURFACE row naming each leg's disposition \
             (crates/before/src/surface.rs), or a dated exception in \
             crates/before/surfacecheck/src/check.rs",
            &self.unrostered,
        );
        block(
            "roster rows naming no reachable public item",
            "remove or rename the row in crates/before/src/surface.rs",
            &self.orphaned,
        );
        block(
            "reachable trait impls with no census pin and no exception",
            "a new impl is a deliberate API event: pin it in TRAIT_IMPLS \
             (crates/before/surfacecheck/src/census.rs) and, for a new \
             operator or trait family, add the FAMILY_SURFACE row \
             (crates/before/src/surface.rs) naming its leg dispositions",
            &self.unrostered_impls,
        );
        block(
            "census impl pins naming no reachable trait impl",
            "remove or rename the pin in \
             crates/before/surfacecheck/src/census.rs",
            &self.orphaned_impls,
        );
        block(
            "reachable consts, statics, associated types, or macros with \
             no census pin and no exception",
            "pin the item in ITEMS \
             (crates/before/surfacecheck/src/census.rs)",
            &self.unrostered_items,
        );
        block(
            "census item pins naming no reachable item",
            "remove or rename the pin in \
             crates/before/surfacecheck/src/census.rs",
            &self.orphaned_items,
        );
        block(
            "dead or roster-shadowing item exceptions",
            "remove the exception, or resolve the conflict with its roster row",
            &self.dead_item_exceptions,
        );
        block(
            "module exceptions matching no extracted item",
            "remove the module exception",
            &self.dead_module_exceptions,
        );
        block(
            "liveness anchors missing from the extraction",
            "the extractor walked the wrong tree; fix surfacecheck before \
             trusting any of its verdicts",
            &self.missing_anchors,
        );
        block(
            "exceptions violating the exemption discipline",
            "every exception carries a YYYY-MM-DD ruling date and a substantive \
             reason, and module prefixes end in `::`",
            &self.malformed_exceptions,
        );
        out
    }
}

/// Reconcile the extracted surface against the roster and the pinned
/// censuses, under the committed exception lists and anchors.
pub(crate) fn reconcile(surface: &Surface, rostered: &BTreeSet<&str>) -> Findings {
    reconcile_with(
        surface,
        rostered,
        ITEM_EXCEPTIONS,
        MODULE_EXCEPTIONS,
        ANCHORS,
        census::TRAIT_IMPLS,
        census::ITEMS,
    )
}

/// [`reconcile`] over explicit exception, anchor, and census tables, so
/// the judgment is a pure function the unit tests drive with synthetic
/// inputs.
pub(crate) fn reconcile_with(
    surface: &Surface,
    rostered: &BTreeSet<&str>,
    item_exceptions: &[Exception],
    module_exceptions: &[Exception],
    anchors: &[&str],
    impl_census: &[&str],
    item_census: &[&str],
) -> Findings {
    let extracted = &surface.functions;
    let excepted_items: BTreeSet<&str> = item_exceptions.iter().map(|e| e.name).collect();
    let module_covers = |name: &str| module_exceptions.iter().any(|e| name.starts_with(e.name));
    // Census reconciliation is two-way set equality outside the
    // module-excepted trees: an extracted row without a pin is
    // unrostered, a pin without a row is orphaned.
    let census_diff = |extracted: &BTreeSet<String>, pinned: &[&str]| {
        let pinned_set: BTreeSet<&str> = pinned.iter().copied().collect();
        let unrostered: Vec<String> = extracted
            .iter()
            .filter(|row| !pinned_set.contains(row.as_str()) && !module_covers(row))
            .cloned()
            .collect();
        let orphaned: Vec<String> = pinned
            .iter()
            .filter(|row| !extracted.contains(**row))
            .map(|row| (*row).to_owned())
            .collect();
        (unrostered, orphaned)
    };
    let (unrostered_impls, orphaned_impls) = census_diff(&surface.impls, impl_census);
    let (unrostered_items, orphaned_items) = census_diff(&surface.items, item_census);
    // A decision date is a real `YYYY-MM-DD` shape — digits in the digit
    // positions, dashes at positions 4 and 7, month 01-12, day 01-31 —
    // never merely ten characters holding two dashes somewhere.
    let dated = |e: &Exception| {
        let bytes = e.decided.as_bytes();
        let digits = |range: std::ops::Range<usize>| bytes[range].iter().all(u8::is_ascii_digit);
        bytes.len() == 10
            && bytes[4] == b'-'
            && bytes[7] == b'-'
            && digits(0..4)
            && digits(5..7)
            && digits(8..10)
            && (1..=12).contains(&e.decided[5..7].parse::<u8>().unwrap_or(0))
            && (1..=31).contains(&e.decided[8..10].parse::<u8>().unwrap_or(0))
    };
    let mut malformed_exceptions: Vec<String> = item_exceptions
        .iter()
        .chain(module_exceptions)
        .filter(|e| !dated(e) || e.reason.len() < 20)
        .map(|e| e.name.to_owned())
        .collect();
    malformed_exceptions.extend(
        module_exceptions
            .iter()
            .filter(|e| !e.name.ends_with("::"))
            .map(|e| e.name.to_owned()),
    );
    Findings {
        unrostered: extracted
            .iter()
            .filter(|name| {
                !rostered.contains(name.as_str())
                    && !excepted_items.contains(name.as_str())
                    && !module_covers(name)
            })
            .cloned()
            .collect(),
        orphaned: rostered
            .iter()
            .filter(|op| !extracted.contains(**op))
            .map(|op| (*op).to_owned())
            .collect(),
        unrostered_impls,
        orphaned_impls,
        unrostered_items,
        orphaned_items,
        dead_item_exceptions: item_exceptions
            .iter()
            .map(|e| e.name)
            .filter(|name| !extracted.contains(*name) || rostered.contains(name))
            .map(str::to_owned)
            .collect(),
        dead_module_exceptions: module_exceptions
            .iter()
            .map(|e| e.name)
            .filter(|prefix| {
                let covers = |name: &String| name.starts_with(prefix);
                !extracted.iter().any(covers)
                    && !surface.impls.iter().any(covers)
                    && !surface.items.iter().any(covers)
            })
            .map(str::to_owned)
            .collect(),
        missing_anchors: anchors
            .iter()
            .filter(|anchor| !extracted.contains(**anchor))
            .map(|anchor| (*anchor).to_owned())
            .collect(),
        malformed_exceptions,
    }
}
