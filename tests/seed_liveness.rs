//! The proptest seed files' provenance sweep: every committed seed
//! lives at a path proptest's configured persistence actually derives
//! from a live test source.
//!
//! No regression corpus may go silently dead when a module moves,
//! retires, or a seed lands in a location the library never reads.
//!
//! Committed seeds are instruments of record — each `cc` line replays a
//! shrunk failure before novel cases are generated — but a seed replays
//! only from the one path proptest resolves from its test's source
//! location. The default persistence (`SourceParallel`) walks up from
//! the source file to the directory holding `lib.rs` or `main.rs` and
//! reads `<its parent>/proptest-regressions/<suffix>.txt`; where no such
//! anchor exists on the walk (integration-test binaries), it falls back
//! to the sibling `<source>.proptest-regressions` file. A seed anywhere
//! else — however plausible the path looks — is never read: it sits
//! committed, looking like coverage, replaying nothing. This sweep
//! reconstructs that resolution in reverse for every committed seed and
//! fails on any file whose owning source is missing or whose location
//! proptest would not derive.

use std::path::{Path, PathBuf};

/// Directories the walk never descends into: build output, VCS, and
/// vendored/editor trees that cannot own seed files.
const SKIP_DIRS: &[&str] = &["target", ".git", "node_modules", ".claude"];

/// One committed seed file and the verdict of reversing proptest's
/// persistence resolution on it.
struct Seed {
    path: PathBuf,
    verdict: Result<(), String>,
}

/// Whether `dir` directly contains the `lib.rs`/`main.rs` anchor that
/// stops proptest's upward walk from a source file.
fn is_anchor(dir: &Path) -> bool {
    dir.join("lib.rs").is_file() || dir.join("main.rs").is_file()
}

/// Whether any directory on the walk from `start` up to `stop`
/// (inclusive) is an anchor: if so, a source under `start` persists via
/// the `SourceParallel` layout and its sibling fallback file is never
/// read.
fn anchored_walk(start: &Path, stop: &Path) -> bool {
    let mut dir = start;
    loop {
        if is_anchor(dir) {
            return true;
        }
        if dir == stop {
            return false;
        }
        match dir.parent() {
            Some(parent) => dir = parent,
            None => return false,
        }
    }
}

/// Walk `dir` collecting every committed seed location with its verdict.
fn scan(dir: &Path, root: &Path, out: &mut Vec<Seed>) {
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
                // The `SourceParallel` layout: proptest reads
                // `D/proptest-regressions/<suffix>.txt` only for a source
                // at `D/<anchor dir>/<suffix>.rs`, where the anchor dir
                // directly contains `lib.rs` or `main.rs`. Anywhere else
                // (`tests/proptest-regressions/…` included) no resolution
                // produces the path, so every file under it is dead.
                let anchors: Vec<PathBuf> = std::fs::read_dir(dir)
                    .expect("workspace directory is readable")
                    .map(|e| e.expect("directory entry is readable").path())
                    .filter(|p| p.is_dir() && is_anchor(p))
                    .collect();
                collect_txt(&path, &path, &anchors, out);
                continue;
            }
            scan(&path, root, out);
        } else if let Some(binary) = name.strip_suffix(".proptest-regressions") {
            // The `WithSource` fallback: `<source>.proptest-regressions`
            // beside the source, read only when the upward walk from the
            // source finds no anchor (otherwise `SourceParallel` wins and
            // this sibling file is dead). The walk is bounded at the
            // repository root: proptest's own walk continues to the
            // filesystem root, and an anchor above the repository would
            // be pathological.
            let owner = dir.join(format!("{binary}.rs"));
            let verdict = if !owner.is_file() {
                Err(format!(
                    "its owning source {} does not exist",
                    owner.display()
                ))
            } else if anchored_walk(dir, root) {
                Err(format!(
                    "a `lib.rs`/`main.rs` anchor on the walk above {} routes its \
                     tests' persistence to a proptest-regressions directory, so \
                     this sibling file is never read",
                    owner.display(),
                ))
            } else {
                Ok(())
            };
            out.push(Seed { path, verdict });
        }
    }
}

/// Collect every `.txt` under a `proptest-regressions` directory (rooted
/// at `regressions`), each required to reverse-resolve to a live source
/// under one of `anchors`.
fn collect_txt(dir: &Path, regressions: &Path, anchors: &[PathBuf], out: &mut Vec<Seed>) {
    for entry in std::fs::read_dir(dir).expect("seed directory is readable") {
        let path = entry.expect("directory entry is readable").path();
        if path.is_dir() {
            collect_txt(&path, regressions, anchors, out);
        } else if path.extension().is_some_and(|e| e == "txt") {
            let suffix = path
                .strip_prefix(regressions)
                .expect("seed file lives under its regression root")
                .with_extension("rs");
            let verdict = if anchors.is_empty() {
                Err(format!(
                    "{} has no sibling source directory containing `lib.rs` or \
                     `main.rs`, so proptest resolves no persistence path into it \
                     (for `tests/` suites the live location is the sibling \
                     `tests/<binary>.proptest-regressions` file)",
                    regressions.display(),
                ))
            } else if anchors.iter().any(|a| a.join(&suffix).is_file()) {
                Ok(())
            } else {
                Err(format!(
                    "no live source {} exists under {}",
                    suffix.display(),
                    anchors
                        .iter()
                        .map(|a| a.display().to_string())
                        .collect::<Vec<_>>()
                        .join(" or "),
                ))
            };
            out.push(Seed { path, verdict });
        }
    }
}

/// Every committed proptest seed file resolves, through proptest's own
/// persistence rules, to a live test source file.
///
/// An orphaned corpus — its module moved or retired, or the file sitting
/// at a path no resolution derives — replays nowhere: a dead instrument
/// that still looks like coverage. Retiring a module, or relocating a
/// seed, means deciding in a reviewed diff what happens to its seeds,
/// never orphaning them silently.
#[test]
fn every_committed_seed_file_is_read_by_proptest() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut seeds = Vec::new();
    scan(&root, &root, &mut seeds);
    assert!(
        !seeds.is_empty(),
        "the walk found no seed files at all: the sweep would be vacuous"
    );
    let orphans: Vec<String> = seeds
        .iter()
        .filter_map(|seed| {
            seed.verdict.as_ref().err().map(|why| {
                format!(
                    "{}: {why}",
                    seed.path
                        .strip_prefix(&root)
                        .unwrap_or(&seed.path)
                        .display(),
                )
            })
        })
        .collect();
    assert!(
        orphans.is_empty(),
        "committed seed files proptest never reads: {orphans:#?}"
    );
}
