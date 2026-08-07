//! The claims-roster binding tests.
//!
//! They hold the roster total over the public surface, the crate page's
//! operations table byte-equal to the roster's cost cells, every cited
//! witness alive as a `#[test]` in its file, and every
//! (operation, witness) edge *reaching*: the witness's body (or a helper
//! it calls in the same file) invokes the operation it evidences, or the
//! edge carries a stated-mechanism exemption in [`REACH_EXEMPT`]. The
//! roster and the table scanner live in the parent module; the shared
//! scanners come from the `surface-scan` crate.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use surface_scan::{extract_public_fns, test_fns};

use super::{cost_table, Evidence, CLAIMS, FAMILY_SURFACE, SOURCES};

/// The (operation, witness) edges whose evidence flows through a stated
/// mechanism instead of a direct invocation, mirroring the
/// [`Evidence::Excluded`] reason discipline.
///
/// Each entry is `(op, witness fn, mechanism)`, the mechanism
/// substantial and checked so, and the exemption is held *load-bearing*
/// — a witness that starts invoking the operation orphans its entry
/// here, so the list can never silently outlive the delegation it
/// describes.
const REACH_EXEMPT: &[(&str, &str, &str)] = &[
    (
        "Accumulator::is_negative",
        "no_collapse_fold_re_scans_the_prefix",
        "is_negative delegates to sign() in one line (a comparison on its result), so \
         its cost evidence is sign()'s own: the witness drives sign() directly",
    ),
    (
        "Accumulator::is_negative",
        "accum_static_prefix_touches_flat",
        "is_negative delegates to sign() in one line (a comparison on its result), so \
         its cost evidence is sign()'s own: the witness drives sign() directly",
    ),
    (
        "Accumulator::add_magnitude_shl",
        "magnitude_dispatch_costs_its_width_path",
        "the witness evidences the width-dispatch mechanism the shifted and unshifted \
         magnitude entries share, through the unshifted entries; the shift axis is \
         priced by the co-cited witness, which drives add_magnitude_shl itself",
    ),
    (
        "Accumulator::sub_magnitude_shl",
        "magnitude_dispatch_costs_its_width_path",
        "the witness evidences the width-dispatch mechanism the shifted and unshifted \
         magnitude entries share, through the unshifted entries; the shift axis is \
         priced by the co-cited witness, which drives sub_magnitude_shl itself",
    ),
];

/// The crate root at test time.
fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Every `fn` in a source file, mapped to its body text (the def line
/// through its balancing close brace), nested fns included as their own
/// entries.
///
/// A line scan over rustfmt-normalized shape, like the shared scanners:
/// a definition is a line whose trimmed form starts with `fn ` (or
/// `pub fn `), and its body ends where the braces opened since the def
/// line balance. Brace counting is textual — the witness files carry no
/// unbalanced brace in any literal or comment (format placeholders pair)
/// — and an unclosed body panics rather than silently truncating the
/// reach analysis.
fn fn_bodies(source: &str) -> BTreeMap<String, String> {
    let lines: Vec<&str> = source.lines().collect();
    let mut bodies = BTreeMap::new();
    for (def_line, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed
            .strip_prefix("fn ")
            .or_else(|| trimmed.strip_prefix("pub fn "))
        else {
            continue;
        };
        let name = surface_scan::fn_name(rest);
        let mut depth = 0i64;
        let mut opened = false;
        let mut end = None;
        for (scan_line, body_line) in lines.iter().enumerate().skip(def_line) {
            for character in body_line.chars() {
                match character {
                    '{' => {
                        depth += 1;
                        opened = true;
                    }
                    '}' => depth -= 1,
                    _ => {}
                }
            }
            if opened && depth <= 0 {
                end = Some(scan_line);
                break;
            }
        }
        let end = end.unwrap_or_else(|| panic!("fn {name}: body never closes its braces"));
        bodies.insert(name.to_owned(), lines[def_line..=end].join("\n"));
    }
    bodies
}

/// Whether `body` invokes the method `name` — a `.name(` call, the only
/// shape a receiver method takes.
///
/// The leading dot bounds the token on the left and the parenthesis on
/// the right, so `.shl(` never matches `.add_wide_shl(` and a prose
/// mention never counts as an invocation.
fn invokes_method(body: &str, name: &str) -> bool {
    body.contains(&format!(".{name}("))
}

/// Whether `body` calls the free function `name` (a `name(` occurrence
/// whose preceding character is not part of an identifier and not a
/// field access), the shape the witness files' helpers are called in.
fn calls_helper(body: &str, name: &str) -> bool {
    let needle = format!("{name}(");
    let mut from = 0;
    while let Some(pos) = body[from..].find(&needle) {
        let at = from + pos;
        let before = body[..at].chars().next_back();
        if !before.is_some_and(|character| {
            character.is_alphanumeric() || character == '_' || character == '.'
        }) {
            return true;
        }
        from = at + needle.len();
    }
    false
}

/// Whether `witness`'s body — or, transitively, any same-file helper it
/// calls — invokes the method `op_method`.
fn reaches(bodies: &BTreeMap<String, String>, witness: &str, op_method: &str) -> bool {
    let mut queue = vec![witness.to_owned()];
    let mut visited = BTreeSet::new();
    while let Some(name) = queue.pop() {
        if !visited.insert(name.clone()) {
            continue;
        }
        let Some(body) = bodies.get(&name) else {
            continue;
        };
        if invokes_method(body, op_method) {
            return true;
        }
        for helper in bodies.keys() {
            if !visited.contains(helper) && calls_helper(body, helper) {
                queue.push(helper.clone());
            }
        }
    }
    false
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
                        .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
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

/// Every (operation, witness) edge reaches: the witness's body — or a
/// helper it calls in the same file — invokes the operation it
/// evidences, unless the edge carries a stated-mechanism exemption in
/// [`REACH_EXEMPT`].
///
/// Every exemption must in turn be well-formed: it names a real edge,
/// states a substantial mechanism, and is load-bearing — the witness
/// genuinely does not invoke the operation.
///
/// Existence alone (`cited_witnesses_exist`) cannot see a witness
/// hollowed out from the inside: delete the one leg that drives the
/// operation and the fn still exists under its cited name while the
/// claim's evidence is gone. Reach closes that path — the edge fails
/// by name until the invocation returns or the delegation earns a
/// stated exemption. The invocation scan is syntactic (`.method(`
/// receiver calls, helper calls followed into the same file), which is
/// exactly as strong as the witness files' rustfmt-normalized shape;
/// a prose mention never counts.
#[test]
fn cited_witnesses_reach_their_operations() {
    // One body map per witness file, read once.
    let mut bodies_by_file: BTreeMap<&str, BTreeMap<String, String>> = BTreeMap::new();
    let mut errors = Vec::new();
    let mut edges = BTreeSet::new();
    for claim in CLAIMS {
        let Evidence::Witnessed(pairs) = &claim.evidence else {
            continue;
        };
        let Some(method) = claim.op.rsplit("::").next() else {
            continue;
        };
        for (file, witness) in *pairs {
            edges.insert((claim.op, *witness));
            let bodies = bodies_by_file.entry(file).or_insert_with(|| {
                let path = crate_root().join(file);
                let text = std::fs::read_to_string(&path)
                    .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
                fn_bodies(&text)
            });
            let exempt = REACH_EXEMPT
                .iter()
                .find(|(op, cited, _)| op == &claim.op && cited == witness);
            let reached = reaches(bodies, witness, method);
            match (reached, exempt) {
                // The strengthened binding: the witness drives the op.
                (true, None) => {}
                // A stated-mechanism delegation edge.
                (false, Some(_)) => {}
                (false, None) => errors.push(format!(
                    "{}: `{witness}` ({file}) never invokes `.{method}(` (directly or \
                     through a same-file helper) — the witness no longer evidences the \
                     operation it is cited for; restore the invocation or record the \
                     delegation mechanism in REACH_EXEMPT",
                    claim.op
                )),
                (true, Some(_)) => errors.push(format!(
                    "{}: `{witness}` invokes `.{method}(`, so its REACH_EXEMPT entry is \
                     stale — delete the exemption; the direct binding is stronger",
                    claim.op
                )),
            }
        }
    }
    for (op, witness, mechanism) in REACH_EXEMPT {
        if !edges.contains(&(*op, *witness)) {
            errors.push(format!(
                "REACH_EXEMPT names ({op}, {witness}), which is not a cited \
                 (operation, witness) edge in CLAIMS — delete or re-derive the entry"
            ));
        }
        assert!(
            mechanism.trim().len() >= 20,
            "({op}, {witness}): a reach exemption must state a mechanism, not a shrug"
        );
    }
    assert!(
        errors.is_empty(),
        "claims cite witnesses that do not reach their operations:\n  {}",
        errors.join("\n  ")
    );
}
