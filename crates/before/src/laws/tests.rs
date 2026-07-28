//! Guards on the law collection itself (the laws are *asserted* by the
//! drivers in [`crate::testing`]'s algebraic-laws suite and by the fuzz
//! workspace; here we pin the collection's own invariants).

use std::collections::BTreeSet;

/// Every law name is unique across all groups, so a failure anywhere names
/// exactly one law — the property the whole collection exists to provide.
#[test]
fn law_names_are_unique_across_groups() {
    let names: Vec<&str> = std::iter::empty()
        .chain(super::VERSION_SOLO.iter().map(|(name, _)| *name))
        .chain(super::VERSION_PAIR.iter().map(|(name, _)| *name))
        .chain(super::VERSION_TRIPLE.iter().map(|(name, _)| *name))
        .chain(super::PARTY_SOLO.iter().map(|(name, _)| *name))
        .chain(super::PARTY_PAIR.iter().map(|(name, _)| *name))
        .chain(super::PARTY_TRIPLE.iter().map(|(name, _)| *name))
        .chain(super::VERSION_PARTY.iter().map(|(name, _)| *name))
        .chain(super::VERSION_PAIR_PARTY.iter().map(|(name, _)| *name))
        .chain(super::VERSION_PARTY_PAIR.iter().map(|(name, _)| *name))
        .chain(super::VERSION_PAIR_PARTY_PAIR.iter().map(|(name, _)| *name))
        .chain(super::RANK_TRIPLE.iter().map(|(name, _)| *name))
        .chain(super::CLOCK_SOLO.iter().map(|(name, _)| *name))
        .chain(super::CLOCK_VERSION.iter().map(|(name, _)| *name))
        .collect();
    let mut seen = BTreeSet::new();
    let duplicates: Vec<&str> = names
        .iter()
        .filter(|name| !seen.insert(**name))
        .copied()
        .collect();
    assert!(duplicates.is_empty(), "duplicate law names: {duplicates:?}");
}
