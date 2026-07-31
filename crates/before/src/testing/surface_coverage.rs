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
//! (`|`, `&`, `/`, comparison matrices, `Display`/`FromStr`, serde/borsh)
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
//! tests — proptest properties included, helpers and kernels never) or a
//! law name registered in [`crate::laws`]'s tables (read from the tables
//! the drivers run, never from a text scan). A renamed or deleted binding
//! test fails the roster by name even when a same-named helper survives.
//!
//! # Leg vocabulary
//!
//! - [`Leg::Bound`]: a direct differential on that leg; the named test
//!   drives both sides. One test may bind several legs when its body
//!   performs each comparison (the distance/lag triple asserts prod,
//!   tree, and fs results equal in one proptest); the citation is
//!   per-leg, the comparisons per-body.
//! - [`Leg::Law`]: pinned by an algebraic law on production alone (no
//!   reference on the right-hand side); used where no reference counterpart
//!   exists or the contract promises only a law.
//! - [`Leg::Trans`]: bound transitively — the operation reduces by
//!   definition to a bound one, or the leg is the composition of the other
//!   two bound legs; the named test anchors the reduction.
//! - [`Leg::Excluded`]: not bound, with the reason. The function-space
//!   boundary's exclusion dispositions are the owner's, marked
//!   "ratified by owner, 2026-07-26" at each reason.
//!
//! # Exclusion families
//!
//! Codecs and text (no wire format exists in the references; correctness is
//! production-side canonicality/round-trip/strict-rejection pins), linearity
//! and aliasing mechanics (`Clone` references cannot express them;
//! compile-fail tests own them), `causally` (a definitional combinator over
//! the bound causal
//! order), rank arithmetic (not a paper object; bound to the in-test
//! alignment oracle), n-ary hand-back mechanics (value identity and order
//! are not functions of the geometry), depth beyond the function-space grid
//! (`GRID_N` caps resolution; `deep_tree_stack_safety` is impl-only by
//! documented necessity), and the meter/error/iter plumbing.
//!
//! # Adequacy tripwires
//!
//! Each leg keeps a committed artifact proving its criterion can fail
//! ([`TRIPWIRES`], names checked live): prod↔tree keeps the fold seeds
//! replaying through the `join_all` differentials (the seeds are pinned
//! committed by `d1_seeds_stay_committed`) and the brute-force grow
//! reference as the independent fourth leg; prod↔fs keeps the grid-cap
//! premise guard; tree↔fs keeps the paper worked-value anchors. Named
//! obligation, not yet wired: a permanently-red known-bad artifact per leg
//! (the wrong-child-descent mutation is demonstrated in history, not
//! committed as a rostered red).

use std::collections::BTreeSet;
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
/// criterion can fail. Names are checked live by the roster tests; the
/// prod↔tree seeds are additionally pinned committed by
/// `d1_seeds_stay_committed`.
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
        "prod↔fs: the grid-resolution premise guard",
        "grid_cap_is_never_reached",
    ),
    (
        "tree↔fs: the paper worked-value anchor",
        "embedding_matches_paper_worked_value",
    ),
    (
        "tree↔fs: the leaf-interval constancy anchor",
        "lifted_event_is_constant_within_a_leaf_interval",
    ),
];

// The extractor, the doc-section scanner, and their line discipline are
// the workspace-shared claims machinery (the `complexity-claims` crate);
// this module supplies before's source list and naming context and keeps
// the callers' entry points.
pub(crate) use ::complexity_claims::{fn_name, SourceSpec};

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
        type_overrides: &[("Range", "causally::Range"), ("Span", "causally::Span")],
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
/// [`complexity_claims::extract_public_fns`]'s docs): a line scan resting
/// on rustfmt-normalized shape, panicking on any `pub fn` it cannot name
/// rather than silently under-reporting the surface it exists to pin.
pub(crate) fn extract_public_fns() -> BTreeSet<String> {
    ::complexity_claims::extract_public_fns(&crate_root(), SURFACE_SOURCES)
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
    let mut names = BTreeSet::new();
    let mut stack = vec![crate_root().join("src")];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
            let path = entry.expect("directory entry").path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let text = fs::read_to_string(&path)
                    .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
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
                                    names.insert(name.to_owned());
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
