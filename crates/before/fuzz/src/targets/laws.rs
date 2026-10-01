//! Algebraic laws evaluated over values decoded from arbitrary bytes.
//!
//! The in-tree properties generate ordinary normal-form values. Decoding first
//! lets coverage-guided mutation present the same law collection with unusual
//! but valid tree shapes and numeric widths. Invalid chunks fall back to each
//! type's canonical starting value, so every input still evaluates every law;
//! successful decoding earns coverage by changing the operands.

use before::{Clock, Party, Rank, Version};

use crate::input::law_input;

/// Assert every law in a group, naming the violated law on failure.
macro_rules! drive {
    ($group:expr, $($input:expr),+) => {
        for (name, law) in $group {
            assert!(law($($input),+), "law violated: {name}");
        }
    };
}

/// Values selected for each signature in the shared law-group list.
struct Environment<'a> {
    /// Three decoded versions.
    versions: [&'a Version; 3],
    /// Two decoded parties and the decoded clock's party.
    parties: [&'a Party; 3],
    /// Two version ranks and their distance.
    ranks: [&'a Rank; 3],
    /// The decoded clock and a clock assembled from decoded components.
    clocks: [&'a Clock; 2],
    /// The version script's selected values.
    version_list: &'a [Version],
    /// The party script's selected values.
    party_list: &'a [Party],
    /// The clock script's selected values.
    clock_list: &'a [Clock],
}

/// Map every law signature to values from [`Environment`].
///
/// `for_each_law_group!` owns the list of groups. A new signature fails to
/// compile here until its inputs are chosen deliberately.
macro_rules! drive_groups {
    (args: ($env:expr); $(($group:ident, $driver:ident, $shape:tt)),* $(,)?) => {
        $( drive_groups!(@one $env, $group, $shape); )*
    };
    (@one $env:expr, $group:ident, (version)) => {
        drive!(before::testing::laws::$group, $env.versions[0]);
    };
    (@one $env:expr, $group:ident, (version, version)) => {
        drive!(before::testing::laws::$group, $env.versions[0], $env.versions[1]);
    };
    (@one $env:expr, $group:ident, (version, version, version)) => {
        drive!(before::testing::laws::$group, $env.versions[0], $env.versions[1], $env.versions[2]);
    };
    (@one $env:expr, $group:ident, (party)) => {
        drive!(before::testing::laws::$group, $env.parties[0]);
    };
    (@one $env:expr, $group:ident, (party, party)) => {
        drive!(before::testing::laws::$group, $env.parties[0], $env.parties[1]);
    };
    (@one $env:expr, $group:ident, (party, party, party)) => {
        drive!(before::testing::laws::$group, $env.parties[0], $env.parties[1], $env.parties[2]);
    };
    (@one $env:expr, $group:ident, (version, party)) => {
        drive!(before::testing::laws::$group, $env.versions[0], $env.parties[0]);
    };
    (@one $env:expr, $group:ident, (version, version, party)) => {
        drive!(before::testing::laws::$group, $env.versions[0], $env.versions[1], $env.parties[0]);
    };
    (@one $env:expr, $group:ident, (version, party, party)) => {
        drive!(before::testing::laws::$group, $env.versions[0], $env.parties[0], $env.parties[1]);
    };
    (@one $env:expr, $group:ident, (version, version, party, party)) => {
        drive!(before::testing::laws::$group, $env.versions[0], $env.versions[1], $env.parties[0], $env.parties[1]);
    };
    (@one $env:expr, $group:ident, (rank, rank, rank)) => {
        drive!(before::testing::laws::$group, $env.ranks[0], $env.ranks[1], $env.ranks[2]);
    };
    (@one $env:expr, $group:ident, (clock)) => {
        drive!(before::testing::laws::$group, $env.clocks[0]);
    };
    (@one $env:expr, $group:ident, (clock, clock)) => {
        drive!(before::testing::laws::$group, $env.clocks[0], $env.clocks[1]);
    };
    (@one $env:expr, $group:ident, (clock, version)) => {
        drive!(before::testing::laws::$group, $env.clocks[0], $env.versions[0]);
    };
    (@one $env:expr, $group:ident, (versions)) => {
        drive!(before::testing::laws::$group, $env.version_list);
    };
    (@one $env:expr, $group:ident, (version, versions)) => {
        drive!(before::testing::laws::$group, $env.versions[0], $env.version_list);
    };
    (@one $env:expr, $group:ident, (party, parties)) => {
        drive!(before::testing::laws::$group, $env.parties[0], $env.party_list);
    };
    (@one $env:expr, $group:ident, (clock, clocks)) => {
        drive!(before::testing::laws::$group, $env.clocks[0], $env.clock_list);
    };
}

/// Decode one input and evaluate every group in the law list.
pub fn run(data: &[u8]) {
    let input = law_input(data);
    let a = Version::decode(input.versions[0]).unwrap_or_default();
    let b = Version::decode(input.versions[1]).unwrap_or_default();
    let c = Version::decode(input.versions[2]).unwrap_or_default();
    let p = Party::decode(input.parties[0]).unwrap_or_else(|_| Party::seed());
    let q = Party::decode(input.parties[1]).unwrap_or_else(|_| Party::seed());
    let clock = Clock::decode(input.clock).unwrap_or_else(|_| Clock::seed());

    let zero = Version::new();
    let version_pool = [&a, &b, &c, &zero];
    let version_list = input
        .version_indices
        .into_iter()
        .map(|index| version_pool[index].clone())
        .collect::<Vec<_>>();

    let party_pool = [&p, &q, clock.party()];
    let party_list = input
        .party_indices
        .into_iter()
        .map(|index| party_pool[index].dangerously_alias())
        .collect::<Vec<_>>();

    let assembled_a = Clock::from_parts(p.dangerously_alias(), a.clone());
    let assembled_b = Clock::from_parts(q.dangerously_alias(), b.clone());
    let clock_pool = [&clock, &assembled_a, &assembled_b];
    let clock_list = input
        .clock_indices
        .into_iter()
        .map(|index| clock_pool[index].dangerously_alias())
        .collect::<Vec<_>>();

    let rank_a = a.rank();
    let rank_b = b.rank();
    let distance = a.distance(&b);
    let environment = Environment {
        versions: [&a, &b, &c],
        parties: [&p, &q, clock.party()],
        ranks: [&rank_a, &rank_b, &distance],
        clocks: [&clock, &assembled_a],
        version_list: &version_list,
        party_list: &party_list,
        clock_list: &clock_list,
    };
    before::for_each_law_group!(drive_groups(&environment));
}
