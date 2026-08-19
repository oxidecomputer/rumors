//! Direct pins on the web's latent-ladder gates that no packed-stream walk
//! reaches, and on the seam contracts no packed stream drives.
//!
//! Deliberate internal-entry tests, driving [`MinWeb`] at its own seam. The
//! worked pins read a live latent only immediately behind a raise-decision
//! read (`compare_above`) on the same folded state: comparable scales collapse
//! the latent inside that read, and a decided drop past the latent routes the
//! raise to the tracked minimum's emission — so `emit_offset`'s post-collapse
//! restore and the undercut's latent annihilation in `drop_below` execute on
//! no input either walk can be handed. The third latent-ladder arm, a
//! dominating latent refusing a drop that never reaches the minimum, is
//! reachable from a packed stream in principle but only grazed by generated
//! populations, and nondeterministically — a directed pin holds it here so the
//! arm's coverage does not depend on the draw. The public-surface family
//! (`fill/tests.rs`'s latent-ladder suite) pins every walk-reachable arm
//! against the recursive oracle; the worked pins here hold the web's
//! remaining gates to the same polarity discipline at the only seam that can
//! reach them.
//!
//! The close seam's precondition — callers close only armed ranges — is
//! debug-asserted at [`MinWeb::close`]; the proptest family here drives the
//! legal batch schedule directly at the seam: `n` ranges armed by one
//! emission close one by one, each consuming exactly one range record, with
//! the outer range's tracked minimum exactly where its arming emission put
//! it throughout.

use core::cmp::Ordering;

use dashu_int::UBig;
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
/// Past the word range this is the only spelling; within it, the same value
/// spelled wide is the redundant spelling the accumulator's certification is
/// sensitive to, which is what makes it worth constructing deliberately.
fn wide(n: &UBig) -> Int {
    Int::Wide(Base::from(n.clone()))
}

/// A priced offset `−n` at a wide-spelled magnitude.
fn below_wide(n: &UBig) -> Signed {
    Signed {
        sign: Sign::Negative,
        magnitude: wide(n),
    }
}

/// After a comparable-scale collapse re-bases the anchor to the true minimum
/// with the emission not below it, `emit_offset` restores the priced fold
/// exactly: every later read still sees `gap = h − m`.
///
/// Two ranges arm at height `0`; an inner range arms `1000` higher and closes,
/// parking `Λ = 1000` (anchor `A = 1000`, true minimum `m = 0`). An emission
/// `975` below the anchor is comparable-scale on both sides, so the ladder
/// collapses the latent and the re-test reads `v = 25` at or above `m`: the
/// declined-drop exit whose restore is under test. Restoring the fold with the
/// wrong polarity leaves the web displaced by `2 · 975`, and the three-point
/// anchor probe below reads the minimum away from `0`.
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

/// A drop that dominates a live latent annihilates the latent into the
/// undercut's residue: the drop leaving the dying anchor is `m − v`, never
/// `A − v`, so the boundary surviving above the stopping range stays exact.
///
/// Two ranges arm; a middle range arms `2^36` higher, an inner one `50` above
/// that, and the inner close parks `Λ = 50`. An emission then drops `2^34 +
/// 50` below the anchor: the drop's register certificate dominates the
/// word-scale latent, a true undercut reaching `drop_below` with the latent
/// still live — the annihilation under test. The middle boundary must survive
/// as exactly `2^36 − 2^34`; skipping the annihilation leaves it long by `Λ`,
/// and the close-then-probe below reads the outer minimum displaced by `50`.
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

/// A drop below the anchor that never reaches the latent-parked minimum
/// refuses the undercut and leaves the web exactly as it stood.
///
/// Two ranges arm at `0`; an inner range arms `2^36` higher and closes, parking
/// `Λ = 2^36` (anchor `A = 2^36`, true minimum `m = 0`). The height then drops
/// `50`, so the emission sits at `v = 2^36 − 50`: strictly between `m` and `A`,
/// and scale-disparate enough that the latent's top dominates the gap's, so the
/// domination read answers `m < v < A` with no fold and no state change. The
/// latent must survive that read — a comparable-scale collapse would retire it,
/// and a true undercut would annihilate it into a residue — and the tracked
/// minimum must still be `0`. Mistaking the drop for an undercut would seat the
/// minimum at `v`, which the three-point anchor probe below reads displaced by
/// `2^36 − 50`.
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

/// The same refusal when the parked latent is too wide to sit in the register:
/// the certification regime changes, the answer does not.
///
/// [`a_drop_short_of_the_latent_minimum_refuses_the_undercut`] parks a latent
/// the accumulator holds exactly, where domination is certified by direct
/// magnitude comparison. A latent of `2^200` spills to the digit engine, which
/// instead certifies through the sign fold's running partial and its decision
/// index — a genuinely different test that the same input must answer the same
/// way. The drop stays word-scale, so the latent still dominates and the web
/// must again come through untouched.
#[test]
fn a_spilled_latent_refuses_the_drop_on_the_folded_certificate() {
    let lambda = UBig::from(1u8) << 200;
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
    let height = &lambda - UBig::from(50u8);
    assert_eq!(
        web.compare_above(&below_wide(&height)),
        Ordering::Equal,
        "the probe at the true minimum reads exact"
    );
    assert_eq!(
        web.compare_above(&below_wide(&(&height - UBig::from(1u8)))),
        Ordering::Greater,
        "a probe above the minimum reads above"
    );
    assert_eq!(
        web.compare_above(&below_wide(&(&height + UBig::from(1u8)))),
        Ordering::Less,
        "a probe below the minimum reads below"
    );
}

proptest! {
    /// A dominated undercut moves its residue *out of* every live follower, by
    /// exactly `m − v`, at every scale that reaches the arm.
    ///
    /// One range arms at `0` with a follower installed at a known offset, the
    /// height drops `2^b`, and a word-scale offset emission arrives. Past
    /// `2^128` the gap's sign dominates a word whatever spelling the
    /// accumulator holds it in, so the emission takes the scale-disparate
    /// undercut answered with no fold: the residue `m − v = 2^b + k` moves out
    /// whole, and a follower tracking `m − X` must come down by exactly it.
    ///
    /// The polarity is the whole content. Folding the residue in at the
    /// opposite sign leaves the follower above where it stood rather than
    /// below, and any residue other than `m − v` displaces it by the
    /// difference — both refused here. The committed regression family drives
    /// this decision with *no* follower installed, so the propagation loop
    /// there runs zero times and its polarity goes unread.
    #[test]
    fn a_dominated_undercut_subtracts_its_residue_from_live_followers(
        b in 128usize..=300,
        k in 1u64..=u64::from(u32::MAX),
        start in 0u64..=1_000_000,
    ) {
        const SLOT: usize = 0;
        let drop = UBig::from(1u8) << b;
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
        let residue = &drop + UBig::from(k);
        prop_assert_eq!(
            moved.sign,
            Sign::Negative,
            "the residue leaves the follower below where it stood, never above"
        );
        prop_assert_eq!(
            moved.magnitude,
            wide(&(residue - UBig::from(start))),
            "the follower moved by exactly the residue m − v"
        );
    }
}

proptest! {
    /// A drop landing strictly between the latent-parked minimum and the anchor
    /// never moves the minimum, at any scale and under either certification
    /// regime — and leaves the web able to take a later true undercut exactly.
    ///
    /// The worked pins above fix two points on this surface; the family is the
    /// claim they are points of. The latent's width spans the register-held and
    /// spilled regimes, and the drop spans one and two digits, so the domination
    /// read is exercised at both floor indices. Which arm answers varies across
    /// the space — a dominating latent refuses outright, comparable scales
    /// collapse the latent and re-test against the re-based anchor — and that is
    /// the point: the routing is allowed to differ, the answer is not. An arm
    /// that refused correctly but corrupted the web would pass the refusal check
    /// and fail the undercut that follows it.
    #[test]
    fn a_drop_inside_the_latent_never_moves_the_minimum(
        b in 34usize..=260,
        d in 1u64..=(1u64 << 33),
    ) {
        let lambda = UBig::from(1u8) << b;
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
        let height = &lambda - UBig::from(d);
        prop_assert_eq!(
            web.compare_above(&below_wide(&height)),
            Ordering::Equal,
            "the probe at the true minimum reads exact"
        );
        prop_assert_eq!(
            web.compare_above(&below_wide(&(&height - UBig::from(1u8)))),
            Ordering::Greater,
            "a probe above the minimum reads above"
        );
        prop_assert_eq!(
            web.compare_above(&below_wide(&(&height + UBig::from(1u8)))),
            Ordering::Less,
            "a probe below the minimum reads below"
        );
        // The refusal left the web intact: a drop that does pass the minimum
        // still seats it exactly.
        web.fold_height(Sign::Negative, &wide(&(&height + UBig::from(1u8)))); // h = −1
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
    /// A batch of `n` ranges armed by one emission closes range by range,
    /// each close consuming exactly one range record.
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
