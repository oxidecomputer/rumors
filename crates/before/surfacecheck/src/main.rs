//! Surface totality against rustdoc JSON: every public function-like item
//! of `before` is rostered or excepted, checked from the compiler's own
//! account of the public surface.
//!
//! The surface roster (`before::surface::METHOD_SURFACE`) is enforced
//! in-tree against a line-scan extractor over a hand-maintained source
//! list, which cannot see a public item added in a file the list does not
//! name. This binary closes that hole from the other side: it parses the
//! nightly rustdoc JSON for `before` (built by the `just surface-totality`
//! recipe with `--all-features`, so feature-gated modules are visible),
//! walks the publicly reachable item tree, and holds every public
//! function-like item — free functions, inherent methods, and
//! public-trait-declared methods — to exactly one of two dispositions:
//! a roster row in `METHOD_SURFACE`, or a named, dated exception in
//! [`check`]. Trait-*impl* methods (operators, `Display`/`FromStr`,
//! serde/borsh) are out of scope here: the roster covers them by family
//! (`FAMILY_SURFACE`), whose totality is by review.
//!
//! Exit status is the verdict: zero with a one-line census on a clean
//! sweep, nonzero with every finding named otherwise. The check runs
//! before parsing anything else: a `format_version` mismatch between the
//! JSON and the pinned schema crate is a loud, named error, never a
//! silently wrong parse.

use std::collections::BTreeSet;
use std::process::ExitCode;

mod check;
mod extract;

/// Read the rustdoc JSON at the path given as the sole CLI argument,
/// refuse a format-version mismatch, and reconcile the extracted surface
/// against the roster and the exception lists.
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

    let extracted = extract::function_like_items(&krate);
    let rostered: BTreeSet<&str> = before::surface::METHOD_SURFACE
        .iter()
        .map(|row| row.op)
        .collect();
    if list.is_some() {
        for name in &extracted {
            let disposition = if rostered.contains(name.as_str()) {
                "rostered".to_owned()
            } else if check::ITEM_EXCEPTIONS.iter().any(|e| e.name == name) {
                "excepted (item)".to_owned()
            } else if let Some(e) = check::MODULE_EXCEPTIONS
                .iter()
                .find(|e| name.starts_with(e.name))
            {
                format!("excepted (module {})", e.name)
            } else {
                "UNROSTERED".to_owned()
            };
            println!("{name:60} {disposition}");
        }
    }
    let findings = check::reconcile(&extracted, &rostered);
    if findings.is_clean() {
        println!(
            "surface totality: {} public function-like items = {} rostered + {} \
             excepted ({} item exceptions, {} module-scope)",
            extracted.len(),
            rostered.len(),
            extracted.len() - rostered.len(),
            check::ITEM_EXCEPTIONS.len(),
            check::MODULE_EXCEPTIONS.len(),
        );
        ExitCode::SUCCESS
    } else {
        eprint!("{}", findings.render());
        ExitCode::FAILURE
    }
}
