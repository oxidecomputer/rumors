//! Guards on the law collection itself (the laws are *asserted* by the
//! drivers in [`crate::testing`]'s algebraic-laws suite and by the fuzz
//! workspace; here we pin the collection's own invariants).

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

/// Every law name is unique across all groups, so a failure anywhere names
/// exactly one law — the property the whole collection exists to provide.
#[test]
fn law_names_are_unique_across_groups() {
    let names = super::registered_names();
    let mut seen = BTreeSet::new();
    let duplicates: Vec<&str> = names
        .iter()
        .filter(|name| !seen.insert(**name))
        .copied()
        .collect();
    assert!(duplicates.is_empty(), "duplicate law names: {duplicates:?}");
}

/// One consumer file's source text, read from the crate root.
fn source(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
}

/// Occurrences of `!(laws::GROUP` with the group name ending at a word
/// boundary: the macro-invocation spelling every driver uses
/// (`assert_laws!(laws::GROUP, ...)`, `drive!(laws::GROUP, ...)`),
/// which rustdoc links and prose mentions of a group cannot satisfy.
fn drive_sites(text: &str, group: &str) -> usize {
    let needle = format!("!(laws::{group}");
    let mut count = 0;
    let mut rest = text;
    while let Some(at) = rest.find(&needle) {
        let after = &rest[at + needle.len()..];
        let boundary = after
            .chars()
            .next()
            .is_none_or(|c| !c.is_alphanumeric() && c != '_');
        if boundary {
            count += 1;
        }
        rest = &rest[at + needle.len()..];
    }
    count
}

/// Every `pub static` law group in `laws.rs` is chained into
/// [`super::registered_names`] — no group can compile, ship, and never
/// execute.
///
/// The name-facing checks (the uniqueness pin above, the coverage
/// roster's citation pin) all resolve against `registered_names`, so a
/// group left out of its chain is invisible to every one of them by
/// construction; this pin closes the roster from the other side, by
/// comparing the chain's own group list against a source scan of the
/// `pub static` declarations in this module (its only `pub static`s are
/// law groups). The surface-totality gate cannot carry this: its
/// extractor walks function-like items only, so statics never reach it.
#[test]
fn every_law_group_is_registered() {
    let text = source("src/laws.rs");
    let mut declared = BTreeSet::new();
    for line in text.lines() {
        if let Some(rest) = line.trim_start().strip_prefix("pub static ") {
            let name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            assert!(!name.is_empty(), "unnamed pub static in laws.rs: {line}");
            declared.insert(name);
        }
    }
    let registered: BTreeSet<String> = super::REGISTERED_GROUPS
        .iter()
        .map(|g| (*g).to_string())
        .collect();
    assert_eq!(
        declared, registered,
        "the law-group statics in laws.rs and the register_groups! list \
         must be the same set: an unregistered group never executes, and \
         a registered phantom names nothing"
    );
}

/// Every registered law group is driven by every consumer: the two
/// in-crate drivers (the per-group proptest driver and the
/// organic-populations driver, both in the algebraic-laws suite) and
/// the fuzz target.
///
/// Group wiring is hand-maintained per consumer — a new group needs a
/// driver fn, an organic-populations arm, and a fuzz arm — and nothing
/// type-level forces any of them, so this pin scans each consumer's
/// source for the `!(laws::GROUP` drive site. The algebraic-laws suite
/// must drive each group at least twice (once per driver); the fuzz
/// target at least once.
#[test]
fn every_law_group_reaches_every_consumer() {
    let suite = source("src/testing/algebraic_laws/tests.rs");
    let fuzz = source("fuzz/fuzz_targets/fuzz_laws.rs");
    let mut missing = Vec::new();
    for group in super::REGISTERED_GROUPS {
        let in_suite = drive_sites(&suite, group);
        if in_suite < 2 {
            missing.push(format!(
                "{group}: {in_suite} drive site(s) in the algebraic-laws \
                 suite (needs the per-group proptest driver AND the \
                 organic-populations arm)"
            ));
        }
        if drive_sites(&fuzz, group) == 0 {
            missing.push(format!("{group}: no drive site in fuzz_laws"));
        }
    }
    assert!(
        missing.is_empty(),
        "law groups not wired to every consumer:\n  {}",
        missing.join("\n  ")
    );
}
