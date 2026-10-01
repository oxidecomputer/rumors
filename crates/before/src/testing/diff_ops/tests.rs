//! Checks the differential descriptors' coverage and registration.

use proptest::prelude::*;

use super::DiffOp;
use crate::testing::generators::{arb_oracle_party_nonempty, arb_oracle_version};
use crate::testing::optrace::{run, world_strategy};
use crate::testing::oracles::tree;

/// Asserts every descriptor in a slice and identifies a failing descriptor.
macro_rules! assert_diff_ops {
    ($group:expr, $($input:expr),+) => {
        for (name, check) in $group {
            prop_assert!(check($($input),+), "descriptor violated: {}", name);
        }
    };
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
// Every arm also owes the function-space spellings a bounded population:
// a function-space comparison scans `2^g` grid points at the grid `fs_grid` derives from its
// operands' structural depths, so each arm's operands must come from a
// depth-capped source. Both populations satisfy it by construction — the
// arbitrary generators recurse to `generators::ARB_DEPTH`, and the organic
// traces bound tree depth by `optrace::MAX_TRACE_OPS` (one level per op at
// most) — and `fs_grid`'s own assert holds the one representation bound
// (the `Dyadic` u64 index width) against a population that escapes both
// derivations.

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
            /// arbitrary normal-form version/party pairings.
            ///
            /// The party's shape is unrelated to the version's, which is
            /// where the full-subtree arms, the cost folding, and the
            /// root-ward tie-break live.
            #[test]
            fn $driver(a in arb_oracle_version(), p in arb_oracle_party_nonempty()) {
                assert_diff_ops!(super::$group, &a, &p);
            }
        }
    };
    (@one $group:ident, $driver:ident, (party)) => {
        proptest! {
            /// Every descriptor in the group agrees with the oracle on
            /// arbitrary non-empty normal-form parties.
            #[test]
            fn $driver(a in arb_oracle_party_nonempty()) {
                assert_diff_ops!(super::$group, &a);
            }
        }
    };
    (@one $group:ident, $driver:ident, (party, party)) => {
        proptest! {
            /// Every descriptor in the group agrees with the oracle on
            /// arbitrary nonempty normal-form party pairs.
            ///
            /// The pairs are typically unrelated and frequently
            /// overlapping, so the overlap and empty-result arms a
            /// seed-derived pipeline rarely produces remain reachable here.
            #[test]
            fn $driver(a in arb_oracle_party_nonempty(), b in arb_oracle_party_nonempty()) {
                assert_diff_ops!(super::$group, &a, &b);
            }
        }
    };
    (@one $group:ident, $driver:ident, (disjoint_party, disjoint_party)) => {
        proptest! {
            /// Every descriptor in the group agrees with the oracle on
            /// fork-derived disjoint party pairs.
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
            /// arbitrary normal-form operands: an unrelated version, an
            /// unrelated region to project through, and an unrelated
            /// version to compare against.
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
            /// arbitrary normal-form operands, each version projected
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
            /// arbitrary canonical party/version pairings.
            ///
            /// Every such pairing is a valid clock, including ones no
            /// operation sequence reaches: the party need bear no relation to
            /// where the version is nonzero.
            #[test]
            fn $driver(p in arb_oracle_party_nonempty(), a in arb_oracle_version()) {
                let c = tree::Clock::from_parts(p, a);
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
    v: [&'a tree::Version; 3],
    /// Two live parties from the same trace — distinct picks are disjoint by
    /// single-seed linearity, but the pairing indices are independent, so
    /// the two may be the same clock's party (overlapping with itself).
    p: [&'a tree::Party; 2],
    /// A reachable clock from the same trace.
    c: &'a tree::Clock,
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
    (@one $env:expr, $group:ident, (party)) => {
        assert_diff_ops!(super::$group, $env.p[0]);
    };
    (@one $env:expr, $group:ident, (party, party)) => {
        assert_diff_ops!(super::$group, $env.p[0], $env.p[1]);
    };
    // A disjoint organic pair is derived, never picked: the two picks may
    // alias the same clock's party (the pairing indices are independent), so
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
    /// shapes real fork/tick/join/sync schedules produce: live sibling parties
    /// and causally related versions, where domination and equality are
    /// common rather than vanishing.
    ///
    /// The drive list runs twice per case, over two pairings of the same
    /// picks. In the first, each version travels with its *own* clock's
    /// party — the regime where that party owns exactly the regions the
    /// version may advance. In the second the parties are exchanged, so a version
    /// meets a sibling's region: the cross-region shapes masking and
    /// projection answer non-trivially on.
    #[test]
    fn diff_ops_match_the_oracle_on_organic_populations(
        ops in world_strategy(),
        i in 0usize..64,
        j in 0usize..64,
        k in 0usize..64,
    ) {
        let cs = run(&ops);
        let len = cs.len();
        let (pa, va) = cs[i % len].trees();
        let (pb, vb) = cs[j % len].trees();
        let (_, vc) = cs[k % len].trees();
        let c = &cs[i % len];

        let own = Organic { v: [va, vb, vc], p: [pa, pb], c };
        for_each_diff_group!(organic_drive(&own));

        let crossed = Organic { v: [va, vb, vc], p: [pb, pa], c };
        for_each_diff_group!(organic_drive(&crossed));
    }
}

// ─────────────── deliberately incorrect descriptors ───────────────
//
// A table centralizes each operation's oracle spelling: one descriptor is
// the only transcription every population sees, where a body per population
// was an independent transcription each. That trade is only payable if a
// wrong transcription cannot pass, so the wrong ones are committed here and
// rejected by focused tests. These groups live in the test module and are
// deliberately absent from the real descriptor groups.

diff_ops! {
    /// The mis-transcribed version-pair descriptor: the oracle leg spells
    /// the join where the production leg spells the meet.
    ///
    /// The likeliest transcription slip is a dual operation, since the two
    /// sides read alike and differ only in one operator.
    static KNOWN_BAD_VERSION_PAIR: (a: version, b: version);

    /// `&` on production against `|` on the oracle.
    fn meet_transcribed_as_join {
        production: a.clone() & b.clone(),
        recursive: a.clone() | b.clone(),
        function(_g): crate::testing::oracles::function::join(a, b),
    }
}

diff_ops! {
    /// The mis-transcribed party-pair descriptor: the oracle leg takes the
    /// region difference in the opposite operand order.
    ///
    /// The other likely slip is an operand swap on an asymmetric
    /// operation, which no amount of type checking catches.
    static KNOWN_BAD_PARTY_PAIR: (a: party, b: party);

    /// `a \ b` on production against `b \ a` on the oracle.
    fn without_transcribed_with_swapped_operands {
        production: a.without(&b),
        recursive: b.without(&a),
        function(_g): crate::testing::oracles::function::diff(b, a),
    }
}

diff_ops! {
    /// The mis-transcribed function-space column: both tree-based legs spell the meet, the
    /// function-space leg spells the join.
    ///
    /// This models a likely transcription error: choosing the dual
    /// combinator in only one of the three implementations.
    static KNOWN_BAD_FS_VERSION_PAIR: (a: version, b: version);

    /// `&` on both walk legs against the pointwise max in the function
    /// space.
    fn fs_meet_transcribed_as_join {
        production: a.clone() & b.clone(),
        recursive: a.clone() & b.clone(),
        function(_g): crate::testing::oracles::function::join(a, b),
    }
}

/// Checks a version-pair descriptor group.
// The group's element type is the signature it carries; naming it would
// require a redundant alias for each signature to appease the lint.
#[allow(clippy::type_complexity)]
fn check_version_pair(
    group: &[DiffOp<fn(&tree::Version, &tree::Version) -> bool>],
    a: &tree::Version,
    b: &tree::Version,
) -> Result<(), TestCaseError> {
    assert_diff_ops!(group, a, b);
    Ok(())
}

/// Checks a party-pair descriptor group.
// The group's element type is the signature it carries; naming it would
// require a redundant alias for each signature to appease the lint.
#[allow(clippy::type_complexity)]
fn check_party_pair(
    group: &[DiffOp<fn(&tree::Party, &tree::Party) -> bool>],
    a: &tree::Party,
    b: &tree::Party,
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
    use crate::testing::oracles::tree::Version as V;

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
    let mut keep = tree::Party::seed();
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

/// Function-space assertions distinguish an incorrect combinator from an
/// equivalent result.
///
/// The descriptor's walk operations both compute the meet, so the differing
/// input isolates [`FunctionMatches`](super::FunctionMatches). An agreeing input confirms
/// that the comparison does not reject indiscriminately.
#[test]
fn function_space_checks_reject_a_mistranscribed_operation() {
    use crate::testing::oracles::tree::Version as V;

    // The join and the meet of an ordered pair differ, so the swapped
    // combinator changes the function-space answer while both tree-based legs agree.
    assert!(
        check_version_pair(KNOWN_BAD_FS_VERSION_PAIR, &V::leaf(1u64), &V::leaf(2u64)).is_err(),
        "the join-transcribed function-space spelling must be rejected where the join \
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
