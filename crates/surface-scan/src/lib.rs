//! The workspace's shared source scanners: the public-surface extractor
//! behind rosters' totality tests, and the witness scanner behind their
//! evidence bindings.
//!
//! A consuming crate keeps committed rosters — coverage tables, evidence
//! bindings — keyed by operation name, and tests that hold them total
//! against the actual public surface in both directions. The pieces here
//! are the crate-agnostic layer:
//!
//! - [`SourceSpec`] and [`extract_public_fns`]: the public-surface
//!   extractor, so a totality test can hold "every public operation has
//!   exactly one row" in both directions.
//! - [`test_fns`]: the witness scanner, so a roster can require its cited
//!   evidence tests to exist as `#[test]`-attributed items, by name.
//!
//! Everything crate-specific — the roster rows themselves, evidence
//! vocabularies — lives in each consuming crate's own test modules.
//!
//! # Line discipline
//!
//! The scanners are line scans, not parsers, resting on rustfmt-normalized
//! shape: `impl` headers and `pub mod name {` blocks at column 0, inherent
//! methods and module-block functions at one indent. A `pub fn` at an
//! unexpected position panics rather than silently vanishing — the
//! extractor must never under-report the surface it exists to pin.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

#[cfg(test)]
mod tests;

/// A public-API source file a scanner walks, with the naming context the
/// file cannot carry itself.
pub struct SourceSpec {
    /// Path relative to the consuming crate's manifest directory.
    pub path: &'static str,
    /// Namespace for module-level `pub fn`s (`None`: the file must have
    /// none).
    pub module_prefix: Option<&'static str>,
    /// Public names for the file's inherent-impl types, keyed by local
    /// type name.
    ///
    /// For types whose local name is not their public path or that live
    /// under a public module. A type absent from the list keeps its
    /// parsed name (and a roster's totality test is what catches a
    /// mapping a new public type still needs).
    pub type_overrides: &'static [(&'static str, &'static str)],
}

/// The roster-facing name of one inherent-impl type: its
/// [`type_overrides`](SourceSpec::type_overrides) mapping, or the local
/// name itself.
fn public_type_name<'a>(spec: &SourceSpec, local: &'a str) -> &'a str {
    spec.type_overrides
        .iter()
        .find(|(from, _)| *from == local)
        .map_or(local, |(_, to)| *to)
}

/// Extract the `pub fn` surface from `specs` under `root`, named as a
/// roster names it: `Type::fn` inside an inherent impl block, `mod::fn`
/// inside a column-0 `pub mod` block, `module_prefix::fn` at file top
/// level.
///
/// Trait impl blocks (headers containing ` for `) cannot hold `pub fn`s
/// and are skipped. A `pub fn` at an unexpected position, or an `impl`
/// block nested inside a `pub mod` block, panics rather than silently
/// vanishing from the listing.
pub fn extract_public_fns(root: &Path, specs: &[SourceSpec]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for spec in specs {
        let path = root.join(spec.path);
        let text =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
        // The public name of the current inherent impl block, if inside one.
        let mut current_type: Option<String> = None;
        // The name of the current column-0 `pub mod` block, if inside one.
        let mut current_mod: Option<String> = None;
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("impl") {
                if line.contains(" for ") {
                    current_type = None; // trait impl: cannot hold `pub fn`
                } else {
                    current_type = parse_impl_self_type(rest)
                        .map(|name| public_type_name(spec, &name).to_owned());
                }
                continue;
            }
            if let Some(rest) = line.strip_prefix("pub mod ") {
                if line.ends_with('{') {
                    current_mod = Some(fn_name(rest).to_owned());
                }
                continue;
            }
            if line == "}" {
                current_type = None;
                current_mod = None;
                continue;
            }
            if current_mod.is_some() && line.starts_with("    impl") {
                panic!(
                    "{}: an impl block nested inside a `pub mod` block is beyond \
                     the extractor's line discipline",
                    spec.path
                );
            }
            if let Some(rest) = line.strip_prefix("    pub fn ") {
                let name = fn_name(rest);
                let context = current_type.as_deref().or(current_mod.as_deref());
                let ty = context.unwrap_or_else(|| {
                    panic!(
                        "{}: `pub fn {name}` outside an inherent impl or pub mod block",
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
pub fn parse_impl_self_type(rest: &str) -> Option<String> {
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
pub fn fn_name(rest: &str) -> &str {
    rest.split(|c: char| !c.is_alphanumeric() && c != '_')
        .next()
        .unwrap_or("")
}

/// Every `#[test]`-attributed function name in a source file.
///
/// The witness scanner behind rosters' evidence bindings:
/// attribute-gated, so a prose mention of a deleted test never counts as
/// its existence, and cfg attributes between `#[test]` and the fn keep
/// the arming.
pub fn test_fns(source: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut armed = false;
    for line in source.lines() {
        let t = line.trim();
        if t == "#[test]" {
            armed = true;
            continue;
        }
        if t.starts_with("#[") || t.is_empty() {
            continue;
        }
        if armed {
            if let Some(rest) = t.strip_prefix("fn ") {
                if let Some(name) = rest.split('(').next() {
                    names.insert(name.to_string());
                }
            }
            armed = false;
        }
    }
    names
}
