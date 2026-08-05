//! The adequacy-kernel roster: every committed-failing
//! `_reads_superlinear` tripwire is pinned here by name, so deleting or
//! renaming one is a reviewable diff, never a silent decoration of the
//! band it keeps honest.
//!
//! Each flatness band's adequacy rests on a committed kernel that
//! demonstrates the refuted mechanism (absolute-position accounting, a
//! schoolbook settle, a sequential reduce, ...) still reads red through
//! the band's own meters — instruments-before-cures, held forever. But
//! a kernel binds nowhere mechanically unless something names it: the
//! class contracts name only their own witnesses, and the band parity
//! scanner matches only band names, so an unrostered kernel could be
//! deleted with every gate leg green while its band stayed green as
//! decoration. This roster is the missing jaw: the committed list below
//! must match the `#[test]` fns whose names carry `_reads_superlinear`,
//! in both directions.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Every `_reads_superlinear` adequacy kernel, as
/// `(file relative to the crate root, test fn name)`.
///
/// A new kernel earns a reviewed row here; a deleted or renamed one
/// orphans its row and reads red below — this roster binds the whole
/// genre uniformly.
const TRIPWIRE_ROSTER: &[(&str, &str)] = &[
    (
        "src/version/skyline/query/tests.rs",
        "absolute_position_accounting_reads_superlinear_on_freeze_position",
    ),
    (
        "src/version/skyline/query/tests.rs",
        "per_digit_window_absorb_reads_superlinear_on_dense_suffix",
    ),
    (
        "src/version/skyline/query/tests.rs",
        "schoolbook_settle_reads_superlinear_on_plateau_puncture",
    ),
    (
        "src/version/skyline/query/tests.rs",
        "schoolbook_settle_reads_superlinear_on_wide_arming",
    ),
    (
        "src/version/skyline/query/tests.rs",
        "span_promotion_accounting_reads_superlinear_on_rearm_pair",
    ),
    (
        "src/version/skyline/query/tests.rs",
        "span_promotion_accounting_reads_superlinear_on_rearm_spine",
    ),
    (
        "src/version/skyline/query/tests.rs",
        "suffix_walk_settle_reads_superlinear_on_dense_suffix",
    ),
    (
        "src/version/skyline/query/tests.rs",
        "suffix_walk_settle_reads_superlinear_on_dense_suffix_pair",
    ),
    (
        "src/version/skyline/text/tests.rs",
        "schoolbook_parse_reads_superlinear_on_wide_arming",
    ),
    (
        "tests/meter.rs",
        "sequential_meet_reduce_reads_superlinear_on_shade",
    ),
];

/// Collect every `fn` whose name carries `_reads_superlinear` under
/// `dir`, keyed by the path relative to the crate root.
fn scan(dir: &Path, root: &Path, found: &mut BTreeSet<(String, String)>) {
    for entry in std::fs::read_dir(dir).expect("source directory is readable") {
        let path = entry.expect("directory entry is readable").path();
        if path.is_dir() {
            scan(&path, root, found);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let text = std::fs::read_to_string(&path).expect("source file is readable");
            for line in text.lines() {
                let Some(rest) = line.trim_start().strip_prefix("fn ") else {
                    continue;
                };
                let name: String = rest
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if name.contains("_reads_superlinear") {
                    let rel = path
                        .strip_prefix(root)
                        .expect("scanned file lives under the crate root")
                        .to_string_lossy()
                        .into_owned();
                    found.insert((rel, name));
                }
            }
        }
    }
}

/// The committed-failing adequacy kernels match the roster exactly, in
/// both directions.
///
/// A deleted or renamed kernel orphans its roster row (its band is
/// decoration until re-bound); a new kernel reads red until it gains a
/// reviewed row. The convention is part of the contract: adequacy
/// kernels are named `..._reads_superlinear_on_...`, so the scan sees
/// them.
#[test]
fn superlinear_tripwires_match_the_committed_roster() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut found = BTreeSet::new();
    scan(&root.join("src"), &root, &mut found);
    scan(&root.join("tests"), &root, &mut found);
    let expected: BTreeSet<(String, String)> = TRIPWIRE_ROSTER
        .iter()
        .map(|&(file, name)| (file.to_owned(), name.to_owned()))
        .collect();
    assert_eq!(
        found, expected,
        "the _reads_superlinear kernel set drifted from the committed \
         roster: an adequacy kernel that binds nowhere is silently \
         deletable, and its flatness band green as decoration"
    );
}
