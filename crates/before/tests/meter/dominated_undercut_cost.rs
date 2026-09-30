//! Liveness and cost checks for dominated minimum emissions.
//!
//! The family contains `k` sites where a narrow candidate minimum is known to
//! lie below a much wider saved value. Each site must take the constant-work
//! dominated case; rereading the saved value at every site would make the
//! touch count grow with both its width and `k`.
//!
//! Two observations make the check conclusive. The traffic floor requires one
//! dominated decision per constructed site, proving that the intended path ran.
//! The touch band remains flat per input byte across a joint width-and-site
//! doubling, proving that the path does not rescan the wide value. Closed-form
//! results supply the independent semantic check.

use before::testing::meter;
use before::testing::meter::registry::Shape;
use before::Party;
use suanpan::touch_meter;

/// One tick run over the family cross.
///
/// The tick's input bytes, accumulator digit touches, and
/// dominated-undercut decisions.
struct Run {
    input: u64,
    touches: u64,
    undercuts: u64,
}

/// Tick the `DU(k, b)` cross and read the touch and decision counters
/// over the tick body alone.
///
/// Enforces the arm-liveness floor before returning: the family
/// constructs exactly one dominated-undercut emission per site (the
/// generator doc carries the reachability derivation), so a reading
/// under `k` means the walk re-routed the family's emissions off the
/// arm — the arm is undriven again no matter how green every value
/// and cost pin reads.
fn tick_run(k: usize, b: usize) -> Run {
    let ev = Shape::DominatedUndercut.build2(k, b);
    let id = Shape::DominatedUndercutId.build1(k);
    let mut v = ev.version();
    let p = Party::decode(&id.bytes[..]).expect("the generator's id is canonical");
    let input = (v.encode().len() + id.bytes.len()) as u64;
    meter::reset_emit_traffic();
    touch_meter::reset();
    v.tick(&p);
    let run = Run {
        input,
        touches: touch_meter::touches(),
        undercuts: meter::emit_traffic().dominated_undercut,
    };
    assert!(
        run.undercuts >= k as u64,
        "dominated_undercut({k}, {b}): {} dominated-undercut decisions under \
         the one-per-site liveness floor {k}: the walk no longer routes the \
         family's block-minimum emissions through the dominated-undercut arm",
        run.undercuts,
    );
    run
}

/// Touch liveness floor on the larger run, derived from the walk's
/// irreducible work on this family — never from a measured basis.
///
/// Per site, the mechanism cannot avoid: the two wide input codes
/// (the climb and the block's return) each folding into the running
/// height once, and the arm's residue dying by one fold into the
/// site's arming boundary at the boundary's own width — three
/// wide-operand folds at one digit touch per 64-bit limb, `3·b/64` —
/// plus the domination read and the re-seated gap's offset fold, one
/// touch each. At (k, b) = (1,024, 1,024): 1,024·(48 + 2). A design
/// that honestly does less is a floor-premise finding — re-derive the
/// premise before trusting the trip.
const DOMINATED_UNDERCUT_TOUCH_FLOOR: u64 = 51_200;

/// Absolute touch ceiling on the larger run: the measured record
/// ×1.25, rounded up (the record lives in the pin commit).
const DOMINATED_UNDERCUT_TOUCH_CEILING: u64 = 907_525;

/// The dominated-undercut arm fires once per site and stays flat per
/// input byte across the joint (k, b) doubling.
///
/// Touches grow by at most ×1.25 per byte across the doubling, under
/// an absolute band on the larger run, with the decision counter's
/// one-per-site floor certifying the arm is the path taken.
///
/// Each site's cost is its own two wide codes' folds plus the residue's one
/// annihilation fold — flat per byte, one decision per site at
/// both scales — and the arm's decision, take-out, and re-seat are
/// O(1) beside them.
#[test]
fn tick_dominated_undercut_arm_is_flat_per_unit() {
    let small = tick_run(512, 512);
    let large = tick_run(1_024, 1_024);
    eprintln!(
        "MEASURED dominated_undercut: small={}/{}B/{}dec large={}/{}B/{}dec",
        small.touches, small.input, small.undercuts, large.touches, large.input, large.undercuts,
    );
    assert!(
        u128::from(large.touches) * u128::from(small.input) * 100
            <= u128::from(small.touches) * u128::from(large.input) * 125,
        "dominated_undercut: per-byte touch growth across the joint doubling \
         exceeds x1.25 ({}/{}B -> {}/{}B): the dominated-undercut emission has \
         picked up a width term beyond the input-funded folds",
        small.touches,
        small.input,
        large.touches,
        large.input,
    );
    assert!(
        large.touches <= DOMINATED_UNDERCUT_TOUCH_CEILING,
        "dominated_undercut: {} touches at (k, b) = (1,024, 1,024) exceed the \
         pinned ceiling {DOMINATED_UNDERCUT_TOUCH_CEILING}",
        large.touches,
    );
    assert!(
        large.touches >= DOMINATED_UNDERCUT_TOUCH_FLOOR,
        "dominated_undercut: {} touches read below the \
         {DOMINATED_UNDERCUT_TOUCH_FLOOR} liveness floor (the walk's derived \
         irreducible work): the walk's accumulator work left the metered \
         representation",
        large.touches,
    );
}
