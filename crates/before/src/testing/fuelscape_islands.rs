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

    // Every `/fuelscapes/<op>.html` occurrence in the crate's sources.
    let mut included = BTreeSet::new();
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
                for site in source.split("/fuelscapes/").skip(1) {
                    let Some((op, _)) = site.split_once(".html") else {
                        continue;
                    };
                    let island_shaped = !op.is_empty()
                        && op
                            .bytes()
                            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
                    if island_shaped {
                        included.insert(op.to_string());
                    }
                }
            }
        }
    }

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
