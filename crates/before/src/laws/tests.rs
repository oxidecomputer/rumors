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

/// The conservation laws' deterministic witness: the feed order
/// [a, alias(a), b, alias(b), c] holds both `join_all` conservation laws
/// while its hand-back contains a *coalesced* group.
///
/// Two pins ride on the constructed shape. First, the party and clock
/// conservation laws hold on exactly the family whose retained group a
/// discipline-level drop would lose — the deterministic tripwire beside
/// the pool-driven law family, red the day the fold misroutes that group
/// (dropping it outright flips the verdict, which the acceptance laws
/// police; dropping it only when the rejection channel is already
/// nonempty is visible to the conservation laws alone). Second, the
/// hand-back genuinely contains a coalesced group, which is why the laws
/// are stated over region unions and never over byte identity with the
/// input list.
#[test]
fn conservation_witness_coalesces_the_hand_back() {
    use crate::{Clock, Party};

    // The party face.
    let mut p = Party::seed();
    let shares: Vec<Party> = p.forks(3).collect();
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
        "the party conservation law must hold on the retained-group witness"
    );
    let mut acc = p.dangerously_alias();
    let returned = acc
        .join_all(items.iter().map(Party::dangerously_alias))
        .expect_err("aliased inputs must be refused");
    assert!(
        returned
            .iter()
            .any(|back| [&a, &b, &c].iter().all(|input| back != *input)),
        "the closing drain hands back a coalesced group, not input bytes"
    );

    // The clock face, over the same feed shape with ticked-apart lines.
    let mut seed = Clock::seed();
    let mut lines: Vec<Clock> = seed.forks(3).collect();
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
        "the clock conservation law must hold on the retained-group witness"
    );
    let mut acc = seed.dangerously_alias();
    let returned = acc
        .join_all(items.iter().map(Clock::dangerously_alias))
        .expect_err("aliased inputs must be refused");
    assert!(
        returned
            .iter()
            .any(|back| [&a, &b, &c].iter().all(|input| back != *input)),
        "the closing drain hands back a coalesced group, not input bytes"
    );
}
