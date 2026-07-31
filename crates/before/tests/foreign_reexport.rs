//! The foreign re-export roster: every public re-export of a dependency
//! crate is pinned by name, so re-exporting foreign surface past the
//! totality pincer is tamper-evident.
//!
//! A `pub use` of another crate's type (say `pub use bytes::Bytes;` at
//! the root) makes that type's entire inherent method surface reachable
//! `before` API — yet both jaws of the surface-totality pincer are
//! structurally blind to it: the rustdoc-JSON leg
//! (`crates/before/surfacecheck`) skips any `use` whose target id is
//! not in the local index (foreign items never are), and the in-tree
//! roster scan covers only inherent `pub fn`s declared in its named
//! source files. Demonstrated: with `pub use bytes::Bytes;` added at
//! the crate root, the surface-totality leg reads the same 197 items
//! and exits clean. `pub extern crate <dep>` and a `pub type` alias of
//! a foreign type open the same door through different spellings, so
//! the scan matches all three. This pin closes the channel: the
//! committed roster below names every dependency re-export in the
//! library source (today: none), so adding one is a reviewable diff
//! here — the reviewer then decides whether the foreign surface needs
//! roster rows, an exception, or a wrapper type — never a silent
//! escape.

use std::path::{Path, PathBuf};

/// Every allowed public re-export of a dependency (`pub use`,
/// `pub extern crate`, or a `pub type` alias of a foreign type), as
/// `(file, line-content)` — empty at this tip: `before` re-exports no
/// foreign surface.
const FOREIGN_REEXPORT_ROSTER: &[(&str, &str)] = &[];

/// The dependency crate names of `before`, read mechanically from
/// `[dependencies]` in its Cargo.toml (keys normalized `-` to `_`, the
/// spelling a `use` path must write).
fn dependency_names(manifest: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut in_deps = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_deps = line == "[dependencies]";
            continue;
        }
        if !in_deps || line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, _)) = line.split_once('=') {
            let key = key.trim();
            if !key.is_empty() && key.chars().all(|c| c.is_alphanumeric() || "-_".contains(c)) {
                names.push(key.replace('-', "_"));
            }
        }
    }
    assert!(
        !names.is_empty(),
        "the manifest parse found no [dependencies]: the scan below would be vacuous"
    );
    names
}

/// Collect every non-comment source line under `dir` that `pub use`s a
/// dependency crate, as `(file relative to src/, line content)`.
fn scan(dir: &Path, root: &Path, deps: &[String], found: &mut Vec<(String, String)>) {
    for entry in std::fs::read_dir(dir).expect("src/ is readable") {
        let path = entry.expect("directory entry is readable").path();
        if path.is_dir() {
            scan(&path, root, deps, found);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let text = std::fs::read_to_string(&path).expect("source file is readable");
            for line in text.lines() {
                let trimmed = line.trim_start();
                if trimmed.starts_with("//") {
                    continue;
                }
                // The three spellings that publish foreign surface: a
                // re-export, a whole-crate re-export, and a type alias
                // whose target is foreign (its inherent methods become
                // reachable at the alias).
                let pathed = trimmed.contains("pub use") || trimmed.contains("pub type");
                let whole_crate = trimmed.contains("pub extern crate");
                if !(pathed || whole_crate) {
                    continue;
                }
                if deps.iter().any(|dep| {
                    (pathed
                        && (trimmed.contains(&format!("{dep}::"))
                            || trimmed.contains(&format!("::{dep}"))))
                        || (whole_crate && trimmed.contains(&format!(" {dep}")))
                }) {
                    let rel = path
                        .strip_prefix(root)
                        .expect("scanned file lives under src/")
                        .to_string_lossy()
                        .into_owned();
                    found.push((rel, trimmed.to_owned()));
                }
            }
        }
    }
}

/// The library's dependency re-exports match the committed roster exactly.
///
/// A foreign type re-exported into `before`'s public surface is
/// invisible to both jaws of the surface-totality pincer (the
/// rustdoc-JSON leg skips foreign ids; the in-tree roster scan sees
/// only local inherent `pub fn`s), so every occurrence is pinned here
/// by name.
#[test]
fn dependency_reexports_match_the_committed_roster() {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let manifest =
        std::fs::read_to_string(crate_root.join("Cargo.toml")).expect("Cargo.toml is readable");
    let deps = dependency_names(&manifest);
    let src = crate_root.join("src");
    let mut found = Vec::new();
    scan(&src, &src, &deps, &mut found);
    found.sort();
    let expected: Vec<(String, String)> = FOREIGN_REEXPORT_ROSTER
        .iter()
        .map(|&(file, line)| (file.to_owned(), line.to_owned()))
        .collect();
    assert_eq!(
        found, expected,
        "dependency re-exports drifted from the roster: a re-exported foreign \
         type's methods are public before API that the surface-totality \
         pincer structurally cannot see, so every occurrence is pinned here"
    );
}
