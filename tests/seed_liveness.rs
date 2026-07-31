//! The proptest seed files' liveness sweep: every committed seed maps
//! to a live test source file, so no regression corpus goes silently
//! dead when a module moves or retires.
//!
//! Committed seeds are instruments of record — each `cc` line replays a
//! shrunk failure before novel cases are generated — but a seed replays
//! only from the path proptest derives from its test's source location.
//! A module rename or removal orphans its seed file: every seed in it
//! silently stops replaying while the file sits committed, looking like
//! coverage. This sweep walks the workspace for both committed layouts
//! (`<crate>/proptest-regressions/<module path>.txt` for `src/` tests,
//! and the per-binary forms next to and under `tests/`) and requires
//! the owning source file to exist, so an orphaned corpus is a red test
//! naming the seed, never a quiet death.

use std::path::{Path, PathBuf};

/// Directories the walk never descends into: build output, VCS, and
/// vendored/editor trees that cannot own seed files.
const SKIP_DIRS: &[&str] = &["target", ".git", "node_modules", ".claude"];

/// Walk `dir` collecting every proptest seed location, as
/// `(seed path, owning source path it requires)`.
fn scan(dir: &Path, out: &mut Vec<(PathBuf, PathBuf)>) {
    for entry in std::fs::read_dir(dir).expect("workspace directory is readable") {
        let path = entry.expect("directory entry is readable").path();
        let name = path
            .file_name()
            .expect("read_dir yields named entries")
            .to_string_lossy()
            .into_owned();
        if path.is_dir() {
            if SKIP_DIRS.contains(&name.as_str()) {
                continue;
            }
            if name == "proptest-regressions" {
                let crate_root = dir.parent().filter(|_| dir.ends_with("tests"));
                match crate_root {
                    // `tests/proptest-regressions/<binary>.txt`: the
                    // integration-suite layout, owned by
                    // `tests/<binary>.rs`.
                    Some(_) => collect_txt(&path, &path, dir, out),
                    // `<crate>/proptest-regressions/<module path>.txt`:
                    // the `src/` layout, owned by `src/<module path>.rs`.
                    None => collect_txt(&path, &path, &dir.join("src"), out),
                }
                continue;
            }
            scan(&path, out);
        } else if let Some(binary) = name.strip_suffix(".proptest-regressions") {
            // `tests/<binary>.proptest-regressions`: the per-binary
            // sibling layout, owned by `tests/<binary>.rs`.
            out.push((path.clone(), dir.join(format!("{binary}.rs"))));
        }
    }
}

/// Collect every `.txt` under `dir` (rooted at `root`), each requiring
/// the parallel `.rs` under `owner_root`.
fn collect_txt(dir: &Path, root: &Path, owner_root: &Path, out: &mut Vec<(PathBuf, PathBuf)>) {
    for entry in std::fs::read_dir(dir).expect("seed directory is readable") {
        let path = entry.expect("directory entry is readable").path();
        if path.is_dir() {
            collect_txt(&path, root, owner_root, out);
        } else if path.extension().is_some_and(|e| e == "txt") {
            let rel = path
                .strip_prefix(root)
                .expect("seed file lives under its regression root")
                .with_extension("rs");
            out.push((path.clone(), owner_root.join(rel)));
        }
    }
}

/// Every committed proptest seed file maps to a live test source file.
///
/// An orphaned seed corpus (its module moved or retired) replays
/// nowhere: a dead instrument that still looks like coverage. Retiring
/// a module means deciding — in a reviewed diff — what happens to its
/// seeds, never orphaning them silently.
#[test]
fn every_committed_seed_file_has_a_live_owner() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut seeds = Vec::new();
    scan(&root, &mut seeds);
    assert!(
        !seeds.is_empty(),
        "the walk found no seed files at all: the sweep would be vacuous"
    );
    let orphans: Vec<String> = seeds
        .iter()
        .filter(|(_, owner)| !owner.is_file())
        .map(|(seed, owner)| {
            format!(
                "{} (expects {})",
                seed.strip_prefix(&root).unwrap_or(seed).display(),
                owner.strip_prefix(&root).unwrap_or(owner).display(),
            )
        })
        .collect();
    assert!(
        orphans.is_empty(),
        "committed seed files with no live owning test source: {orphans:#?}"
    );
}
