//! [`OwnVersion`] tests: the fused projected comparisons against the recursive
//! oracle's composed projection-and-compare, and the view's semantic-equality
//! contract.
//!
//! The materialized-form coherence (`view ⋚ w ≡ view.to_version() ⋚ w`, three-
//! and four-stream, plus the seed-mask coherence) lives in [`crate::laws`] and
//! runs through every law consumer; the suites here bind the fused walks to the
//! *oracle* — the independent recursive implementation the impl shares nothing
//! with — over arbitrary normal-form operands and organic op-trace populations.

use proptest::prelude::*;

use super::OwnVersion;
use crate::testing::bridge::{from_oracle_party, from_oracle_version};
use crate::testing::generators::{arb_oracle_party_nonempty, arb_oracle_version};
use crate::testing::optrace::{run, versions, world_strategy};
use crate::{oracle, Clock, Party, Version};

/// The oracle's composed verdict for `(v / p) ⋚ w`: materialize the projection
/// on the recursive trees, then compare.
fn oracle_view_cmp(
    v: &oracle::Version,
    p: &oracle::Party,
    w: &oracle::Version,
) -> Option<std::cmp::Ordering> {
    (v.clone() / p).partial_cmp(w)
}

proptest! {
    /// Differential. The fused three-stream comparison `(v / p) ⋚ w` matches
    /// the oracle's materialize-then-compare on arbitrary normal-form operands,
    /// in both operand orders and under `==`.
    #[test]
    fn view_cmp_matches_oracle_composed(
        v in arb_oracle_version(),
        w in arb_oracle_version(),
        p in arb_oracle_party_nonempty(),
    ) {
        let (iv, iw, ip) = (from_oracle_version(&v), from_oracle_version(&w), from_oracle_party(&p));
        let expected = oracle_view_cmp(&v, &p, &w);
        prop_assert_eq!((&iv / &ip).partial_cmp(&iw), expected);
        prop_assert_eq!(iw.partial_cmp(&(&iv / &ip)), expected.map(std::cmp::Ordering::reverse));
        prop_assert_eq!((&iv / &ip) == iw, expected == Some(std::cmp::Ordering::Equal));
    }
}

proptest! {
    /// Differential. The fused four-stream comparison `(v/p) ⋚ (w/q)` matches
    /// the oracle's materialize-then-compare on arbitrary normal-form operands,
    /// under `partial_cmp` and `==` alike.
    #[test]
    fn view_pair_cmp_matches_oracle_composed(
        v in arb_oracle_version(),
        w in arb_oracle_version(),
        p in arb_oracle_party_nonempty(),
        q in arb_oracle_party_nonempty(),
    ) {
        let (iv, iw) = (from_oracle_version(&v), from_oracle_version(&w));
        let (ip, iq) = (from_oracle_party(&p), from_oracle_party(&q));
        let expected = (v.clone() / &p).partial_cmp(&(w.clone() / &q));
        prop_assert_eq!((&iv / &ip).partial_cmp(&(&iw / &iq)), expected);
        prop_assert_eq!(
            (&iv / &ip) == (&iw / &iq),
            expected == Some(std::cmp::Ordering::Equal)
        );
    }
}

proptest! {
    /// Differential. Both fused walks match the oracle's composed verdicts over
    /// organic op-trace populations.
    ///
    /// Live sibling parties and causally related versions — the value shapes
    /// real fork/tick/join/sync schedules produce, where domination and
    /// equality actually occur.
    #[test]
    fn view_cmp_matches_oracle_on_organic_populations(
        ops in world_strategy(),
        i in 0usize..64,
        j in 0usize..64,
        k in 0usize..64,
    ) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        let (ov, ow) = (&vs[i % n], &vs[j % n]);
        let (op_, oq) = (cs[k % n].party(), cs[(k + 1) % n].party());
        let (iv, iw) = (from_oracle_version(ov), from_oracle_version(ow));
        let (ip, iq) = (from_oracle_party(op_), from_oracle_party(oq));
        prop_assert_eq!(
            (&iv / &ip).partial_cmp(&iw),
            oracle_view_cmp(ov, op_, ow)
        );
        prop_assert_eq!(
            (&iv / &ip).partial_cmp(&(&iw / &iq)),
            (ov.clone() / op_).partial_cmp(&(ow.clone() / oq))
        );
    }
}

/// Equality on the view is semantic, not representational: `view == w` requires
/// `w` to be zero outside the party's region.
///
/// A version equal to the projection *inside* the region but live outside it
/// compares unequal (and strictly greater).
#[test]
fn view_equality_requires_zero_outside_the_region() {
    let mut a = Clock::seed();
    let mut b = a.fork();
    a.tick();
    b.tick();
    let joined = a.version() | b.version(); // live on both halves
    let view = &joined / a.party();
    // Inside a's region the two agree; b's tick lives outside it.
    assert_ne!(view, joined);
    assert!(view < joined);
    // Against its own materialization — zero outside by construction —
    // the view is equal.
    assert_eq!(view, view.to_version());
}

/// The `From` impl is `to_version`, and the view is `Copy`: one view can be
/// compared and materialized repeatedly without re-projecting.
#[test]
fn from_impl_is_to_version() {
    let mut a = Clock::seed();
    a.tick();
    let view: OwnVersion<'_> = a.own_version();
    let via_from = Version::from(view);
    assert_eq!(via_from, view.to_version());
    assert_eq!(view, via_from); // the copy still compares after the move
}

/// The seed-party view is the identity view: `(&v / &seed)` compares
/// equal to `v` itself in both directions.
#[test]
fn seed_view_is_identity() {
    let mut c = Clock::seed();
    c.tick();
    let seed = Party::seed();
    let v = c.version();
    assert_eq!(v / &seed, *v);
    assert_eq!(*v, v / &seed);
}
