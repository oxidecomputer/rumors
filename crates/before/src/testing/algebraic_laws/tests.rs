//! The law drivers: one generic proptest per signature group, iterating
//! [`crate::laws`]' slices so every assertion names the law it checks.
//!
//! Each driver's doc comment states the meta-invariant; the individual
//! laws' statements live with their predicates in [`crate::laws`]. No
//! assertion's right-hand side mentions the oracle: the laws hold by the
//! ITC algebra, so they catch a defect the impl and the recursive oracle
//! would share.

use proptest::prelude::*;

use crate::laws;
use crate::oracle;
use crate::testing::generators::{
    arb_clock_family, arb_fold_arity, arb_oracle_party_nonempty, arb_oracle_version,
    arb_party_family, arb_version_family,
};
use crate::testing::optrace::{run, step_impl, world_strategy};
use crate::{Clock, Party, Version};

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

proptest! {
    /// Every [`laws::VERSION_SOLO`] law holds on arbitrary normal-form
    /// versions, including large-base events.
    #[test]
    fn version_solo_laws(a in arb_oracle_version()) {
        assert_laws!(laws::VERSION_SOLO, &ver(&a));
    }
}

proptest! {
    /// Every [`laws::VERSION_PAIR`] law holds on arbitrary normal-form
    /// version pairs.
    #[test]
    fn version_pair_laws(a in arb_oracle_version(), b in arb_oracle_version()) {
        assert_laws!(laws::VERSION_PAIR, &ver(&a), &ver(&b));
    }
}

proptest! {
    /// Every [`laws::VERSION_TRIPLE`] law holds on arbitrary normal-form
    /// version triples.
    #[test]
    fn version_triple_laws(
        a in arb_oracle_version(),
        b in arb_oracle_version(),
        c in arb_oracle_version(),
    ) {
        assert_laws!(laws::VERSION_TRIPLE, &ver(&a), &ver(&b), &ver(&c));
    }
}

proptest! {
    /// Every [`laws::PARTY_SOLO`] law holds on arbitrary non-empty
    /// normal-form ids.
    #[test]
    fn party_solo_laws(p in arb_oracle_party_nonempty()) {
        assert_laws!(laws::PARTY_SOLO, &party(&p));
    }
}

proptest! {
    /// Every [`laws::PARTY_PAIR`] law holds on arbitrary non-empty id pairs
    /// — typically unrelated and frequently overlapping, shapes the op
    /// pipeline never produces.
    #[test]
    fn party_pair_laws(p in arb_oracle_party_nonempty(), q in arb_oracle_party_nonempty()) {
        assert_laws!(laws::PARTY_PAIR, &party(&p), &party(&q));
    }
}

proptest! {
    /// Every [`laws::PARTY_TRIPLE`] law holds on arbitrary non-empty id
    /// triples.
    #[test]
    fn party_triple_laws(
        p in arb_oracle_party_nonempty(),
        q in arb_oracle_party_nonempty(),
        r in arb_oracle_party_nonempty(),
    ) {
        assert_laws!(laws::PARTY_TRIPLE, &party(&p), &party(&q), &party(&r));
    }
}

proptest! {
    /// Every [`laws::VERSION_PARTY`] law holds on arbitrary version/id
    /// pairings — the tick and projection laws on regions unrelated to the
    /// history they act on.
    #[test]
    fn version_party_laws(a in arb_oracle_version(), p in arb_oracle_party_nonempty()) {
        assert_laws!(laws::VERSION_PARTY, &ver(&a), &party(&p));
    }
}

proptest! {
    /// Every [`laws::VERSION_PAIR_PARTY`] law holds on arbitrary
    /// version-pair/id combinations.
    #[test]
    fn version_pair_party_laws(
        a in arb_oracle_version(),
        b in arb_oracle_version(),
        p in arb_oracle_party_nonempty(),
    ) {
        assert_laws!(laws::VERSION_PAIR_PARTY, &ver(&a), &ver(&b), &party(&p));
    }
}

proptest! {
    /// Every [`laws::VERSION_PARTY_PAIR`] law holds on arbitrary
    /// version/id-pair combinations.
    #[test]
    fn version_party_pair_laws(
        a in arb_oracle_version(),
        p in arb_oracle_party_nonempty(),
        q in arb_oracle_party_nonempty(),
    ) {
        assert_laws!(laws::VERSION_PARTY_PAIR, &ver(&a), &party(&p), &party(&q));
    }
}

proptest! {
    /// Every [`laws::VERSION_PAIR_PARTY_PAIR`] law holds on arbitrary
    /// version-pair/id-pair combinations.
    #[test]
    fn version_pair_party_pair_laws(
        a in arb_oracle_version(),
        b in arb_oracle_version(),
        p in arb_oracle_party_nonempty(),
        q in arb_oracle_party_nonempty(),
    ) {
        assert_laws!(laws::VERSION_PAIR_PARTY_PAIR, &ver(&a), &ver(&b), &party(&p), &party(&q));
    }
}

proptest! {
    /// Every [`laws::RANK_TRIPLE`] law holds on ranks derived from
    /// arbitrary normal-form versions (their own ranks and a genuine
    /// distance) — organically related magnitudes.
    ///
    /// The adversarial spilled-magnitude regime is driven where the rank
    /// machinery lives, in the version suite's rank driver.
    #[test]
    fn rank_triple_laws(a in arb_oracle_version(), b in arb_oracle_version()) {
        let (va, vb) = (ver(&a), ver(&b));
        let (ra, rb, rc) = (va.rank(), vb.rank(), va.distance(&vb));
        assert_laws!(laws::RANK_TRIPLE, &ra, &rb, &rc);
    }
}

proptest! {
    /// Every [`laws::CLOCK_SOLO`] law holds on arbitrary canonical
    /// party/version pairings — every such pairing is a valid clock,
    /// including ones no op sequence reaches.
    #[test]
    fn clock_solo_laws(p in arb_oracle_party_nonempty(), a in arb_oracle_version()) {
        assert_laws!(laws::CLOCK_SOLO, &Clock::from_parts(party(&p), ver(&a)));
    }
}

proptest! {
    /// Every [`laws::CLOCK_VERSION`] law holds on arbitrary clocks paired
    /// with arbitrary (typically concurrent, unrelated) messages.
    #[test]
    fn clock_version_laws(
        p in arb_oracle_party_nonempty(),
        a in arb_oracle_version(),
        m in arb_oracle_version(),
    ) {
        assert_laws!(laws::CLOCK_VERSION, &Clock::from_parts(party(&p), ver(&a)), &ver(&m));
    }
}

proptest! {
    /// Every [`laws::VERSION_LIST`] law holds on pool-indexed version
    /// lists whose arities sweep the balanced fold's boundary band
    /// (`arb_fold_arity` documents the derivation).
    #[test]
    fn version_list_laws((_, xs) in arb_version_family()) {
        let xs: Vec<Version> = xs.iter().map(ver).collect();
        assert_laws!(laws::VERSION_LIST, &xs);
    }
}

proptest! {
    /// Every [`laws::VERSION_AND_LIST`] law holds on pool-indexed
    /// version families — a receiver plus boundary-swept items, repeats
    /// and the empty version under mass at every arity.
    #[test]
    fn version_and_list_laws((r, xs) in arb_version_family()) {
        let receiver = ver(&r);
        let xs: Vec<Version> = xs.iter().map(ver).collect();
        assert_laws!(laws::VERSION_AND_LIST, &receiver, &xs);
    }
}

proptest! {
    /// Every [`laws::PARTY_AND_LIST`] law holds on pool-indexed party
    /// families — aliased repeats keep the refusal arm under mass, the
    /// constructed laws' fork trees the accepted arm, at every
    /// boundary-band arity.
    #[test]
    fn party_and_list_laws((r, items) in arb_party_family()) {
        let receiver = party(&r);
        let items: Vec<Party> = items.iter().map(party).collect();
        assert_laws!(laws::PARTY_AND_LIST, &receiver, &items);
    }
}

proptest! {
    /// Every [`laws::CLOCK_AND_LIST`] law holds on pool-indexed clock
    /// families — arbitrary canonical party/version pairings, aliased
    /// parties under mass, at every boundary-band arity.
    #[test]
    fn clock_and_list_laws((r, items) in arb_clock_family()) {
        let clock = |(p, v): &(oracle::Party, oracle::Version)| Clock::from_parts(party(p), ver(v));
        let receiver = clock(&r);
        let items: Vec<Clock> = items.iter().map(clock).collect();
        assert_laws!(laws::CLOCK_AND_LIST, &receiver, &items);
    }
}

// ───────────────────── organic op-trace populations ─────────────────────

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

        assert_laws!(laws::VERSION_SOLO, &ia);
        assert_laws!(laws::VERSION_PAIR, &ia, &ib);
        assert_laws!(laws::VERSION_TRIPLE, &ia, &ib, &ic);
        assert_laws!(laws::PARTY_SOLO, &qa);
        assert_laws!(laws::PARTY_PAIR, &qa, &qb);
        assert_laws!(laws::PARTY_TRIPLE, &qa, &qb, &qc);
        assert_laws!(laws::VERSION_PARTY, &ia, &qb);
        assert_laws!(laws::VERSION_PAIR_PARTY, &ia, &ib, &qc);
        assert_laws!(laws::VERSION_PARTY_PAIR, &ia, &qb, &qc);
        assert_laws!(laws::VERSION_PAIR_PARTY_PAIR, &ia, &ib, &qb, &qc);
        let (ra, rb, rc) = (ia.rank(), ib.rank(), ia.distance(&ib));
        assert_laws!(laws::RANK_TRIPLE, &ra, &rb, &rc);

        // Variadic families over the same population: pool-indexed picks
        // at fold-boundary arities. Repeated picks are repeated raw
        // versions and *aliased* live parties/clocks — the input classes
        // the list laws' fold and refusal arms exist for.
        let vlist: Vec<Version> = list_picks.iter().map(|t| ver(cs[t % n].trees().1)).collect();
        assert_laws!(laws::VERSION_LIST, &vlist);
        assert_laws!(laws::VERSION_AND_LIST, &ia, &vlist);
        let plist: Vec<Party> = list_picks.iter().map(|t| party(cs[t % n].trees().0)).collect();
        assert_laws!(laws::PARTY_AND_LIST, &qa, &plist);

        // Clocks: replay the trace on the impl for real, reachable clocks.
        let mut imp = vec![Clock::seed()];
        for op in &ops {
            step_impl(&mut imp, op);
        }
        let ca = &imp[picks[0] % imp.len()];
        assert_laws!(laws::CLOCK_SOLO, ca);
        assert_laws!(laws::CLOCK_VERSION, ca, &ib);
        let clist: Vec<Clock> = list_picks
            .iter()
            .map(|t| imp[t % imp.len()].dangerously_alias())
            .collect();
        assert_laws!(laws::CLOCK_AND_LIST, ca, &clist);
    }
}
