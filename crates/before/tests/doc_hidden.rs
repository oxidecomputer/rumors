//! The `#[doc(hidden)]` roster: every hidden public item is pinned by
//! name, so hiding surface from the totality pincer is tamper-evident.
//!
//! `#[doc(hidden)]` items never appear in rustdoc JSON, so the
//! surface-totality leg (`crates/before/surfacecheck`) cannot see them,
//! and the in-tree roster scan covers only inherent `pub fn`s in its
//! named source files — a hidden public item is reachable API that both
//! jaws of the pincer structurally miss. This pin closes that channel:
//! the committed roster below names every `#[doc(hidden)]` occurrence in
//! the library source, so adding one is a reviewable diff here, never a
//! silent escape.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Every `#[doc(hidden)]` occurrence in `src/`, as `(file, count)`.
///
/// The only entries are the sealed `PartyLiteral` trait and its method
/// (the `Party::try_from` tuple-literal plumbing): deliberately hidden,
/// sealed against implementation, and carrying no independent contract.
const DOC_HIDDEN_ROSTER: &[(&str, usize)] = &[("party.rs", 2)];

/// Collect `#[doc(hidden)]` occurrence counts per source file under
/// `dir`, keyed by the path relative to `src/`.
fn scan(dir: &Path, root: &Path, found: &mut BTreeMap<String, usize>) {
    for entry in std::fs::read_dir(dir).expect("src/ is readable") {
        let path = entry.expect("directory entry is readable").path();
        if path.is_dir() {
            scan(&path, root, found);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let text = std::fs::read_to_string(&path).expect("source file is readable");
            let count = text.matches("#[doc(hidden)]").count();
            if count > 0 {
                let rel = path
                    .strip_prefix(root)
                    .expect("scanned file lives under src/")
                    .to_string_lossy()
                    .into_owned();
                found.insert(rel, count);
            }
        }
    }
}

/// The library's `#[doc(hidden)]` occurrences match the committed
/// roster exactly, in both directions.
///
/// A new hidden public item (an escape from the surface-totality
/// pincer, which cannot see hidden items in rustdoc JSON) fails here
/// until it gains a reviewed roster entry, and a removed one orphans
/// its entry.
#[test]
fn doc_hidden_occurrences_match_the_committed_roster() {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut found = BTreeMap::new();
    scan(&src, &src, &mut found);
    let expected: BTreeMap<String, usize> = DOC_HIDDEN_ROSTER
        .iter()
        .map(|&(file, count)| (file.to_owned(), count))
        .collect();
    assert_eq!(
        found, expected,
        "#[doc(hidden)] occurrences drifted from the roster: hidden public \
         items are invisible to the surface-totality leg, so every one is \
         pinned here by name"
    );
}
