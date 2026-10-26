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
use crate::testing::generators::leaf_list::{
    arb_deep_version, arb_deep_version_family, arb_deep_version_pair, arb_deep_version_triple,
    ONE_VERSION_DIVISOR, THREE_VERSION_DIVISOR, TWO_VERSION_DIVISOR,
};
use crate::testing::generators::regions::{
    arb_deep_clock_family, arb_deep_party, arb_deep_party_family,
};
use crate::testing::oracles::tree;
use crate::{Clock, Party, Version};

/// Expands the law-group roster into one module of deep-input proptests per
/// group, one per law, named by the roster's driver column inside this module.
///
/// A group with a novel signature refuses to compile until an arm says how to
/// feed it deep inputs.
macro_rules! deep_group_drivers {
    (args: (); $(($group:ident, $driver:ident, $shape:tt)),* $(,)?) => {
        $( deep_group_drivers!(@one $group, $driver, $shape); )*
    };
    (@one $group:ident, $driver:ident, (version)) => {
        law_proptests! {
            /// Every law in the group holds on deep versions.
            mod $driver = $driver($group);
            divisor ONE_VERSION_DIVISOR;
            params (a in arb_deep_version());
            check |law| law(&ver(&a.tree()));
        }
    };
    (@one $group:ident, $driver:ident, (version, version)) => {
        law_proptests! {
            /// Every law in the group holds on deep version pairs, the
            /// second independent of the first or derived from it.
            mod $driver = $driver($group);
            divisor TWO_VERSION_DIVISOR;
            params ((a, b) in arb_deep_version_pair());
            check |law| law(&ver(&a.tree()), &ver(&b.tree()));
        }
    };
    (@one $group:ident, $driver:ident, (version, version, version)) => {
        law_proptests! {
            /// Every law in the group holds on deep version triples, the
            /// second and third each independent of the first or derived
            /// from it.
            mod $driver = $driver($group);
            divisor THREE_VERSION_DIVISOR;
            params ((a, b, c) in arb_deep_version_triple());
            check |law| law(&ver(&a.tree()), &ver(&b.tree()), &ver(&c.tree()));
        }
    };
    (@one $group:ident, $driver:ident, (party)) => {
        law_proptests! {
            /// Every law in the group holds on deep, fragmented parties.
            mod $driver = $driver($group);
            divisor 1;
            params (p in arb_deep_party());
            check |law| law(&party(&p));
        }
    };
    (@one $group:ident, $driver:ident, (party, party)) => {
        law_proptests! {
            /// Every law in the group holds on pairs of independent deep
            /// parties: disjoint, nested, overlapping, or equal.
            mod $driver = $driver($group);
            divisor 1;
            params (p in arb_deep_party(), q in arb_deep_party());
            check |law| law(&party(&p), &party(&q));
        }
    };
    (@one $group:ident, $driver:ident, (party, party, party)) => {
        law_proptests! {
            /// Every law in the group holds on triples of independent deep
            /// parties.
            mod $driver = $driver($group);
            divisor 1;
            params (p in arb_deep_party(), q in arb_deep_party(), r in arb_deep_party());
            check |law| law(&party(&p), &party(&q), &party(&r));
        }
    };
    (@one $group:ident, $driver:ident, (version, party)) => {
        law_proptests! {
            /// Every law in the group holds on a deep version paired with an
            /// unrelated deep party.
            mod $driver = $driver($group);
            divisor ONE_VERSION_DIVISOR;
            params (a in arb_deep_version(), p in arb_deep_party());
            check |law| law(&ver(&a.tree()), &party(&p));
        }
    };
    (@one $group:ident, $driver:ident, (version, version, party)) => {
        law_proptests! {
            /// Every law in the group holds on a deep version pair and an
            /// unrelated deep party.
            mod $driver = $driver($group);
            divisor TWO_VERSION_DIVISOR;
            params ((a, b) in arb_deep_version_pair(), p in arb_deep_party());
            check |law| law(&ver(&a.tree()), &ver(&b.tree()), &party(&p));
        }
    };
    (@one $group:ident, $driver:ident, (version, party, party)) => {
        law_proptests! {
            /// Every law in the group holds on a deep version and two
            /// unrelated deep parties.
            mod $driver = $driver($group);
            divisor ONE_VERSION_DIVISOR;
            params (a in arb_deep_version(), p in arb_deep_party(), q in arb_deep_party());
            check |law| law(&ver(&a.tree()), &party(&p), &party(&q));
        }
    };
    (@one $group:ident, $driver:ident, (version, version, party, party)) => {
        law_proptests! {
            /// Every law in the group holds on a deep version pair and two
            /// unrelated deep parties.
            mod $driver = $driver($group);
            divisor TWO_VERSION_DIVISOR;
            params ((a, b) in arb_deep_version_pair(), p in arb_deep_party(), q in arb_deep_party(),);
            check |law| law(&ver(&a.tree()), &ver(&b.tree()), &party(&p), &party(&q));
        }
    };
    (@one $group:ident, $driver:ident, (rank, rank, rank)) => {
        law_proptests! {
            /// Every law in the group holds on the ranks of a deep version
            /// pair and the distance between them: integers and fractions to
            /// thousands of bits, and equal ranks of distinct versions.
            mod $driver = $driver($group);
            divisor TWO_VERSION_DIVISOR;
            params ((a, b) in arb_deep_version_pair());
            check |law| {
                let (va, vb) = (ver(&a.tree()), ver(&b.tree()));
                law(&va.rank(), &vb.rank(), &va.distance(&vb))
            };
        }
    };
    (@one $group:ident, $driver:ident, (clock)) => {
        law_proptests! {
            /// Every law in the group holds on deep parties paired with
            /// unrelated deep versions.
            mod $driver = $driver($group);
            divisor ONE_VERSION_DIVISOR;
            params (p in arb_deep_party(), a in arb_deep_version());
            check |law| law(&Clock::from_parts(party(&p), ver(&a.tree())));
        }
    };
    (@one $group:ident, $driver:ident, (clock, clock)) => {
        law_proptests! {
            /// Every law in the group holds on two clocks built from
            /// independent deep parties and a deep version pair.
            mod $driver = $driver($group);
            divisor TWO_VERSION_DIVISOR;
            params (p in arb_deep_party(), q in arb_deep_party(), (a, b) in arb_deep_version_pair(),);
            check |law| law(&Clock::from_parts(party(&p), ver(&a.tree())), &Clock::from_parts(party(&q), ver(&b.tree())));
        }
    };
    (@one $group:ident, $driver:ident, (clock, version)) => {
        law_proptests! {
            /// Every law in the group holds on a deep clock receiving a deep
            /// message, the message independent of the clock's version or
            /// derived from it.
            mod $driver = $driver($group);
            divisor TWO_VERSION_DIVISOR;
            params (p in arb_deep_party(), (a, m) in arb_deep_version_pair());
            check |law| law(&Clock::from_parts(party(&p), ver(&a.tree())), &ver(&m.tree()));
        }
    };
    (@one $group:ident, $driver:ident, (versions)) => {
        law_proptests! {
            /// Every law in the group holds on pool-indexed lists of deep
            /// versions at the balanced fold's boundary arities.
            mod $driver = $driver($group);
            divisor TWO_VERSION_DIVISOR;
            params ((_, xs) in arb_deep_version_family());
            check |law| {
                let xs: Vec<Version> = xs.iter().map(ver).collect();
                law(&xs)
            };
        }
    };
    (@one $group:ident, $driver:ident, (version, versions)) => {
        law_proptests! {
            /// Every law in the group holds on pool-indexed families of deep
            /// versions, repeats and the empty version among them.
            mod $driver = $driver($group);
            divisor TWO_VERSION_DIVISOR;
            params ((r, xs) in arb_deep_version_family());
            check |law| {
                let receiver = ver(&r);
                let xs: Vec<Version> = xs.iter().map(ver).collect();
                law(&receiver, &xs)
            };
        }
    };
    (@one $group:ident, $driver:ident, (party, parties)) => {
        law_proptests! {
            /// Every law in the group holds on families of 2 to 10 deep
            /// parties, pairwise disjoint or overlapping in one inserted
            /// party, so both the accepting and the rejecting arm meet large
            /// families of unrelated parties.
            mod $driver = $driver($group);
            divisor 1;
            params ((r, items) in arb_deep_party_family());
            check |law| {
                let receiver = party(&r);
                let items: Vec<Party> = items.iter().map(party).collect();
                law(&receiver, &items)
            };
        }
    };
    (@one $group:ident, $driver:ident, (clock, clocks)) => {
        law_proptests! {
            /// Every law in the group holds on families of 2 to 10 clocks
            /// over deep parties, pairwise disjoint or overlapping in one
            /// inserted party, so `join_all` and `sync_all` accept and reject
            /// large families of unrelated clocks.
            mod $driver = $driver($group);
            divisor TWO_VERSION_DIVISOR;
            params ((r, items) in arb_deep_clock_family());
            check |law| {
                let clock = |(p, v): &(tree::Party, tree::Version)| {
                    Clock::from_parts(party(p), ver(v))
                };
                let receiver = clock(&r);
                let items: Vec<Clock> = items.iter().map(clock).collect();
                law(&receiver, &items)
            };
        }
    };
}

crate::for_each_law_group!(deep_group_drivers);
