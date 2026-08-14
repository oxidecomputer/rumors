//! The consumption-roster binding tests.
//!
//! They hold every roster row fully stated, the row ↔ witness binding
//! total in both directions over the declared-test scan the coverage
//! roster itself runs on, and every consumption site anchored to live
//! code with identifier-boundary, comment-stripped matching. The
//! roster lives in the parent module; the anchor checker's own red
//! fixtures are committed beside the bindings.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use crate::testing::surface_coverage::declared_test_names_by_file;

use super::{ROSTER, WITNESSES};

/// The crate root at test time.
fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Read a crate-relative source file whole.
fn read_source(file: &str) -> String {
    let path = crate_root().join(file);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()))
}

/// Whether an anchor occurrence at `at` in `text` sits on identifier
/// boundaries.
///
/// Neither the character before the match nor the one after it may
/// continue an identifier, whenever the anchor's own edge is an
/// identifier character.
fn on_identifier_boundary(text: &str, at: usize, anchor: &str) -> bool {
    let ident = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let before_clear =
        !anchor.starts_with(ident) || !text[..at].chars().next_back().is_some_and(ident);
    let after_clear =
        !anchor.ends_with(ident) || !text[at + anchor.len()..].chars().next().is_some_and(ident);
    before_clear && after_clear
}

/// Whether `text` contains `anchor` outside comments, on identifier
/// boundaries: `SEAM_CLEARANCE` does not survive on
/// `SEAM_CLEARANCE_WIDE`, and a prose mention in a comment does not
/// keep a row green.
///
/// The comment strip is lexical — a line's first `//` starts its
/// comment (covering `//`, `///`, and `//!` alike) — exactly as strong
/// as the anchored files' rustfmt-normalized shape, which carries no
/// block comments and no `//` inside string literals on anchor lines.
fn anchor_resolves(text: &str, anchor: &str) -> bool {
    for line in text.lines() {
        let code = match line.find("//") {
            Some(comment) => &line[..comment],
            None => line,
        };
        let mut from = 0;
        while let Some(pos) = code[from..].find(anchor) {
            let at = from + pos;
            if on_identifier_boundary(code, at, anchor) {
                return true;
            }
            from = at + 1;
        }
    }
    false
}

/// Red fixture: the anchor checker refuses a longer identifier
/// containing the anchor.
///
/// `SEAM_CLEARANCE` must not resolve on a source whose only
/// occurrence is `SEAM_CLEARANCE_WIDE`: the drift shape that would
/// keep a roster row green after the real constant died.
#[test]
fn anchor_checker_refuses_identifier_extensions() {
    assert!(
        !anchor_resolves("const SEAM_CLEARANCE_WIDE: usize = 10;", "SEAM_CLEARANCE"),
        "an identifier extension must not satisfy the anchor"
    );
    assert!(
        !anchor_resolves("const WIDE_SEAM_CLEARANCE: usize = 10;", "SEAM_CLEARANCE"),
        "an identifier prefix must not satisfy the anchor"
    );
    assert!(
        anchor_resolves("const SEAM_CLEARANCE: usize = 5;", "SEAM_CLEARANCE"),
        "the exact identifier still resolves"
    );
}

/// Red fixture: the anchor checker refuses comment-only mentions, so
/// prose citing an anchor cannot keep a row green after the code it
/// names dies.
#[test]
fn anchor_checker_refuses_comment_mentions() {
    assert!(
        !anchor_resolves("// SEAM_CLEARANCE: the clearance line", "SEAM_CLEARANCE"),
        "a line-comment mention must not satisfy the anchor"
    );
    assert!(
        !anchor_resolves("/// prose about fn settle( here", "fn settle("),
        "a doc-comment mention must not satisfy the anchor"
    );
    assert!(
        anchor_resolves("pub(super) fn settle(&mut self) {", "fn settle("),
        "the declaration itself still resolves"
    );
    assert!(
        !anchor_resolves("pub(super) fn settle_segment(&mut self) {", "fn settle("),
        "a longer fn name must not satisfy a parenthesized anchor"
    );
}

/// Every roster row is fully stated: a property, at least one
/// consumption site, at least one witness, and no two rows share a
/// property.
#[test]
fn roster_rows_are_fully_stated() {
    let mut properties = BTreeSet::new();
    for row in ROSTER {
        assert!(
            !row.property.trim().is_empty(),
            "a roster row states its property"
        );
        assert!(
            properties.insert(row.property),
            "duplicate roster row: {:?}",
            row.property
        );
        assert!(
            !row.sites.is_empty(),
            "a consumed property names where it is consumed: {:?}",
            row.property
        );
        assert!(
            !row.witnesses.is_empty(),
            "a consumed property cites at least one witness: {:?}",
            row.property
        );
    }
    assert!(!ROSTER.is_empty(), "the roster has rows");
}

/// Row ↔ witness binding, both directions: every cited witness is a
/// `#[test]` declared in the witness file of record, and every
/// `#[test]` declared there is cited by some row.
///
/// The declared-name scan is the runner-faithful one behind the
/// coverage roster ([`declared_test_names_by_file`]), which tolerates
/// attributes and comments between `#[test]` and its `fn` and carries
/// its own adequacy test. The unconditional non-empty floor makes an
/// empty scan a failure in both directions, never a green.
#[test]
fn witnesses_bind_to_the_roster_both_ways() {
    let declared: BTreeSet<String> = declared_test_names_by_file()
        .into_iter()
        .filter(|(_, files)| files.contains(WITNESSES))
        .map(|(name, _)| name)
        .collect();
    assert!(
        !declared.is_empty(),
        "the declared-test scan found no #[test] fns in {WITNESSES}: the scan or the \
         file is broken, not the binding"
    );
    let cited: BTreeSet<String> = ROSTER
        .iter()
        .flat_map(|row| row.witnesses.iter().map(|witness| (*witness).to_owned()))
        .collect();
    let missing: Vec<&String> = cited.difference(&declared).collect();
    let uncited: Vec<&String> = declared.difference(&cited).collect();
    assert!(
        missing.is_empty() && uncited.is_empty(),
        "the roster and the witness file disagree:\n  \
         cited but not declared in {WITNESSES}: {missing:?}\n  \
         declared but cited by no row: {uncited:?}"
    );
}

/// Every consumption site anchors to live code: the named file exists
/// and still contains its anchor at an identifier boundary outside
/// comments.
///
/// A refactor that moves or renames the consuming code therefore
/// turns the row red instead of orphaning its prose.
#[test]
fn consumption_sites_are_anchored_to_live_code() {
    let mut sources: BTreeMap<&str, String> = BTreeMap::new();
    let mut errors = Vec::new();
    for row in ROSTER {
        for &(file, anchor) in row.sites {
            let text = sources.entry(file).or_insert_with(|| read_source(file));
            if !anchor_resolves(text, anchor) {
                errors.push(format!(
                    "{file} no longer contains the anchor {anchor:?} for: {}",
                    row.property
                ));
            }
        }
    }
    assert!(
        errors.is_empty(),
        "roster rows anchor to code that moved:\n  {}",
        errors.join("\n  ")
    );
}
