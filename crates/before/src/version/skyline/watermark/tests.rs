//! Direct pins on the web's latent-ladder gates that no packed-stream walk
//! reaches, and on the seam contracts no packed stream drives.
//!
//! Deliberate internal-entry tests, driving [`MinWeb`] at its own seam. Both
//! sweeps read a live latent only immediately behind a raise-decision read
//! (`compare_above`) on the same folded state: comparable scales collapse the
//! latent inside that read, and a decided drop past the latent routes the
//! raise to the tracked minimum's emission — so `emit_offset`'s post-collapse
//! restore and the undercut's latent annihilation in `drop_below` execute on
//! no input either walk can be handed. The public-surface family
//! (`fill/tests.rs`'s latent-ladder suite) pins every walk-reachable arm
//! against the recursive oracle; the two worked pins here hold the web's
//! remaining gates to the same polarity discipline at the only seam that can
//! reach them.
//!
//! One seam contract also lives only here: closing a range no emission ever
//! armed consumes exactly its pending slot ([`Close::Pending`]). Both walks
//! happen to emit inside every range they open, so no packed stream drives
//! this arm.

use core::cmp::Ordering;

use proptest::prelude::*;

use crate::codec::Int;

use super::super::signed::{Sign, Signed};
use super::{Close, MinWeb};

/// A priced word-scale offset `−n`: an emission `n` below the running height.
fn below(n: u64) -> Signed {
    Signed {
        sign: Sign::Negative,
        magnitude: Int::Small(n),
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

proptest! {
    /// Closing a range no emission ever armed consumes exactly its pending
    /// slot ([`Close::Pending`]): the web neither retires early, disturbs an
    /// armed range's tracked minimum, nor miscounts the ranges left open.
    #[test]
    fn pending_closes_consume_exactly_one_pending_slot(n in 1usize..40) {
        // A fresh web: every close of a never-armed range reports it
        // pending, and the last one leaves the web drained on both counts.
        let mut web: MinWeb<()> = MinWeb::new();
        web.open(n);
        for _ in 0..n {
            prop_assert!(
                matches!(web.close(), Close::Pending),
                "a never-armed range closes as pending"
            );
        }
        prop_assert!(!web.has_pending(), "the pending closes balance the opens");
        prop_assert!(!web.armed(), "no emission ever armed the fresh web");

        // An armed web: pending closes spend no armed range and leave the
        // tracked minimum exactly where the arming emission put it.
        let mut web: MinWeb<()> = MinWeb::new();
        web.open(1);
        web.emit_here(); // arms at v = 0
        web.fold_height(Sign::Positive, &Int::Small(7)); // h = 7
        web.open(n);
        for _ in 0..n {
            prop_assert!(
                matches!(web.close(), Close::Pending),
                "a never-armed range closes as pending under an armed one"
            );
        }
        prop_assert!(!web.has_pending(), "the pending closes balance the opens");
        prop_assert!(web.armed(), "the armed range survives its inner pending closes");
        prop_assert_eq!(
            web.compare_above(&below(7)),
            Ordering::Equal,
            "the probe at the armed minimum reads exact"
        );
        prop_assert_eq!(
            web.compare_above(&below(6)),
            Ordering::Greater,
            "a probe above the armed minimum reads above"
        );
        prop_assert_eq!(
            web.compare_above(&below(8)),
            Ordering::Less,
            "a probe below the armed minimum reads below"
        );
    }
}
