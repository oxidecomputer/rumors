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

use super::OwnVersion;
use crate::{Clock, Version};

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
