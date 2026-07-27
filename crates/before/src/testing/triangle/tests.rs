//! The triangle roster's enforcement: totality against the extracted
//! public surface, liveness of every cited binding test, and the
//! committed-seed tripwire.

use std::collections::BTreeSet;
use std::fs;

use super::{
    cited_test_names, crate_root, declared_fn_names, extract_public_fns, FAMILY_SURFACE,
    METHOD_SURFACE, TRIPWIRES,
};

/// The roster is total over the public inherent-`pub fn` surface, both
/// directions: every extracted operation has exactly one named row, and
/// every method row names an operation that still exists — so a new
/// public op forces a reviewed row, and a removed op orphans one.
#[test]
fn roster_is_total_over_the_public_fn_surface() {
    let extracted = extract_public_fns();
    let rostered: BTreeSet<String> = METHOD_SURFACE.iter().map(|row| row.op.to_owned()).collect();
    assert_eq!(
        rostered.len(),
        METHOD_SURFACE.len(),
        "duplicate roster rows: every op gets exactly one row"
    );
    let missing: Vec<&String> = extracted.difference(&rostered).collect();
    let orphaned: Vec<&String> = rostered.difference(&extracted).collect();
    assert!(
        missing.is_empty() && orphaned.is_empty(),
        "the triangle roster and the public surface disagree.\n\
         public ops without a roster row (add one, naming each leg's \
         disposition): {missing:?}\n\
         roster rows without a public op (remove or rename): {orphaned:?}"
    );
}

/// Every test name the roster or a tripwire cites is declared somewhere
/// under `src/` — a renamed or deleted binding test fails here by name,
/// so a disposition can never silently point at nothing.
#[test]
fn every_cited_binding_test_exists() {
    let declared = declared_fn_names();
    let dead: Vec<&str> = cited_test_names()
        .into_iter()
        .filter(|name| !declared.contains(*name))
        .collect();
    assert!(
        dead.is_empty(),
        "roster/tripwire citations name no declared fn: {dead:?}"
    );
}

/// The family roster's rows are unique by op description (totality over
/// the operator/trait surface is by review of the file; this pins the
/// table's internal hygiene).
#[test]
fn family_rows_are_unique() {
    let ops: BTreeSet<&str> = FAMILY_SURFACE.iter().map(|row| row.op).collect();
    assert_eq!(ops.len(), FAMILY_SURFACE.len(), "duplicate family rows");
}

/// Every excluded leg states a non-trivial reason: an exclusion is a
/// documented boundary decision, never a bare opt-out.
#[test]
fn every_exclusion_states_a_reason() {
    for row in METHOD_SURFACE.iter().chain(FAMILY_SURFACE) {
        for leg in [&row.prod_tree, &row.prod_fs, &row.tree_fs] {
            if let Some(reason) = leg.exclusion_reason() {
                assert!(
                    reason.len() >= 20,
                    "{}: exclusion reason too thin: {reason:?}",
                    row.op
                );
            }
        }
    }
}

/// Every tripwire leg label is nonempty and unique — the tripwire list
/// stays a legible per-leg inventory, not a grab bag.
#[test]
fn tripwires_are_labeled() {
    let labels: BTreeSet<&str> = TRIPWIRES.iter().map(|(label, _)| *label).collect();
    assert_eq!(labels.len(), TRIPWIRES.len(), "duplicate tripwire labels");
    assert!(
        TRIPWIRES.iter().all(|(label, _)| !label.is_empty()),
        "empty tripwire label"
    );
}

/// The prod↔tree adequacy seeds stay committed: the two fold-mutation
/// witnesses replay through the `join_all` differentials from these
/// files on every run, and this pin makes stripping them a red diff.
#[test]
fn d1_seeds_stay_committed() {
    for (file, seed) in [
        ("proptest-regressions/party/tests.txt", "cc e1aea6c3"),
        ("proptest-regressions/clock/tests.txt", "cc efc8c717"),
    ] {
        let path = crate_root().join(file);
        let text =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
        assert!(
            text.contains(seed),
            "{file} no longer carries the committed fold seed {seed}"
        );
    }
}
