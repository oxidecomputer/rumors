//! The insta snapshot corpus's provenance sweep: every committed
//! snapshot resolves to a live generating test.
//!
//! No pinned wire capture or format pin may go silently dead when a
//! test is renamed, moved, or retired: an orphaned `.snap` sits
//! committed, looking like a byte-for-byte pin, while nothing ever
//! compares against it again.
//!
//! The resolution rules this sweep reverses, transcribed from insta's
//! snapshot-path derivation (the snapshot lands in a `snapshots`
//! directory beside the asserting module's source file, named by the
//! crate, the module path, and the snapshot name, `__`-separated):
//!
//! - An integration-test snapshot lives at
//!   `tests/snapshots/<suite>__<name>.snap`: the stem splits at its
//!   first `__` into the suite binary (`tests/<suite>.rs`) and the
//!   snapshot name.
//! - A unit-test snapshot lives at
//!   `<source dir>/snapshots/rumors__<module path>__<name>.snap`,
//!   where the `__`-separated module path starts with the source
//!   directory's own path below `src/`, continues with the module
//!   file's name, and ends with the snapshot name. This transcription
//!   assumes the tree's file-per-module layout (`mod foo;` with a
//!   sibling `foo.rs`; no snapshot-minting inline modules); a snapshot
//!   from an inline module fails here, and extending the rule is then
//!   a reviewed decision, not a silent skip.
//!
//! A snapshot's name is its generating test's function name, or the
//! explicit first argument of `insta::assert_snapshot!("<name>", …)`
//! (the bookmark format pins use the explicit form). The sweep
//! therefore requires the resolved source to contain `fn <name>(` or
//! the quoted string `"<name>"`, and fails naming the orphan and the
//! file it searched. Nothing under a snapshots directory is skipped: a
//! pending `.snap.new` (unaccepted insta output) fails loudly, and so
//! does any other stray file.
//!
//! The containment rule is a substring match, and that carries the
//! pairing design's accepted residual: a renamed test's orphan stays
//! green when a same-named `fn`, string, or comment survives in the
//! resolved source, and a live function whose snapshot assertion was
//! deleted also passes — the sweep pairs names with sources, not
//! assertions with snapshots.
//!
//! Provenance of the transcribed rules: insta 1.x snapshot-path
//! resolution (module path plus source-file directory). Re-verify the
//! transcription when the workspace's insta dependency moves to a
//! release that touches snapshot-path resolution.

use std::path::{Path, PathBuf};

/// One committed snapshot-corpus file and the verdict of reversing
/// insta's path resolution on it.
struct Snap {
    path: PathBuf,
    verdict: Result<(), String>,
}

/// Whether `source` contains a generator for snapshot `name`: the test
/// function itself, or the explicit name passed to the assertion.
fn generates(source: &str, name: &str) -> bool {
    source.contains(&format!("fn {name}(")) || source.contains(&format!("\"{name}\""))
}

/// Judge one snapshot name against its resolved `source` file: the
/// file must exist and contain a generator for `name`.
fn contained(source: &Path, name: &str) -> Result<(), String> {
    let Ok(text) = std::fs::read_to_string(source) else {
        return Err(format!(
            "its generating source {} does not exist",
            source.display()
        ));
    };
    if generates(&text, name) {
        Ok(())
    } else {
        Err(format!(
            "{} contains neither `fn {name}(` nor `\"{name}\"`",
            source.display()
        ))
    }
}

/// Classify one file name under a snapshots directory: the stem of a
/// committed `.snap`, or the conviction for everything else (nothing
/// is skipped).
fn snap_stem(name: &str) -> Result<&str, String> {
    if name.ends_with(".snap.new") {
        Err(
            "unaccepted snapshot committed: `.snap.new` is insta's pending \
             output; accept it deliberately (`cargo insta review`) or delete \
             it"
            .to_owned(),
        )
    } else if let Some(stem) = name.strip_suffix(".snap") {
        Ok(stem)
    } else {
        Err(
            "not a `.snap` snapshot; nothing else belongs under a snapshots \
             directory"
                .to_owned(),
        )
    }
}

/// Judge every file under `<root>/tests/snapshots`: the stem's prefix
/// before the first `__` names the suite binary, the rest the
/// snapshot.
fn scan_tests_side(root: &Path, out: &mut Vec<Snap>) {
    let dir = root.join("tests").join("snapshots");
    for entry in std::fs::read_dir(&dir).expect("tests/snapshots is readable") {
        let path = entry.expect("directory entry is readable").path();
        let name = path
            .file_name()
            .expect("read_dir yields named entries")
            .to_string_lossy()
            .into_owned();
        let verdict = match snap_stem(&name) {
            Err(why) => Err(why),
            Ok(stem) => match stem.split_once("__") {
                None => Err("the stem has no `__` separating the suite binary \
                             from the snapshot name"
                    .to_owned()),
                Some((suite, name)) => {
                    contained(&root.join("tests").join(format!("{suite}.rs")), name)
                }
            },
        };
        out.push(Snap { path, verdict });
    }
}

/// Walk `dir` (under the crate's `src`) collecting every snapshots
/// directory's files with their verdicts.
fn scan_src_side(src: &Path, dir: &Path, out: &mut Vec<Snap>) {
    for entry in std::fs::read_dir(dir).expect("source directory is readable") {
        let path = entry.expect("directory entry is readable").path();
        if !path.is_dir() {
            continue;
        }
        if path.file_name().is_some_and(|n| n == "snapshots") {
            judge_snapshots_dir(src, &path, out);
        } else {
            scan_src_side(src, &path, out);
        }
    }
}

/// Judge every file inside one src-side `snapshots` directory against
/// the module layout around it.
fn judge_snapshots_dir(src: &Path, snapshots: &Path, out: &mut Vec<Snap>) {
    let owner_dir = snapshots
        .parent()
        .expect("a snapshots directory has a parent");
    let components: Vec<String> = owner_dir
        .strip_prefix(src)
        .expect("the walk stays under src")
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    for entry in std::fs::read_dir(snapshots).expect("snapshots directory is readable") {
        let path = entry.expect("directory entry is readable").path();
        if path.is_dir() {
            out.push(Snap {
                path,
                verdict: Err("a directory under a snapshots directory; insta \
                              writes flat files only"
                    .to_owned()),
            });
            continue;
        }
        let name = path
            .file_name()
            .expect("read_dir yields named entries")
            .to_string_lossy()
            .into_owned();
        let verdict = match snap_stem(&name) {
            Err(why) => Err(why),
            Ok(stem) => judge_src_stem(owner_dir, &components, stem),
        };
        out.push(Snap { path, verdict });
    }
}

/// Reverse-resolve one src-side snapshot stem: the crate prefix, then
/// the module-path segments spelling the snapshot directory's own
/// location, then the module file, then the snapshot name.
fn judge_src_stem(owner_dir: &Path, components: &[String], stem: &str) -> Result<(), String> {
    let Some(rest) = stem.strip_prefix("rumors__") else {
        return Err("the stem does not open with this crate's `rumors__` prefix".to_owned());
    };
    let segments: Vec<&str> = rest.split("__").collect();
    if segments.len() < components.len() + 2 {
        return Err(format!(
            "the module path is too short to spell {} plus a module file and \
             a snapshot name",
            owner_dir.display()
        ));
    }
    let (dir_segments, rest_segments) = segments.split_at(components.len());
    if dir_segments
        .iter()
        .zip(components)
        .any(|(s, c)| *s != c.as_str())
    {
        return Err(format!(
            "the module path does not spell the snapshot directory's own \
             location {}",
            owner_dir.display()
        ));
    }
    let source = owner_dir.join(format!("{}.rs", rest_segments[0]));
    contained(&source, &rest_segments[1..].join("__"))
}

/// Every committed insta snapshot resolves, through insta's own path
/// rules, to a live generating test.
///
/// The corpus is the wire-format pin: an orphaned snapshot — its test
/// renamed, moved, or retired — is a dead instrument that still looks
/// like a byte-for-byte guarantee. Retiring or renaming a snapshot
/// test means deciding in a reviewed diff what happens to its
/// snapshots, never orphaning them silently.
#[test]
fn every_committed_snapshot_has_a_live_generator() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut snaps = Vec::new();
    scan_tests_side(&root, &mut snaps);
    let tests_side = snaps.len();
    scan_src_side(&root.join("src"), &root.join("src"), &mut snaps);
    assert!(
        tests_side > 0,
        "no integration-suite snapshots found: the sweep would be vacuous"
    );
    assert!(
        snaps.len() > tests_side,
        "no unit-test snapshots found: the sweep would be vacuous"
    );
    let orphans: Vec<String> = snaps
        .iter()
        .filter_map(|snap| {
            snap.verdict.as_ref().err().map(|why| {
                format!(
                    "{}: {why}",
                    snap.path
                        .strip_prefix(&root)
                        .unwrap_or(&snap.path)
                        .display(),
                )
            })
        })
        .collect();
    assert!(
        orphans.is_empty(),
        "committed snapshots no test generates: {orphans:#?}"
    );
}

/// A scratch directory tree for the fixture tests below, removed on
/// drop.
struct FixtureTree(PathBuf);

impl FixtureTree {
    /// Create a unique empty directory under the system temp dir.
    fn new(tag: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("snapshot-liveness-{tag}-{}", std::process::id()));
        // A stale run's leftovers must not leak into this one.
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("fixture root is creatable");
        FixtureTree(dir)
    }

    /// Create `rel` (and its parents) holding `content`.
    fn file(&self, rel: &str, content: &str) -> &Self {
        let path = self.0.join(rel);
        std::fs::create_dir_all(path.parent().expect("fixture files have parents"))
            .expect("fixture directories are creatable");
        std::fs::write(path, content).expect("fixture file is writable");
        self
    }

    /// Run both sweeps over the fixture tree, returning each
    /// snapshot's root-relative path and verdict, path-sorted.
    fn sweep(&self) -> Vec<(String, Result<(), String>)> {
        let mut snaps = Vec::new();
        scan_tests_side(&self.0, &mut snaps);
        scan_src_side(&self.0.join("src"), &self.0.join("src"), &mut snaps);
        let mut verdicts: Vec<(String, Result<(), String>)> = snaps
            .into_iter()
            .map(|snap| {
                let rel = snap
                    .path
                    .strip_prefix(&self.0)
                    .expect("snapshots live under the fixture root")
                    .to_string_lossy()
                    .replace('\\', "/");
                (rel, snap.verdict)
            })
            .collect();
        verdicts.sort_by(|a, b| a.0.cmp(&b.0));
        verdicts
    }
}

impl Drop for FixtureTree {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// An integration-suite snapshot is convicted when its generator is
/// missing, and stays green when it exists.
///
/// The suite binary lacking the test convicts naming the file
/// searched; a deleted suite binary convicts naming the missing
/// source; a live generator passes under both the function-name and
/// explicit-name forms.
#[test]
fn suite_snapshots_resolve_to_their_binary() {
    let tree = FixtureTree::new("suite");
    tree.file(
        "tests/gossip.rs",
        "fn by_function() {}\ninsta::assert_snapshot!(\"by_name\", x);\n",
    )
    .file("tests/snapshots/gossip__by_function.snap", "")
    .file("tests/snapshots/gossip__by_name.snap", "")
    .file("tests/snapshots/gossip__retired.snap", "")
    .file("tests/snapshots/vanished__anything.snap", "")
    // The src side must be walkable even when empty.
    .file("src/lib.rs", "");

    let verdicts = tree.sweep();
    assert_eq!(verdicts.len(), 4, "every file is judged: {verdicts:?}");
    assert!(
        verdicts[0].1.is_ok() && verdicts[1].1.is_ok(),
        "both generator forms are live: {verdicts:?}"
    );
    let retired = verdicts[2].1.as_ref().expect_err("a renamed test convicts");
    assert!(
        retired.contains("tests/gossip.rs") || retired.contains("tests\\gossip.rs"),
        "the conviction names the file searched: {retired}"
    );
    let vanished = verdicts[3]
        .1
        .as_ref()
        .expect_err("a deleted suite convicts");
    assert!(
        vanished.contains("does not exist"),
        "the conviction names the missing source: {vanished}"
    );
}

/// A src-side snapshot is convicted when its module file lacks the
/// generating test, and a committed `.snap.new` is convicted outright
/// as unaccepted insta output.
#[test]
fn module_snapshots_resolve_through_the_module_path() {
    let tree = FixtureTree::new("module");
    tree.file("tests/gossip.rs", "fn live() {}\n")
        .file("tests/snapshots/gossip__live.snap", "")
        .file("src/foo/tests.rs", "fn present() {}\n")
        .file("src/foo/snapshots/rumors__foo__tests__present.snap", "")
        .file("src/foo/snapshots/rumors__foo__tests__gone.snap", "")
        .file("src/foo/snapshots/rumors__foo__tests__pending.snap.new", "");

    let verdicts = tree.sweep();
    assert_eq!(verdicts.len(), 4, "every file is judged: {verdicts:?}");
    let by_path: std::collections::BTreeMap<&str, &Result<(), String>> = verdicts
        .iter()
        .map(|(rel, verdict)| (rel.as_str(), verdict))
        .collect();
    assert!(
        by_path["src/foo/snapshots/rumors__foo__tests__present.snap"].is_ok(),
        "the live snapshot stays: {verdicts:?}"
    );
    let gone = by_path["src/foo/snapshots/rumors__foo__tests__gone.snap"]
        .as_ref()
        .expect_err("a retired test convicts");
    assert!(
        gone.contains("tests.rs") && gone.contains("gone"),
        "the conviction names the file searched and the name missed: {gone}"
    );
    let pending = by_path["src/foo/snapshots/rumors__foo__tests__pending.snap.new"]
        .as_ref()
        .expect_err("pending insta output convicts");
    assert!(
        pending.contains("unaccepted snapshot"),
        "named as such: {pending}"
    );
}
