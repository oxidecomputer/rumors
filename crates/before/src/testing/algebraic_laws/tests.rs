//! The law drivers: one generic proptest per signature group, iterating
//! [`crate::laws`]' slices so every assertion names the law it checks.
//!
//! Both driver genres — the per-group proptests and the
//! organic-populations drive list — expand from the law-group roster
//! (`crate::for_each_law_group!`), so every registered group is driven
//! here by construction. Each driver's doc comment states the
//! meta-invariant; the individual laws' statements live with their
//! predicates in [`crate::laws`]. No assertion's right-hand side mentions
//! the oracle: the laws hold by the ITC algebra, so they catch a defect
//! the impl and the recursive oracle would share.

use proptest::prelude::*;

use crate::laws;
use crate::oracle;
use crate::testing::generators::{
    arb_clock_family, arb_fold_arity, arb_oracle_party_nonempty, arb_oracle_version,
    arb_party_family, arb_version_family,
};
use crate::testing::optrace::{run, step_impl, world_strategy};
use crate::{Clock, Party, Rank, Version};

/// Build a fresh impl `Version` from an oracle source tree. The oracle tree
/// is only a carrier of canonical bits here.
fn ver(o: &oracle::Version) -> Version {
    crate::testing::bridge::from_oracle_version(o)
}

/// Build a fresh impl `Party` from an oracle source tree. `Party` is
/// `!Clone`, so every use that consumes or borrows a party rebuilds one from
/// its (cheap, `Clone`) oracle source.
fn party(o: &oracle::Party) -> Party {
    crate::testing::bridge::from_oracle_party(o)
}

/// Assert every law in a slice, naming the violated law on failure.
macro_rules! assert_laws {
    ($group:expr, $($input:expr),+) => {
        for (name, law) in $group {
            prop_assert!(law($($input),+), "law violated: {}", name);
        }
    };
}

// ───────────────────── arbitrary normal-form inputs ─────────────────────

/// Expands the law-group roster into the per-group drivers: one proptest
/// per group, named by the roster's driver column, keyed on the group's
/// input signature, feeding it the arbitrary normal-form generators.
///
/// The arms carry each signature's input regime (and its doc comment);
/// the *list* of groups lives only in `crate::for_each_law_group!`, so a
/// group added to the roster with a known signature is driven here with
/// no further wiring, and one with a novel signature refuses to compile
/// until an arm says how to feed it.
macro_rules! group_drivers {
    (args: (); $(($group:ident, $driver:ident, $shape:tt)),* $(,)?) => {
        $( group_drivers!(@one $group, $driver, $shape); )*
    };
    (@one $group:ident, $driver:ident, (version)) => {
        proptest! {
            /// Every law in the group holds on arbitrary normal-form
            /// versions, including large-base events.
            #[test]
            fn $driver(a in arb_oracle_version()) {
                assert_laws!(laws::$group, &ver(&a));
            }
        }
    };
    (@one $group:ident, $driver:ident, (version, version)) => {
        proptest! {
            /// Every law in the group holds on arbitrary normal-form
            /// version pairs.
            #[test]
            fn $driver(a in arb_oracle_version(), b in arb_oracle_version()) {
                assert_laws!(laws::$group, &ver(&a), &ver(&b));
            }
        }
    };
    (@one $group:ident, $driver:ident, (version, version, version)) => {
        proptest! {
            /// Every law in the group holds on arbitrary normal-form
            /// version triples.
            #[test]
            fn $driver(
                a in arb_oracle_version(),
                b in arb_oracle_version(),
                c in arb_oracle_version(),
            ) {
                assert_laws!(laws::$group, &ver(&a), &ver(&b), &ver(&c));
            }
        }
    };
    (@one $group:ident, $driver:ident, (party)) => {
        proptest! {
            /// Every law in the group holds on arbitrary non-empty
            /// normal-form ids.
            #[test]
            fn $driver(p in arb_oracle_party_nonempty()) {
                assert_laws!(laws::$group, &party(&p));
            }
        }
    };
    (@one $group:ident, $driver:ident, (party, party)) => {
        proptest! {
            /// Every law in the group holds on arbitrary non-empty id
            /// pairs — typically unrelated and frequently overlapping,
            /// shapes the op pipeline never produces.
            #[test]
            fn $driver(p in arb_oracle_party_nonempty(), q in arb_oracle_party_nonempty()) {
                assert_laws!(laws::$group, &party(&p), &party(&q));
            }
        }
    };
    (@one $group:ident, $driver:ident, (party, party, party)) => {
        proptest! {
            /// Every law in the group holds on arbitrary non-empty id
            /// triples.
            #[test]
            fn $driver(
                p in arb_oracle_party_nonempty(),
                q in arb_oracle_party_nonempty(),
                r in arb_oracle_party_nonempty(),
            ) {
                assert_laws!(laws::$group, &party(&p), &party(&q), &party(&r));
            }
        }
    };
    (@one $group:ident, $driver:ident, (version, party)) => {
        proptest! {
            /// Every law in the group holds on arbitrary version/id
            /// pairings — the tick and projection laws on regions
            /// unrelated to the history they act on.
            #[test]
            fn $driver(a in arb_oracle_version(), p in arb_oracle_party_nonempty()) {
                assert_laws!(laws::$group, &ver(&a), &party(&p));
            }
        }
    };
    (@one $group:ident, $driver:ident, (version, version, party)) => {
        proptest! {
            /// Every law in the group holds on arbitrary version-pair/id
            /// combinations.
            #[test]
            fn $driver(
                a in arb_oracle_version(),
                b in arb_oracle_version(),
                p in arb_oracle_party_nonempty(),
            ) {
                assert_laws!(laws::$group, &ver(&a), &ver(&b), &party(&p));
            }
        }
    };
    (@one $group:ident, $driver:ident, (version, party, party)) => {
        proptest! {
            /// Every law in the group holds on arbitrary version/id-pair
            /// combinations.
            #[test]
            fn $driver(
                a in arb_oracle_version(),
                p in arb_oracle_party_nonempty(),
                q in arb_oracle_party_nonempty(),
            ) {
                assert_laws!(laws::$group, &ver(&a), &party(&p), &party(&q));
            }
        }
    };
    (@one $group:ident, $driver:ident, (version, version, party, party)) => {
        proptest! {
            /// Every law in the group holds on arbitrary
            /// version-pair/id-pair combinations.
            #[test]
            fn $driver(
                a in arb_oracle_version(),
                b in arb_oracle_version(),
                p in arb_oracle_party_nonempty(),
                q in arb_oracle_party_nonempty(),
            ) {
                assert_laws!(laws::$group, &ver(&a), &ver(&b), &party(&p), &party(&q));
            }
        }
    };
    (@one $group:ident, $driver:ident, (rank, rank, rank)) => {
        proptest! {
            /// Every law in the group holds on ranks derived from
            /// arbitrary normal-form versions (their own ranks and a
            /// genuine distance) — organically related magnitudes.
            ///
            /// The adversarial spilled-magnitude regime is driven where
            /// the rank machinery lives, in the version suite's rank
            /// driver.
            #[test]
            fn $driver(a in arb_oracle_version(), b in arb_oracle_version()) {
                let (va, vb) = (ver(&a), ver(&b));
                let (ra, rb, rc) = (va.rank(), vb.rank(), va.distance(&vb));
                assert_laws!(laws::$group, &ra, &rb, &rc);
            }
        }
    };
    (@one $group:ident, $driver:ident, (clock)) => {
        proptest! {
            /// Every law in the group holds on arbitrary canonical
            /// party/version pairings — every such pairing is a valid
            /// clock, including ones no op sequence reaches.
            #[test]
            fn $driver(p in arb_oracle_party_nonempty(), a in arb_oracle_version()) {
                assert_laws!(laws::$group, &Clock::from_parts(party(&p), ver(&a)));
            }
        }
    };
    (@one $group:ident, $driver:ident, (clock, version)) => {
        proptest! {
            /// Every law in the group holds on arbitrary clocks paired
            /// with arbitrary (typically concurrent, unrelated) messages.
            #[test]
            fn $driver(
                p in arb_oracle_party_nonempty(),
                a in arb_oracle_version(),
                m in arb_oracle_version(),
            ) {
                assert_laws!(laws::$group, &Clock::from_parts(party(&p), ver(&a)), &ver(&m));
            }
        }
    };
    (@one $group:ident, $driver:ident, (versions)) => {
        proptest! {
            /// Every law in the group holds on pool-indexed version
            /// lists whose arities sweep the balanced fold's boundary
            /// band (`arb_fold_arity` documents the derivation).
            #[test]
            fn $driver((_, xs) in arb_version_family()) {
                let xs: Vec<Version> = xs.iter().map(ver).collect();
                assert_laws!(laws::$group, &xs);
            }
        }
    };
    (@one $group:ident, $driver:ident, (version, versions)) => {
        proptest! {
            /// Every law in the group holds on pool-indexed version
            /// families — a receiver plus boundary-swept items, repeats
            /// and the empty version under mass at every arity.
            #[test]
            fn $driver((r, xs) in arb_version_family()) {
                let receiver = ver(&r);
                let xs: Vec<Version> = xs.iter().map(ver).collect();
                assert_laws!(laws::$group, &receiver, &xs);
            }
        }
    };
    (@one $group:ident, $driver:ident, (party, parties)) => {
        proptest! {
            /// Every law in the group holds on pool-indexed party
            /// families — aliased repeats keep the refusal arm under
            /// mass, the constructed laws' fork trees the accepted arm,
            /// at every boundary-band arity.
            #[test]
            fn $driver((r, items) in arb_party_family()) {
                let receiver = party(&r);
                let items: Vec<Party> = items.iter().map(party).collect();
                assert_laws!(laws::$group, &receiver, &items);
            }
        }
    };
    (@one $group:ident, $driver:ident, (clock, clocks)) => {
        proptest! {
            /// Every law in the group holds on pool-indexed clock
            /// families — arbitrary canonical party/version pairings,
            /// aliased parties under mass, at every boundary-band arity.
            #[test]
            fn $driver((r, items) in arb_clock_family()) {
                let clock = |(p, v): &(oracle::Party, oracle::Version)| {
                    Clock::from_parts(party(p), ver(v))
                };
                let receiver = clock(&r);
                let items: Vec<Clock> = items.iter().map(clock).collect();
                assert_laws!(laws::$group, &receiver, &items);
            }
        }
    };
}

crate::for_each_law_group!(group_drivers);

// ───────────────────── organic op-trace populations ─────────────────────

/// One organic population's picks, borrowed: the pools the
/// roster-derived organic drive list selects each group's inputs from.
struct Organic<'a> {
    /// Three organically related versions from the trace.
    v: [&'a Version; 3],
    /// Three live parties from the same trace.
    p: [&'a Party; 3],
    /// Ranks derived from the versions: two own ranks and a genuine
    /// distance.
    r: [&'a Rank; 3],
    /// One reachable clock, replayed from the trace on the impl.
    c: &'a Clock,
    /// Pool-indexed lists at fold-boundary arities. Repeated picks are
    /// repeated raw versions and *aliased* live parties/clocks — the
    /// input classes the list laws' fold and refusal arms exist for.
    versions: &'a [Version],
    parties: &'a [Party],
    clocks: &'a [Clock],
}

/// Expands the law-group roster into the organic drive list: one
/// `assert_laws!` per group, keyed on the group's input signature,
/// selecting that signature's inputs from an [`Organic`] environment.
///
/// The group list lives only in `crate::for_each_law_group!`; a roster
/// signature without an arm here refuses to compile.
macro_rules! organic_drive {
    (args: ($env:expr); $(($group:ident, $driver:ident, $shape:tt)),* $(,)?) => {
        $( organic_drive!(@one $env, $group, $shape); )*
    };
    (@one $env:expr, $group:ident, (version)) => {
        assert_laws!(laws::$group, $env.v[0]);
    };
    (@one $env:expr, $group:ident, (version, version)) => {
        assert_laws!(laws::$group, $env.v[0], $env.v[1]);
    };
    (@one $env:expr, $group:ident, (version, version, version)) => {
        assert_laws!(laws::$group, $env.v[0], $env.v[1], $env.v[2]);
    };
    (@one $env:expr, $group:ident, (party)) => {
        assert_laws!(laws::$group, $env.p[0]);
    };
    (@one $env:expr, $group:ident, (party, party)) => {
        assert_laws!(laws::$group, $env.p[0], $env.p[1]);
    };
    (@one $env:expr, $group:ident, (party, party, party)) => {
        assert_laws!(laws::$group, $env.p[0], $env.p[1], $env.p[2]);
    };
    (@one $env:expr, $group:ident, (version, party)) => {
        assert_laws!(laws::$group, $env.v[0], $env.p[1]);
    };
    (@one $env:expr, $group:ident, (version, version, party)) => {
        assert_laws!(laws::$group, $env.v[0], $env.v[1], $env.p[2]);
    };
    (@one $env:expr, $group:ident, (version, party, party)) => {
        assert_laws!(laws::$group, $env.v[0], $env.p[1], $env.p[2]);
    };
    (@one $env:expr, $group:ident, (version, version, party, party)) => {
        assert_laws!(laws::$group, $env.v[0], $env.v[1], $env.p[1], $env.p[2]);
    };
    (@one $env:expr, $group:ident, (rank, rank, rank)) => {
        assert_laws!(laws::$group, $env.r[0], $env.r[1], $env.r[2]);
    };
    (@one $env:expr, $group:ident, (clock)) => {
        assert_laws!(laws::$group, $env.c);
    };
    (@one $env:expr, $group:ident, (clock, version)) => {
        assert_laws!(laws::$group, $env.c, $env.v[1]);
    };
    (@one $env:expr, $group:ident, (versions)) => {
        assert_laws!(laws::$group, $env.versions);
    };
    (@one $env:expr, $group:ident, (version, versions)) => {
        assert_laws!(laws::$group, $env.v[0], $env.versions);
    };
    (@one $env:expr, $group:ident, (party, parties)) => {
        assert_laws!(laws::$group, $env.p[0], $env.parties);
    };
    (@one $env:expr, $group:ident, (clock, clocks)) => {
        assert_laws!(laws::$group, $env.c, $env.clocks);
    };
}

proptest! {
    /// The whole law collection holds over organic op-trace populations.
    ///
    /// The same slices the arbitrary-normal-form drivers iterate, landed on
    /// the value shapes real fork/tick/join/sync schedules produce (live
    /// sibling parties, causally related versions, reachable clocks).
    #[test]
    fn laws_hold_on_organic_populations(
        ops in world_strategy(),
        i in 0usize..64,
        j in 0usize..64,
        k in 0usize..64,
        list_picks in arb_fold_arity()
            .prop_flat_map(|arity| proptest::collection::vec(0usize..64, arity)),
    ) {
        let cs = run(&ops);
        let n = cs.len();
        let picks = [i % n, j % n, k % n];
        let (pa, va) = cs[picks[0]].trees();
        let (pb, vb) = cs[picks[1]].trees();
        let (pc, vc) = cs[picks[2]].trees();
        let (ia, ib, ic) = (ver(va), ver(vb), ver(vc));
        let (qa, qb, qc) = (party(pa), party(pb), party(pc));
        let (ra, rb, rc) = (ia.rank(), ib.rank(), ia.distance(&ib));

        // Variadic families over the same population: pool-indexed picks
        // at fold-boundary arities.
        let vlist: Vec<Version> = list_picks.iter().map(|t| ver(cs[t % n].trees().1)).collect();
        let plist: Vec<Party> = list_picks.iter().map(|t| party(cs[t % n].trees().0)).collect();

        // Clocks: replay the trace on the impl for real, reachable clocks.
        let mut imp = vec![Clock::seed()];
        for op in &ops {
            step_impl(&mut imp, op);
        }
        let ca = &imp[picks[0] % imp.len()];
        let clist: Vec<Clock> = list_picks
            .iter()
            .map(|t| imp[t % imp.len()].dangerously_alias())
            .collect();

        let organic = Organic {
            v: [&ia, &ib, &ic],
            p: [&qa, &qb, &qc],
            r: [&ra, &rb, &rc],
            c: ca,
            versions: &vlist,
            parties: &plist,
            clocks: &clist,
        };
        crate::for_each_law_group!(organic_drive(&organic));
    }
}
