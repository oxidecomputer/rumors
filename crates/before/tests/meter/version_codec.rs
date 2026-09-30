//! Validation envelopes for encoded version streams.

use super::*;

// ─── Version decoding scenarios ─────────────────────────────────────────────
//
// The Version validator over the registered worst-case event families, with the
// stream transcoded outside measurement. The rows pin the validator's
// transient (~2 bits of open-ancestor stack per level plus the
// cliff-free accumulator), denominated against Version input bytes: the
// validator is the piece that carries the wire-bit-linear claim, and the
// public decode rows price the wrap beside it.

/// The Version validator on the dense spine stays within its envelope.
///
/// The transient is ~2 bits per open ancestor (bit stack plus
/// reallocation growth): bits per level, not frames.
#[test]
fn skyline_validate_dense_envelope() {
    let enc = version_of(&Shape::Dense.build1(DENSE_DEPTH));
    let r = metered(
        "skyline_validate_dense",
        enc.as_bytes().len(),
        &envelope::SKYLINE_VALIDATE_DENSE,
        || meter::version::validate(&enc),
    );
    assert!(r.is_ok(), "the transcoded dense spine is canonical");
}

/// The Version validator on the boundary comb stays within its envelope.
///
/// Every 3-bit `±1` delta sits on the `2^k` carry boundary, and the
/// accumulator's redundant representation keeps the nonnegativity check
/// amortized O(1) per delta (the flatness pin below is the cross-scale
/// witness; a plain big-integer accumulator is quadratic here).
#[test]
fn skyline_validate_cliff_envelope() {
    let enc = version_of(&Shape::CliffComb.build2(CLIFF_SCALE, CLIFF_SCALE));
    let r = metered(
        "skyline_validate_cliff",
        enc.as_bytes().len(),
        &envelope::SKYLINE_VALIDATE_CLIFF,
        || meter::version::validate(&enc),
    );
    assert!(r.is_ok(), "the transcoded boundary comb is canonical");
}

/// The Version validator on the wide-tooth comb stays within its envelope:
/// each `±2^w` delta is a wide operand paid for by its own zigzag code, so
/// accumulator work stays linear per input bit at every tooth width.
#[test]
fn skyline_validate_wide_tooth_envelope() {
    let enc =
        version_of(&Shape::WideToothComb.build3(CLIFF_SCALE, WIDE_TOOTH_WIDTH_BITS, CLIFF_SCALE));
    let r = metered(
        "skyline_validate_wide_tooth",
        enc.as_bytes().len(),
        &envelope::SKYLINE_VALIDATE_WIDE_TOOTH,
        || meter::version::validate(&enc),
    );
    assert!(r.is_ok(), "the transcoded wide-tooth comb is canonical");
}

/// The Version validator on the hugeleaf analog — a single huge first
/// leaf, the whole stream one absolute gamma code — stays within its
/// envelope.
///
/// The cost is one wide decode plus one wide accumulator load, both
/// linear in the code's own width.
#[test]
fn skyline_validate_hugeleaf_envelope() {
    let enc = version_of(&Shape::Hugeleaf.build1(HUGELEAF_MAGNITUDE_BITS));
    let r = metered(
        "skyline_validate_hugeleaf",
        enc.as_bytes().len(),
        &envelope::SKYLINE_VALIDATE_HUGELEAF,
        || meter::version::validate(&enc),
    );
    assert!(r.is_ok(), "the transcoded hugeleaf is canonical");
}

/// The Version validator on the alternating-binary spine stays within its
/// envelope: the direction of descent flips every level, so per-level
/// state is maximally non-uniform — and still costs 2 bits per level, not
/// a frame.
#[test]
fn skyline_validate_alt_spine_envelope() {
    let enc = version_of(&Shape::AltSpine.build1(DENSE_DEPTH));
    let r = metered(
        "skyline_validate_alt_spine",
        enc.as_bytes().len(),
        &envelope::SKYLINE_VALIDATE_ALT_SPINE,
        || meter::version::validate(&enc),
    );
    assert!(r.is_ok(), "the transcoded alternating spine is canonical");
}

/// The validator rows' scan floor is live.
///
/// Judged against `SKYLINE_VALIDATE_DENSE` with its touch band opened, over
/// the whole dense stream's length, a validator stubbed to
/// `Ok(())` and one that reads only half the stream each fail on the scan
/// floor and nothing else.
///
/// Both control bodies do less work than the real validator, so only the floor
/// can reject them.
#[test]
fn stopped_validator_fails_the_validate_row() {
    let whole = version_of(&Shape::Dense.build1(DENSE_DEPTH));
    let half = version_of(&Shape::Dense.build1(DENSE_DEPTH / 2));
    let input = whole.as_bytes().len();
    // Open the touch band so only the scan floor can reject either control.
    let scan_only = Envelope {
        touch: band(u64::MAX, 0),
        ..envelope::SKYLINE_VALIDATE_DENSE
    };
    let failure = |name: &str, body: &dyn Fn()| {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            metered(name, input, &scan_only, body)
        }))
        .err()
        .and_then(|payload| payload.downcast_ref::<String>().cloned())
        .expect("the row must fail, with a message")
    };
    let stubbed = failure("validate_stub_probe", &|| ());
    assert!(
        stubbed.contains("liveness floor"),
        "a stubbed validator must fail the scan floor, not: {stubbed}"
    );
    let stopped = failure("validate_stopped_probe", &|| {
        meter::version::validate(&half).expect("the half-depth spine is canonical");
    });
    assert!(
        stopped.contains("liveness floor"),
        "a validator that reads half its input must fail the scan floor, not: {stopped}"
    );
}
