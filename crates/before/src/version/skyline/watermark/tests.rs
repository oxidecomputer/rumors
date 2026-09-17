//! Direct tests of [`MinWeb`]'s latent-boundary and range-closing invariants.
//!
//! Fixed cases reach internal branches that encoded walks cannot reliably
//! isolate. Properties vary magnitudes across accumulator representations and
//! verify that closing batch-armed ranges consumes one record at a time.

use core::cmp::Ordering;

use num_bigint::BigUint;
use proptest::prelude::*;
use suanpan::Accumulator;

use crate::codec::{Base, Int};

use super::super::signed::{fold_signed_int, Sign, Signed};
use super::{Close, MinWeb};

/// A priced word-scale offset `−n`: an emission `n` below the running height.
fn below(n: u64) -> Signed {
    Signed {
        sign: Sign::Negative,
        magnitude: Int::Small(n),
    }
}

/// The magnitude `n` spelled wide.
///
/// This forces the accumulator's multi-word path even for small values.
fn wide(n: &BigUint) -> Int {
    Int::Wide(Base::from(n.clone()))
}

/// A priced offset `−n` at a wide-spelled magnitude.
fn below_wide(n: &BigUint) -> Signed {
    Signed {
        sign: Sign::Negative,
        magnitude: wide(n),
    }
}

/// Collapsing a comparable-scale latent preserves the tracked minimum.
///
/// The fixture parks a latent of `1000`, then emits at `25`. Probes below, at,
/// and above the original minimum verify the restored fold's value and sign.
#[test]
fn post_collapse_restore_returns_the_priced_fold() {
    let mut web: MinWeb<()> = MinWeb::new();
    web.open(2);
    web.emit_here(); // both ranges arm at v = 0
    web.open(1);
    web.fold_height(Sign::Positive, &Int::Small(1000)); // h = 1000
    web.emit_here(); // the inner range arms at v = 1000
    web.close(); // parks the popped boundary: Λ = 1000, A = 1000, m = 0
    assert!(web.latent_live(), "the close parks the popped boundary");
    web.emit_offset(&below(975)); // v = 25: comparable scales, not below m
    assert!(
        !web.latent_live(),
        "comparable scales collapse the latent inside the ladder"
    );
    // h = 1000 and m = 0: an exact restore leaves every probe reading the
    // true minimum.
    assert_eq!(
        web.compare_above(&below(1000)),
        Ordering::Equal,
        "the probe at the true minimum reads exact"
    );
    assert_eq!(
        web.compare_above(&below(975)),
        Ordering::Greater,
        "a probe above the minimum reads above"
    );
    assert_eq!(
        web.compare_above(&below(1001)),
        Ordering::Less,
        "a probe below the minimum reads below"
    );
}

/// A drop larger than a live latent transfers the undercut residue exactly.
///
/// The fixture parks a latent of `50`, then undercuts the prior minimum by
/// `2^34`. Probes before and after closing the range verify that the surviving
/// boundary contains `m - v`, not `A - v`.
#[test]
fn dominated_latent_annihilates_into_the_undercut_residue() {
    const D: u64 = 1 << 36;
    const E: u64 = 1 << 34;
    let mut web: MinWeb<()> = MinWeb::new();
    web.open(2);
    web.emit_here(); // both ranges arm at v = 0
    web.fold_height(Sign::Positive, &Int::Small(D));
    web.open(1);
    web.emit_here(); // the middle range arms at v = D
    web.fold_height(Sign::Positive, &Int::Small(50)); // h = D + 50
    web.open(1);
    web.emit_here(); // the inner range arms at v = D + 50
    web.close(); // parks Λ = 50: A = D + 50, innermost minimum m = D
    assert!(web.latent_live(), "the close parks the popped boundary");
    web.emit_offset(&below(50 + E)); // v = D − E: a dominating drop
    assert!(
        !web.latent_live(),
        "the undercut annihilates the latent into its residue"
    );
    // The undercut seated the innermost minimum at v = D − E exactly.
    assert_eq!(
        web.compare_above(&below(50 + E)),
        Ordering::Equal,
        "the probe at the undercut emission reads exact"
    );
    // Close the dropped range: the boundary that survived the residue pops
    // and parks, and the probes then read the outer minimum 0 through it —
    // exact only if the residue annihilated the latent.
    web.close();
    assert_eq!(
        web.compare_above(&below(D + 50)),
        Ordering::Equal,
        "the probe at the outer minimum reads exact"
    );
    assert_eq!(
        web.compare_above(&below(D + 49)),
        Ordering::Greater,
        "a probe above the outer minimum reads above"
    );
    assert_eq!(
        web.compare_above(&below(D + 51)),
        Ordering::Less,
        "a probe below the outer minimum reads below"
    );
}

/// A drop that remains above the parked minimum leaves the web unchanged.
///
/// A latent of `2^36` dominates a drop of `50`. The latent remains live and
/// three probes establish that the tracked minimum is still zero.
#[test]
fn a_drop_short_of_the_latent_minimum_refuses_the_undercut() {
    const D: u64 = 1 << 36;
    let mut web: MinWeb<()> = MinWeb::new();
    web.open(2);
    web.emit_here(); // both ranges arm at v = 0
    web.open(1);
    web.fold_height(Sign::Positive, &Int::Small(D)); // h = D
    web.emit_here(); // the inner range arms at v = D
    web.close(); // parks Λ = D: A = D, m = 0
    assert!(web.latent_live(), "the close parks the popped boundary");
    web.fold_height(Sign::Negative, &Int::Small(50)); // h = D − 50
    web.emit_here(); // v = D − 50: below the anchor, above the minimum
    assert!(
        web.latent_live(),
        "a dominating latent answers the drop with no state change"
    );
    // h = D − 50 and m = 0: the minimum is read through the surviving latent.
    assert_eq!(
        web.compare_above(&below(D - 50)),
        Ordering::Equal,
        "the probe at the true minimum reads exact"
    );
    assert_eq!(
        web.compare_above(&below(D - 51)),
        Ordering::Greater,
        "a probe above the minimum reads above"
    );
    assert_eq!(
        web.compare_above(&below(D - 49)),
        Ordering::Less,
        "a probe below the minimum reads below"
    );
}

/// The spilled-accumulator path also preserves a minimum above the drop.
///
/// A latent of `2^200` forces the digit engine, while the word-sized drop keeps
/// the expected ordering easy to calculate. Three probes establish the result.
#[test]
fn a_spilled_latent_refuses_the_drop_on_the_folded_certificate() {
    let lambda = BigUint::from(1u8) << 200;
    let mut web: MinWeb<()> = MinWeb::new();
    web.open(2);
    web.emit_here(); // both ranges arm at v = 0
    web.open(1);
    web.fold_height(Sign::Positive, &wide(&lambda)); // h = Λ
    web.emit_here(); // the inner range arms at v = Λ
    web.close(); // parks Λ = 2^200: A = Λ, m = 0
    assert!(web.latent_live(), "the close parks the popped boundary");
    web.fold_height(Sign::Negative, &Int::Small(50)); // h = Λ − 50
    web.emit_here(); // v = Λ − 50: below the anchor, above the minimum
    assert!(
        web.latent_live(),
        "a dominating latent answers the drop with no state change"
    );
    let height = &lambda - BigUint::from(50u8);
    assert_eq!(
        web.compare_above(&below_wide(&height)),
        Ordering::Equal,
        "the probe at the true minimum reads exact"
    );
    assert_eq!(
        web.compare_above(&below_wide(&(&height - BigUint::from(1u8)))),
        Ordering::Greater,
        "a probe above the minimum reads above"
    );
    assert_eq!(
        web.compare_above(&below_wide(&(&height + BigUint::from(1u8)))),
        Ordering::Less,
        "a probe below the minimum reads below"
    );
}

proptest! {
    /// A dominant undercut subtracts exactly `m - v` from every live follower.
    ///
    /// One range arms at `0` with a follower installed at a known offset, the
    /// height drops `2^b`, and a word-scale offset emission arrives. Past
    /// `2^128` the gap's sign dominates a word whatever spelling the
    /// accumulator holds it in, so the emission takes the scale-disparate
    /// undercut answered with no fold: the residue `m − v = 2^b + k` moves out
    /// whole, and a follower tracking `m − X` must come down by exactly it.
    ///
    /// Varying the drop and starting value checks both the residue's magnitude
    /// and its negative sign.
    #[test]
    fn a_dominated_undercut_subtracts_its_residue_from_live_followers(
        b in 128usize..=300,
        k in 1u64..=u64::from(u32::MAX),
        start in 0u64..=1_000_000,
    ) {
        const SLOT: usize = 0;
        let drop = BigUint::from(1u8) << b;
        let mut web: MinWeb<()> = MinWeb::new();
        web.open(1);
        web.emit_here(); // the range arms at v = 0: A = 0, m = 0
        let mut follower = Accumulator::new();
        fold_signed_int(&mut follower, Sign::Positive, &Int::Small(start));
        web.follower_set(SLOT, follower); // a live follower at m − X = start
        web.fold_height(Sign::Negative, &wide(&drop)); // h = −2^b
        web.emit_offset(&below(k)); // v = −2^b − k: the dominated undercut
        let taken = web.follower_take(SLOT);
        let moved = web.materialize(taken);
        // start − (2^b + k), necessarily negative: the residue dwarfs `start`.
        let residue = &drop + BigUint::from(k);
        prop_assert_eq!(
            moved.sign,
            Sign::Negative,
            "the residue leaves the follower below where it stood, never above"
        );
        prop_assert_eq!(
            moved.magnitude,
            wide(&(residue - BigUint::from(start))),
            "the follower moved by exactly the residue m − v"
        );
    }
}

proptest! {
    /// A drop between the parked minimum and anchor preserves the minimum and
    /// permits a later undercut.
    ///
    /// Magnitudes cover register-held and spilled accumulators. Probes verify
    /// the first drop, then a true undercut verifies that the preserved state
    /// remains usable.
    #[test]
    fn a_drop_inside_the_latent_never_moves_the_minimum(
        b in 34usize..=260,
        d in 1u64..=(1u64 << 33),
    ) {
        let lambda = BigUint::from(1u8) << b;
        let mut web: MinWeb<()> = MinWeb::new();
        web.open(2);
        web.emit_here(); // both ranges arm at v = 0
        web.open(1);
        web.fold_height(Sign::Positive, &wide(&lambda)); // h = Λ
        web.emit_here(); // the inner range arms at v = Λ
        web.close(); // parks Λ: A = Λ, m = 0
        prop_assert!(web.latent_live(), "the close parks the popped boundary");
        web.fold_height(Sign::Negative, &Int::Small(d)); // h = Λ − d
        web.emit_here(); // v = Λ − d: strictly inside (m, A)
        // The minimum is still 0, whichever arm answered.
        let height = &lambda - BigUint::from(d);
        prop_assert_eq!(
            web.compare_above(&below_wide(&height)),
            Ordering::Equal,
            "the probe at the true minimum reads exact"
        );
        prop_assert_eq!(
            web.compare_above(&below_wide(&(&height - BigUint::from(1u8)))),
            Ordering::Greater,
            "a probe above the minimum reads above"
        );
        prop_assert_eq!(
            web.compare_above(&below_wide(&(&height + BigUint::from(1u8)))),
            Ordering::Less,
            "a probe below the minimum reads below"
        );
        // The refusal left the web intact: a drop that does pass the minimum
        // still seats it exactly.
        web.fold_height(Sign::Negative, &wide(&(&height + BigUint::from(1u8)))); // h = −1
        web.emit_here(); // v = −1: past m = 0, a true undercut
        // The undercut runs with the outer range still armed, so it propagates
        // to a live follower. Close the dropped range and read the outer
        // minimum back through the boundary that parked: the value survives
        // only if the follower's residue moved at the right polarity.
        web.close();
        web.fold_height(Sign::Positive, &Int::Small(100)); // h = 99
        prop_assert_eq!(
            web.compare_above(&below(100)),
            Ordering::Equal,
            "the outer minimum reads the undercut emission through the parked boundary"
        );
        prop_assert_eq!(
            web.compare_above(&below(99)),
            Ordering::Greater,
            "a probe above the outer minimum reads above"
        );
        prop_assert_eq!(
            web.compare_above(&below(101)),
            Ordering::Less,
            "a probe below the outer minimum reads below"
        );
    }
}

proptest! {
    /// Batch-armed ranges close one at a time and consume one record each.
    ///
    /// The records consumed are `n − 1` zero-run entries, then the one
    /// stacked boundary parking; throughout, the outer range stays armed
    /// with its tracked minimum exactly where its own arming emission put
    /// it.
    #[test]
    fn batch_armed_closes_consume_exactly_one_range_record(n in 1usize..40) {
        let mut web: MinWeb<()> = MinWeb::new();
        web.open(1);
        web.emit_here(); // the outer range arms at v = 0
        web.fold_height(Sign::Positive, &Int::Small(7)); // h = 7
        web.open(n as u64);
        web.emit_here(); // all n inner ranges arm at v = 7: one boundary, n − 1 zeros
        for i in 0..n {
            if i < n - 1 {
                prop_assert!(
                    matches!(web.close(), Close::ZeroRun),
                    "a batch-armed inner range closes as one zero-run entry"
                );
            } else {
                prop_assert!(
                    matches!(web.close(), Close::Parked(())),
                    "the batch's last close pops the one stacked boundary"
                );
            }
        }
        prop_assert!(!web.has_pending(), "the arming emission left nothing pending");
        prop_assert!(web.armed(), "the outer range survives its inner ranges' closes");
        prop_assert_eq!(
            web.compare_above(&below(7)),
            Ordering::Equal,
            "the probe at the outer minimum reads exact"
        );
        prop_assert_eq!(
            web.compare_above(&below(6)),
            Ordering::Greater,
            "a probe above the outer minimum reads above"
        );
        prop_assert_eq!(
            web.compare_above(&below(8)),
            Ordering::Less,
            "a probe below the outer minimum reads below"
        );
    }
}
