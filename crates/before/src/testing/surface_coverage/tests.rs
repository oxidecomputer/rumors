//! Checks for the public API coverage roster and its cited tests.

use std::collections::BTreeSet;
use std::fs;

use super::{
    cited_test_names, crate_root, declared_test_names, declared_test_names_by_file,
    extract_public_fns, FAMILY_SURFACE, METHOD_SURFACE, TRIPWIRES,
};

/// Test names intentionally used in more than one file.
///
/// Coverage citations contain only a test name. Recording duplicates here
/// ensures that removing one same-named test cannot go unnoticed.
const DUPLICATE_TEST_NAMES: &[(&str, &[&str])] = &[
    (
        "as_bytes_matches_encode",
        &["src/party/tests.rs", "src/version/tests.rs"],
    ),
    (
        "byte_equality_matches_bit_equality",
        &["src/party/tests.rs", "src/version/tests.rs"],
    ),
    (
        "decode_encode_arbitrary",
        &["src/party/tests.rs", "src/version/tests.rs"],
    ),
    (
        "exhaustive_small_scope_agrees",
        &[
            "src/version/skyline/query/tests.rs",
            "src/version/skyline/sweep/tests.rs",
        ],
    ),
    (
        "heterogeneous_joins",
        &["src/clock/tests.rs", "src/oracle/tests.rs"],
    ),
    (
        "join_all_matches_the_recursive_oracle",
        &["src/clock/tests.rs", "src/party/tests.rs"],
    ),
    (
        "organic_histories_agree",
        &[
            "src/version/skyline/query/tests.rs",
            "src/version/skyline/sweep/tests.rs",
        ],
    ),
    ("sync", &["src/clock/tests.rs", "src/oracle/tests.rs"]),
    (
        "worked_example",
        &["src/clock/tests.rs", "src/oracle/tests.rs"],
    ),
];

/// Every public inherent method has exactly one coverage row.
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
        "the coverage roster and the public surface disagree.\n\
         public ops without a roster row (add one, naming each leg's \
         disposition): {missing:?}\n\
         roster rows without a public op (remove or rename): {orphaned:?}"
    );
}

/// Every coverage citation names an executable test, registered law, or
/// registered operation descriptor.
#[test]
fn every_cited_binding_test_exists() {
    let mut declared = declared_test_names();
    declared.extend(
        crate::laws::registered_names()
            .into_iter()
            .map(str::to_owned),
    );
    declared.extend(
        crate::testing::diff_ops::registered_names()
            .into_iter()
            .map(str::to_owned),
    );
    let dead: Vec<&str> = cited_test_names()
        .into_iter()
        .filter(|name| !declared.contains(*name))
        .collect();
    assert!(
        dead.is_empty(),
        "coverage citations resolve to no `#[test]` item or \
         registered law: {dead:?}"
    );
}

/// Test citations cannot be satisfied by an ordinary helper function.
///
/// The source scan excludes known helpers and includes both a conventional test
/// and a property test. Registered-law names likewise exclude law helpers.
#[test]
fn citations_resolve_only_to_executable_tests() {
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
            "{test} is an attributed test and must appear in the source scan"
        );
    }
    // Registered law names also exclude ordinary helper functions.
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

/// Every exclusion cites existing evidence of the behavior it omits.
///
/// Test and law names must resolve, capacity guards must be executable tests,
/// and a model quantity delegated elsewhere must name an existing coverage row.
#[test]
fn exclusion_payload_citations_resolve() {
    use crate::surface::Exclusion;
    let mut resolvable = declared_test_names();
    resolvable.extend(
        crate::laws::registered_names()
            .into_iter()
            .map(str::to_owned),
    );
    resolvable.extend(
        crate::testing::diff_ops::registered_names()
            .into_iter()
            .map(str::to_owned),
    );
    let tests_only = declared_test_names();
    let rows: BTreeSet<&str> = METHOD_SURFACE
        .iter()
        .chain(FAMILY_SURFACE)
        .map(|row| row.op)
        .collect();
    let mut dead: Vec<String> = Vec::new();
    for row in METHOD_SURFACE.iter().chain(FAMILY_SURFACE) {
        for leg in [&row.prod_tree, &row.prod_fs, &row.tree_fs] {
            let Some(family) = leg.exclusion() else {
                continue;
            };
            let mut names: Vec<&str> = Vec::new();
            match family {
                Exclusion::NoWireFormatInReferences { pins }
                | Exclusion::DefinitionalCombinator { pins }
                | Exclusion::NAryNotInReferences { pins }
                | Exclusion::LinearityMechanics { pins } => names.extend(*pins),
                Exclusion::NotAPaperObject { bound_at, pins } => {
                    names.extend(*pins);
                    if !rows.contains(bound_at) {
                        names.push(bound_at);
                    }
                }
                Exclusion::GridCap { guard } => {
                    if !tests_only.contains(*guard) {
                        dead.push(format!(
                            "{}: GridCap guard {guard} is no executable #[test]",
                            row.op
                        ));
                    }
                }
                Exclusion::RepresentationMechanics { license } => names.push(license),
            }
            for name in names {
                if !resolvable.contains(name) {
                    dead.push(format!("{}: {name}", row.op));
                }
            }
        }
    }
    assert!(
        dead.is_empty(),
        "exclusion payloads cite names that resolve to no `#[test]` item, \
         registered law, or (for bound_at) roster row: {dead:?}"
    );
}

/// Every exclusion variant is used by at least one coverage row.
#[test]
fn every_exclusion_family_is_used() {
    use std::collections::BTreeMap;
    let mut census: BTreeMap<&str, usize> = BTreeMap::new();
    for row in METHOD_SURFACE.iter().chain(FAMILY_SURFACE) {
        for leg in [&row.prod_tree, &row.prod_fs, &row.tree_fs] {
            if let Some(family) = leg.exclusion() {
                *census.entry(family.family()).or_default() += 1;
            }
        }
    }
    for family in crate::surface::Exclusion::FAMILIES {
        assert!(
            census.get(family).copied().unwrap_or(0) > 0,
            "exclusion family {family} is unused"
        );
    }
}

/// Every test name used in multiple files records all of those files.
///
/// This keeps bare-name citations from silently resolving to the wrong copy
/// after one same-named test is removed.
#[test]
fn duplicate_test_names_are_recorded() {
    let scanned: Vec<(String, Vec<String>)> = declared_test_names_by_file()
        .into_iter()
        .filter(|(_, files)| files.len() > 1)
        .map(|(name, files)| (name, files.into_iter().collect()))
        .collect();
    let rostered: Vec<(String, Vec<String>)> = DUPLICATE_TEST_NAMES
        .iter()
        .map(|(name, files)| {
            (
                (*name).to_owned(),
                files.iter().map(|f| (*f).to_owned()).collect(),
            )
        })
        .collect();
    assert_eq!(
        scanned, rostered,
        "the same-named tests in the tree and the DUPLICATE_TEST_NAMES \
         roster must agree exactly: an unrostered duplicate makes bare-name \
         citations ambiguous, and a stale entry names a deleted copy"
    );
}

/// A citation name identifies exactly one kind of evidence.
///
/// Test, law, and operation-descriptor names must not overlap, or removing one
/// definition could leave the citation resolving to another.
#[test]
fn binding_kinds_never_shadow_each_other() {
    let tests = declared_test_names();
    let laws: BTreeSet<&str> = crate::laws::registered_names().into_iter().collect();
    let descriptors: BTreeSet<&str> = crate::testing::diff_ops::registered_names()
        .into_iter()
        .collect();
    let shadowed: Vec<&str> = laws
        .iter()
        .chain(&descriptors)
        .copied()
        .filter(|name| tests.contains(*name))
        .chain(laws.intersection(&descriptors).copied())
        .collect();
    assert!(
        shadowed.is_empty(),
        "names bound twice (as a #[test], a law, or a descriptor) make \
         citations ambiguous: {shadowed:?}"
    );
}

/// Trait-family coverage rows have unique descriptions.
#[test]
fn family_rows_are_unique() {
    let ops: BTreeSet<&str> = FAMILY_SURFACE.iter().map(|row| row.op).collect();
    assert_eq!(ops.len(), FAMILY_SURFACE.len(), "duplicate family rows");
}

/// Every verification-test label is nonempty and unique.
#[test]
fn verification_tests_are_labeled() {
    let labels: BTreeSet<&str> = TRIPWIRES.iter().map(|(label, _)| *label).collect();
    assert_eq!(
        labels.len(),
        TRIPWIRES.len(),
        "duplicate verification-test labels"
    );
    assert!(
        TRIPWIRES.iter().all(|(label, _)| !label.is_empty()),
        "empty verification-test label"
    );
}

/// The committed `join_all` property-test cases remain in their active seed
/// files.
#[test]
fn committed_join_all_seeds_exist() {
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
