//! The law drivers over deep inputs: every law group, fed versions and
//! parties tens to hundreds of levels deep.
//!
//! The arbitrary drivers stop at depth four. These drivers expand the same
//! roster (`crate::for_each_law_group!`) over the deep strategies in
//! [`crate::testing::generators::leaf_list`] and
//! [`crate::testing::generators::regions`]: versions to depth 300 with
//! heights to 2,200 bits, related pairs that are equal, mirrored, refined, or
//! tie exactly in rank, parties to depth 80, and families of up to ten
//! parties that are pairwise disjoint or overlap in one party. The laws need
//! no oracle, so depth costs them nothing but time.
//!
//! A case holding a deep version costs many arbitrary cases, so each such
//! driver runs the configured case count divided by a divisor that grows
//! with the versions its inputs hold.

use proptest::prelude::*;

use super::{party, ver};
use crate::testing::generators::divided_config;
use crate::testing::generators::leaf_list::{
    arb_deep_version, arb_deep_version_family, arb_deep_version_pair, arb_deep_version_triple,
    ONE_VERSION_DIVISOR, THREE_VERSION_DIVISOR, TWO_VERSION_DIVISOR,
};
use crate::testing::generators::regions::{
    arb_deep_clock_family, arb_deep_party, arb_deep_party_family,
};
use crate::testing::laws;
use crate::testing::oracles::tree;
use crate::{Clock, Party, Version};

/// Expands the law-group roster into one deep-input proptest per group,
/// named by the roster's driver column inside this module.
///
/// A group with a novel signature refuses to compile until an arm says how to
/// feed it deep inputs.
macro_rules! deep_group_drivers {
    (args: (); $(($group:ident, $driver:ident, $shape:tt)),* $(,)?) => {
        $( deep_group_drivers!(@one $group, $driver, $shape); )*
    };
    (@one $group:ident, $driver:ident, (version)) => {
        proptest! {
            #![proptest_config(divided_config(ONE_VERSION_DIVISOR))]
            /// Every law in the group holds on deep versions.
            #[test]
            fn $driver(a in arb_deep_version()) {
                assert_laws!(laws::$group, &ver(&a.tree()));
            }
        }
    };
    (@one $group:ident, $driver:ident, (version, version)) => {
        proptest! {
            #![proptest_config(divided_config(TWO_VERSION_DIVISOR))]
            /// Every law in the group holds on deep version pairs, the
            /// second independent of the first or derived from it.
            #[test]
            fn $driver((a, b) in arb_deep_version_pair()) {
                assert_laws!(laws::$group, &ver(&a.tree()), &ver(&b.tree()));
            }
        }
    };
    (@one $group:ident, $driver:ident, (version, version, version)) => {
        proptest! {
            #![proptest_config(divided_config(THREE_VERSION_DIVISOR))]
            /// Every law in the group holds on deep version triples, the
            /// second and third each independent of the first or derived
            /// from it.
            #[test]
            fn $driver((a, b, c) in arb_deep_version_triple()) {
                assert_laws!(laws::$group, &ver(&a.tree()), &ver(&b.tree()), &ver(&c.tree()));
            }
        }
    };
    (@one $group:ident, $driver:ident, (party)) => {
        proptest! {
            /// Every law in the group holds on deep, fragmented parties.
            #[test]
            fn $driver(p in arb_deep_party()) {
                assert_laws!(laws::$group, &party(&p));
            }
        }
    };
    (@one $group:ident, $driver:ident, (party, party)) => {
        proptest! {
            /// Every law in the group holds on pairs of independent deep
            /// parties: disjoint, nested, overlapping, or equal.
            #[test]
            fn $driver(p in arb_deep_party(), q in arb_deep_party()) {
                assert_laws!(laws::$group, &party(&p), &party(&q));
            }
        }
    };
    (@one $group:ident, $driver:ident, (party, party, party)) => {
        proptest! {
            /// Every law in the group holds on triples of independent deep
            /// parties.
            #[test]
            fn $driver(p in arb_deep_party(), q in arb_deep_party(), r in arb_deep_party()) {
                assert_laws!(laws::$group, &party(&p), &party(&q), &party(&r));
            }
        }
    };
    (@one $group:ident, $driver:ident, (version, party)) => {
        proptest! {
            #![proptest_config(divided_config(ONE_VERSION_DIVISOR))]
            /// Every law in the group holds on a deep version paired with an
            /// unrelated deep party.
            #[test]
            fn $driver(a in arb_deep_version(), p in arb_deep_party()) {
                assert_laws!(laws::$group, &ver(&a.tree()), &party(&p));
            }
        }
    };
    (@one $group:ident, $driver:ident, (version, version, party)) => {
        proptest! {
            #![proptest_config(divided_config(TWO_VERSION_DIVISOR))]
            /// Every law in the group holds on a deep version pair and an
            /// unrelated deep party.
            #[test]
            fn $driver((a, b) in arb_deep_version_pair(), p in arb_deep_party()) {
                assert_laws!(laws::$group, &ver(&a.tree()), &ver(&b.tree()), &party(&p));
            }
        }
    };
    (@one $group:ident, $driver:ident, (version, party, party)) => {
        proptest! {
            #![proptest_config(divided_config(ONE_VERSION_DIVISOR))]
            /// Every law in the group holds on a deep version and two
            /// unrelated deep parties.
            #[test]
            fn $driver(a in arb_deep_version(), p in arb_deep_party(), q in arb_deep_party()) {
                assert_laws!(laws::$group, &ver(&a.tree()), &party(&p), &party(&q));
            }
        }
    };
    (@one $group:ident, $driver:ident, (version, version, party, party)) => {
        proptest! {
            #![proptest_config(divided_config(TWO_VERSION_DIVISOR))]
            /// Every law in the group holds on a deep version pair and two
            /// unrelated deep parties.
            #[test]
            fn $driver(
                (a, b) in arb_deep_version_pair(),
                p in arb_deep_party(),
                q in arb_deep_party(),
            ) {
                assert_laws!(
                    laws::$group,
                    &ver(&a.tree()),
                    &ver(&b.tree()),
                    &party(&p),
                    &party(&q)
                );
            }
        }
    };
    (@one $group:ident, $driver:ident, (rank, rank, rank)) => {
        proptest! {
            #![proptest_config(divided_config(TWO_VERSION_DIVISOR))]
            /// Every law in the group holds on the ranks of a deep version
            /// pair and the distance between them: integers and fractions to
            /// thousands of bits, and equal ranks of distinct versions.
            #[test]
            fn $driver((a, b) in arb_deep_version_pair()) {
                let (va, vb) = (ver(&a.tree()), ver(&b.tree()));
                assert_laws!(laws::$group, &va.rank(), &vb.rank(), &va.distance(&vb));
            }
        }
    };
    (@one $group:ident, $driver:ident, (clock)) => {
        proptest! {
            #![proptest_config(divided_config(ONE_VERSION_DIVISOR))]
            /// Every law in the group holds on deep parties paired with
            /// unrelated deep versions.
            #[test]
            fn $driver(p in arb_deep_party(), a in arb_deep_version()) {
                assert_laws!(laws::$group, &Clock::from_parts(party(&p), ver(&a.tree())));
            }
        }
    };
    (@one $group:ident, $driver:ident, (clock, clock)) => {
        proptest! {
            #![proptest_config(divided_config(TWO_VERSION_DIVISOR))]
            /// Every law in the group holds on two clocks built from
            /// independent deep parties and a deep version pair.
            #[test]
            fn $driver(
                p in arb_deep_party(),
                q in arb_deep_party(),
                (a, b) in arb_deep_version_pair(),
            ) {
                assert_laws!(
                    laws::$group,
                    &Clock::from_parts(party(&p), ver(&a.tree())),
                    &Clock::from_parts(party(&q), ver(&b.tree()))
                );
            }
        }
    };
    (@one $group:ident, $driver:ident, (clock, version)) => {
        proptest! {
            #![proptest_config(divided_config(TWO_VERSION_DIVISOR))]
            /// Every law in the group holds on a deep clock receiving a deep
            /// message, the message independent of the clock's version or
            /// derived from it.
            #[test]
            fn $driver(p in arb_deep_party(), (a, m) in arb_deep_version_pair()) {
                assert_laws!(
                    laws::$group,
                    &Clock::from_parts(party(&p), ver(&a.tree())),
                    &ver(&m.tree())
                );
            }
        }
    };
    (@one $group:ident, $driver:ident, (versions)) => {
        proptest! {
            #![proptest_config(divided_config(TWO_VERSION_DIVISOR))]
            /// Every law in the group holds on pool-indexed lists of deep
            /// versions at the balanced fold's boundary arities.
            #[test]
            fn $driver((_, xs) in arb_deep_version_family()) {
                let xs: Vec<Version> = xs.iter().map(ver).collect();
                assert_laws!(laws::$group, &xs);
            }
        }
    };
    (@one $group:ident, $driver:ident, (version, versions)) => {
        proptest! {
            #![proptest_config(divided_config(TWO_VERSION_DIVISOR))]
            /// Every law in the group holds on pool-indexed families of deep
            /// versions, repeats and the empty version among them.
            #[test]
            fn $driver((r, xs) in arb_deep_version_family()) {
                let receiver = ver(&r);
                let xs: Vec<Version> = xs.iter().map(ver).collect();
                assert_laws!(laws::$group, &receiver, &xs);
            }
        }
    };
    (@one $group:ident, $driver:ident, (party, parties)) => {
        proptest! {
            /// Every law in the group holds on families of 2 to 10 deep
            /// parties, pairwise disjoint or overlapping in one inserted
            /// party, so both the accepting and the rejecting arm meet large
            /// families of unrelated parties.
            #[test]
            fn $driver((r, items) in arb_deep_party_family()) {
                let receiver = party(&r);
                let items: Vec<Party> = items.iter().map(party).collect();
                assert_laws!(laws::$group, &receiver, &items);
            }
        }
    };
    (@one $group:ident, $driver:ident, (clock, clocks)) => {
        proptest! {
            #![proptest_config(divided_config(TWO_VERSION_DIVISOR))]
            /// Every law in the group holds on families of 2 to 10 clocks
            /// over deep parties, pairwise disjoint or overlapping in one
            /// inserted party, so `join_all` and `sync_all` accept and reject
            /// large families of unrelated clocks.
            #[test]
            fn $driver((r, items) in arb_deep_clock_family()) {
                let clock = |(p, v): &(tree::Party, tree::Version)| {
                    Clock::from_parts(party(p), ver(v))
                };
                let receiver = clock(&r);
                let items: Vec<Clock> = items.iter().map(clock).collect();
                assert_laws!(laws::$group, &receiver, &items);
            }
        }
    };
}

crate::for_each_law_group!(deep_group_drivers);
