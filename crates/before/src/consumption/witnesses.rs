//! The consumption witnesses: each roster row's property, read back
//! from [`suanpan::Accumulator`] through its public API.
//!
//! Every constant here is re-derived from suanpan's public rustdoc,
//! cited at its use; nothing is transcribed from this crate's fixture
//! comments. The load-bearing clauses, from the accumulator's docs:
//!
//! - digits denote `value = sum of d_i * 2^(32*i)`, so one digit index
//!   is worth 32 bits (the crate page's representation section);
//! - the quick register holds magnitudes to `2^96` and shifts to 30
//!   bits, so wider operands and larger shifts put the value in the
//!   digit engine (the crate page's representation section);
//! - the sign fold decides at running partial `|s| >= 3`, and
//!   `sign_dominates_at(floor)` certifies when that happens at digit
//!   index `floor + 2` or higher (the `sign` and `sign_dominates_at`
//!   rustdoc), so a top digit of 5 decides on its first step and a top
//!   digit of 2 cannot;
//! - `sign_magnitude_shl` returns the all-zero prefix below the lowest
//!   written position as the shift, sign queries count as writers, and
//!   a collapsing sign read can lower the returned shift (the
//!   `sign_magnitude_shl` rustdoc).
//!
//! Each witness carries its adequacy leg in the same test: the
//! known-bad counterpart failing or reading undecided, so no witness
//! passes vacuously. The touch-metered witnesses ride the `limb-meter`
//! feature, which lights `suanpan/touch-meter`; nextest's
//! process-per-test isolation keeps the process-global touch counter
//! private to each witness.

use core::cmp::Ordering;

use dashu_int::UBig;
use suanpan::Accumulator;

#[cfg(feature = "limb-meter")]
use suanpan::{touch_meter, Magnitude};

#[cfg(feature = "limb-meter")]
use crate::codec::Base;

/// After a sign read, `digit_count` reports the collapsed top, and a
/// domination floor derived from that count decides; derived from the
/// stale count instead, the same read refuses.
///
/// The watermark's latent ladder reads the latent's sign before
/// deriving any domination floor from a digit count (watermark.rs,
/// `decide_undercut_through_latent`): this witness pins that premise.
/// Adequacy leg: with the sign read omitted, the stale spelling still
/// counts its cancelling prefix, and the floor it yields is
/// undecidable even for an operand that dwarfs the true value.
#[test]
fn sign_collapse_tightens_the_top_and_arms_domination() {
    // The crate page's canonical cancelling prefix: +2^320, then
    // -(2^320 - 1), value 1 spelled across eleven digits (index 10
    // down to 0; 320 = 32 * 10). Both operands exceed the register's
    // 2^96 magnitude bound, so the spelling lives in the digit engine.
    let mut latent = Accumulator::new();
    latent.add_wide(&(UBig::ONE << 320usize));
    latent.sub_wide(&((UBig::ONE << 320usize) - UBig::ONE));
    let stale = latent.digit_count();
    assert_eq!(
        stale, 11,
        "the cancelling prefix leaves the stale top at the added operand's width"
    );
    // A residue with top digit 5 at index 4: decision-bound (5 >= 3,
    // the fold's decision bound) and spilled past the register.
    let mut residue = Accumulator::new();
    residue.add_wide(&(UBig::from(5u8) << 128usize));
    // Adequacy leg (sign read omitted): a floor derived from the stale
    // count demands clearance no honest operand of the latent's true
    // scale needs, and the read refuses.
    assert_eq!(
        residue.clone().sign_dominates_at(stale - 1),
        (Ordering::Greater, false),
        "a domination floor derived from the stale count must refuse"
    );
    // The sign read collapses the spelling; the count is now tight.
    assert_eq!(latent.sign(), Ordering::Greater);
    assert_eq!(
        latent.digit_count(),
        1,
        "after the sign read, digit_count reports the collapsed top"
    );
    // The collapsed count arms the decision: floor 0 sits two or more
    // digit indexes under the residue's top, so the read decides.
    assert_eq!(
        residue.sign_dominates_at(latent.digit_count() - 1),
        (Ordering::Greater, true),
        "a domination floor derived from the collapsed count decides"
    );
}

/// `sign_dominates_at` decides with a decision-bound top exactly two
/// digit indexes above the floor, and refuses one digit short or with
/// a below-bound top: clearance is sufficient and necessary.
///
/// The undercut propagation's width guards skip domination reads
/// unless one side clears the other's digit count by two
/// (watermark.rs, `propagate`): this witness pins that the skipped
/// reads are exactly the ones that could never decide, and that the
/// decided certificate is semantically good: folding in the largest
/// covered adjustment leaves the sign fixed.
#[test]
fn domination_clearance_two_digits_suffice_and_one_short_refuses() {
    // Top digit 5 at index 4: 5 * 2^128 exceeds the register's 2^96
    // bound, so the reads below are the digit engine's. The fold's
    // first partial is 5, past the decision bound 3, at index
    // 4 = floor + 2 for floor 2 (the sign_dominates_at rustdoc).
    let mut acc = Accumulator::new();
    acc.add_wide(&(UBig::from(5u8) << 128usize));
    assert_eq!(
        acc.sign_dominates_at(2),
        (Ordering::Greater, true),
        "two digits of clearance with a decision-bound top decides"
    );
    // The certificate's guarantee, exercised at its own boundary: any
    // adjustment with |a| < 2^(32 * (floor + 1)) = 2^96 leaves the
    // sign fixed.
    let mut probed = acc.clone();
    probed.sub_wide(&((UBig::ONE << 96usize) - UBig::ONE));
    assert_eq!(
        probed.sign(),
        Ordering::Greater,
        "the decided certificate covers the largest in-range adjustment"
    );
    // Adequacy leg, one digit short: the same decision-bound top at
    // floor 3 (index 4 = floor + 1) refuses.
    assert_eq!(
        acc.sign_dominates_at(3),
        (Ordering::Greater, false),
        "one digit short of clearance refuses even with a decision-bound top"
    );
    // Adequacy leg, below-bound top: top digit 2 < 3 cannot decide on
    // the top digit even at two digits of clearance.
    let mut low_top = Accumulator::new();
    low_top.add_wide(&(UBig::from(2u8) << 128usize));
    assert_eq!(
        low_top.sign_dominates_at(2),
        (Ordering::Greater, false),
        "a top digit below the decision bound refuses at the clearance line"
    );
}

/// An interleaved sign read lowers `sign_magnitude_shl`'s returned
/// shift: the collapse re-deposits its partial below the written span,
/// and the shift is the write watermark.
///
/// Why the integral sweep reads segment mass with no prior sign read
/// and opens each new segment by buffer replacement (integral.rs,
/// `settle_segment`, `settle`, and the freeze): a sign read before the
/// scaled read surrenders part of the never-written-prefix skip the
/// segment pricing rests on. The first read is the adequacy leg: with
/// the sign read omitted, the shift is the full written span, exactly.
#[test]
fn collapsing_sign_read_lowers_the_scaled_read_shift() {
    // One unit parked at digit 40: the 1280-bit shift exceeds the
    // register's 30-bit shift bound, so the value lives in the digit
    // engine with a one-digit written span at index 40.
    let mut mass = Accumulator::new();
    mass.add_wide_shl(&UBig::ONE, 1280);
    // Sign read omitted: the scaled read prices the written span and
    // returns the whole never-written prefix as the shift.
    let (sign, magnitude, shift) = mass.sign_magnitude_shl();
    assert_eq!(sign, Ordering::Greater);
    assert_eq!(
        (magnitude, shift),
        (UBig::ONE, 1280),
        "untouched, the scaled read returns the full written-span shift"
    );
    // The collapsing read: top digit 1 sits under the decision bound
    // 3, so the fold descends one digit and re-deposits the partial
    // there; sign queries count as writers (the sign_magnitude_shl
    // rustdoc), so the watermark drops with it.
    assert_eq!(mass.sign(), Ordering::Greater);
    let (sign, magnitude, shift) = mass.sign_magnitude_shl();
    assert_eq!(sign, Ordering::Greater);
    assert!(
        shift < 1280,
        "the collapsing sign read must lower the returned shift"
    );
    assert_eq!(
        (&magnitude, shift),
        (&(UBig::ONE << 32usize), 1248),
        "the collapse re-deposits one digit down: shift 32 * 39, magnitude 2^32"
    );
    // The pair is one honest spelling of the unchanged value.
    assert_eq!(
        magnitude << usize::try_from(shift).expect("the shift fits the address space"),
        UBig::ONE << 1280usize,
        "the collapse is value-preserving"
    );
}

/// `Magnitude::to_word` on `Base` answers the width dispatch with zero
/// digit touches, word-held and spilled both: the O(1) dispatch read
/// the small path's cost accounting assumes free.
///
/// The `Magnitude` rustdoc makes O(1) dispatch a contract on
/// implementors; this witness holds `Base`'s implementation to it in
/// the touch denomination. Adequacy leg (meter liveness): the same
/// counter reads nonzero across an actual digit-touching operation, so
/// a dead meter cannot satisfy the zero.
#[cfg(feature = "limb-meter")]
#[test]
fn base_dispatch_read_touches_no_digits() {
    let word_held = Base::from(7u64);
    let spilled = Base::from(UBig::ONE << 200usize);
    touch_meter::reset();
    assert_eq!(Magnitude::to_word(&word_held), Some(7));
    assert_eq!(Magnitude::to_word(&spilled), None);
    assert_eq!(
        touch_meter::touches(),
        0,
        "the dispatch read is word-scale: no digit touched on either arm"
    );
    // Meter liveness: the counter counts when digits actually move.
    let mut acc = Accumulator::new();
    acc.add_magnitude(&spilled);
    assert!(
        touch_meter::touches() > 0,
        "meter liveness: a wide add must touch digits"
    );
}

/// A magnitude with top digit 5 at index `i` decides
/// `sign_dominates_at(i - 2)` on its first digit touch: exactly one
/// metered touch, decided, sign exact.
///
/// The seam fixtures' closed forms (tests/meter.rs, `seam_plunge_ticks`
/// and kin) put top digit 5 at the guards' clearance line so every hop
/// decides without descending; this witness pins that premise against
/// the accumulator itself. Adequacy legs: one digit short of clearance
/// the same top refuses on the same single touch, and a below-bound
/// top digit 2 refuses while descending, at more than one touch, so a
/// fold that stopped deciding at the line could not pass.
#[cfg(feature = "limb-meter")]
#[test]
fn decision_bound_top_decides_on_the_first_touch() {
    // Top digit 5 at index 4, spilled past the register's 2^96 bound.
    let mut acc = Accumulator::new();
    acc.add_wide(&(UBig::from(5u8) << 128usize));
    touch_meter::reset();
    assert_eq!(acc.sign_dominates_at(2), (Ordering::Greater, true));
    assert_eq!(
        touch_meter::touches(),
        1,
        "the decision-bound top decides on the first digit touch"
    );
    // Adequacy leg, one digit short: the fold still stops on its first
    // touch (the partial is decided), but the certificate refuses.
    touch_meter::reset();
    assert_eq!(acc.sign_dominates_at(3), (Ordering::Greater, false));
    assert_eq!(
        touch_meter::touches(),
        1,
        "one digit short refuses without reading further"
    );
    // Adequacy leg, below-bound top: the fold must descend past the
    // top digit, so the read cannot decide on its first touch.
    let mut low_top = Accumulator::new();
    low_top.add_wide(&(UBig::from(2u8) << 128usize));
    touch_meter::reset();
    assert_eq!(low_top.sign_dominates_at(2), (Ordering::Greater, false));
    assert!(
        touch_meter::touches() > 1,
        "a below-bound top cannot decide on its first touch"
    );
}
