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
//! [`FAMILY_SURFACE`], whose totality is by review of this file alone.
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

/// A public-API source file the extractor scans, with the naming context
/// the file cannot carry itself.
pub(crate) struct SourceSpec {
    /// Path relative to the crate root.
    pub(crate) path: &'static str,
    /// Namespace for module-level `pub fn`s (`None`: the file must have
    /// none).
    pub(crate) module_prefix: Option<&'static str>,
    /// Override for the inherent-impl type name — for files whose local
    /// type name is not its public path (the two `Forks` iterators) or
    /// whose type lives under a public module.
    pub(crate) type_override: Option<&'static str>,
}

/// The public-API source files of record. A new public module with
/// inherent methods must be added here (and the roster test's coverage
/// note updated), which is itself a reviewed diff.
pub(crate) const SURFACE_SOURCES: &[SourceSpec] = &[
    SourceSpec {
        path: "src/party.rs",
        module_prefix: None,
        type_override: None,
    },
    SourceSpec {
        path: "src/version.rs",
        module_prefix: None,
        type_override: None,
    },
    SourceSpec {
        path: "src/clock.rs",
        module_prefix: None,
        type_override: None,
    },
    SourceSpec {
        path: "src/version/own.rs",
        module_prefix: None,
        type_override: None,
    },
    SourceSpec {
        path: "src/version/rank.rs",
        module_prefix: None,
        type_override: None,
    },
    SourceSpec {
        path: "src/version/ranked.rs",
        module_prefix: None,
        type_override: None,
    },
    SourceSpec {
        path: "src/version/ticks.rs",
        module_prefix: None,
        type_override: None,
    },
    SourceSpec {
        path: "src/party/forks.rs",
        module_prefix: None,
        type_override: Some("iter::Party"),
    },
    SourceSpec {
        path: "src/clock/forks.rs",
        module_prefix: None,
        type_override: Some("iter::Clock"),
    },
    SourceSpec {
        path: "src/causally.rs",
        module_prefix: Some("causally"),
        type_override: Some("causally::Range"),
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
/// A line scan, not a parser, resting on rustfmt-normalized shape: impl
/// headers at column 0 (trait impls contain ` for ` and cannot hold
/// `pub fn`s), inherent methods at one indent level. `pub fn` at an
/// unexpected position panics rather than silently vanishing from the
/// listing — the scan must never under-report the surface it exists to
/// pin.
pub(crate) fn extract_public_fns() -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for spec in SURFACE_SOURCES {
        let path = crate_root().join(spec.path);
        let text =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
        // The public name of the current inherent impl block, if inside one.
        let mut current_type: Option<String> = None;
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("impl") {
                if line.contains(" for ") {
                    current_type = None; // trait impl: cannot hold `pub fn`
                } else {
                    current_type = parse_impl_self_type(rest)
                        .map(|name| spec.type_override.map(str::to_owned).unwrap_or(name));
                }
                continue;
            }
            if line == "}" {
                current_type = None;
                continue;
            }
            if let Some(rest) = line.strip_prefix("    pub fn ") {
                let name = fn_name(rest);
                let ty = current_type.as_deref().unwrap_or_else(|| {
                    panic!(
                        "{}: `pub fn {name}` outside an inherent impl block",
                        spec.path
                    )
                });
                out.insert(format!("{ty}::{name}"));
                continue;
            }
            if let Some(rest) = line.strip_prefix("pub fn ") {
                let name = fn_name(rest);
                let prefix = spec.module_prefix.unwrap_or_else(|| {
                    panic!("{}: unexpected module-level `pub fn {name}`", spec.path)
                });
                out.insert(format!("{prefix}::{name}"));
            }
        }
    }
    out
}

/// The self-type name from an impl header's remainder (after `impl`):
/// skip a balanced generics list, then read the first identifier.
/// Shared with the complexity-claims scanner, which walks the same files.
pub(crate) fn parse_impl_self_type(rest: &str) -> Option<String> {
    let mut chars = rest.chars().peekable();
    if chars.peek() == Some(&'<') {
        let mut depth = 0usize;
        for c in chars.by_ref() {
            match c {
                '<' => depth += 1,
                '>' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
        }
    }
    let name: String = chars
        .skip_while(|c| c.is_whitespace())
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    (!name.is_empty()).then_some(name)
}

/// The function name from the remainder after `pub fn `.
/// Shared with the complexity-claims scanner, which walks the same files.
pub(crate) fn fn_name(rest: &str) -> &str {
    rest.split(|c: char| !c.is_alphanumeric() && c != '_')
        .next()
        .unwrap_or("")
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
