//! The consumption-roster binding tests.
//!
//! They hold every roster row fully stated, every cited witness alive
//! as a `#[test]` in its file with the witness file total over the
//! rows (both directions), and every consumption site anchored to live
//! code. The roster itself lives in the parent module; the witness
//! scanner comes from the `surface-scan` crate.

use std::collections::BTreeSet;
use std::path::PathBuf;

use surface_scan::test_fns;

use super::{ROSTER, WITNESSES};

/// A witness name asserted absent from the witness file, then required
/// to fail resolution on every run: proof the scanner-backed binding
/// can reject at all.
const FABRICATED: &str = "consumption_fabricated_witness_tripwire";

/// The crate root at test time.
fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Read a manifest-relative source file whole.
fn read_source(file: &str) -> String {
    let path = crate_root().join(file);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()))
}

/// Every roster row is fully stated: a plain-language property of
/// substance, at least one consumption site, at least one witness, and
/// no two rows share a property.
#[test]
fn roster_rows_are_fully_stated() {
    let mut properties = BTreeSet::new();
    for row in ROSTER {
        assert!(
            row.property.trim().len() >= 20,
            "a roster row must state its property, not a shrug: {:?}",
            row.property
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

/// Row ↔ witness binding, both directions: every cited witness exists
/// as a `#[test]` in its file, and every `#[test]` in the witness file
/// is cited by some row.
///
/// A witness can neither rot away under a live citation nor drift in
/// uncited. The scan floor (a witness file with zero `#[test]` fns is a broken
/// scan, not a green) and the fabricated-name tripwire (a name
/// asserted absent must resolve dead) keep the binding itself honest.
#[test]
fn witnesses_bind_to_the_roster_both_ways() {
    let mut errors = Vec::new();
    let mut cited_in_witness_file = BTreeSet::new();
    // Direction one: every cited witness exists, by name, in its file.
    let mut fns_by_file: std::collections::BTreeMap<&str, BTreeSet<String>> =
        std::collections::BTreeMap::new();
    for row in ROSTER {
        for (file, witness) in row.witnesses {
            let fns = fns_by_file
                .entry(file)
                .or_insert_with(|| test_fns(&read_source(file)));
            assert!(
                !fns.is_empty(),
                "the witness scanner found no #[test] fns in {file}: the scan is broken, \
                 not the witnesses"
            );
            if !fns.contains(*witness) {
                errors.push(format!(
                    "{file} no longer holds the #[test] fn `{witness}` cited by: {}",
                    row.property
                ));
            }
            if *file == WITNESSES {
                cited_in_witness_file.insert((*witness).to_owned());
            }
        }
    }
    // Direction two: the witness file is total over the roster — every
    // `#[test]` it holds is cited by some row.
    let witness_fns = fns_by_file
        .entry(WITNESSES)
        .or_insert_with(|| test_fns(&read_source(WITNESSES)));
    for name in witness_fns.iter() {
        if !cited_in_witness_file.contains(name) {
            errors.push(format!(
                "{WITNESSES} holds the #[test] fn `{name}` no roster row cites: add the \
                 row its evidence belongs to, or retire the test"
            ));
        }
    }
    // Resolver liveness: a fabricated name must come back dead.
    assert!(
        !witness_fns.contains(FABRICATED),
        "the fabricated-name tripwire resolved: the witness scan has gone permissive"
    );
    assert!(
        errors.is_empty(),
        "the roster and the witness file disagree:\n  {}",
        errors.join("\n  ")
    );
}

/// Every consumption site anchors to live code: the named file exists
/// and still contains its anchor token, so a refactor that moves or
/// renames the consuming code turns the row red instead of orphaning
/// its prose.
#[test]
fn consumption_sites_are_anchored_to_live_code() {
    let mut errors = Vec::new();
    for row in ROSTER {
        for (file, anchor) in row.sites {
            assert!(
                anchor.trim().len() >= 4,
                "a site anchor must be a real token, not a shrug: {anchor:?}"
            );
            let text = read_source(file);
            if !text.contains(anchor) {
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
