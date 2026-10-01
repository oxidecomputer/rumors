//! Public-surface totality against rustdoc JSON.
//!
//! This binary parses `before`'s nightly rustdoc JSON, built with all features
//! so feature-gated modules remain visible. It walks the publicly reachable
//! item tree and gives every item exactly one disposition:
//!
//! - function-like items (free functions, inherent methods, and
//!   public-trait-declared methods): one disposition in the amplification
//!   board, or a named exception in [`check`];
//! - trait implementations: a pin in [`census::TRAIT_IMPLS`], reconciled in
//!   both directions. The board records their reviewed resource decisions by
//!   trait family;
//! - associated consts and types, module consts, statics, and macros: a
//!   pinned row in [`census::ITEMS`], reconciled the same way.
//!
//! Exit status is the verdict: zero with a one-line census on a clean
//! sweep, nonzero with every finding named otherwise. The check runs
//! before parsing anything else: a `format_version` mismatch between the
//! JSON and the pinned schema crate is a loud, named error, never a
//! silently wrong parse.

use std::collections::BTreeSet;
use std::process::ExitCode;

mod census;
mod check;
mod extract;

/// The board entries that name public free functions or inherent methods.
///
/// Grouped trait families use explanatory phrases containing spaces or
/// punctuation. Rust paths contain only identifier characters and `::`, so
/// this split follows the labels' syntax rather than another hand-maintained
/// list. A root free function is a one-segment path.
fn board_function_inventory() -> BTreeSet<&'static str> {
    before::testing::meter::board::BOARD_PRICED
        .iter()
        .map(|(operation, _)| *operation)
        .chain(
            before::testing::meter::board::BOARD_NOT_APPLICABLE
                .iter()
                .map(|(operation, _)| *operation),
        )
        .filter(|operation| is_rust_path(operation))
        .collect()
}

/// Whether an operation label is a Rust item path rather than a grouped trait
/// family description.
fn is_rust_path(operation: &str) -> bool {
    operation
        .split("::")
        .all(|part| !part.is_empty() && part.chars().all(|c| c == '_' || c.is_alphanumeric()))
}

/// Read the rustdoc JSON at the path given as the sole CLI argument,
/// refuse a format-version mismatch, and reconcile the extracted surface.
///
/// The check covers resource decisions, pinned censuses, and named exceptions.
fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    // `--list` renders the census (every extracted item with its
    // disposition) before the verdict, for triage and review.
    let list = args.iter().position(|a| a == "--list").inspect(|&i| {
        args.remove(i);
    });
    let [path] = args.as_slice() else {
        eprintln!("usage: surfacecheck [--list] <path to before.json>");
        return ExitCode::FAILURE;
    };
    let path = path.as_str();
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) => {
            eprintln!("surfacecheck: reading {path}: {err}");
            return ExitCode::FAILURE;
        }
    };
    let value: serde_json::Value = match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("surfacecheck: {path} is not JSON: {err}");
            return ExitCode::FAILURE;
        }
    };

    // The format gate, before any schema-typed parse: a nightly that
    // emits a different format version must fail HERE, naming both
    // numbers — deserializing mismatched JSON through the pinned schema
    // could otherwise succeed incidentally and report a wrong surface.
    let found = value.get("format_version").and_then(|v| v.as_u64());
    if found != Some(u64::from(rustdoc_types::FORMAT_VERSION)) {
        eprintln!(
            "surfacecheck: rustdoc JSON format_version mismatch: the document at \
             {path} carries {found:?}, but the pinned rustdoc-types crate speaks \
             format {}.\n\
             The nightly toolchain and this check must move together: bump the \
             `rustdoc-types` pin in crates/before/surfacecheck/Cargo.toml to the \
             release whose FORMAT_VERSION matches the new nightly's output (the \
             justfile's surface-totality recipe comment documents the procedure), \
             then re-run the gate.",
            rustdoc_types::FORMAT_VERSION,
        );
        return ExitCode::FAILURE;
    }

    let krate: rustdoc_types::Crate = match serde_json::from_value(value) {
        Ok(krate) => krate,
        Err(err) => {
            eprintln!(
                "surfacecheck: {path} does not deserialize as rustdoc JSON \
                 format {} despite carrying that format_version: {err}",
                rustdoc_types::FORMAT_VERSION,
            );
            return ExitCode::FAILURE;
        }
    };

    let surface = extract::public_surface(&krate);
    let board = board_function_inventory();
    if list.is_some() {
        render_list(&surface, &board);
    }
    let findings = check::reconcile(&surface, &board);
    if findings.is_clean() {
        let outside = |set: &BTreeSet<String>| {
            set.iter()
                .filter(|row| {
                    !check::MODULE_EXCEPTIONS
                        .iter()
                        .any(|e| row.starts_with(e.name))
                })
                .count()
        };
        println!(
            "surface totality: {} public function-like items = {} board-covered + {} \
             excepted ({} item exceptions, {} module-scope); {} trait impls = {} \
             pinned + {} module-excepted; {} items = {} pinned + {} module-excepted",
            surface.functions.len(),
            board.len(),
            surface.functions.len() - board.len(),
            check::ITEM_EXCEPTIONS.len(),
            check::MODULE_EXCEPTIONS.len(),
            surface.impls.len(),
            outside(&surface.impls),
            surface.impls.len() - outside(&surface.impls),
            surface.items.len(),
            outside(&surface.items),
            surface.items.len() - outside(&surface.items),
        );
        ExitCode::SUCCESS
    } else {
        eprint!("{}", findings.render());
        ExitCode::FAILURE
    }
}

/// Render every extracted row with its disposition, category by
/// category, for triage and review.
fn render_list(surface: &extract::Surface, board: &BTreeSet<&str>) {
    let module_exception = |name: &str| {
        check::MODULE_EXCEPTIONS
            .iter()
            .find(|e| name.starts_with(e.name))
    };
    for name in &surface.functions {
        let disposition = if board.contains(name.as_str()) {
            "board".to_owned()
        } else if check::ITEM_EXCEPTIONS.iter().any(|e| e.name == name) {
            "excepted (item)".to_owned()
        } else if let Some(e) = module_exception(name) {
            format!("excepted (module {})", e.name)
        } else {
            "UNCOVERED".to_owned()
        };
        println!("{name:60} {disposition}");
    }
    let census_disposition = |row: &String, pinned: &[&str]| {
        if pinned.contains(&row.as_str()) {
            "pinned".to_owned()
        } else if let Some(e) = module_exception(row) {
            format!("excepted (module {})", e.name)
        } else {
            "UNPINNED".to_owned()
        }
    };
    for row in &surface.impls {
        println!("{row:100} {}", census_disposition(row, census::TRAIT_IMPLS));
    }
    for row in &surface.items {
        println!("{row:60} {}", census_disposition(row, census::ITEMS));
    }
}

#[cfg(test)]
mod inventory_tests {
    use super::is_rust_path;

    /// Item paths are admitted while descriptive trait-family labels remain
    /// under the separate trait census.
    #[test]
    fn function_inventory_distinguishes_paths_from_families() {
        assert!(is_rust_path("root_function"));
        assert!(is_rust_path("Party::seed"));
        assert!(is_rust_path("causally::strictly_before"));
        assert!(!is_rust_path("Version PartialOrd (owned and borrowed)"));
        assert!(!is_rust_path("serde / borsh impls"));
    }
}
