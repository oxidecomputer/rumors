//! Records the facts about this build that a results directory names: the
//! source revision, the compiler, and the target.
//!
//! The revision is for the people who read results. What identifies a
//! build, for resuming a run and for the calibration cache, is a digest of
//! the running executable, which the tool computes when it runs: two builds
//! of one revision can differ, and the revision cannot say so.
//!
//! The revision comes from git through vergen: the checked-out commit,
//! with `-dirty` when a tracked file differs from it. A tree with no git
//! repository, such as one copied to a measurement host, names its commit
//! in a `COMMIT` file at the workspace's root instead, and the revision
//! records that it came from there, since nothing then says whether the
//! tree differs from the commit.
//!
//! The script reruns when the revision could change: vergen watches git's
//! `HEAD` and the branch it names, so a new commit reruns it, and the script
//! watches every tracked file, so an edit anywhere in the repository does,
//! the library's included. Without git it watches the `COMMIT` file, even
//! while the file is absent, which makes cargo rerun it on every build
//! until one exists.

use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use vergen_gitcl::{Emitter, Gitcl};

/// The hexadecimal digits of a commit ID the revision keeps, as
/// `git rev-parse --short=12` would.
const REVISION_DIGITS: usize = 12;

/// The file that names the commit of a tree with no git repository, at the
/// workspace's root.
const COMMIT_FILE: &str = "COMMIT";

fn main() -> Result<(), Box<dyn Error>> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let revision = match git_revision(&manifest_dir)? {
        Some(revision) => revision,
        None => commit_file_revision(&manifest_dir),
    };
    let rustc = rustc_version::version_meta()
        .map(|meta| meta.short_version_string)
        .unwrap_or_else(|_| "unknown".to_string());
    let target = env::var("TARGET").unwrap_or_else(|_| "unknown".to_string());
    println!("cargo::rustc-env=NETPERF_REVISION={revision}");
    println!("cargo::rustc-env=NETPERF_RUSTC={rustc}");
    println!("cargo::rustc-env=NETPERF_TARGET={target}");
    Ok(())
}

/// The revision from git, emitting what makes cargo rerun this script when
/// it changes, or `None` when the crate is not in a git work tree or git
/// cannot run.
///
/// # Errors
///
/// If vergen read the repository but could not write its instructions.
fn git_revision(manifest_dir: &Path) -> Result<Option<String>, Box<dyn Error>> {
    let gitcl = Gitcl::builder().sha(false).dirty(false).build();
    let mut emitter = Emitter::default();
    // With `fail_on_error`, vergen returns an error without emitting
    // anything when git cannot answer, rather than placeholder values, so
    // the `COMMIT` file can stand in.
    let Ok(emitter) = emitter.fail_on_error().add_instructions(&gitcl) else {
        return Ok(None);
    };
    let value = |name: &str| {
        emitter
            .cargo_rustc_env_map()
            .iter()
            .find(|(key, _)| key.name() == name)
            .map(|(_, value)| value.clone())
    };
    let (Some(sha), Some(dirty)) = (value("VERGEN_GIT_SHA"), value("VERGEN_GIT_DIRTY")) else {
        return Ok(None);
    };
    emitter.emit()?;
    watch_tracked_files(manifest_dir);
    let commit: String = sha.chars().take(REVISION_DIGITS).collect();
    Ok(Some(if dirty == "true" {
        format!("{commit}-dirty")
    } else {
        commit
    }))
}

/// Makes cargo rerun this script when any file git tracks changes, so the
/// dirty mark follows edits outside this crate.
///
/// A path that holds a newline cannot appear in a cargo instruction, so its
/// directory is watched in its place.
fn watch_tracked_files(manifest_dir: &Path) {
    let git = |dir: &Path, args: &[&str]| {
        Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| output.stdout)
    };
    let Some(top) = git(manifest_dir, &["rev-parse", "--show-toplevel"]) else {
        return;
    };
    let top = PathBuf::from(String::from_utf8_lossy(&top).trim_end_matches('\n'));
    // `ls-files` lists only what lies below the directory it runs in, so it
    // runs at the repository's top.
    let Some(listing) = git(&top, &["ls-files", "-z"]) else {
        return;
    };
    for name in listing
        .split(|&byte| byte == 0)
        .filter(|name| !name.is_empty())
    {
        let path = top.join(String::from_utf8_lossy(name).as_ref());
        let watched = match path.to_str() {
            Some(text) if !text.contains('\n') => path.clone(),
            _ => path.parent().map_or_else(|| top.clone(), Path::to_path_buf),
        };
        println!("cargo::rerun-if-changed={}", watched.display());
    }
}

/// The revision a tree with no git repository records in its `COMMIT`
/// file, at the workspace's root: the nearest ancestor of this crate that
/// holds `Cargo.lock`.
///
/// The file's first line is the commit ID in hexadecimal. A file that is
/// absent, unreadable, or holds anything else leaves the revision
/// `unknown`, with a cargo warning saying why.
fn commit_file_revision(manifest_dir: &Path) -> String {
    let Some(root) = manifest_dir
        .ancestors()
        .find(|dir| dir.join("Cargo.lock").is_file())
    else {
        println!("cargo::warning=no Cargo.lock above this crate, so no {COMMIT_FILE} file to read");
        return "unknown".to_string();
    };
    let path = root.join(COMMIT_FILE);
    println!("cargo::rerun-if-changed={}", path.display());
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) => {
            println!(
                "cargo::warning=no git repository, and {} cannot be read ({error}); the revision is \
                 unknown, so write the commit ID there",
                path.display()
            );
            return "unknown".to_string();
        }
    };
    let commit = text.lines().next().unwrap_or("").trim();
    if commit.is_empty() || !commit.chars().all(|c| c.is_ascii_hexdigit()) {
        println!(
            "cargo::warning={}'s first line is not a commit ID in hexadecimal; the revision is unknown",
            path.display()
        );
        return "unknown".to_string();
    }
    let commit: String = commit
        .chars()
        .take(REVISION_DIGITS)
        .map(|c| c.to_ascii_lowercase())
        .collect();
    format!("{commit} ({COMMIT_FILE} file)")
}
