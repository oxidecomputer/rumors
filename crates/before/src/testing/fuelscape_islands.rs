//! Doc-attachment totality for the fuelscape islands: every measured
//! operation's island reaches the rendered docs, or carries a reviewed
//! exemption.
//!
//! `build.rs` formats one island per committed widget dataset into
//! `$OUT_DIR/fuelscapes/` and writes their names to `index`; the doc
//! comments pull islands in by `include_str!` path. A dangling include
//! is already a compile error, so the direction this suite must hold is
//! the other one: an island nothing includes renders nowhere, silently.
//! Membership is enforced, never remembered — the exemption reasons are
//! the reviewed artifact, in the ops-roster [`EXEMPTIONS`] idiom.
//!
//! Every island site also states its claim as a literal for the readers
//! rustdoc's own pages never reach: the island is attached under
//! `cfg(doc)` and the literal under `cfg(not(doc))`, so a site carrying
//! one without the other leaves a `# Complexity` heading empty for one
//! audience, silently, and a literal that drifts from the dataset states
//! a claim nothing measured. The suite holds the line after every island
//! include to the literal of record (`$OUT_DIR/fuelscape-contracts/`),
//! and every claim-shaped literal anywhere in the sources to some text
//! of record, which covers the macro invocations that pass the literal
//! through a `$contract` parameter.

use std::collections::BTreeSet;
use std::path::Path;

/// Operations whose islands deliberately appear in no doc comment, each
/// with its reviewed reason.
///
/// Currently empty: every measured operation's island reaches the
/// rendered docs (the operator matrices and the conjunction cells carry
/// theirs through their generating macros).
const EXEMPTIONS: &[(&str, &str)] = &[];

/// Every emitted island is included by some doc comment or exempted with
/// a reason, and every exemption names an emitted island (a stale
/// exemption is as dead as a stale island).
#[test]
fn every_island_is_included_or_exempted() {
    let emitted = std::fs::read_to_string(concat!(env!("OUT_DIR"), "/fuelscapes/index"))
        .expect("build.rs writes the island index");
    let emitted: BTreeSet<&str> = emitted.lines().collect();
    assert!(!emitted.is_empty(), "the island index is never empty");

    // Every `/fuelscapes/<op>.html` occurrence in the crate's sources, and
    // every site whose next line is not the claim literal of record.
    let contracts = Path::new(concat!(env!("OUT_DIR"), "/fuelscape-contracts"));
    let of_record = texts_of_record(contracts);
    let mut included = BTreeSet::new();
    let mut faults = Vec::new();
    let mut stack = vec![Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src")).to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("source directories are readable") {
            let path = entry.expect("source entries are readable").path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|x| x == "rs") {
                let source = std::fs::read_to_string(&path).expect("source files are readable");
                // An include site reads `/fuelscapes/<op>.html`; the
                // charset filter drops this suite's own prose and code,
                // which mention the directory without naming an island.
                included.extend(include_sites(&source, "/fuelscapes/", ".html"));
                let lines: Vec<&str> = source.lines().collect();
                for (i, line) in lines.iter().enumerate() {
                    if let Some(op) = island_include(line) {
                        // The crate root's open variant pairs with its op's text.
                        let op = op.strip_suffix(".open").unwrap_or(op);
                        let want = if op == "\", $island, \"" {
                            // A macro body: the literal arrives as a parameter.
                            "$contract".to_string()
                        } else if island_shaped(op) {
                            format!("\"{}\"", text_of_record(contracts, op))
                        } else {
                            // Prose naming the directory, this suite's own included.
                            continue;
                        };
                        let found = not_doc_literal(&lines[i + 1..]);
                        if found.as_deref() != Some(want.as_str()) {
                            faults.push(format!(
                                "{}:{}: the attribute after the island include must be \
                                 `#[cfg_attr(not(doc), doc = {want})]`, found {found:?}",
                                path.display(),
                                i + 2,
                            ));
                        }
                    }
                    // Any claim-shaped literal, a `$contract` argument included,
                    // must be some text of record.
                    if let Some(claim) = claim_literal(line) {
                        if !of_record.contains(claim) {
                            faults.push(format!(
                                "{}:{}: the claim literal `{claim}` is no measured operation's text of record",
                                path.display(),
                                i + 1
                            ));
                        }
                    }
                }
            }
        }
    }

    for fault in &faults {
        eprintln!("{fault}");
    }
    assert!(
        faults.is_empty(),
        "island sites and claim literals disagree with the texts of record (listed above)"
    );

    for (op, _reason) in EXEMPTIONS {
        assert!(
            emitted.contains(op),
            "exemption {op:?} names no emitted island: retire the exemption"
        );
        assert!(
            !included.contains(*op),
            "{op} is both included and exempted: retire the exemption"
        );
    }
    let exempt: BTreeSet<&str> = EXEMPTIONS.iter().map(|(op, _)| *op).collect();
    for op in &emitted {
        assert!(
            included.contains(*op) || exempt.contains(op),
            "island {op} is emitted but no doc comment includes it and no \
             exemption covers it: attach it at its operation's doc site, or \
             add a reviewed exemption"
        );
    }
    for op in &included {
        assert!(
            emitted.contains(op.as_str()),
            "a doc comment includes island {op}, which build.rs does not emit"
        );
    }
}

/// The island-shaped names that `source` includes from `dir`, each read
/// between the directory and `ext`.
fn include_sites(source: &str, dir: &str, ext: &str) -> BTreeSet<String> {
    let mut ops = BTreeSet::new();
    for site in source.split(dir).skip(1) {
        let Some((op, _)) = site.split_once(ext) else {
            continue;
        };
        if island_shaped(op) {
            ops.insert(op.to_string());
        }
    }
    ops
}

/// Whether `op` is spelled as an island name: lowercase, digits, and
/// underscores, non-empty.
fn island_shaped(op: &str) -> bool {
    !op.is_empty()
        && op
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// The `doc` value of a `#[cfg_attr(not(doc), doc = ...)]` attribute
/// opening at the first of `lines`, however rustfmt wrapped it, or None
/// when no such attribute opens there.
fn not_doc_literal(lines: &[&str]) -> Option<String> {
    let mut block = String::new();
    for line in lines.iter().take(8) {
        block.push_str(line.trim());
        if block.ends_with(")]") {
            break;
        }
    }
    let inner = block
        .strip_prefix("#[cfg_attr(")
        .or_else(|| block.strip_prefix("#![cfg_attr("))?
        .strip_suffix(")]")?;
    let (cfg, value) = inner.split_once("doc = ")?;
    (cfg.trim().trim_end_matches(',') == "not(doc)").then(|| value.trim().to_string())
}

/// The `<op>` an island include line names, or None for a line that
/// includes no island.
fn island_include(line: &str) -> Option<&str> {
    let (_, rest) = line.split_once("/fuelscapes/")?;
    let (op, _) = rest.split_once(".html")?;
    Some(op)
}

/// A claim-shaped string literal on `line`: one opening with a backticked
/// big-O, the form every text of record takes.
fn claim_literal(line: &str) -> Option<&str> {
    // Assembled, so this source never carries the shape it scans for.
    let opening = ['"', '`', 'O', '('].iter().collect::<String>();
    let start = line.find(&opening)? + 1;
    let end = start + line[start..].find('"')?;
    Some(&line[start..end])
}

/// The literal of record for `op`, as build.rs formatted it.
fn text_of_record(contracts: &Path, op: &str) -> String {
    std::fs::read_to_string(contracts.join(format!("{op}.md")))
        .unwrap_or_else(|e| panic!("no text of record for island {op}: {e}"))
        .trim_end()
        .to_string()
}

/// Every text of record, for the literals no island include anchors.
fn texts_of_record(contracts: &Path) -> BTreeSet<String> {
    std::fs::read_dir(contracts)
        .expect("build.rs writes the texts of record")
        .map(|entry| {
            let path = entry.expect("readable").path();
            std::fs::read_to_string(path)
                .expect("readable")
                .trim_end()
                .to_string()
        })
        .collect()
}
