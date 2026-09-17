//! Consistency checks for the registered algebraic laws.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

/// Every registered law has a unique name.
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

/// Every law group declared in `laws.rs` is registered for execution.
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
        "declared and registered law groups differ"
    );
}

/// The conservation laws hold when several aliases overlap within one fold.
#[test]
fn conservation_laws_cover_multiple_aliases() {
    use crate::{Clock, Party};

    let mut p = Party::seed();
    let shares: Vec<Party> = p.forks(3u64).collect();
    let [a, b, c] = shares.try_into().expect("three shares");
    let items = vec![
        a.dangerously_alias(),
        a.dangerously_alias(),
        b.dangerously_alias(),
        b.dangerously_alias(),
        c.dangerously_alias(),
    ];
    assert!(
        super::party_join_all_err_conserves_the_region_union(&p, &items),
        "the party conservation law failed with repeated aliases"
    );

    let mut seed = Clock::seed();
    let mut lines: Vec<Clock> = seed.forks(3u64).collect();
    for line in &mut lines {
        line.tick();
    }
    let [a, b, c] = lines.try_into().expect("three lines");
    let items = vec![
        a.dangerously_alias(),
        a.dangerously_alias(),
        b.dangerously_alias(),
        b.dangerously_alias(),
        c.dangerously_alias(),
    ];
    assert!(
        super::clock_join_all_err_conserves_the_region_union(&seed, &items),
        "the clock conservation law failed with repeated aliases"
    );
}
