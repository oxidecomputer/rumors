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
use crate::testing::generators::{arb_oracle_party_nonempty, arb_oracle_version};
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
    /// Every [`laws::RANK_TRIPLE`] law holds on ranks derived from
    /// arbitrary normal-form versions (their own ranks and a genuine
    /// distance) — organically related magnitudes; the adversarial
    /// spilled-magnitude regime is driven where the rank machinery lives,
    /// in the version suite's rank driver.
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

// ───────────────────── organic op-trace populations ─────────────────────

proptest! {
    /// The whole law collection holds over organic op-trace populations:
    /// the same slices the arbitrary-normal-form drivers iterate, landed on
    /// the value shapes real fork/tick/join/sync schedules produce (live
    /// sibling parties, causally related versions, reachable clocks).
    #[test]
    fn laws_hold_on_organic_populations(
        ops in world_strategy(),
        i in 0usize..64,
        j in 0usize..64,
        k in 0usize..64,
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
        let (ra, rb, rc) = (ia.rank(), ib.rank(), ia.distance(&ib));
        assert_laws!(laws::RANK_TRIPLE, &ra, &rb, &rc);

        // Clocks: replay the trace on the impl for real, reachable clocks.
        let mut imp = vec![Clock::seed()];
        for op in &ops {
            step_impl(&mut imp, op);
        }
        let ca = &imp[picks[0] % imp.len()];
        assert_laws!(laws::CLOCK_SOLO, ca);
        assert_laws!(laws::CLOCK_VERSION, ca, &ib);
    }
}
