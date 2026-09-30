//! Focused regressions for conservation laws whose setup needs overlapping aliases.

use crate::{Clock, Party};

/// Party and clock conservation laws hold when a fold contains several aliases
/// of each owned region.
#[test]
fn conservation_laws_cover_multiple_aliases() {
    let mut party = Party::seed();
    let shares: Vec<Party> = party.forks(3u64).collect();
    let [a, b, c] = shares.try_into().expect("three shares");
    let items = vec![
        a.dangerously_alias(),
        a.dangerously_alias(),
        b.dangerously_alias(),
        b.dangerously_alias(),
        c.dangerously_alias(),
    ];
    let party_conservation = super::super::PARTY_AND_LIST
        .iter()
        .find_map(|(name, law)| {
            (*name == "party_join_all_err_conserves_the_region_union").then_some(law)
        })
        .expect("party conservation law is registered");
    assert!(
        party_conservation(&party, &items),
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
    let clock_conservation = super::super::CLOCK_AND_LIST
        .iter()
        .find_map(|(name, law)| {
            (*name == "clock_join_all_err_conserves_the_region_union").then_some(law)
        })
        .expect("clock conservation law is registered");
    assert!(
        clock_conservation(&seed, &items),
        "the clock conservation law failed with repeated aliases"
    );
}
