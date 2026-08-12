//! [`OwnVersion`] tests: the view's semantic-equality contract and its
//! explicit materialization.
//!
//! The fused projected walks are bound to the *oracle* — the independent
//! recursive implementation the impl shares nothing with — by the
//! differential table's three- and four-stream descriptors, over arbitrary
//! normal-form operands and organic op-trace populations alike. The
//! materialized-form coherence (`view ⋚ w ≡ view.to_version() ⋚ w`, both
//! comparison directions and `==`, plus the seed-mask coherence) lives in
//! [`crate::laws`] and runs through every law consumer. What remains here
//! is what neither of those states: that equality on the view is semantic
//! rather than representational, and that the `From` impl is the
//! materialization.

use core::cmp::Ordering;

use proptest::prelude::*;

use super::OwnVersion;
use crate::testing::bridge::{from_oracle_party, from_oracle_version};
use crate::testing::generators::{arb_oracle_party_nonempty, arb_oracle_version};
use crate::{Clock, Party, Version};

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

// The seed-party view as the identity view is law-pinned:
// `laws::VERSION_SOLO::seed_projection_is_identity` (the view compares equal
// to the version) and `laws::VERSION_PAIR_PARTY::
// own_version_seed_mask_coherence` (the seed mask changes no verdict), with
// both comparison directions covered by own_version_cmp_matches_materialized
// — all driven on the three law populations.

/// One deep-unowned-region case: both orientations of the plain-version ⋚
/// view matrix cells against the materialized projection, plus their mutual
/// antisymmetry and the equality cells' coherence.
fn assert_mirror_cells(w: &Version, v: &Version, p: &Party) {
    let view = v / p;
    let materialized = view.to_version();
    assert_eq!(
        w.partial_cmp(&view),
        w.partial_cmp(&materialized),
        "w vs view must match w vs the materialized projection: {w} vs {v} / {p}"
    );
    assert_eq!(
        view.partial_cmp(w),
        w.partial_cmp(&view).map(Ordering::reverse),
        "the two orientations must be antisymmetric: {w} vs {v} / {p}"
    );
    assert_eq!(
        *w == view,
        *w == materialized,
        "equality on the view is semantic: {w} vs {v} / {p}"
    );
}

/// A version tree whose right arm is a `depth`-level descending spine: the
/// deep half a one-sided mask leaves unowned, so the mirror comparison's
/// co-walk block-skips it on the masked side.
fn deep_right_spine(depth: u32, top: u64) -> Version {
    use crate::oracle::Version as V;
    let mut spine = V::leaf(0u64);
    for level in 1..depth {
        spine = V::node(0u64, V::leaf(u64::from(level) + top), spine);
    }
    from_oracle_version(&V::node(0u64, V::leaf(top), spine))
}

/// The mirror matrix cells (`w ⋚ view`, mask on the walk's second side) agree
/// with the materialized projection when the view's version runs deep under
/// an unowned mask region.
///
/// The one-sided co-walk block-consumes the unowned deep spine on the masked
/// side with no height integrator for it (the unmasked side never reads one),
/// so the whole spine's movement lands in the running difference in one
/// block; a fold or skip-bound error there displaces the verdict, which the
/// materialized projection refutes. The worked points hold the three verdict
/// classes — the plain version above, below, and concurrent with the view.
#[test]
fn mirror_cells_agree_on_deep_unowned_spines() {
    use crate::oracle::Party as P;
    let owns_left = from_oracle_party(&P::node(P::seed(), P::Leaf(false)));
    let v = deep_right_spine(24, 40);
    // Above: w carries the projection's owned plateau and more.
    assert_mirror_cells(&Version::try_from(41u64).unwrap(), &v, &owns_left);
    // Below: the projection's owned plateau exceeds w.
    assert_mirror_cells(&Version::try_from(1u64).unwrap(), &v, &owns_left);
    // Concurrent: w is live where the projection is zero and behind where it
    // is live.
    let w = from_oracle_version(&{
        use crate::oracle::Version as V;
        V::node(0u64, V::leaf(0u64), V::leaf(7u64))
    });
    assert_mirror_cells(&w, &v, &owns_left);
}

proptest! {
    /// The mirror matrix cells agree with the materialized projection over
    /// arbitrary normal-form versions and nonempty parties on both sides.
    ///
    /// The shape-generic sweep behind the deep-spine worked points, covering
    /// every mask arrangement the public matrix can spell.
    #[test]
    fn mirror_cells_agree_on_arbitrary_triples(
        ow in arb_oracle_version(),
        ov in arb_oracle_version(),
        op in arb_oracle_party_nonempty(),
    ) {
        let w = from_oracle_version(&ow);
        let v = from_oracle_version(&ov);
        let p = from_oracle_party(&op);
        assert_mirror_cells(&w, &v, &p);
    }
}
