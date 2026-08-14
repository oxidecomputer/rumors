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
//! the source file and stops at the FIRST directory holding `lib.rs` or
//! `main.rs` — the deepest such anchor above the source — then reads
//! `<the anchor's parent>/proptest-regressions/<suffix>.txt`, where the
//! suffix is the source's path below the anchor; where no anchor exists
//! on the walk (integration-test binaries), it falls back to the
//! sibling `<source>.proptest-regressions` file. A seed anywhere else —
//! however plausible the path looks — is never read: it sits committed,
//! looking like coverage, replaying nothing. This sweep reconstructs
//! that resolution in reverse for every committed file under a seed
//! location and fails on any whose owning source is missing or whose
//! location proptest would not derive.
//!
//! Provenance of the transcribed rules: proptest 1.11.0,
//! `FileFailurePersistence::resolve` (`failure_persistence/file.rs`).
//! Re-verify the transcription when the workspace's proptest dependency
//! moves to a release that touches persistence resolution.

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

/// The first anchor on the walk from `start` up to `stop` (inclusive):
/// the directory whose parent hosts the `proptest-regressions` layout
/// proptest resolves for sources under `start`. `None` when the walk
/// finds no anchor, which routes persistence to the sibling-file
/// fallback instead.
///
/// The walk is bounded at `stop` (the repository root): proptest's own
/// walk continues to the filesystem root, and an anchor above the
/// repository would be pathological.
fn first_anchor_above(start: &Path, stop: &Path) -> Option<PathBuf> {
    let mut dir = start;
    loop {
        if is_anchor(dir) {
            return Some(dir.to_path_buf());
        }
        if dir == stop {
            return None;
        }
        dir = dir.parent()?;
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
                // at `D/<anchor dir>/<suffix>.rs` whose upward walk stops
                // at exactly that anchor. Anywhere else
                // (`tests/proptest-regressions/…` included) no resolution
                // produces the path, so every file under it is dead.
                let anchors: Vec<PathBuf> = std::fs::read_dir(dir)
                    .expect("workspace directory is readable")
                    .map(|e| e.expect("directory entry is readable").path())
                    .filter(|p| p.is_dir() && is_anchor(p))
                    .collect();
                collect_regressions(&path, &path, &anchors, root, out);
                continue;
            }
            scan(&path, root, out);
        } else if let Some(binary) = name.strip_suffix(".proptest-regressions") {
            // The `WithSource` fallback: `<source>.proptest-regressions`
            // beside the source, read only when the upward walk from the
            // source finds no anchor (otherwise `SourceParallel` wins and
            // this sibling file is never read).
            let owner = dir.join(format!("{binary}.rs"));
            let verdict = if !owner.is_file() {
                Err(format!(
                    "its owning source {} does not exist",
                    owner.display()
                ))
            } else if let Some(anchor) = first_anchor_above(dir, root) {
                Err(format!(
                    "the anchor {} above {} routes its tests' persistence to \
                     a proptest-regressions directory, so this sibling file \
                     is never read",
                    anchor.display(),
                    owner.display(),
                ))
            } else {
                Ok(())
            };
            out.push(Seed { path, verdict });
        }
    }
}

/// Collect every file under a `proptest-regressions` directory (rooted
/// at `regressions`), each judged against proptest's resolution: a
/// `.txt` must reverse-resolve to a live source whose deepest anchor is
/// one of `anchors`, and any other file is an orphan outright —
/// proptest writes and reads only `<suffix>.txt` here.
fn collect_regressions(
    dir: &Path,
    regressions: &Path,
    anchors: &[PathBuf],
    root: &Path,
    out: &mut Vec<Seed>,
) {
    for entry in std::fs::read_dir(dir).expect("seed directory is readable") {
        let path = entry.expect("directory entry is readable").path();
        if path.is_dir() {
            collect_regressions(&path, regressions, anchors, root, out);
            continue;
        }
        if !path.extension().is_some_and(|e| e == "txt") {
            out.push(Seed {
                path,
                verdict: Err("not a `.txt` seed file; proptest writes and reads only \
                     `<module path>.txt` under a proptest-regressions \
                     directory"
                    .to_string()),
            });
            continue;
        }
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
        } else if let Some(anchor) = anchors.iter().find(|a| a.join(&suffix).is_file()) {
            // The source exists below this anchor, but proptest resolves
            // at the DEEPEST anchor above the source: a nested anchor
            // between this one and the source claims the persistence and
            // orphans this file.
            let source = anchor.join(&suffix);
            let walk_from = source.parent().expect("a source file has a directory");
            match first_anchor_above(walk_from, root) {
                Some(found) if &found == anchor => Ok(()),
                Some(found) => Err(format!(
                    "the deeper anchor {} claims {}'s persistence (proptest \
                     stops at the first anchor above the source), so this \
                     file is never read",
                    found.display(),
                    source.display(),
                )),
                None => Err(format!(
                    "no anchor found above {} (the sweep's walk disagrees \
                     with the layout; re-verify the transcription)",
                    source.display(),
                )),
            }
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

/// A scratch directory tree for the fixture tests below, removed on drop.
struct FixtureTree(PathBuf);

impl FixtureTree {
    /// Create a unique empty directory under the system temp dir.
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("seed-liveness-{tag}-{}", std::process::id()));
        // A stale run's leftovers must not leak into this one.
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("fixture root is creatable");
        FixtureTree(dir)
    }

    /// Create `rel` (and its parents) with placeholder content.
    fn file(&self, rel: &str) -> &Self {
        let path = self.0.join(rel);
        std::fs::create_dir_all(path.parent().expect("fixture files have parents"))
            .expect("fixture directories are creatable");
        std::fs::write(path, b"// fixture\n").expect("fixture file is writable");
        self
    }

    /// Run the sweep over the fixture tree, returning each seed's
    /// root-relative path and verdict.
    fn sweep(&self) -> Vec<(String, Result<(), String>)> {
        let mut seeds = Vec::new();
        scan(&self.0, &self.0, &mut seeds);
        seeds
            .into_iter()
            .map(|seed| {
                let rel = seed
                    .path
                    .strip_prefix(&self.0)
                    .expect("seeds live under the fixture root")
                    .to_string_lossy()
                    .replace('\\', "/");
                (rel, seed.verdict)
            })
            .collect()
    }
}

impl Drop for FixtureTree {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Persistence resolves at the deepest anchor above the source, so a
/// seed filed under a shallower anchor's layout is an orphan even
/// though the source exists below that anchor — and the seed at the
/// deep anchor's own layout is the one that is read.
#[test]
fn nested_anchor_claims_the_persistence_path() {
    let tree = FixtureTree::new("nested-anchor");
    tree.file("src/lib.rs")
        .file("src/nested/main.rs")
        .file("src/nested/foo.rs")
        // Dead: resolves only if the walk stopped at src/, but it stops
        // at src/nested/ first.
        .file("proptest-regressions/nested/foo.txt")
        // Live: the deep anchor src/nested/ places persistence beside
        // itself, under its parent src/.
        .file("src/proptest-regressions/foo.txt");

    let mut verdicts = tree.sweep();
    verdicts.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(
        verdicts.len(),
        2,
        "both fixture seeds are swept: {verdicts:?}"
    );
    let (dead, live) = (&verdicts[0], &verdicts[1]);
    assert_eq!(dead.0, "proptest-regressions/nested/foo.txt");
    let why = dead
        .1
        .as_ref()
        .expect_err("the shallow-layout seed is dead");
    assert!(
        why.contains("deeper anchor"),
        "the conviction names the claiming anchor: {why}"
    );
    assert_eq!(live.0, "src/proptest-regressions/foo.txt");
    assert!(live.1.is_ok(), "the deep-anchor layout is read: {live:?}");
}

/// Any non-`.txt` file under a proptest-regressions directory is an
/// orphan outright: proptest writes and reads only `<suffix>.txt`
/// there, so anything else sits committed while replaying nothing.
#[test]
fn non_txt_files_under_regressions_are_orphans() {
    let tree = FixtureTree::new("non-txt");
    tree.file("src/lib.rs")
        .file("src/foo.rs")
        .file("proptest-regressions/foo.txt")
        .file("proptest-regressions/notes.md");

    let mut verdicts = tree.sweep();
    verdicts.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(verdicts.len(), 2, "both files are judged: {verdicts:?}");
    assert_eq!(verdicts[0].0, "proptest-regressions/foo.txt");
    assert!(verdicts[0].1.is_ok(), "the honest seed stays: {verdicts:?}");
    assert_eq!(verdicts[1].0, "proptest-regressions/notes.md");
    let why = verdicts[1]
        .1
        .as_ref()
        .expect_err("a non-.txt file is an orphan");
    assert!(
        why.contains("not a `.txt` seed file"),
        "named as such: {why}"
    );
}
