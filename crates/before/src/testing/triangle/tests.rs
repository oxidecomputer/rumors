//! The triangle roster's enforcement: totality against the extracted
//! public surface, liveness of every cited binding test, and the
//! committed-seed tripwire.

use std::collections::BTreeSet;
use std::fs;

use super::{
    cited_test_names, crate_root, declared_test_names, extract_public_fns, FAMILY_SURFACE,
    METHOD_SURFACE, TRIPWIRES,
};

/// The roster is total over the public inherent-`pub fn` surface, both
/// directions.
///
/// Every extracted operation has exactly one named row, and every method
/// row names an operation that still exists — so a new public op forces
/// a reviewed row, and a removed op orphans one.
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

/// Every test name the roster or a tripwire cites resolves to an
/// executable binding.
///
/// The bindings: a `#[test]`-attributed item under `src/`, or a law
/// name registered in [`crate::laws`]'s tables (the entries the
/// algebraic-laws drivers run). A renamed or deleted binding test fails
/// here by name, so a disposition can never silently point at nothing,
/// and a same-named helper or kernel can never stand in for the test a
/// row claims.
#[test]
fn every_cited_binding_test_exists() {
    let mut declared = declared_test_names();
    declared.extend(
        crate::laws::registered_names()
            .into_iter()
            .map(str::to_owned),
    );
    let dead: Vec<&str> = cited_test_names()
        .into_iter()
        .filter(|name| !declared.contains(*name))
        .collect();
    assert!(
        dead.is_empty(),
        "roster/tripwire citations resolve to no `#[test]` item or \
         registered law: {dead:?}"
    );
}

/// The citation haystack admits only executable tests (the seal for the
/// #37 review's F5 tamper hole): a helper `fn` must never satisfy a
/// binding-test citation.
///
/// Two directions. Negative: named non-test helpers — declared `fn`s the
/// old bare-name scan accepted — are absent from
/// [`declared_test_names`], so a roster row whose cited differential test
/// is deleted goes red even while a same-named helper survives. Positive
/// (the scan's own liveness): a known `#[test]` item and a known
/// proptest-block property both resolve, so the seal cannot green by
/// scanning nothing.
#[test]
fn citation_haystack_admits_only_attributed_tests() {
    let declared = declared_test_names();
    for helper in ["declared_test_names", "parse_impl_self_type", "fn_name"] {
        assert!(
            !declared.contains(helper),
            "{helper} is a helper fn, not a test, and must not be able to \
             satisfy a binding-test citation"
        );
    }
    for test in [
        // This suite's own plain `#[test]`.
        "every_cited_binding_test_exists",
        // A `proptest!`-block property the roster cites.
        "join_all_matches_the_recursive_oracle",
    ] {
        assert!(
            declared.contains(test),
            "{test} is an attributed test and must resolve in the haystack"
        );
    }
    // The law leg: registered names come from the tables the drivers run,
    // and the tables' local helper fns are not registered.
    let laws = crate::laws::registered_names();
    assert!(
        laws.contains(&"forks_matches_from_array"),
        "a roster-cited law must be registered in its table"
    );
    assert!(
        !laws.contains(&"le") && !laws.contains(&"hash_of"),
        "laws.rs helper fns must not be able to satisfy a citation"
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
