//! Boundary and latent-state checks for minimum-tick accounting.

use super::*;

// ─── wide-hop guards at their clearance boundary ───
//
// The range-minimum tracker's undercut propagation decides each wide hop by
// top-index domination before any fold, and the dying side — residue
// or stacked boundary — folds once at its own width. The committed
// cascades sit far from the decision boundary (word-scale dying
// differences under a residue dozens of digits wide, or exact
// annihilation), so the guards' clearance line itself — hops decided
// at exactly two digits of daylight, in both directions — is reached
// by this family alone. These bands put both arms on the line:
// the plunge drives the residue-dominates arm (k three-digit
// boundaries dying into one r-digit residue), the stop drives the
// boundary-dominates arm (k three-digit residues dying against one
// five-digit survivor), and each is paired with a wire-near-identical
// control whose run difference isolates the hops from the shared
// consume/arm work. The clearance band moves only the residue's
// digit clearance and holds per-byte cost flat in both directions: a
// guard that stops deciding at the line (or starts demanding more
// clearance) inflates exactly the minimal-clearance point.

/// Sites in the boundary bands' small runs; large runs double this count.
const BOUNDARY_SMALL_K: usize = 512;

/// The plunge residue's digit count at the guards' minimal decidable
/// clearance: three-digit boundaries plus the two-digit certificate.
///
/// The closed forms rest on every hop deciding without a descent: a
/// decision-bound top (the boundary shapes' top digit 5) decides two digit
/// indexes below itself on its first touch — suanpan's witness
/// `decision_bound_top_decides_on_the_first_touch`.
const BOUNDARY_CLEARANCE: usize = 5;

/// The clearance band's second point: the same hops decided with five
/// digits of daylight, every dying width unchanged.
const BOUNDARY_CLEARANCE_WIDE: usize = 10;

/// The descending-boundary closed form: `(k + 1)` ascending leaves over all-zero
/// minima plus the plunge.
fn descending_boundary_ticks(k: usize, r: usize) -> BigUint {
    (BigUint::from(5u8) << (32 * (r - 1))) * BigUint::from((k + 1) as u64)
        + (BigUint::from(5u8) << 64usize) * BigUint::from(((k + 1) * (k + 2) / 2) as u64)
}

/// The leveled control's closed form: the ascent on the bases, the
/// terminal one rung up.
fn descending_boundary_control_ticks(k: usize, r: usize) -> BigUint {
    (BigUint::from(5u8) << (32 * (r - 1)))
        + (BigUint::from(5u8) << 64usize) * BigUint::from((k + 2) as u64)
}

/// The stopping pair's shared closed form (the control differs only in
/// zero-base wrapping).
fn stopping_boundary_ticks(k: usize) -> BigUint {
    (BigUint::from(5u8) << 128usize)
        + (BigUint::from((k - 1) as u64) << 80usize)
        + (BigUint::from(5u8) << 64usize) * BigUint::from((k * (k - 1) / 2) as u64)
}

/// Touch liveness floor on the descending shape's larger run, derived from
/// the propagation's irreducible work — never from a measured basis.
///
/// The plunge consumes every stacked boundary: `k` dying folds, each
/// reading its dying operand's three base-2^32 digits once (the dying
/// side is a spilled two-limb magnitude, so its held digit span is
/// exactly three), `3k` touches at `k = 1,024`. Everything else the run
/// does is on top; a reading below this means the propagation's folds
/// left the metered representation.
const DESCENDING_TOUCH_FLOOR: u64 = 3 * 2 * BOUNDARY_SMALL_K as u64;

/// Absolute touch ceiling on the descending shape's larger run: the measured
/// record ×1.25, rounded up (the record lives in the pin commit).
const DESCENDING_TOUCH_CEILING: u64 = 26_945;

/// Band on the control-minus-plunge touch difference at the larger run:
/// the measured record ×0.75 down and ×1.25 up (the record lives in the
/// pin commit).
///
/// The two runs share every consume and arm; they part only at the end,
/// where the control's drain parks each surviving boundary into the
/// latent register (one merge at the dying side's three digits per
/// close) while the plunge kills the same boundaries by domination hops
/// (one O(1) read plus the same-width dying fold each). The measured
/// surplus is positive: a domination hop costs *less* than a close-park
/// of the same boundary, by a small constant per site. A per-hop read
/// of the surviving residue's width in the plunge (the regression the
/// wide-hop guards exist to prevent) drives this surplus through zero —
/// the floor is the tripwire — while a park regression inflates it past
/// the ceiling.
const DESCENDING_PARK_SURPLUS_BAND: (i64, i64) = (1_514, 2_523);

/// The descending cascade is flat per retiring digit width.
///
/// Per-byte touches stay flat (×1.25) across a site doubling at the
/// guards' minimal clearance, under an absolute band, over the derived
/// floor, with the control-minus-plunge surplus banded.
///
/// Semantics first: both shapes fold to their closed forms, so the cost
/// legs ride on pinned values. The signature: each site pays its own
/// three-digit code a constant number of times, and the plunge adds one
/// domination read plus one dying three-digit fold per boundary — flat
/// per byte, with the surplus leg isolating the hops against the
/// control's drain parks.
#[test]
fn skyline_min_ticks_descending_boundary_is_flat_per_unit() {
    let run = |k: usize| {
        let plunge = min_ticks_family_run(
            Shape::SeamPlunge.build2(k, BOUNDARY_CLEARANCE),
            &descending_boundary_ticks(k, BOUNDARY_CLEARANCE),
        );
        let control = min_ticks_family_run(
            Shape::SeamPlungeControl.build2(k, BOUNDARY_CLEARANCE),
            &descending_boundary_control_ticks(k, BOUNDARY_CLEARANCE),
        );
        (plunge, control)
    };
    let (small, small_control) = run(BOUNDARY_SMALL_K);
    let (large, large_control) = run(2 * BOUNDARY_SMALL_K);
    eprintln!(
        "MEASURED seam_plunge: small={}/{}B large={}/{}B control_small={}/{}B \
         control_large={}/{}B diff_small={} diff_large={}",
        small.touches,
        small.bytes,
        large.touches,
        large.bytes,
        small_control.touches,
        small_control.bytes,
        large_control.touches,
        large_control.bytes,
        small.touches as i64 - small_control.touches as i64,
        large.touches as i64 - large_control.touches as i64,
    );
    assert_flat(
        "seam_plunge_touches",
        "byte",
        (small.touches, small.bytes),
        (large.touches, large.bytes),
    );
    assert!(
        large.touches <= DESCENDING_TOUCH_CEILING,
        "seam_plunge: {} touches exceed the pinned ceiling \
         {DESCENDING_TOUCH_CEILING}",
        large.touches,
    );
    assert!(
        large.touches >= DESCENDING_TOUCH_FLOOR,
        "seam_plunge: {} touches read below the {DESCENDING_TOUCH_FLOOR} \
         liveness floor (the k dying three-digit folds alone): the \
         propagation's work left the metered representation",
        large.touches,
    );
    let surplus = i64::try_from(large_control.touches).expect("touch counts fit i64")
        - i64::try_from(large.touches).expect("touch counts fit i64");
    assert!(
        surplus >= DESCENDING_PARK_SURPLUS_BAND.0,
        "seam_plunge: the control-minus-plunge surplus {surplus} fell below its \
         band floor {} — a per-hop read of the surviving residue's width is \
         back in the cascade (or the parks got cheaper: attribute and re-pin)",
        DESCENDING_PARK_SURPLUS_BAND.0,
    );
    assert!(
        surplus <= DESCENDING_PARK_SURPLUS_BAND.1,
        "seam_plunge: the control-minus-plunge surplus {surplus} exceeds its \
         band ceiling {} — the drain parks picked up width, or the hops got \
         cheaper: attribute and re-pin",
        DESCENDING_PARK_SURPLUS_BAND.1,
    );
}

/// The wide-hop guards decide at their minimal clearance.
///
/// Per-byte touches agree within ×1.25 *in both directions* between the
/// plunge whose hops have exactly two digits of daylight and the one
/// whose hops have five, at equal site count and dying widths.
///
/// The `r` knob moves only the residue's digit clearance (and its two
/// wide codes' width), so the expected mechanism reads near-identical
/// per byte at both points. A clearance regression is one-sided:
/// demanding more daylight (or mis-certifying at the line) reroutes
/// exactly the minimal-clearance point's hops onto the comparable-scale
/// fold, inflating it against the wide point — the direction the
/// standard growth band never checks.
#[test]
fn skyline_min_ticks_descending_boundary_clearance_band() {
    let k = 2 * BOUNDARY_SMALL_K;
    let tight = min_ticks_family_run(
        Shape::SeamPlunge.build2(k, BOUNDARY_CLEARANCE),
        &descending_boundary_ticks(k, BOUNDARY_CLEARANCE),
    );
    let wide = min_ticks_family_run(
        Shape::SeamPlunge.build2(k, BOUNDARY_CLEARANCE_WIDE),
        &descending_boundary_ticks(k, BOUNDARY_CLEARANCE_WIDE),
    );
    eprintln!(
        "MEASURED seam_plunge_clearance: tight={}/{}B wide={}/{}B",
        tight.touches, tight.bytes, wide.touches, wide.bytes,
    );
    for (name, a, b) in [
        ("tight-over-wide", &tight, &wide),
        ("wide-over-tight", &wide, &tight),
    ] {
        assert!(
            u128::from(a.touches) * u128::from(b.bytes) * u128::from(SLACK_DEN)
                <= u128::from(b.touches) * u128::from(a.bytes) * u128::from(SLACK_NUM),
            "seam_plunge_clearance ({name}): per-byte touches diverge more than \
             ×1.25 across the clearance change ({}/{}B vs {}/{}B): the wide-hop \
             guards' decision boundary moved",
            a.touches,
            a.bytes,
            b.touches,
            b.bytes,
        );
    }
}

/// Touch liveness floor on the stopping shape's larger run, derived from the
/// stopping arm's irreducible work — never from a measured basis.
///
/// Each of the `k` descents dies by one terminal fold into the
/// surviving boundary, reading its own three base-2^32 digits once
/// (each dying residue is a spilled two-limb magnitude), `3k` touches
/// at `k = 1,024`. A reading below this means the stopping folds left
/// the metered representation.
const STOPPING_TOUCH_FLOOR: u64 = 3 * 2 * BOUNDARY_SMALL_K as u64;

/// Absolute touch ceiling on the stopping shape's larger run: the measured
/// record ×1.25, rounded up (the record lives in the pin commit).
const STOPPING_TOUCH_CEILING: u64 = 38_017;

/// Band on the stop-minus-control touch difference at the larger run:
/// the measured record ×0.75 down and ×1.25 up (the record lives in
/// the pin commit).
///
/// The two runs share every consume, arm, and residue; they part only
/// at the stack: the stop's residues each pass the zero run, take one
/// domination read on the surviving boundary, and die by their own
/// three-digit terminal fold into it, while the control's residues
/// retire unfolded at its empty stack. The difference is therefore the
/// boundary-dominates hop itself (plus the one stacked-boundary arming
/// and its drain park, O(1) in `k`): a per-hop read of the surviving
/// boundary's width lands whole here, undiluted by the shared work.
const STOPPING_DIFF_BAND: (u64, u64) = (5_904, 9_840);

/// The stopping arm uses O(1) work per hop beside the retiring fold:
/// per-byte touches stay flat (×1.25) across a site doubling, under an
/// absolute band, over the derived floor, with the stop-minus-control
/// difference banded.
///
/// Semantics first: both shapes fold to the same closed form (the
/// control drops only zero-base wrapping), so the cost legs ride on
/// pinned values — and the value leg is what pins the surviving
/// boundary's shrink polarity: each hop leaves the survivor smaller by
/// exactly the dying residue.
#[test]
fn skyline_min_ticks_stopping_boundary_is_flat_per_unit() {
    let run = |k: usize| {
        let stop = min_ticks_family_run(Shape::SeamStop.build1(k), &stopping_boundary_ticks(k));
        let control = min_ticks_family_run(
            Shape::SeamStopControl.build1(k),
            &stopping_boundary_ticks(k),
        );
        (stop, control)
    };
    let (small, small_control) = run(BOUNDARY_SMALL_K);
    let (large, large_control) = run(2 * BOUNDARY_SMALL_K);
    eprintln!(
        "MEASURED seam_stop: small={}/{}B large={}/{}B control_small={}/{}B \
         control_large={}/{}B diff_small={} diff_large={}",
        small.touches,
        small.bytes,
        large.touches,
        large.bytes,
        small_control.touches,
        small_control.bytes,
        large_control.touches,
        large_control.bytes,
        small.touches as i64 - small_control.touches as i64,
        large.touches as i64 - large_control.touches as i64,
    );
    assert_flat(
        "seam_stop_touches",
        "byte",
        (small.touches, small.bytes),
        (large.touches, large.bytes),
    );
    assert!(
        large.touches <= STOPPING_TOUCH_CEILING,
        "seam_stop: {} touches exceed the pinned ceiling {STOPPING_TOUCH_CEILING}",
        large.touches,
    );
    assert!(
        large.touches >= STOPPING_TOUCH_FLOOR,
        "seam_stop: {} touches read below the {STOPPING_TOUCH_FLOOR} liveness \
         floor (the k dying three-digit folds alone): the stopping folds left \
         the metered representation",
        large.touches,
    );
    let diff = large
        .touches
        .checked_sub(large_control.touches)
        .expect("the stop does at least its control's work");
    assert!(
        diff >= STOPPING_DIFF_BAND.0,
        "seam_stop: the stop-minus-control difference {diff} fell below its \
         band floor {} (measured ×0.75): attribute the improvement and re-pin",
        STOPPING_DIFF_BAND.0,
    );
    assert!(
        diff <= STOPPING_DIFF_BAND.1,
        "seam_stop: the stop-minus-control difference {diff} exceeds its band \
         ceiling {} — a per-hop read of the surviving boundary's width is back \
         in the stopping arm",
        STOPPING_DIFF_BAND.1,
    );
}

// ─── the latent ladder: the parked-latent decision's O(1) claim ──────────

/// Width (base-2^32 digits) of the ladder's parked latent at the small
/// point; the band doubles it.
const LADDER_WIDTH: usize = 64;

/// Decisions of the ladder's small runs; the marginal doubles it.
const LADDER_K: usize = 512;

/// The latent-ladder closed form: the parked pair over the floor plus
/// `k` ladder leaves one to `k` under the anchor.
fn latent_ladder_ticks(w: usize, k: usize) -> BigUint {
    (BigUint::from(5u8) << (32 * (w - 1))) * BigUint::from((k + 1) as u64) + BigUint::ONE
        - BigUint::from((k * (k + 1) / 2) as u64)
}

/// Touch liveness floor on the ladder's per-width `k`-marginal, derived
/// from each decision leaf's irreducible work — never from a measured
/// basis.
///
/// Per additional leaf the fold cannot avoid three register folds: the
/// consumed step into the live component and the anchor gap (both
/// word-scale, one touch each), and the leaf's offset into the running
/// total — `3k` touches on a marginal of `k = 512` leaves. A marginal
/// below this means the decision leaves' work left the metered
/// representation.
const LADDER_MARGINAL_TOUCH_FLOOR: u64 = 3 * LADDER_K as u64;

/// Absolute touch ceiling on the ladder's `k`-marginal at the doubled
/// width: the measured record ×1.25, rounded up (the record lives in
/// the pin commit).
const LADDER_MARGINAL_TOUCH_CEILING: u64 = 4_480;

/// The parked-latent undercut decision answers scale-disparate drops
/// in O(1).
///
/// The per-decision marginal cost is flat (×1.25, both directions)
/// across a doubling of the parked latent's width, under an absolute
/// ceiling and over the derived floor.
///
/// The marginal `T(w, 2k) − T(w, k)` isolates the `k` extra decision
/// leaves exactly — the parked boundary's own arming, park, freeze, and
/// settle are identical at both `k` and cancel — so a decision that
/// reads the latent across its width (the regression
/// `decide_undercut_through_latent`'s O(1) claim forbids) doubles the
/// marginal when the width doubles. Semantics first: all four points
/// fold to their closed forms, so the marginal rides on pinned values.
#[test]
fn skyline_min_ticks_latent_ladder_is_flat_per_unit() {
    let marginal = |w: usize| {
        let base = min_ticks_family_run(
            Shape::LatentLadder.build2(w, LADDER_K),
            &latent_ladder_ticks(w, LADDER_K),
        );
        let doubled = min_ticks_family_run(
            Shape::LatentLadder.build2(w, 2 * LADDER_K),
            &latent_ladder_ticks(w, 2 * LADDER_K),
        );
        doubled
            .touches
            .checked_sub(base.touches)
            .expect("more decisions cost more touches")
    };
    let narrow = marginal(LADDER_WIDTH);
    let wide = marginal(2 * LADDER_WIDTH);
    eprintln!("MEASURED latent_ladder: marginal narrow={narrow} wide={wide}");
    for (name, a, b) in [
        ("wide-over-narrow", wide, narrow),
        ("narrow-over-wide", narrow, wide),
    ] {
        assert!(
            u128::from(a) * u128::from(SLACK_DEN) <= u128::from(b) * u128::from(SLACK_NUM),
            "latent_ladder ({name}): the per-decision marginal moved more than \
             ×1.25 across the width doubling ({narrow} -> {wide}): the undercut \
             decision is reading the parked latent's width",
        );
    }
    assert!(
        wide <= LADDER_MARGINAL_TOUCH_CEILING,
        "latent_ladder: the doubled-width marginal {wide} exceeds the pinned \
         ceiling {LADDER_MARGINAL_TOUCH_CEILING}",
    );
    assert!(
        wide >= LADDER_MARGINAL_TOUCH_FLOOR,
        "latent_ladder: the doubled-width marginal {wide} read below the \
         {LADDER_MARGINAL_TOUCH_FLOOR} liveness floor (three register folds \
         per decision leaf): the decision leaves' work left the metered \
         representation",
    );
}
