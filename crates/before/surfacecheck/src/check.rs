//! The judgment: extracted surface against the roster, with the
//! committed exception lists and the scan-liveness anchors.
//!
//! Every exception is named, dated, and reasoned — the registry
//! exemption-list discipline — and every exception must still *match*
//! something: an exception whose item vanished is a dead entry this
//! check fails on, so the lists are self-pruning. The anchors are the
//! extractor's liveness floor: a walk that returns nothing (or the wrong
//! tree) cannot pass, because two known-public items must be present by
//! name.

use std::collections::BTreeSet;

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
/// discipline. Prefixes must end in `::` so they can never match a
/// sibling module by accident.
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
    /// Item exceptions matching no extracted item, or also rostered
    /// (an exception may never shadow a roster row).
    pub dead_item_exceptions: Vec<String>,
    /// Module exceptions matching no extracted item.
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

/// Reconcile the extracted surface against the roster under the
/// committed exception lists and anchors.
pub(crate) fn reconcile(extracted: &BTreeSet<String>, rostered: &BTreeSet<&str>) -> Findings {
    reconcile_with(
        extracted,
        rostered,
        ITEM_EXCEPTIONS,
        MODULE_EXCEPTIONS,
        ANCHORS,
    )
}

/// [`reconcile`] over explicit exception and anchor tables, so the
/// judgment is a pure function the unit tests drive with synthetic
/// inputs.
pub(crate) fn reconcile_with(
    extracted: &BTreeSet<String>,
    rostered: &BTreeSet<&str>,
    item_exceptions: &[Exception],
    module_exceptions: &[Exception],
    anchors: &[&str],
) -> Findings {
    let excepted_items: BTreeSet<&str> = item_exceptions.iter().map(|e| e.name).collect();
    let module_covers = |name: &str| module_exceptions.iter().any(|e| name.starts_with(e.name));
    let dated = |e: &Exception| {
        e.decided.len() == 10 && e.decided.chars().filter(|&c| c == '-').count() == 2
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
        dead_item_exceptions: item_exceptions
            .iter()
            .map(|e| e.name)
            .filter(|name| !extracted.contains(*name) || rostered.contains(name))
            .map(str::to_owned)
            .collect(),
        dead_module_exceptions: module_exceptions
            .iter()
            .map(|e| e.name)
            .filter(|prefix| !extracted.iter().any(|name| name.starts_with(prefix)))
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
