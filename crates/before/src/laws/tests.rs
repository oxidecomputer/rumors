//! Guards on the law collection itself (the laws are *asserted* by the drivers
//! in [`crate::testing`]'s algebraic-laws suite and by the fuzz workspace; here
//! we pin the collection's own invariants).

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

/// Every `pub static` law group in `laws.rs` is carried by the roster
/// (`for_each_law_group!`) — no group can compile, ship, and never execute.
///
/// Every consumer — the name chain the name-facing checks resolve against, both
/// algebraic-laws drivers, and the fuzz target's drive loop — derives from the
/// roster by macro expansion, so a rostered group is executed by construction
/// and needs no per-consumer pin. The one door that machinery leaves open is a
/// group static missing from the roster, which nothing would ever execute; this
/// pin closes it by comparing the roster's own group list against a source scan
/// of the `pub static` declarations in this module (its only `pub static`s are
/// law groups). The surface-totality gate cannot carry this: its extractor
/// walks function-like items only, so statics never reach it.
#[test]
fn every_law_group_is_registered() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/laws.rs");
    let text =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
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
        "the law-group statics in laws.rs and the for_each_law_group! \
         roster must be the same set: an unrostered group never executes, \
         and a rostered phantom names nothing"
    );
}
