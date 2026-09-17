//! Checks the differential descriptors' coverage and registration.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

use proptest::prelude::*;

use super::{registered_names, BespokeCategory, DiffOp, DIFF_BESPOKE, REGISTERED_GROUPS};
use crate::oracle;
use crate::surface::{Leg, FAMILY_SURFACE, METHOD_SURFACE};
use crate::testing::generators::{arb_oracle_party, arb_oracle_party_nonempty, arb_oracle_version};
use crate::testing::optrace::{run, world_strategy};
use crate::Ticks;

/// Asserts every descriptor in a slice and identifies a failing descriptor.
macro_rules! assert_diff_ops {
    ($group:expr, $($input:expr),+) => {
        for (name, check) in $group {
            prop_assert!(check($($input),+), "descriptor violated: {}", name);
        }
    };
}

/// Collects every test cited by a `Leg::Bound` disposition.
fn bound_citations() -> BTreeSet<&'static str> {
    METHOD_SURFACE
        .iter()
        .chain(FAMILY_SURFACE)
        .flat_map(|row| {
            [&row.prod_tree, &row.prod_fs, &row.tree_fs]
                .into_iter()
                .filter_map(|leg| match leg {
                    Leg::Bound(test) => Some(*test),
                    _ => None,
                })
        })
        .collect()
}

/// Every bound citation is classified exactly once as derived or bespoke.
///
/// The test compares the coverage citations with the descriptor and bespoke
/// tables in both directions. It therefore catches missing, duplicate, and
/// stale classifications.
#[test]
fn diff_ops_tile_the_bound_citations() {
    let cited = bound_citations();
    let derived: BTreeSet<&str> = registered_names().into_iter().collect();
    assert_eq!(
        derived.len(),
        registered_names().len(),
        "duplicate descriptor names: a failure must name exactly one descriptor"
    );

    let mut bespoke: BTreeMap<&str, BespokeCategory> = BTreeMap::new();
    for (name, category) in DIFF_BESPOKE {
        assert!(
            cited.contains(*name),
            "DIFF_BESPOKE names {name:?}, which no roster row cites as a \
             Bound differential: remove or rename the entry"
        );
        assert!(
            !derived.contains(*name),
            "{name}: derived from the descriptor table AND rostered as \
             bespoke — the tiling sides must stay disjoint; remove one"
        );
        assert!(
            bespoke.insert(*name, *category).is_none(),
            "{name} appears twice in DIFF_BESPOKE"
        );
    }

    let unclassified: Vec<&str> = cited
        .iter()
        .copied()
        .filter(|name| !derived.contains(name) && !bespoke.contains_key(name))
        .collect();
    assert!(
        unclassified.is_empty(),
        "Bound citations neither derived from the descriptor table nor \
         listed in DIFF_BESPOKE with a category: {unclassified:?}"
    );

    // The reverse leg on the derived side: a descriptor no row cites is a
    // check nothing in the roster claims, which the coverage suite would
    // never notice going missing.
    let orphans: Vec<&str> = derived
        .iter()
        .copied()
        .filter(|name| !cited.contains(name))
        .collect();
    assert!(
        orphans.is_empty(),
        "registered descriptors cited by no roster row (cite each from the \
         row it binds, or retire the descriptor): {orphans:?}"
    );
}

/// Every bespoke category classifies at least one differential.
#[test]
fn every_bespoke_category_is_used() {
    let mut census: BTreeMap<&str, usize> = BTreeMap::new();
    for (_, category) in DIFF_BESPOKE {
        *census.entry(category.name()).or_default() += 1;
    }
    for category in BespokeCategory::CATEGORIES {
        assert!(
            census.get(category).copied().unwrap_or(0) > 0,
            "bespoke category {category} classifies no differential"
        );
    }
}

/// Every descriptor group declared in `diff_ops.rs` is registered.
///
/// The test compares the source declarations with `for_each_diff_group!`,
/// which generates each consumer. Known-bad descriptors live in this test
/// module and are intentionally absent from both sets.
#[test]
fn every_descriptor_group_is_registered() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/testing/diff_ops.rs");
    let text =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    let mut declared = BTreeSet::new();
    for line in text.lines() {
        if let Some(rest) = line.trim_start().strip_prefix("pub(crate) static ") {
            let name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            assert!(
                !name.is_empty(),
                "unnamed pub(crate) static in diff_ops.rs: {line}"
            );
            declared.insert(name);
        }
    }
    let registered: BTreeSet<String> = REGISTERED_GROUPS
        .iter()
        .map(|group| (*group).to_string())
        .collect();
    assert_eq!(
        declared, registered,
        "the descriptor-group statics in diff_ops.rs and the \
         for_each_diff_group! roster must be the same set: an unrostered \
         group never executes, and a rostered phantom names nothing"
    );
}

// ───────────────────── the drivers ─────────────────────
//
// Two populations, one drive list each, both expanded from the group
// roster: a descriptor added to a group with a known signature meets both
// with no further wiring, and a group with a novel signature refuses to
// compile until each consumer grows an arm. The populations are held
// oracle-side and raised through the bridge inside each descriptor, which
// is also why `!Clone` production types cost nothing here.
//
// Every arm also owes the descriptors' fs spellings a bounded population:
// an fs leg scans `2^g` grid points at the grid `fs_grid` derives from its
// operands' structural depths, so each arm's operands must come from a
// depth-capped source. Both populations satisfy it by construction — the
// arbitrary generators recurse to `generators::ARB_DEPTH`, and the organic
// traces bound tree depth by `optrace::MAX_TRACE_OPS` (one level per op at
// most) — and `fs_grid`'s own assert holds the one representation bound
// (the `Dyadic` u64 index width) against a population that escapes both
// derivations.

/// Tick counts small enough for the recursive oracle to evaluate directly.
const DRIVEN_TICK_COUNTS: std::ops::Range<u64> = 0..24;

/// Generates one arbitrary-population property for each descriptor group.
macro_rules! group_drivers {
    (args: (); $(($group:ident, $driver:ident, $shape:tt)),* $(,)?) => {
        $( group_drivers!(@one $group, $driver, $shape); )*
    };
    (@one $group:ident, $driver:ident, (version)) => {
        proptest! {
            /// Every descriptor in the group agrees with the oracle on
            /// arbitrary normal-form versions, large-base events included.
            #[test]
            fn $driver(a in arb_oracle_version()) {
                assert_diff_ops!(super::$group, &a);
            }
        }
    };
    (@one $group:ident, $driver:ident, (version, party)) => {
        proptest! {
            /// Every descriptor in the group agrees with the oracle on
            /// arbitrary normal-form version/id pairings.
            ///
            /// The id's shape is unrelated to the history's, which is
            /// where the full-subtree arms, the cost folding, and the
            /// root-ward tie-break live.
            #[test]
            fn $driver(a in arb_oracle_version(), p in arb_oracle_party_nonempty()) {
                assert_diff_ops!(super::$group, &a, &p);
            }
        }
    };
    (@one $group:ident, $driver:ident, (version, party, ticks)) => {
        proptest! {
            /// Every descriptor in the group agrees with the oracle on
            /// arbitrary normal-form version/id pairings across the
            /// affordable tick counts, the zero count included.
            #[test]
            fn $driver(
                a in arb_oracle_version(),
                p in arb_oracle_party_nonempty(),
                n in DRIVEN_TICK_COUNTS,
            ) {
                assert_diff_ops!(super::$group, &a, &p, &Ticks::from(n));
            }
        }
    };
    (@one $group:ident, $driver:ident, (party)) => {
        proptest! {
            /// Every descriptor in the group agrees with the oracle on
            /// arbitrary non-empty normal-form ids.
            #[test]
            fn $driver(a in arb_oracle_party_nonempty()) {
                assert_diff_ops!(super::$group, &a);
            }
        }
    };
    (@one $group:ident, $driver:ident, (party, party)) => {
        proptest! {
            /// Every descriptor in the group agrees with the oracle on
            /// arbitrary normal-form id pairs.
            ///
            /// The pairs are typically unrelated and frequently
            /// overlapping, and the anonymous id is admitted, so the
            /// overlap and emptying arms a seed-derived pipeline never
            /// produces are reachable here.
            #[test]
            fn $driver(a in arb_oracle_party(), b in arb_oracle_party()) {
                assert_diff_ops!(super::$group, &a, &b);
            }
        }
    };
    (@one $group:ident, $driver:ident, (disjoint_party, disjoint_party)) => {
        proptest! {
            /// Every descriptor in the group agrees with the oracle on
            /// fork-derived disjoint id pairs.
            ///
            /// The pair is the paper split of one arbitrary nonempty
            /// region — two nonempty disjoint halves, run in both operand
            /// orders — so the population premise the group's descriptors
            /// rest on holds on every case, and is asserted rather than
            /// assumed.
            #[test]
            fn $driver(p in arb_oracle_party_nonempty(), swap in proptest::bool::ANY) {
                let mut a = p;
                let b = a.fork();
                let (a, b) = if swap { (b, a) } else { (a, b) };
                prop_assert!(
                    a.is_disjoint(&b),
                    "fork halves are disjoint by construction"
                );
                assert_diff_ops!(super::$group, &a, &b);
            }
        }
    };
    (@one $group:ident, $driver:ident, (version, party, version)) => {
        proptest! {
            /// Every descriptor in the group agrees with the oracle on
            /// arbitrary normal-form operands: an unrelated history, an
            /// unrelated region to project through, and an unrelated
            /// history to compare against.
            #[test]
            fn $driver(
                a in arb_oracle_version(),
                p in arb_oracle_party_nonempty(),
                b in arb_oracle_version(),
            ) {
                assert_diff_ops!(super::$group, &a, &p, &b);
            }
        }
    };
    (@one $group:ident, $driver:ident, (version, party, version, party)) => {
        proptest! {
            /// Every descriptor in the group agrees with the oracle on
            /// arbitrary normal-form operands, each history projected
            /// through its own unrelated region.
            #[test]
            fn $driver(
                a in arb_oracle_version(),
                p in arb_oracle_party_nonempty(),
                b in arb_oracle_version(),
                q in arb_oracle_party_nonempty(),
            ) {
                assert_diff_ops!(super::$group, &a, &p, &b, &q);
            }
        }
    };
    (@one $group:ident, $driver:ident, (clock)) => {
        proptest! {
            /// Every descriptor in the group agrees with the oracle on
            /// arbitrary canonical id/history pairings.
            ///
            /// Every such pairing is a valid clock, including ones no op
            /// sequence reaches: the id's region need bear no relation to
            /// where the history is live.
            #[test]
            fn $driver(p in arb_oracle_party_nonempty(), a in arb_oracle_version()) {
                let c = oracle::Clock::from_parts(p, a);
                assert_diff_ops!(super::$group, &c);
            }
        }
    };
    (@one $group:ident, $driver:ident, (version, version)) => {
        proptest! {
            /// Every descriptor in the group agrees with the oracle on
            /// arbitrary normal-form version pairs: independent shapes,
            /// and base magnitudes whose root-to-leaf path sums run past
            /// what a machine word holds.
            #[test]
            fn $driver(a in arb_oracle_version(), b in arb_oracle_version()) {
                assert_diff_ops!(super::$group, &a, &b);
            }
        }
    };
}

for_each_diff_group!(group_drivers);

/// Inputs selected from one organically generated clock population.
struct Organic<'a> {
    /// Three versions from the trace, causally related.
    v: [&'a oracle::Version; 3],
    /// Two live ids from the same trace — distinct picks are disjoint by
    /// single-seed linearity, but the pairing indices are independent, so
    /// the two may be the same clock's id (overlapping with itself).
    p: [&'a oracle::Party; 2],
    /// A tick count from [`DRIVEN_TICK_COUNTS`].
    n: Ticks,
    /// A reachable clock from the same trace.
    c: &'a oracle::Clock,
}

/// Applies every descriptor group to compatible inputs from [`Organic`].
macro_rules! organic_drive {
    (args: ($env:expr); $(($group:ident, $driver:ident, $shape:tt)),* $(,)?) => {
        $( organic_drive!(@one $env, $group, $shape); )*
    };
    (@one $env:expr, $group:ident, (version)) => {
        assert_diff_ops!(super::$group, $env.v[0]);
    };
    (@one $env:expr, $group:ident, (version, party)) => {
        assert_diff_ops!(super::$group, $env.v[0], $env.p[0]);
    };
    (@one $env:expr, $group:ident, (version, party, ticks)) => {
        assert_diff_ops!(super::$group, $env.v[0], $env.p[0], &$env.n);
    };
    (@one $env:expr, $group:ident, (party)) => {
        assert_diff_ops!(super::$group, $env.p[0]);
    };
    (@one $env:expr, $group:ident, (party, party)) => {
        assert_diff_ops!(super::$group, $env.p[0], $env.p[1]);
    };
    // A disjoint organic pair is derived, never picked: the two picks may
    // alias the same clock's id (the pairing indices are independent), so
    // the arm forks one organic region into its two disjoint halves — the
    // same derivation as the arbitrary driver, on organic shapes.
    (@one $env:expr, $group:ident, (disjoint_party, disjoint_party)) => {
        let mut left = ::core::clone::Clone::clone($env.p[0]);
        let right = left.fork();
        assert_diff_ops!(super::$group, &left, &right);
    };
    (@one $env:expr, $group:ident, (version, party, version)) => {
        assert_diff_ops!(super::$group, $env.v[0], $env.p[0], $env.v[1]);
    };
    (@one $env:expr, $group:ident, (version, party, version, party)) => {
        assert_diff_ops!(super::$group, $env.v[0], $env.p[0], $env.v[1], $env.p[1]);
    };
    (@one $env:expr, $group:ident, (clock)) => {
        assert_diff_ops!(super::$group, $env.c);
    };
    (@one $env:expr, $group:ident, (version, version)) => {
        assert_diff_ops!(super::$group, $env.v[0], $env.v[1]);
    };
}

proptest! {
    /// Every registered descriptor agrees with the oracle over organic
    /// op-trace populations.
    ///
    /// The same descriptors the arbitrary drivers run, landed on the value
    /// shapes real fork/tick/join/sync schedules produce: live sibling ids
    /// and causally related versions, where domination and equality are
    /// common rather than vanishing.
    ///
    /// The drive list runs twice per case, over two pairings of the same
    /// picks. In the first, each version travels with its *own* clock's
    /// id — the regime where the id owns exactly the regions that history
    /// may inflate. In the second the ids are exchanged, so a version
    /// meets a sibling's region: the cross-region shapes masking and
    /// projection answer non-trivially on.
    #[test]
    fn diff_ops_match_the_oracle_on_organic_populations(
        ops in world_strategy(),
        i in 0usize..64,
        j in 0usize..64,
        k in 0usize..64,
        ticks in DRIVEN_TICK_COUNTS,
    ) {
        let cs = run(&ops);
        let len = cs.len();
        let (pa, va) = cs[i % len].trees();
        let (pb, vb) = cs[j % len].trees();
        let (_, vc) = cs[k % len].trees();
        let n = Ticks::from(ticks);

        let c = &cs[i % len];

        let own = Organic { v: [va, vb, vc], p: [pa, pb], n: n.clone(), c };
        for_each_diff_group!(organic_drive(&own));

        let crossed = Organic { v: [va, vb, vc], p: [pb, pa], n, c };
        for_each_diff_group!(organic_drive(&crossed));
    }
}

// ─────────────── deliberately incorrect descriptors ───────────────
//
// A table centralizes each operation's oracle spelling: one descriptor is
// the only transcription every population sees, where a body per population
// was an independent transcription each. That trade is only payable if a
// wrong transcription cannot pass, so the wrong ones are committed here and
// rejected by focused tests. These groups are deliberately absent from the roster —
// registering them would drive them as if they were real — and the
// registration totality pin scans only the table's own file, so their
// `pub(crate) static`s do not reach it.

diff_ops! {
    /// The mis-transcribed version-pair descriptor: the oracle leg spells
    /// the join where the production leg spells the meet.
    ///
    /// The likeliest transcription slip is a dual operation, since the two
    /// sides read alike and differ only in one operator.
    pub(crate) static KNOWN_BAD_VERSION_PAIR: (a: version, b: version);

    /// `&` on production against `|` on the oracle.
    fn meet_transcribed_as_join {
        prod: a.clone() & b.clone(),
        tree: a.clone() | b.clone(),
    }
}

diff_ops! {
    /// The mis-transcribed id-pair descriptor: the oracle leg takes the
    /// region difference in the opposite operand order.
    ///
    /// The other likely slip is an operand swap on an asymmetric
    /// operation, which no amount of type checking catches.
    pub(crate) static KNOWN_BAD_PARTY_PAIR: (a: party, b: party);

    /// `a \ b` on production against `b \ a` on the oracle.
    fn without_transcribed_with_swapped_operands {
        prod: a.without(&b),
        tree: b.without(&a),
    }
}

diff_ops! {
    /// The mis-transcribed fs column: both walk legs spell the meet, the
    /// function-space leg spells the join.
    ///
    /// The likeliest fs transcription slip is the same dual-combinator
    /// slip the prod/tree known-bad commits — the combinators read alike
    /// and differ in one comparison — landed on the column where only the
    /// [`super::FsMatches`] comparison stands between it and green.
    pub(crate) static KNOWN_BAD_FS_VERSION_PAIR: (a: version, b: version);

    /// `&` on both walk legs against the pointwise max in the function
    /// space.
    fn fs_meet_transcribed_as_join {
        prod: a.clone() & b.clone(),
        tree: a.clone() & b.clone(),
        fs(_g): crate::testing::semantic_oracle::join(a, b),
    }
}

/// Checks a version-pair descriptor group.
// The group's element type is the signature it carries; naming it would
// require a redundant alias for each signature to appease the lint.
#[allow(clippy::type_complexity)]
fn check_version_pair(
    group: &[DiffOp<fn(&oracle::Version, &oracle::Version) -> bool>],
    a: &oracle::Version,
    b: &oracle::Version,
) -> Result<(), TestCaseError> {
    assert_diff_ops!(group, a, b);
    Ok(())
}

/// Checks a party-pair descriptor group.
// The group's element type is the signature it carries; naming it would
// require a redundant alias for each signature to appease the lint.
#[allow(clippy::type_complexity)]
fn check_party_pair(
    group: &[DiffOp<fn(&oracle::Party, &oracle::Party) -> bool>],
    a: &oracle::Party,
    b: &oracle::Party,
) -> Result<(), TestCaseError> {
    assert_diff_ops!(group, a, b);
    Ok(())
}

/// Descriptor assertions distinguish incorrect operations from equivalent
/// results.
///
/// Each known-bad descriptor is checked on one input where its operations
/// differ and one where they agree. This proves the comparison neither accepts
/// everything nor rejects everything.
#[test]
fn descriptor_checks_reject_a_mistranscribed_operation() {
    use crate::oracle::Version as V;

    // The join and the meet of an ordered pair differ, so the swapped
    // operator changes the answer.
    assert!(
        check_version_pair(KNOWN_BAD_VERSION_PAIR, &V::leaf(1u64), &V::leaf(2u64)).is_err(),
        "the meet-transcribed-as-join descriptor must be rejected where \
         the join and the meet disagree"
    );
    // On a coincident pair the two operations agree.
    assert!(
        check_version_pair(KNOWN_BAD_VERSION_PAIR, &V::leaf(3u64), &V::leaf(3u64)).is_ok(),
        "the same descriptor must pass where the join and the meet coincide: \
         a comparison that rejects everything proves nothing"
    );

    // Two disjoint halves: each survives the other's removal, and the two
    // remainders are different regions, so the operand swap changes the
    // answer.
    let mut keep = oracle::Party::seed();
    let give = keep.fork();
    assert!(
        check_party_pair(KNOWN_BAD_PARTY_PAIR, &keep, &give).is_err(),
        "the operand-swapped difference descriptor must be rejected where \
         the two orders yield different regions"
    );
    // Against itself the difference is empty in either order.
    assert!(
        check_party_pair(KNOWN_BAD_PARTY_PAIR, &keep, &keep).is_ok(),
        "the same descriptor must pass where both operand orders yield the \
         empty region"
    );
}

/// Filesystem-result assertions distinguish an incorrect combinator from an
/// equivalent result.
///
/// The descriptor's walk operations both compute the meet, so the differing
/// input isolates [`FsMatches`](super::FsMatches). An agreeing input confirms
/// that the comparison does not reject indiscriminately.
#[test]
fn filesystem_checks_reject_a_mistranscribed_operation() {
    use crate::oracle::Version as V;

    // The join and the meet of an ordered pair differ, so the swapped
    // combinator changes the fs answer while both walk legs agree.
    assert!(
        check_version_pair(KNOWN_BAD_FS_VERSION_PAIR, &V::leaf(1u64), &V::leaf(2u64)).is_err(),
        "the join-transcribed fs spelling must be rejected where the join \
         and the meet disagree"
    );
    // On a coincident pair every spelling agrees, so nothing is there to
    // reject it.
    assert!(
        check_version_pair(KNOWN_BAD_FS_VERSION_PAIR, &V::leaf(3u64), &V::leaf(3u64)).is_ok(),
        "the same descriptor must pass where the join and the meet coincide: \
         a comparison that rejects everything proves nothing"
    );
}
