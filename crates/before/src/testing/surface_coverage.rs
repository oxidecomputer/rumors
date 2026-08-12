//! The public-surface coverage suite: one committed roster binding every
//! public operation to the disposition of each differential leg.
//!
//! Three implementations cover the semantic surface: the production packed
//! implementation (*prod*), the recursive paper-transcription oracle in
//! [`crate::oracle`] (*tree* — the semantic definition of record), and the
//! function-space semantic oracle in [`super::semantic_oracle`] (*fs*). Three
//! legs connect them — prod↔tree, prod↔fs, tree↔fs — and the *roster*
//! ([`crate::surface`], re-exported below) holds one row per public
//! operation naming, for each leg, either
//! the primary test that binds it or the reason it is excluded. The roster
//! indexes the differentials that live beside the code they test; it never
//! re-implements one.
//!
//! # Tamper-evident totality
//!
//! [`METHOD_SURFACE`] must match, name for name, the inherent `pub fn`
//! surface extracted from the public-API source files
//! ([`extract_public_fns`], over [`SURFACE_SOURCES`]) — both directions, so
//! a *new* public operation fails the roster test until a named row is
//! added, and a removed operation orphans its row until the row is removed;
//! either way the reviewer sees a named diff. Operator and trait surfaces
//! (`|`, `&`, `^`, `/`, comparison matrices, `Display`/`FromStr`, serde/borsh)
//! are not reachable by that scan; they are rostered by family in
//! [`FAMILY_SURFACE`] for their leg dispositions, and the surface-totality
//! gate (`crates/before/surfacecheck`, over nightly rustdoc JSON) holds
//! the concrete impl inventory behind those families mechanically total:
//! every reachable trait impl is pinned there by name, so a new operator
//! or trait impl fails the gate until its pin — and, for a new family,
//! the family row here — is added.
//! Every test name a row cites must resolve to an executable binding:
//! a `#[test]`-attributed item under `src/` ([`cited_test_names`] against
//! [`declared_test_names`], a source scan that admits only attributed
//! tests — proptest properties included, helpers and kernels never), a
//! law name registered in [`crate::laws`]'s tables, or a descriptor name
//! registered in [`super::diff_ops`]'s tables (both read from the tables
//! the drivers run, never from a text scan). A renamed or deleted binding
//! test fails the roster by name even when a same-named helper survives.
//! Which `Bound` citations are descriptors and which are hand-written is
//! itself pinned, in the descriptor table's own tiling test.
//!
//! # Leg vocabulary
//!
//! - [`Leg::Bound`]: a direct differential on that leg; the named test
//!   or descriptor drives both sides. One test may bind several legs when
//!   its body performs each comparison (the distance/lag triple asserts
//!   prod, tree, and fs results equal in one proptest); the citation is
//!   per-leg, the comparisons per-body.
//! - [`Leg::Law`]: pinned by an algebraic law on production alone (no
//!   reference on the right-hand side); used where no reference counterpart
//!   exists or the contract promises only a law.
//! - [`Leg::Trans`]: bound transitively — the operation reduces by
//!   definition to a bound one, or the leg is the composition of the other
//!   two bound legs; the named test anchors the reduction.
//! - [`Leg::Excluded`]: not bound, with the reason. The function-space
//!   boundary's exclusion dispositions are the owner's, marked
//!   "ratified by owner" at each reason.
//!
//! # Exclusion families
//!
//! An excluded leg carries a variant of the typed [`crate::surface::Exclusion`]
//! vocabulary — seven families, each variant's documentation defending its
//! argument once. The suite enforces the families' obligations: every payload
//! name resolves exactly as citations do, every family is inhabited (an empty
//! family is a dead category), a `GridCap` guard is a live executable test,
//! and a `NotAPaperObject` binding site is a real roster row, test, or law.
//! The function-space non-adoption dispositions are the owner's, ratified
//! where the variants say so.
//!
//! # Adequacy tripwires
//!
//! Each leg keeps committed artifacts proving its criterion can fail
//! ([`TRIPWIRES`], names checked live), in two genres. *Liveness anchors*
//! prove the machinery runs: prod↔tree keeps the fold seeds replaying
//! through the `join_all` differentials (the seeds are pinned committed
//! by `d1_seeds_stay_committed`) and the brute-force grow reference as
//! the independent fourth leg; prod↔fs keeps the grid-cap premise guard;
//! tree↔fs keeps the paper worked-value anchors. *Known-bad references,
//! held convicted*, prove the comparisons can reject: each leg commits a
//! deliberately-wrong reference variant behind an inverted assertion —
//! the leg's differential comparison must convict it over a committed
//! input family — so a criterion that has gone blind reads red instead
//! of green (prod↔tree convicts the dropped-group fold, prod↔fs the
//! cell-dropping Riemann sum, tree↔fs the mirrored embedding, whose
//! conviction test also documents that the leg's pointwise differentials
//! alone are blind to a twin-substituted mirror). The prod↔tree leg
//! carries a second known-bad genre for the descriptors it derives: a
//! mis-transcribed descriptor, convicted where its two spellings differ
//! and passed where they coincide, so the vehicle is shown to
//! discriminate rather than merely to fail.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

#[cfg(test)]
mod tests;

// The roster itself — `Leg`, `SurfaceRow`, `METHOD_SURFACE`,
// `FAMILY_SURFACE` — lives in `crate::surface`, exported under the
// `meter` feature so external instrument crates bind their coverage to
// the same rows this suite enforces totality over. Re-exported here so
// the suite and its sibling scanners keep one naming context (rows are
// reached through the surfaces, so `SurfaceRow` itself is not re-imported).
pub(crate) use crate::surface::{Leg, FAMILY_SURFACE, METHOD_SURFACE};

/// Per-leg adequacy tripwires: committed artifacts proving each leg's
/// criterion can fail.
///
/// Both of the module doc's genres are rostered here — the liveness
/// anchors and the known-bad references held convicted. Names are
/// checked live by the roster tests; the prod↔tree seeds are
/// additionally pinned committed by `d1_seeds_stay_committed`.
pub(crate) const TRIPWIRES: &[(&str, &str)] = &[
    (
        "prod↔tree: the fold seeds replay through the join_all differentials",
        "join_all_matches_the_recursive_oracle",
    ),
    (
        "prod↔tree: the independent full-enumeration fourth leg for grow",
        "grow_matches_brute_force",
    ),
    (
        "prod↔tree: the known-bad dropped-group fold, held convicted",
        "join_all_differential_convicts_the_dropped_group_oracle",
    ),
    (
        "prod↔tree: the known-bad mis-transcribed descriptors, held convicted",
        "the_drivers_convict_a_mis_transcribed_descriptor",
    ),
    (
        "prod↔fs: the grid-resolution premise guard",
        "grid_cap_is_never_reached",
    ),
    (
        "prod↔fs: the known-bad cell-dropping Riemann sum, held convicted",
        "rank_differential_convicts_the_cell_dropping_riemann_sum",
    ),
    (
        "tree↔fs: the paper worked-value anchor",
        "embedding_matches_paper_worked_value",
    ),
    (
        "tree↔fs: the leaf-interval constancy anchor",
        "lifted_event_is_constant_within_a_leaf_interval",
    ),
    (
        "tree↔fs: the known-bad mirrored embedding, held convicted",
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

/// Extract the inherent `pub fn` surface from [`SURFACE_SOURCES`], named
/// as the roster names it (`Type::fn` inside an inherent impl block,
/// `module::fn` at file top level).
///
/// The shared extractor's line discipline (see
/// [`surface_scan::extract_public_fns`]'s docs): a line scan resting
/// on rustfmt-normalized shape, panicking on any `pub fn` it cannot name
/// rather than silently under-reporting the surface it exists to pin.
pub(crate) fn extract_public_fns() -> BTreeSet<String> {
    ::surface_scan::extract_public_fns(&crate_root(), SURFACE_SOURCES)
}

/// Every test name the roster and tripwires cite.
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

/// Every `#[test]`-attributed `fn` name declared anywhere under `src/` —
/// the haystack the cited-name check searches.
///
/// The scan resolves a name only when a `#[test]` attribute (including
/// the ones inside `proptest!` blocks, which attach `#[test]` to each
/// property) sits directly above the `fn`, with only further attributes,
/// doc comments, and plain comments between. Helper functions, production
/// kernels, and test-support plumbing never enter the haystack, so a
/// citation is satisfiable only by an item the test runner actually
/// executes.
pub(crate) fn declared_test_names() -> BTreeSet<String> {
    declared_test_names_by_file().into_keys().collect()
}

/// Every `#[test]`-attributed `fn` name declared under `src/`, with the
/// crate-relative paths of the files declaring it.
///
/// The same scan as [`declared_test_names`], keeping the declaring files:
/// a bare-name set collapses same-named tests across files into one entry,
/// which is exactly the ambiguity the duplicate-name roster pin holds
/// tamper-evident.
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
