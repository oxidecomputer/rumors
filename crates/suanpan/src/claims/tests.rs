//! The complexity-claims binding tests.
//!
//! They hold the roster total over the public surface, every site's
//! `# Complexity` section ending with its bound's rendered line, the
//! crate page's operations table byte-equal to the roster's cost cells,
//! and every cited witness alive as a `#[test]` in its file. The roster
//! and the table scanner live in the parent module; the shared scanners
//! come from the `complexity-claims` crate.

use std::collections::BTreeSet;
use std::path::PathBuf;

use complexity_claims::{doc_index, extract_public_fns, test_fns, Bound};

use super::{cost_table, Claim, Evidence, CLAIMS, FAMILY_SURFACE, SOURCES};

/// The crate root at test time.
fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The claims roster is total over the public surface, exactly.
///
/// Every mechanically extracted `pub fn` and every family row has one
/// claim, and nothing else does: a new public operation fails here
/// until its documented cost is pinned, and a removed one orphans its
/// claim. The extractor-liveness probe guards the premise — an
/// extractor that silently stopped seeing the file would otherwise
/// drain both sides at once.
#[test]
fn claims_are_total_over_the_public_surface() {
    let mut surface = extract_public_fns(&crate_root(), SOURCES);
    assert!(
        surface.contains("Accumulator::sign") && surface.contains("touch_meter::touches"),
        "extractor liveness: the known surface must be seen"
    );
    surface.extend(FAMILY_SURFACE.iter().map(|op| (*op).to_owned()));
    let mut claimed = BTreeSet::new();
    for claim in CLAIMS {
        assert!(
            claimed.insert(claim.op.to_owned()),
            "duplicate claim row: {}",
            claim.op
        );
    }
    let unclaimed: Vec<_> = surface.difference(&claimed).collect();
    let orphaned: Vec<_> = claimed.difference(&surface).collect();
    assert!(
        unclaimed.is_empty() && orphaned.is_empty(),
        "the claims roster and the public surface disagree:\n  \
         public operations with no complexity claim: {unclaimed:?}\n  \
         claims naming no public operation: {orphaned:?}"
    );
}

/// Every claim's `# Complexity` section exists at its recorded site and
/// ends with the roster bound's rendered `**Complexity**:` line, byte
/// for byte — and every Custom bound states a substantial reason.
///
/// A cost edit in the rustdoc that skips this roster (or vice versa) is
/// a named failure, and every site's normative claim is the roster's
/// own rendering, never hand-drifted prose.
#[test]
fn complexity_sections_end_with_their_rendered_lines() {
    let index = doc_index(&crate_root(), SOURCES);
    let mut errors = Vec::new();
    for claim in CLAIMS {
        for check in claim.checks {
            if let Bound::Custom { reason, .. } = check.bound {
                if reason.trim().len() < 20 {
                    errors.push(format!(
                        "{}: a Custom bound must state a substantial reason",
                        claim.op
                    ));
                }
            }
            match index.section(claim.op, check.site) {
                Err(err) => errors.push(err),
                Ok(section) => {
                    let want = check.bound.render();
                    let got = section.lines().rev().find(|l| !l.trim().is_empty());
                    if got != Some(want.as_str()) {
                        errors.push(format!(
                            "{}: the `# Complexity` section at {:?} does not end with the \
                             rendered bound\n    want: {want}\n    got:  {}",
                            claim.op,
                            check.site,
                            got.unwrap_or("<empty section>")
                        ));
                    }
                }
            }
        }
    }
    assert!(
        errors.is_empty(),
        "rustdoc complexity sections drifted from the claims roster:\n  {}",
        errors.join("\n  ")
    );
}

/// The crate page's operations table binds to the roster, both ways:
/// every claim with a table row finds exactly one row naming it whose
/// cost cell byte-equals the claim's, and every table row is named by
/// at least one claim.
///
/// The table is the crate's most-read cost surface and was twice found
/// wrong in review; this binding makes editing it without the roster
/// (or vice versa) a named failure.
#[test]
fn cost_table_rows_bind_to_the_roster() {
    let rows = cost_table();
    let mut matched = vec![false; rows.len()];
    let mut errors = Vec::new();
    for claim in CLAIMS {
        let Some(want) = claim.table_cost else {
            continue;
        };
        // The op's link target names it uniquely inside an ops cell; the
        // closing parenthesis keeps `sign` from matching `sign_dominates_*`.
        let short = claim.op.rsplit("::").next().expect("ops are pathed");
        let locator = format!("](Accumulator::{short})");
        let hits: Vec<usize> = rows
            .iter()
            .enumerate()
            .filter(|(_, (ops, _))| ops.contains(&locator))
            .map(|(i, _)| i)
            .collect();
        match hits.as_slice() {
            [i] => {
                matched[*i] = true;
                if rows[*i].1 != want {
                    errors.push(format!(
                        "{}: the table row's cost cell drifted from the roster\n    \
                         want: {want}\n    got:  {}",
                        claim.op, rows[*i].1
                    ));
                }
            }
            [] => errors.push(format!(
                "{}: table_cost is pinned but no table row links the operation",
                claim.op
            )),
            _ => errors.push(format!(
                "{}: {} table rows link the operation; the locator must be unique",
                claim.op,
                hits.len()
            )),
        }
    }
    for (i, hit) in matched.iter().enumerate() {
        if !hit {
            errors.push(format!(
                "table row with no claim naming it (add table_cost to its claims, or \
                 retire the row): {:?}",
                rows[i].0
            ));
        }
    }
    assert!(
        errors.is_empty(),
        "the crate page's operations table and the claims roster disagree:\n  {}",
        errors.join("\n  ")
    );
}

/// Every witness a claim cites exists as a `#[test]`-attributed
/// function in its file, and every exclusion states a mechanism.
///
/// A renamed or deleted instrument orphans the claims that leaned on it
/// by name — including the `accum_streams` digit-touch bands committed
/// beside the consumer in before's meter suite.
#[test]
fn cited_witnesses_exist() {
    let mut errors = Vec::new();
    for claim in CLAIMS {
        match &claim.evidence {
            Evidence::Witnessed(pairs) => {
                assert!(
                    !pairs.is_empty(),
                    "{}: a witnessed claim must cite at least one test",
                    claim.op
                );
                for (file, witness) in *pairs {
                    let path = crate_root().join(file);
                    let text = std::fs::read_to_string(&path)
                        .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
                    let fns = test_fns(&text);
                    assert!(
                        !fns.is_empty(),
                        "the witness scanner found no #[test] fns in {file}: the scan is \
                         broken, not the witnesses"
                    );
                    if !fns.contains(*witness) {
                        errors.push(format!(
                            "{}: {file} no longer holds the #[test] fn `{witness}` — \
                             re-derive the claim with the change that moved it",
                            claim.op
                        ));
                    }
                }
            }
            Evidence::Excluded(reason) => {
                assert!(
                    reason.trim().len() >= 20,
                    "{}: an exclusion reason must state a mechanism, not a shrug",
                    claim.op
                );
            }
        }
    }
    assert!(
        errors.is_empty(),
        "claims cite witnesses that do not exist:\n  {}",
        errors.join("\n  ")
    );
}

/// A `Claim` is inspectable in failure messages (`Site` derives
/// `Debug`); keep the type checked so the roster stays printable.
#[test]
fn claim_rows_are_printable() {
    let row: &Claim = &CLAIMS[1];
    assert!(!format!("{:?}", row.checks[0].site).is_empty());
}
