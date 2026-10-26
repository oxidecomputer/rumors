//! The law drivers: one proptest per law in [`crate::testing::laws`], for each
//! input regime, named for the law it checks.
//!
//! A driver per signature group expands each group's law roster into a module
//! named for the group, holding one test per law. Each test draws its own
//! cases, so every law runs the full configured case count, and nextest runs
//! the laws of one group in parallel.
//!
//! Every driver expands from the law-group roster
//! (`crate::for_each_law_group!`): the per-law proptests over arbitrary
//! normal forms, the organic-populations drive list, the per-law proptests
//! over deep inputs in [`deep`], those over multi-scale pairs in
//! [`multiscale`], which take the groups over one version, over two, and over
//! ranks, and those over co-generated tick cases in [`co_generated`], which
//! take the groups over one version and one party. So every registered group
//! is driven here by construction. Each driver's doc comment states the
//! meta-invariant; the individual laws' statements live with their predicates
//! in [`crate::testing::laws`]. No assertion's right-hand side mentions the
//! oracle: the laws hold by the ITC algebra, so they catch a defect the impl
//! and the recursive oracle would share.

use proptest::prelude::*;

use crate::testing::generators::{
    arb_clock_family, arb_fold_arity, arb_oracle_party_nonempty, arb_oracle_version,
    arb_party_family, arb_version_family,
};
use crate::testing::laws::{self, Law};
use crate::testing::optrace::{run, step, world_strategy, Production};
use crate::testing::oracles::tree;
use crate::{Clock, Party, Rank, Version};

/// Build a fresh impl `Version` from an oracle source tree. The oracle tree
/// is only a carrier of canonical bits here.
fn ver(o: &tree::Version) -> Version {
    crate::testing::bridge::from_oracle_version(o)
}

/// Build a fresh impl `Party` from an oracle source tree. `Party` is
/// `!Clone`, so every use that consumes or borrows a party rebuilds one from
/// its (cheap, `Clone`) oracle source.
fn party(o: &tree::Party) -> Party {
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

/// Whether the law registered as `name` in `group` holds, given the inputs
/// `check` feeds it.
///
/// Taking the group first fixes the law's type before `check` is inferred, so
/// a driver's check closure needs no annotation.
fn holds<F: Copy>(group: &[Law<F>], name: &str, check: impl FnOnce(F) -> bool) -> bool {
    let (_, law) = group
        .iter()
        .find(|(registered, _)| *registered == name)
        .expect("each law test is named for a law its group's roster lists");
    check(*law)
}

/// Expands one property test per law in a group, inside a module of its own.
///
/// `mod $module = $roster($group)` names the module and the group, whose
/// roster macro supplies the law names; each test is named for its law.
/// `divisor` divides the configured case count, as
/// [`crate::testing::property::run`] does, `params` binds the strategies every
/// test draws from, and `check` is a closure that feeds one law its inputs.
/// Each test draws its own cases, so every law runs the full divided count,
/// and nextest runs the laws of one group in parallel.
macro_rules! law_proptests {
    (
        $(#[$doc:meta])*
        mod $module:ident = $roster:ident($group:ident);
        divisor $divisor:expr;
        params ($($parm:pat in $strategy:expr),+ $(,)?);
        check |$law_param:ident| $check:expr;
    ) => {
        $(#[$doc])*
        mod $module {
            use super::*;

            /// Runs the property of the law registered as `name` over this
            /// module's inputs.
            ///
            /// Every law test in the module calls this one function, so the
            /// strategy and the runner compile once per module rather than
            /// once per law.
            fn check_law(name: &'static str, test_name: &'static str) {
                crate::testing::property::run(
                    $divisor,
                    test_name,
                    file!(),
                    ($($strategy,)+),
                    |($($parm,)+)| {
                        prop_assert!(
                            crate::testing::algebraic_laws::tests::holds(
                                crate::testing::laws::$group,
                                name,
                                |$law_param| $check,
                            ),
                            "law violated: {}",
                            name
                        );
                        Ok(())
                    },
                );
            }

            crate::testing::laws::$roster!(law_proptests(@tests));
        }
    };
    (@tests; $($law:ident),+) => {
        $(
            /// The law this test is named for holds on the inputs its module
            /// describes.
            #[test]
            fn $law() {
                check_law(stringify!($law), concat!(module_path!(), "::", stringify!($law)));
            }
        )+
    };
}

// ───────────────────── arbitrary normal-form inputs ─────────────────────

/// Expands the law-group roster into one module per group, named by the
/// roster's driver column, holding one proptest per law over the arbitrary
/// normal-form generators for the group's input signature.
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
        law_proptests! {
            /// Every law in the group holds on arbitrary normal-form
            /// versions, including large-base events.
            mod $driver = $driver($group);
            divisor 1;
            params (a in arb_oracle_version());
            check |law| law(&ver(&a));
        }
    };
    (@one $group:ident, $driver:ident, (version, version)) => {
        law_proptests! {
            /// Every law in the group holds on arbitrary normal-form
            /// version pairs.
            mod $driver = $driver($group);
            divisor 1;
            params (a in arb_oracle_version(), b in arb_oracle_version());
            check |law| law(&ver(&a), &ver(&b));
        }
    };
    (@one $group:ident, $driver:ident, (version, version, version)) => {
        law_proptests! {
            /// Every law in the group holds on arbitrary normal-form
            /// version triples.
            mod $driver = $driver($group);
            divisor 1;
            params (a in arb_oracle_version(), b in arb_oracle_version(), c in arb_oracle_version(),);
            check |law| law(&ver(&a), &ver(&b), &ver(&c));
        }
    };
    (@one $group:ident, $driver:ident, (party)) => {
        law_proptests! {
            /// Every law in the group holds on arbitrary non-empty
            /// normal-form ids.
            mod $driver = $driver($group);
            divisor 1;
            params (p in arb_oracle_party_nonempty());
            check |law| law(&party(&p));
        }
    };
    (@one $group:ident, $driver:ident, (party, party)) => {
        law_proptests! {
            /// Every law in the group holds on arbitrary non-empty id
            /// pairs, typically unrelated and frequently overlapping,
            /// shapes the op pipeline never produces.
            mod $driver = $driver($group);
            divisor 1;
            params (p in arb_oracle_party_nonempty(), q in arb_oracle_party_nonempty());
            check |law| law(&party(&p), &party(&q));
        }
    };
    (@one $group:ident, $driver:ident, (party, party, party)) => {
        law_proptests! {
            /// Every law in the group holds on arbitrary non-empty id
            /// triples.
            mod $driver = $driver($group);
            divisor 1;
            params (p in arb_oracle_party_nonempty(), q in arb_oracle_party_nonempty(), r in arb_oracle_party_nonempty(),);
            check |law| law(&party(&p), &party(&q), &party(&r));
        }
    };
    (@one $group:ident, $driver:ident, (version, party)) => {
        law_proptests! {
            /// Every law in the group holds on arbitrary version/id
            /// pairings: the tick and projection laws on regions
            /// unrelated to the history they act on.
            mod $driver = $driver($group);
            divisor 1;
            params (a in arb_oracle_version(), p in arb_oracle_party_nonempty());
            check |law| law(&ver(&a), &party(&p));
        }
    };
    (@one $group:ident, $driver:ident, (version, version, party)) => {
        law_proptests! {
            /// Every law in the group holds on arbitrary version-pair/id
            /// combinations.
            mod $driver = $driver($group);
            divisor 1;
            params (a in arb_oracle_version(), b in arb_oracle_version(), p in arb_oracle_party_nonempty(),);
            check |law| law(&ver(&a), &ver(&b), &party(&p));
        }
    };
    (@one $group:ident, $driver:ident, (version, party, party)) => {
        law_proptests! {
            /// Every law in the group holds on arbitrary version/id-pair
            /// combinations.
            mod $driver = $driver($group);
            divisor 1;
            params (a in arb_oracle_version(), p in arb_oracle_party_nonempty(), q in arb_oracle_party_nonempty(),);
            check |law| law(&ver(&a), &party(&p), &party(&q));
        }
    };
    (@one $group:ident, $driver:ident, (version, version, party, party)) => {
        law_proptests! {
            /// Every law in the group holds on arbitrary
            /// version-pair/id-pair combinations.
            mod $driver = $driver($group);
            divisor 1;
            params (a in arb_oracle_version(), b in arb_oracle_version(), p in arb_oracle_party_nonempty(), q in arb_oracle_party_nonempty(),);
            check |law| law(&ver(&a), &ver(&b), &party(&p), &party(&q));
        }
    };
    (@one $group:ident, $driver:ident, (rank, rank, rank)) => {
        law_proptests! {
            /// Every law in the group holds on ranks derived from
            /// arbitrary normal-form versions (their own ranks and a
            /// genuine distance), organically related magnitudes.
            ///
            /// The adversarial spilled-magnitude regime is driven where
            /// the rank machinery lives, in the version suite's rank
            /// driver.
            mod $driver = $driver($group);
            divisor 1;
            params (a in arb_oracle_version(), b in arb_oracle_version());
            check |law| {
                let (va, vb) = (ver(&a), ver(&b));
                let (ra, rb, rc) = (va.rank(), vb.rank(), va.distance(&vb));
                law(&ra, &rb, &rc)
            };
        }
    };
    (@one $group:ident, $driver:ident, (clock)) => {
        law_proptests! {
            /// Every law in the group holds on arbitrary canonical
            /// party/version pairings; every such pairing is a valid
            /// clock, including ones no op sequence reaches.
            mod $driver = $driver($group);
            divisor 1;
            params (p in arb_oracle_party_nonempty(), a in arb_oracle_version());
            check |law| law(&Clock::from_parts(party(&p), ver(&a)));
        }
    };
    (@one $group:ident, $driver:ident, (clock, clock)) => {
        law_proptests! {
            /// Every law in the group holds on pairs of arbitrary canonical
            /// party/version pairings, typically unrelated stamps sharing
            /// neither region nor history, the regime where a non-injective
            /// encoding boundary would hide.
            mod $driver = $driver($group);
            divisor 1;
            params (p in arb_oracle_party_nonempty(), a in arb_oracle_version(), q in arb_oracle_party_nonempty(), b in arb_oracle_version(),);
            check |law| law(&Clock::from_parts(party(&p), ver(&a)), &Clock::from_parts(party(&q), ver(&b)));
        }
    };
    (@one $group:ident, $driver:ident, (clock, version)) => {
        law_proptests! {
            /// Every law in the group holds on arbitrary clocks paired
            /// with arbitrary (typically concurrent, unrelated) messages.
            mod $driver = $driver($group);
            divisor 1;
            params (p in arb_oracle_party_nonempty(), a in arb_oracle_version(), m in arb_oracle_version(),);
            check |law| law(&Clock::from_parts(party(&p), ver(&a)), &ver(&m));
        }
    };
    (@one $group:ident, $driver:ident, (versions)) => {
        law_proptests! {
            /// Every law in the group holds on pool-indexed version
            /// lists whose arities sweep the balanced fold's boundary
            /// band (`arb_fold_arity` documents the derivation).
            mod $driver = $driver($group);
            divisor 1;
            params ((_, xs) in arb_version_family());
            check |law| {
                let xs: Vec<Version> = xs.iter().map(ver).collect();
                law(&xs)
            };
        }
    };
    (@one $group:ident, $driver:ident, (version, versions)) => {
        law_proptests! {
            /// Every law in the group holds on pool-indexed version
            /// families: a receiver plus boundary-swept items, repeats
            /// and the empty version under mass at every arity.
            mod $driver = $driver($group);
            divisor 1;
            params ((r, xs) in arb_version_family());
            check |law| {
                let receiver = ver(&r);
                let xs: Vec<Version> = xs.iter().map(ver).collect();
                law(&receiver, &xs)
            };
        }
    };
    (@one $group:ident, $driver:ident, (party, parties)) => {
        law_proptests! {
            /// Every law in the group holds on pool-indexed party
            /// families: aliased repeats keep the refusal arm under
            /// mass, the constructed laws' fork trees the accepted arm,
            /// at every boundary-band arity.
            mod $driver = $driver($group);
            divisor 1;
            params ((r, items) in arb_party_family());
            check |law| {
                let receiver = party(&r);
                let items: Vec<Party> = items.iter().map(party).collect();
                law(&receiver, &items)
            };
        }
    };
    (@one $group:ident, $driver:ident, (clock, clocks)) => {
        law_proptests! {
            /// Every law in the group holds on clock families picked
            /// from a pool or dealt disjoint shares, at every
            /// boundary-band arity.
            ///
            /// Picked families pair arbitrary canonical parties and
            /// versions and alias parties anywhere; dealt families are
            /// disjoint lists, whole or with one overlap after a disjoint
            /// prefix.
            mod $driver = $driver($group);
            divisor 1;
            params ((r, items) in arb_clock_family());
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
    /// Two reachable clocks, replayed from the trace on the impl,
    /// possibly the same pick, so the pair laws' equality arms stay
    /// live on this population.
    c: [&'a Clock; 2],
    /// Pool-indexed lists at fold-boundary arities. Repeated picks are
    /// repeated raw versions and *aliased* live parties/clocks, the
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
        assert_laws!(laws::$group, $env.c[0]);
    };
    (@one $env:expr, $group:ident, (clock, clock)) => {
        assert_laws!(laws::$group, $env.c[0], $env.c[1]);
    };
    (@one $env:expr, $group:ident, (clock, version)) => {
        assert_laws!(laws::$group, $env.c[0], $env.v[1]);
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
        assert_laws!(laws::$group, $env.c[0], $env.clocks);
    };
}

proptest! {
    /// The whole law collection holds over organic op-trace populations.
    ///
    /// The same slices the arbitrary-normal-form drivers iterate, landed on
    /// the value shapes rule-respecting operation schedules produce (live
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
            step::<Production>(&mut imp, op, &mut ());
        }
        let ca = &imp[picks[0] % imp.len()];
        let cb = &imp[picks[1] % imp.len()];
        let clist: Vec<Clock> = list_picks
            .iter()
            .map(|t| imp[t % imp.len()].dangerously_alias())
            .collect();

        let organic = Organic {
            v: [&ia, &ib, &ic],
            p: [&qa, &qb, &qc],
            r: [&ra, &rb, &rc],
            c: [ca, cb],
            versions: &vlist,
            parties: &plist,
            clocks: &clist,
        };
        crate::for_each_law_group!(organic_drive(&organic));
    }
}

// ─────────────────── deep, multi-scale, and co-generated inputs ───────────────────

mod co_generated;
mod deep;
mod multiscale;
