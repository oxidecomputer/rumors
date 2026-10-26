//! The differential drivers over deep inputs: every descriptor group, fed
//! versions and parties tens to hundreds of levels deep, compared against the
//! recursive oracle.
//!
//! The arbitrary and trace drivers stop at depth four and at the trace length
//! cap. These drivers expand the same roster (`for_each_diff_group!`) over the
//! deep strategies in [`crate::testing::generators::leaf_list`] and
//! [`crate::testing::generators::regions`]. The recursive oracle handles their
//! depths, which stay at a few hundred levels; the function space cannot scan
//! them, so every check runs with [`Legs::TreeOracle`].
//!
//! A case holding a deep version costs many arbitrary cases, so each such
//! driver runs the configured case count divided by a divisor that grows with
//! the versions its inputs hold.

use proptest::prelude::*;

use super::Legs;
use crate::testing::generators::leaf_list::{
    arb_deep_version, arb_deep_version_pair, ONE_VERSION_DIVISOR, TWO_VERSION_DIVISOR,
};
use crate::testing::generators::regions::{arb_deep_party, arb_disjoint_parties};
use crate::testing::oracles::tree;
use crate::testing::property::divided_proptest;

/// Expands the descriptor-group roster into one deep-input proptest per group,
/// named by the roster's driver column inside this module.
///
/// A group with a novel signature refuses to compile until an arm says how to
/// feed it deep inputs.
macro_rules! deep_group_drivers {
    (args: (); $(($group:ident, $driver:ident, $shape:tt)),* $(,)?) => {
        $( deep_group_drivers!(@one $group, $driver, $shape); )*
    };
    (@one $group:ident, $driver:ident, (version)) => {
        divided_proptest! {
            #![divisor(ONE_VERSION_DIVISOR)]
            /// Every descriptor in the group agrees with the recursive oracle
            /// on deep versions.
            #[test]
            fn $driver(a in arb_deep_version()) {
                assert_diff_ops!(legs: Legs::TreeOracle, super::super::$group, &a.tree());
            }
        }
    };
    (@one $group:ident, $driver:ident, (version, version)) => {
        divided_proptest! {
            #![divisor(TWO_VERSION_DIVISOR)]
            /// Every descriptor in the group agrees with the recursive oracle
            /// on deep version pairs, the second independent of the first or
            /// derived from it.
            #[test]
            fn $driver((a, b) in arb_deep_version_pair()) {
                assert_diff_ops!(legs: Legs::TreeOracle, super::super::$group, &a.tree(), &b.tree());
            }
        }
    };
    (@one $group:ident, $driver:ident, (version, party)) => {
        divided_proptest! {
            #![divisor(ONE_VERSION_DIVISOR)]
            /// Every descriptor in the group agrees with the recursive oracle
            /// on a deep version viewed through an unrelated deep party.
            #[test]
            fn $driver(a in arb_deep_version(), p in arb_deep_party()) {
                assert_diff_ops!(legs: Legs::TreeOracle, super::super::$group, &a.tree(), &p);
            }
        }
    };
    (@one $group:ident, $driver:ident, (party)) => {
        proptest! {
            /// Every descriptor in the group agrees with the recursive oracle
            /// on deep, fragmented parties.
            #[test]
            fn $driver(p in arb_deep_party()) {
                assert_diff_ops!(legs: Legs::TreeOracle, super::super::$group, &p);
            }
        }
    };
    (@one $group:ident, $driver:ident, (party, party)) => {
        proptest! {
            /// Every descriptor in the group agrees with the recursive oracle
            /// on pairs of independent deep parties: disjoint, nested,
            /// overlapping, or equal.
            #[test]
            fn $driver(p in arb_deep_party(), q in arb_deep_party()) {
                assert_diff_ops!(legs: Legs::TreeOracle, super::super::$group, &p, &q);
            }
        }
    };
    (@one $group:ident, $driver:ident, (disjoint_party, disjoint_party)) => {
        proptest! {
            /// Every descriptor in the group agrees with the recursive oracle
            /// on two disjoint deep parties dealt from one cut set, in both
            /// operand orders.
            #[test]
            fn $driver(pair in arb_disjoint_parties(2)) {
                let [a, b] = <[tree::Party; 2]>::try_from(pair)
                    .expect("the strategy deals exactly two parties");
                prop_assert!(a.is_disjoint(&b), "dealt parties are disjoint by construction");
                assert_diff_ops!(legs: Legs::TreeOracle, super::super::$group, &a, &b);
                assert_diff_ops!(legs: Legs::TreeOracle, super::super::$group, &b, &a);
            }
        }
    };
    (@one $group:ident, $driver:ident, (clock)) => {
        divided_proptest! {
            #![divisor(ONE_VERSION_DIVISOR)]
            /// Every descriptor in the group agrees with the recursive oracle
            /// on a deep party paired with an unrelated deep version.
            #[test]
            fn $driver(p in arb_deep_party(), a in arb_deep_version()) {
                let c = tree::Clock::from_parts(p, a.tree());
                assert_diff_ops!(legs: Legs::TreeOracle, super::super::$group, &c);
            }
        }
    };
    (@one $group:ident, $driver:ident, (version, party, version)) => {
        divided_proptest! {
            #![divisor(TWO_VERSION_DIVISOR)]
            /// Every descriptor in the group agrees with the recursive oracle
            /// on a deep version viewed through an unrelated deep party and
            /// compared with a second deep version.
            #[test]
            fn $driver((a, b) in arb_deep_version_pair(), p in arb_deep_party()) {
                assert_diff_ops!(
                    legs: Legs::TreeOracle,
                    super::super::$group,
                    &a.tree(),
                    &p,
                    &b.tree()
                );
            }
        }
    };
    (@one $group:ident, $driver:ident, (version, party, version, party)) => {
        divided_proptest! {
            #![divisor(TWO_VERSION_DIVISOR)]
            /// Every descriptor in the group agrees with the recursive oracle
            /// on two deep versions, each viewed through its own unrelated
            /// deep party.
            #[test]
            fn $driver(
                (a, b) in arb_deep_version_pair(),
                p in arb_deep_party(),
                q in arb_deep_party(),
            ) {
                assert_diff_ops!(
                    legs: Legs::TreeOracle,
                    super::super::$group,
                    &a.tree(),
                    &p,
                    &b.tree(),
                    &q
                );
            }
        }
    };
}

for_each_diff_group!(deep_group_drivers);
