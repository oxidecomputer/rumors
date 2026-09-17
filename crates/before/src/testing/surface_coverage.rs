//! Verifies coverage of the public API by independent implementations and laws.
//!
//! Production behavior is compared with the recursive oracle and the
//! function-space oracle. Each [`crate::surface`] row records a direct
//! comparison, an algebraic law, a comparison supplied transitively by other
//! rows, or a documented reason that a reference does not apply.
//!
//! Tests keep the method roster synchronized with the public API, require
//! trait families to be recorded, and require every cited test, law, or
//! operation descriptor to exist. Exclusion payloads are checked in the same
//! way. The function-space exclusions marked "ratified by owner" are deliberate
//! boundaries of that reference.
//!
//! # Verifying the checks
//!
//! The verification list below names tests that show each comparison is active.
//! Some exercise an independent reference or a required premise. Others
//! substitute a known incorrect implementation and verify that the comparison
//! rejects it. The roster tests require every named test to exist.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

#[cfg(test)]
mod tests;

// Use the same roster exposed to external verification tools.
pub(crate) use crate::surface::{Leg, FAMILY_SURFACE, METHOD_SURFACE};

/// Tests that demonstrate each comparison is active and can detect an error.
pub(crate) const TRIPWIRES: &[(&str, &str)] = &[
    (
        "prod↔tree: committed join_all cases exercise the comparison",
        "join_all_matches_the_recursive_oracle",
    ),
    (
        "prod↔tree: grow is checked by complete enumeration",
        "grow_matches_brute_force",
    ),
    (
        "prod↔tree: incorrect operation descriptors are rejected",
        "the_drivers_convict_a_mis_transcribed_descriptor",
    ),
    (
        "prod↔fs: the function-space grid is sufficiently fine",
        "grid_cap_is_never_reached",
    ),
    (
        "tree↔fs: an incorrect function-space operation is rejected",
        "the_drivers_convict_a_mis_transcribed_fs_realization",
    ),
    (
        "prod↔fs: a sum that omits a cell is rejected",
        "rank_differential_convicts_the_cell_dropping_riemann_sum",
    ),
    (
        "tree↔fs: the published example is reproduced",
        "embedding_matches_known_values",
    ),
    (
        "tree↔fs: events are constant within each leaf interval",
        "lifted_event_is_constant_within_a_leaf_interval",
    ),
    (
        "tree↔fs: a mirrored embedding is rejected",
        "worked_value_anchor_convicts_the_mirrored_embedding",
    ),
];

// The extractor and its line discipline are the workspace-shared source
// scanners (the `surface-scan` crate); this module supplies before's
// source list and naming context and keeps the callers' entry points.
pub(crate) use ::surface_scan::{fn_name, SourceSpec};

/// The public-API source files of record. A new public module with
/// inherent methods must be added here (and the roster test's coverage
/// note updated), which is itself a reviewed diff.
pub(crate) const SURFACE_SOURCES: &[SourceSpec] = &[
    SourceSpec {
        path: "src/party.rs",
        module_prefix: None,
        type_overrides: &[],
    },
    SourceSpec {
        path: "src/version.rs",
        module_prefix: None,
        type_overrides: &[],
    },
    SourceSpec {
        path: "src/clock.rs",
        module_prefix: None,
        type_overrides: &[],
    },
    SourceSpec {
        path: "src/version/own.rs",
        module_prefix: None,
        type_overrides: &[],
    },
    SourceSpec {
        path: "src/version/rank.rs",
        module_prefix: None,
        type_overrides: &[],
    },
    SourceSpec {
        path: "src/version/ranked.rs",
        module_prefix: None,
        type_overrides: &[],
    },
    SourceSpec {
        path: "src/version/ticks.rs",
        module_prefix: None,
        type_overrides: &[],
    },
    SourceSpec {
        path: "src/shape.rs",
        module_prefix: Some("shape"),
        type_overrides: &[],
    },
    SourceSpec {
        path: "src/party/forks.rs",
        module_prefix: None,
        type_overrides: &[("Forks", "iter::Party")],
    },
    SourceSpec {
        path: "src/clock/forks.rs",
        module_prefix: None,
        type_overrides: &[("Forks", "iter::Clock")],
    },
    SourceSpec {
        path: "src/causally.rs",
        module_prefix: Some("causally"),
        type_overrides: &[],
    },
    SourceSpec {
        path: "src/causally/forms.rs",
        module_prefix: Some("causally"),
        type_overrides: &[
            ("Floor", "causally::Floor"),
            ("Ceiling", "causally::Ceiling"),
        ],
    },
    SourceSpec {
        path: "src/causally/query.rs",
        module_prefix: Some("causally"),
        type_overrides: &[("Query", "causally::Query")],
    },
    SourceSpec {
        path: "src/span/own.rs",
        module_prefix: None,
        type_overrides: &[],
    },
    SourceSpec {
        path: "src/span/algebra.rs",
        module_prefix: None,
        type_overrides: &[],
    },
    SourceSpec {
        path: "src/span/wire.rs",
        module_prefix: None,
        type_overrides: &[],
    },
    SourceSpec {
        path: "src/span.rs",
        module_prefix: None,
        type_overrides: &[],
    },
];

/// The crate root at test time.
fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Return the public methods found in [`SURFACE_SOURCES`].
///
/// Names use `Type::fn` for inherent methods and `module::fn` for top-level
/// functions. The extractor fails if it encounters a public function it cannot
/// identify.
pub(crate) fn extract_public_fns() -> BTreeSet<String> {
    ::surface_scan::extract_public_fns(&crate_root(), SURFACE_SOURCES)
}

/// Every test cited by the coverage roster or verification list.
pub(crate) fn cited_test_names() -> BTreeSet<&'static str> {
    METHOD_SURFACE
        .iter()
        .chain(FAMILY_SURFACE)
        .flat_map(|row| {
            [&row.prod_tree, &row.prod_fs, &row.tree_fs]
                .into_iter()
                .filter_map(Leg::cited)
        })
        .chain(TRIPWIRES.iter().map(|(_, test)| *test))
        .collect()
}

/// Return the names of executable tests declared under `src`.
///
/// This includes properties generated by `proptest!` and excludes ordinary
/// helper functions.
pub(crate) fn declared_test_names() -> BTreeSet<String> {
    declared_test_names_by_file().into_keys().collect()
}

/// Return each executable test name and the files that declare it.
pub(crate) fn declared_test_names_by_file() -> BTreeMap<String, BTreeSet<String>> {
    let root = crate_root();
    let mut names: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut stack = vec![root.join("src")];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
            let path = entry.expect("directory entry").path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let text = fs::read_to_string(&path)
                    .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
                let relative = path
                    .strip_prefix(&root)
                    .expect("scanned files live under the crate root")
                    .to_string_lossy()
                    .into_owned();
                // Whether a `#[test]` attribute is pending for the next
                // `fn` declaration.
                let mut test_pending = false;
                for line in text.lines() {
                    let trimmed = line.trim_start();
                    if trimmed.starts_with("#[test]") {
                        test_pending = true;
                        continue;
                    }
                    // Other attributes, doc comments, and comments sit
                    // between `#[test]` and its `fn` without detaching it.
                    if trimmed.starts_with("#[")
                        || trimmed.starts_with("///")
                        || trimmed.starts_with("//")
                        || trimmed.is_empty()
                    {
                        continue;
                    }
                    if test_pending {
                        if let Some(pos) = trimmed.find("fn ") {
                            let boundary = pos == 0 || trimmed[..pos].ends_with(' ');
                            if boundary {
                                let name = fn_name(&trimmed[pos + 3..]);
                                if !name.is_empty() {
                                    names
                                        .entry(name.to_owned())
                                        .or_default()
                                        .insert(relative.clone());
                                }
                            }
                        }
                        test_pending = false;
                    }
                }
            }
        }
    }
    names
}
