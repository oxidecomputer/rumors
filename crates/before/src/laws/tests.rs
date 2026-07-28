//! Guards on the law collection itself (the laws are *asserted* by the
//! drivers in [`crate::testing`]'s algebraic-laws suite and by the fuzz
//! workspace; here we pin the collection's own invariants).

use std::collections::BTreeSet;

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
